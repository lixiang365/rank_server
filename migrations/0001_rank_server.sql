-- Add migration script here
-- 管理员用户表
CREATE TABLE `user` (
                        `id` int NOT NULL AUTO_INCREMENT,
                        `first_name` varchar(190) DEFAULT NULL,
                        `last_name` varchar(190) DEFAULT NULL,
                        `user_name` varchar(190) NOT NULL,
                        `email` varchar(190) NOT NULL,
                        `password` varchar(190) NOT NULL,
                        `created_at` timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
                        `updated_at` timestamp NULL,
                        `is_active` tinyint(1) NOT NULL DEFAULT '0',
                        PRIMARY KEY (`id`),
                        UNIQUE KEY `user_name` (`user_name`),
                        UNIQUE KEY `email` (`email`)
) ENGINE=InnoDB AUTO_INCREMENT=0 DEFAULT CHARSET=utf8mb4;

-- 排行榜配置表
CREATE TABLE IF NOT EXISTS `rank_table_config` (
                        `appid` varchar(190) NOT NULL ,
                        `app_secret` varchar(190) NOT NULL ,
                        `rank_key` varchar(190) NOT NULL ,
                        `cron_expression` varchar(190) NOT NULL DEFAULT '' ,
                        `remark` varchar(190) NOT NULL DEFAULT '',
                        PRIMARY KEY (`appid`,`rank_key`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- 动态创建排行榜表存储过程
DELIMITER ;;
CREATE PROCEDURE CREATE_RANK_TABLE(IN appid VARCHAR(64) CHARSET utf8,IN rank_key VARCHAR(64) CHARSET utf8)
BEGIN
		SET @createTbsql = CONCAT('CREATE TABLE IF NOT EXISTS ','rank_',appid,'_',rank_key,
		"(
			`openid` varchar(190) NOT NULL ,
			`nick_name` varchar(190) NOT NULL DEFAULT 'momo',
			`score` int NOT NULL,
			PRIMARY KEY (`openid`)
		) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4"
		);
		-- 执行动态生成的sql语句
		-- 预定义sql语句，从用户变量中获取
		PREPARE temp FROM @createTbsql;
		EXECUTE temp;
		-- 释放资源，后续还可以使用
		deallocate prepare temp;
END
;;
DELIMITER ;
