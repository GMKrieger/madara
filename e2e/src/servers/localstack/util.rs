use crate::servers::docker::DockerError;

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

const DEFAULT_LOCALSTACK_PORT: u16 = 4566;
const DEFAULT_LOCALSTACK_IMAGE: &str =
    "localstack/localstack@sha256:763947722c6c8d33d5fbf7e8d52b4bddec5be35274a0998fdc6176d733375314";
const DEFAULT_LOCALSTACK_CONTAINER_NAME: &str = "localstack-service";

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
