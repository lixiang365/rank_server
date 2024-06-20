use crate::db::database::Database;
use crate::dto::rank_dto::{UpdateScoreRequest, UserScoreRes};
use crate::error::api_error::ApiError;
use crate::error::db_error::DbError;
use crate::model::user::RankTableConfig;
use crate::repository::rank_repository::{RankRepository, RankRepositoryTrait};
use deadpool_redis::Pool;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct RankService {
    rank_repo: RankRepository,
    db_conn: Arc<Database>,
    rank_table_configs: Arc<Mutex<Vec<RankTableConfig>>>,
}

impl RankService {
    pub fn new(
        db_conn: &Arc<Database>,
        redis_con_pool: &Pool,
        rank_table_configs: &Arc<Mutex<Vec<RankTableConfig>>>,
    ) -> Self {
        Self {
            rank_repo: RankRepository::new(db_conn, redis_con_pool),
            db_conn: Arc::clone(db_conn),
            rank_table_configs: Arc::clone(rank_table_configs),
        }
    }

    pub async fn test_db(&self) -> Result<String, ApiError> {
        let _ = self.rank_repo.test_redis().await;
        Ok("ok".to_string())
    }

    pub async fn update_rank_score(&self, payload: UpdateScoreRequest) -> Result<(), ApiError> {
        // 更新到mysql
        match self.rank_repo.update_rank_score_to_mysql(&payload).await {
            // 更新到redis
            Ok(_) => match self.rank_repo.update_rank_score_to_redis(&payload).await {
                Ok(_) => match self.rank_repo.update_user_info_to_redis(&payload).await {
                    Ok(_) => Ok(()),
                    Err(err) => {
                        tracing::error!("update user info to redis error :{}", err.to_string());
                        Err(DbError::SomethingWentWrong(err.to_string()))?
                    }
                },
                Err(err) => {
                    // TODO@ 当更新排行榜出错后需要重试，或者记录错误信息到本地
                    tracing::error!("update score to redis error :{}", err.to_string());
                    Err(DbError::SomethingWentWrong(err.to_string()))?
                }
            },
            Err(err) => {
                tracing::error!("update score to mysql error :{}", err.to_string());
                Err(DbError::SomethingWentWrong(err.to_string()))?
            }
        }
    }

    pub async fn get_user_score(
        &self,
        appid: &String,
        openid: &String,
        rank_key: &String,
    ) -> Result<i32, ApiError> {
        match self
            .rank_repo
            .get_user_score_from_redis(appid, openid, rank_key)
            .await
        {
            Ok(score) => {
                if score != -1 {
                    Ok(score)
                } else {
                    match self
                        .rank_repo
                        .get_user_score_info_from_mysql(appid, openid, rank_key)
                        .await
                    {
                        Ok(user_info) => {
                            let _ = self
                                .rank_repo
                                .update_rank_score_to_redis(&UpdateScoreRequest {
                                    appid: appid.clone(),
                                    openid: openid.clone(),
                                    rank_key: rank_key.clone(),
                                    score: user_info.score,
                                    nick_name: user_info.nick_name,
                                })
                                .await;
                            Ok(user_info.score)
                        }
                        Err(sqlx::Error::RowNotFound) => {
                            tracing::error!(
                                "get score from mysql error :{}",
                                sqlx::Error::RowNotFound.to_string()
                            );
                            Err(DbError::SomethingWentWrong(
                                "openid is not exist".to_string(),
                            ))?
                        }
                        Err(err) => {
                            tracing::error!("get score from mysql error2 :{}", err.to_string());
                            Err(DbError::SomethingWentWrong(err.to_string()))?
                        }
                    }
                }
            }
            Err(err) => {
                tracing::error!("get user score to redis error :{}", err.to_string());
                Err(DbError::SomethingWentWrong(err.to_string()))?
            }
        }
    }

    pub async fn get_user_ranking(
        &self,
        appid: &String,
        openid: &String,
        rank_key: &String,
    ) -> Result<i32, ApiError> {
        match self
            .rank_repo
            .get_user_ranking(appid, openid, rank_key)
            .await
        {
            Ok(ranking) => Ok(ranking),
            Err(err) => {
                tracing::error!("get user ranking from redis error :{}", err.to_string());
                Err(DbError::SomethingWentWrong(err.to_string()))?
            }
        }
    }

    pub async fn get_top_user_rank(
        &self,
        appid: &String,
        rank_type_key: &String,
        top: i32,
    ) -> Result<Vec<UserScoreRes>, ApiError> {
        match self
            .rank_repo
            .get_top_user_rank(appid, rank_type_key, top)
            .await
        {
            Ok(users) => {
                let count = users.len();
                let mut i = 0;
                let mut rank = 1;
                let mut res = vec![];
                while i < count {
                    // 转化分数
                    let score_f: f64 = users[i + 1].parse().unwrap();
                    let score = score_f as i32;
                    let user = UserScoreRes {
                        openid: Some(users[i].clone()),
                        ranking: Some(rank),
                        score: Some(score),
                        nick_name: None,
                    };
                    res.push(user);
                    rank += 1;
                    i += 2;
                }

                for user in &mut res {
                    match self
                        .rank_repo
                        .get_user_info_from_redis(appid, &user.openid.clone().unwrap())
                        .await
                    {
                        Ok(nick_name) => user.nick_name = Some(nick_name),
                        Err(err) => {
                            tracing::error!(" user name find failed, error: {}", err.to_string());
                            user.nick_name = Some("momo".to_string())
                        }
                    }
                }
                Ok(res)
            }
            Err(err) => {
                tracing::error!("get top user ranking from redis error :{}", err.to_string());
                Err(DbError::SomethingWentWrong(err.to_string()))?
            }
        }
    }
}
