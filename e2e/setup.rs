// We write all things madara here!
pub struct Setup {
    pub sequencer : SequencerServer,
    pub orchestrator : OrchestratorServer,
    pub fullnode : PathfinderServer,
    pub bootstrapper : BootstrapperServer,
    
    
    pub context : Arc<Context>
    
    // Fields and methods for the Sequencer struct
}

impl Setup {
    pub fn new() {
            
        // We take inspiration from orchestrator's resoruce setup.
        // We use 
        //  - JoinSet
        //  - Context
        
    }

    pub fn l2_setup(config : ) {}

    pub fn l3_setup() {}

}
