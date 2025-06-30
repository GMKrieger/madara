#[rstest::fixture]
pub(crate) async fn l3_setup(#[default("default_value")] config: &str, #[default(42)] port: u32) -> SomeSetupType {
    
    // sending values like AWS_PREFIX
    let new_setup = Setup {};

    new_setup.l3_setup.await?;

    new_setup.context();
}
