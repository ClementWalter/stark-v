//! BlockHash database component from [`crate::db::Database`]
//! it is used inside [crate::db::DatabaseComponents`]

use crate::{B256, U256};
#[cfg(target_has_atomic = "ptr")]
use alloc::sync::Arc;
#[cfg(not(target_has_atomic = "ptr"))]
use alloc::rc::Rc as Arc;
use auto_impl::auto_impl;
use core::ops::Deref;

#[auto_impl(& mut, Box)]
pub trait BlockHash {
    type Error;

    /// Get block hash by block number
    fn block_hash(&mut self, number: U256) -> Result<B256, Self::Error>;
}

#[cfg_attr(
    target_has_atomic = "ptr",
    auto_impl(&, Box, alloc::sync::Arc)
)]
#[cfg_attr(
    not(target_has_atomic = "ptr"),
    auto_impl(&, Box, alloc::rc::Rc)
)]
pub trait BlockHashRef {
    type Error;

    /// Get block hash by block number
    fn block_hash(&self, number: U256) -> Result<B256, Self::Error>;
}

impl<T> BlockHash for &T
where
    T: BlockHashRef,
{
    type Error = <T as BlockHashRef>::Error;

    fn block_hash(&mut self, number: U256) -> Result<B256, Self::Error> {
        BlockHashRef::block_hash(*self, number)
    }
}

impl<T> BlockHash for Arc<T>
where
    T: BlockHashRef,
{
    type Error = <T as BlockHashRef>::Error;

    fn block_hash(&mut self, number: U256) -> Result<B256, Self::Error> {
        BlockHashRef::block_hash(self.deref(), number)
    }
}
