// timebound testcase!
// 



#[rstest::rstest]
#[tokio::test]
async fn brige_deposit_test(
    #[future]
    #[with("custom_config", 8080)]
    l2_setup: SomeSetupType,
) {
    AWS_PRIFIX = "scenario1"
    DB_NAME = "scenario1"
    
    // Your test code here
}


