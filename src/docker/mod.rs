use bollard::{Docker, container::*, image::*, volume::*};
use bollard::models::*;
use anyhow::Result;
use crate::database::{get_db_templates, DbTemplate};
use std::collections::HashMap;

pub struct DockerManager {
    docker: Docker,
    templates: HashMap<String, DbTemplate>,
}

impl DockerManager {
    pub fn new() -> Result<Self> {
        let docker = Docker::connect_with_local_defaults()?;
        let templates = get_db_templates();
        
        Ok(Self { docker, templates })
    }
    
    pub async fn pull_image(&self, image: &str) -> Result<()> {
        println!("Pulling image: {}", image);
        
        let options = CreateImageOptions {
            from_image: image,
            ..Default::default()
        };
        
        let mut stream = self.docker.create_image(Some(options), None, None);
        
        while let Some(result) = stream.next().await {
            match result {
                Ok(info) => {
                    if let Some(progress) = info.progress {
                        print!("\r{}", progress);
                        std::io::stdout().flush()?;
                    }
                }
                Err(e) => return Err(e.into()),
            }
        }
        
        println!("\n✓ Image pulled successfully");
        Ok(())
    }
    
    pub async fn create_database_container(
        &self,
        name: &str,
        db_type: &str,
        credentials: &DbCredentials,
    ) -> Result<String> {
        let template = self.templates.get(db_type)
            .ok_or_else(|| anyhow::anyhow!("Unsupported database type: {}", db_type))?;
        
        // Pull image if not exists
        self.pull_image(&template.image).await?;
        
        // Create volume for persistence
        let volume_name = format!("{}_data", name);
        self.create_volume(&volume_name).await?;
        
        // Build environment variables
        let env_vars = self.build_env_vars(template, name, credentials);
        
        // Create container configuration
        let config = Config {
            image: Some(template.image.clone()),
            env: Some(env_vars),
            exposed_ports: Some(HashMap::from([
                (format!("{}/tcp", template.default_port), HashMap::new())
            ])),
            host_config: Some(HostConfig {
                port_bindings: Some(HashMap::from([
                    (format!("{}/tcp", template.default_port), Some(vec![
                        PortBinding {
                            host_ip: Some("127.0.0.1".to_string()),
                            host_port: Some(credentials.port.to_string()),
                        }
                    ]))
                ])),
                binds: Some(template.volumes.iter().map(|v| {
                    v.replace("{name}", name)
                }).collect()),
                restart_policy: Some(RestartPolicy {
                    name: Some(RestartPolicyNameEnum::UNLESS_STOPPED),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        };
        
        // Create container
        let options = CreateContainerOptions { name };
        let container = self.docker.create_container(Some(options), config).await?;
        
        println!("✓ Container '{}' created successfully", name);
        Ok(container.id)
    }
    
    async fn create_volume(&self, name: &str) -> Result<()> {
        let config = CreateVolumeOptions {
            name: name.to_string(),
            ..Default::default()
        };
        
        match self.docker.create_volume(config).await {
            Ok(_) => println!("✓ Volume '{}' created", name),
            Err(bollard::errors::Error::DockerResponseServerError { status_code: 409, .. }) => {
                // Volume already exists
                println!("ℹ Volume '{}' already exists", name);
            }
            Err(e) => return Err(e.into()),
        }
        
        Ok(())
    }
    
    fn build_env_vars(
        &self,
        template: &DbTemplate,
        name: &str,
        credentials: &DbCredentials,
    ) -> Vec<String> {
        template.env_vars.iter().map(|(key, value)| {
            let resolved_value = value
                .replace("{database}", &credentials.database)
                .replace("{username}", &credentials.username)
                .replace("{password}", &credentials.password)
                .replace("{name}", name)
                .replace("{root_password}", &credentials.root_password.as_deref().unwrap_or("rootpass"));
            
            format!("{}={}", key, resolved_value)
        }).collect()
    }
    
    async fn start_container(&self, container_id: &str) -> Result<()> {
        self.docker.start_container(container_id, None::<StartContainerOptions<String>>).await?;
        println!("✓ Container started successfully");
        Ok(())
    }
    
    async fn wait_for_health(&self, container_id: &str, timeout_secs: u64) -> Result<()> {
        use tokio::time::{sleep, Duration};
        
        println!("Waiting for container to be healthy...");
        
        for i in 0..timeout_secs {
            let info = self.docker.inspect_container(container_id, None).await?;
            
            if let Some(state) = info.state {
                if let Some(health) = state.health {
                    if health.status == Some(HealthStatusEnum::HEALTHY) {
                        println!("✓ Container is healthy");
                        return Ok(());
                    }
                }
                
                if state.running == Some(true) && i > 10 {
                    // Assume healthy if running for more than 10 seconds
                    println!("✓ Container is running");
                    return Ok(());
                }
            }
            
            sleep(Duration::from_secs(1)).await;
        }
        
        Err(anyhow::anyhow!("Container failed to become healthy within {} seconds", timeout_secs))
    }
}