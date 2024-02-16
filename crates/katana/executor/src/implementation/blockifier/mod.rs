mod output;
mod state;
mod utils;

use blockifier::block_context::BlockContext;
use katana_primitives::block::ExecutableBlock;
use katana_primitives::env::{BlockEnv, CfgEnv};
use katana_primitives::receipt::Receipt;
use katana_primitives::transaction::{ExecutableTxWithHash, TxWithHash};
use katana_primitives::FieldElement;
use katana_provider::traits::state::StateProvider;

use self::state::CachedState;
pub use self::utils::Error;
use crate::{
    abstraction, BlockExecutor, EntryPointCall, ExecutionSimulationFlag, ExecutorResult,
    StateProviderDb, TransactionExecutionOutput,
};

pub struct ExecutorFactory {
    cfg: CfgEnv,
    flags: ExecutionSimulationFlag,
}

impl abstraction::ExecutorFactory for ExecutorFactory {
    fn with_state<'a, P>(&self, state: P) -> Box<dyn BlockExecutor<'a> + 'a>
    where
        P: StateProvider + 'a,
    {
        todo!()
    }

    fn with_state_and_block_env<'a, P>(
        &self,
        state: P,
        block_env: BlockEnv,
    ) -> Box<dyn BlockExecutor<'a> + 'a>
    where
        P: StateProvider + 'a,
    {
        todo!()
    }

    fn cfg(&self) -> &CfgEnv {
        &self.cfg
    }
}

pub struct StarknetVMProcessor<'a> {
    block_context: BlockContext,
    state: CachedState<StateProviderDb<'a>>,
    transactions: Vec<(TxWithHash, Option<Receipt>)>,
    simulation_flags: ExecutionSimulationFlag,
}

impl<'a> StarknetVMProcessor<'a> {
    pub fn new(
        state: Box<dyn StateProvider + 'a>,
        block_env: BlockEnv,
        cfg_env: CfgEnv,
        simulation_flags: ExecutionSimulationFlag,
    ) -> Self {
        let transactions = Vec::new();
        let block_context = utils::block_context_from_envs(&block_env, &cfg_env);
        let state = state::CachedState::new(StateProviderDb(state));
        Self { block_context, state, transactions, simulation_flags }
    }
}

impl<'a> abstraction::TransactionExecutor for StarknetVMProcessor<'a> {
    fn execute(
        &mut self,
        tx: ExecutableTxWithHash,
    ) -> ExecutorResult<Box<dyn TransactionExecutionOutput>> {
        todo!()
    }

    fn simulate(
        &self,
        tx: ExecutableTxWithHash,
        flags: ExecutionSimulationFlag,
    ) -> ExecutorResult<Box<dyn TransactionExecutionOutput>> {
        todo!()
    }

    fn call(&self, call: EntryPointCall, initial_gas: u128) -> ExecutorResult<Vec<FieldElement>> {
        todo!()
    }
}

impl<'a> abstraction::BlockExecutor<'a> for StarknetVMProcessor<'a> {
    fn execute_block(&mut self, block: ExecutableBlock) -> ExecutorResult<()> {
        todo!()
    }

    fn take_execution_output(&mut self) -> crate::ExecutionOutput {
        todo!()
    }

    fn state(&self) -> Box<dyn StateProvider + 'a> {
        Box::new(self.state.clone())
    }

    fn transactions(&self) -> &[(TxWithHash, Option<Receipt>)] {
        &self.transactions
    }

    fn block_env(&self) -> BlockEnv {
        todo!()
    }
}
