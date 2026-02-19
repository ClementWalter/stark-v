//! Withdrawal type (EIP-4895)
//!
//! Withdrawals are applied after transaction execution to credit validator
//! balances. Amounts are specified in gwei.

#[cfg(target_arch = "riscv32")]
extern crate alloc;

#[cfg(not(target_arch = "riscv32"))]
use std::vec::Vec;

#[cfg(target_arch = "riscv32")]
use alloc::vec::Vec;

use crate::crypto::rlp;
use crate::crypto::rlp::RlpError;
use crate::types::{Address, U256};

const GWEI_IN_WEI: u64 = 1_000_000_000;

/// Withdrawal entry (EIP-4895).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Withdrawal {
    /// Withdrawal index in the block.
    pub index: u64,
    /// Validator index for the withdrawal.
    pub validator_index: u64,
    /// Recipient address.
    pub address: Address,
    /// Amount in gwei.
    pub amount_gwei: u64,
}

impl Withdrawal {
    /// Encodes the withdrawal as RLP.
    pub fn encode_rlp(&self) -> Vec<u8> {
        let items = [
            rlp::encode_u64(self.index),
            rlp::encode_u64(self.validator_index),
            rlp::encode_address(&self.address),
            rlp::encode_u64(self.amount_gwei),
        ];
        rlp::encode_list(&items)
    }

    /// Decodes a withdrawal from RLP.
    pub fn decode_rlp(input: &[u8]) -> Result<Self, RlpError> {
        let (items, rest) = rlp::decode_list(input)?;
        if !rest.is_empty() || items.len() != 4 {
            return Err(RlpError::InvalidLength);
        }

        let (index, _) = rlp::decode_u64(&items[0])?;
        let (validator_index, _) = rlp::decode_u64(&items[1])?;
        let (address, _) = rlp::decode_address(&items[2])?;
        let (amount_gwei, _) = rlp::decode_u64(&items[3])?;

        Ok(Withdrawal {
            index,
            validator_index,
            address,
            amount_gwei,
        })
    }

    /// Amount in wei.
    pub fn amount_wei(&self) -> U256 {
        U256::from(self.amount_gwei) * U256::from(GWEI_IN_WEI)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_withdrawal_encode_decode_roundtrip() {
        let withdrawal = Withdrawal {
            index: 1,
            validator_index: 42,
            address: Address::from([0x11; 20]),
            amount_gwei: 64,
        };

        let encoded = withdrawal.encode_rlp();
        let decoded = Withdrawal::decode_rlp(&encoded).expect("decode withdrawal");

        assert_eq!(decoded, withdrawal);
    }

    #[test]
    fn test_withdrawal_amount_wei() {
        let withdrawal = Withdrawal {
            index: 0,
            validator_index: 0,
            address: Address::from([0x22; 20]),
            amount_gwei: 2,
        };

        assert_eq!(
            withdrawal.amount_wei(),
            U256::from(2u64) * U256::from(GWEI_IN_WEI)
        );
    }
}
