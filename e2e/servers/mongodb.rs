// =============================================================================
// MONGODB SERVICE - Using Docker and generic Server
// =============================================================================

const DEFAULT_MONGO_PORT: u16 = 27017;
const DEFAULT_MONGO_IMAGE: &str = "mongo:latest";
const DEFAULT_MONGO_CONTAINER_NAME: &str = "mongodb-service";

#[derive(Debug, thiserror::Error)]
pub enum MongoError {
    #[error("Docker error: {0}")]
    Docker(#[from] DockerError),
    #[error("MongoDB container already running on port {0}")]
    AlreadyRunning(u16),
    #[error("Port {0} is already in use")]
    PortInUse(u16),
    #[error("MongoDB connection failed: {0}")]
    ConnectionFailed(String),
}

#[derive(Debug, Clone)]
pub struct MongoConfig {
    pub port: u16,
    pub image: String,
    pub container_name: String,
}

impl Default for MongoConfig {
    fn default() -> Self {
        Self {
            port: DEFAULT_MONGO_PORT,
            image: DEFAULT_MONGO_IMAGE.to_string(),
            container_name: DEFAULT_MONGO_CONTAINER_NAME.to_string(),
        }
    }
}

pub struct MongoService {
    server: Server,
    config: MongoConfig,
}

impl MongoService {
    /// Start a new MongoDB service
    /// Will panic if MongoDB is already running as per pattern
    pub async fn start(config: MongoConfig) -> Result<Self, MongoError> {
        // Validate Docker is running
        if !DockerServer::is_docker_running() {
            return Err(MongoError::Docker(DockerError::NotRunning));
        }

        // Check if container is already running - PANIC as per pattern
        if DockerServer::is_container_running(&config.container_name)? {
            panic!(
                "MongoDB container '{}' is already running on port {}. Please stop it first.",
                config.container_name, config.port
            );
        }

        // Check if port is in use
        if DockerServer::is_port_in_use(config.port) {
            return Err(MongoError::PortInUse(config.port));
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
            connection_attempts: 30, // MongoDB usually starts quickly
            connection_delay_ms: 1000,
            ..Default::default()
        };

        // Start the server using the generic Server::start_process
        let server = Server::start_process(command, server_config)
            .await
            .map_err(|e| MongoError::Docker(DockerError::Server(e)))?;

        Ok(Self { server, config })
    }

    /// Build the Docker command for MongoDB
    fn build_docker_command(config: &MongoConfig) -> Command {
        let mut command = Command::new("docker");
        command.arg("run");
        command.arg("--rm"); // Remove container when it stops
        command.arg("--name").arg(&config.container_name);
        command.arg("-p").arg(format!("{}:27017", config.port));

        command.arg(&config.image);

        command
    }

    /// Validate if MongoDB is ready and responsive
    pub async fn validate_connection(&self) -> Result<bool, MongoError> {
        // Basic validation - try to connect to MongoDB
        // In a real implementation, you might want to use the MongoDB driver
        // For now, we'll just check if the port is responding

        let addr = format!("{}:{}", self.server.host(), self.server.port());
        match tokio::net::TcpStream::connect(&addr).await {
            Ok(_) => Ok(true),
            Err(e) => Err(MongoError::ConnectionFailed(e.to_string())),
        }
    }

    /// Get the endpoint URL for the MongoDB service
    pub fn endpoint(&self) -> Url {
        // MongoDB doesn't use HTTP, but we'll return the TCP endpoint
        Url::parse(&format!("mongodb://{}:{}", self.server.host(), self.server.port())).unwrap()
    }

    pub fn server(&self) -> &Server {
        &self.server
    }
}
