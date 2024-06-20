use crate::parameter;
use async_trait::async_trait;
use sqlx::mysql::MySqlPoolOptions;
use sqlx::{Error, MySql, Pool};

pub struct Database {
    master_pool: Pool<MySql>,
    slave_pool: Pool<MySql>,
}

#[async_trait]
pub trait DatabaseTrait {
    async fn init() -> Result<Self, Error>
    where
        Self: Sized;
    fn get_master_pool(&self) -> &Pool<MySql>;
    fn get_slave_pool(&self) -> &Pool<MySql>;
}

#[async_trait]
impl DatabaseTrait for Database {
    async fn init() -> Result<Self, Error> {
        let database_url = parameter::get("MASTER_DB_URL");
        // let pool = MySqlPool::connect(&database_url).await?;
        let master_pool = MySqlPoolOptions::new()
            .max_connections(10)
            .max_lifetime(std::time::Duration::from_secs(6 * 60 * 60))
            .connect(&database_url)
            .await?;
        let slave_database_url = parameter::get("SLAVE_DB_URL");
        // let pool = MySqlPool::connect(&database_url).await?;
        let slave_pool = MySqlPoolOptions::new()
            .max_connections(10)
            .max_lifetime(std::time::Duration::from_secs(6 * 60 * 60))
            .connect(&slave_database_url)
            .await?;

        Ok(Self {
            master_pool,
            slave_pool,
        })
    }

    fn get_master_pool(&self) -> &Pool<MySql> {
        &self.master_pool
    }

    fn get_slave_pool(&self) -> &Pool<MySql> {
        &self.slave_pool
    }
}
