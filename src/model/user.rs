//! 内部用到的数据模型
//!
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize, sqlx::FromRow)]
pub struct User {
    pub id: i32,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub user_name: String,
    pub email: String,
    pub password: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub is_active: i8,
}

/// 数据库存储的排行榜分数
#[derive(Clone, Deserialize, Serialize, sqlx::FromRow)]
pub struct UserScoreInfo {
    #[sqlx(skip)]
    pub appid: String,
    pub openid: String,
    pub nick_name: String,
    pub score: i32,
}

// 数据库存储的排行榜配置
#[derive(Clone, Debug, Deserialize, Serialize, sqlx::FromRow)]
pub struct RankTableConfig {
    pub appid: String,
    pub rank_key: String,
    pub app_secret: String,
    // 计划任务表达式
    pub cron_expression: String,
    #[sqlx(skip)]
    pub cron_uuid: String,
}

impl Default for RankTableConfig {
    fn default() -> Self {
        Self {
            appid: Default::default(),
            app_secret: Default::default(),
            rank_key: Default::default(),
            cron_expression: Default::default(),
            cron_uuid: Default::default(),
        }
    }
}
