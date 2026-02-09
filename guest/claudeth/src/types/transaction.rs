//! Ethereum transaction types
//!
//! This module provides transaction types for Ethereum post-Fusaka:
//! - Legacy transactions (Type 0)
//! - EIP-2930 transactions (Type 1)
//! - EIP-1559 transactions (Type 2)
//!
//! Each transaction type supports RLP encoding/decoding, hashing, and
//! signature recovery for sender address verification.

#[cfg(target_arch = "riscv32")]
extern crate alloc;

#[cfg(not(target_arch = "riscv32"))]
use std::{vec, vec::Vec};

#[cfg(target_arch = "riscv32")]
use alloc::{vec, vec::Vec};

use core::fmt;

use serde::{Deserialize, Serialize};

use crate::crypto::{keccak256, recover_address, rlp, RlpError, Secp256k1Error};
use crate::types::{Address, Bytes, Hash, U256};

// =============================================================================
// Access List Types
// =============================================================================

/// Access list entry for EIP-2930 and EIP-1559 transactions.
///
/// An access list specifies addresses and storage keys that the transaction
/// plans to access, allowing for reduced gas costs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccessListEntry {
    /// The address to access
    pub address: Address,
    /// Storage keys to access at this address
    pub storage_keys: Vec<Hash>,
}

impl AccessListEntry {
    /// Encodes the access list entry as RLP.
    pub fn encode_rlp(&self) -> Vec<u8> {
        let address_encoded = rlp::encode_address(&self.address);
        let keys_encoded: Vec<Vec<u8>> = self
            .storage_keys
            .iter()
            .map(rlp::encode_hash)
            .collect();
        let keys_list = rlp::encode_list(&keys_encoded);

        rlp::encode_list(&[address_encoded, keys_list])
    }

    /// Decodes an access list entry from RLP.
    pub fn decode_rlp(input: &[u8]) -> Result<(Self, &[u8]), RlpError> {
        let (items, rest) = rlp::decode_list(input)?;

        if items.len() != 2 {
            return Err(RlpError::InvalidEncoding);
        }

        let (address, _) = rlp::decode_address(&items[0])?;
        let (keys_items, _) = rlp::decode_list(&items[1])?;

        let mut storage_keys = Vec::with_capacity(keys_items.len());
        for key_item in keys_items {
            let (key, _) = rlp::decode_hash(&key_item)?;
            storage_keys.push(key);
        }

        Ok((
            AccessListEntry {
                address,
                storage_keys,
            },
            rest,
        ))
    }
}

// =============================================================================
// Legacy Transaction (Type 0)
// =============================================================================

/// Legacy Ethereum transaction (pre-EIP-2718).
///
/// This is the original transaction format used before typed transactions.
///
/// # Examples
///
/// ```
/// use claudeth::types::{Address, Hash, U256, Bytes};
/// use claudeth::types::transaction::LegacyTransaction;
///
/// let tx = LegacyTransaction {
///     nonce: U256::from(0u64),
///     gas_price: U256::from(20_000_000_000u64),
///     gas_limit: U256::from(21000u64),
///     to: Some(Address::ZERO),
///     value: U256::from(1_000_000_000_000_000_000u64), // 1 ETH
///     data: Bytes::new(),
///     v: U256::from(27u64),
///     r: U256::ZERO,
///     s: U256::ZERO,
/// };
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegacyTransaction {
    /// Transaction nonce
    pub nonce: U256,
    /// Gas price in wei
    pub gas_price: U256,
    /// Gas limit
    pub gas_limit: U256,
    /// Recipient address (None for contract creation)
    pub to: Option<Address>,
    /// Value in wei
    pub value: U256,
    /// Transaction data
    pub data: Bytes,
    /// Signature v component
    pub v: U256,
    /// Signature r component
    pub r: U256,
    /// Signature s component
    pub s: U256,
}

impl LegacyTransaction {
    /// Encodes the transaction as RLP (including signature).
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeth::types::{Address, U256, Bytes};
    /// use claudeth::types::transaction::LegacyTransaction;
    ///
    /// let tx = LegacyTransaction {
    ///     nonce: U256::from(0u64),
    ///     gas_price: U256::from(20_000_000_000u64),
    ///     gas_limit: U256::from(21000u64),
    ///     to: Some(Address::ZERO),
    ///     value: U256::from(1_000_000_000_000_000_000u64),
    ///     data: Bytes::new(),
    ///     v: U256::from(27u64),
    ///     r: U256::ZERO,
    ///     s: U256::ZERO,
    /// };
    ///
    /// let encoded = tx.encode_rlp();
    /// assert!(!encoded.is_empty());
    /// ```
    pub fn encode_rlp(&self) -> Vec<u8> {
        let to_encoded = if let Some(to) = self.to {
            rlp::encode_address(&to)
        } else {
            rlp::encode_bytes(&[])
        };

        let items = vec![
            rlp::encode_u256(&self.nonce),
            rlp::encode_u256(&self.gas_price),
            rlp::encode_u256(&self.gas_limit),
            to_encoded,
            rlp::encode_u256(&self.value),
            rlp::encode_bytes(self.data.as_ref()),
            rlp::encode_u256(&self.v),
            rlp::encode_u256(&self.r),
            rlp::encode_u256(&self.s),
        ];

        rlp::encode_list(&items)
    }

    /// Decodes a legacy transaction from RLP.
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeth::types::{Address, U256, Bytes};
    /// use claudeth::types::transaction::LegacyTransaction;
    ///
    /// let tx = LegacyTransaction {
    ///     nonce: U256::from(0u64),
    ///     gas_price: U256::from(20_000_000_000u64),
    ///     gas_limit: U256::from(21000u64),
    ///     to: Some(Address::ZERO),
    ///     value: U256::from(1_000_000_000_000_000_000u64),
    ///     data: Bytes::new(),
    ///     v: U256::from(27u64),
    ///     r: U256::ZERO,
    ///     s: U256::ZERO,
    /// };
    ///
    /// let encoded = tx.encode_rlp();
    /// let decoded = LegacyTransaction::decode_rlp(&encoded).unwrap();
    /// assert_eq!(tx, decoded);
    /// ```
    pub fn decode_rlp(input: &[u8]) -> Result<Self, RlpError> {
        let (items, _) = rlp::decode_list(input)?;

        if items.len() != 9 {
            return Err(RlpError::InvalidEncoding);
        }

        let (nonce, _) = rlp::decode_u256(&items[0])?;
        let (gas_price, _) = rlp::decode_u256(&items[1])?;
        let (gas_limit, _) = rlp::decode_u256(&items[2])?;

        // Decode 'to' field (empty bytes = None)
        let (to_bytes, _) = rlp::decode_bytes(&items[3])?;
        let to = if to_bytes.is_empty() {
            None
        } else {
            if to_bytes.len() != 20 {
                return Err(RlpError::InvalidLength);
            }
            let mut addr_bytes = [0u8; 20];
            addr_bytes.copy_from_slice(&to_bytes);
            Some(Address::from(addr_bytes))
        };

        let (value, _) = rlp::decode_u256(&items[4])?;
        let (data_bytes, _) = rlp::decode_bytes(&items[5])?;
        let data = Bytes::from_slice(&data_bytes);
        let (v, _) = rlp::decode_u256(&items[6])?;
        let (r, _) = rlp::decode_u256(&items[7])?;
        let (s, _) = rlp::decode_u256(&items[8])?;

        Ok(LegacyTransaction {
            nonce,
            gas_price,
            gas_limit,
            to,
            value,
            data,
            v,
            r,
            s,
        })
    }

    /// Computes the transaction hash (Keccak256 of RLP encoding).
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeth::types::{Address, U256, Bytes};
    /// use claudeth::types::transaction::LegacyTransaction;
    ///
    /// let tx = LegacyTransaction {
    ///     nonce: U256::from(0u64),
    ///     gas_price: U256::from(20_000_000_000u64),
    ///     gas_limit: U256::from(21000u64),
    ///     to: Some(Address::ZERO),
    ///     value: U256::from(1_000_000_000_000_000_000u64),
    ///     data: Bytes::new(),
    ///     v: U256::from(27u64),
    ///     r: U256::ZERO,
    ///     s: U256::ZERO,
    /// };
    ///
    /// let hash = tx.hash();
    /// assert_eq!(hash.as_bytes().len(), 32);
    /// ```
    pub fn hash(&self) -> Hash {
        let encoded = self.encode_rlp();
        keccak256(&encoded)
    }

    /// Computes the signing hash (hash of unsigned transaction data).
    ///
    /// This is the hash that was signed to produce the v, r, s values.
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeth::types::{Address, U256, Bytes};
    /// use claudeth::types::transaction::LegacyTransaction;
    ///
    /// let tx = LegacyTransaction {
    ///     nonce: U256::from(0u64),
    ///     gas_price: U256::from(20_000_000_000u64),
    ///     gas_limit: U256::from(21000u64),
    ///     to: Some(Address::ZERO),
    ///     value: U256::from(1_000_000_000_000_000_000u64),
    ///     data: Bytes::new(),
    ///     v: U256::from(27u64),
    ///     r: U256::ZERO,
    ///     s: U256::ZERO,
    /// };
    ///
    /// let signing_hash = tx.signing_hash();
    /// assert_eq!(signing_hash.as_bytes().len(), 32);
    /// ```
    pub fn signing_hash(&self) -> Hash {
        // For legacy transactions, extract chain_id from v if present (EIP-155)
        // v = chain_id * 2 + 35 + {0,1}
        // If v >= 35, it's EIP-155, otherwise it's 27 or 28
        let v_u64 = self.v.as_u64();

        let items = if v_u64 >= 35 {
            // EIP-155: include chain_id in signing hash
            let chain_id = (v_u64 - 35) / 2;
            let to_encoded = if let Some(to) = self.to {
                rlp::encode_address(&to)
            } else {
                rlp::encode_bytes(&[])
            };

            vec![
                rlp::encode_u256(&self.nonce),
                rlp::encode_u256(&self.gas_price),
                rlp::encode_u256(&self.gas_limit),
                to_encoded,
                rlp::encode_u256(&self.value),
                rlp::encode_bytes(self.data.as_ref()),
                rlp::encode_u64(chain_id),
                rlp::encode_bytes(&[]), // r = empty
                rlp::encode_bytes(&[]), // s = empty
            ]
        } else {
            // Pre-EIP-155: no chain_id
            let to_encoded = if let Some(to) = self.to {
                rlp::encode_address(&to)
            } else {
                rlp::encode_bytes(&[])
            };

            vec![
                rlp::encode_u256(&self.nonce),
                rlp::encode_u256(&self.gas_price),
                rlp::encode_u256(&self.gas_limit),
                to_encoded,
                rlp::encode_u256(&self.value),
                rlp::encode_bytes(self.data.as_ref()),
            ]
        };

        let encoded = rlp::encode_list(&items);
        keccak256(&encoded)
    }

    /// Recovers the sender address from the transaction signature.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use claudeth::types::{Address, U256, Bytes};
    /// use claudeth::types::transaction::LegacyTransaction;
    ///
    /// let tx = LegacyTransaction {
    ///     nonce: U256::from(0u64),
    ///     gas_price: U256::from(20_000_000_000u64),
    ///     gas_limit: U256::from(21000u64),
    ///     to: Some(Address::ZERO),
    ///     value: U256::from(1_000_000_000_000_000_000u64),
    ///     data: Bytes::new(),
    ///     v: U256::from(27u64),
    ///     r: U256::from(1u64),
    ///     s: U256::from(1u64),
    /// };
    ///
    /// // This will fail with invalid signature, but demonstrates API
    /// let result = tx.recover_sender();
    /// assert!(result.is_err());
    /// ```
    pub fn recover_sender(&self) -> Result<Address, Secp256k1Error> {
        let signing_hash = self.signing_hash();

        // Extract recovery_id from v
        let v_u64 = self.v.as_u64();
        let recovery_id = if v_u64 >= 35 {
            // EIP-155: v = chain_id * 2 + 35 + {0,1}
            ((v_u64 - 35) % 2) as u8
        } else {
            // Pre-EIP-155: v = 27 or 28
            (v_u64 - 27) as u8
        };

        // Convert r and s to 64-byte signature
        let r_bytes = self.r.to_be_bytes();
        let s_bytes = self.s.to_be_bytes();
        let mut signature = [0u8; 64];
        signature[..32].copy_from_slice(&r_bytes);
        signature[32..].copy_from_slice(&s_bytes);

        recover_address(&signing_hash, &signature, recovery_id)
    }
}

// =============================================================================
// EIP-2930 Transaction (Type 1)
// =============================================================================

/// EIP-2930 transaction with access list.
///
/// This transaction type was introduced to support access lists, which
/// specify addresses and storage keys that the transaction plans to access.
///
/// # Examples
///
/// ```
/// use claudeth::types::{Address, Hash, U256, Bytes};
/// use claudeth::types::transaction::{Eip2930Transaction, AccessListEntry};
///
/// let tx = Eip2930Transaction {
///     chain_id: U256::from(1u64),
///     nonce: U256::from(0u64),
///     gas_price: U256::from(20_000_000_000u64),
///     gas_limit: U256::from(21000u64),
///     to: Some(Address::ZERO),
///     value: U256::from(1_000_000_000_000_000_000u64),
///     data: Bytes::new(),
///     access_list: vec![],
///     v: U256::from(0u64),
///     r: U256::ZERO,
///     s: U256::ZERO,
/// };
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Eip2930Transaction {
    /// Chain ID
    pub chain_id: U256,
    /// Transaction nonce
    pub nonce: U256,
    /// Gas price in wei
    pub gas_price: U256,
    /// Gas limit
    pub gas_limit: U256,
    /// Recipient address (None for contract creation)
    pub to: Option<Address>,
    /// Value in wei
    pub value: U256,
    /// Transaction data
    pub data: Bytes,
    /// Access list
    pub access_list: Vec<AccessListEntry>,
    /// Signature v component (0 or 1 for EIP-2930)
    pub v: U256,
    /// Signature r component
    pub r: U256,
    /// Signature s component
    pub s: U256,
}

impl Eip2930Transaction {
    /// Encodes the transaction as RLP (including signature).
    ///
    /// The encoding is: 0x01 || rlp([...])
    pub fn encode_rlp(&self) -> Vec<u8> {
        let to_encoded = if let Some(to) = self.to {
            rlp::encode_address(&to)
        } else {
            rlp::encode_bytes(&[])
        };

        let access_list_encoded: Vec<Vec<u8>> =
            self.access_list.iter().map(|e| e.encode_rlp()).collect();

        let items = vec![
            rlp::encode_u256(&self.chain_id),
            rlp::encode_u256(&self.nonce),
            rlp::encode_u256(&self.gas_price),
            rlp::encode_u256(&self.gas_limit),
            to_encoded,
            rlp::encode_u256(&self.value),
            rlp::encode_bytes(self.data.as_ref()),
            rlp::encode_list(&access_list_encoded),
            rlp::encode_u256(&self.v),
            rlp::encode_u256(&self.r),
            rlp::encode_u256(&self.s),
        ];

        let mut result = vec![0x01];
        result.extend(rlp::encode_list(&items));
        result
    }

    /// Decodes an EIP-2930 transaction from RLP (without type prefix).
    pub fn decode_rlp(input: &[u8]) -> Result<Self, RlpError> {
        let (items, _) = rlp::decode_list(input)?;

        if items.len() != 11 {
            return Err(RlpError::InvalidEncoding);
        }

        let (chain_id, _) = rlp::decode_u256(&items[0])?;
        let (nonce, _) = rlp::decode_u256(&items[1])?;
        let (gas_price, _) = rlp::decode_u256(&items[2])?;
        let (gas_limit, _) = rlp::decode_u256(&items[3])?;

        let (to_bytes, _) = rlp::decode_bytes(&items[4])?;
        let to = if to_bytes.is_empty() {
            None
        } else {
            if to_bytes.len() != 20 {
                return Err(RlpError::InvalidLength);
            }
            let mut addr_bytes = [0u8; 20];
            addr_bytes.copy_from_slice(&to_bytes);
            Some(Address::from(addr_bytes))
        };

        let (value, _) = rlp::decode_u256(&items[5])?;
        let (data_bytes, _) = rlp::decode_bytes(&items[6])?;
        let data = Bytes::from_slice(&data_bytes);

        let (access_list_items, _) = rlp::decode_list(&items[7])?;
        let mut access_list = Vec::with_capacity(access_list_items.len());
        for item in access_list_items {
            let (entry, _) = AccessListEntry::decode_rlp(&item)?;
            access_list.push(entry);
        }

        let (v, _) = rlp::decode_u256(&items[8])?;
        let (r, _) = rlp::decode_u256(&items[9])?;
        let (s, _) = rlp::decode_u256(&items[10])?;

        Ok(Eip2930Transaction {
            chain_id,
            nonce,
            gas_price,
            gas_limit,
            to,
            value,
            data,
            access_list,
            v,
            r,
            s,
        })
    }

    /// Computes the transaction hash (Keccak256 of RLP encoding with type).
    pub fn hash(&self) -> Hash {
        let encoded = self.encode_rlp();
        keccak256(&encoded)
    }

    /// Computes the signing hash (hash of unsigned transaction data with type).
    pub fn signing_hash(&self) -> Hash {
        let to_encoded = if let Some(to) = self.to {
            rlp::encode_address(&to)
        } else {
            rlp::encode_bytes(&[])
        };

        let access_list_encoded: Vec<Vec<u8>> =
            self.access_list.iter().map(|e| e.encode_rlp()).collect();

        let items = vec![
            rlp::encode_u256(&self.chain_id),
            rlp::encode_u256(&self.nonce),
            rlp::encode_u256(&self.gas_price),
            rlp::encode_u256(&self.gas_limit),
            to_encoded,
            rlp::encode_u256(&self.value),
            rlp::encode_bytes(self.data.as_ref()),
            rlp::encode_list(&access_list_encoded),
        ];

        let mut result = vec![0x01];
        result.extend(rlp::encode_list(&items));
        keccak256(&result)
    }

    /// Recovers the sender address from the transaction signature.
    pub fn recover_sender(&self) -> Result<Address, Secp256k1Error> {
        let signing_hash = self.signing_hash();

        // For EIP-2930, v is 0 or 1
        let recovery_id = self.v.as_u64() as u8;

        // Convert r and s to 64-byte signature
        let r_bytes = self.r.to_be_bytes();
        let s_bytes = self.s.to_be_bytes();
        let mut signature = [0u8; 64];
        signature[..32].copy_from_slice(&r_bytes);
        signature[32..].copy_from_slice(&s_bytes);

        recover_address(&signing_hash, &signature, recovery_id)
    }
}

// =============================================================================
// EIP-1559 Transaction (Type 2)
// =============================================================================

/// EIP-1559 transaction with dynamic fee.
///
/// This transaction type was introduced with EIP-1559 to support a dynamic
/// fee market with base fees and priority fees.
///
/// # Examples
///
/// ```
/// use claudeth::types::{Address, Hash, U256, Bytes};
/// use claudeth::types::transaction::{Eip1559Transaction, AccessListEntry};
///
/// let tx = Eip1559Transaction {
///     chain_id: U256::from(1u64),
///     nonce: U256::from(0u64),
///     max_priority_fee_per_gas: U256::from(2_000_000_000u64),
///     max_fee_per_gas: U256::from(20_000_000_000u64),
///     gas_limit: U256::from(21000u64),
///     to: Some(Address::ZERO),
///     value: U256::from(1_000_000_000_000_000_000u64),
///     data: Bytes::new(),
///     access_list: vec![],
///     v: U256::from(0u64),
///     r: U256::ZERO,
///     s: U256::ZERO,
/// };
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Eip1559Transaction {
    /// Chain ID
    pub chain_id: U256,
    /// Transaction nonce
    pub nonce: U256,
    /// Max priority fee per gas (tip)
    pub max_priority_fee_per_gas: U256,
    /// Max fee per gas
    pub max_fee_per_gas: U256,
    /// Gas limit
    pub gas_limit: U256,
    /// Recipient address (None for contract creation)
    pub to: Option<Address>,
    /// Value in wei
    pub value: U256,
    /// Transaction data
    pub data: Bytes,
    /// Access list
    pub access_list: Vec<AccessListEntry>,
    /// Signature v component (0 or 1 for EIP-1559)
    pub v: U256,
    /// Signature r component
    pub r: U256,
    /// Signature s component
    pub s: U256,
}

impl Eip1559Transaction {
    /// Encodes the transaction as RLP (including signature).
    ///
    /// The encoding is: 0x02 || rlp([...])
    pub fn encode_rlp(&self) -> Vec<u8> {
        let to_encoded = if let Some(to) = self.to {
            rlp::encode_address(&to)
        } else {
            rlp::encode_bytes(&[])
        };

        let access_list_encoded: Vec<Vec<u8>> =
            self.access_list.iter().map(|e| e.encode_rlp()).collect();

        let items = vec![
            rlp::encode_u256(&self.chain_id),
            rlp::encode_u256(&self.nonce),
            rlp::encode_u256(&self.max_priority_fee_per_gas),
            rlp::encode_u256(&self.max_fee_per_gas),
            rlp::encode_u256(&self.gas_limit),
            to_encoded,
            rlp::encode_u256(&self.value),
            rlp::encode_bytes(self.data.as_ref()),
            rlp::encode_list(&access_list_encoded),
            rlp::encode_u256(&self.v),
            rlp::encode_u256(&self.r),
            rlp::encode_u256(&self.s),
        ];

        let mut result = vec![0x02];
        result.extend(rlp::encode_list(&items));
        result
    }

    /// Decodes an EIP-1559 transaction from RLP (without type prefix).
    pub fn decode_rlp(input: &[u8]) -> Result<Self, RlpError> {
        let (items, _) = rlp::decode_list(input)?;

        if items.len() != 12 {
            return Err(RlpError::InvalidEncoding);
        }

        let (chain_id, _) = rlp::decode_u256(&items[0])?;
        let (nonce, _) = rlp::decode_u256(&items[1])?;
        let (max_priority_fee_per_gas, _) = rlp::decode_u256(&items[2])?;
        let (max_fee_per_gas, _) = rlp::decode_u256(&items[3])?;
        let (gas_limit, _) = rlp::decode_u256(&items[4])?;

        let (to_bytes, _) = rlp::decode_bytes(&items[5])?;
        let to = if to_bytes.is_empty() {
            None
        } else {
            if to_bytes.len() != 20 {
                return Err(RlpError::InvalidLength);
            }
            let mut addr_bytes = [0u8; 20];
            addr_bytes.copy_from_slice(&to_bytes);
            Some(Address::from(addr_bytes))
        };

        let (value, _) = rlp::decode_u256(&items[6])?;
        let (data_bytes, _) = rlp::decode_bytes(&items[7])?;
        let data = Bytes::from_slice(&data_bytes);

        let (access_list_items, _) = rlp::decode_list(&items[8])?;
        let mut access_list = Vec::with_capacity(access_list_items.len());
        for item in access_list_items {
            let (entry, _) = AccessListEntry::decode_rlp(&item)?;
            access_list.push(entry);
        }

        let (v, _) = rlp::decode_u256(&items[9])?;
        let (r, _) = rlp::decode_u256(&items[10])?;
        let (s, _) = rlp::decode_u256(&items[11])?;

        Ok(Eip1559Transaction {
            chain_id,
            nonce,
            max_priority_fee_per_gas,
            max_fee_per_gas,
            gas_limit,
            to,
            value,
            data,
            access_list,
            v,
            r,
            s,
        })
    }

    /// Computes the transaction hash (Keccak256 of RLP encoding with type).
    pub fn hash(&self) -> Hash {
        let encoded = self.encode_rlp();
        keccak256(&encoded)
    }

    /// Computes the signing hash (hash of unsigned transaction data with type).
    pub fn signing_hash(&self) -> Hash {
        let to_encoded = if let Some(to) = self.to {
            rlp::encode_address(&to)
        } else {
            rlp::encode_bytes(&[])
        };

        let access_list_encoded: Vec<Vec<u8>> =
            self.access_list.iter().map(|e| e.encode_rlp()).collect();

        let items = vec![
            rlp::encode_u256(&self.chain_id),
            rlp::encode_u256(&self.nonce),
            rlp::encode_u256(&self.max_priority_fee_per_gas),
            rlp::encode_u256(&self.max_fee_per_gas),
            rlp::encode_u256(&self.gas_limit),
            to_encoded,
            rlp::encode_u256(&self.value),
            rlp::encode_bytes(self.data.as_ref()),
            rlp::encode_list(&access_list_encoded),
        ];

        let mut result = vec![0x02];
        result.extend(rlp::encode_list(&items));
        keccak256(&result)
    }

    /// Recovers the sender address from the transaction signature.
    pub fn recover_sender(&self) -> Result<Address, Secp256k1Error> {
        let signing_hash = self.signing_hash();

        // For EIP-1559, v is 0 or 1
        let recovery_id = self.v.as_u64() as u8;

        // Convert r and s to 64-byte signature
        let r_bytes = self.r.to_be_bytes();
        let s_bytes = self.s.to_be_bytes();
        let mut signature = [0u8; 64];
        signature[..32].copy_from_slice(&r_bytes);
        signature[32..].copy_from_slice(&s_bytes);

        recover_address(&signing_hash, &signature, recovery_id)
    }
}

// =============================================================================
// Transaction Enum
// =============================================================================

/// Ethereum transaction (any type).
///
/// This enum wraps all transaction types and provides unified methods for
/// encoding, decoding, hashing, and signature recovery.
///
/// # Examples
///
/// ```
/// use claudeth::types::{Address, U256, Bytes};
/// use claudeth::types::transaction::{Transaction, LegacyTransaction};
///
/// let legacy = LegacyTransaction {
///     nonce: U256::from(0u64),
///     gas_price: U256::from(20_000_000_000u64),
///     gas_limit: U256::from(21000u64),
///     to: Some(Address::ZERO),
///     value: U256::from(1_000_000_000_000_000_000u64),
///     data: Bytes::new(),
///     v: U256::from(27u64),
///     r: U256::ZERO,
///     s: U256::ZERO,
/// };
///
/// let tx = Transaction::Legacy(legacy);
/// let encoded = tx.encode_rlp();
/// assert!(!encoded.is_empty());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Transaction {
    /// Legacy transaction (type 0)
    Legacy(LegacyTransaction),
    /// EIP-2930 transaction (type 1)
    Eip2930(Eip2930Transaction),
    /// EIP-1559 transaction (type 2)
    Eip1559(Eip1559Transaction),
}

impl Transaction {
    /// Encodes the transaction as RLP.
    ///
    /// For legacy transactions, this is just the RLP encoding.
    /// For typed transactions, this includes the type prefix.
    pub fn encode_rlp(&self) -> Vec<u8> {
        match self {
            Transaction::Legacy(tx) => tx.encode_rlp(),
            Transaction::Eip2930(tx) => tx.encode_rlp(),
            Transaction::Eip1559(tx) => tx.encode_rlp(),
        }
    }

    /// Decodes a transaction from RLP.
    ///
    /// Automatically detects the transaction type based on the first byte.
    pub fn decode_rlp(input: &[u8]) -> Result<Self, RlpError> {
        if input.is_empty() {
            return Err(RlpError::UnexpectedEnd);
        }

        let first_byte = input[0];

        if first_byte == 0x01 {
            // EIP-2930 transaction
            let tx = Eip2930Transaction::decode_rlp(&input[1..])?;
            Ok(Transaction::Eip2930(tx))
        } else if first_byte == 0x02 {
            // EIP-1559 transaction
            let tx = Eip1559Transaction::decode_rlp(&input[1..])?;
            Ok(Transaction::Eip1559(tx))
        } else if first_byte >= 0xc0 {
            // Legacy transaction (RLP list)
            let tx = LegacyTransaction::decode_rlp(input)?;
            Ok(Transaction::Legacy(tx))
        } else {
            Err(RlpError::InvalidEncoding)
        }
    }

    /// Computes the transaction hash.
    pub fn hash(&self) -> Hash {
        match self {
            Transaction::Legacy(tx) => tx.hash(),
            Transaction::Eip2930(tx) => tx.hash(),
            Transaction::Eip1559(tx) => tx.hash(),
        }
    }

    /// Computes the signing hash.
    pub fn signing_hash(&self) -> Hash {
        match self {
            Transaction::Legacy(tx) => tx.signing_hash(),
            Transaction::Eip2930(tx) => tx.signing_hash(),
            Transaction::Eip1559(tx) => tx.signing_hash(),
        }
    }

    /// Recovers the sender address from the transaction signature.
    pub fn recover_sender(&self) -> Result<Address, Secp256k1Error> {
        match self {
            Transaction::Legacy(tx) => tx.recover_sender(),
            Transaction::Eip2930(tx) => tx.recover_sender(),
            Transaction::Eip1559(tx) => tx.recover_sender(),
        }
    }
}

impl fmt::Display for Transaction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Transaction::Legacy(_) => write!(f, "Legacy Transaction"),
            Transaction::Eip2930(_) => write!(f, "EIP-2930 Transaction"),
            Transaction::Eip1559(_) => write!(f, "EIP-1559 Transaction"),
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;
    use k256::ecdsa::SigningKey;

    // =========================================================================
    // AccessListEntry Tests
    // =========================================================================

    #[test]
    fn test_access_list_entry_encode_empty() {
        let entry = AccessListEntry {
            address: Address::ZERO,
            storage_keys: vec![],
        };
        let encoded = entry.encode_rlp();
        assert!(!encoded.is_empty());
    }

    #[test]
    fn test_access_list_entry_roundtrip() {
        let entry = AccessListEntry {
            address: Address::from([0x42; 20]),
            storage_keys: vec![Hash::from([0x01; 32]), Hash::from([0x02; 32])],
        };
        let encoded = entry.encode_rlp();
        let (decoded, _) = AccessListEntry::decode_rlp(&encoded).unwrap();
        assert_eq!(entry, decoded);
    }

    #[test]
    fn test_access_list_entry_empty_keys() {
        let entry = AccessListEntry {
            address: Address::from([0x42; 20]),
            storage_keys: vec![],
        };
        let encoded = entry.encode_rlp();
        let (decoded, _) = AccessListEntry::decode_rlp(&encoded).unwrap();
        assert_eq!(entry, decoded);
    }

    // =========================================================================
    // Legacy Transaction Tests
    // =========================================================================

    #[test]
    fn test_legacy_tx_encode() {
        let tx = LegacyTransaction {
            nonce: U256::from(0u64),
            gas_price: U256::from(20_000_000_000u64),
            gas_limit: U256::from(21000u64),
            to: Some(Address::ZERO),
            value: U256::from(1_000_000_000_000_000_000u64),
            data: Bytes::new(),
            v: U256::from(27u64),
            r: U256::ZERO,
            s: U256::ZERO,
        };
        let encoded = tx.encode_rlp();
        assert!(!encoded.is_empty());
        // Check it's an RLP list (starts with 0xc0 or higher)
        assert!(encoded[0] >= 0xc0);
    }

    #[test]
    fn test_legacy_tx_roundtrip() {
        let tx = LegacyTransaction {
            nonce: U256::from(5u64),
            gas_price: U256::from(20_000_000_000u64),
            gas_limit: U256::from(50000u64),
            to: Some(Address::from([0x42; 20])),
            value: U256::from(1_000_000_000_000_000_000u64),
            data: Bytes::from_slice(&[0x01, 0x02, 0x03]),
            v: U256::from(27u64),
            r: U256::from(1u64),
            s: U256::from(2u64),
        };
        let encoded = tx.encode_rlp();
        let decoded = LegacyTransaction::decode_rlp(&encoded).unwrap();
        assert_eq!(tx, decoded);
    }

    #[test]
    fn test_legacy_tx_contract_creation() {
        let tx = LegacyTransaction {
            nonce: U256::from(0u64),
            gas_price: U256::from(20_000_000_000u64),
            gas_limit: U256::from(100000u64),
            to: None, // Contract creation
            value: U256::ZERO,
            data: Bytes::from_slice(&[0x60, 0x60, 0x60]), // Some bytecode
            v: U256::from(27u64),
            r: U256::from(1u64),
            s: U256::from(1u64),
        };
        let encoded = tx.encode_rlp();
        let decoded = LegacyTransaction::decode_rlp(&encoded).unwrap();
        assert_eq!(tx, decoded);
        assert_eq!(decoded.to, None);
    }

    #[test]
    fn test_legacy_tx_empty_data() {
        let tx = LegacyTransaction {
            nonce: U256::from(0u64),
            gas_price: U256::from(20_000_000_000u64),
            gas_limit: U256::from(21000u64),
            to: Some(Address::ZERO),
            value: U256::from(1_000_000_000_000_000_000u64),
            data: Bytes::new(),
            v: U256::from(27u64),
            r: U256::ZERO,
            s: U256::ZERO,
        };
        let encoded = tx.encode_rlp();
        let decoded = LegacyTransaction::decode_rlp(&encoded).unwrap();
        assert_eq!(tx, decoded);
    }

    #[test]
    fn test_legacy_tx_large_values() {
        let tx = LegacyTransaction {
            nonce: U256::MAX,
            gas_price: U256::MAX,
            gas_limit: U256::MAX,
            to: Some(Address::from([0xff; 20])),
            value: U256::MAX,
            data: Bytes::from_slice(&[0xff; 100]),
            v: U256::from(27u64),
            r: U256::MAX,
            s: U256::MAX,
        };
        let encoded = tx.encode_rlp();
        let decoded = LegacyTransaction::decode_rlp(&encoded).unwrap();
        assert_eq!(tx, decoded);
    }

    #[test]
    fn test_legacy_tx_hash() {
        let tx = LegacyTransaction {
            nonce: U256::from(0u64),
            gas_price: U256::from(20_000_000_000u64),
            gas_limit: U256::from(21000u64),
            to: Some(Address::ZERO),
            value: U256::from(1_000_000_000_000_000_000u64),
            data: Bytes::new(),
            v: U256::from(27u64),
            r: U256::ZERO,
            s: U256::ZERO,
        };
        let hash = tx.hash();
        assert_eq!(hash.as_bytes().len(), 32);
        assert_ne!(hash, Hash::ZERO);
    }

    #[test]
    fn test_legacy_tx_hash_deterministic() {
        let tx = LegacyTransaction {
            nonce: U256::from(0u64),
            gas_price: U256::from(20_000_000_000u64),
            gas_limit: U256::from(21000u64),
            to: Some(Address::ZERO),
            value: U256::from(1_000_000_000_000_000_000u64),
            data: Bytes::new(),
            v: U256::from(27u64),
            r: U256::ZERO,
            s: U256::ZERO,
        };
        let hash1 = tx.hash();
        let hash2 = tx.hash();
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_legacy_tx_signing_hash() {
        let tx = LegacyTransaction {
            nonce: U256::from(0u64),
            gas_price: U256::from(20_000_000_000u64),
            gas_limit: U256::from(21000u64),
            to: Some(Address::ZERO),
            value: U256::from(1_000_000_000_000_000_000u64),
            data: Bytes::new(),
            v: U256::from(27u64), // Pre-EIP-155
            r: U256::ZERO,
            s: U256::ZERO,
        };
        let signing_hash = tx.signing_hash();
        assert_eq!(signing_hash.as_bytes().len(), 32);
    }

    #[test]
    fn test_legacy_tx_signing_hash_eip155() {
        let tx = LegacyTransaction {
            nonce: U256::from(0u64),
            gas_price: U256::from(20_000_000_000u64),
            gas_limit: U256::from(21000u64),
            to: Some(Address::ZERO),
            value: U256::from(1_000_000_000_000_000_000u64),
            data: Bytes::new(),
            v: U256::from(37u64), // EIP-155: chain_id=1, v=37 (1*2+35+0)
            r: U256::ZERO,
            s: U256::ZERO,
        };
        let signing_hash = tx.signing_hash();
        assert_eq!(signing_hash.as_bytes().len(), 32);
    }

    // =========================================================================
    // EIP-2930 Transaction Tests
    // =========================================================================

    #[test]
    fn test_eip2930_tx_encode() {
        let tx = Eip2930Transaction {
            chain_id: U256::from(1u64),
            nonce: U256::from(0u64),
            gas_price: U256::from(20_000_000_000u64),
            gas_limit: U256::from(21000u64),
            to: Some(Address::ZERO),
            value: U256::from(1_000_000_000_000_000_000u64),
            data: Bytes::new(),
            access_list: vec![],
            v: U256::from(0u64),
            r: U256::ZERO,
            s: U256::ZERO,
        };
        let encoded = tx.encode_rlp();
        assert!(!encoded.is_empty());
        assert_eq!(encoded[0], 0x01); // Type prefix
    }

    #[test]
    fn test_eip2930_tx_roundtrip() {
        let tx = Eip2930Transaction {
            chain_id: U256::from(1u64),
            nonce: U256::from(5u64),
            gas_price: U256::from(20_000_000_000u64),
            gas_limit: U256::from(50000u64),
            to: Some(Address::from([0x42; 20])),
            value: U256::from(1_000_000_000_000_000_000u64),
            data: Bytes::from_slice(&[0x01, 0x02, 0x03]),
            access_list: vec![AccessListEntry {
                address: Address::from([0x11; 20]),
                storage_keys: vec![Hash::from([0x22; 32])],
            }],
            v: U256::from(1u64),
            r: U256::from(1u64),
            s: U256::from(2u64),
        };
        let encoded = tx.encode_rlp();
        let decoded = Eip2930Transaction::decode_rlp(&encoded[1..]).unwrap();
        assert_eq!(tx, decoded);
    }

    #[test]
    fn test_eip2930_tx_empty_access_list() {
        let tx = Eip2930Transaction {
            chain_id: U256::from(1u64),
            nonce: U256::from(0u64),
            gas_price: U256::from(20_000_000_000u64),
            gas_limit: U256::from(21000u64),
            to: Some(Address::ZERO),
            value: U256::from(1_000_000_000_000_000_000u64),
            data: Bytes::new(),
            access_list: vec![],
            v: U256::from(0u64),
            r: U256::ZERO,
            s: U256::ZERO,
        };
        let encoded = tx.encode_rlp();
        let decoded = Eip2930Transaction::decode_rlp(&encoded[1..]).unwrap();
        assert_eq!(tx, decoded);
    }

    #[test]
    fn test_eip2930_tx_contract_creation() {
        let tx = Eip2930Transaction {
            chain_id: U256::from(1u64),
            nonce: U256::from(0u64),
            gas_price: U256::from(20_000_000_000u64),
            gas_limit: U256::from(100000u64),
            to: None, // Contract creation
            value: U256::ZERO,
            data: Bytes::from_slice(&[0x60, 0x60, 0x60]),
            access_list: vec![],
            v: U256::from(0u64),
            r: U256::from(1u64),
            s: U256::from(1u64),
        };
        let encoded = tx.encode_rlp();
        let decoded = Eip2930Transaction::decode_rlp(&encoded[1..]).unwrap();
        assert_eq!(tx, decoded);
        assert_eq!(decoded.to, None);
    }

    #[test]
    fn test_eip2930_tx_hash() {
        let tx = Eip2930Transaction {
            chain_id: U256::from(1u64),
            nonce: U256::from(0u64),
            gas_price: U256::from(20_000_000_000u64),
            gas_limit: U256::from(21000u64),
            to: Some(Address::ZERO),
            value: U256::from(1_000_000_000_000_000_000u64),
            data: Bytes::new(),
            access_list: vec![],
            v: U256::from(0u64),
            r: U256::ZERO,
            s: U256::ZERO,
        };
        let hash = tx.hash();
        assert_eq!(hash.as_bytes().len(), 32);
        assert_ne!(hash, Hash::ZERO);
    }

    #[test]
    fn test_eip2930_tx_signing_hash() {
        let tx = Eip2930Transaction {
            chain_id: U256::from(1u64),
            nonce: U256::from(0u64),
            gas_price: U256::from(20_000_000_000u64),
            gas_limit: U256::from(21000u64),
            to: Some(Address::ZERO),
            value: U256::from(1_000_000_000_000_000_000u64),
            data: Bytes::new(),
            access_list: vec![],
            v: U256::from(0u64),
            r: U256::ZERO,
            s: U256::ZERO,
        };
        let signing_hash = tx.signing_hash();
        assert_eq!(signing_hash.as_bytes().len(), 32);
    }

    // =========================================================================
    // EIP-1559 Transaction Tests
    // =========================================================================

    #[test]
    fn test_eip1559_tx_encode() {
        let tx = Eip1559Transaction {
            chain_id: U256::from(1u64),
            nonce: U256::from(0u64),
            max_priority_fee_per_gas: U256::from(2_000_000_000u64),
            max_fee_per_gas: U256::from(20_000_000_000u64),
            gas_limit: U256::from(21000u64),
            to: Some(Address::ZERO),
            value: U256::from(1_000_000_000_000_000_000u64),
            data: Bytes::new(),
            access_list: vec![],
            v: U256::from(0u64),
            r: U256::ZERO,
            s: U256::ZERO,
        };
        let encoded = tx.encode_rlp();
        assert!(!encoded.is_empty());
        assert_eq!(encoded[0], 0x02); // Type prefix
    }

    #[test]
    fn test_eip1559_tx_roundtrip() {
        let tx = Eip1559Transaction {
            chain_id: U256::from(1u64),
            nonce: U256::from(5u64),
            max_priority_fee_per_gas: U256::from(2_000_000_000u64),
            max_fee_per_gas: U256::from(20_000_000_000u64),
            gas_limit: U256::from(50000u64),
            to: Some(Address::from([0x42; 20])),
            value: U256::from(1_000_000_000_000_000_000u64),
            data: Bytes::from_slice(&[0x01, 0x02, 0x03]),
            access_list: vec![AccessListEntry {
                address: Address::from([0x11; 20]),
                storage_keys: vec![Hash::from([0x22; 32])],
            }],
            v: U256::from(1u64),
            r: U256::from(1u64),
            s: U256::from(2u64),
        };
        let encoded = tx.encode_rlp();
        let decoded = Eip1559Transaction::decode_rlp(&encoded[1..]).unwrap();
        assert_eq!(tx, decoded);
    }

    #[test]
    fn test_eip1559_tx_empty_access_list() {
        let tx = Eip1559Transaction {
            chain_id: U256::from(1u64),
            nonce: U256::from(0u64),
            max_priority_fee_per_gas: U256::from(2_000_000_000u64),
            max_fee_per_gas: U256::from(20_000_000_000u64),
            gas_limit: U256::from(21000u64),
            to: Some(Address::ZERO),
            value: U256::from(1_000_000_000_000_000_000u64),
            data: Bytes::new(),
            access_list: vec![],
            v: U256::from(0u64),
            r: U256::ZERO,
            s: U256::ZERO,
        };
        let encoded = tx.encode_rlp();
        let decoded = Eip1559Transaction::decode_rlp(&encoded[1..]).unwrap();
        assert_eq!(tx, decoded);
    }

    #[test]
    fn test_eip1559_tx_contract_creation() {
        let tx = Eip1559Transaction {
            chain_id: U256::from(1u64),
            nonce: U256::from(0u64),
            max_priority_fee_per_gas: U256::from(2_000_000_000u64),
            max_fee_per_gas: U256::from(20_000_000_000u64),
            gas_limit: U256::from(100000u64),
            to: None, // Contract creation
            value: U256::ZERO,
            data: Bytes::from_slice(&[0x60, 0x60, 0x60]),
            access_list: vec![],
            v: U256::from(0u64),
            r: U256::from(1u64),
            s: U256::from(1u64),
        };
        let encoded = tx.encode_rlp();
        let decoded = Eip1559Transaction::decode_rlp(&encoded[1..]).unwrap();
        assert_eq!(tx, decoded);
        assert_eq!(decoded.to, None);
    }

    #[test]
    fn test_eip1559_tx_hash() {
        let tx = Eip1559Transaction {
            chain_id: U256::from(1u64),
            nonce: U256::from(0u64),
            max_priority_fee_per_gas: U256::from(2_000_000_000u64),
            max_fee_per_gas: U256::from(20_000_000_000u64),
            gas_limit: U256::from(21000u64),
            to: Some(Address::ZERO),
            value: U256::from(1_000_000_000_000_000_000u64),
            data: Bytes::new(),
            access_list: vec![],
            v: U256::from(0u64),
            r: U256::ZERO,
            s: U256::ZERO,
        };
        let hash = tx.hash();
        assert_eq!(hash.as_bytes().len(), 32);
        assert_ne!(hash, Hash::ZERO);
    }

    #[test]
    fn test_eip1559_tx_signing_hash() {
        let tx = Eip1559Transaction {
            chain_id: U256::from(1u64),
            nonce: U256::from(0u64),
            max_priority_fee_per_gas: U256::from(2_000_000_000u64),
            max_fee_per_gas: U256::from(20_000_000_000u64),
            gas_limit: U256::from(21000u64),
            to: Some(Address::ZERO),
            value: U256::from(1_000_000_000_000_000_000u64),
            data: Bytes::new(),
            access_list: vec![],
            v: U256::from(0u64),
            r: U256::ZERO,
            s: U256::ZERO,
        };
        let signing_hash = tx.signing_hash();
        assert_eq!(signing_hash.as_bytes().len(), 32);
    }

    // =========================================================================
    // Transaction Enum Tests
    // =========================================================================

    #[test]
    fn test_transaction_enum_legacy() {
        let legacy = LegacyTransaction {
            nonce: U256::from(0u64),
            gas_price: U256::from(20_000_000_000u64),
            gas_limit: U256::from(21000u64),
            to: Some(Address::ZERO),
            value: U256::from(1_000_000_000_000_000_000u64),
            data: Bytes::new(),
            v: U256::from(27u64),
            r: U256::ZERO,
            s: U256::ZERO,
        };
        let tx = Transaction::Legacy(legacy.clone());
        let encoded = tx.encode_rlp();
        let decoded = Transaction::decode_rlp(&encoded).unwrap();
        assert_eq!(tx, decoded);
    }

    #[test]
    fn test_transaction_enum_eip2930() {
        let eip2930 = Eip2930Transaction {
            chain_id: U256::from(1u64),
            nonce: U256::from(0u64),
            gas_price: U256::from(20_000_000_000u64),
            gas_limit: U256::from(21000u64),
            to: Some(Address::ZERO),
            value: U256::from(1_000_000_000_000_000_000u64),
            data: Bytes::new(),
            access_list: vec![],
            v: U256::from(0u64),
            r: U256::ZERO,
            s: U256::ZERO,
        };
        let tx = Transaction::Eip2930(eip2930);
        let encoded = tx.encode_rlp();
        let decoded = Transaction::decode_rlp(&encoded).unwrap();
        assert_eq!(tx, decoded);
    }

    #[test]
    fn test_transaction_enum_eip1559() {
        let eip1559 = Eip1559Transaction {
            chain_id: U256::from(1u64),
            nonce: U256::from(0u64),
            max_priority_fee_per_gas: U256::from(2_000_000_000u64),
            max_fee_per_gas: U256::from(20_000_000_000u64),
            gas_limit: U256::from(21000u64),
            to: Some(Address::ZERO),
            value: U256::from(1_000_000_000_000_000_000u64),
            data: Bytes::new(),
            access_list: vec![],
            v: U256::from(0u64),
            r: U256::ZERO,
            s: U256::ZERO,
        };
        let tx = Transaction::Eip1559(eip1559);
        let encoded = tx.encode_rlp();
        let decoded = Transaction::decode_rlp(&encoded).unwrap();
        assert_eq!(tx, decoded);
    }

    #[test]
    fn test_transaction_decode_invalid_type() {
        let data = vec![0x03, 0xc0]; // Invalid type 0x03
        let result = Transaction::decode_rlp(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_transaction_decode_empty() {
        let result = Transaction::decode_rlp(&[]);
        assert_eq!(result, Err(RlpError::UnexpectedEnd));
    }

    #[test]
    fn test_transaction_hash_delegation() {
        let legacy = LegacyTransaction {
            nonce: U256::from(0u64),
            gas_price: U256::from(20_000_000_000u64),
            gas_limit: U256::from(21000u64),
            to: Some(Address::ZERO),
            value: U256::from(1_000_000_000_000_000_000u64),
            data: Bytes::new(),
            v: U256::from(27u64),
            r: U256::ZERO,
            s: U256::ZERO,
        };
        let tx = Transaction::Legacy(legacy.clone());
        assert_eq!(tx.hash(), legacy.hash());
    }

    #[test]
    fn test_transaction_display() {
        let legacy = LegacyTransaction {
            nonce: U256::from(0u64),
            gas_price: U256::from(20_000_000_000u64),
            gas_limit: U256::from(21000u64),
            to: Some(Address::ZERO),
            value: U256::from(1_000_000_000_000_000_000u64),
            data: Bytes::new(),
            v: U256::from(27u64),
            r: U256::ZERO,
            s: U256::ZERO,
        };
        let tx = Transaction::Legacy(legacy);
        let s = tx.to_string();
        assert!(s.contains("Legacy"));
    }

    // =========================================================================
    // Signature Recovery Tests
    // =========================================================================

    #[test]
    fn test_legacy_tx_recover_sender_invalid() {
        let tx = LegacyTransaction {
            nonce: U256::from(0u64),
            gas_price: U256::from(20_000_000_000u64),
            gas_limit: U256::from(21000u64),
            to: Some(Address::ZERO),
            value: U256::from(1_000_000_000_000_000_000u64),
            data: Bytes::new(),
            v: U256::from(27u64),
            r: U256::ZERO, // Invalid signature
            s: U256::ZERO,
        };
        let result = tx.recover_sender();
        assert!(result.is_err());
    }

    #[test]
    fn test_eip2930_tx_recover_sender_invalid() {
        let tx = Eip2930Transaction {
            chain_id: U256::from(1u64),
            nonce: U256::from(0u64),
            gas_price: U256::from(20_000_000_000u64),
            gas_limit: U256::from(21000u64),
            to: Some(Address::ZERO),
            value: U256::from(1_000_000_000_000_000_000u64),
            data: Bytes::new(),
            access_list: vec![],
            v: U256::from(0u64),
            r: U256::ZERO, // Invalid signature
            s: U256::ZERO,
        };
        let result = tx.recover_sender();
        assert!(result.is_err());
    }

    #[test]
    fn test_eip1559_tx_recover_sender_invalid() {
        let tx = Eip1559Transaction {
            chain_id: U256::from(1u64),
            nonce: U256::from(0u64),
            max_priority_fee_per_gas: U256::from(2_000_000_000u64),
            max_fee_per_gas: U256::from(20_000_000_000u64),
            gas_limit: U256::from(21000u64),
            to: Some(Address::ZERO),
            value: U256::from(1_000_000_000_000_000_000u64),
            data: Bytes::new(),
            access_list: vec![],
            v: U256::from(0u64),
            r: U256::ZERO, // Invalid signature
            s: U256::ZERO,
        };
        let result = tx.recover_sender();
        assert!(result.is_err());
    }

    // =========================================================================
    // Real Signature Tests
    // =========================================================================

    #[test]
    fn test_legacy_tx_sign_and_recover() {
        let signing_key = test_signing_key(1);
        let verifying_key = signing_key.verifying_key();

        // Create unsigned transaction
        let mut tx = LegacyTransaction {
            nonce: U256::from(0u64),
            gas_price: U256::from(20_000_000_000u64),
            gas_limit: U256::from(21000u64),
            to: Some(Address::from([0x42; 20])),
            value: U256::from(1_000_000_000_000_000_000u64),
            data: Bytes::new(),
            v: U256::from(27u64), // Pre-EIP-155
            r: U256::ZERO,
            s: U256::ZERO,
        };

        // Compute signing hash
        let signing_hash = tx.signing_hash();

        // Sign the transaction
        let (signature, recovery_id) = signing_key
            .sign_prehash_recoverable(signing_hash.as_bytes())
            .expect("Failed to sign");

        let sig_bytes = signature.to_bytes();

        // Set signature fields
        let mut r_bytes = [0u8; 32];
        r_bytes.copy_from_slice(&sig_bytes[..32]);
        tx.r = U256::from_be_bytes(r_bytes);

        let mut s_bytes = [0u8; 32];
        s_bytes.copy_from_slice(&sig_bytes[32..]);
        tx.s = U256::from_be_bytes(s_bytes);

        tx.v = U256::from(27u64 + recovery_id.to_byte() as u64);

        // Recover sender
        let recovered_address = tx.recover_sender().expect("Failed to recover");

        // Compute expected address
        let pk_encoded = verifying_key.to_encoded_point(false);
        let pk_bytes = pk_encoded.as_bytes();
        let pk_hash = keccak256(&pk_bytes[1..]);
        let mut expected_address_bytes = [0u8; 20];
        expected_address_bytes.copy_from_slice(&pk_hash.as_bytes()[12..]);
        let expected_address = Address::from(expected_address_bytes);

        assert_eq!(recovered_address, expected_address);
    }

    #[test]
    fn test_eip1559_tx_sign_and_recover() {
        let signing_key = test_signing_key(2);
        let verifying_key = signing_key.verifying_key();

        let mut tx = Eip1559Transaction {
            chain_id: U256::from(1u64),
            nonce: U256::from(0u64),
            max_priority_fee_per_gas: U256::from(2_000_000_000u64),
            max_fee_per_gas: U256::from(20_000_000_000u64),
            gas_limit: U256::from(21000u64),
            to: Some(Address::from([0x42; 20])),
            value: U256::from(1_000_000_000_000_000_000u64),
            data: Bytes::new(),
            access_list: vec![],
            v: U256::from(0u64),
            r: U256::ZERO,
            s: U256::ZERO,
        };

        let signing_hash = tx.signing_hash();

        let (signature, recovery_id) = signing_key
            .sign_prehash_recoverable(signing_hash.as_bytes())
            .expect("Failed to sign");

        let sig_bytes = signature.to_bytes();

        let mut r_bytes = [0u8; 32];
        r_bytes.copy_from_slice(&sig_bytes[..32]);
        tx.r = U256::from_be_bytes(r_bytes);

        let mut s_bytes = [0u8; 32];
        s_bytes.copy_from_slice(&sig_bytes[32..]);
        tx.s = U256::from_be_bytes(s_bytes);

        tx.v = U256::from(recovery_id.to_byte() as u64);

        let recovered_address = tx.recover_sender().expect("Failed to recover");

        let pk_encoded = verifying_key.to_encoded_point(false);
        let pk_bytes = pk_encoded.as_bytes();
        let pk_hash = keccak256(&pk_bytes[1..]);
        let mut expected_address_bytes = [0u8; 20];
        expected_address_bytes.copy_from_slice(&pk_hash.as_bytes()[12..]);
        let expected_address = Address::from(expected_address_bytes);

        assert_eq!(recovered_address, expected_address);
    }

    // =========================================================================
    // Edge Cases
    // =========================================================================

    #[test]
    fn test_decode_invalid_to_address_length() {
        // Manually craft invalid RLP with wrong 'to' address length
        let items = vec![
            rlp::encode_u256(&U256::ZERO),
            rlp::encode_u256(&U256::from(20_000_000_000u64)),
            rlp::encode_u256(&U256::from(21000u64)),
            rlp::encode_bytes(&[0x42; 19]), // Wrong length
            rlp::encode_u256(&U256::ZERO),
            rlp::encode_bytes(&[]),
            rlp::encode_u256(&U256::from(27u64)),
            rlp::encode_u256(&U256::ZERO),
            rlp::encode_u256(&U256::ZERO),
        ];
        let encoded = rlp::encode_list(&items);
        let result = LegacyTransaction::decode_rlp(&encoded);
        assert_eq!(result, Err(RlpError::InvalidLength));
    }

    #[test]
    fn test_transaction_signing_hash_differs_from_hash() {
        let tx = LegacyTransaction {
            nonce: U256::from(0u64),
            gas_price: U256::from(20_000_000_000u64),
            gas_limit: U256::from(21000u64),
            to: Some(Address::ZERO),
            value: U256::from(1_000_000_000_000_000_000u64),
            data: Bytes::new(),
            v: U256::from(27u64),
            r: U256::from(1u64),
            s: U256::from(2u64),
        };

        let hash = tx.hash();
        let signing_hash = tx.signing_hash();

        // They should be different (hash includes signature, signing_hash doesn't)
        assert_ne!(hash, signing_hash);
    }

    #[test]
    fn test_access_list_multiple_entries() {
        let access_list = vec![
            AccessListEntry {
                address: Address::from([0x11; 20]),
                storage_keys: vec![Hash::from([0x22; 32]), Hash::from([0x33; 32])],
            },
            AccessListEntry {
                address: Address::from([0x44; 20]),
                storage_keys: vec![],
            },
        ];

        let tx = Eip1559Transaction {
            chain_id: U256::from(1u64),
            nonce: U256::from(0u64),
            max_priority_fee_per_gas: U256::from(2_000_000_000u64),
            max_fee_per_gas: U256::from(20_000_000_000u64),
            gas_limit: U256::from(50000u64),
            to: Some(Address::from([0x42; 20])),
            value: U256::from(1_000_000_000_000_000_000u64),
            data: Bytes::new(),
            access_list,
            v: U256::from(0u64),
            r: U256::ZERO,
            s: U256::ZERO,
        };

        let encoded = tx.encode_rlp();
        let decoded = Eip1559Transaction::decode_rlp(&encoded[1..]).unwrap();
        assert_eq!(tx, decoded);
        assert_eq!(decoded.access_list.len(), 2);
    }

    fn test_signing_key(seed: u8) -> SigningKey {
        let mut key_bytes = [0u8; 32];
        key_bytes[31] = seed;
        SigningKey::from_bytes(&key_bytes.into()).expect("valid test signing key")
    }

    #[test]
    fn test_legacy_tx_decode_wrong_field_count() {
        let items = vec![
            rlp::encode_u256(&U256::ZERO),
            rlp::encode_u256(&U256::from(20_000_000_000u64)),
            // Missing fields
        ];
        let encoded = rlp::encode_list(&items);
        let result = LegacyTransaction::decode_rlp(&encoded);
        assert_eq!(result, Err(RlpError::InvalidEncoding));
    }

    #[test]
    fn test_eip2930_tx_decode_wrong_field_count() {
        let items = vec![
            rlp::encode_u256(&U256::from(1u64)),
            rlp::encode_u256(&U256::ZERO),
            // Missing fields
        ];
        let encoded = vec![0x01];
        let mut full = encoded.clone();
        full.extend(rlp::encode_list(&items));
        let result = Transaction::decode_rlp(&full);
        assert!(result.is_err());
    }

    #[test]
    fn test_all_transaction_types_different_hashes() {
        let legacy = LegacyTransaction {
            nonce: U256::from(0u64),
            gas_price: U256::from(20_000_000_000u64),
            gas_limit: U256::from(21000u64),
            to: Some(Address::ZERO),
            value: U256::from(1_000_000_000_000_000_000u64),
            data: Bytes::new(),
            v: U256::from(27u64),
            r: U256::ZERO,
            s: U256::ZERO,
        };

        let eip2930 = Eip2930Transaction {
            chain_id: U256::from(1u64),
            nonce: U256::from(0u64),
            gas_price: U256::from(20_000_000_000u64),
            gas_limit: U256::from(21000u64),
            to: Some(Address::ZERO),
            value: U256::from(1_000_000_000_000_000_000u64),
            data: Bytes::new(),
            access_list: vec![],
            v: U256::from(0u64),
            r: U256::ZERO,
            s: U256::ZERO,
        };

        let eip1559 = Eip1559Transaction {
            chain_id: U256::from(1u64),
            nonce: U256::from(0u64),
            max_priority_fee_per_gas: U256::from(2_000_000_000u64),
            max_fee_per_gas: U256::from(20_000_000_000u64),
            gas_limit: U256::from(21000u64),
            to: Some(Address::ZERO),
            value: U256::from(1_000_000_000_000_000_000u64),
            data: Bytes::new(),
            access_list: vec![],
            v: U256::from(0u64),
            r: U256::ZERO,
            s: U256::ZERO,
        };

        let hash1 = legacy.hash();
        let hash2 = eip2930.hash();
        let hash3 = eip1559.hash();

        // All three should be different
        assert_ne!(hash1, hash2);
        assert_ne!(hash2, hash3);
        assert_ne!(hash1, hash3);
    }
}
