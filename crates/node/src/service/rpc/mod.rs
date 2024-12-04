use std::sync::Arc;

use jsonrpsee::server::ServerHandle;

use mc_db::MadaraBackend;
use mc_rpc::{providers::AddTransactionProvider, rpc_api_admin, rpc_api_user, Starknet};
use mp_utils::service::{MadaraService, Service, ServiceRunner};

use metrics::RpcMetrics;
use server::{start_server, ServerConfig};

use crate::cli::RpcParams;

use self::server::rpc_api_build;

mod metrics;
mod middleware;
mod server;

#[derive(Clone)]
pub enum RpcType {
    User,
    Admin,
}

pub struct RpcService {
    config: RpcParams,
    backend: Arc<MadaraBackend>,
    add_txs_method_provider: Arc<dyn AddTransactionProvider>,
    server_handle: Option<ServerHandle>,
    rpc_type: RpcType,
}

impl RpcService {
    pub fn user(
        config: RpcParams,
        backend: Arc<MadaraBackend>,
        add_txs_method_provider: Arc<dyn AddTransactionProvider>,
    ) -> Self {
        Self { config, backend, add_txs_method_provider, server_handle: None, rpc_type: RpcType::User }
    }

    pub fn admin(
        config: RpcParams,
        backend: Arc<MadaraBackend>,
        add_txs_method_provider: Arc<dyn AddTransactionProvider>,
    ) -> Self {
        Self { config, backend, add_txs_method_provider, server_handle: None, rpc_type: RpcType::Admin }
    }
}

#[async_trait::async_trait]
impl Service for RpcService {
    async fn start<'a>(&mut self, runner: ServiceRunner<'a>) -> anyhow::Result<()> {
        let config = self.config.clone();
        let backend = Arc::clone(&self.backend);
        let add_txs_method_provider = Arc::clone(&self.add_txs_method_provider);
        let rpc_type = self.rpc_type.clone();

        let (stop_handle, server_handle) = jsonrpsee::server::stop_channel();

        self.server_handle = Some(server_handle);

        runner.service_loop(move |mut ctx| async move {
            let starknet = Starknet::new(
                backend.clone(),
                add_txs_method_provider.clone(),
                config.storage_proof_config(),
                ctx.clone(),
            );
            let metrics = RpcMetrics::register()?;

            let server_config = {
                let (name, addr, api_rpc, rpc_version_default) = match rpc_type {
                    RpcType::User => (
                        "JSON-RPC".to_string(),
                        config.addr_user(),
                        rpc_api_user(&starknet)?,
                        mp_chain_config::RpcVersion::RPC_VERSION_LATEST,
                    ),
                    RpcType::Admin => (
                        "JSON-RPC (Admin)".to_string(),
                        config.addr_admin(),
                        rpc_api_admin(&starknet)?,
                        mp_chain_config::RpcVersion::RPC_VERSION_LATEST_ADMIN,
                    ),
                };
                let methods = rpc_api_build("rpc", api_rpc).into();

                ServerConfig {
                    name,
                    addr,
                    batch_config: config.batch_config(),
                    max_connections: config.rpc_max_connections,
                    max_payload_in_mb: config.rpc_max_request_size,
                    max_payload_out_mb: config.rpc_max_response_size,
                    max_subs_per_conn: config.rpc_max_subscriptions_per_connection,
                    message_buffer_capacity: config.rpc_message_buffer_capacity_per_connection,
                    methods,
                    metrics,
                    cors: config.cors(),
                    rpc_version_default,
                }
            };

            // Services need to be running until they are stopped or else the
            // monitor will enter an invalid state. Maybe there is a better way
            // to represent this contract but for now this works.
            start_server(server_config, ctx.clone(), stop_handle).await?;
            ctx.cancelled().await;

            anyhow::Ok(())
        });

        anyhow::Ok(())
    }

    fn id(&self) -> MadaraService {
        match self.rpc_type {
            RpcType::User => MadaraService::RpcUser,
            RpcType::Admin => MadaraService::RpcAdmin,
        }
    }
}
