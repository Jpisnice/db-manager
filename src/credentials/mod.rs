use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use crate::database::{DbType, get_db_templates};
use crate::docker::DockerManager;

// Encryption imports
use chacha20poly1305::{
    aead::{Aead, NewAead},
    ChaCha20Poly1305, Nonce, Key
};
use scrypt::{scrypt, Params};
use rand::{rngs::OsRng, RngCore};

// Use platform-appropriate config directory
use directories::ProjectDirs;

#[derive(Serialize, Deserialize)]
pub struct AppConfig {
    passphrase_hash: String,
    salt: Vec<u8>,
    databases: HashMap<String, EncryptedDbConfig>,
    version: u32, // for future migrations
}

#[derive(Serialize, Deserialize)]
struct EncryptedDbConfig {
    name: String,
    db_type: String, // postgres, mysql, redis, etc.
    container_id: String,
    encrypted_credentials: Vec<u8>,
    nonce: Vec<u8>,
    encrypted_connection_string: Vec<u8>,
    connection_nonce: Vec<u8>,
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

#[derive(Clone)]
pub struct DecryptedDbInfo {
    pub name: String,
    pub db_type: DbType,
    pub container_id: String,
    pub credentials: DbCredentials,
    pub connection_string: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

fn get_config_path() -> PathBuf {
    let proj_dirs = ProjectDirs::from("com", "yourname", "dbmanager")
        .expect("Failed to get project directories");
    
    let config_dir = proj_dirs.config_dir();
    
    // Create directory if it doesn't exist
    fs::create_dir_all(config_dir).expect("Failed to create config directory");
    
    config_dir.join("config.json")
}

impl AppConfig {
    /// Create a new configuration with the given passphrase
    pub fn new(passphrase: &str) -> Result<Self, anyhow::Error> {
        let mut salt = vec![0u8; 32];
        OsRng.fill_bytes(&mut salt);
        
        // Create a hash for passphrase verification
        let key = Self::derive_key(passphrase, &salt)?;
        let passphrase_hash = format!("scrypt:{}", base64::encode(&key));

        Ok(AppConfig {
            passphrase_hash,
            salt,
            databases: HashMap::new(),
            version: 1,
        })
    }

    /// Load configuration from file, or create new if doesn't exist
    pub fn load_or_create(passphrase: &str) -> Result<Self, anyhow::Error> {
        let config_path = get_config_path();
        
        if config_path.exists() {
            Self::load(passphrase)
        } else {
            println!("Creating new configuration...");
            let config = Self::new(passphrase)?;
            config.save()?;
            Ok(config)
        }
    }

    /// Load existing configuration from file
    pub fn load(passphrase: &str) -> Result<Self, anyhow::Error> {
        let config_path = get_config_path();
        let content = fs::read_to_string(&config_path)
            .map_err(|_| anyhow::anyhow!("Configuration file not found. Run the app once to initialize."))?;
        
        let config: AppConfig = serde_json::from_str(&content)?;
        
        // Verify passphrase
        config.verify_passphrase(passphrase)?;
        
        Ok(config)
    }

    /// Verify the provided passphrase against the stored hash
    fn verify_passphrase(&self, passphrase: &str) -> Result<(), anyhow::Error> {
        if let Some(hash_part) = self.passphrase_hash.strip_prefix("scrypt:") {
            let stored_key = base64::decode(hash_part)?;
            let derived_key = Self::derive_key(passphrase, &self.salt)?;
            
            if stored_key == derived_key {
                Ok(())
            } else {
                Err(anyhow::anyhow!("Invalid passphrase"))
            }
        } else {
            Err(anyhow::anyhow!("Invalid hash format"))
        }
    }

    /// Derive encryption key from passphrase and salt
    fn derive_key(passphrase: &str, salt: &[u8]) -> Result<Vec<u8>, anyhow::Error> {
        let params = Params::new(15, 8, 1)?; // log_n=15, r=8, p=1
        let mut key = vec![0u8; 32];
        scrypt(passphrase.as_bytes(), salt, &params, &mut key)?;
        Ok(key)
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<(), anyhow::Error> {
        let config_path = get_config_path();
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&config_path, content)?;
        println!("Configuration saved to: {}", config_path.display());
        Ok(())
    }

    /// Encrypt data using ChaCha20Poly1305
    fn encrypt_data(&self, data: &[u8], passphrase: &str) -> Result<(Vec<u8>, Vec<u8>), anyhow::Error> {
        let key = Self::derive_key(passphrase, &self.salt)?;
        let cipher = ChaCha20Poly1305::new(Key::from_slice(&key));
        
        let mut nonce_bytes = vec![0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        let ciphertext = cipher.encrypt(nonce, data)
            .map_err(|_| anyhow::anyhow!("Encryption failed"))?;
        
        Ok((ciphertext, nonce_bytes))
    }

    /// Decrypt data using ChaCha20Poly1305
    fn decrypt_data(&self, ciphertext: &[u8], nonce: &[u8], passphrase: &str) -> Result<Vec<u8>, anyhow::Error> {
        let key = Self::derive_key(passphrase, &self.salt)?;
        let cipher = ChaCha20Poly1305::new(Key::from_slice(&key));
        let nonce = Nonce::from_slice(nonce);
        
        let plaintext = cipher.decrypt(nonce, ciphertext)
            .map_err(|_| anyhow::anyhow!("Decryption failed"))?;
        
        Ok(plaintext)
    }

    /// Generate connection string for the database
    fn generate_connection_string(&self, db_type: &str, credentials: &DbCredentials) -> Result<String, anyhow::Error> {
        let templates = get_db_templates();
        let template = templates.get(db_type)
            .ok_or_else(|| anyhow::anyhow!("Unsupported database type: {}", db_type))?;
        
        if let Some(conn_template) = &template.connection_string {
            let connection_string = conn_template
                .replace("{username}", &credentials.username)
                .replace("{password}", &credentials.password)
                .replace("{database}", &credentials.database)
                .replace("{port}", &credentials.port.to_string());
            Ok(connection_string)
        } else {
            Err(anyhow::anyhow!("No connection string template for {}", db_type))
        }
    }

    /// Create a new database and store its encrypted configuration
    pub async fn create_database(
        &mut self,
        name: String,
        db_type: String,
        credentials: DbCredentials,
        passphrase: &str,
    ) -> Result<(), anyhow::Error> {
        // Check if database already exists
        if self.databases.contains_key(&name) {
            return Err(anyhow::anyhow!("Database '{}' already exists", name));
        }

        let docker_manager = DockerManager::new()?;
        
        // Create and start container
        let container_id = docker_manager
            .create_database_container(&name, &db_type, &credentials)
            .await?;
        
        docker_manager.start_container(&container_id).await?;
        docker_manager.wait_for_health(&container_id, 60).await?;

        // Generate connection string
        let connection_string = self.generate_connection_string(&db_type, &credentials)?;

        // Encrypt credentials
        let credentials_json = serde_json::to_vec(&credentials)?;
        let (encrypted_credentials, cred_nonce) = self.encrypt_data(&credentials_json, passphrase)?;

        // Encrypt connection string
        let (encrypted_connection_string, conn_nonce) = self.encrypt_data(connection_string.as_bytes(), passphrase)?;

        // Store encrypted configuration
        self.databases.insert(name.clone(), EncryptedDbConfig {
            name: name.clone(),
            db_type: db_type.clone(),
            container_id,
            encrypted_credentials,
            nonce: cred_nonce,
            encrypted_connection_string,
            connection_nonce: conn_nonce,
            created_at: chrono::Utc::now(),
        });
        
        self.save()?;
        
        println!("✅ Database '{}' is ready!", name);
        println!("🔗 Connection string: {}", connection_string);
        Ok(())
    }

    /// Get decrypted database information
    pub fn get_database(&self, name: &str, passphrase: &str) -> Result<DecryptedDbInfo, anyhow::Error> {
        let encrypted_config = self.databases.get(name)
            .ok_or_else(|| anyhow::anyhow!("Database '{}' not found", name))?;

        // Decrypt credentials
        let credentials_data = self.decrypt_data(
            &encrypted_config.encrypted_credentials,
            &encrypted_config.nonce,
            passphrase,
        )?;
        let credentials: DbCredentials = serde_json::from_slice(&credentials_data)?;

        // Decrypt connection string
        let connection_data = self.decrypt_data(
            &encrypted_config.encrypted_connection_string,
            &encrypted_config.connection_nonce,
            passphrase,
        )?;
        let connection_string = String::from_utf8(connection_data)?;

        // Convert string to DbType enum
        let db_type = match encrypted_config.db_type.to_lowercase().as_str() {
            "postgres" => DbType::Postgres,
            "mysql" => DbType::MySQL,
            "redis" => DbType::Redis,
            _ => return Err(anyhow::anyhow!("Unknown database type: {}", encrypted_config.db_type)),
        };

        Ok(DecryptedDbInfo {
            name: encrypted_config.name.clone(),
            db_type,
            container_id: encrypted_config.container_id.clone(),
            credentials,
            connection_string,
            created_at: encrypted_config.created_at,
        })
    }

    /// List all database names
    pub fn list_databases(&self) -> Vec<String> {
        self.databases.keys().cloned().collect()
    }

    /// Remove a database configuration
    pub fn remove_database(&mut self, name: &str) -> Result<(), anyhow::Error> {
        if self.databases.remove(name).is_some() {
            self.save()?;
            println!("✅ Database '{}' configuration removed", name);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Database '{}' not found", name))
        }
    }

    /// Get all decrypted database information
    pub fn get_all_databases(&self, passphrase: &str) -> Result<Vec<DecryptedDbInfo>, anyhow::Error> {
        let mut databases = Vec::new();
        for name in self.databases.keys() {
            databases.push(self.get_database(name, passphrase)?);
        }
        Ok(databases)
    }

    /// Get database info without decrypting (for listing purposes)
    pub fn get_database_info(&self, name: &str) -> Result<(&str, &str, &chrono::DateTime<chrono::Utc>), anyhow::Error> {
        let config = self.databases.get(name)
            .ok_or_else(|| anyhow::anyhow!("Database '{}' not found", name))?;
        
        Ok((&config.db_type, &config.container_id, &config.created_at))
    }

    /// Reset configuration - removes the config file (USE WITH CAUTION)
    /// This will delete all stored database configurations
    pub fn reset_config() -> Result<(), anyhow::Error> {
        let config_path = get_config_path();
        
        if config_path.exists() {
            fs::remove_file(&config_path)?;
            println!("Configuration file deleted: {}", config_path.display());
            println!("All database configurations have been removed.");
            println!("You can now start fresh with a new passphrase.");
        } else {
            println!("No configuration file found at: {}", config_path.display());
        }
        
        Ok(())
    }

    /// Check if config file exists
    pub fn config_exists() -> bool {
        get_config_path().exists()
    }
}
