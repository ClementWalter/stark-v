//! State database component from [`crate::db::Database`]
//! it is used inside [crate::db::DatabaseComponents`]

use crate::{AccountInfo, Address, Bytecode, B256, U256};
#[cfg(target_has_atomic = "ptr")]
use alloc::sync::Arc;
#[cfg(not(target_has_atomic = "ptr"))]
use alloc::rc::Rc as Arc;
use auto_impl::auto_impl;
use core::ops::Deref;

#[auto_impl(& mut, Box)]
pub trait State {
    type Error;

    /// Get basic account information.
    fn basic(&mut self, address: Address) -> Result<Option<AccountInfo>, Self::Error>;
    /// Get account code by its hash
    fn code_by_hash(&mut self, code_hash: B256) -> Result<Bytecode, Self::Error>;
    /// Get storage value of address at index.
    fn storage(&mut self, address: Address, index: U256) -> Result<U256, Self::Error>;
}

#[cfg_attr(
    target_has_atomic = "ptr",
    auto_impl(&, Box, alloc::sync::Arc)
)]
#[cfg_attr(
    not(target_has_atomic = "ptr"),
    auto_impl(&, Box, alloc::rc::Rc)
)]
pub trait StateRef {
    type Error;

    /// Whether account at address exists.
    //fn exists(&self, address: Address) -> Option<AccountInfo>;
    /// Get basic account information.
    fn basic(&self, address: Address) -> Result<Option<AccountInfo>, Self::Error>;
    /// Get account code by its hash
    fn code_by_hash(&self, code_hash: B256) -> Result<Bytecode, Self::Error>;
    /// Get storage value of address at index.
    fn storage(&self, address: Address, index: U256) -> Result<U256, Self::Error>;
}

impl<T> State for &T
where
    T: StateRef,
{
    type Error = <T as StateRef>::Error;

    fn basic(&mut self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        StateRef::basic(*self, address)
    }

    fn code_by_hash(&mut self, code_hash: B256) -> Result<Bytecode, Self::Error> {
        StateRef::code_by_hash(*self, code_hash)
    }

    fn storage(&mut self, address: Address, index: U256) -> Result<U256, Self::Error> {
        StateRef::storage(*self, address, index)
    }
}

impl<T> State for Arc<T>
where
    T: StateRef,
{
    type Error = <T as StateRef>::Error;

    fn basic(&mut self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        StateRef::basic(self.deref(), address)
    }

    fn code_by_hash(&mut self, code_hash: B256) -> Result<Bytecode, Self::Error> {
        StateRef::code_by_hash(self.deref(), code_hash)
    }

    fn storage(&mut self, address: Address, index: U256) -> Result<U256, Self::Error> {
        StateRef::storage(self.deref(), address, index)
    }
}
