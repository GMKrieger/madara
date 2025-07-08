// For localstack we have to use the localstack rule
// Setup
// cargo run --bin orchestrator setup --layer l2 --aws --aws-s3 --aws-sqs --aws-sns --aws-event-bridge --event-bridge-type rule
// cargo run --bin orchestrator setup --layer l3 --aws --aws-s3 --aws-sqs --aws-sns --aws-event-bridge --event-bridge-type rule

// Run
// cargo run --bin orchestrator run --layer l2 --aws --settle-on-ethereum --aws-s3 --aws-sqs --aws-sns --da-on-ethereum --mongodb --atlantic
// cargo run --bin orchestrator run --layer l3 --aws --settle-on-starknet --aws-s3 --aws-sqs --aws-sns --da-on-starknet --mongodb --atlantic

// =============================================================================
// ORCHESTRATOR SERVICE - Using generic Server
// =============================================================================

use std::path::PathBuf;
use strum::Display;

#[derive(Display, Debug, Clone, PartialEq, Eq)]
pub enum OrchestratorMode {
    #[strum(serialize = "run")]
    Run,
    #[strum(serialize = "setup")]
    Setup,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Layer {
    L2,
    L3,
}

impl std::fmt::Display for Layer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Layer::L2 => write!(f, "l2"),
            Layer::L3 => write!(f, "l3"),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum OrchestratorError {
    #[error("Repository root not found")]
    RepositoryRootNotFound,
    #[error("Failed to change working directory: {0}")]
    WorkingDirectoryFailed(std::io::Error),
    #[error("Server error: {0}")]
    Server(#[from] ServerError),
    #[error("Setup mode failed with exit code: {0}")]
    SetupFailed(i32),
    #[error("Missing required dependency: {0}")]
    MissingDependency(String),
}

#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    pub mode: OrchestratorMode,
    pub layer: Layer,
    pub port: Option<u16>,
    pub repository_root: Option<PathBuf>,
    pub environment_vars: Vec<(String, String)>,

    // AWS Configuration
    pub aws: bool,
    pub aws_s3: bool,
    pub aws_sqs: bool,
    pub aws_sns: bool,
    pub aws_event_bridge: bool,
    pub event_bridge_type: Option<String>,

    // Layer-specific options
    pub settle_on_ethereum: bool,
    pub settle_on_starknet: bool,
    pub da_on_ethereum: bool,
    pub da_on_starknet: bool,
    pub sharp: bool,
    pub mongodb: bool,
    pub atlantic: bool,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            mode: OrchestratorMode::Run,
            layer: Layer::L2,
            port: None,
            repository_root: None,
            environment_vars: vec![],
            aws: true,
            aws_s3: true,
            aws_sqs: true,
            aws_sns: true,
            aws_event_bridge: false,
            event_bridge_type: None,
            settle_on_ethereum: true,
            settle_on_starknet: false,
            da_on_ethereum: true,
            da_on_starknet: false,
            sharp: false,
            mongodb: true,
            atlantic: false,
        }
    }
}

pub struct OrchestratorService {
    server: Option<Server>, // None for setup mode
    config: OrchestratorConfig,
    address: Option<String>,
}

impl OrchestratorService {
    /// Start the orchestrator service
    pub async fn start(mut config: OrchestratorConfig) -> Result<Self, OrchestratorError> {
        // Set repository root if not provided
        if config.repository_root.is_none() {
            config.repository_root = Some(Self::get_repository_root()?);
        }

        let repository_root = config.repository_root.as_ref().unwrap();

        // Change to repository root directory
        std::env::set_current_dir(repository_root).map_err(OrchestratorError::WorkingDirectoryFailed)?;

        match config.mode {
            OrchestratorMode::Setup => Self::run_setup_mode(config).await,
            OrchestratorMode::Run => Self::run_run_mode(config).await,
        }
    }

    /// Run in setup mode (blocking, returns when complete)
    async fn run_setup_mode(config: OrchestratorConfig) -> Result<Self, OrchestratorError> {
        let command = Self::build_setup_command(&config);

        println!("Running orchestrator in setup mode");

        // For setup mode, we run the command directly and wait for completion
        let mut child = command.spawn().map_err(|e| OrchestratorError::Server(ServerError::StartupFailed(e)))?;

        // Wait for the process to complete
        let status = child.wait().map_err(|e| OrchestratorError::Server(ServerError::Io(e)))?;

        if status.success() {
            println!("Orchestrator cloud setup completed ✅");
            Ok(Self { server: None, config, address: None })
        } else {
            let exit_code = status.code().unwrap_or(-1);
            Err(OrchestratorError::SetupFailed(exit_code))
        }
    }

    /// Run in run mode (async, returns immediately with running server)
    async fn run_run_mode(mut config: OrchestratorConfig) -> Result<Self, OrchestratorError> {
        // Get a free port if not specified
        if config.port.is_none() {
            config.port = Some(Self::get_free_port());
        }

        let port = config.port.unwrap();
        let address = format!("127.0.0.1:{}", port);

        // Add port to environment variables
        config.environment_vars.push(("MADARA_ORCHESTRATOR_PORT".to_string(), port.to_string()));

        let command = Self::build_run_command(&config);

        println!("Running orchestrator in run mode on {}", address);

        // Create server config
        let server_config = ServerConfig {
            port,
            connection_attempts: 60, // Orchestrator might take time to start
            connection_delay_ms: 2000,
            ..Default::default()
        };

        // Start the server using the generic Server::start_process
        let server = Server::start_process(command, server_config).await?;

        Ok(Self { server: Some(server), config, address: Some(address) })
    }

    /// Build command for setup mode
    fn build_setup_command(config: &OrchestratorConfig) -> Command {
        let mut command = Command::new("cargo");
        command
            .arg("run")
            .arg("--release")
            .arg("-p")
            .arg("orchestrator")
            .arg("--features")
            .arg("testing")
            .arg("setup")
            .arg(&format!("--layer={}", config.layer));

        // Add AWS flags
        if config.aws {
            command.arg("--aws");
        }
        if config.aws_s3 {
            command.arg("--aws-s3");
        }
        if config.aws_sqs {
            command.arg("--aws-sqs");
        }
        if config.aws_sns {
            command.arg("--aws-sns");
        }
        if config.aws_event_bridge {
            command.arg("--aws-event-bridge");
        }
        if let Some(ref event_bridge_type) = config.event_bridge_type {
            command.arg("--event-bridge-type").arg(event_bridge_type);
        }

        // For setup mode, inherit stdio to show output directly
        command.stdout(Stdio::inherit()).stderr(Stdio::inherit());

        // Add environment variables
        for (key, value) in &config.environment_vars {
            command.env(key, value);
        }

        if let Some(ref repo_root) = config.repository_root {
            command.current_dir(repo_root);
        }

        command
    }

    /// Build command for run mode
    fn build_run_command(config: &OrchestratorConfig) -> Command {
        let mut command = Command::new("cargo");
        command
            .arg("run")
            .arg("--release")
            .arg("-p")
            .arg("orchestrator")
            .arg("--features")
            .arg("testing")
            .arg("run")
            .arg(&format!("--layer={}", config.layer));

        // Add AWS flags
        if config.aws {
            command.arg("--aws");
        }
        if config.aws_s3 {
            command.arg("--aws-s3");
        }
        if config.aws_sqs {
            command.arg("--aws-sqs");
        }
        if config.aws_sns {
            command.arg("--aws-sns");
        }

        // Add settlement and DA options
        if config.settle_on_ethereum {
            command.arg("--settle-on-ethereum");
        }
        if config.settle_on_starknet {
            command.arg("--settle-on-starknet");
        }
        if config.da_on_ethereum {
            command.arg("--da-on-ethereum");
        }
        if config.da_on_starknet {
            command.arg("--da-on-starknet");
        }
        if config.sharp {
            command.arg("--sharp");
        }
        if config.mongodb {
            command.arg("--mongodb");
        }
        if config.atlantic {
            command.arg("--atlantic");
        }

        // For run mode, pipe stdout and stderr
        command.stdout(Stdio::piped()).stderr(Stdio::piped());

        // Add environment variables
        for (key, value) in &config.environment_vars {
            command.env(key, value);
        }

        if let Some(ref repo_root) = config.repository_root {
            command.current_dir(repo_root);
        }

        command
    }

    /// Get the repository root directory
    fn get_repository_root() -> Result<PathBuf, OrchestratorError> {
        // Try to find git repository root
        let mut current_dir = std::env::current_dir().map_err(OrchestratorError::WorkingDirectoryFailed)?;

        loop {
            if current_dir.join(".git").exists() {
                return Ok(current_dir);
            }

            if let Some(parent) = current_dir.parent() {
                current_dir = parent.to_path_buf();
            } else {
                break;
            }
        }

        // Fallback to current directory
        std::env::current_dir().map_err(OrchestratorError::WorkingDirectoryFailed)
    }

    /// Get the dependencies required by the orchestrator
    pub fn dependencies(&self) -> Vec<String> {
        vec![
            // internal
            "anvil".to_string(),
            "madara".to_string(),
            "pathfinder".to_string(),
            // TODO: Actually bootstrapper is not a direct dep of orchestrator
            // we can remove this
            "bootstrapper_l1".to_string(),
            "bootstrapper_l2".to_string(),
            // external
            "atlantic".to_string(),
            "localstack".to_string(),
            "mongodb".to_string(),
        ]
    }

    /// Validate that all required dependencies are available and running
    /// TODO: might move this to a a fn in setup
    pub fn validate_dependencies(&self) -> Result<(), OrchestratorError> {
        // TODO: complete this!
        let dependencies = self.dependencies();

        for dep in dependencies {
            // For now, just check if the command exists
            // You might want to implement more sophisticated checking
            let result = Command::new(&dep).arg("--version").output();

            if result.is_err() {
                return Err(OrchestratorError::MissingDependency(dep));
            }
        }

        Ok(())
    }
    /// Create a setup configuration for L2
    pub fn setup_l2_config() -> OrchestratorConfig {
        OrchestratorConfig {
            mode: OrchestratorMode::Setup,
            layer: Layer::L2,
            aws_event_bridge: true,
            event_bridge_type: Some("rule".to_string()),
            ..Default::default()
        }
    }

    /// Create a setup configuration for L3
    pub fn setup_l3_config() -> OrchestratorConfig {
        OrchestratorConfig {
            mode: OrchestratorMode::Setup,
            layer: Layer::L3,
            aws_event_bridge: true,
            event_bridge_type: Some("rule".to_string()),
            ..Default::default()
        }
    }

    /// Create a run configuration for L2
    pub fn run_l2_config() -> OrchestratorConfig {
        OrchestratorConfig {
            mode: OrchestratorMode::Run,
            layer: Layer::L2,
            settle_on_ethereum: true,
            da_on_ethereum: true,
            mongodb: true,
            atlantic: true,
            ..Default::default()
        }
    }

    /// Create a run configuration for L3
    pub fn run_l3_config() -> OrchestratorConfig {
        OrchestratorConfig {
            mode: OrchestratorMode::Run,
            layer: Layer::L3,
            settle_on_starknet: true,
            da_on_starknet: true,
            mongodb: true,
            atlantic: true,
            ..Default::default()
        }
    }

    /// Get the endpoint URL for the orchestrator service (run mode only)
    pub fn endpoint(&self) -> Option<Url> {
        // TODO: validate run mode is being used
        if let Some(ref address) = self.address {
            Url::parse(&format!("http://{}", address)).ok()
        } else {
            None
        }
    }

    /// Get the current mode
    pub fn mode(&self) -> &OrchestratorMode {
        &self.config.mode
    }

    /// Get the port number (run mode only)
    pub fn port(&self) -> Option<u16> {
        self.config.port
    }

    /// Get the layer
    pub fn layer(&self) -> &Layer {
        &self.config.layer
    }

    pub fn server(&self) -> &Server {
        &self.server
    }
}
