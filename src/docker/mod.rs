use shiplift::{Docker, PullOptions, ContainerOptions, RmContainerOptions};
use std::collections::HashMap;


pub struct DockerManager {
    docker: Docker,
}

impl DockerManager {
    fn new() -> Self {
        let docker = Docker::new();
        Self { docker }
    }
    
    async fn pull_image(&self, image: &str) -> Result<(), Box<dyn std::error::Error>> {
        println!("Pulling image: {}", image);
        
        let mut stream = self.docker.images().pull(&PullOptions::builder().image(image).build());
        
        while let Some(pull_result) = stream.next().await {
            match pull_result {
                Ok(output) => {
                    if let Some(status) = output.status {
                        println!("Status: {}", status);
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
    ) -> Result<String, Box<dyn std::error::Error>> {
        
        let mut container_options = ContainerOptions::builder(image)
            .name(name)
            .env(env_vars.iter().map(|s| s.as_str()).collect::<Vec<_>>());
        
        // Add port mappings
        for (container_port, host_port) in port_mappings {
            container_options = container_options.expose(&container_port, "tcp", &host_port);
        }
        
        // Add volumes
        for volume in volumes {
            container_options = container_options.volumes(vec![&volume]);
        }
        
        let container = self.docker.containers().create(&container_options.build()).await?;
        
        println!("✓ Container '{}' created with ID: {}", name, container.id);
        Ok(container.id)
    }
    
    async fn start_container(&self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.docker.containers().get(id).start().await?;
        println!("✓ Container started");
        Ok(())
    }
    
    async fn create_database_container(
        &self,
        name: &str,
        db_type: &str,
        credentials: &DbCredentials,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let template = get_db_template(db_type)?;
        
        // Pull image
        self.pull_image(&template.image).await?;
        
        // Build environment variables
        let env_vars = build_env_vars(&template, name, credentials);
        
        // Build port mapping
        let mut port_mappings = HashMap::new();
        port_mappings.insert(
            format!("{}/tcp", template.default_port),
            credentials.port.to_string(),
        );
        
        // Build volumes
        let volumes = template.volumes.iter()
            .map(|v| v.replace("{name}", name))
            .collect();
        
        // Create container
        let container_id = self.create_container(
            name,
            &template.image,
            env_vars,
            port_mappings,
            volumes,
        ).await?;
        
        // Start container
        self.start_container(&container_id).await?;
        
        Ok(container_id)
    }

    fn build_env_vars(
        template: &DbTemplate,
        name: &str,
        credentials: &DbCredentials,
    ) -> Vec<String> {
        template.env_vars.iter().map(|(key, value)| {
            value.replace("{name}", name)
                .replace("{username}", &credentials.username)
                .replace("{password}", &credentials.password)
                .replace("{database}", &credentials.database)
                .replace("{port}", &credentials.port.to_string())
                .replace("{root_password}", credentials.root_password.as_deref().unwrap_or(""))
        }).collect()
    }
}