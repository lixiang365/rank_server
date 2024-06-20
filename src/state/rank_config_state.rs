use crate::db::database::Database;
use crate::repository::rank_repository::{RankRepository, RankRepositoryTrait};
use crate::service::rank_config_service::RankConfigService;
use deadpool_redis::Pool;
use std::sync::Arc;

#[derive(Clone)]
pub struct RankConfigState {
    pub rank_config_service: Arc<RankConfigService>,
    pub rank_repo: RankRepository,
}

impl RankConfigState {
    pub fn new(
        db_conn: &Arc<Database>,
        redis_con_pool: &Pool,
        rank_config_service: &Arc<RankConfigService>,
    ) -> Self {
        Self {
            rank_config_service: rank_config_service.clone(),
            rank_repo: RankRepository::new(db_conn, redis_con_pool),
        }
    }
}
