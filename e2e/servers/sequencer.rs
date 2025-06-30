// We write madara cmd builder here!
// Aim is to create a list of args / envs and supply that to SequencerServer
pub struct SequencerCMDBuilder {
    args: Vec<String>,
    env: HashMap<String, String>,
}

pub struct SequencerCMD {
}

// TODO: Let's ensure that orch tests and these tests use the same CMD and Builder,
// might we wanna define these somewhere else and re-export from here !


// We write all things madara here!
pub struct SequencerServer {
    // Fields and methods for the Sequencer struct

    pub fn new(args: SequencerCMD) -> Self {
        // ...
        // This is where we fetch the ports from Server and use them for gateway and rpc endpoints!

        SequencerServer {
            // Initialize fields using args
        }
    }
}

impl SequencerServer {
    // Methods for the Sequencer struct
}

impl Server for SequencerServer {
    // Methods for the Sequencer struct
}

impl Filesystem for SequencerServer {

}
