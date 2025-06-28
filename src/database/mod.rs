use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(serde::Serialize, serde::Deserialize)]
pub enum DbType {
    Postgres,
    MySQL,
    Redis,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DbTemplate {
    image: String,
    default_port: u16,
    env_vars: HashMap<String, String>, // Template env vars
    volumes: Vec<String>,
    health_check: Option<String>,
    connection_string: Option<String>, // Optional connection string
}

// Built-in database templates
pub fn get_db_templates() -> HashMap<String, DbTemplate> {
    let mut templates = HashMap::new();
    
    templates.insert("postgres".to_string(), DbTemplate {
        image: "postgres:15".to_string(),
        default_port: 5432,
        env_vars: HashMap::from([
            ("POSTGRES_DB".to_string(), "{database}".to_string()),
            ("POSTGRES_USER".to_string(), "{username}".to_string()),
            ("POSTGRES_PASSWORD".to_string(), "{password}".to_string()),
        ]),
        volumes: vec!["{name}_data:/var/lib/postgresql/data".to_string()],
        health_check: Some("pg_isready -U {username}".to_string()),
        connection_string: Some("postgresql://{username}:{password}@localhost:{port}/{database}".to_string()),
    });
    
    templates.insert("mysql".to_string(), DbTemplate {
        image: "mysql:8.0".to_string(),
        default_port: 3306,
        env_vars: HashMap::from([
            ("MYSQL_DATABASE".to_string(), "{database}".to_string()),
            ("MYSQL_USER".to_string(), "{username}".to_string()),
            ("MYSQL_PASSWORD".to_string(), "{password}".to_string()),
            ("MYSQL_ROOT_PASSWORD".to_string(), "{root_password}".to_string()),
        ]),
        volumes: vec!["{name}_data:/var/lib/mysql".to_string()],
        health_check: Some("mysqladmin ping -h localhost".to_string()),
        connection_string: Some("mysql://{username}:{password}@localhost:{port}/{database}".to_string()),
    });
    
    templates.insert("redis".to_string(), DbTemplate {
        image: "redis:7-alpine".to_string(),
        default_port: 6379,
        env_vars: HashMap::new(),
        volumes: vec!["{name}_data:/data".to_string()],
        health_check: Some("redis-cli ping".to_string()),
        connection_string: Some("redis://localhost:{port}".to_string()),
    });
    
    templates
}