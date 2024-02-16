use std::sync::Arc;

use blockifier::block_context::{BlockContext, BlockInfo, ChainInfo, FeeTokenAddresses, GasPrices};
use blockifier::execution::common_hints::ExecutionMode;
use blockifier::execution::entry_point::{
    CallEntryPoint, EntryPointExecutionContext, ExecutionResources,
};
use blockifier::execution::errors::EntryPointExecutionError;
use blockifier::fee::fee_utils::calculate_tx_l1_gas_usages;
use blockifier::state::cached_state::{self, GlobalContractCache};
use blockifier::state::state_api::StateReader;
use blockifier::transaction::account_transaction::AccountTransaction;
use blockifier::transaction::errors::{TransactionExecutionError, TransactionFeeError};
use blockifier::transaction::objects::{
    AccountTransactionContext, DeprecatedAccountTransactionContext,
};
use blockifier::transaction::transaction_execution::Transaction;
use blockifier::transaction::transactions::{
    DeclareTransaction, DeployAccountTransaction, ExecutableTransaction, InvokeTransaction,
    L1HandlerTransaction,
};
use katana_primitives::conversion::blockifier::to_class;
use katana_primitives::env::{BlockEnv, CfgEnv};
use katana_primitives::transaction::{DeclareTx, ExecutableTx, ExecutableTxWithHash};
use katana_primitives::FieldElement;
use starknet_api::block::{BlockNumber, BlockTimestamp};
use starknet_api::core::{ClassHash, CompiledClassHash, EntryPointSelector, Nonce};
use starknet_api::transaction::{
    Calldata, ContractAddressSalt, DeclareTransaction as ApiDeclareTransaction,
    DeclareTransactionV0V1, DeclareTransactionV2,
    DeployAccountTransaction as ApiDeployAccountTransaction, DeployAccountTransactionV1, Fee,
    InvokeTransaction as ApiInvokeTransaction, TransactionHash, TransactionSignature,
    TransactionVersion,
};

use super::output::TransactionExecutionInfo;
use super::state::CachedState;
use crate::abstraction::{EntryPointCall, ExecutionSimulationFlag};
use crate::StateProviderDb;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to execute call: {0}")]
    CallError(#[from] EntryPointExecutionError),

    #[error("fee error: {0}")]
    TransactionFee(#[from] TransactionFeeError),

    #[error("failed to execute transaction: {0}")]
    TransactionExecution(#[from] TransactionExecutionError),
}

pub(super) fn transact<S: StateReader>(
    tx: ExecutableTxWithHash,
    state: &mut CachedState<S>,
    block_context: &BlockContext,
    simulation_flags: &ExecutionSimulationFlag,
) -> Result<TransactionExecutionInfo, Error> {
    let class_declaration_artifacts = if let ExecutableTx::Declare(tx) = tx.as_ref() {
        let class_hash = tx.class_hash();
        Some((class_hash, tx.compiled_class.clone(), tx.sierra_class.clone()))
    } else {
        None
    };

    let inner = &mut state.0.write().inner;
    let validate = !simulation_flags.skip_validate;
    let charge_fee = !simulation_flags.skip_fee_transfer;

    let res = match to_executor_tx(tx) {
        Transaction::AccountTransaction(tx) => {
            tx.execute(inner, block_context, charge_fee, validate)
        }

        Transaction::L1HandlerTransaction(tx) => {
            tx.execute(inner, block_context, charge_fee, validate)
        }
    };

    match res {
        Ok(inner) => {
            if let Some((class_hash, compiled_class, sierra_class)) = class_declaration_artifacts {
                state.0.write().declared_classes.insert(class_hash, (compiled_class, sierra_class));
            }

            let gas_used =
                calculate_tx_l1_gas_usages(&inner.actual_resources, &block_context)?.gas_usage;
            Ok(TransactionExecutionInfo { gas_used, inner })
        }

        Err(err) => Err(Error::TransactionExecution(err)),
    }
}

/// Perform a function call on a contract and retrieve the return values.
pub(super) fn call<'a, S: StateReader>(
    request: EntryPointCall,
    state: StateProviderDb<'a>,
    block_context: BlockContext,
    initial_gas: u128,
) -> Result<Vec<FieldElement>, Error> {
    let mut state = cached_state::CachedState::new(state, GlobalContractCache::default());

    let call = CallEntryPoint {
        initial_gas: initial_gas as u64,
        storage_address: request.contract_address.into(),
        entry_point_selector: EntryPointSelector(request.entry_point_selector.into()),
        calldata: Calldata(Arc::new(request.calldata.into_iter().map(|f| f.into()).collect())),
        ..Default::default()
    };

    // TODO: this must be false if fees are disabled I assume.
    let limit_steps_by_resources = true;

    // Now, the max step is not given directly to this function.
    // It's computed by a new function max_steps, and it tooks the values
    // from teh block context itself instead of the input give.
    // https://github.com/starkware-libs/blockifier/blob/51b343fe38139a309a69b2482f4b484e8caa5edf/crates/blockifier/src/execution/entry_point.rs#L165
    // The blockifier patch must be adjusted to modify this function to return
    // the limit we have into the block context without min applied:
    // https://github.com/starkware-libs/blockifier/blob/51b343fe38139a309a69b2482f4b484e8caa5edf/crates/blockifier/src/execution/entry_point.rs#L215
    let res = call.execute(
        &mut state,
        &mut ExecutionResources::default(),
        &mut EntryPointExecutionContext::new(
            &block_context,
            // TODO: the current does not have Default, let's use the old one for now.
            &AccountTransactionContext::Deprecated(DeprecatedAccountTransactionContext::default()),
            ExecutionMode::Execute,
            limit_steps_by_resources,
        )?,
    )?;

    let retdata = res.execution.retdata.0;
    let retdata = retdata.into_iter().map(|f| f.into()).collect::<Vec<FieldElement>>();
    Ok(retdata)
}

fn to_executor_tx(tx: ExecutableTxWithHash) -> Transaction {
    let hash = tx.hash;

    match tx.transaction {
        ExecutableTx::Invoke(tx) => {
            let calldata = tx.calldata.into_iter().map(|f| f.into()).collect();
            let signature = tx.signature.into_iter().map(|f| f.into()).collect();
            Transaction::AccountTransaction(AccountTransaction::Invoke(InvokeTransaction {
                tx: ApiInvokeTransaction::V1(starknet_api::transaction::InvokeTransactionV1 {
                    max_fee: Fee(tx.max_fee),
                    nonce: Nonce(tx.nonce.into()),
                    sender_address: tx.sender_address.into(),
                    signature: TransactionSignature(signature),
                    calldata: Calldata(Arc::new(calldata)),
                }),
                tx_hash: TransactionHash(hash.into()),
                only_query: false,
            }))
        }

        ExecutableTx::DeployAccount(tx) => {
            let calldata = tx.constructor_calldata.into_iter().map(|f| f.into()).collect();
            let signature = tx.signature.into_iter().map(|f| f.into()).collect();
            Transaction::AccountTransaction(AccountTransaction::DeployAccount(
                DeployAccountTransaction {
                    contract_address: tx.contract_address.into(),
                    tx: ApiDeployAccountTransaction::V1(DeployAccountTransactionV1 {
                        max_fee: Fee(tx.max_fee),
                        nonce: Nonce(tx.nonce.into()),
                        signature: TransactionSignature(signature),
                        class_hash: ClassHash(tx.class_hash.into()),
                        constructor_calldata: Calldata(Arc::new(calldata)),
                        contract_address_salt: ContractAddressSalt(tx.contract_address_salt.into()),
                    }),
                    tx_hash: TransactionHash(hash.into()),
                    only_query: false,
                },
            ))
        }

        ExecutableTx::Declare(tx) => {
            let contract_class = tx.compiled_class;

            let tx = match tx.transaction {
                DeclareTx::V1(tx) => {
                    let signature = tx.signature.into_iter().map(|f| f.into()).collect();
                    ApiDeclareTransaction::V1(DeclareTransactionV0V1 {
                        max_fee: Fee(tx.max_fee),
                        nonce: Nonce(tx.nonce.into()),
                        sender_address: tx.sender_address.into(),
                        signature: TransactionSignature(signature),
                        class_hash: ClassHash(tx.class_hash.into()),
                    })
                }

                DeclareTx::V2(tx) => {
                    let signature = tx.signature.into_iter().map(|f| f.into()).collect();
                    ApiDeclareTransaction::V2(DeclareTransactionV2 {
                        max_fee: Fee(tx.max_fee),
                        nonce: Nonce(tx.nonce.into()),
                        sender_address: tx.sender_address.into(),
                        signature: TransactionSignature(signature),
                        class_hash: ClassHash(tx.class_hash.into()),
                        compiled_class_hash: CompiledClassHash(tx.compiled_class_hash.into()),
                    })
                }
            };

            let tx = DeclareTransaction::new(
                tx,
                TransactionHash(hash.into()),
                to_class(contract_class).unwrap(),
            )
            .expect("class mismatch");
            Transaction::AccountTransaction(AccountTransaction::Declare(tx))
        }

        ExecutableTx::L1Handler(tx) => {
            let calldata = tx.calldata.into_iter().map(|f| f.into()).collect();
            Transaction::L1HandlerTransaction(L1HandlerTransaction {
                paid_fee_on_l1: Fee(tx.paid_fee_on_l1),
                tx: starknet_api::transaction::L1HandlerTransaction {
                    nonce: Nonce(tx.nonce.into()),
                    calldata: Calldata(Arc::new(calldata)),
                    version: TransactionVersion(1u128.into()),
                    contract_address: tx.contract_address.into(),
                    entry_point_selector: EntryPointSelector(tx.entry_point_selector.into()),
                },
                tx_hash: TransactionHash(hash.into()),
            })
        }
    }
}

/// Create a block context from the chain environment values.
pub(super) fn block_context_from_envs(block_env: &BlockEnv, cfg_env: &CfgEnv) -> BlockContext {
    let fee_token_addresses = FeeTokenAddresses {
        eth_fee_token_address: cfg_env.fee_token_addresses.eth.into(),
        strk_fee_token_address: cfg_env.fee_token_addresses.strk.into(),
    };

    let gas_prices = GasPrices {
        eth_l1_gas_price: block_env.l1_gas_prices.eth.try_into().unwrap(),
        strk_l1_gas_price: block_env.l1_gas_prices.strk.try_into().unwrap(),
        eth_l1_data_gas_price: 0,
        strk_l1_data_gas_price: 0,
    };

    BlockContext {
        block_info: BlockInfo {
            gas_prices,
            use_kzg_da: false,
            block_number: BlockNumber(block_env.number),
            block_timestamp: BlockTimestamp(block_env.timestamp),
            sequencer_address: block_env.sequencer_address.into(),
            max_recursion_depth: cfg_env.max_recursion_depth,
            validate_max_n_steps: cfg_env.validate_max_n_steps,
            invoke_tx_max_n_steps: cfg_env.invoke_tx_max_n_steps,
            vm_resource_fee_cost: cfg_env.vm_resource_fee_cost.clone().into(),
        },
        chain_info: ChainInfo { fee_token_addresses, chain_id: cfg_env.chain_id.into() },
    }
}
