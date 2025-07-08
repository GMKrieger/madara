use std::process::{Child, Command, ExitStatus, Stdio};

pub struct Server {
    process: Option<Result<Child>>,
    port: Option<u16>,
}

const CONNECTION_ATTEMPTS: usize = 720;
const CONNECTION_ATTEMPT_DELAY_MS: u64 = 1000;

impl Server {
    pub fn start_server() -> Result<(), Error> {
        // Implementation for starting the server
        unimplemented!()
    }

    pub fn endpoint(&self) -> Url {
        let addr = format!("127.0.0.1:{}", self.port.unwrap());
        Url::parse(&format!("http://{}", addr)).unwrap()
    }

    pub fn has_exited(&mut self) -> Option<ExitStatus> {
        self.process.try_wait().expect("Failed to get node exit status")
    }

    pub async fn wait_till_started(&mut self) {
        let mut attempts = CONNECTION_ATTEMPTS;
        loop {
            match TcpStream::connect(&self.address).await {
                Ok(_) => return,
                Err(err) => {
                    if let Some(status) = self.has_exited() {
                        panic!("Node exited early with {}", status);
                    }
                    if attempts == 0 {
                        panic!("Failed to connect to {}: {}", self.address, err);
                    }
                }
            };

            attempts -= 1;
            tokio::time::sleep(Duration::from_millis(CONNECTION_ATTEMPT_DELAY_MS)).await;
        }
    }

    pub fn get_pid(&self) -> u32 {
        // handle if process has ended
        self.process.id()
    }

    pub fn get_port(&self) -> u16 {
        // handle if process has ended.
        self.port.unwrap()
    }

    pub fn kill_process_on_port(&mut self) {
        let mut kill =
            Command::new("kill").args(["-s", "TERM", &self.process.id().to_string()]).spawn().expect("Failed to kill");
        kill.wait().expect("Failed to kill the process");
        self.drop_port();
    }

    fn drop_port(&mut self) {
        self.port = None;
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let mut kill =
            Command::new("kill").args(["-s", "TERM", &self.process.id().to_string()]).spawn().expect("Failed to kill");
        kill.wait().expect("Failed to kill the process");
    }
}

// Filesystem
pub trait Filesystem {
    // dump db

    // load from db
    pub fn load_db_files(paths: &Vec<Path>) {
        // Implementation here
    }

    pub fn dump_db_files(paths: &Vec<Path>) {
        // Implementation here
    }
}
