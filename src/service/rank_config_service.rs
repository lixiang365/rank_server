use crate::config::parameter;
use crate::db::database::Database;
use crate::dto::rank_dto::{AddRankConfigReq, UpdateScoreRequest};
use crate::error::api_error::ApiError;
use crate::error::db_error::DbError;
use crate::error::request_error::RequestError;
use crate::model::user::RankTableConfig;
use crate::repository::rank_repository::{RankRepository, RankRepositoryTrait};
use crate::pb::update_rank_config;
use deadpool_redis::Pool;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, atomic::{AtomicU64, Ordering}};
use tokio::sync::RwLock;
use std::time::Duration;
use tokio_cron_scheduler::{Job, JobScheduler};
use uuid::Uuid;

#[derive(Clone)]
pub struct RankConfigService {
    pub master_node: bool,
    rank_repo: RankRepository,
    _db_conn: Arc<Database>,
    sched: JobScheduler,
    pub rank_table_configs: Arc<Mutex<Vec<RankTableConfig>>>,
    pub rank_config_secret_map: Arc<RwLock<HashMap<String, String>>>,
    pub config_update_time: Arc<AtomicU64>,
}

impl RankConfigService {
    pub fn new(
        db_conn: &Arc<Database>,
        redis_con_pool: &Pool,
        sched: &JobScheduler,
        master_node: bool,
    ) -> Self {
        Self {
            master_node,
            rank_repo: RankRepository::new(db_conn, redis_con_pool),
            _db_conn: Arc::clone(db_conn),
            sched: sched.clone(),
            rank_table_configs: Default::default(),
            rank_config_secret_map: Default::default(),
            config_update_time: Arc::new(AtomicU64::new(0)),
        }
    }

    /// 启动加载排行榜数据
    pub async fn init_rank(&self) -> bool {
        // 1 读取排行榜配置
        let rank_table_configs = match self.load_rank_table_config().await {
            Ok(configs) => configs,
            Err(err) => {
                tracing::error!("rank table config err:{}", err.to_string());
                return false;
            }
        };
        // 添加密钥映射
        {
            let mut secret_map = self.rank_config_secret_map.write().await;
            for config in &rank_table_configs {
                secret_map.insert(config.appid.clone(), config.app_secret.clone());
            }
        }
        {
            let mut guard = self.rank_table_configs.lock().unwrap();
            *guard = rank_table_configs;
        }
		self.config_update_time.store(chrono::Utc::now().timestamp_millis() as u64,Ordering::Relaxed);

		// 非master节点无需 开启计划任务，管理排行配置
		if !self.master_node {
			return true;
		}
        // 保存定时任务启动的uuid
        let uuids = Vec::<(String, String, String)>::new();
        let arc_uuids = Arc::new(Mutex::new(uuids));

        {
            let rank_table_configs: &Vec<RankTableConfig>;
            // 最小锁使用范围
            let guard = self.rank_table_configs.lock().unwrap();
            rank_table_configs = guard.as_ref();

            // 根据启动参数，判断是否需要把所有mysql排行榜数据加载到redis
            if parameter::CMD_ARGS.get().is_some()
                && parameter::CMD_ARGS.get().unwrap().contains("--sync_redis")
            {
                for rank_tbl_cfg in rank_table_configs {
                    if self.read_mysql_rank_write_to_redis(rank_tbl_cfg).await == false {
                        tracing::error!(
                            "init_rank err appid:{} | rank_key:{}",
                            rank_tbl_cfg.appid,
                            rank_tbl_cfg.rank_key
                        );
                        return false;
                    }
                }
            }

            // 启动定时任务
            for rank_tbl_cfg in rank_table_configs {
                let arc_uuids_clone = Arc::clone(&arc_uuids);
                if let Some(uuid) = self.start_rank_cron_job(rank_tbl_cfg).await {
                    if !uuid.is_nil() {
                        let mut guard = arc_uuids_clone.lock().unwrap();
                        guard.push((
                            rank_tbl_cfg.appid.clone(),
                            rank_tbl_cfg.rank_key.clone(),
                            uuid.to_string(),
                        ));
                    }
                } else {
                    return false;
                }
            }
        }

        // 更新定时任务的uuid到rank_service
        let guard = arc_uuids.lock().unwrap();
        for (appid, rank_key, uuid) in &(*guard) {
            self.update_sched_uuid(uuid, appid, rank_key);
        }

        match self.sched.start().await {
            Ok(_) => {}
            Err(err) => {
                tracing::error!("init_rank sched start err:{}", err.to_string());
                return false;
            }
        }
        return true;
    }

	/// 初始化grpc服务
    pub async fn init_update_rank_config_grpc_service(
        &self,
        rank_config_service_arc: Arc<RankConfigService>,
    ) -> () {
        if self.master_node {
            tokio::spawn(async move {
                let addr = format!("0.0.0.0:{}", parameter::get("GRPC_SERVER_PORT"));
                let service = UpdateRankConfigGrpcServer::new(rank_config_service_arc);
                tracing::info!("GRPC service listing on addr: {}", addr);
                tonic::transport::Server::builder()
                    .add_service(
                        update_rank_config::update_rank_config_server::UpdateRankConfigServer::new(
                            service,
                        ),
                    )
                    .serve(addr.parse().unwrap())
                    .await
                    .unwrap();
            });
        } else {
            tokio::spawn(async move {
                let addr = parameter::get("GRPC_SERVER_URL");
				let client = 
				match update_rank_config::update_rank_config_client::UpdateRankConfigClient::connect(addr).await {
					Ok(client)=>{
						client
					},
					Err(e)=>{
						panic!("update rank config grpc client connect failed! error:{}",e.to_string());
					}
				};
				let mut service = UpdateRankConfigGrpcClient::new(rank_config_service_arc,client);

				loop {
					service.send_update_config_request().await;
					tokio::time::sleep(Duration::from_secs(30)).await;
				}
            });
        }

    }

    /// 启动加载排行榜表配置
    pub async fn load_rank_table_config(&self) -> Result<Vec<RankTableConfig>, String> {
        let rank = match self.rank_repo.get_rank_table_config_from_mysql().await {
            Ok(configs) => Ok(configs),
            Err(err) => {
                tracing::error!("load_rank_table_config err :{}", err.to_string());
                Err(err.to_string())
            }
        };
        return rank;
    }

    /// 从数据库读取排行榜然后写入redis
    pub async fn read_mysql_rank_write_to_redis(&self, table_config: &RankTableConfig) -> bool {
        const PAGE_SIZE: u64 = 100;
        let mut start_index = 0;
        loop {
            match self
                .rank_repo
                .get_pagination_all_users_score_info_from_mysql(
                    &table_config.appid,
                    &table_config.rank_key,
                    start_index,
                    PAGE_SIZE,
                )
                .await
            {
                Ok(user_list) => {
                    for user_info in &user_list {
                        let user_score_info = UpdateScoreRequest {
                            appid: table_config.appid.clone(),
                            openid: user_info.openid.clone(),
                            rank_key: table_config.rank_key.clone(),
                            nick_name: user_info.nick_name.clone(),
                            score: user_info.score,
                        };
                        match self
                            .rank_repo
                            .update_rank_score_to_redis(&user_score_info)
                            .await
                        {
                            Ok(_) => {
                                match self
                                    .rank_repo
                                    .update_user_info_to_redis(&user_score_info)
                                    .await
                                {
                                    Ok(_) => {}
                                    Err(err) => {
                                        tracing::error!(
                                            "update user info to redis error :{}",
                                            err.to_string()
                                        );
                                        return false;
                                    }
                                }
                            }
                            Err(err) => {
                                // TODO@ 当更新排行榜出错后需要重试，或者记录错误信息到本地
                                tracing::error!("update score to redis error :{}", err.to_string());
                                return false;
                            }
                        }
                    }
                    if (user_list.len() as u64) < PAGE_SIZE {
                        break;
                    } else {
                        start_index += PAGE_SIZE;
                    }
                }
                Err(err) => {
                    tracing::error!(
                        "get_pagination_all_users_score_info_from_mysql error :{}",
                        err.to_string()
                    );
                    return false;
                }
            }
        }
        return true;
    }

    // 开启排行榜定时任务
    pub async fn start_rank_cron_job(&self, rank_table_config: &RankTableConfig) -> Option<Uuid> {
        // 永久
        if rank_table_config.cron_expression.len() == 0 {
            return Some(Uuid::default());
        }

        let n_rank_service = self.clone();
        let appid = rank_table_config.appid.clone();
        let rank_key = rank_table_config.rank_key.clone();
        let job = match Job::new_async(
            rank_table_config.cron_expression.as_str(),
            move |_uuid, mut _l| {
                let n_n_rank_service = n_rank_service.clone();
                let n_appid = appid.clone();
                let n_rank_key = rank_key.clone();
                Box::pin(async move {
                    n_n_rank_service.clear_rank_data(n_appid, n_rank_key).await;
                    ()
                })
            },
        ) {
            Ok(job) => job,
            Err(err) => {
                tracing::error!(
                    "start_rank_cron_job job, appid:{} | rank_key:{} | error:{}",
                    rank_table_config.appid,
                    rank_table_config.rank_key,
                    err.to_string()
                );
                return Option::None;
            }
        };
        match self.sched.add(job).await {
            Ok(uuid) => {
                return Some(uuid);
            }
            Err(err) => {
                tracing::error!(
                    "start_rank_cron_job add job, appid:{} | rank_key:{} | error:{}",
                    rank_table_config.appid,
                    rank_table_config.rank_key,
                    err.to_string()
                );
                return Option::None;
            }
        };
    }

    // 清理排行榜
    pub async fn clear_rank_data(&self, appid: String, rank_key: String) {
        match self
            .rank_repo
            .clear_all_users_score_info_from_mysql(&appid, &rank_key)
            .await
        {
            Ok(_) => {
                tracing::info!(
                    "clear_rank_data mysql success, appid:{} | rank_key:{}",
                    appid,
                    rank_key
                );
            }
            Err(err) => {
                tracing::info!(
                    "clear_rank_data mysql error, appid:{} | rank_key:{} | error:{}",
                    appid,
                    rank_key,
                    err.to_string()
                );
            }
        }
        match self
            .rank_repo
            .clear_all_users_score_info_from_redis(&appid, &rank_key)
            .await
        {
            Ok(_) => {
                tracing::info!(
                    "clear_rank_data redis success, appid:{} | rank_key{}",
                    appid,
                    rank_key
                );
            }
            Err(err) => {
                tracing::info!(
                    "clear_rank_data redis error, appid:{} | rank_key{} | error:{}",
                    appid,
                    rank_key,
                    err.to_string()
                );
            }
        }
    }

	/// 添加排行榜配置
    pub async fn add_rank_config(&self, payload: &AddRankConfigReq) -> Result<(), ApiError> {
        // 校验计划任务表达式是否正确
        use cron::Schedule;
        use std::str::FromStr;
        if let Err(_) = Schedule::from_str(&payload.cron_expression) {
            tracing::error!(
                "add_rank_config - cron expression is not valid:{}",
                payload.cron_expression
            );
            Err(RequestError::CommonError(
                "cron expression is not valid".to_string(),
            ))?
        }

        // 校验秘钥是否与现在的 appid 秘钥对应
        {
            let secret_map = self.rank_config_secret_map.read().await;
            if secret_map.contains_key(&payload.appid) {
                if secret_map[&payload.appid] != payload.app_secret {
                    return Err(RequestError::CommonError(
                        "appid and app secret is mismatching ".to_string(),
                    ))?;
                }
            }
        }

        // 校验配置是否已经存在
        {
            let guard = self.rank_table_configs.lock().unwrap();
            for rank_tbl_config in &(*guard) {
                if rank_tbl_config.appid == payload.appid
                    && rank_tbl_config.rank_key == payload.rank_key
                {
                    return Err(RequestError::CommonError(
                        "rank config has exist!! ".to_string(),
                    ))?;
                }
            }
        }

        match self.rank_repo.add_rank_config_to_mysql(payload).await {
            Ok(_) => {}
            Err(sqlx::Error::Database(err)) => match err.code() {
                Some(code) => {
                    if code == "1062" {
                        Err(DbError::UniqueConstraintViolation(err.to_string()))?
                    } else {
                        Err(DbError::SomethingWentWrong(err.to_string()))?
                    }
                }
                _ => Err(DbError::SomethingWentWrong(err.to_string()))?,
            },
            Err(err) => {
                tracing::error!("add_rank_config mysql error :{}", err.to_string());
                Err(DbError::SomethingWentWrong(err.to_string()))?
            }
        }

        let mut rank_table_config = RankTableConfig {
            appid: payload.appid.clone(),
            app_secret: payload.app_secret.clone(),
            rank_key: payload.rank_key.clone(),
            cron_expression: payload.cron_expression.clone(),
            cron_uuid: String::new(),
        };

        // 开启定时任务
        if let Some(uuid) = self.start_rank_cron_job(&rank_table_config).await {
            if !uuid.is_nil() {
                tracing::info!("add_rank_config - uuid:{}", uuid.to_string());
                rank_table_config.cron_uuid = uuid.to_string();
            }
			{
				let mut guard = self.rank_table_configs.lock().unwrap();
				let rank_table_configs: &mut Vec<RankTableConfig> = guard.as_mut();
				rank_table_configs.push(rank_table_config.clone());
			}
			// 添加秘钥映射
			{
				let mut secret_map = self.rank_config_secret_map.write().await;
				if (*secret_map).contains_key(&rank_table_config.appid) {
					secret_map.insert(rank_table_config.appid, rank_table_config.app_secret);
				} else {
					secret_map.insert(rank_table_config.appid, rank_table_config.app_secret);
				}
			}
			self.config_update_time.store(chrono::Utc::now().timestamp_millis() as u64,Ordering::Relaxed);
            Ok(())
        } else {
            tracing::error!(
                "add_rank_config sched start job error appid:{} | rank_key:{} | cron_exporess:{}",
                rank_table_config.appid,
                rank_table_config.rank_key,
                rank_table_config.cron_expression
            );
            Ok(())
        }
    }

	/// 删除排行榜配置
    pub async fn delete_rank_config(
        &self,
        appid: &String,
        rank_key: &String,
    ) -> Result<(), String> {
        let config;
        let mut appid_rank_total = 0;
        // 1. 最小锁范围 2. 跨线程变量安全
        {
            // 判断 是否是合法的
            let mut guard = self.rank_table_configs.lock().unwrap();
            let rank_table_configs: &mut Vec<RankTableConfig> = guard.as_mut();

            let mut idx = 0;
            let mut wait_delete_index = 0;
            let mut is_find = false;
            for rank_tbl_config in rank_table_configs {
                if rank_tbl_config.appid == *appid && rank_tbl_config.rank_key == *rank_key {
                    is_find = true;
                    wait_delete_index = idx;
                }
                if rank_tbl_config.appid == *appid {
                    appid_rank_total += 1;
                }
                idx += 1;
            }

            if !is_find {
                tracing::error!(
                    "delete_rank_config - appid:{} | rank_key:{} | list:{:?}",
                    appid,
                    rank_key,
                    self.rank_table_configs
                );
                return Err("rank config is not exist".to_string());
            }
            let rank_table_configs: &mut Vec<RankTableConfig> = guard.as_mut();
            config = rank_table_configs.remove(wait_delete_index);
        }
		self.config_update_time.store(chrono::Utc::now().timestamp_millis() as u64,Ordering::Relaxed);

        // 如果没有这个appid的所有排行榜就删除密钥映射
        {
            if appid_rank_total <= 1 {
                let mut secret_map = self.rank_config_secret_map.write().await;
                if (*secret_map).contains_key(&config.appid) {
                    (*secret_map).remove(&config.appid);
                }
            }
        }

        // 清理配置
        match self
            .rank_repo
            .delete_rank_config_from_mysql(&config.appid, &config.rank_key)
            .await
        {
            Ok(_) => {}
            Err(e) => {
                tracing::error!(
                    "delete_rank_config - mysql delete err:{} | appid:{} | rank_key:{}",
                    e.to_string(),
                    appid,
                    rank_key
                );
            }
        }

        // 先清理 数据库
        self.clear_rank_data(config.appid, config.rank_key).await;

        // 判断是否有定时任务
        if !config.cron_expression.is_empty() && config.cron_uuid.len() > 0 {
            tracing::info!("delete_rank_config - cancel sched");
            if let Ok(uuid) = Uuid::parse_str(config.cron_uuid.as_str()) {
                match self.sched.remove(&uuid).await {
                    Ok(_) => {
                        return Ok(());
                    }
                    Err(e) => {
                        tracing::error!(
							"delete_rank_config ,cancel sched uuid:{} | cron_expression:{} | err:{}",
							config.cron_uuid,
							config.cron_expression,
							e.to_string()
						);
                    }
                }
            } else {
                tracing::error!(
                    "delete_rank_config ,cancel sched uuid:{} | cron_expression:{}",
                    config.cron_uuid,
                    config.cron_expression
                );
                return Err("sched cancel error".to_string());
            }
        }
        Ok(())
    }

	/// 从主节点中获取到的新排行榜更新配置
    pub async fn update_rank_config_from_master_server(
        &self,
        update_time: u64,
        rank_tbl_configs: Vec<RankTableConfig>,
    ) {
        let curr_update_time = self.config_update_time.load(Ordering::Relaxed);
        if curr_update_time == update_time {
            return;
        }
        // 添加密钥映射
        {
            let mut secret_map = self.rank_config_secret_map.write().await;
            for config in &rank_tbl_configs {
                secret_map.insert(config.appid.clone(), config.app_secret.clone());
            }
        }
        {
            let mut configs_guard = self.rank_table_configs.lock().unwrap();
            configs_guard.clear();
            *configs_guard = rank_tbl_configs;
			tracing::info!("update_rank_config_from_master_server new : {:?}", *configs_guard);
        }
		self.config_update_time.store(update_time,Ordering::Relaxed);
    }
    /// ---更新计划任务id
    fn update_sched_uuid(&self, uuid: &String, appid: &String, rank_key: &String) {
        let mut guard = self.rank_table_configs.lock().unwrap();
        let rank_table_configs: &mut Vec<RankTableConfig> = guard.as_mut();
        for rank_tbl_config in rank_table_configs {
            if rank_tbl_config.appid == *appid && rank_tbl_config.rank_key == *rank_key {
                rank_tbl_config.cron_uuid = uuid.clone();
            }
        }
    }
}

/// 主节点更新排行榜的GRPC server
pub struct UpdateRankConfigGrpcServer {
    rank_config_service: Arc<RankConfigService>,
}

impl UpdateRankConfigGrpcServer {
    pub fn new(rank_config_service: Arc<RankConfigService>) -> Self {
        Self {
            rank_config_service,
        }
    }
}

#[tonic::async_trait]
impl update_rank_config::update_rank_config_server::UpdateRankConfig
    for UpdateRankConfigGrpcServer
{
    async fn get_last_update_time(
        &self,
        request: tonic::Request<update_rank_config::UpdataTimeRequest>,
    ) -> std::result::Result<tonic::Response<update_rank_config::UpdateTimeResponse>, tonic::Status>
    {
        tracing::debug!("get_last_update_time - Got a request from {:?}", request.remote_addr());

        let a = self
            .rank_config_service
            .config_update_time
            .load(Ordering::Relaxed);
        let reply = update_rank_config::UpdateTimeResponse {
            update_time: Some(a),
        };
        Ok(tonic::Response::new(reply))
    }
    async fn get_rank_table_config(
        &self,
        request: tonic::Request<update_rank_config::UpdataConfigRequest>,
    ) -> std::result::Result<tonic::Response<update_rank_config::UpdataConfigResponse>, tonic::Status>
    {
        let mut msg = update_rank_config::UpdataConfigResponse::default();
        // let list = msg.rank_table_configs.as_mut();
        tracing::debug!("get_rank_table_config - Got a request from {:?}", request.remote_addr());
        msg.update_time = self
            .rank_config_service
            .config_update_time
            .load(Ordering::Relaxed);
        {
            let guard = self.rank_config_service.rank_table_configs.lock().unwrap();

            for config in &(*guard) {
                msg.rank_table_configs
                    .push(update_rank_config::RankTableConfig {
                        appid: config.appid.clone(),
                        app_secret: config.app_secret.clone(),
                        rank_key: config.rank_key.clone(),
                        cron_expression: config.cron_expression.clone(),
                    });
            }
        }
        Ok(tonic::Response::new(msg))
    }
}

/// 更新排行榜配置GRPC client
pub struct UpdateRankConfigGrpcClient {
    rank_config_service: Arc<RankConfigService>,
    client: update_rank_config::update_rank_config_client::UpdateRankConfigClient<
        tonic::transport::Channel,
    >,
}

impl UpdateRankConfigGrpcClient {
    pub fn new(
        rank_config_service: Arc<RankConfigService>,
        client: update_rank_config::update_rank_config_client::UpdateRankConfigClient<
            tonic::transport::Channel,
        >,
    ) -> Self {
        Self {
            rank_config_service,
            client,
        }
    }

    pub async fn send_update_config_request(&mut self) {
        let request = tonic::Request::new(update_rank_config::UpdataTimeRequest {});

        let response = self.client.get_last_update_time(request).await;
        let mut is_update = false;
        match response {
            Ok(response) => {
                let resp = response.into_inner();
                if let Some(update_time) = resp.update_time {
                    let curr_update_time = self
                        .rank_config_service
                        .config_update_time
                        .load(Ordering::Relaxed);
                    // 有新的更新
                    if update_time != curr_update_time {
                        is_update = true;
                    }
                }
            }
            Err(e) => {
                tracing::error!("send_request - failed, error:{}", e.message());
            }
        }

        if !is_update {
            return;
        }
        let request = tonic::Request::new(update_rank_config::UpdataConfigRequest {});
        let response = self.client.get_rank_table_config(request).await;

        match response {
            Ok(response) => {
                let resp = response.into_inner();

                let mut list = vec![];
                for config in resp.rank_table_configs {
                    list.push(RankTableConfig {
                        appid: config.appid,
                        app_secret: config.app_secret,
                        rank_key: config.rank_key,
                        cron_expression: config.cron_expression,
                        cron_uuid: String::default(),
                    })
                }

                self.rank_config_service
                    .update_rank_config_from_master_server(resp.update_time, list)
                    .await;
            }
            Err(e) => {
                tracing::error!("get_rank_table_config - failed, error:{}", e.message());
            }
        }
    }
}
