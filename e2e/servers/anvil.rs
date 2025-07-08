// Builder for constructing AnvilCMD
pub struct AnvilCMDBuilder {
    port: u16,
    fork_url: Option<String>,
    load_db: Option<String>,
    dump_db: Option<String>,
}

impl AnvilCMDBuilder {
    /// Create a new builder with default values
    pub fn new() -> Self {
        Self { port: 8545, fork_url: None, load_db: None, dump_db: None }
    }

    /// Set the port (default: 8545)
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Set the fork URL for forking from an existing network
    pub fn fork_url<S: Into<String>>(mut self, url: S) -> Self {
        self.fork_url = Some(url.into());
        self
    }

    /// Set the database file to load state from
    pub fn load_db<S: Into<String>>(mut self, path: S) -> Self {
        self.load_db = Some(path.into());
        self
    }

    /// Set the database file to dump state to
    pub fn dump_db<S: Into<String>>(mut self, path: S) -> Self {
        self.dump_db = Some(path.into());
        self
    }

    /// Build the final AnvilCMD
    pub fn build(self) -> AnvilConfig {
        AnvilConfig { port: self.port, fork_url: self.fork_url, load_db: self.load_db, dump_db: self.dump_db }
    }
}

impl Default for AnvilCMDBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// ANVIL SERVICE - Specific implementation using the generic Server
// =============================================================================

#[derive(Debug, thiserror::Error)]
pub enum AnvilError {
    #[error("Anvil is not installed on the system")]
    NotInstalled,
    #[error("Server error: {0}")]
    Server(#[from] ServerError),
}

// Configuration specific to Anvil
#[derive(Debug, Clone)]
pub struct AnvilConfig {
    pub port: u16,
    pub host: String,
    pub fork_url: Option<String>,
    pub load_db: Option<String>,
    pub dump_db: Option<String>,
}

impl Default for AnvilConfig {
    fn default() -> Self {
        Self {
            port: 8545,
            fork_url: None,
            load_db: None,
            dump_db: None,
            host: "127.0.0.1".to_string(),
        }
    }
}

// Anvil service that uses the generic Server
pub struct AnvilService {
    server: Server,
    config: AnvilConfig,
}

impl AnvilService {
    /// Start a new Anvil service with the given configuration
    pub async fn start(config: AnvilConfig) -> Result<Self, AnvilError> {
        // Validate that anvil is present in the system
        if !Self::check_anvil_installed() {
            return Err(AnvilError::NotInstalled);
        }

        // Build the anvil command
        let command = Self::build_anvil_command(&config);
        
        // Create server config
        let server_config = ServerConfig {
            port: config.port,
            host: config.host.clone(),
            ..Default::default()
        };

        // Start the server using the generic Server::start_process
        let server = Server::start_process(command, server_config).await?;
        
        Ok(Self { server, config })
    }

    /// Build the anvil command with all arguments
    fn build_anvil_command(config: &AnvilConfig) -> Command {
        let mut command = Command::new("anvil");
        command.arg("--port").arg(config.port.to_string());
        command.arg("--host").arg(&config.host);
        
        if let Some(fork_url) = &config.fork_url {
            command.arg("--fork-url").arg(fork_url);
        }
        
        if let Some(load_db) = &config.load_db {
            command.arg("--load-db").arg(load_db);
        }
        
        if let Some(dump_db) = &config.dump_db {
            command.arg("--dump-db").arg(dump_db);
        }

        command
    }

    /// Check if Anvil is installed on the system
    fn check_anvil_installed() -> bool {
        Command::new("anvil")
            .arg("--version")
            .output()
            .is_ok()
    }

    /// Get the endpoint URL for the running Anvil instance
    pub fn endpoint(&self) -> Url {
        self.server.endpoint()
    }

    /// Get the port number
    pub fn port(&self) -> u16 {
        self.server.port()
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

    /// Gracefully stop the Anvil service
    pub fn stop(&mut self) -> Result<(), AnvilError> {
        self.server.stop().map_err(AnvilError::Server)
    }
}