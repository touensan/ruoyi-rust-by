CREATE TABLE `sys_dept` (
  `dept_id` BIGINT UNSIGNED AUTO_INCREMENT NOT NULL,
  `parent_id` BIGINT NOT NULL DEFAULT '0',
  `ancestors` VARCHAR(50) NOT NULL DEFAULT '',
  `dept_name` VARCHAR(30) NOT NULL DEFAULT '',
  `order_num` BIGINT NOT NULL DEFAULT '0',
  `leader` VARCHAR(20) NULL,
  `phone` VARCHAR(11) NULL,
  `email` VARCHAR(50) NULL,
  `status` VARCHAR(1) NOT NULL DEFAULT '0',
  `create_by` VARCHAR(64) NOT NULL DEFAULT '',
  `create_time` DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  `update_by` VARCHAR(64) NOT NULL DEFAULT '',
  `update_time` DATETIME NULL,
  `delete_time` DATETIME NULL,
  PRIMARY KEY (`dept_id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE `sys_user` (
  `user_id` BIGINT UNSIGNED AUTO_INCREMENT NOT NULL,
  `dept_id` BIGINT NOT NULL DEFAULT '0',
  `user_name` VARCHAR(30) NOT NULL,
  `nick_name` VARCHAR(30) NOT NULL,
  `user_type` VARCHAR(2) NOT NULL DEFAULT '00',
  `email` VARCHAR(50) NOT NULL DEFAULT '',
  `phonenumber` VARCHAR(11) NOT NULL DEFAULT '',
  `sex` VARCHAR(1) NOT NULL DEFAULT '0',
  `avatar` VARCHAR(100) NOT NULL DEFAULT '',
  `password` VARCHAR(100) NOT NULL DEFAULT '',
  `login_ip` VARCHAR(128) NOT NULL DEFAULT '',
  `login_date` DATETIME NULL,
  `status` VARCHAR(1) NOT NULL DEFAULT '0',
  `create_by` VARCHAR(64) NOT NULL DEFAULT '',
  `create_time` DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  `update_by` VARCHAR(64) NOT NULL DEFAULT '',
  `update_time` DATETIME NULL,
  `delete_time` DATETIME NULL,
  `remark` VARCHAR(500) NULL,
  PRIMARY KEY (`user_id`),
  UNIQUE KEY uk_user_name (user_name)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE `sys_post` (
  `post_id` BIGINT UNSIGNED AUTO_INCREMENT NOT NULL,
  `post_code` VARCHAR(64) NOT NULL,
  `post_name` VARCHAR(50) NOT NULL,
  `post_sort` BIGINT NOT NULL DEFAULT '0',
  `status` VARCHAR(1) NOT NULL DEFAULT '0',
  `create_by` VARCHAR(64) NOT NULL DEFAULT '',
  `create_time` DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  `update_by` VARCHAR(64) NOT NULL DEFAULT '',
  `update_time` DATETIME NULL,
  `delete_time` DATETIME NULL,
  `remark` VARCHAR(500) NULL,
  PRIMARY KEY (`post_id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE `sys_role` (
  `role_id` BIGINT UNSIGNED AUTO_INCREMENT NOT NULL,
  `role_name` VARCHAR(30) NOT NULL,
  `role_key` VARCHAR(100) NOT NULL,
  `role_sort` BIGINT NOT NULL,
  `data_scope` VARCHAR(1) NOT NULL DEFAULT '1',
  `menu_check_strictly` BIGINT NOT NULL DEFAULT '1',
  `dept_check_strictly` BIGINT NOT NULL DEFAULT '1',
  `status` VARCHAR(1) NOT NULL DEFAULT '0',
  `create_by` VARCHAR(64) NOT NULL DEFAULT '',
  `create_time` DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  `update_by` VARCHAR(64) NOT NULL DEFAULT '',
  `update_time` DATETIME NULL,
  `delete_time` DATETIME NULL,
  `remark` VARCHAR(500) NULL,
  PRIMARY KEY (`role_id`),
  UNIQUE KEY uk_role_key (role_key)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE `sys_menu` (
  `menu_id` BIGINT UNSIGNED AUTO_INCREMENT NOT NULL,
  `menu_name` VARCHAR(50) NOT NULL,
  `parent_id` BIGINT NOT NULL DEFAULT '0',
  `order_num` BIGINT NOT NULL DEFAULT '0',
  `path` VARCHAR(200) NOT NULL DEFAULT '',
  `component` VARCHAR(255) NULL,
  `query` VARCHAR(255) NULL,
  `route_name` VARCHAR(50) NOT NULL DEFAULT '',
  `is_frame` BIGINT NOT NULL DEFAULT '1',
  `is_cache` BIGINT NOT NULL DEFAULT '0',
  `menu_type` VARCHAR(1) NOT NULL DEFAULT '',
  `visible` VARCHAR(1) NOT NULL DEFAULT '0',
  `perms` VARCHAR(100) NULL,
  `icon` VARCHAR(100) NOT NULL DEFAULT '#',
  `status` VARCHAR(1) NOT NULL DEFAULT '0',
  `create_by` VARCHAR(64) NOT NULL DEFAULT '',
  `create_time` DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  `update_by` VARCHAR(64) NOT NULL DEFAULT '',
  `update_time` DATETIME NULL,
  `delete_time` DATETIME NULL,
  `remark` VARCHAR(500) NULL,
  PRIMARY KEY (`menu_id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE `sys_user_role` (
  `user_id` BIGINT NOT NULL,
  `role_id` BIGINT NOT NULL,
  PRIMARY KEY (`user_id`,`role_id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE `sys_role_menu` (
  `role_id` BIGINT NOT NULL,
  `menu_id` BIGINT NOT NULL,
  PRIMARY KEY (`role_id`,`menu_id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE `sys_role_dept` (
  `role_id` BIGINT NOT NULL,
  `dept_id` BIGINT NOT NULL,
  PRIMARY KEY (`role_id`,`dept_id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE `sys_user_post` (
  `user_id` BIGINT NOT NULL,
  `post_id` BIGINT NOT NULL,
  PRIMARY KEY (`user_id`,`post_id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE `sys_oper_log` (
  `oper_id` BIGINT UNSIGNED AUTO_INCREMENT NOT NULL,
  `title` VARCHAR(50) NOT NULL DEFAULT '',
  `business_type` BIGINT NOT NULL DEFAULT '0',
  `method` VARCHAR(200) NOT NULL DEFAULT '',
  `request_method` VARCHAR(10) NOT NULL DEFAULT '',
  `oper_name` VARCHAR(50) NOT NULL DEFAULT '',
  `dept_name` VARCHAR(50) NOT NULL DEFAULT '',
  `oper_url` VARCHAR(255) NOT NULL DEFAULT '',
  `oper_ip` VARCHAR(128) NOT NULL DEFAULT '',
  `oper_location` VARCHAR(255) NOT NULL DEFAULT '',
  `oper_param` VARCHAR(2000) NOT NULL DEFAULT '',
  `json_result` VARCHAR(2000) NOT NULL DEFAULT '',
  `status` VARCHAR(1) NOT NULL DEFAULT '0',
  `error_msg` VARCHAR(2000) NOT NULL DEFAULT '',
  `oper_time` DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  `cost_time` BIGINT NOT NULL DEFAULT '0',
  PRIMARY KEY (`oper_id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE `sys_dict_type` (
  `dict_id` BIGINT UNSIGNED AUTO_INCREMENT NOT NULL,
  `dict_name` VARCHAR(100) NOT NULL DEFAULT '',
  `dict_type` VARCHAR(100) NOT NULL DEFAULT '',
  `status` VARCHAR(1) NOT NULL DEFAULT '0',
  `create_by` VARCHAR(64) NOT NULL DEFAULT '',
  `create_time` DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  `update_by` VARCHAR(64) NOT NULL DEFAULT '',
  `update_time` DATETIME NULL,
  `remark` VARCHAR(500) NULL,
  PRIMARY KEY (`dict_id`),
  UNIQUE KEY uk_dict_type (dict_type)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE `sys_dict_data` (
  `dict_code` BIGINT UNSIGNED AUTO_INCREMENT NOT NULL,
  `dict_sort` BIGINT NOT NULL DEFAULT '0',
  `dict_label` VARCHAR(100) NOT NULL DEFAULT '',
  `dict_value` VARCHAR(100) NOT NULL DEFAULT '',
  `dict_type` VARCHAR(100) NOT NULL DEFAULT '',
  `css_class` VARCHAR(100) NULL,
  `list_class` VARCHAR(100) NULL,
  `is_default` VARCHAR(1) NULL DEFAULT 'N',
  `status` VARCHAR(1) NOT NULL DEFAULT '0',
  `create_by` VARCHAR(64) NOT NULL DEFAULT '',
  `create_time` DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  `update_by` VARCHAR(64) NOT NULL DEFAULT '',
  `update_time` DATETIME NULL,
  `remark` VARCHAR(500) NULL,
  PRIMARY KEY (`dict_code`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE `sys_config` (
  `config_id` BIGINT UNSIGNED AUTO_INCREMENT NOT NULL,
  `config_name` VARCHAR(100) NOT NULL DEFAULT '',
  `config_key` VARCHAR(100) NOT NULL DEFAULT '',
  `config_value` VARCHAR(500) NOT NULL DEFAULT '',
  `config_type` VARCHAR(1) NOT NULL DEFAULT 'N',
  `create_by` VARCHAR(64) NOT NULL DEFAULT '',
  `create_time` DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  `update_by` VARCHAR(64) NOT NULL DEFAULT '',
  `update_time` DATETIME NULL,
  `remark` VARCHAR(500) NULL,
  PRIMARY KEY (`config_id`),
  UNIQUE KEY uk_config_key (config_key)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE `sys_system_setting` (
  `setting_key` VARCHAR(100) NOT NULL,
  `setting_group` VARCHAR(50) NOT NULL DEFAULT '',
  `setting_value` TEXT NULL,
  `remark` VARCHAR(500) NULL,
  `create_by` VARCHAR(64) NOT NULL DEFAULT '',
  `create_time` DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  `update_by` VARCHAR(64) NOT NULL DEFAULT '',
  `update_time` DATETIME NULL,
  PRIMARY KEY (`setting_key`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE `sys_logininfor` (
  `info_id` BIGINT UNSIGNED AUTO_INCREMENT NOT NULL,
  `user_name` VARCHAR(50) NOT NULL DEFAULT '',
  `ipaddr` VARCHAR(128) NOT NULL DEFAULT '',
  `login_location` VARCHAR(255) NOT NULL DEFAULT '',
  `browser` VARCHAR(50) NOT NULL DEFAULT '',
  `os` VARCHAR(50) NOT NULL DEFAULT '',
  `status` VARCHAR(1) NOT NULL DEFAULT '0',
  `msg` VARCHAR(255) NOT NULL DEFAULT '',
  `login_time` DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (`info_id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE `sys_notice` (
  `notice_id` BIGINT UNSIGNED AUTO_INCREMENT NOT NULL,
  `notice_title` VARCHAR(255) NOT NULL DEFAULT '',
  `notice_type` VARCHAR(1) NOT NULL DEFAULT '1',
  `notice_content` TEXT NOT NULL,
  `status` VARCHAR(1) NOT NULL DEFAULT '0',
  `create_by` VARCHAR(64) NOT NULL DEFAULT '',
  `create_time` DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  `update_time` DATETIME NULL,
  `remark` VARCHAR(500) NOT NULL DEFAULT '',
  PRIMARY KEY (`notice_id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE sys_access_token (
 id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY, user_id BIGINT NOT NULL, token_hash VARCHAR(64) NOT NULL UNIQUE,
 ip VARCHAR(45) NOT NULL DEFAULT '', browser VARCHAR(255) NOT NULL DEFAULT '', created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
 expires_at DATETIME NOT NULL, INDEX(user_id), INDEX(expires_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE sys_notice_read (notice_id BIGINT NOT NULL, user_id BIGINT NOT NULL, read_time DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP, PRIMARY KEY(notice_id,user_id)) ENGINE=InnoDB;

CREATE TABLE gen_table (table_id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY, table_name VARCHAR(64) NOT NULL UNIQUE, table_comment VARCHAR(255) NOT NULL DEFAULT '', class_name VARCHAR(100) NOT NULL, module_name VARCHAR(64) NOT NULL DEFAULT 'business', business_name VARCHAR(64) NOT NULL, function_name VARCHAR(255) NOT NULL DEFAULT '', function_author VARCHAR(100) NOT NULL DEFAULT 'touensan', tpl_category VARCHAR(20) NOT NULL DEFAULT 'crud', options TEXT, create_time DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
