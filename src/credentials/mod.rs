use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use crate::database::{DbType};
use crate::docker::DockerManager;

#[derive(Serialize, Deserialize)]
struct AppConfig {
    passphrase_hash: String,
    salt: Vec<u8>,
    databases: HashMap<String, EncryptedDbConfig>,
    version: u32, // for future migrations
}

#[derive(Serialize, Deserialize)]
struct EncryptedDbConfig {
    name: String,
    db_type: DbType,
    container_id: String,
    encrypted_credentials: Vec<u8>,
    nonce: Vec<u8>,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DbCredentials {
    pub username: String,
    pub password: String,
    pub database: String,
    pub port: u16,
    pub root_password: Option<String>, // For MySQL
}

impl AppConfig {
    fn encrypt_credentials(&self, credentials: &DbCredentials, _passphrase: &str) -> Result<Vec<u8>, anyhow::Error> {
        // Placeholder implementation - in a real app you'd use proper encryption
        let serialized = serde_json::to_string(credentials)?;
        Ok(serialized.into_bytes())
    }

    fn save(&self) -> Result<(), anyhow::Error> {
        // Placeholder implementation - in a real app you'd save to a file
        println!("Saving configuration...");
        Ok(())
    }

    async fn create_database(
        &mut self,
        name: String,
        db_type: String,
        credentials: DbCredentials,
        passphrase: &str,
    ) -> Result<(),anyhow::Error> {
        let docker_manager = DockerManager::new()?;
        
        // Create and start container
        let container_id = docker_manager
            .create_database_container(&name, &db_type, &credentials)
            .await?;
        
        docker_manager.start_container(&container_id).await?;
        docker_manager.wait_for_health(&container_id, 60).await?;
        
        // Encrypt and store credentials
        let encrypted_config = self.encrypt_credentials(&credentials, passphrase)?;
        
        // Convert string to DbType enum
        let db_type_enum = match db_type.to_lowercase().as_str() {
            "postgres" => DbType::Postgres,
            "mysql" => DbType::MySQL,
            "redis" => DbType::Redis,
            _ => return Err(anyhow::anyhow!("Unsupported database type: {}", db_type)),
        };
        
        self.databases.insert(name.clone(), EncryptedDbConfig {
            name: name.clone(),
            db_type: db_type_enum,
            container_id,
            encrypted_credentials: encrypted_config,
            nonce: vec![], // Placeholder, should be set by encryption
            created_at: chrono::Utc::now(),
        });
        
        self.save()?;
        
        println!("✅ Database '{}' is ready!", name);
        Ok(())
    }
}