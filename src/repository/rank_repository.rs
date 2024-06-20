use crate::config::parameter::CALC_SCORE_BASE_TIME_STAMP;
use crate::db::database::{Database, DatabaseTrait};
use async_trait::async_trait;
use deadpool_redis::{Pool, PoolError};
// use sqlx::Error;
use redis::cmd;
use std::sync::Arc;

use crate::dto::rank_dto::{AddRankConfigReq, UpdateScoreRequest};
use crate::model::user::{RankTableConfig, UserScoreInfo};
use chrono::Utc;
#[derive(Clone)]
pub struct RankRepository {
    /// 主从分离
    pub(crate) db_conn: Arc<Database>,
    pub(crate) redis_con_pool: Pool,
}

#[async_trait]
pub trait RankRepositoryTrait {
    fn new(
        db_conn: &Arc<Database>,
        redis_con_pool: &Pool,
    ) -> Self;

    async fn test_redis(&self) -> Result<(), String>;

    /// 更新用户分数
    async fn update_rank_score_to_mysql(
        &self,
        payload: &UpdateScoreRequest,
    ) -> Result<(), sqlx::Error>;

    /// 获取用户分数
    async fn get_user_score_info_from_mysql(
        &self,
        appid: &String,
        openid: &String,
        rank_key: &String,
    ) -> Result<UserScoreInfo, sqlx::Error>;

    /// 更新用户分数
    async fn update_rank_score_to_redis(
        &self,
        payload: &UpdateScoreRequest,
    ) -> Result<(), PoolError>;

    /// 用户信息写入redis
    async fn update_user_info_to_redis(
        &self,
        payload: &UpdateScoreRequest,
    ) -> Result<(), PoolError>;

    /// 从redis获取用户信息
    async fn get_user_info_from_redis(
        &self,
        appid: &String,
        openid: &String,
    ) -> Result<String, PoolError>;

    /// 获取用户分数
    async fn get_user_score_from_redis(
        &self,
        appid: &String,
        openid: &String,
        rank_key: &String,
    ) -> Result<i32, PoolError>;

    /// 获取用户排名
    async fn get_user_ranking(
        &self,
        appid: &String,
        openid: &String,
        rank_key: &String,
    ) -> Result<i32, PoolError>;

    /// 获取top用户
    ///
    async fn get_top_user_rank(
        &self,
        appid: &String,
        rank_key: &String,
        top: i32,
    ) -> Result<Vec<String>, PoolError>;

    /// 获取排行榜表配置
    async fn get_rank_table_config_from_mysql(&self) -> Result<Vec<RankTableConfig>, sqlx::Error>;

    /// 分页获取用户信息
    async fn get_pagination_all_users_score_info_from_mysql(
        &self,
        appid: &String,
        rank_key: &String,
        start_index: u64,
        page_size: u64,
    ) -> Result<Vec<UserScoreInfo>, sqlx::Error>;

    /// 清理mysql排行榜数据
    async fn clear_all_users_score_info_from_mysql(
        &self,
        appid: &String,
        rank_key: &String,
    ) -> Result<(), sqlx::Error>;

    /// 清理redis排行榜数据
    ///
    async fn clear_all_users_score_info_from_redis(
        &self,
        appid: &String,
        rank_key: &String,
    ) -> Result<(), PoolError>;

    /// 添加排行榜配置到mysql
    async fn add_rank_config_to_mysql(&self, payload: &AddRankConfigReq)
        -> Result<(), sqlx::Error>;

    /// 动态创建rank表
    async fn create_rank_table_to_mysql(
        &self,
        payload: &AddRankConfigReq,
    ) -> Result<(), sqlx::Error>;

    /// 删除排行榜配置到mysql
    async fn delete_rank_config_from_mysql(
        &self,
        appid: &String,
        rank_key: &String,
    ) -> Result<(), sqlx::Error>;

}

#[async_trait]
impl RankRepositoryTrait for RankRepository {
    fn new(
        db_conn: &Arc<Database>,
        redis_con_pool: &Pool,
    ) -> Self {
        Self {
            db_conn: Arc::clone(db_conn),
            redis_con_pool: redis_con_pool.clone(),
        }
    }

    // test
    async fn test_redis(&self) -> Result<(), String> {
        let pool_res = self.redis_con_pool.get().await;
        tracing::debug!("test_redis");

        match pool_res {
            Ok(mut pool) => {
                let res = cmd("SET")
                    .arg("test_redis_key")
                    .arg("value")
                    .query_async::<_, ()>(&mut pool)
                    .await;
                match res {
                    Err(err) => {
                        tracing::debug!("error-----{:?}", err.to_string());
                    }
                    _ => {
                        tracing::debug!("success");
                    }
                }
            }
            Err(err) => {
                tracing::debug!("----------{:?}", err.to_string());
            }
        }
        Ok(())
    }

    // 更新分数到mysql
    async fn update_rank_score_to_mysql(
        &self,
        payload: &UpdateScoreRequest,
    ) -> Result<(), sqlx::Error> {
        let table_name = get_mysql_table_name(&payload.appid, &payload.rank_key);
        let sql = format!(
            "INSERT INTO {} (
			openid,
			nick_name,
			score
		)
		VALUES('{}','{}',{}) 
		ON DUPLICATE KEY
		UPDATE nick_name='{}',score={};",
            table_name,
            payload.openid,
            payload.nick_name,
            payload.score,
            payload.nick_name,
            payload.score
        );
        let sql_ret = sqlx::query(&sql)
            .execute(self.db_conn.get_master_pool())
            .await?;
        // 成功
        tracing::debug!("set_user_score - rows_affected:{}", sql_ret.rows_affected());
        Ok(())
    }

    async fn get_user_score_info_from_mysql(
        &self,
        appid: &String,
        openid: &String,
        rank_key: &String,
    ) -> Result<UserScoreInfo, sqlx::Error> {
        let table_name = get_mysql_table_name(appid, rank_key);
        let sql = format!("SELECT * FROM {table_name} WHERE openid = ?");
        let user = sqlx::query_as::<_, UserScoreInfo>(&sql)
            .bind(openid)
            .fetch_one(self.db_conn.get_slave_pool())
            .await;
        return user;
    }

    // 更新分数到redis
    async fn update_rank_score_to_redis(
        &self,
        payload: &UpdateScoreRequest,
    ) -> Result<(), PoolError> {
        let mut con = self.redis_con_pool.get().await?;
        let score = calc_score(payload.score);

        let mut cmd_pipe = redis::pipe();
        let key = get_redis_rank_key(&payload.appid, &payload.rank_key);
        cmd_pipe
            .cmd("ZADD")
            .arg(key)
            .arg(score)
            .arg(&payload.openid);

        let _ = cmd_pipe.query_async(&mut con).await?;
        Ok(())
    }

    // 更新用户信息到redis
    async fn update_user_info_to_redis(
        &self,
        payload: &UpdateScoreRequest,
    ) -> Result<(), PoolError> {
        let mut con = self.redis_con_pool.get().await?;
        let key = get_redis_user_key(&payload.appid);

        let _ = redis::cmd("HSET")
            .arg(key)
            .arg(&payload.openid)
            .arg(&payload.nick_name)
            .query_async(&mut con)
            .await?;
        Ok(())
    }

    async fn get_user_info_from_redis(
        &self,
        appid: &String,
        openid: &String,
    ) -> Result<String, PoolError> {
        let mut con = self.redis_con_pool.get().await?;
        let key = get_redis_user_key(appid);
        let nick_name: String = redis::cmd("HGET")
            .arg(key)
            .arg(openid)
            .query_async(&mut con)
            .await?;
        Ok(nick_name)
    }

    // 获取用户分数
    async fn get_user_score_from_redis(
        &self,
        appid: &String,
        openid: &String,
        rank_key: &String,
    ) -> Result<i32, PoolError> {
        let mut con = self.redis_con_pool.get().await?;

        let key = get_redis_rank_key(appid, rank_key);
        let cur_score: Option<String> = redis::cmd("ZSCORE")
            .arg(key.clone())
            .arg(openid.clone())
            .query_async(&mut con)
            .await?;
        if cur_score.is_some() {
            let score_f: f64 = cur_score.unwrap().parse().unwrap();
            Ok(score_f as i32)
        } else {
            Ok(-1)
        }
    }

    /// 获取用户排名
    async fn get_user_ranking(
        &self,
        appid: &String,
        openid: &String,
        rank_key: &String,
    ) -> Result<i32, PoolError> {
        let mut con = self.redis_con_pool.get().await?;
        let key = get_redis_rank_key(appid, rank_key);

        let cur_ranking: Option<i32> = redis::cmd("ZREVRANK")
            .arg(key.clone())
            .arg(openid.clone())
            .query_async(&mut con)
            .await?;
        // 0 未上榜
        match cur_ranking {
            Some(ranking) => Ok(ranking + 1),
            None => Ok(0),
        }
    }

    /// 获取top用户
    async fn get_top_user_rank(
        &self,
        appid: &String,
        rank_key: &String,
        top: i32,
    ) -> Result<Vec<String>, PoolError> {
        if top <= 0 {
            tracing::error!("top <= 0 is error!!");
            return Ok(vec![]);
        }
        let mut con = self.redis_con_pool.get().await?;
        let key = get_redis_rank_key(appid, rank_key);
        let users: Vec<String> = redis::cmd("ZREVRANGE")
            .arg(key.clone())
            .arg(0)
            .arg(top - 1)
            .arg("WITHSCORES")
            .query_async(&mut con)
            .await?;
        Ok(users)
    }

    /// 获取排行榜表配置
    async fn get_rank_table_config_from_mysql(&self) -> Result<Vec<RankTableConfig>, sqlx::Error> {
        let table_name = "rank_table_config";
        let sql = format!("SELECT * FROM {table_name}");
        let rank_config = sqlx::query_as::<_, RankTableConfig>(&sql)
            .fetch_all(self.db_conn.get_master_pool())
            .await?;
        Ok(rank_config)
    }

    /// 分页获取用户信息
    async fn get_pagination_all_users_score_info_from_mysql(
        &self,
        appid: &String,
        rank_key: &String,
        start_index: u64,
        page_size: u64,
    ) -> Result<Vec<UserScoreInfo>, sqlx::Error> {
        let table_name = get_mysql_table_name(appid, rank_key);
        let sql = format!("SELECT * FROM {table_name} LIMIT ?,?");
        let user = sqlx::query_as::<_, UserScoreInfo>(&sql)
            .bind(start_index)
            .bind(page_size)
            .fetch_all(self.db_conn.get_slave_pool())
            .await?;
        Ok(user)
    }

    /// 清理mysql排行榜数据
    async fn clear_all_users_score_info_from_mysql(
        &self,
        appid: &String,
        rank_key: &String,
    ) -> Result<(), sqlx::Error> {
        let table_name = get_mysql_table_name(appid, rank_key);
        let sql = format!("DELETE FROM {table_name}");
        sqlx::query(&sql)
            .execute(self.db_conn.get_master_pool())
            .await?;
        Ok(())
    }

    /// 清理redis排行榜数据
    ///
    async fn clear_all_users_score_info_from_redis(
        &self,
        appid: &String,
        rank_key: &String,
    ) -> Result<(), PoolError> {
        let mut con = self.redis_con_pool.get().await?;

        let mut cmd_pipe = redis::pipe();
        let key = get_redis_rank_key(appid, rank_key);
        cmd_pipe.cmd("DEL").arg(key);
        let _ = cmd_pipe.query_async(&mut con).await?;
        Ok(())
    }

    // 添加排行榜配置到mysql
    async fn add_rank_config_to_mysql(
        &self,
        payload: &AddRankConfigReq,
    ) -> Result<(), sqlx::Error> {
        // 开启事务
        sqlx::raw_sql("START TRANSACTION;")
            .execute(self.db_conn.get_master_pool())
            .await?;

        let table_name = "rank_table_config";
        let sql = format!(
            "INSERT INTO {} 
			VALUES('{}','{}','{}','{}','{}')",
            table_name,
            payload.appid,
            payload.app_secret,
            payload.rank_key,
            payload.cron_expression,
            payload.remark
        );
        match sqlx::query(&sql)
            .execute(self.db_conn.get_master_pool())
            .await
        {
            Ok(_) => match self.create_rank_table_to_mysql(payload).await {
                Ok(_) => {
                    sqlx::raw_sql("COMMIT;")
                        .execute(self.db_conn.get_master_pool())
                        .await?;
                    Ok(())
                }
                Err(err) => {
                    sqlx::raw_sql("ROLLBACK;")
                        .execute(self.db_conn.get_master_pool())
                        .await?;
                    tracing::error!("add_rank_config_to_mysql 1- err:{}", err.to_string());
                    Err(err)
                }
            },
            Err(err) => {
                sqlx::raw_sql("ROLLBACK;")
                    .execute(self.db_conn.get_master_pool())
                    .await?;
                tracing::error!("add_rank_config_to_mysql 2- err:{}", err.to_string());
                Err(err)
            }
        }
    }

    // 添加排行榜配置到mysql
    async fn create_rank_table_to_mysql(
        &self,
        payload: &AddRankConfigReq,
    ) -> Result<(), sqlx::Error> {
        let sql = format!(
            "call CREATE_RANK_TABLE('{}','{}');",
            payload.appid, payload.rank_key
        );
        sqlx::raw_sql(&sql)
            .execute(self.db_conn.get_master_pool())
            .await?;
        Ok(())
    }

    // 删除排行榜配置到mysql
    async fn delete_rank_config_from_mysql(
        &self,
        appid: &String,
        rank_key: &String,
    ) -> Result<(), sqlx::Error> {
        let table_name = "rank_table_config";
        let sql = format!(
            "DELETE FROM {} WHERE appid='{}' AND rank_key='{}'",
            table_name, appid, rank_key
        );
        match sqlx::query(&sql)
            .execute(self.db_conn.get_master_pool())
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => {
                tracing::error!("delete_rank_config_from_mysql - delete config err, appid :{} | rank_key:{} | err:{}",appid,rank_key,e.to_string());
                Err(e)
            }
        }
    }

    // async fn set_master_rank_service_flag_to_redis(
    //     &self,
    // ) -> Result<(), PoolError>{
    // 	let mut con = self.redis_con_pool.get().await?;

    //     let key = "rank_server_master";
    //     let cur_score: Option<String> = redis::cmd("ZSCORE")
    //         .arg(key.clone())
    //         .arg(openid.clone())
    //         .query_async(&mut con)
    //         .await?;
    //     if cur_score.is_some() {
    //         let score_f: f64 = cur_score.unwrap().parse().unwrap();
    //         Ok(score_f as i32)
    //     } else {
    //         Ok(-1)
    //     }
    // }
}

// 计算分数
fn calc_score(origin_score: i32) -> f64 {
    let cur_time_seconds = Utc::now().timestamp();
    let score = origin_score as f64
        + (CALC_SCORE_BASE_TIME_STAMP - cur_time_seconds as f64) / CALC_SCORE_BASE_TIME_STAMP;

    tracing::debug!("socre:{}", score);
    score
}

/// 获取mysql表名
fn get_mysql_table_name(appid: &String, rank_key: &String) -> String {
    format!("rank_{}_{}", appid, rank_key)
}

/// 获取redis 排行榜的key
fn get_redis_rank_key(appid: &String, rank_key: &str) -> String {
    format!("rank:{appid}:{rank_key}")
}

/// 获取redis 用户信息的key
fn get_redis_user_key(appid: &String) -> String {
    format!("userinfo:{appid}")
}
