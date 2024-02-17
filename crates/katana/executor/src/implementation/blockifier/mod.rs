mod output;
mod state;
mod utils;

use blockifier::block_context::BlockContext;
use katana_primitives::block::{ExecutableBlock, GasPrices, PartialHeader};
use katana_primitives::env::{BlockEnv, CfgEnv};
use katana_primitives::receipt::Receipt;
use katana_primitives::transaction::{ExecutableTxWithHash, TxWithHash};
use katana_primitives::FieldElement;
use katana_provider::traits::state::StateProvider;
use starknet_api::block::{BlockNumber, BlockTimestamp};

use self::state::CachedState;
pub use self::utils::Error;
use crate::{
    abstraction, BlockExecutor, EntryPointCall, ExecutionOutput, ExecutionSimulationFlag,
    ExecutorResult, StateProviderDb, TransactionExecutionOutput, TransactionExecutor,
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
        self.with_state_and_block_env(state, BlockEnv::default())
    }

    fn with_state_and_block_env<'a, P>(
        &self,
        state: P,
        block_env: BlockEnv,
    ) -> Box<dyn BlockExecutor<'a> + 'a>
    where
        P: StateProvider + 'a,
    {
        let cfg_env = self.cfg.clone();
        let flags = self.flags.clone();
        Box::new(StarknetVMProcessor::new(Box::new(state), block_env, cfg_env, flags))
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

    fn fill_block_env_from_header(&mut self, header: &PartialHeader) {
        let number = BlockNumber(header.number);
        let timestamp = BlockTimestamp(header.timestamp);
        let eth_l1_gas_price = header.gas_prices.eth as u128;
        let strk_l1_gas_price = header.gas_prices.strk as u128;

        self.block_context.block_info.block_number = number;
        self.block_context.block_info.block_timestamp = timestamp;
        self.block_context.block_info.gas_prices.eth_l1_gas_price = eth_l1_gas_price;
        self.block_context.block_info.gas_prices.strk_l1_gas_price = strk_l1_gas_price;
        self.block_context.block_info.sequencer_address = header.sequencer_address.into();
    }
}

impl<'a> abstraction::TransactionExecutor for StarknetVMProcessor<'a> {
    fn execute(
        &mut self,
        tx: ExecutableTxWithHash,
    ) -> ExecutorResult<Box<dyn TransactionExecutionOutput>> {
        let state = &self.state;
        let block_context = &self.block_context;
        let flags = &self.simulation_flags;

        let res = utils::transact(tx, state, block_context, flags)?;

        Ok(Box::new(res))
    }

    fn simulate(
        &self,
        tx: ExecutableTxWithHash,
        flags: ExecutionSimulationFlag,
    ) -> ExecutorResult<Box<dyn TransactionExecutionOutput>> {
        todo!()
    }

    fn call(&self, call: EntryPointCall, initial_gas: u128) -> ExecutorResult<Vec<FieldElement>> {
        let block_context = &self.block_context;
        let res = utils::call(call, &self.state, block_context, initial_gas)?;

        let retdata = res.execution.retdata.0;
        let retdata = retdata.into_iter().map(|f| f.into()).collect::<Vec<FieldElement>>();

        Ok(retdata)
    }
}

impl<'a> abstraction::BlockExecutor<'a> for StarknetVMProcessor<'a> {
    fn execute_block(&mut self, block: ExecutableBlock) -> ExecutorResult<()> {
        self.fill_block_env_from_header(&block.header);

        for tx in block.body {
            let tx_ = TxWithHash::from(&tx);

            let res = self.execute(tx)?;
            let receipt = res.receipt(tx_.as_ref());

            self.transactions.push((tx_, Some(receipt)));
        }

        Ok(())
    }

    fn take_execution_output(&mut self) -> ExecutorResult<ExecutionOutput> {
        let states = utils::state_update_from_cached_state(&self.state);
        let transactions = std::mem::take(&mut self.transactions);
        Ok(ExecutionOutput { states, transactions })
    }

    fn state(&self) -> Box<dyn StateProvider + 'a> {
        Box::new(self.state.clone())
    }

    fn transactions(&self) -> &[(TxWithHash, Option<Receipt>)] {
        &self.transactions
    }

    fn block_env(&self) -> BlockEnv {
        BlockEnv {
            number: self.block_context.block_info.block_number.0,
            timestamp: self.block_context.block_info.block_timestamp.0,
            sequencer_address: self.block_context.block_info.sequencer_address.into(),
            l1_gas_prices: GasPrices {
                eth: self.block_context.block_info.gas_prices.eth_l1_gas_price as u64,
                strk: self.block_context.block_info.gas_prices.strk_l1_gas_price as u64,
            },
        }
    }
}
