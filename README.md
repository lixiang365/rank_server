# rank_server

这是一个通用的排行榜服务，基于axum 框架，使用redis和mysql，做排行和存储。支持主从节点配置，可以横向扩展多个服务。

## 介绍

1. 这个项目可以配置成主从节点，满足扩展需要，只能有一个主节点，可以有多个从节点。
主节点负责管理排行榜配置和计划任务、排行榜服务。
从节点只负责排行榜服务，会定时给主节点发消息更新排行榜配置。
主从节点间使用GRPC通信。
2. 排行榜使用redis计算排行，同时使用MySQL保存所有数据。
3. 使用计划任务定时更新排行榜，在添加排行榜配置时可以编写`cron_expression`自定义排行榜的更新时间。

## 快速开始

1. 克隆这个项目
2. 配置`.env`文件
3. 安装 `sqlx-cli` 然后运行 `cargo sqlx database create` 创建数据库
4. 使用 `cargo sqlx migrate run` 运行sql迁移文件。这将运行项目根目录中存在的迁移文件夹中的迁移文件。
5. 使用 `cargo build` 构建项目和依赖项
6. 使用 `cargo run` 运行项目

## .env 文件说明

rank_server会从环境变量中读取所需的配置，同时使用`dotenv`从`.env`文件中加载

```sh
# master负责开启计划任务、管理排行榜配置、排行榜服务
# slave 只负责排行榜服务
# 默认是master
# SERVICE_NODE=master
SERVICE_NODE=master

# 排行服务监听的端口
PORT=3000

# 管理排行榜配置的管理员用户登录所需的
JWT_SECRET=secret
JWT_TTL_IN_MINUTES=30

# The generic format of the connection URL:
# mysql://[host][/database][?properties]
# 主从数据库（不设置从库，默认使用主库）
MASTER_DB_URL=mysql://user:pwd000000@127.0.0.1:3306/test
SLAVE_DB_URL=mysql://user:pwd000000@127.0.0.1:3306/test
# redis 地址
REDIS_URL=redis://127.0.0.1:6379/0

# 主节点只配置`GRPC_SERVER_PORT`就可以，从节点必须配置 `GRPC_SERVER_URL`
# 主节点 grpc 服务端口
GRPC_SERVER_PORT=3500 
# 从节点 grpc 客户端需要访问的url
GRPC_SERVER_URL=127.0.0.1:3500 

# 日志等级
RUST_LOG=info,axum=error
```

## 命令行参数

`--sync_redis` 把所有MySQL中存的排行榜数据都加载到redis里（只有主节点可用,在迁移或者其他特殊情况才会使用）
