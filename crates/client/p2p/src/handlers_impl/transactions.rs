//! TODO: range check contract addresses?

use super::FromModelError;
use crate::{
    handlers_impl::{
        block_stream_config,
        error::{OptionExt, ResultExt},
    },
    model::{self, receipt::execution_resources::BuiltinCounter},
    sync_handlers::{self, ReqContext},
    MadaraP2pContext,
};
use futures::{channel::mpsc::Sender, SinkExt, Stream, StreamExt};
use mc_db::db_block_id::DbBlockId;
use mp_block::TransactionWithReceipt;
use mp_convert::{felt_to_u128, felt_to_u32, felt_to_u64};
use mp_receipt::{
    DeclareTransactionReceipt, DeployAccountTransactionReceipt, DeployTransactionReceipt, ExecutionResources,
    ExecutionResult, FeePayment, InvokeTransactionReceipt, L1Gas, L1HandlerTransactionReceipt, MsgToL1, PriceUnit,
    TransactionReceipt,
};
use mp_transactions::{
    DataAvailabilityMode, DeclareTransaction, DeclareTransactionV0, DeclareTransactionV1, DeclareTransactionV2,
    DeclareTransactionV3, DeployAccountTransaction, DeployAccountTransactionV1, DeployAccountTransactionV3,
    DeployTransaction, InvokeTransaction, InvokeTransactionV0, InvokeTransactionV1, InvokeTransactionV3,
    L1HandlerTransaction, ResourceBounds, ResourceBoundsMapping, Transaction, TransactionWithHash,
};
use starknet_core::types::Felt;
use tokio::pin;

impl From<TransactionWithReceipt> for model::TransactionWithReceipt {
    fn from(value: TransactionWithReceipt) -> Self {
        Self {
            transaction: Some(model::Transaction {
                transaction_hash: Some(value.receipt.transaction_hash().into()),
                txn: Some(value.transaction.into()),
            }),
            receipt: Some(value.receipt.into()),
        }
    }
}

impl TryFrom<model::TransactionWithReceipt> for TransactionWithReceipt {
    type Error = FromModelError;
    fn try_from(value: model::TransactionWithReceipt) -> Result<Self, Self::Error> {
        let tx = TransactionWithHash::try_from(value.transaction.unwrap_or_default())?;
        Ok(Self { transaction: tx.transaction, receipt: value.receipt.unwrap_or_default().parse_model(tx.hash)? })
    }
}

impl TryFrom<model::Transaction> for TransactionWithHash {
    type Error = FromModelError;
    fn try_from(value: model::Transaction) -> Result<Self, Self::Error> {
        Ok(Self {
            transaction: value
                .txn
                .ok_or_else(|| FromModelError::missing_field("Transaction::transaction"))?
                .try_into()?,
            hash: value
                .transaction_hash
                .ok_or_else(|| FromModelError::missing_field("Transaction::transaction_hash"))?
                .into(),
        })
    }
}

impl TryFrom<model::transaction::Txn> for Transaction {
    type Error = FromModelError;
    fn try_from(value: model::transaction::Txn) -> Result<Self, Self::Error> {
        use model::transaction::Txn;
        Ok(match value {
            Txn::DeclareV0(tx) => Self::Declare(DeclareTransaction::V0(tx.try_into()?)),
            Txn::DeclareV1(tx) => Self::Declare(DeclareTransaction::V1(tx.try_into()?)),
            Txn::DeclareV2(tx) => Self::Declare(DeclareTransaction::V2(tx.try_into()?)),
            Txn::DeclareV3(tx) => Self::Declare(DeclareTransaction::V3(tx.try_into()?)),
            Txn::Deploy(tx) => Self::Deploy(tx.try_into()?),
            Txn::DeployAccountV1(tx) => Self::DeployAccount(DeployAccountTransaction::V1(tx.try_into()?)),
            Txn::DeployAccountV3(tx) => Self::DeployAccount(DeployAccountTransaction::V3(tx.try_into()?)),
            Txn::InvokeV0(tx) => Self::Invoke(InvokeTransaction::V0(tx.try_into()?)),
            Txn::InvokeV1(tx) => Self::Invoke(InvokeTransaction::V1(tx.try_into()?)),
            Txn::InvokeV3(tx) => Self::Invoke(InvokeTransaction::V3(tx.try_into()?)),
            Txn::L1Handler(tx) => Self::L1Handler(tx.try_into()?),
        })
    }
}

impl TryFrom<model::transaction::DeclareV0> for DeclareTransactionV0 {
    type Error = FromModelError;
    fn try_from(value: model::transaction::DeclareV0) -> Result<Self, Self::Error> {
        Ok(Self {
            sender_address: value.sender.unwrap_or_default().into(),
            max_fee: value.max_fee.unwrap_or_default().into(),
            signature: value.signature.unwrap_or_default().parts.into_iter().map(Into::into).collect(),
            class_hash: value.class_hash.unwrap_or_default().into(),
        })
    }
}

impl TryFrom<model::transaction::DeclareV1> for DeclareTransactionV1 {
    type Error = FromModelError;
    fn try_from(value: model::transaction::DeclareV1) -> Result<Self, Self::Error> {
        Ok(Self {
            sender_address: value.sender.unwrap_or_default().into(),
            max_fee: value.max_fee.unwrap_or_default().into(),
            signature: value.signature.unwrap_or_default().parts.into_iter().map(Into::into).collect(),
            nonce: value.nonce.unwrap_or_default().into(),
            class_hash: value.class_hash.unwrap_or_default().into(),
        })
    }
}

impl TryFrom<model::transaction::DeclareV2> for DeclareTransactionV2 {
    type Error = FromModelError;
    fn try_from(value: model::transaction::DeclareV2) -> Result<Self, Self::Error> {
        Ok(Self {
            sender_address: value.sender.unwrap_or_default().into(),
            compiled_class_hash: value.compiled_class_hash.unwrap_or_default().into(),
            max_fee: value.max_fee.unwrap_or_default().into(),
            signature: value.signature.unwrap_or_default().parts.into_iter().map(Into::into).collect(),
            nonce: value.nonce.unwrap_or_default().into(),
            class_hash: value.class_hash.unwrap_or_default().into(),
        })
    }
}

impl TryFrom<model::transaction::DeclareV3> for DeclareTransactionV3 {
    type Error = FromModelError;
    fn try_from(value: model::transaction::DeclareV3) -> Result<Self, Self::Error> {
        Ok(Self {
            sender_address: value.sender.unwrap_or_default().into(),
            compiled_class_hash: value.compiled_class_hash.unwrap_or_default().into(),
            signature: value.signature.unwrap_or_default().parts.into_iter().map(Into::into).collect(),
            nonce: value.nonce.unwrap_or_default().into(),
            class_hash: value.class_hash.unwrap_or_default().into(),
            resource_bounds: value.resource_bounds.unwrap_or_default().try_into()?,
            tip: value.tip,
            paymaster_data: value.paymaster_data.into_iter().map(Into::into).collect(),
            account_deployment_data: value.account_deployment_data.into_iter().map(Into::into).collect(),
            nonce_data_availability_mode: model::VolitionDomain::try_from(value.nonce_data_availability_mode)
                .map_err(|_| {
                    FromModelError::invalid_enum_variant("VolitionDomain", value.nonce_data_availability_mode)
                })?
                .into(),
            fee_data_availability_mode: model::VolitionDomain::try_from(value.fee_data_availability_mode)
                .map_err(|_| FromModelError::invalid_enum_variant("VolitionDomain", value.fee_data_availability_mode))?
                .into(),
        })
    }
}

impl TryFrom<model::transaction::Deploy> for DeployTransaction {
    type Error = FromModelError;
    fn try_from(value: model::transaction::Deploy) -> Result<Self, Self::Error> {
        Ok(Self {
            version: value.version.into(),
            contract_address_salt: value.address_salt.unwrap_or_default().into(),
            constructor_calldata: value.calldata.into_iter().map(Into::into).collect(),
            class_hash: value.class_hash.unwrap_or_default().into(),
        })
    }
}

impl TryFrom<model::transaction::DeployAccountV1> for DeployAccountTransactionV1 {
    type Error = FromModelError;
    fn try_from(value: model::transaction::DeployAccountV1) -> Result<Self, Self::Error> {
        Ok(Self {
            max_fee: value.max_fee.unwrap_or_default().into(),
            signature: value.signature.unwrap_or_default().parts.into_iter().map(Into::into).collect(),
            nonce: value.nonce.unwrap_or_default().into(),
            contract_address_salt: value.address_salt.unwrap_or_default().into(),
            constructor_calldata: value.calldata.into_iter().map(Into::into).collect(),
            class_hash: value.class_hash.unwrap_or_default().into(),
        })
    }
}

impl TryFrom<model::transaction::DeployAccountV3> for DeployAccountTransactionV3 {
    type Error = FromModelError;
    fn try_from(value: model::transaction::DeployAccountV3) -> Result<Self, Self::Error> {
        Ok(Self {
            signature: value.signature.unwrap_or_default().parts.into_iter().map(Into::into).collect(),
            nonce: value.nonce.unwrap_or_default().into(),
            contract_address_salt: value.address_salt.unwrap_or_default().into(),
            constructor_calldata: value.calldata.into_iter().map(Into::into).collect(),
            class_hash: value.class_hash.unwrap_or_default().into(),
            resource_bounds: value.resource_bounds.unwrap_or_default().try_into()?,
            tip: value.tip,
            paymaster_data: value.paymaster_data.into_iter().map(Into::into).collect(),
            nonce_data_availability_mode: model::VolitionDomain::try_from(value.nonce_data_availability_mode)
                .map_err(|_| {
                    FromModelError::invalid_enum_variant("VolitionDomain", value.nonce_data_availability_mode)
                })?
                .into(),
            fee_data_availability_mode: model::VolitionDomain::try_from(value.fee_data_availability_mode)
                .map_err(|_| FromModelError::invalid_enum_variant("VolitionDomain", value.fee_data_availability_mode))?
                .into(),
        })
    }
}

impl TryFrom<model::transaction::InvokeV0> for InvokeTransactionV0 {
    type Error = FromModelError;
    fn try_from(value: model::transaction::InvokeV0) -> Result<Self, Self::Error> {
        Ok(Self {
            max_fee: value.max_fee.unwrap_or_default().into(),
            signature: value.signature.unwrap_or_default().parts.into_iter().map(Into::into).collect(),
            contract_address: value.address.unwrap_or_default().into(),
            entry_point_selector: value.entry_point_selector.unwrap_or_default().into(),
            calldata: value.calldata.into_iter().map(Into::into).collect(),
        })
    }
}

impl TryFrom<model::transaction::InvokeV1> for InvokeTransactionV1 {
    type Error = FromModelError;
    fn try_from(value: model::transaction::InvokeV1) -> Result<Self, Self::Error> {
        Ok(Self {
            sender_address: value.sender.unwrap_or_default().into(),
            calldata: value.calldata.into_iter().map(Into::into).collect(),
            max_fee: value.max_fee.unwrap_or_default().into(),
            signature: value.signature.unwrap_or_default().parts.into_iter().map(Into::into).collect(),
            nonce: value.nonce.unwrap_or_default().into(),
        })
    }
}

impl TryFrom<model::transaction::InvokeV3> for InvokeTransactionV3 {
    type Error = FromModelError;
    fn try_from(value: model::transaction::InvokeV3) -> Result<Self, Self::Error> {
        Ok(Self {
            sender_address: value.sender.unwrap_or_default().into(),
            calldata: value.calldata.into_iter().map(Into::into).collect(),
            signature: value.signature.unwrap_or_default().parts.into_iter().map(Into::into).collect(),
            nonce: value.nonce.unwrap_or_default().into(),
            resource_bounds: value.resource_bounds.unwrap_or_default().try_into()?,
            tip: value.tip,
            paymaster_data: value.paymaster_data.into_iter().map(Into::into).collect(),
            account_deployment_data: value.account_deployment_data.into_iter().map(Into::into).collect(),
            nonce_data_availability_mode: model::VolitionDomain::try_from(value.nonce_data_availability_mode)
                .map_err(|_| {
                    FromModelError::invalid_enum_variant("VolitionDomain", value.nonce_data_availability_mode)
                })?
                .into(),
            fee_data_availability_mode: model::VolitionDomain::try_from(value.fee_data_availability_mode)
                .map_err(|_| FromModelError::invalid_enum_variant("VolitionDomain", value.fee_data_availability_mode))?
                .into(),
        })
    }
}

impl TryFrom<model::transaction::L1HandlerV0> for L1HandlerTransaction {
    type Error = FromModelError;
    fn try_from(value: model::transaction::L1HandlerV0) -> Result<Self, Self::Error> {
        Ok(Self {
            version: Felt::ZERO,
            nonce: felt_to_u64(&value.nonce.unwrap_or_default())
                .map_err(|_| FromModelError::invalid_field("L1HandlerV0::nonce"))?,
            contract_address: value.address.unwrap_or_default().into(),
            entry_point_selector: value.entry_point_selector.unwrap_or_default().into(),
            calldata: value.calldata.into_iter().map(Into::into).collect(),
        })
    }
}

impl TryFrom<model::ResourceBounds> for ResourceBoundsMapping {
    type Error = FromModelError;
    fn try_from(value: model::ResourceBounds) -> Result<Self, Self::Error> {
        Ok(Self {
            l1_gas: value.l1_gas.unwrap_or_default().try_into()?,
            l2_gas: value.l2_gas.unwrap_or_default().try_into()?,
        })
    }
}

impl TryFrom<model::ResourceLimits> for ResourceBounds {
    type Error = FromModelError;
    fn try_from(value: model::ResourceLimits) -> Result<Self, Self::Error> {
        Ok(Self {
            max_amount: felt_to_u64(&value.max_amount.unwrap_or_default())
                .map_err(|_| FromModelError::invalid_field("ResourceLimits::max_amount"))?,
            max_price_per_unit: felt_to_u128(&value.max_price_per_unit.unwrap_or_default())
                .map_err(|_| FromModelError::invalid_field("ResourceLimits::max_price_per_unit"))?,
        })
    }
}

impl From<model::VolitionDomain> for DataAvailabilityMode {
    fn from(value: model::VolitionDomain) -> Self {
        use model::VolitionDomain;
        match value {
            VolitionDomain::L1 => DataAvailabilityMode::L1,
            VolitionDomain::L2 => DataAvailabilityMode::L2,
        }
    }
}

fn execution_result(revert_reason: Option<String>) -> ExecutionResult {
    match revert_reason {
        Some(reason) => ExecutionResult::Reverted { reason },
        None => ExecutionResult::Succeeded,
    }
}

impl model::Receipt {
    pub fn parse_model(self, transaction_hash: Felt) -> Result<TransactionReceipt, FromModelError> {
        use model::receipt::Type;

        Ok(match self.r#type.ok_or(FromModelError::missing_field("Receipt::type"))? {
            Type::Invoke(tx) => TransactionReceipt::Invoke(tx.parse_model(transaction_hash)?),
            Type::L1Handler(tx) => TransactionReceipt::L1Handler(tx.parse_model(transaction_hash)?),
            Type::Declare(tx) => TransactionReceipt::Declare(tx.parse_model(transaction_hash)?),
            Type::DeprecatedDeploy(tx) => TransactionReceipt::Deploy(tx.parse_model(transaction_hash)?),
            Type::DeployAccount(tx) => TransactionReceipt::DeployAccount(tx.parse_model(transaction_hash)?),
        })
    }
}

impl model::receipt::Invoke {
    pub fn parse_model(self, transaction_hash: Felt) -> Result<InvokeTransactionReceipt, FromModelError> {
        let common = self.common.unwrap_or_default();
        Ok(InvokeTransactionReceipt {
            transaction_hash,
            actual_fee: FeePayment {
                unit: common.price_unit().into(),
                amount: common.actual_fee.unwrap_or_default().into(),
            },
            messages_sent: common.messages_sent.into_iter().map(TryInto::try_into).collect::<Result<_, _>>()?,
            events: vec![],
            execution_resources: common.execution_resources.unwrap_or_default().try_into()?,
            execution_result: execution_result(common.revert_reason),
        })
    }
}

impl model::receipt::L1Handler {
    pub fn parse_model(self, transaction_hash: Felt) -> Result<L1HandlerTransactionReceipt, FromModelError> {
        let common = self.common.unwrap_or_default();
        Ok(L1HandlerTransactionReceipt {
            transaction_hash,
            actual_fee: FeePayment {
                unit: common.price_unit().into(),
                amount: common.actual_fee.unwrap_or_default().into(),
            },
            messages_sent: common.messages_sent.into_iter().map(TryInto::try_into).collect::<Result<_, _>>()?,
            events: vec![],
            execution_resources: common.execution_resources.unwrap_or_default().try_into()?,
            execution_result: execution_result(common.revert_reason),
            message_hash: self.msg_hash.unwrap_or_default().into(),
        })
    }
}

impl model::receipt::Declare {
    pub fn parse_model(self, transaction_hash: Felt) -> Result<DeclareTransactionReceipt, FromModelError> {
        let common = self.common.unwrap_or_default();
        Ok(DeclareTransactionReceipt {
            transaction_hash,
            actual_fee: FeePayment {
                unit: common.price_unit().into(),
                amount: common.actual_fee.unwrap_or_default().into(),
            },
            messages_sent: common.messages_sent.into_iter().map(TryInto::try_into).collect::<Result<_, _>>()?,
            events: vec![],
            execution_resources: common.execution_resources.unwrap_or_default().try_into()?,
            execution_result: execution_result(common.revert_reason),
        })
    }
}

impl model::receipt::Deploy {
    pub fn parse_model(self, transaction_hash: Felt) -> Result<DeployTransactionReceipt, FromModelError> {
        let common = self.common.unwrap_or_default();
        Ok(DeployTransactionReceipt {
            transaction_hash,
            actual_fee: FeePayment {
                unit: common.price_unit().into(),
                amount: common.actual_fee.unwrap_or_default().into(),
            },
            messages_sent: common.messages_sent.into_iter().map(TryInto::try_into).collect::<Result<_, _>>()?,
            events: vec![],
            execution_resources: common.execution_resources.unwrap_or_default().try_into()?,
            execution_result: execution_result(common.revert_reason),
            contract_address: self.contract_address.unwrap_or_default().into(),
        })
    }
}

impl model::receipt::DeployAccount {
    pub fn parse_model(self, transaction_hash: Felt) -> Result<DeployAccountTransactionReceipt, FromModelError> {
        let common = self.common.unwrap_or_default();
        Ok(DeployAccountTransactionReceipt {
            transaction_hash,
            actual_fee: FeePayment {
                unit: common.price_unit().into(),
                amount: common.actual_fee.unwrap_or_default().into(),
            },
            messages_sent: common.messages_sent.into_iter().map(TryInto::try_into).collect::<Result<_, _>>()?,
            events: vec![],
            execution_resources: common.execution_resources.unwrap_or_default().try_into()?,
            execution_result: execution_result(common.revert_reason),
            contract_address: self.contract_address.unwrap_or_default().into(),
        })
    }
}

impl TryFrom<model::MessageToL1> for MsgToL1 {
    type Error = FromModelError;
    fn try_from(value: model::MessageToL1) -> Result<Self, Self::Error> {
        Ok(Self {
            from_address: value.from_address.unwrap_or_default().into(),
            to_address: value.to_address.unwrap_or_default().into(),
            payload: value.payload.into_iter().map(Into::into).collect(),
        })
    }
}

impl TryFrom<model::receipt::ExecutionResources> for ExecutionResources {
    type Error = FromModelError;
    fn try_from(value: model::receipt::ExecutionResources) -> Result<Self, Self::Error> {
        let opt = |builtin: u32| -> Option<u64> {
            // TODO: we should fix that in mp_receipt
            if builtin == 0 {
                None
            } else {
                Some(builtin.into())
            }
        };
        let builtins = value.builtins.unwrap_or_default();
        Ok(Self {
            steps: value.steps.into(),
            memory_holes: opt(value.memory_holes),
            range_check_builtin_applications: opt(builtins.range_check),
            pedersen_builtin_applications: opt(builtins.pedersen),
            poseidon_builtin_applications: opt(builtins.poseidon),
            ec_op_builtin_applications: opt(builtins.ec_op),
            ecdsa_builtin_applications: opt(builtins.ecdsa),
            bitwise_builtin_applications: opt(builtins.bitwise),
            keccak_builtin_applications: opt(builtins.keccak),
            // TODO: missing builtins (blockifier update needed)
            // TODO: what's that again? why is the naming convention different and why don't we have the field for it
            // segment_arena_builtin: builtins.,
            segment_arena_builtin: None,
            data_availability: L1Gas {
                l1_gas: felt_to_u128(&value.l1_gas.unwrap_or_default())
                    .map_err(|_| FromModelError::invalid_field("ExecutionResources::l1_gas"))?,
                l1_data_gas: felt_to_u128(&value.l1_data_gas.unwrap_or_default())
                    .map_err(|_| FromModelError::invalid_field("ExecutionResources::l1_data_gas"))?,
            },
            // TODO: wrong, update blockifier
            total_gas_consumed: L1Gas::default(),
            // l1_gas: ..
            // l1_data_gas: ..
            // total_l1_gas: ..
        })
    }
}

impl From<TransactionWithHash> for model::Transaction {
    fn from(value: TransactionWithHash) -> Self {
        Self { transaction_hash: Some(value.hash.into()), txn: Some(value.transaction.into()) }
    }
}

impl From<Transaction> for model::transaction::Txn {
    fn from(value: Transaction) -> Self {
        match value {
            Transaction::Invoke(tx) => match tx {
                InvokeTransaction::V0(tx) => Self::InvokeV0(tx.into()),
                InvokeTransaction::V1(tx) => Self::InvokeV1(tx.into()),
                InvokeTransaction::V3(tx) => Self::InvokeV3(tx.into()),
            },
            Transaction::L1Handler(tx) => Self::L1Handler(tx.into()),
            Transaction::Declare(tx) => match tx {
                DeclareTransaction::V0(tx) => Self::DeclareV0(tx.into()),
                DeclareTransaction::V1(tx) => Self::DeclareV1(tx.into()),
                DeclareTransaction::V2(tx) => Self::DeclareV2(tx.into()),
                DeclareTransaction::V3(tx) => Self::DeclareV3(tx.into()),
            },
            Transaction::Deploy(tx) => Self::Deploy(tx.into()),
            Transaction::DeployAccount(tx) => match tx {
                DeployAccountTransaction::V1(tx) => Self::DeployAccountV1(tx.into()),
                DeployAccountTransaction::V3(tx) => Self::DeployAccountV3(tx.into()),
            },
        }
    }
}

impl From<InvokeTransactionV0> for model::transaction::InvokeV0 {
    fn from(value: InvokeTransactionV0) -> Self {
        Self {
            max_fee: Some(value.max_fee.into()),
            signature: Some(model::AccountSignature { parts: value.signature.into_iter().map(Into::into).collect() }),
            address: Some(value.contract_address.into()),
            entry_point_selector: Some(value.entry_point_selector.into()),
            calldata: value.calldata.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<InvokeTransactionV1> for model::transaction::InvokeV1 {
    fn from(value: InvokeTransactionV1) -> Self {
        Self {
            sender: Some(value.sender_address.into()),
            max_fee: Some(value.max_fee.into()),
            signature: Some(model::AccountSignature { parts: value.signature.into_iter().map(Into::into).collect() }),
            calldata: value.calldata.into_iter().map(Into::into).collect(),
            nonce: Some(value.nonce.into()),
        }
    }
}

impl From<InvokeTransactionV3> for model::transaction::InvokeV3 {
    fn from(value: InvokeTransactionV3) -> Self {
        Self {
            sender: Some(value.sender_address.into()),
            signature: Some(model::AccountSignature { parts: value.signature.into_iter().map(Into::into).collect() }),
            calldata: value.calldata.into_iter().map(Into::into).collect(),
            resource_bounds: Some(value.resource_bounds.into()),
            tip: value.tip,
            paymaster_data: value.paymaster_data.into_iter().map(Into::into).collect(),
            account_deployment_data: value.account_deployment_data.into_iter().map(Into::into).collect(),
            nonce_data_availability_mode: model::VolitionDomain::from(value.nonce_data_availability_mode).into(),
            fee_data_availability_mode: model::VolitionDomain::from(value.fee_data_availability_mode).into(),
            nonce: Some(value.nonce.into()),
        }
    }
}

impl From<L1HandlerTransaction> for model::transaction::L1HandlerV0 {
    fn from(value: L1HandlerTransaction) -> Self {
        Self {
            nonce: Some(Felt::from(value.nonce).into()),
            address: Some(value.contract_address.into()),
            entry_point_selector: Some(value.entry_point_selector.into()),
            calldata: value.calldata.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<DeclareTransactionV0> for model::transaction::DeclareV0 {
    fn from(value: DeclareTransactionV0) -> Self {
        Self {
            sender: Some(value.sender_address.into()),
            max_fee: Some(value.max_fee.into()),
            signature: Some(model::AccountSignature { parts: value.signature.into_iter().map(Into::into).collect() }),
            class_hash: Some(value.class_hash.into()),
        }
    }
}

impl From<DeclareTransactionV1> for model::transaction::DeclareV1 {
    fn from(value: DeclareTransactionV1) -> Self {
        Self {
            sender: Some(value.sender_address.into()),
            max_fee: Some(value.max_fee.into()),
            signature: Some(model::AccountSignature { parts: value.signature.into_iter().map(Into::into).collect() }),
            class_hash: Some(value.class_hash.into()),
            nonce: Some(value.nonce.into()),
        }
    }
}

impl From<DeclareTransactionV2> for model::transaction::DeclareV2 {
    fn from(value: DeclareTransactionV2) -> Self {
        Self {
            sender: Some(value.sender_address.into()),
            max_fee: Some(value.max_fee.into()),
            signature: Some(model::AccountSignature { parts: value.signature.into_iter().map(Into::into).collect() }),
            class_hash: Some(value.class_hash.into()),
            nonce: Some(value.nonce.into()),
            compiled_class_hash: Some(value.compiled_class_hash.into()),
        }
    }
}

impl From<DeclareTransactionV3> for model::transaction::DeclareV3 {
    fn from(value: DeclareTransactionV3) -> Self {
        Self {
            sender: Some(value.sender_address.into()),
            signature: Some(model::AccountSignature { parts: value.signature.into_iter().map(Into::into).collect() }),
            class_hash: Some(value.class_hash.into()),
            nonce: Some(value.nonce.into()),
            compiled_class_hash: Some(value.compiled_class_hash.into()),
            resource_bounds: Some(value.resource_bounds.into()),
            tip: value.tip,
            paymaster_data: value.paymaster_data.into_iter().map(Into::into).collect(),
            account_deployment_data: value.account_deployment_data.into_iter().map(Into::into).collect(),
            nonce_data_availability_mode: model::VolitionDomain::from(value.nonce_data_availability_mode).into(),
            fee_data_availability_mode: model::VolitionDomain::from(value.fee_data_availability_mode).into(),
        }
    }
}

impl From<DeployTransaction> for model::transaction::Deploy {
    fn from(value: DeployTransaction) -> Self {
        Self {
            class_hash: Some(value.class_hash.into()),
            address_salt: Some(value.contract_address_salt.into()),
            calldata: value.constructor_calldata.into_iter().map(Into::into).collect(),
            // TODO(dto-faillible-conversion)
            version: felt_to_u32(&value.version).expect("DeployTransaction version is not an u32"),
        }
    }
}

impl From<DeployAccountTransactionV1> for model::transaction::DeployAccountV1 {
    fn from(value: DeployAccountTransactionV1) -> Self {
        Self {
            max_fee: Some(value.max_fee.into()),
            signature: Some(model::AccountSignature { parts: value.signature.into_iter().map(Into::into).collect() }),
            class_hash: Some(value.class_hash.into()),
            nonce: Some(value.nonce.into()),
            address_salt: Some(value.contract_address_salt.into()),
            calldata: value.constructor_calldata.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<DeployAccountTransactionV3> for model::transaction::DeployAccountV3 {
    fn from(value: DeployAccountTransactionV3) -> Self {
        Self {
            signature: Some(model::AccountSignature { parts: value.signature.into_iter().map(Into::into).collect() }),
            class_hash: Some(value.class_hash.into()),
            nonce: Some(value.nonce.into()),
            address_salt: Some(value.contract_address_salt.into()),
            calldata: value.constructor_calldata.into_iter().map(Into::into).collect(),
            resource_bounds: Some(value.resource_bounds.into()),
            tip: value.tip,
            paymaster_data: value.paymaster_data.into_iter().map(Into::into).collect(),
            nonce_data_availability_mode: model::VolitionDomain::from(value.nonce_data_availability_mode).into(),
            fee_data_availability_mode: model::VolitionDomain::from(value.fee_data_availability_mode).into(),
        }
    }
}

impl From<ResourceBoundsMapping> for model::ResourceBounds {
    fn from(value: ResourceBoundsMapping) -> Self {
        Self { l1_gas: Some(value.l1_gas.into()), l2_gas: Some(value.l2_gas.into()) }
    }
}

impl From<ResourceBounds> for model::ResourceLimits {
    fn from(value: ResourceBounds) -> Self {
        Self {
            max_amount: Some(Felt::from(value.max_amount).into()),
            max_price_per_unit: Some(Felt::from(value.max_price_per_unit).into()),
        }
    }
}

impl From<DataAvailabilityMode> for model::VolitionDomain {
    fn from(value: DataAvailabilityMode) -> Self {
        match value {
            DataAvailabilityMode::L1 => model::VolitionDomain::L1,
            DataAvailabilityMode::L2 => model::VolitionDomain::L2,
        }
    }
}

impl From<TransactionReceipt> for model::Receipt {
    fn from(value: TransactionReceipt) -> Self {
        use model::receipt::Type;
        Self {
            r#type: Some(match value {
                TransactionReceipt::Invoke(receipt) => Type::Invoke(receipt.into()),
                TransactionReceipt::L1Handler(receipt) => Type::L1Handler(receipt.into()),
                TransactionReceipt::Declare(receipt) => Type::Declare(receipt.into()),
                TransactionReceipt::Deploy(receipt) => Type::DeprecatedDeploy(receipt.into()),
                TransactionReceipt::DeployAccount(receipt) => Type::DeployAccount(receipt.into()),
            }),
        }
    }
}

impl From<InvokeTransactionReceipt> for model::receipt::Invoke {
    fn from(value: InvokeTransactionReceipt) -> Self {
        Self {
            common: Some(model::receipt::Common {
                actual_fee: Some(value.actual_fee.amount.into()),
                price_unit: model::PriceUnit::from(value.actual_fee.unit).into(),
                messages_sent: value.messages_sent.into_iter().map(Into::into).collect(),
                execution_resources: Some(value.execution_resources.into()),
                revert_reason: value.execution_result.revert_reason().map(String::from),
            }),
        }
    }
}

impl From<L1HandlerTransactionReceipt> for model::receipt::L1Handler {
    fn from(value: L1HandlerTransactionReceipt) -> Self {
        Self {
            common: Some(model::receipt::Common {
                actual_fee: Some(value.actual_fee.amount.into()),
                price_unit: model::PriceUnit::from(value.actual_fee.unit).into(),
                messages_sent: value.messages_sent.into_iter().map(Into::into).collect(),
                execution_resources: Some(value.execution_resources.into()),
                revert_reason: value.execution_result.revert_reason().map(String::from),
            }),
            msg_hash: Some(value.message_hash.into()),
        }
    }
}

impl From<DeclareTransactionReceipt> for model::receipt::Declare {
    fn from(value: DeclareTransactionReceipt) -> Self {
        Self {
            common: Some(model::receipt::Common {
                actual_fee: Some(value.actual_fee.amount.into()),
                price_unit: model::PriceUnit::from(value.actual_fee.unit).into(),
                messages_sent: value.messages_sent.into_iter().map(Into::into).collect(),
                execution_resources: Some(value.execution_resources.into()),
                revert_reason: value.execution_result.revert_reason().map(String::from),
            }),
        }
    }
}

impl From<DeployTransactionReceipt> for model::receipt::Deploy {
    fn from(value: DeployTransactionReceipt) -> Self {
        Self {
            common: Some(model::receipt::Common {
                actual_fee: Some(value.actual_fee.amount.into()),
                price_unit: model::PriceUnit::from(value.actual_fee.unit).into(),
                messages_sent: value.messages_sent.into_iter().map(Into::into).collect(),
                execution_resources: Some(value.execution_resources.into()),
                revert_reason: value.execution_result.revert_reason().map(String::from),
            }),
            contract_address: Some(value.contract_address.into()),
        }
    }
}

impl From<DeployAccountTransactionReceipt> for model::receipt::DeployAccount {
    fn from(value: DeployAccountTransactionReceipt) -> Self {
        Self {
            common: Some(model::receipt::Common {
                actual_fee: Some(value.actual_fee.amount.into()),
                price_unit: model::PriceUnit::from(value.actual_fee.unit).into(),
                messages_sent: value.messages_sent.into_iter().map(Into::into).collect(),
                execution_resources: Some(value.execution_resources.into()),
                revert_reason: value.execution_result.revert_reason().map(String::from),
            }),
            contract_address: Some(value.contract_address.into()),
        }
    }
}

impl From<MsgToL1> for model::MessageToL1 {
    fn from(value: MsgToL1) -> Self {
        Self {
            from_address: Some(value.from_address.into()),
            payload: value.payload.into_iter().map(Into::into).collect(),
            to_address: Some(value.to_address.into()),
        }
    }
}

impl From<ExecutionResources> for model::receipt::ExecutionResources {
    fn from(value: ExecutionResources) -> Self {
        Self {
            // TODO(dto-faillible-conversion)
            builtins: Some(BuiltinCounter {
                bitwise: value
                    .bitwise_builtin_applications
                    .unwrap_or_default()
                    .try_into()
                    .expect("bitwise_builtin > u32::MAX"),
                ecdsa: value
                    .ecdsa_builtin_applications
                    .unwrap_or_default()
                    .try_into()
                    .expect("ecdsa_builtin > u32::MAX"),
                ec_op: value
                    .ec_op_builtin_applications
                    .unwrap_or_default()
                    .try_into()
                    .expect("ec_op_builtin > u32::MAX"),
                pedersen: value
                    .pedersen_builtin_applications
                    .unwrap_or_default()
                    .try_into()
                    .expect("pedersen_builtin > u32::MAX"),
                range_check: value
                    .range_check_builtin_applications
                    .unwrap_or_default()
                    .try_into()
                    .expect("range_check_builtin > u32::MAX"),
                poseidon: value
                    .poseidon_builtin_applications
                    .unwrap_or_default()
                    .try_into()
                    .expect("poseidon_builtin > u32::MAX"),
                keccak: value
                    .keccak_builtin_applications
                    .unwrap_or_default()
                    .try_into()
                    .expect("keccak_builtin > u32::MAX"),
                // TODO: missing builtins
                // output: value.output_builtin_applications.unwrap_or_default().try_into().expect("output_builtin > u32::MAX"),
                // add_mod: value.add_mod_builtin_applications.unwrap_or_default().try_into().expect("add_mod_builtin > u32::MAX"),
                // mul_mod: value.mul_mod_builtin_applications.unwrap_or_default().try_into().expect("mul_mod_builtin > u32::MAX"),
                // range_check96: value
                //     .range_check96_builtin_applications
                //     .unwrap_or_default().try_into().expect("range_check96_builtin > u32::MAX"),
                ..Default::default()
            }),
            // TODO(dto-faillible-conversion)
            steps: value.steps.try_into().expect("steps > u32::MAX"),
            // TODO(dto-faillible-conversion)
            memory_holes: value.memory_holes.unwrap_or(0).try_into().expect("memory_holes > u32::MAX"),
            l1_gas: Some(Felt::from(value.total_gas_consumed.l1_gas).into()),
            l1_data_gas: Some(Felt::from(value.total_gas_consumed.l1_data_gas).into()),
            total_l1_gas: Some(Felt::from(value.total_gas_consumed.l1_gas).into()),
        }
    }
}

impl From<PriceUnit> for model::PriceUnit {
    fn from(value: PriceUnit) -> Self {
        match value {
            PriceUnit::Wei => Self::Wei,
            PriceUnit::Fri => Self::Fri,
        }
    }
}
impl From<model::PriceUnit> for PriceUnit {
    fn from(value: model::PriceUnit) -> Self {
        match value {
            model::PriceUnit::Wei => Self::Wei,
            model::PriceUnit::Fri => Self::Fri,
        }
    }
}

/// Reply to a transactions sync request.
pub async fn transactions_sync(
    ctx: ReqContext<MadaraP2pContext>,
    req: model::TransactionsRequest,
    mut out: Sender<model::TransactionsResponse>,
) -> Result<(), sync_handlers::Error> {
    let stream = ctx
        .app_ctx
        .backend
        .block_info_stream(block_stream_config(&ctx.app_ctx.backend, req.iteration.unwrap_or_default())?);
    pin!(stream);

    tracing::debug!("transactions sync!");

    while let Some(res) = stream.next().await {
        let header = res.or_internal_server_error("Error while reading from block stream")?;

        let block_inner = ctx
            .app_ctx
            .backend
            .get_block_inner(&DbBlockId::Number(header.header.block_number))
            .or_internal_server_error("Getting block state diff")?
            .ok_or_internal_server_error("No body for block")?;

        for (transaction, receipt) in block_inner.transactions.into_iter().zip(block_inner.receipts) {
            let el = TransactionWithReceipt { transaction, receipt };

            out.send(model::TransactionsResponse {
                transaction_message: Some(model::transactions_response::TransactionMessage::TransactionWithReceipt(
                    el.into(),
                )),
            })
            .await?
        }
    }

    // Add the Fin message
    out.send(model::TransactionsResponse {
        transaction_message: Some(model::transactions_response::TransactionMessage::Fin(model::Fin {})),
    })
    .await?;

    Ok(())
}

/// Used by [`crate::commands::P2pCommands::make_transactions_stream`] to send a transactions stream request.
/// Note that the events in the transaction receipt will not be filled in, as it needs to be fetched using the events stream request.
pub async fn read_transactions_stream(
    res: impl Stream<Item = model::TransactionsResponse>,
    transactions_count: usize,
) -> Result<Vec<TransactionWithReceipt>, sync_handlers::Error> {
    pin!(res);

    let mut vec = Vec::with_capacity(transactions_count);
    for i in 0..transactions_count {
        let handle_fin = || {
            if i == 0 {
                sync_handlers::Error::EndOfStream
            } else {
                sync_handlers::Error::bad_request(format!(
                    "Expected {} messages in stream, got {}",
                    transactions_count, i
                ))
            }
        };

        let Some(res) = res.next().await else { return Err(handle_fin()) };
        let val = match res.transaction_message.ok_or_bad_request("No message")? {
            model::transactions_response::TransactionMessage::TransactionWithReceipt(message) => message,
            model::transactions_response::TransactionMessage::Fin(_) => return Err(handle_fin()),
        };
        let res = TransactionWithReceipt::try_from(val).or_bad_request("Converting transaction with receipt")?;
        vec.push(res);
    }

    Ok(vec)
}
