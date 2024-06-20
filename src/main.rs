use crate::config::parameter;
use crate::db::axredis::get_redis_connect_pool;
use crate::db::{
    axredis,
    database::{self, DatabaseTrait},
};
use crate::service::rank_config_service::RankConfigService;

use tokio_cron_scheduler::JobScheduler;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::fmt::time;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use std::sync::Arc;

mod config;
mod db;
mod dto;
mod error;
mod handler;
mod middleware;
mod model;
mod pb;
mod repository;
mod response;
mod routes;
mod service;
mod state;
mod utils;

// 内存分配器
#[cfg(not(target_env = "msvc"))]
use tikv_jemallocator::Jemalloc;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

#[cfg(target_env = "msvc")]
use mimalloc::MiMalloc;

#[cfg(target_env = "msvc")]
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[tokio::main]
async fn main() {
    // 参数初始化
    parameter::init();

    let connection = database::Database::init()
        .await
        .unwrap_or_else(|e| panic!("Database error: {}", e.to_string()));
    let mysql_pool = Arc::new(connection);

    // 初始化redi
    axredis::init_redis_pool()
        .await
        .unwrap_or_else(|err| panic!("redis init failed, error:{}", err.to_string()));

    // 日志
    let file_appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix("logger")
        .filename_suffix("log")
        .max_log_files(60)
        .build("log")
        .expect("file log init failed!");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    let file_log_subscriber = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_timer(time::LocalTime::rfc_3339());

    let console_subscriber = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stdout)
        .with_timer(time::LocalTime::rfc_3339());
    tracing_subscriber::registry()
        .with(file_log_subscriber)
        .with(console_subscriber)
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let host = format!("0.0.0.0:{}", parameter::get("PORT"));
    let listener = tokio::net::TcpListener::bind(host).await.unwrap();

    let sched = JobScheduler::new().await.unwrap();
    let rank_config_service = RankConfigService::new(
        &mysql_pool,
        &(get_redis_connect_pool().unwrap()),
        &sched,
        parameter::get("SERVICE_NODE") == "master",
    );
    let arc_rank_config_service = Arc::new(rank_config_service);
    // 初始化排行榜配置
    if !arc_rank_config_service.init_rank().await {
        panic!("Server error : rank service init failed!!!");
    }

    // 初始化GRPC
    arc_rank_config_service
        .init_update_rank_config_grpc_service(arc_rank_config_service.clone())
        .await;

    tracing::info!(
        "listening on {} | master_node: {}",
        listener.local_addr().unwrap(),
        arc_rank_config_service.master_node
    );

    axum::serve(
        listener,
        routes::root::routes(
            Arc::clone(&mysql_pool),
            Arc::clone(&arc_rank_config_service),
        ),
    )
    .await
    .unwrap_or_else(|e| panic!("Server error: {}", e.to_string()));
}
