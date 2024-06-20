use crate::db::database::Database;
use crate::repository::rank_repository::{RankRepository, RankRepositoryTrait};
use crate::service::rank_config_service::RankConfigService;
use crate::service::rank_service::RankService;
use deadpool_redis::Pool;
use std::sync::Arc;

#[derive(Clone)]
pub struct RankState {
    pub rank_service: Arc<RankService>,
    pub rank_repo: RankRepository,
}

impl RankState {
    pub fn new(
        db_conn: &Arc<Database>,
        redis_con_pool: &Pool,
        rank_config_service: &Arc<RankConfigService>,
    ) -> Self {
        Self {
            rank_service: Arc::new(RankService::new(
                db_conn,
                redis_con_pool,
                &rank_config_service.rank_table_configs,
            )),
            rank_repo: RankRepository::new(db_conn, redis_con_pool),
        }
    }
}
