const DEFAULT_PATHFINDER_PORT: u16 = 9545;
const DEFAULT_PATHFINDER_IMAGE: &str = "eqlabs/pathfinder:v0.17.0-beta.2";
const DEFAULT_PATHFINDER_CONTAINER_NAME: &str = "pathfinder-service";
const DEFAULT_PATHFINDER_MONITOR_PORT: u16 = 9090;

// =============================================================================
// PATHFINDER SERVICE - Using Docker and generic Server
// =============================================================================

#[derive(Debug, thiserror::Error)]
pub enum PathfinderError {
    #[error("Docker error: {0}")]
    Docker(#[from] DockerError),
    #[error("Pathfinder container already running on port {0}")]
    AlreadyRunning(u16),
    #[error("Port {0} is already in use")]
    PortInUse(u16),
    #[error("Pathfinder connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Missing required configuration: {0}")]
    MissingConfig(String),
}

#[derive(Debug, Clone)]
pub struct PathfinderConfig {
    pub port: u16,
    pub monitor_port: u16,
    pub image: String,
    pub container_name: String,
    pub ethereum_url: String,
    pub data_directory: String,
    pub rpc_root_version: String,
    pub network: String,
    pub chain_id: String,
    pub gateway_url: Option<String>,
    pub feeder_gateway_url: Option<String>,
    pub storage_state_tries: String,
    pub gateway_request_timeout: u64,
    pub data_volume: Option<String>, // For persistent data
    pub environment_vars: Vec<(String, String)>,
}

impl Default for PathfinderConfig {
    fn default() -> Self {
        Self {
            port: DEFAULT_PATHFINDER_PORT,
            monitor_port: DEFAULT_PATHFINDER_MONITOR_PORT,
            image: DEFAULT_PATHFINDER_IMAGE.to_string(),
            container_name: DEFAULT_PATHFINDER_CONTAINER_NAME.to_string(),
            ethereum_url: "https://eth-sepolia.g.alchemy.com/v2/YOUR_API_KEY".to_string(),
            data_directory: "/var/pathfinder".to_string(),
            rpc_root_version: "v07".to_string(),
            network: "custom".to_string(),
            chain_id: "MADARA_DEVNET".to_string(),
            gateway_url: Some("http://host.docker.internal:9943/feeder".to_string()),
            feeder_gateway_url: Some("http://host.docker.internal:9943/feeder_gateway".to_string()),
            storage_state_tries: "archive".to_string(),
            gateway_request_timeout: 1000,
            data_volume: None,
            environment_vars: vec![],
        }
    }
}

pub struct PathfinderService {
    server: Server,
    config: PathfinderConfig,
}

impl PathfinderService {
    /// Start a new Pathfinder service
    /// Will panic if Pathfinder is already running as per pattern
    pub async fn start(config: PathfinderConfig) -> Result<Self, PathfinderError> {
        // Validate Docker is running
        if !DockerServer::is_docker_running() {
            return Err(PathfinderError::Docker(DockerError::NotRunning));
        }

        // Validate required configuration
        Self::validate_config(&config)?;

        // Check if container is already running - PANIC as per pattern
        if DockerServer::is_container_running(&config.container_name)? {
            panic!(
                "Pathfinder container '{}' is already running on port {}. Please stop it first.",
                config.container_name, config.port
            );
        }

        // Check if ports are in use
        if DockerServer::is_port_in_use(config.port) {
            return Err(PathfinderError::PortInUse(config.port));
        }
        if DockerServer::is_port_in_use(config.monitor_port) {
            return Err(PathfinderError::PortInUse(config.monitor_port));
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
            connection_attempts: 60, // Pathfinder takes time to sync
            connection_delay_ms: 2000,
            ..Default::default()
        };

        // Start the server using the generic Server::start_process
        let server = Server::start_process(command, server_config)
            .await
            .map_err(|e| PathfinderError::Docker(DockerError::Server(e)))?;

        Ok(Self { server, config })
    }

    /// Validate the configuration
    fn validate_config(config: &PathfinderConfig) -> Result<(), PathfinderError> {
        if config.ethereum_url.contains("YOUR_API_KEY") {
            return Err(PathfinderError::MissingConfig("ethereum_url must contain a valid API key".to_string()));
        }
        Ok(())
    }

    /// Build the Docker command for Pathfinder
    fn build_docker_command(config: &PathfinderConfig) -> Command {
        let mut command = Command::new("docker");
        command.arg("run");
        command.arg("--rm"); // Remove container when it stops
        command.arg("--name").arg(&config.container_name);

        // Port mappings
        command.arg("-p").arg(format!("{}:{}", config.port, config.port));
        command.arg("-p").arg(format!("{}:{}", config.monitor_port, config.monitor_port));

        // Add data volume if specified
        if let Some(volume) = &config.data_volume {
            command.arg("-v").arg(format!("{}:{}", volume, config.data_directory));
        }

        // Add custom environment variables
        for (key, value) in &config.environment_vars {
            command.arg("-e").arg(format!("{}={}", key, value));
        }

        // Add the image
        command.arg(&config.image);

        // Add pathfinder binary command and arguments
        command.arg("pathfinder");
        command.arg("--ethereum.url").arg(&config.ethereum_url);
        command.arg("--data-directory").arg(&config.data_directory);
        command.arg("--http-rpc").arg(format!("0.0.0.0:{}", config.port));
        command.arg("--rpc.root-version").arg(&config.rpc_root_version);
        command.arg("--monitor-address").arg(format!("0.0.0.0:{}", config.monitor_port));
        command.arg("--network").arg(&config.network);
        command.arg("--chain-id").arg(&config.chain_id);

        if let Some(ref gateway_url) = config.gateway_url {
            command.arg("--gateway-url").arg(gateway_url);
        }

        if let Some(ref feeder_gateway_url) = config.feeder_gateway_url {
            command.arg("--feeder-gateway-url").arg(feeder_gateway_url);
        }

        command.arg("--storage.state-tries").arg(&config.storage_state_tries);
        command.arg("--gateway.request-timeout").arg(config.gateway_request_timeout.to_string());

        command
    }

    /// Get the dependencies required by Pathfinder
    pub fn dependencies(&self) -> Vec<String> {
        vec!["madara".to_string(), "anvil".to_string()]
    }

    /// Validate that all required dependencies are available
    pub fn validate_dependencies(&self) -> Result<(), PathfinderError> {
        let dependencies = self.dependencies();

        for dep in dependencies {
            let result = Command::new(&dep).arg("--version").output();

            if result.is_err() {
                return Err(PathfinderError::MissingConfig(format!("Required dependency '{}' not found", dep)));
            }
        }

        Ok(())
    }

    /// Validate if Pathfinder is ready and responsive
    pub async fn validate_connection(&self) -> Result<bool, PathfinderError> {
        // Try to connect to the RPC endpoint
        let rpc_addr = format!("{}:{}", self.server.host(), self.server.port());
        match tokio::net::TcpStream::connect(&rpc_addr).await {
            Ok(_) => Ok(true),
            Err(e) => Err(PathfinderError::ConnectionFailed(e.to_string())),
        }
    }

    /// Check if Pathfinder is syncing by making an RPC call
    pub async fn get_sync_status(&self) -> Result<bool, PathfinderError> {
        // In a real implementation, you would make an RPC call to check sync status
        // For now, we'll just check if the connection is available
        self.validate_connection().await
    }

    /// Get the RPC endpoint URL
    pub fn rpc_endpoint(&self) -> Url {
        Url::parse(&format!("http://{}:{}", self.server.host(), self.server.port())).unwrap()
    }

    /// Get the monitor endpoint URL
    pub fn monitor_endpoint(&self) -> Url {
        Url::parse(&format!("http://{}:{}", self.server.host(), self.config.monitor_port)).unwrap()
    }

    /// Get the endpoint URL for the Pathfinder service (alias for rpc_endpoint)
    pub fn endpoint(&self) -> Url {
        self.rpc_endpoint()
    }

    /// Get the monitor port number
    pub fn monitor_port(&self) -> u16 {
        self.config.monitor_port
    }

    /// Get the network name
    pub fn network(&self) -> &str {
        &self.config.network
    }

    /// Get the chain ID
    pub fn chain_id(&self) -> &str {
        &self.config.chain_id
    }

    /// Get the Ethereum URL
    pub fn ethereum_url(&self) -> &str {
        &self.config.ethereum_url
    }

    pub fn server(&self) -> &Server {
        &self.server
    }

    // dump and load from db fns!
    // volume attachment !
}
