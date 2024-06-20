use super::auth;
use crate::db::axredis::get_redis_connect_pool;
use crate::db::database::Database;
use crate::middleware::auth as auth_middleware;
use crate::middleware::body_signature::body_signature_verify;
use crate::service::rank_config_service::RankConfigService;

use crate::routes::{profile, rank, register};
use crate::state::auth_state::AuthState;
use crate::state::rank_config_state::RankConfigState;
use crate::state::rank_state::RankState;
use crate::state::token_state::TokenState;
use crate::state::user_state::UserState;
use axum::routing::{get, IntoMakeService};
use axum::{extract::State, middleware, Router};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;

pub fn routes(
    db_conn: Arc<Database>,
    rank_config_service: Arc<RankConfigService>,
) -> IntoMakeService<Router> {
    let merged_router = {
        let auth_state = AuthState::new(&db_conn);
        let user_state = UserState::new(&db_conn);
        let token_state = TokenState::new(&db_conn);
        let rank_config_state = RankConfigState::new(
            &db_conn,
            get_redis_connect_pool().unwrap(),
            &rank_config_service,
        );
        let rank_state = RankState::new(
            &db_conn,
            get_redis_connect_pool().unwrap(),
            &rank_config_service,
        );

        let mut router = Router::new()
            .merge(rank::routes().with_state(rank_state.clone()).layer(
                middleware::from_fn_with_state(rank_config_state.clone(), body_signature_verify),
            ))
            .merge(Router::new().route("/health", get(|| async move { "Healthy..." })));
        // 主服务开启排行榜配置管理
        if rank_config_service.master_node {
            router = router
                .nest(
                    "/user",
                    auth::routes()
                        .with_state(auth_state)
                        .merge(register::routes().with_state(user_state))
                        .merge(profile::routes().layer(ServiceBuilder::new().layer(
                            middleware::from_fn_with_state(token_state, auth_middleware::auth),
                        ))),
                )
                .merge(rank::rank_config_routes().with_state(rank_config_state.clone()));
        }
        router
    };

    let app_router = Router::new()
        .nest("/api", merged_router)
        .layer(TraceLayer::new_for_http());

    app_router.into_make_service()
}
