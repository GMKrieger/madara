// =============================================================================
// LOCALSTACK SERVICE - Using Docker and generic Server
// =============================================================================

const DEFAULT_LOCALSTACK_PORT: u16 = 4566;
const DEFAULT_LOCALSTACK_IMAGE: &str =
    "localstack/localstack@sha256:763947722c6c8d33d5fbf7e8d52b4bddec5be35274a0998fdc6176d733375314";
const DEFAULT_LOCALSTACK_CONTAINER_NAME: &str = "localstack-service";

#[derive(Debug, thiserror::Error)]
pub enum LocalstackError {
    #[error("Docker error: {0}")]
    Docker(#[from] DockerError),
    #[error("Localstack container already running on port {0}")]
    AlreadyRunning(u16),
    #[error("Port {0} is already in use")]
    PortInUse(u16),
}

#[derive(Debug, Clone)]
pub struct LocalstackConfig {
    pub port: u16,
    pub image: String,
    pub container_name: String,
    pub aws_prefix: Option<String>,
    pub environment_vars: Vec<(String, String)>,
}

impl Default for LocalstackConfig {
    fn default() -> Self {
        Self {
            port: DEFAULT_LOCALSTACK_PORT,
            image: DEFAULT_LOCALSTACK_IMAGE.to_string(),
            container_name: DEFAULT_LOCALSTACK_CONTAINER_NAME.to_string(),
            aws_prefix: None,
            environment_vars: vec![
                ("DEBUG".to_string(), "1".to_string()),
                ("SERVICES".to_string(), "s3,dynamodb,lambda,sqs,sns".to_string()),
            ],
        }
    }
}

pub struct LocalstackService {
    server: Server,
    config: LocalstackConfig,
}

impl LocalstackService {
    /// Start a new Localstack service
    /// Will panic if Localstack is already running as per your requirement
    pub async fn start(config: LocalstackConfig) -> Result<Self, LocalstackError> {
        // Validate Docker is running
        if !DockerServer::is_docker_running() {
            return Err(LocalstackError::Docker(DockerError::NotRunning));
        }

        // Check if container is already running - PANIC as requested
        if DockerServer::is_container_running(&config.container_name)? {
            panic!(
                "Localstack container '{}' is already running on port {}. Please stop it first.",
                config.container_name, config.port
            );
        }

        // Check if port is in use
        if DockerServer::is_port_in_use(config.port) {
            return Err(LocalstackError::PortInUse(config.port));
        }

        // Clean up any existing stopped container with the same name
        if DockerServer::does_container_exist(&config.container_name)? {
            DockerServer::remove_container(&config.container_name)?;
        }

        // Build the docker command
        let command = Self::build_docker_command(&config);

        // Create server config
        let server_config = ServerConfig {
            port: config.port,
            connection_attempts: 60, // Localstack takes longer to start
            connection_delay_ms: 2000,
            ..Default::default()
        };

        // Start the server using the generic Server::start_process
        let server = Server::start_process(command, server_config)
            .await
            .map_err(|e| LocalstackError::Docker(DockerError::Server(e)))?;

        Ok(Self { server, config })
    }

    /// Build the Docker command for Localstack
    fn build_docker_command(config: &LocalstackConfig) -> Command {
        let mut command = Command::new("docker");
        command.arg("run");
        command.arg("--rm"); // Remove container when it stops
        command.arg("--name").arg(&config.container_name);
        command.arg("-p").arg(format!("{}:{}", config.port, config.port));

        // Add environment variables
        for (key, value) in &config.environment_vars {
            command.arg("-e").arg(format!("{}={}", key, value));
        }

        // Add AWS prefix if specified
        if let Some(prefix) = &config.aws_prefix {
            command.arg("-e").arg(format!("AWS_PREFIX={}", prefix));
        }

        command.arg(&config.image);

        command
    }

    /// Validate if AWS resources with the given prefix are available
    /// This helps determine if the scenario setup is ready
    pub async fn validate_resources(&self, aws_prefix: &str) -> Result<bool, LocalstackError> {
        // This is a basic implementation - you might want to extend this
        // to check specific resources like S3 buckets, DynamoDB tables, etc.

        // Example: Check if we can connect to Localstack's health endpoint
        let health_url = format!("http://{}:{}/health", self.server.host(), self.server.port());

        match reqwest::get(&health_url).await {
            Ok(response) => {
                if response.status().is_success() {
                    // You can add more specific validation here
                    // For example, check if specific AWS resources exist
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            Err(_) => Ok(false),
        }
    }

    /// Get the endpoint URL for the Localstack service
    pub fn endpoint(&self) -> Url {
        self.server.endpoint()
    }

    /// Get the port number
    pub fn port(&self) -> u16 {
        self.server.port()
    }

    /// Get the AWS prefix
    pub fn aws_prefix(&self) -> Option<&str> {
        self.config.aws_prefix.as_deref()
    }

    /// Get the process ID
    pub fn pid(&self) -> Option<u32> {
        self.server.pid()
    }

    /// Check if the process has exited
    pub fn has_exited(&mut self) -> Option<ExitStatus> {
        self.server.has_exited()
    }

    /// Check if the service is running
    pub fn is_running(&mut self) -> bool {
        self.server.is_running()
    }

    /// Stop the Localstack service
    pub fn stop(&mut self) -> Result<(), LocalstackError> {
        self.server.stop().map_err(|e| LocalstackError::Docker(DockerError::Server(e)))
    }

    /// Get AWS endpoint URL for a specific service
    pub fn aws_endpoint(&self, service: &str) -> String {
        format!("http://{}:{}", self.server.host(), self.server.port())
    }
}
