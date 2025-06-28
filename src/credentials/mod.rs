use serde::{Serialize, Deserialize};
use std::path::PathBuf;
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
struct DbCredentials {
    username: String,
    password: String,
    database: String,
    port: u16,
    root_password: Option<String>, // For MySQL
}

impl AppConfig {
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
        
        self.databases.insert(name.clone(), EncryptedDbConfig {
            name: name.clone(),
            db_type,
            container_id,
            encrypted_credentials: encrypted_config,
            // ... other fields
        });
        
        self.save()?;
        
        println!("✅ Database '{}' is ready!", name);
        Ok(())
    }
}