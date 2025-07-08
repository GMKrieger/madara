// This is going to be the first test that will validate
// if the system is setting up properly or not
// System setup :

// 1. A clean Anvil instance comes alive.
// 2. Bootstrapper L1 setup completes.
// 3. A clean Madara instance comes alive.
// 4. Bootstrapper L2 setup completes.
// 5. Madara is restarted with larger block time.
// 6. Pathfinder setup completes.
// 7. Orchestrator setup happens parallel to 6.
// 8. Orchestrator runs.



#[rstest]
#[tokio::test]
async fn e2e_test_setup() {

}