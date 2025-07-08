use crate::servers::docker::DockerError;

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
