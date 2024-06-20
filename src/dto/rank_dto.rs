//! 排行榜传输用到的数据结构
//!
//!

use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Clone, Serialize, Deserialize, Validate)]
pub struct UpdateScoreRequest {
    #[validate(length(
        min = 3,
        max = 64,
        message = "appid must be between 3 and 64 characters"
    ))]
    pub appid: String,
    #[validate(length(
        min = 3,
        max = 20,
        message = "openid must be between 3 and 20 characters"
    ))]
    pub rank_key: String,
    #[validate(length(min = 3, max = 64, message = "key must be between 3 and 64 characters"))]
    pub openid: String,
    pub nick_name: String,
    #[validate(range(
        min = 0,
        max = 100_000_000,
        message = "score must be between 0 and 100_000_000"
    ))]
    pub score: i32,
}

#[derive(Clone, Serialize)]
pub struct UserScoreRes {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub openid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nick_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ranking: Option<i32>,
}

impl Default for UserScoreRes {
    fn default() -> Self {
        Self {
            openid: Default::default(),
            nick_name: Default::default(),
            score: Default::default(),
            ranking: Default::default(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Validate)]
pub struct UserRankingReq {
    #[validate(length(
        min = 3,
        max = 64,
        message = "appid must be between 3 and 64 characters"
    ))]
    pub appid: String,
    #[validate(length(
        min = 3,
        max = 64,
        message = "openid must be between 3 and 64 characters"
    ))]
    pub openid: String,
    #[validate(length(
        min = 3,
        max = 20,
        message = "rank_key must be between 3 and 20 characters"
    ))]
    pub rank_key: String,
}

#[derive(Clone, Deserialize, Validate)]
pub struct TopNUserReq {
    #[validate(length(
        min = 3,
        max = 64,
        message = "appid must be between 3 and 64 characters"
    ))]
    pub appid: String,
    #[validate(length(
        min = 3,
        max = 20,
        message = "rank_key must be between 1 and 20 characters"
    ))]
    pub rank_key: String,
    #[validate(range(min = 1, max = 30, message = "top_n must be between 1 and 30"))]
    pub top_n: i32,
}

#[derive(Clone, Deserialize, Validate)]
pub struct AddRankConfigReq {
    #[validate(length(
        min = 3,
        max = 64,
        message = "appid must be between 3 and 64 characters"
    ))]
    pub appid: String,
    #[validate(length(
        min = 1,
        max = 64,
        message = "rank_key must be between 1 and 64 characters"
    ))]
    pub rank_key: String,
    #[validate(length(
        min = 8,
        max = 64,
        message = "app_secret must be between 8 and 64 characters"
    ))]
    pub app_secret: String,
    #[validate(length(
        min = 0,
        max = 200,
        message = "cron_expression must be between 1 and 200 characters"
    ))]
    pub cron_expression: String,
    #[validate(length(
        min = 0,
        max = 200,
        message = "remark must be between 1 and 200 characters"
    ))]
    pub remark: String,
}
