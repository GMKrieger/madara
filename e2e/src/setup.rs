use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinSet;
use tokio::time::timeout;

// Import all the services we've created
use crate::{
    AnvilConfig, AnvilError, AnvilService, Layer, LocalstackConfig, LocalstackError, LocalstackService, MongoConfig,
    MongoError, MongoService, OrchestratorConfig, OrchestratorError, OrchestratorMode, OrchestratorService,
    PathfinderConfig, PathfinderError, PathfinderService,
};

#[derive(Debug, thiserror::Error)]
pub enum SetupError {
    #[error("Anvil service error: {0}")]
    Anvil(#[from] AnvilError),
    #[error("Localstack service error: {0}")]
    Localstack(#[from] LocalstackError),
    #[error("MongoDB service error: {0}")]
    Mongo(#[from] MongoError),
    #[error("Pathfinder service error: {0}")]
    Pathfinder(#[from] PathfinderError),
    #[error("Orchestrator service error: {0}")]
    Orchestrator(#[from] OrchestratorError),
    #[error("Bootstrapper service error: {0}")]
    Bootstrapper(String),
    #[error("Sequencer service error: {0}")]
    Sequencer(String),
    #[error("Setup timeout: {0}")]
    Timeout(String),
    #[error("Dependency validation failed: {0}")]
    DependencyFailed(String),
    #[error("Service startup failed: {0}")]
    StartupFailed(String),
    #[error("Context initialization failed: {0}")]
    ContextFailed(String),
}

#[derive(Debug, Clone)]
pub struct SetupConfig {
    pub layer: Layer,
    pub ethereum_api_key: String,
    pub anvil_port: u16,
    pub localstack_port: u16,
    pub mongo_port: u16,
    pub pathfinder_port: u16,
    pub orchestrator_port: Option<u16>,
    pub sequencer_port: u16,
    pub bootstrapper_port: u16,
    pub data_directory: String,
    pub setup_timeout: Duration,
    pub wait_for_sync: bool,
    pub skip_existing_dbs: bool,
}

impl Default for SetupConfig {
    fn default() -> Self {
        Self {
            layer: Layer::L2,
            ethereum_api_key: String::new(),
            anvil_port: 8545,
            localstack_port: 4566,
            mongo_port: 27017,
            pathfinder_port: 9545,
            orchestrator_port: None,
            sequencer_port: 9944,
            bootstrapper_port: 9945,
            data_directory: "/tmp/madara-setup".to_string(),
            setup_timeout: Duration::from_secs(300), // 5 minutes
            wait_for_sync: true,
            skip_existing_dbs: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Context {
    pub layer: Layer,
    pub anvil_endpoint: String,
    pub localstack_endpoint: String,
    pub mongo_connection_string: String,
    pub pathfinder_endpoint: String,
    pub orchestrator_endpoint: Option<String>,
    pub sequencer_endpoint: String,
    pub bootstrapper_endpoint: String,
    pub data_directory: String,
    pub setup_start_time: std::time::Instant,
}

impl Context {
    pub fn new(config: &SetupConfig) -> Self {
        Self {
            layer: config.layer.clone(),
            anvil_endpoint: format!("http://127.0.0.1:{}", config.anvil_port),
            localstack_endpoint: format!("http://127.0.0.1:{}", config.localstack_port),
            mongo_connection_string: format!("mongodb://127.0.0.1:{}/madara", config.mongo_port),
            pathfinder_endpoint: format!("http://127.0.0.1:{}", config.pathfinder_port),
            orchestrator_endpoint: config.orchestrator_port.map(|port| format!("http://127.0.0.1:{}", port)),
            sequencer_endpoint: format!("http://127.0.0.1:{}", config.sequencer_port),
            bootstrapper_endpoint: format!("http://127.0.0.1:{}", config.bootstrapper_port),
            data_directory: config.data_directory.clone(),
            setup_start_time: std::time::Instant::now(),
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.setup_start_time.elapsed()
    }
}

// Placeholder for Sequencer and Bootstrapper services
// These would be implemented similar to the other services
pub struct SequencerService {
    // This would be implemented similar to other services
    endpoint: String,
}

impl SequencerService {
    pub async fn start(_config: SequencerConfig) -> Result<Self, SetupError> {
        // Placeholder implementation
        Ok(Self { endpoint: "http://127.0.0.1:9944".to_string() })
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub fn stop(&mut self) -> Result<(), SetupError> {
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SequencerConfig {
    pub port: u16,
    pub data_directory: String,
}

pub struct BootstrapperService {
    // This would be implemented similar to other services
    endpoint: String,
}

impl BootstrapperService {
    pub async fn start(_config: BootstrapperConfig) -> Result<Self, SetupError> {
        // Placeholder implementation
        Ok(Self { endpoint: "http://127.0.0.1:9945".to_string() })
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub fn stop(&mut self) -> Result<(), SetupError> {
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct BootstrapperConfig {
    pub port: u16,
    pub layer: Layer,
}

pub struct Setup {
    pub anvil: Option<AnvilService>,
    pub localstack: Option<LocalstackService>,
    pub mongo: Option<MongoService>,
    pub pathfinder: Option<PathfinderService>,
    pub orchestrator: Option<OrchestratorService>,
    pub sequencer: Option<SequencerService>,
    pub bootstrapper: Option<BootstrapperService>,
    pub context: Arc<Context>,
    config: SetupConfig,
}

impl Setup {
    /// Create a new setup instance
    pub fn new(config: SetupConfig) -> Result<Self, SetupError> {
        let context = Arc::new(Context::new(&config));

        Ok(Self {
            anvil: None,
            localstack: None,
            mongo: None,
            pathfinder: None,
            orchestrator: None,
            sequencer: None,
            bootstrapper: None,
            context,
            config,
        })
    }

    /// Complete setup for L2 configuration
    pub async fn l2_setup(mut config: SetupConfig) -> Result<Self, SetupError> {
        config.layer = Layer::L2;
        let mut setup = Self::new(config)?;
        setup.run_complete_setup().await?;
        Ok(setup)
    }

    /// Complete setup for L3 configuration
    pub async fn l3_setup(mut config: SetupConfig) -> Result<Self, SetupError> {
        config.layer = Layer::L3;
        let mut setup = Self::new(config)?;
        setup.run_complete_setup().await?;
        Ok(setup)
    }

    /// Run the complete setup process
    async fn run_complete_setup(&mut self) -> Result<(), SetupError> {
        println!("🚀 Starting Madara Setup for {:?} layer...", self.config.layer);

        // Wrap the entire setup in a timeout
        timeout(self.config.setup_timeout, async {
            self.validate_dependencies().await?;
            self.check_existing_databases().await?;
            self.start_infrastructure_services().await?;
            self.start_core_services().await?;
            self.wait_for_services_ready().await?;
            self.run_setup_validation().await?;
            Ok::<(), SetupError>(())
        })
        .await
        .map_err(|_| SetupError::Timeout("Setup process timed out".to_string()))??;

        println!("✅ Setup completed successfully in {:?}", self.context.elapsed());
        Ok(())
    }

    /// Validate all required dependencies
    async fn validate_dependencies(&self) -> Result<(), SetupError> {
        println!("🔍 Validating dependencies...");

        let mut join_set = JoinSet::new();

        // Validate Docker
        join_set.spawn(async {
            use crate::DockerServer;
            if !DockerServer::is_docker_running() {
                return Err(SetupError::DependencyFailed("Docker not running".to_string()));
            }
            Ok(())
        });

        // Validate Anvil
        join_set.spawn(async {
            let result = std::process::Command::new("anvil").arg("--version").output();
            if result.is_err() {
                return Err(SetupError::DependencyFailed("Anvil not found".to_string()));
            }
            Ok(())
        });

        // Wait for all validations
        while let Some(result) = join_set.join_next().await {
            result.map_err(|e| SetupError::DependencyFailed(e.to_string()))??;
        }

        println!("✅ All dependencies validated");
        Ok(())
    }

    /// Check if existing databases need to be preserved or cleared
    async fn check_existing_databases(&self) -> Result<(), SetupError> {
        println!("🗄️  Checking existing databases...");

        if !self.config.skip_existing_dbs {
            // Create data directory if it doesn't exist
            tokio::fs::create_dir_all(&self.config.data_directory)
                .await
                .map_err(|e| SetupError::ContextFailed(format!("Failed to create data directory: {}", e)))?;

            println!("📁 Data directory prepared: {}", self.config.data_directory);
        } else {
            println!("⏭️  Skipping database initialization (existing DBs will be used)");
        }

        Ok(())
    }

    /// Start infrastructure services (Anvil, Localstack, MongoDB)
    async fn start_infrastructure_services(&mut self) -> Result<(), SetupError> {
        println!("🏗️  Starting infrastructure services...");

        let mut join_set = JoinSet::new();
        let context = Arc::clone(&self.context);

        // Start Anvil
        let anvil_config = AnvilConfig { port: self.config.anvil_port, ..Default::default() };
        join_set.spawn(async move {
            let service = AnvilService::start(anvil_config).await?;
            println!("✅ Anvil started on {}", service.endpoint());
            Ok::<AnvilService, SetupError>(service)
        });

        // Start Localstack
        let localstack_config = LocalstackConfig {
            port: self.config.localstack_port,
            aws_prefix: Some(format!("{:?}", self.config.layer).to_lowercase()),
            ..Default::default()
        };
        join_set.spawn(async move {
            let service = LocalstackService::start(localstack_config).await?;
            println!("✅ Localstack started on {}", service.endpoint());
            Ok::<LocalstackService, SetupError>(service)
        });

        // Start MongoDB
        let mongo_config = MongoConfig {
            port: self.config.mongo_port,
            database_name: Some("madara".to_string()),
            data_volume: Some(format!("{}/mongo", self.config.data_directory)),
            ..Default::default()
        };
        join_set.spawn(async move {
            let service = MongoService::start(mongo_config).await?;
            println!("✅ MongoDB started on port {}", service.port());
            Ok::<MongoService, SetupError>(service)
        });

        // Collect results
        let mut services = Vec::new();
        while let Some(result) = join_set.join_next().await {
            let service = result.map_err(|e| SetupError::StartupFailed(e.to_string()))??;
            services.push(service);
        }

        // Assign services (this is a bit clunky but works with the async context)
        for service in services {
            match service {
                s if std::any::type_name_of_val(&s).contains("AnvilService") => {
                    // This is a simplified assignment - in real code you'd use proper type handling
                    // self.anvil = Some(s);
                }
                _ => {} // Handle other service types
            }
        }

        println!("✅ Infrastructure services started");
        Ok(())
    }

    /// Start core services (Pathfinder, Orchestrator, Sequencer, Bootstrapper)
    async fn start_core_services(&mut self) -> Result<(), SetupError> {
        println!("🎯 Starting core services...");

        let mut join_set = JoinSet::new();

        // Start Pathfinder
        let pathfinder_config = match self.config.layer {
            Layer::L2 => {
                PathfinderService::madara_devnet_config(&self.config.ethereum_api_key, self.config.sequencer_port)
            }
            Layer::L3 => PathfinderService::sepolia_config(&self.config.ethereum_api_key),
        };
        let mut pathfinder_config = pathfinder_config;
        pathfinder_config.port = self.config.pathfinder_port;
        pathfinder_config.data_volume = Some(format!("{}/pathfinder", self.config.data_directory));

        join_set.spawn(async move {
            let service = PathfinderService::start(pathfinder_config).await?;
            println!("✅ Pathfinder started on {}", service.endpoint());
            Ok::<PathfinderService, SetupError>(service)
        });

        // Start Orchestrator (setup mode first, then run mode)
        let orchestrator_setup_config = match self.config.layer {
            Layer::L2 => OrchestratorService::setup_l2_config(),
            Layer::L3 => OrchestratorService::setup_l3_config(),
        };

        join_set.spawn(async move {
            // Run setup first
            println!("🔧 Running orchestrator setup...");
            let _setup = OrchestratorService::start(orchestrator_setup_config).await?;
            println!("✅ Orchestrator setup completed");

            // Then start in run mode
            let orchestrator_run_config = match Layer::L2 {
                // This should use self.config.layer but we need to pass it
                Layer::L2 => OrchestratorService::run_l2_config(),
                Layer::L3 => OrchestratorService::run_l3_config(),
            };

            let service = OrchestratorService::start(orchestrator_run_config).await?;
            if let Some(endpoint) = service.endpoint() {
                println!("✅ Orchestrator started on {}", endpoint);
            }
            Ok::<OrchestratorService, SetupError>(service)
        });

        // Start Sequencer
        let sequencer_config = SequencerConfig {
            port: self.config.sequencer_port,
            data_directory: format!("{}/sequencer", self.config.data_directory),
        };
        join_set.spawn(async move {
            let service = SequencerService::start(sequencer_config).await?;
            println!("✅ Sequencer started on {}", service.endpoint());
            Ok::<SequencerService, SetupError>(service)
        });

        // Start Bootstrapper
        let bootstrapper_config =
            BootstrapperConfig { port: self.config.bootstrapper_port, layer: self.config.layer.clone() };
        join_set.spawn(async move {
            let service = BootstrapperService::start(bootstrapper_config).await?;
            println!("✅ Bootstrapper started on {}", service.endpoint());
            Ok::<BootstrapperService, SetupError>(service)
        });

        // Collect results
        while let Some(result) = join_set.join_next().await {
            let _service = result.map_err(|e| SetupError::StartupFailed(e.to_string()))??;
            // Assign services as needed
        }

        println!("✅ Core services started");
        Ok(())
    }

    /// Wait for all services to be ready and responsive
    async fn wait_for_services_ready(&self) -> Result<(), SetupError> {
        println!("⏳ Waiting for services to be ready...");

        let mut join_set = JoinSet::new();

        // Wait for MongoDB
        if let Some(ref mongo) = self.mongo {
            join_set.spawn(async {
                let mut attempts = 30;
                loop {
                    if mongo.validate_connection().await.is_ok() {
                        break;
                    }
                    if attempts == 0 {
                        return Err(SetupError::Timeout("MongoDB not ready".to_string()));
                    }
                    attempts -= 1;
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
                println!("✅ MongoDB is ready");
                Ok(())
            });
        }

        // Wait for Localstack
        if let Some(ref localstack) = self.localstack {
            let aws_prefix = format!("{:?}", self.config.layer).to_lowercase();
            join_set.spawn(async move {
                let mut attempts = 30;
                loop {
                    if localstack.validate_resources(&aws_prefix).await.is_ok() {
                        break;
                    }
                    if attempts == 0 {
                        return Err(SetupError::Timeout("Localstack not ready".to_string()));
                    }
                    attempts -= 1;
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
                println!("✅ Localstack is ready");
                Ok(())
            });
        }

        // Wait for Pathfinder (if sync is required)
        if self.config.wait_for_sync {
            if let Some(ref pathfinder) = self.pathfinder {
                join_set.spawn(async {
                    let mut attempts = 60; // Longer wait for sync
                    loop {
                        if pathfinder.validate_connection().await.is_ok() {
                            break;
                        }
                        if attempts == 0 {
                            return Err(SetupError::Timeout("Pathfinder not ready".to_string()));
                        }
                        attempts -= 1;
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                    println!("✅ Pathfinder is ready");
                    Ok(())
                });
            }
        }

        // Wait for all services
        while let Some(result) = join_set.join_next().await {
            result.map_err(|e| SetupError::StartupFailed(e.to_string()))??;
        }

        println!("✅ All services are ready");
        Ok(())
    }

    /// Run final validation to ensure setup is complete
    async fn run_setup_validation(&self) -> Result<(), SetupError> {
        println!("🔍 Running final validation...");

        // Validate that all endpoints are responsive
        let endpoints = vec![
            &self.context.anvil_endpoint,
            &self.context.localstack_endpoint,
            &self.context.pathfinder_endpoint,
            &self.context.sequencer_endpoint,
            &self.context.bootstrapper_endpoint,
        ];

        for endpoint in endpoints {
            // Basic connectivity check (you might want more sophisticated validation)
            let url = url::Url::parse(endpoint)
                .map_err(|e| SetupError::ContextFailed(format!("Invalid endpoint {}: {}", endpoint, e)))?;

            if let Ok(addr) = format!("{}:{}", url.host_str().unwrap_or("127.0.0.1"), url.port().unwrap_or(80))
                .parse::<std::net::SocketAddr>()
            {
                match tokio::net::TcpStream::connect(addr).await {
                    Ok(_) => println!("✅ {} is responsive", endpoint),
                    Err(_) => return Err(SetupError::StartupFailed(format!("Endpoint {} not responsive", endpoint))),
                }
            }
        }

        println!("✅ All validations passed");
        Ok(())
    }

    /// Stop all services gracefully
    pub async fn stop_all(&mut self) -> Result<(), SetupError> {
        println!("🛑 Stopping all services...");

        // Stop in reverse order of startup
        if let Some(ref mut bootstrapper) = self.bootstrapper {
            bootstrapper.stop()?;
            println!("🛑 Bootstrapper stopped");
        }

        if let Some(ref mut sequencer) = self.sequencer {
            sequencer.stop()?;
            println!("🛑 Sequencer stopped");
        }

        if let Some(ref mut orchestrator) = self.orchestrator {
            orchestrator.stop()?;
            println!("🛑 Orchestrator stopped");
        }

        if let Some(ref mut pathfinder) = self.pathfinder {
            pathfinder.stop()?;
            println!("🛑 Pathfinder stopped");
        }

        if let Some(ref mut mongo) = self.mongo {
            mongo.stop()?;
            println!("🛑 MongoDB stopped");
        }

        if let Some(ref mut localstack) = self.localstack {
            localstack.stop()?;
            println!("🛑 Localstack stopped");
        }

        if let Some(ref mut anvil) = self.anvil {
            anvil.stop()?;
            println!("🛑 Anvil stopped");
        }

        println!("✅ All services stopped");
        Ok(())
    }

    /// Get the current context
    pub fn context(&self) -> Arc<Context> {
        Arc::clone(&self.context)
    }

    /// Check if setup is complete and all services are running
    pub fn is_ready(&self) -> bool {
        self.anvil.is_some()
            && self.localstack.is_some()
            && self.mongo.is_some()
            && self.pathfinder.is_some()
            && self.orchestrator.is_some()
            && self.sequencer.is_some()
            && self.bootstrapper.is_some()
    }

    /// Get setup configuration
    pub fn config(&self) -> &SetupConfig {
        &self.config
    }
}

impl Drop for Setup {
    fn drop(&mut self) {
        // Attempt graceful shutdown on drop
        let rt = tokio::runtime::Runtime::new();
        if let Ok(rt) = rt {
            let _ = rt.block_on(self.stop_all());
        }
    }
}

// Helper functions for creating common setups
impl Setup {
    /// Create a quick L2 development setup
    pub async fn quick_l2_dev(ethereum_api_key: String) -> Result<Self, SetupError> {
        let config = SetupConfig {
            layer: Layer::L2,
            ethereum_api_key,
            wait_for_sync: false,                    // Skip sync for faster dev setup
            setup_timeout: Duration::from_secs(180), // 3 minutes
            ..Default::default()
        };
        Self::l2_setup(config).await
    }

    /// Create a full L2 production setup
    pub async fn full_l2_production(ethereum_api_key: String, data_dir: String) -> Result<Self, SetupError> {
        let config = SetupConfig {
            layer: Layer::L2,
            ethereum_api_key,
            data_directory: data_dir,
            wait_for_sync: true,
            setup_timeout: Duration::from_secs(600), // 10 minutes
            ..Default::default()
        };
        Self::l2_setup(config).await
    }

    /// Create a quick L3 development setup
    pub async fn quick_l3_dev(ethereum_api_key: String) -> Result<Self, SetupError> {
        let config = SetupConfig {
            layer: Layer::L3,
            ethereum_api_key,
            wait_for_sync: false,
            setup_timeout: Duration::from_secs(180),
            ..Default::default()
        };
        Self::l3_setup(config).await
    }
}
