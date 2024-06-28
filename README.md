<!-- markdownlint-disable -->
<div align="center">
    <img src="https://github.com/KasarLabs/brand/blob/main/projects/deoxys/Full/GradientFullWhite.png?raw=true" height="125" style="border-radius: 15px;">
</div>
<div align="center">
<br />
<!-- markdownlint-restore -->

[![Workflow - Push](https://github.com/KasarLabs/deoxys/actions/workflows/push.yml/badge.svg)](https://github.com/KasarLabs/deoxys/actions/workflows/push.yml)
[![Project license](https://img.shields.io/github/license/kasarLabs/deoxys.svg?style=flat-square)](LICENSE)
[![Pull Requests welcome](https://img.shields.io/badge/PRs-welcome-ff69b4.svg?style=flat-square)](https://github.com/kasarLabs/deoxys/issues?q=is%3Aissue+is%3Aopen+label%3A%22help+wanted%22)
<a href="https://twitter.com/KasarLabs">
<img src="https://img.shields.io/twitter/follow/KasarLabs?style=social"/> </a>
<a href="https://github.com/kasarlabs/deoxys">
<img src="https://img.shields.io/github/stars/kasarlabs/deoxys?style=social"/>
</a>

</div>

# 👽 Deoxys: Starknet full node client on Substrate

## ⬇️ Installation

### Run from Source

1. **Install dependencies**

    Ensure you have the necessary dependencies:

    ```bash
    sudo apt-get update && sudo apt-get install -y \
      clang \
      protobuf-compiler \
      build-essential
    ```

    Install Rust:

    ```bash
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s
    ```

    Clone the Deoxys repository:

    ```bash
    cd <your-destination-path>
    git clone https://github.com/KasarLabs/deoxys .
    ```

2. **Run Deoxys**

    Start the Deoxys client with synchronization to Starknet mainnet:

    ```bash
    cargo run --release \
      --name deoxys \
      --base-path ../deoxys-db \
      --network main \
      --l1-endpoint ${ETHEREUM_API_URL} \
      --chain starknet \
      --rpc-port 9944 \
      --rpc-cors "*" \
      --rpc-external
    ```

### Run with Docker

1 **Run docker image**

    To run Deoxys with Docker, use the following command:

    ```bash
    docker run -d \
        --name deoxys \
        -p 9944:9944 \
        -v /var/lib/deoxys:/var/lib/deoxys \
        deoxys:latest \
        --base-path ../deoxys-db \
        --network main \
        --l1-endpoint <rpc key> \
        --rpc-port 9944 \
        --rpc-cors "*" \
        --rpc-external
    ```

    Check the logs of the running Deoxys service:

    ```bash
    docker logs -f deoxys
    ```

### Using Docker Compose

1. **Ensure environment variable**

    Set the necessary environment variable:

    ```bash
    export ETHEREUM_API_URL="your-ethereum-api-url"
    ```

    Or create a `.env` file in the same directory as your `docker-compose.yml` file:

    ```
    ETHEREUM_API_URL=your-ethereum-api-url
    ```

2. **Build and Run the Container**

    Navigate to the directory with your `docker-compose.yml` file and run:

    ```bash
    docker-compose up -d
    ```

    Check the logs of the running Deoxys service:

    ```bash
    docker-compose logs -f deoxys
    ```

## ⚙️ Configuration

Configuring your Deoxys node properly ensures it meets your specific needs

### Basic Command-Line Options

Here are the recommended options for a quick and simple configuration of your Deoxys full node:

- **`--name <NAME>`**: The human-readable name for this node. It's used as the network node name.
- **`--base-path <PATH>`**: Set the directory for Starknet data (default is `/tmp/deoxys`).
- **`--network <NETWORK>`**: The network type to connect to (`main`, `test`, or `integration`).
- **`--l1-endpoint <URL>`**: Specify the Layer 1 endpoint the node will verify its state from.
- **`--rpc-port <PORT>`**: Specify the JSON-RPC server TCP port.
- **`--rpc-cors <ORIGINS>`**: Specify browser origins allowed to access the HTTP & WS RPC servers.
- **`--rpc-external`**: Listen to all RPC interfaces. Default is local.
- **`--snap <BLOCK_NUMBER>`**: Start syncing from the closest snapshot available for the desired block (default is highest).

### Advanced Command-Line Options by Namespace

<details>
<summary>Network</summary>

- **`-n, --network <NETWORK>`**: The network type to connect to (default: `integration`).
- **`--port <PORT>`**: Set the network listening port.
- **`--l1-endpoint <URL>`**: Specify the Layer 1 endpoint the node will verify its state from.
- **`--gateway-key <GATEWAY_KEY>`**: Gateway API key to avoid rate limiting (optional).
- **`--sync-polling-interval <SECONDS>`**: Polling interval in seconds (default: 2).
- **`--no-sync-polling`**: Stop sync polling.
- **`--n-blocks-to-sync <NUMBER>`**: Number of blocks to sync.
- **`--starting-block <BLOCK>`**: The block to start syncing from (make sure to set `--disable-root`).

</details>

<details>
<summary>RPC</summary>

- **`--rpc-external`**: Listen to all RPC interfaces. Note: not all RPC methods are safe to be exposed publicly. Use an RPC proxy server to filter out dangerous methods.
- **`--rpc-methods <METHOD_SET>`**: RPC methods to expose (`auto`, `safe`, `unsafe`).
- **`--rpc-max-request-size <SIZE>`**: Set the maximum RPC request payload size in megabytes (default: 15).
- **`--rpc-max-response-size <SIZE>`**: Set the maximum RPC response payload size in megabytes (default: 15).
- **`--rpc-max-subscriptions-per-connection <NUMBER>`**: Set the maximum concurrent subscriptions per connection (default: 1024).
- **`--rpc-port <PORT>`**: Specify JSON-RPC server TCP port.
- **`--rpc-max-connections <NUMBER>`**: Maximum number of RPC server connections (default: 100).
- **`--rpc-cors <ORIGINS>`**: Specify browser origins allowed to access the HTTP & WS RPC servers.

</details>

<details>
<summary>Database</summary>

- **`--base-path <PATH>`**: Specify custom base path (default: `/tmp/deoxys`).
- **`--snap <BLOCK_NUMBER>`**: Start syncing from the closest snapshot available for the desired block.
- **`--tmp`**: Run a temporary node. A temporary directory will be created and deleted at the end of the process.
- **`--cache`**: Enable caching of blocks and transactions to improve response times.
- **`--db-cache <MiB>`**: Limit the memory the database cache can use.
- **`--trie-cache-size <Bytes>`**: Specify the state cache size (default: 67108864).
- **`--backup-every-n-blocks <NUMBER>`**: Specify the number of blocks after which a backup should be created.
- **`--backup-dir <DIR>`**: Specify the directory where backups should be stored.
- **`--restore-from-latest-backup`**: Restore the database from the latest backup available.

</details>

## 📸 Snapshots

Snapshots are under developpement and will be available trought the `--snap <block_number>` parameter.

## 🌐 Interactions

Deoxys fully supports all the JSON-RPC methods as specified in the Starknet mainnet official [JSON-RPC specs](https://github.com/starkware-libs/starknet-specs). These methods can be categorized into three main types: Read-Only Access Methods, Trace Generation Methods, and Write Methods. Below is an overview of how you can interact with your full node using these methods.

### Supported JSON-RPC Methods

#### Read Methods

| Status | Method |
| ------ | ------ |
| ✅ | `starknet_specVersion` |
| ✅ | `starknet_getBlockWithTxHashes` |
| ✅ | `starknet_getBlockWithReceipts` |
| ✅ | `starknet_getBlockWithTxs` |
| ✅ | `starknet_getStateUpdate` |
| ✅ | `starknet_getStorageAt` |
| ✅ | `starknet_getTransactionStatus` |
| ✅ | `starknet_getTransactionByHash` |
| ✅ | `starknet_getTransactionByBlockIdAndIndex` |
| ✅ | `starknet_getTransactionReceipt` |
| ✅ | `starknet_getClass` |
| ✅ | `starknet_getClassHashAt` |
| ✅ | `starknet_getClassAt` |
| ✅ | `starknet_getBlockTransactionCount` |
| ✅ | `starknet_call` |
| ✅ | `starknet_estimateFee` |
| ✅ | `starknet_estimateMessageFee` |
| ✅ | `starknet_blockNumber` |
| ✅ | `starknet_blockHashAndNumber` |
| ✅ | `starknet_chainId` |
| ✅ | `starknet_syncing` |
| ✅ | `starknet_getEvents` |
| ✅ | `starknet_getNonce` |

#### Traces Methods

| Status | Method |
| ------ | ------ |
| ✅ | `starknet_traceTransaction` |
| ✅ | `starknet_simulateTransactions` |
| ✅ | `starknet_traceBlockTransactions` |

#### Write Methods

| Status | Method |
| ------ | ------ |
| ✅ | `starknet_addInvokeTransaction` |
| ✅ | `starknet_addDeclareTransaction` |
| ✅ | `starknet_addDeployAccountTransaction` |

### Example of Calling a JSON-RPC Method

Here is an example of how to call a JSON-RPC method using Deoxys:

```json
{
    "jsonrpc": "2.0",
    "method": "starknet_getBlockWithTxHashes",
    "params": {
        "block_id": "latest"
    },
    "id": 1
}
```

You can use any JSON-RPC client to interact with the Deoxys node, such as `curl`, `httpie`, or a custom client in your preferred programming language. For more detailed information and examples on each method, please refer to the [Starknet JSON-RPC specs](https://github.com/starkware-libs/starknet-specs).

**Note**: Write methods are forwarded to the Sequencer for execution. Ensure you handle errors appropriately as per the JSON-RPC schema.

For a comprehensive list of all supported JSON-RPC methods, please refer to the [documentation](https://github.com/starkware-libs/starknet-specs).

## ✔ Supported Features

## 👍 Contribute

## 🤝 Partnerships

To establish
