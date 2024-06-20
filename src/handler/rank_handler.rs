use std::collections::HashMap;

use crate::dto::rank_dto::{
    AddRankConfigReq, TopNUserReq, UpdateScoreRequest, UserRankingReq, UserScoreRes,
};

use crate::error::{api_error::ApiError, request_error::ValidatedRequest};
use crate::response::api_response::{ApiErrorResponse, ApiSuccessResponse};
use crate::state::rank_config_state::RankConfigState;
use crate::state::rank_state::RankState;
use axum::{
    extract::{Query, State},
    Json,
};
use axum_macros::debug_handler;

pub async fn update_rank_score(
    State(state): State<RankState>,
    ValidatedRequest(payload): ValidatedRequest<UpdateScoreRequest>,
) -> Result<Json<ApiSuccessResponse<()>>, ApiError> {
    state.rank_service.update_rank_score(payload).await?;
    Ok(Json(ApiSuccessResponse::from_with_nodata()))
}

pub async fn get_user_rank(
    State(state): State<RankState>,
    ValidatedRequest(payload): ValidatedRequest<UserRankingReq>,
) -> Result<Json<ApiSuccessResponse<UserScoreRes>>, ApiError> {
    let res = state
        .rank_service
        .get_user_ranking(&payload.appid, &payload.openid, &payload.rank_key)
        .await?;
    Ok(Json(ApiSuccessResponse::send(UserScoreRes {
        ranking: Some(res),
        ..Default::default()
    })))
}

pub async fn get_user_score(
    State(state): State<RankState>,
    ValidatedRequest(payload): ValidatedRequest<UserRankingReq>,
) -> Result<Json<ApiSuccessResponse<UserScoreRes>>, ApiError> {
    let score = state
        .rank_service
        .get_user_score(&payload.appid, &payload.openid, &payload.rank_key)
        .await?;
    Ok(Json(ApiSuccessResponse::send(UserScoreRes {
        score: Some(score),
        ..Default::default()
    })))
}

pub async fn get_top_user_rank(
    State(state): State<RankState>,
    ValidatedRequest(payload): ValidatedRequest<TopNUserReq>,
) -> Result<Json<ApiSuccessResponse<Vec<UserScoreRes>>>, ApiError> {
    let users = state
        .rank_service
        .get_top_user_rank(&payload.appid, &payload.rank_key, payload.top_n)
        .await?;
    Ok(Json(ApiSuccessResponse::send(users)))
}

// 添加排行榜
#[debug_handler]
pub async fn add_rank_config(
    State(state): State<RankConfigState>,
    ValidatedRequest(payload): ValidatedRequest<AddRankConfigReq>,
) -> Result<Json<ApiSuccessResponse<()>>, ApiError> {
    state.rank_config_service.add_rank_config(&payload).await?;
    Ok(Json(ApiSuccessResponse::from_with_nodata()))
}

// 删除排行榜
#[debug_handler]
pub async fn delete_rank_config(
    State(state): State<RankConfigState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<ApiSuccessResponse<()>>, ApiErrorResponse> {
    if !params.contains_key("appid") || !params.contains_key("rank_key") {
        return Err(ApiErrorResponse::new(
            200,
            20004,
            Some("params is error".to_string()),
        ));
    }
    match state
        .rank_config_service
        .delete_rank_config(&params["appid"], &params["rank_key"])
        .await
    {
        Ok(_) => Ok(Json(ApiSuccessResponse::from_with_nodata())),
        Err(e) => Err(ApiErrorResponse::new(200, 20004, Some(e))),
    }
}
