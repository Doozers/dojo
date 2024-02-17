use katana_primitives::block::ExecutableBlock;
use katana_primitives::contract::ContractAddress;
use katana_primitives::env::{BlockEnv, CfgEnv};
use katana_primitives::receipt::Receipt;
use katana_primitives::state::StateUpdatesWithDeclaredClasses;
use katana_primitives::transaction::{ExecutableTxWithHash, Tx, TxWithHash};
use katana_primitives::FieldElement;
use katana_provider::traits::state::StateProvider;

#[derive(Debug, thiserror::Error)]
pub enum ExecutorError {
    #[cfg(feature = "sir")]
    #[error(transparent)]
    SirError(#[from] crate::implementation::sir::Error),

    #[cfg(feature = "blockifier")]
    #[error(transparent)]
    BlockifierError(#[from] crate::implementation::blockifier::Error),
}

pub type ExecutorResult<T> = Result<T, ExecutorError>;

/// A common flags for transaction execution.
#[derive(Debug, Clone, Default)]
pub struct ExecutionSimulationFlag {
    pub skip_validate: bool,
    pub skip_fee_transfer: bool,
}

/// A wrapper around a boxed [StateProvider] for implementing the executor's own state reader
/// traits.
pub struct StateProviderDb<'a>(pub(crate) Box<dyn StateProvider + 'a>);

/// The output of a executor after a series of executions.
#[derive(Debug)]
pub struct ExecutionOutput {
    /// The state updates produced by the executions.
    pub states: StateUpdatesWithDeclaredClasses,
    /// The transactions that have been executed.
    pub transactions: Vec<(TxWithHash, Option<Receipt>)>,
}

#[derive(Debug)]
pub struct EntryPointCall {
    /// The address of the contract whose function you're calling.
    pub contract_address: ContractAddress,
    /// The input to the function.
    pub calldata: Vec<FieldElement>,
    /// The function selector.
    pub entry_point_selector: FieldElement,
}

/// A factory for creating [BlockExecutor] instances.
pub trait ExecutorFactory: Send + Sync {
    /// Construct a new [BlockExecutor] with the given state.
    fn with_state<'a, P>(&self, state: P) -> Box<dyn BlockExecutor<'a> + 'a>
    where
        P: StateProvider + 'a;

    /// Construct a new [BlockExecutor] with the given state and block environment values.
    fn with_state_and_block_env<'a, P>(
        &self,
        state: P,
        block_env: BlockEnv,
    ) -> Box<dyn BlockExecutor<'a> + 'a>
    where
        P: StateProvider + 'a;

    /// Returns the configuration environment of the executor.
    fn cfg(&self) -> &CfgEnv;
}

/// An executor that can execute a block of transactions.
pub trait BlockExecutor<'a>: TransactionExecutor + Send + Sync {
    /// Executes the given block.
    fn execute_block(&mut self, block: ExecutableBlock) -> ExecutorResult<()>;

    /// Takes the output state of the executor.
    fn take_execution_output(&mut self) -> ExecutorResult<ExecutionOutput>;

    /// Returns the current state of the executor.
    fn state(&self) -> Box<dyn StateProvider + 'a>;

    /// Returns the transactions that have been executed.
    fn transactions(&self) -> &[(TxWithHash, Option<Receipt>)];

    /// Returns the current block environment of the executor.
    fn block_env(&self) -> BlockEnv;
}

/// Type that can execute transactions.
pub trait TransactionExecutor {
    /// Executes the given transaction and returns the output.
    fn execute(
        &mut self,
        tx: ExecutableTxWithHash,
    ) -> ExecutorResult<Box<dyn TransactionExecutionOutput>>;

    /// Executes the given transaction according to the simulation flags and returns the output,
    /// without committing to the state.
    fn simulate(
        &self,
        tx: ExecutableTxWithHash,
        flags: ExecutionSimulationFlag,
    ) -> ExecutorResult<Box<dyn TransactionExecutionOutput>>;

    /// TODO: make `initial_gas` as `ExecutorFactory` responsibility
    fn call(&self, call: EntryPointCall, initial_gas: u128) -> ExecutorResult<Vec<FieldElement>>;
}

/// The output of a transaction execution.
pub trait TransactionExecutionOutput {
    /// Retrieves the receipt from the transaction execution ouput.
    fn receipt(&self, tx: &Tx) -> Receipt;

    /// The transaction fee that was actually paid.
    fn actual_fee(&self) -> u128;

    /// The total gas used by the transaction.
    fn gas_used(&self) -> u128;

    /// The error message if the transaction execution reverted, otherwise the value is `None`.
    fn revert_error(&self) -> Option<&str>;
}
