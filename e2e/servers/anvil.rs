// We write madara cmd builder here!
pub struct AnvilCMDBuilder {}

pub struct AnvilCMD {
    // defaults to 8545, might remove option
    port: u16,
    fork_url: Option<String>,
    load_db: Option<String>,
    dump_db: Option<String>,
}

impl Default for AnvilCMD {
    fn default() -> Self {
        Self { port: 8545, fork_url: None, load_db: None, dump_db: None }
    }
}

// We write all things madara here!
pub struct AnvilServer {
    inner: Server, // Fields and methods for the Anvil struct
}

impl AnvilServer {
    // Methods for the Anvil struct

    pub fn start(commands: AnvilCMD) -> Self {
        // validate that anvil is present in the system.
        if !Self::check_anvil_installed() {
            panic!("Anvil is not installed");
        }

        // Running anvil !
        let mut command = Command::new("anvil");
        command.arg("--port").arg(commands.port.to_string())
        if commands.fork_url.is_some() {
            command.arg("--fork-url").arg(commands.fork_url.unwrap_or_default())
        }
        if commands.load_db.is_some() {
            command.arg("--load-db").arg(commands.load_db.unwrap_or_default())
        }
        if commands.dump_db.is_some() {
            command.arg("--dump-db").arg(commands.dump_db.unwrap_or_default())
        }

        command.stdout(Stdio::piped()).stderr(Stdio::piped());

        // maybe use arn and send it to server!
        let mut process = command.spawn().expect("Failed to start process");

        Self { inner: Server::new(process, commands.port) }
    }

    pub fn check_anvil_installed() -> bool {
        Command::new("anvil").arg("--version").output().is_ok()
    }
}

impl Filesystem for AnvilServer {
    // UNSURE IF WE REALLY NEED IT THO
    fn load_db_files(paths: &Vec<Path>) {
        // Implementation here
    }

    fn dump_db_files(paths: &Vec<Path>) {
        // Implementation here
    }


}
