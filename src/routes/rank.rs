use crate::{handler::rank_handler, state::{rank_state::RankState,rank_config_state::RankConfigState}};
use axum::{
    routing::{delete, post},
    Router,
};
pub fn routes() -> Router<RankState> {
    // let rank_state = RankState::new(&db_conn, get_redis_connect_pool().unwrap());

    let router = Router::new().nest(
        "/rank",
        Router::new()
            .route("/update_score", post(rank_handler::update_rank_score))
            .route("/get_user_rank", post(rank_handler::get_user_rank))
            .route("/get_user_score", post(rank_handler::get_user_score)) // .layer(middleware::from_fn(body_signature_verify));
            .route("/get_top_user_rank", post(rank_handler::get_top_user_rank)),
    );
    return router;
}

pub fn rank_config_routes() -> Router<RankConfigState> {
    let router = Router::new().nest(
        "/rank",
        Router::new()
            .route("/add_rank_config", post(rank_handler::add_rank_config))
            .route(
                "/delete_rank_config",
                delete(rank_handler::delete_rank_config),
            ),
    );
    return router;
}
