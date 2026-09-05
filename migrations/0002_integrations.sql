CREATE TABLE sys_payment_order (
 id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
 out_trade_no VARCHAR(64) NOT NULL UNIQUE,
 trade_no VARCHAR(100) NULL,
 merchant_id VARCHAR(80) NOT NULL,
 version VARCHAR(2) NOT NULL,
 pay_type VARCHAR(16) NOT NULL,
 amount_cents BIGINT NOT NULL,
 subject VARCHAR(120) NOT NULL,
 status VARCHAR(16) NOT NULL DEFAULT 'pending',
 pay_info TEXT NULL,
 create_by BIGINT NOT NULL,
 create_time DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
 paid_time DATETIME NULL,
 UNIQUE KEY uk_gateway_trade (merchant_id, trade_no)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
CREATE TABLE sys_job (
 job_id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
 job_name VARCHAR(64) NOT NULL,
 job_group VARCHAR(64) NOT NULL DEFAULT 'DEFAULT',
 invoke_target VARCHAR(100) NOT NULL,
 cron_expression VARCHAR(100) NOT NULL,
 misfire_policy VARCHAR(1) NOT NULL DEFAULT '3',
 concurrent VARCHAR(1) NOT NULL DEFAULT '1',
 status VARCHAR(1) NOT NULL DEFAULT '1',
 remark VARCHAR(500) NOT NULL DEFAULT '',
 next_run_time DATETIME NULL,
 create_by VARCHAR(64) NOT NULL,
 create_time DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
 update_time DATETIME NULL,
 KEY due_jobs (status, next_run_time)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
CREATE TABLE sys_job_log (
 job_log_id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
 job_id BIGINT UNSIGNED NOT NULL,
 job_name VARCHAR(64) NOT NULL,
 job_group VARCHAR(64) NOT NULL,
 invoke_target VARCHAR(100) NOT NULL,
 job_message VARCHAR(500) NOT NULL DEFAULT '',
 status VARCHAR(1) NOT NULL DEFAULT '0',
 exception_info VARCHAR(1000) NOT NULL DEFAULT '',
 start_time DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
 stop_time DATETIME NULL,
 create_time DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
 KEY job_logs (job_id, job_log_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- New menus apply to upgrades as well as fresh initialization.
INSERT INTO sys_menu (menu_id,menu_name,parent_id,order_num,path,component,menu_type,perms,icon) VALUES (2100,'定时任务',2,3,'job','monitor/job/index','C','monitor:job:list','job');
INSERT INTO sys_menu (menu_id,menu_name,parent_id,order_num,path,component,menu_type,perms,icon) VALUES (2101,'缓存监控',2,4,'cache','monitor/cache/index','C','monitor:cache:list','redis');
INSERT INTO sys_menu (menu_id,menu_name,parent_id,order_num,path,component,menu_type,perms,icon) VALUES (2102,'缓存列表',2,5,'cacheList','monitor/cache/list','C','monitor:cache:query','redis-list');
INSERT INTO sys_menu (menu_id,menu_name,parent_id,order_num,path,component,menu_type,perms,icon) VALUES (2103,'任务查询',2100,2103,'','','F','monitor:job:query','#');
INSERT INTO sys_menu (menu_id,menu_name,parent_id,order_num,path,component,menu_type,perms,icon) VALUES (2104,'任务新增',2100,2104,'','','F','monitor:job:add','#');
INSERT INTO sys_menu (menu_id,menu_name,parent_id,order_num,path,component,menu_type,perms,icon) VALUES (2105,'任务修改',2100,2105,'','','F','monitor:job:edit','#');
INSERT INTO sys_menu (menu_id,menu_name,parent_id,order_num,path,component,menu_type,perms,icon) VALUES (2106,'任务删除',2100,2106,'','','F','monitor:job:remove','#');
INSERT INTO sys_menu (menu_id,menu_name,parent_id,order_num,path,component,menu_type,perms,icon) VALUES (2107,'任务执行',2100,2107,'','','F','monitor:job:changeStatus','#');
INSERT INTO sys_menu (menu_id,menu_name,parent_id,order_num,path,component,menu_type,perms,icon) VALUES (2108,'缓存清理',2102,1,'','','F','monitor:cache:remove','#');
INSERT INTO sys_menu (menu_id,menu_name,parent_id,order_num,path,component,menu_type,perms,icon) VALUES (2109,'任务导出',2100,6,'','','F','monitor:job:export','#');
