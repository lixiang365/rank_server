use std::collections::HashSet;

use dotenv;
use once_cell::sync::OnceCell;

pub static CMD_ARGS: OnceCell<HashSet<String>> = OnceCell::new();

pub fn init() {
    dotenv::dotenv().ok().expect("Failed to load .env file");
    // 给日志库设置环境变量
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "debug")
    }
    // 默认是主服务
    if std::env::var_os("SERVICE_NODE").is_none() {
        std::env::set_var("SERVICE_NODE", "master")
    }

    if std::env::var_os("MASTER_DB_URL").is_none() {
        panic!("config -- env var `MASTER_DB_URL` is not exist ");
    }
    if std::env::var_os("SLAVE_DB_URL").is_none() {
        std::env::set_var("SERVICE_NODE", std::env::var_os("SLAVE_DB_URL").unwrap());
    }
    assert!(CMD_ARGS.set(std::env::args().collect()).is_ok());
}

pub fn get(parameter: &str) -> String {
    let env_parameter = std::env::var(parameter)
        .expect(&format!("{} is not defined in the environment.", parameter));
    return env_parameter;
}

// 计算分数的基准时间
// 12023/10/01 00:00:00
pub const CALC_SCORE_BASE_TIME_STAMP: f64 = 317265609600.0;
