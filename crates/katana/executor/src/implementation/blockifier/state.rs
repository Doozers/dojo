use std::collections::HashMap;
use std::sync::Arc;

use blockifier::state::cached_state::{self, GlobalContractCache};
use blockifier::state::state_api::StateReader;
use katana_primitives::class::{CompiledClass, FlattenedSierraClass};
use katana_provider::traits::contract::ContractClassProvider;
use katana_provider::traits::state::StateProvider;
use katana_provider::ProviderResult;
use parking_lot::RwLock;

use crate::StateProviderDb;

impl<'a> StateReader for StateProviderDb<'a> {
    fn get_class_hash_at(
        &mut self,
        contract_address: starknet_api::core::ContractAddress,
    ) -> blockifier::state::state_api::StateResult<starknet_api::core::ClassHash> {
        todo!()
    }

    fn get_compiled_class_hash(
        &mut self,
        class_hash: starknet_api::core::ClassHash,
    ) -> blockifier::state::state_api::StateResult<starknet_api::core::CompiledClassHash> {
        todo!()
    }

    fn get_compiled_contract_class(
        &mut self,
        class_hash: starknet_api::core::ClassHash,
    ) -> blockifier::state::state_api::StateResult<
        blockifier::execution::contract_class::ContractClass,
    > {
        todo!()
    }

    fn get_nonce_at(
        &mut self,
        contract_address: starknet_api::core::ContractAddress,
    ) -> blockifier::state::state_api::StateResult<starknet_api::core::Nonce> {
        todo!()
    }

    fn get_storage_at(
        &mut self,
        contract_address: starknet_api::core::ContractAddress,
        key: starknet_api::state::StorageKey,
    ) -> blockifier::state::state_api::StateResult<starknet_api::hash::StarkFelt> {
        todo!()
    }
}

pub(super) struct CachedState<S: StateReader>(pub(super) Arc<RwLock<CachedStateInner<S>>>);

impl<S> Clone for CachedState<S>
where
    S: StateReader + Send + Sync,
{
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

type DeclaredClass = (CompiledClass, Option<FlattenedSierraClass>);

#[derive(Debug)]
pub(super) struct CachedStateInner<S: StateReader> {
    pub(super) inner: cached_state::CachedState<S>,
    pub(super) declared_classes: HashMap<katana_primitives::class::ClassHash, DeclaredClass>,
}

impl<S> CachedState<S>
where
    S: StateReader + Send + Sync,
{
    pub(super) fn new(state: S) -> Self {
        let declared_classes = HashMap::new();
        let cached_state = cached_state::CachedState::new(state, GlobalContractCache::default());
        let inner = CachedStateInner { inner: cached_state, declared_classes };

        Self(Arc::new(RwLock::new(inner)))
    }
}

impl<S> ContractClassProvider for CachedState<S>
where
    S: StateReader + Send + Sync,
{
    fn class(
        &self,
        hash: katana_primitives::class::ClassHash,
    ) -> ProviderResult<Option<CompiledClass>> {
        let inner = self.0.read();

        let class = inner.declared_classes.get(&hash).map(|(class, _)| class.clone());
        Ok(class)
    }

    fn compiled_class_hash_of_class_hash(
        &self,
        hash: katana_primitives::class::ClassHash,
    ) -> ProviderResult<Option<katana_primitives::class::CompiledClassHash>> {
        todo!()
    }

    fn sierra_class(
        &self,
        hash: katana_primitives::class::ClassHash,
    ) -> ProviderResult<Option<FlattenedSierraClass>> {
        todo!()
    }
}

impl<S> StateProvider for CachedState<S>
where
    S: StateReader + Send + Sync,
{
    fn class_hash_of_contract(
        &self,
        address: katana_primitives::contract::ContractAddress,
    ) -> ProviderResult<Option<katana_primitives::class::ClassHash>> {
        todo!()
    }

    fn nonce(
        &self,
        address: katana_primitives::contract::ContractAddress,
    ) -> ProviderResult<Option<katana_primitives::contract::Nonce>> {
        todo!()
    }

    fn storage(
        &self,
        address: katana_primitives::contract::ContractAddress,
        storage_key: katana_primitives::contract::StorageKey,
    ) -> ProviderResult<Option<katana_primitives::contract::StorageValue>> {
        todo!()
    }
}
