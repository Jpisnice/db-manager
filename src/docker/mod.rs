use futures_util::StreamExt;
use shiplift::{ContainerOptions, Docker, PullOptions};
use std::collections::HashMap;

// Import the DbCredentials type from credentials module
use crate::credentials::DbCredentials;

#[derive(Debug, Clone)]
pub struct DbTemplate {
    pub image: String,
    pub default_port: u16,
    pub env_vars: HashMap<String, String>,
    pub volumes: Vec<String>,
}

pub struct DockerManager {
    docker: Docker,
}

impl DockerManager {
    pub fn new() -> Result<Self, anyhow::Error> {
        let docker = Docker::new();
        Ok(Self { docker })
    }

    async fn pull_image(&self, image: &str) -> Result<(), anyhow::Error> {
        println!("Pulling image: {}", image);

        let mut stream = self
            .docker
            .images()
            .pull(&PullOptions::builder().image(image).build());

        while let Some(pull_result) = stream.next().await {
            match pull_result {
                Ok(output) => {
                    if let Some(status) = output.get("status") {
                        if let Some(status_str) = status.as_str() {
                            println!("Status: {}", status_str);
                        }
                    }
                }
                Err(e) => return Err(e.into()),
            }
        }

        println!("✓ Image pulled successfully");
        Ok(())
    }

    async fn create_container(
        &self,
        name: &str,
        image: &str,
        env_vars: Vec<String>,
        port_mappings: HashMap<String, String>,
        volumes: Vec<String>,
    ) -> Result<String, anyhow::Error> {
        // Parse port mappings first
        let mut parsed_ports = Vec::new();
        for (container_port_str, host_port_str) in port_mappings {
            let container_port = container_port_str
                .parse::<u32>()
                .map_err(|_| anyhow::anyhow!("Invalid container port: {}", container_port_str))?;
            let host_port = host_port_str
                .parse::<u32>()
                .map_err(|_| anyhow::anyhow!("Invalid host port: {}", host_port_str))?;
            parsed_ports.push((container_port, host_port));
        }

        // Build container options all at once
        let env_refs: Vec<&str> = env_vars.iter().map(|s| s.as_str()).collect();
        let volume_refs: Vec<&str> = volumes.iter().map(|s| s.as_str()).collect();

        let mut opts = ContainerOptions::builder(image);
        opts.name(name);
        
        if !env_vars.is_empty() {
            opts.env(env_refs);
        }
        
        for (container_port, host_port) in parsed_ports {
            opts.expose(container_port, "tcp", host_port);
        }
        
        if !volumes.is_empty() {
            opts.volumes(volume_refs);
        }

        let container = self
            .docker
            .containers()
            .create(&opts.build())
            .await?;

        println!("✓ Container '{}' created with ID: {}", name, container.id);
        Ok(container.id)
    }

    pub async fn start_container(&self, id: &str) -> Result<(), anyhow::Error> {
        self.docker.containers().get(id).start().await?;
        println!("✓ Container started");
        Ok(())
    }

    pub async fn wait_for_health(&self, id: &str, timeout_secs: u64) -> Result<(), anyhow::Error> {
        use std::time::{Duration, Instant};
        use tokio::time::sleep;

        println!("⏳ Waiting for container to be healthy...");
        let start = Instant::now();
        let timeout = Duration::from_secs(timeout_secs);

        loop {
            if start.elapsed() > timeout {
                return Err(anyhow::anyhow!("Container health check timeout"));
            }

            match self.docker.containers().get(id).inspect().await {
                Ok(details) => {
                    if details.state.running {
                        println!("✓ Container is healthy and running");
                        return Ok(());
                    }
                }
                Err(_) => {
                    // Container might not be fully started yet
                }
            }

            sleep(Duration::from_secs(2)).await;
        }
    }

    pub async fn create_database_container(
        &self,
        name: &str,
        db_type: &str,
        credentials: &DbCredentials,
    ) -> Result<String, anyhow::Error> {
        let template = get_db_template(db_type)?;

        // Pull image
        self.pull_image(&template.image).await?;

        // Build environment variables
        let env_vars = build_env_vars(&template, name, credentials);

        // Build port mapping
        let mut port_mappings = HashMap::new();
        let container_port_key = template.default_port.to_string();
        port_mappings.insert(
            container_port_key,
            credentials.port.to_string(),
        );

        // Build volumes
        let volumes = template
            .volumes
            .iter()
            .map(|v| v.replace("{name}", name))
            .collect();

        // Create container
        let container_id = self
            .create_container(name, &template.image, env_vars, port_mappings, volumes)
            .await?;

        // Start container
        self.start_container(&container_id).await?;

        Ok(container_id)
    }
}

fn get_db_template(db_type: &str) -> Result<DbTemplate, anyhow::Error> {
    match db_type.to_lowercase().as_str() {
        "mysql" => Ok(DbTemplate {
            image: "mysql:8.0".to_string(),
            default_port: 3306,
            env_vars: {
                let mut env = HashMap::new();
                env.insert("MYSQL_DATABASE".to_string(), "{database}".to_string());
                env.insert("MYSQL_USER".to_string(), "{username}".to_string());
                env.insert("MYSQL_PASSWORD".to_string(), "{password}".to_string());
                env.insert("MYSQL_ROOT_PASSWORD".to_string(), "{root_password}".to_string());
                env
            },
            volumes: vec!["{name}_data:/var/lib/mysql".to_string()],
        }),
        "postgres" => Ok(DbTemplate {
            image: "postgres:15".to_string(),
            default_port: 5432,
            env_vars: {
                let mut env = HashMap::new();
                env.insert("POSTGRES_DB".to_string(), "{database}".to_string());
                env.insert("POSTGRES_USER".to_string(), "{username}".to_string());
                env.insert("POSTGRES_PASSWORD".to_string(), "{password}".to_string());
                env
            },
            volumes: vec!["{name}_data:/var/lib/postgresql/data".to_string()],
        }),
        "redis" => Ok(DbTemplate {
            image: "redis:7-alpine".to_string(),
            default_port: 6379,
            env_vars: HashMap::new(),
            volumes: vec!["{name}_data:/data".to_string()],
        }),
        _ => Err(anyhow::anyhow!("Unsupported database type: {}", db_type)),
    }
}

fn build_env_vars(
    template: &DbTemplate,
    name: &str,
    credentials: &DbCredentials,
) -> Vec<String> {
    template
        .env_vars
        .iter()
        .map(|(key, value)| {
            let replaced_value = value
                .replace("{name}", name)
                .replace("{username}", &credentials.username)
                .replace("{password}", &credentials.password)
                .replace("{database}", &credentials.database)
                .replace("{port}", &credentials.port.to_string())
                .replace(
                    "{root_password}",
                    credentials.root_password.as_deref().unwrap_or(""),
                );
            format!("{}={}", key, replaced_value)
        })
        .collect()
}
