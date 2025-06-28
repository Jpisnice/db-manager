use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug)]
enum DbType {
    POSTGRESS,
    CHROMA,
    REDIS,
}
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
    encrypted_credentials: Vec<u8>,
    nonce: Vec<u8>,
    created_at: chrono::DateTime<chrono::Utc>,
}