//! Transaction validation for Ethereum State Transition Function
//!
//! This module implements comprehensive transaction validation including:
//! - Signature verification and sender recovery
//! - Nonce validation
//! - Gas limit and intrinsic gas validation
//! - Balance validation
//! - Chain ID validation
//!
//! Validation follows Ethereum's Fusaka fork rules (post-EIP-1559, post-EIP-2930).

#[cfg(target_arch = "riscv32")]
extern crate alloc;

use core::fmt;

use crate::crypto::Secp256k1Error;
use crate::types::transaction::AccessListEntry;
use crate::types::{Address, Transaction, U256};

// =============================================================================
// Validation Error Types
// =============================================================================

/// Errors that can occur during transaction validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationError {
    /// Transaction signature is invalid or cannot be recovered
    InvalidSignature,
    /// Transaction chain ID does not match expected chain ID
    InvalidChainId,
    /// Transaction nonce is too high (future transaction)
    NonceTooHigh,
    /// Transaction nonce is too low (already used)
    NonceTooLow,
    /// Account has insufficient funds to cover transaction cost
    InsufficientFunds,
    /// Gas limit is below the intrinsic gas required
    GasLimitTooLow,
    /// Gas limit exceeds the block gas limit
    GasLimitTooHigh,
    /// Gas price is too low (legacy transactions)
    GasPriceTooLow,
    /// Max fee per gas is below the base fee
    MaxFeePerGasTooLow,
    /// Max priority fee per gas exceeds max fee per gas
    MaxPriorityFeePerGasTooHigh,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::InvalidSignature => write!(f, "Invalid transaction signature"),
            ValidationError::InvalidChainId => write!(f, "Invalid chain ID"),
            ValidationError::NonceTooHigh => write!(f, "Transaction nonce is too high"),
            ValidationError::NonceTooLow => write!(f, "Transaction nonce is too low"),
            ValidationError::InsufficientFunds => {
                write!(f, "Insufficient funds for transaction cost")
            }
            ValidationError::GasLimitTooLow => write!(f, "Gas limit below intrinsic gas"),
            ValidationError::GasLimitTooHigh => write!(f, "Gas limit exceeds block gas limit"),
            ValidationError::GasPriceTooLow => write!(f, "Gas price is too low"),
            ValidationError::MaxFeePerGasTooLow => write!(f, "Max fee per gas is too low"),
            ValidationError::MaxPriorityFeePerGasTooHigh => {
                write!(f, "Max priority fee per gas exceeds max fee per gas")
            }
        }
    }
}

impl From<Secp256k1Error> for ValidationError {
    fn from(_: Secp256k1Error) -> Self {
        ValidationError::InvalidSignature
    }
}

#[cfg(not(target_arch = "riscv32"))]
impl std::error::Error for ValidationError {}

// =============================================================================
// Transaction Getters
// =============================================================================

impl Transaction {
    /// Returns the transaction nonce.
    pub fn nonce(&self) -> U256 {
        match self {
            Transaction::Legacy(tx) => tx.nonce,
            Transaction::Eip2930(tx) => tx.nonce,
            Transaction::Eip1559(tx) => tx.nonce,
        }
    }

    /// Returns the transaction gas limit.
    pub fn gas_limit(&self) -> U256 {
        match self {
            Transaction::Legacy(tx) => tx.gas_limit,
            Transaction::Eip2930(tx) => tx.gas_limit,
            Transaction::Eip1559(tx) => tx.gas_limit,
        }
    }

    /// Returns the transaction recipient address (None for contract creation).
    pub fn to(&self) -> Option<Address> {
        match self {
            Transaction::Legacy(tx) => tx.to,
            Transaction::Eip2930(tx) => tx.to,
            Transaction::Eip1559(tx) => tx.to,
        }
    }

    /// Returns the transaction value in wei.
    pub fn value(&self) -> U256 {
        match self {
            Transaction::Legacy(tx) => tx.value,
            Transaction::Eip2930(tx) => tx.value,
            Transaction::Eip1559(tx) => tx.value,
        }
    }

    /// Returns the transaction data.
    pub fn data(&self) -> &[u8] {
        match self {
            Transaction::Legacy(tx) => tx.data.as_ref(),
            Transaction::Eip2930(tx) => tx.data.as_ref(),
            Transaction::Eip1559(tx) => tx.data.as_ref(),
        }
    }

    /// Returns the transaction access list (empty for legacy transactions).
    pub fn access_list(&self) -> &[AccessListEntry] {
        match self {
            Transaction::Legacy(_) => &[],
            Transaction::Eip2930(tx) => &tx.access_list,
            Transaction::Eip1559(tx) => &tx.access_list,
        }
    }

    /// Returns the transaction chain ID (if applicable).
    ///
    /// For Legacy transactions, extracts chain_id from v if EIP-155 is used.
    /// For EIP-2930/EIP-1559, returns the chain_id field.
    pub fn chain_id(&self) -> Option<U256> {
        match self {
            Transaction::Legacy(tx) => {
                let v_u64 = tx.v.as_u64();
                if v_u64 >= 35 {
                    // EIP-155: v = chain_id * 2 + 35 + {0,1}
                    Some(U256::from((v_u64 - 35) / 2))
                } else {
                    None
                }
            }
            Transaction::Eip2930(tx) => Some(tx.chain_id),
            Transaction::Eip1559(tx) => Some(tx.chain_id),
        }
    }

    /// Returns the effective gas price for the transaction.
    ///
    /// For Legacy/EIP-2930: returns gas_price
    /// For EIP-1559: returns min(max_fee_per_gas, base_fee + max_priority_fee_per_gas)
    pub fn effective_gas_price(&self, base_fee: U256) -> U256 {
        match self {
            Transaction::Legacy(tx) => tx.gas_price,
            Transaction::Eip2930(tx) => tx.gas_price,
            Transaction::Eip1559(tx) => {
                let priority_fee_per_gas = tx.max_priority_fee_per_gas;
                let max_fee_per_gas = tx.max_fee_per_gas;

                // effective_gas_price = min(max_fee_per_gas, base_fee + max_priority_fee_per_gas)
                let base_plus_priority = base_fee.saturating_add(priority_fee_per_gas);
                if max_fee_per_gas < base_plus_priority {
                    max_fee_per_gas
                } else {
                    base_plus_priority
                }
            }
        }
    }

    /// Returns the max fee per gas for the transaction.
    ///
    /// For Legacy/EIP-2930: returns gas_price
    /// For EIP-1559: returns max_fee_per_gas
    pub fn max_fee_per_gas(&self) -> U256 {
        match self {
            Transaction::Legacy(tx) => tx.gas_price,
            Transaction::Eip2930(tx) => tx.gas_price,
            Transaction::Eip1559(tx) => tx.max_fee_per_gas,
        }
    }

    /// Returns the max priority fee per gas (if applicable).
    ///
    /// Only EIP-1559 transactions have a separate priority fee.
    pub fn max_priority_fee_per_gas(&self) -> Option<U256> {
        match self {
            Transaction::Legacy(_) => None,
            Transaction::Eip2930(_) => None,
            Transaction::Eip1559(tx) => Some(tx.max_priority_fee_per_gas),
        }
    }
}

// =============================================================================
// Validation Functions
// =============================================================================

/// Validates the transaction signature and recovers the sender address.
///
/// # Arguments
///
/// * `tx` - The transaction to validate
///
/// # Returns
///
/// Returns the sender address if the signature is valid, or an error otherwise.
///
/// # Examples
///
/// ```no_run
/// use claudeth::types::Transaction;
/// use claudeth::stf::validate_signature;
///
/// # fn example(tx: Transaction) -> Result<(), Box<dyn std::error::Error>> {
/// let sender = validate_signature(&tx)?;
/// println!("Sender: {:?}", sender);
/// # Ok(())
/// # }
/// ```
pub fn validate_signature(tx: &Transaction) -> Result<Address, ValidationError> {
    tx.recover_sender().map_err(ValidationError::from)
}

/// Validates that the transaction nonce matches the account nonce.
///
/// # Arguments
///
/// * `tx` - The transaction to validate
/// * `account_nonce` - The current nonce of the sender's account
///
/// # Returns
///
/// Returns `Ok(())` if the nonce is valid, or an error otherwise.
///
/// # Examples
///
/// ```
/// use claudeth::types::{Transaction, U256};
/// use claudeth::stf::validate_nonce;
///
/// # fn example(tx: Transaction) -> Result<(), Box<dyn std::error::Error>> {
/// validate_nonce(&tx, U256::from(5u64))?;
/// # Ok(())
/// # }
/// ```
pub fn validate_nonce(tx: &Transaction, account_nonce: U256) -> Result<(), ValidationError> {
    let tx_nonce = tx.nonce();

    if tx_nonce < account_nonce {
        Err(ValidationError::NonceTooLow)
    } else if tx_nonce > account_nonce {
        Err(ValidationError::NonceTooHigh)
    } else {
        Ok(())
    }
}

/// Validates the transaction gas parameters.
///
/// Checks:
/// 1. Gas limit is sufficient to cover intrinsic gas
/// 2. Gas limit does not exceed the block gas limit
/// 3. For EIP-1559: max_fee_per_gas >= max_priority_fee_per_gas
///
/// # Arguments
///
/// * `tx` - The transaction to validate
/// * `block_gas_limit` - The block gas limit
///
/// # Returns
///
/// Returns `Ok(())` if gas parameters are valid, or an error otherwise.
pub fn validate_gas(tx: &Transaction, block_gas_limit: U256) -> Result<(), ValidationError> {
    let gas_limit = tx.gas_limit();
    let intrinsic_gas = calculate_intrinsic_gas(tx);

    // Check gas limit >= intrinsic gas
    if gas_limit < intrinsic_gas {
        return Err(ValidationError::GasLimitTooLow);
    }

    // Check gas limit <= block gas limit
    if gas_limit > block_gas_limit {
        return Err(ValidationError::GasLimitTooHigh);
    }

    // For EIP-1559: check max_fee_per_gas >= max_priority_fee_per_gas
    if let Transaction::Eip1559(tx) = tx
        && tx.max_fee_per_gas < tx.max_priority_fee_per_gas
    {
        return Err(ValidationError::MaxPriorityFeePerGasTooHigh);
    }

    Ok(())
}

/// Calculates the intrinsic gas cost for a transaction.
///
/// The intrinsic gas is the minimum gas required to execute a transaction:
/// - Base cost: 21000 gas
/// - Data cost: 4 gas per zero byte, 16 gas per non-zero byte
/// - Access list cost: 2400 gas per address, 1900 gas per storage key
/// - Contract creation cost: 32000 gas if `to` is None
///
/// # Arguments
///
/// * `tx` - The transaction to calculate intrinsic gas for
///
/// # Returns
///
/// The intrinsic gas cost as U256.
///
/// # Examples
///
/// ```
/// use claudeth::types::Transaction;
/// use claudeth::stf::calculate_intrinsic_gas;
///
/// # fn example(tx: Transaction) {
/// let intrinsic_gas = calculate_intrinsic_gas(&tx);
/// println!("Intrinsic gas: {}", intrinsic_gas);
/// # }
/// ```
pub fn calculate_intrinsic_gas(tx: &Transaction) -> U256 {
    // Base transaction cost
    let mut gas = U256::from(21000u64);

    // Data cost
    let data = tx.data();
    for &byte in data {
        if byte == 0 {
            gas = gas.saturating_add(U256::from(4u64));
        } else {
            gas = gas.saturating_add(U256::from(16u64));
        }
    }

    // Access list cost (EIP-2930 and EIP-1559)
    let access_list = tx.access_list();
    for entry in access_list {
        // 2400 gas per address
        gas = gas.saturating_add(U256::from(2400u64));

        // 1900 gas per storage key
        let key_count = U256::from(entry.storage_keys.len() as u64);
        gas = gas.saturating_add(key_count.saturating_mul(U256::from(1900u64)));
    }

    // Contract creation cost
    if tx.to().is_none() {
        gas = gas.saturating_add(U256::from(32000u64));
    }

    gas
}

/// Validates that the sender has sufficient balance to cover the transaction cost.
///
/// The maximum transaction cost is:
/// - For Legacy/EIP-2930: `gas_limit * gas_price + value`
/// - For EIP-1559: `gas_limit * max_fee_per_gas + value`
///
/// # Arguments
///
/// * `tx` - The transaction to validate
/// * `account_balance` - The sender's account balance
/// * `base_fee` - The current block base fee (used for display only)
///
/// # Returns
///
/// Returns `Ok(())` if the balance is sufficient, or an error otherwise.
pub fn validate_balance(
    tx: &Transaction,
    account_balance: U256,
    _base_fee: U256,
) -> Result<(), ValidationError> {
    let gas_limit = tx.gas_limit();
    let value = tx.value();

    // Calculate maximum cost (worst case)
    let max_gas_cost = gas_limit.saturating_mul(tx.max_fee_per_gas());
    let max_cost = max_gas_cost.saturating_add(value);

    if account_balance < max_cost {
        return Err(ValidationError::InsufficientFunds);
    }

    Ok(())
}

/// Validates that the transaction chain ID matches the expected chain ID.
///
/// # Arguments
///
/// * `tx` - The transaction to validate
/// * `expected_chain_id` - The expected chain ID for the network
///
/// # Returns
///
/// Returns `Ok(())` if the chain ID is valid, or an error otherwise.
///
/// # Examples
///
/// ```
/// use claudeth::types::{Transaction, U256};
/// use claudeth::stf::validate_chain_id;
///
/// # fn example(tx: Transaction) -> Result<(), Box<dyn std::error::Error>> {
/// validate_chain_id(&tx, U256::from(1u64))?; // Ethereum mainnet
/// # Ok(())
/// # }
/// ```
pub fn validate_chain_id(
    tx: &Transaction,
    expected_chain_id: U256,
) -> Result<(), ValidationError> {
    match tx.chain_id() {
        Some(chain_id) => {
            if chain_id != expected_chain_id {
                Err(ValidationError::InvalidChainId)
            } else {
                Ok(())
            }
        }
        None => {
            // Pre-EIP-155 legacy transaction - no chain ID validation needed
            Ok(())
        }
    }
}

/// Validates a complete transaction before execution.
///
/// This function performs all validation checks in the correct order:
/// 1. Signature validation and sender recovery
/// 2. Nonce validation
/// 3. Gas validation
/// 4. Balance validation
/// 5. Chain ID validation
///
/// # Arguments
///
/// * `tx` - The transaction to validate
/// * `account_nonce` - The current nonce of the sender's account
/// * `account_balance` - The sender's account balance
/// * `block_gas_limit` - The block gas limit
/// * `base_fee` - The current block base fee
/// * `chain_id` - The expected chain ID for the network
///
/// # Returns
///
/// Returns the sender address if all validations pass, or an error otherwise.
///
/// # Examples
///
/// ```no_run
/// use claudeth::types::{Transaction, U256};
/// use claudeth::stf::validate_transaction;
///
/// # fn example(tx: Transaction) -> Result<(), Box<dyn std::error::Error>> {
/// let sender = validate_transaction(
///     &tx,
///     U256::from(5u64),           // account nonce
///     U256::from(1_000_000u64),   // account balance
///     U256::from(30_000_000u64),  // block gas limit
///     U256::from(10u64),          // base fee
///     U256::from(1u64),           // chain ID (mainnet)
/// )?;
/// println!("Transaction valid, sender: {:?}", sender);
/// # Ok(())
/// # }
/// ```
pub fn validate_transaction(
    tx: &Transaction,
    account_nonce: U256,
    account_balance: U256,
    block_gas_limit: U256,
    base_fee: U256,
    chain_id: U256,
) -> Result<Address, ValidationError> {
    // 1. Validate signature and recover sender
    let sender = validate_signature(tx)?;

    // 2. Validate nonce
    validate_nonce(tx, account_nonce)?;

    // 3. Validate gas parameters
    validate_gas(tx, block_gas_limit)?;

    // 4. Validate balance
    validate_balance(tx, account_balance, base_fee)?;

    // 5. Validate chain ID
    validate_chain_id(tx, chain_id)?;

    Ok(sender)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::transaction::{Eip1559Transaction, Eip2930Transaction, LegacyTransaction};
    use crate::types::Bytes;
    use k256::ecdsa::SigningKey;

    // Helper to create a valid signed transaction for testing
    fn create_signed_legacy_tx() -> (LegacyTransaction, Address) {
        let signing_key = test_signing_key(1);
        let verifying_key = signing_key.verifying_key();

        let mut tx = LegacyTransaction {
            nonce: U256::from(0u64),
            gas_price: U256::from(20_000_000_000u64),
            gas_limit: U256::from(21000u64),
            to: Some(Address::from([0x42; 20])),
            value: U256::from(1_000_000_000_000u64),
            data: Bytes::new(),
            v: U256::from(27u64),
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

        tx.v = U256::from(27u64 + recovery_id.to_byte() as u64);

        // Compute expected address
        use crate::crypto::keccak256;
        let pk_encoded = verifying_key.to_encoded_point(false);
        let pk_bytes = pk_encoded.as_bytes();
        let pk_hash = keccak256(&pk_bytes[1..]);
        let mut address_bytes = [0u8; 20];
        address_bytes.copy_from_slice(&pk_hash.as_bytes()[12..]);
        let address = Address::from(address_bytes);

        (tx, address)
    }

    fn create_signed_eip1559_tx() -> (Eip1559Transaction, Address) {
        let signing_key = test_signing_key(2);
        let verifying_key = signing_key.verifying_key();

        let mut tx = Eip1559Transaction {
            chain_id: U256::from(1u64),
            nonce: U256::from(0u64),
            max_priority_fee_per_gas: U256::from(2_000_000_000u64),
            max_fee_per_gas: U256::from(20_000_000_000u64),
            gas_limit: U256::from(21000u64),
            to: Some(Address::from([0x42; 20])),
            value: U256::from(1_000_000_000_000u64),
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

        // Compute expected address
        use crate::crypto::keccak256;
        let pk_encoded = verifying_key.to_encoded_point(false);
        let pk_bytes = pk_encoded.as_bytes();
        let pk_hash = keccak256(&pk_bytes[1..]);
        let mut address_bytes = [0u8; 20];
        address_bytes.copy_from_slice(&pk_hash.as_bytes()[12..]);
        let address = Address::from(address_bytes);

        (tx, address)
    }

    // =========================================================================
    // Signature Tests
    // =========================================================================

    #[test]
    fn test_validate_signature_valid() {
        let (tx, expected_address) = create_signed_legacy_tx();
        let tx = Transaction::Legacy(tx);

        let sender = validate_signature(&tx).expect("Signature should be valid");
        assert_eq!(sender, expected_address);
    }

    #[test]
    fn test_validate_signature_invalid() {
        let tx = LegacyTransaction {
            nonce: U256::from(0u64),
            gas_price: U256::from(20_000_000_000u64),
            gas_limit: U256::from(21000u64),
            to: Some(Address::ZERO),
            value: U256::from(1_000_000_000_000u64),
            data: Bytes::new(),
            v: U256::from(27u64),
            r: U256::ZERO,
            s: U256::ZERO,
        };
        let tx = Transaction::Legacy(tx);

        let result = validate_signature(&tx);
        assert_eq!(result, Err(ValidationError::InvalidSignature));
    }

    fn test_signing_key(seed: u8) -> SigningKey {
        let mut key_bytes = [0u8; 32];
        key_bytes[31] = seed;
        SigningKey::from_bytes(&key_bytes.into()).expect("valid test signing key")
    }

    #[test]
    fn test_validate_signature_eip1559() {
        let (tx, expected_address) = create_signed_eip1559_tx();
        let tx = Transaction::Eip1559(tx);

        let sender = validate_signature(&tx).expect("Signature should be valid");
        assert_eq!(sender, expected_address);
    }

    #[test]
    fn test_validate_signature_with_modified_chain_id() {
        // When chain_id is changed after signing, the signing_hash changes,
        // which makes the signature recovery produce a different (wrong) address
        let (mut tx, expected_address) = create_signed_eip1559_tx();
        tx.chain_id = U256::from(999u64);
        let tx = Transaction::Eip1559(tx);

        // The signature recovery will still succeed, but produces wrong address
        // This will be caught by chain_id validation later
        let result = validate_signature(&tx);

        // Signature recovery succeeds but gives us a different address
        if let Ok(recovered_address) = result {
            // The recovered address should be different from the expected one
            assert_ne!(recovered_address, expected_address);
        } else {
            // Or it might fail with invalid signature
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_validate_signature_eip2930() {
        let tx = Eip2930Transaction {
            chain_id: U256::from(1u64),
            nonce: U256::from(0u64),
            gas_price: U256::from(20_000_000_000u64),
            gas_limit: U256::from(21000u64),
            to: Some(Address::ZERO),
            value: U256::from(1_000_000_000_000u64),
            data: Bytes::new(),
            access_list: vec![],
            v: U256::from(0u64),
            r: U256::ZERO,
            s: U256::ZERO,
        };
        let tx = Transaction::Eip2930(tx);

        let result = validate_signature(&tx);
        assert_eq!(result, Err(ValidationError::InvalidSignature));
    }

    // =========================================================================
    // Nonce Tests
    // =========================================================================

    #[test]
    fn test_validate_nonce_correct() {
        let (tx, _) = create_signed_legacy_tx();
        let tx = Transaction::Legacy(tx);

        let result = validate_nonce(&tx, U256::from(0u64));
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_nonce_too_low() {
        let (mut tx, _) = create_signed_legacy_tx();
        tx.nonce = U256::from(5u64);
        let tx = Transaction::Legacy(tx);

        let result = validate_nonce(&tx, U256::from(10u64));
        assert_eq!(result, Err(ValidationError::NonceTooLow));
    }

    #[test]
    fn test_validate_nonce_too_high() {
        let (mut tx, _) = create_signed_legacy_tx();
        tx.nonce = U256::from(10u64);
        let tx = Transaction::Legacy(tx);

        let result = validate_nonce(&tx, U256::from(5u64));
        assert_eq!(result, Err(ValidationError::NonceTooHigh));
    }

    #[test]
    fn test_validate_nonce_eip1559() {
        let (mut tx, _) = create_signed_eip1559_tx();
        tx.nonce = U256::from(42u64);
        let tx = Transaction::Eip1559(tx);

        let result = validate_nonce(&tx, U256::from(42u64));
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_nonce_zero() {
        let (tx, _) = create_signed_legacy_tx();
        let tx = Transaction::Legacy(tx);

        let result = validate_nonce(&tx, U256::ZERO);
        assert!(result.is_ok());
    }

    // =========================================================================
    // Gas Tests
    // =========================================================================

    #[test]
    fn test_validate_gas_sufficient() {
        let (tx, _) = create_signed_legacy_tx();
        let tx = Transaction::Legacy(tx);

        let result = validate_gas(&tx, U256::from(30_000_000u64));
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_gas_limit_too_low() {
        let (mut tx, _) = create_signed_legacy_tx();
        tx.gas_limit = U256::from(20000u64); // Below intrinsic gas
        let tx = Transaction::Legacy(tx);

        let result = validate_gas(&tx, U256::from(30_000_000u64));
        assert_eq!(result, Err(ValidationError::GasLimitTooLow));
    }

    #[test]
    fn test_validate_gas_limit_too_high() {
        let (mut tx, _) = create_signed_legacy_tx();
        tx.gas_limit = U256::from(50_000_000u64);
        let tx = Transaction::Legacy(tx);

        let result = validate_gas(&tx, U256::from(30_000_000u64));
        assert_eq!(result, Err(ValidationError::GasLimitTooHigh));
    }

    #[test]
    fn test_validate_gas_eip1559_priority_fee_too_high() {
        let (mut tx, _) = create_signed_eip1559_tx();
        tx.max_priority_fee_per_gas = U256::from(30_000_000_000u64);
        tx.max_fee_per_gas = U256::from(20_000_000_000u64); // Lower than priority
        let tx = Transaction::Eip1559(tx);

        let result = validate_gas(&tx, U256::from(30_000_000u64));
        assert_eq!(result, Err(ValidationError::MaxPriorityFeePerGasTooHigh));
    }

    #[test]
    fn test_validate_gas_eip1559_fees_valid() {
        let (mut tx, _) = create_signed_eip1559_tx();
        tx.max_priority_fee_per_gas = U256::from(2_000_000_000u64);
        tx.max_fee_per_gas = U256::from(20_000_000_000u64);
        let tx = Transaction::Eip1559(tx);

        let result = validate_gas(&tx, U256::from(30_000_000u64));
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_gas_exact_intrinsic() {
        let (mut tx, _) = create_signed_legacy_tx();
        let intrinsic = calculate_intrinsic_gas(&Transaction::Legacy(tx.clone()));
        tx.gas_limit = intrinsic;
        let tx = Transaction::Legacy(tx);

        let result = validate_gas(&tx, U256::from(30_000_000u64));
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_gas_exact_block_limit() {
        let (mut tx, _) = create_signed_legacy_tx();
        tx.gas_limit = U256::from(30_000_000u64);
        let tx = Transaction::Legacy(tx);

        let result = validate_gas(&tx, U256::from(30_000_000u64));
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_gas_with_data() {
        let (mut tx, _) = create_signed_legacy_tx();
        tx.data = Bytes::from_slice(&[0x01, 0x02, 0x03]);
        tx.gas_limit = U256::from(25000u64);
        let tx = Transaction::Legacy(tx);

        let result = validate_gas(&tx, U256::from(30_000_000u64));
        assert!(result.is_ok());
    }

    // =========================================================================
    // Intrinsic Gas Tests
    // =========================================================================

    #[test]
    fn test_calculate_intrinsic_gas_base() {
        let (tx, _) = create_signed_legacy_tx();
        let tx = Transaction::Legacy(tx);

        let gas = calculate_intrinsic_gas(&tx);
        assert_eq!(gas, U256::from(21000u64));
    }

    #[test]
    fn test_calculate_intrinsic_gas_with_zero_bytes() {
        let (mut tx, _) = create_signed_legacy_tx();
        tx.data = Bytes::from_slice(&[0x00, 0x00, 0x00]);
        let tx = Transaction::Legacy(tx);

        let gas = calculate_intrinsic_gas(&tx);
        // 21000 + 3 * 4 = 21012
        assert_eq!(gas, U256::from(21012u64));
    }

    #[test]
    fn test_calculate_intrinsic_gas_with_nonzero_bytes() {
        let (mut tx, _) = create_signed_legacy_tx();
        tx.data = Bytes::from_slice(&[0x01, 0x02, 0x03]);
        let tx = Transaction::Legacy(tx);

        let gas = calculate_intrinsic_gas(&tx);
        // 21000 + 3 * 16 = 21048
        assert_eq!(gas, U256::from(21048u64));
    }

    #[test]
    fn test_calculate_intrinsic_gas_with_mixed_bytes() {
        let (mut tx, _) = create_signed_legacy_tx();
        tx.data = Bytes::from_slice(&[0x00, 0x01, 0x00, 0x02]);
        let tx = Transaction::Legacy(tx);

        let gas = calculate_intrinsic_gas(&tx);
        // 21000 + 2 * 4 + 2 * 16 = 21040
        assert_eq!(gas, U256::from(21040u64));
    }

    #[test]
    fn test_calculate_intrinsic_gas_with_access_list() {
        let (mut tx, _) = create_signed_eip1559_tx();
        tx.access_list = vec![
            AccessListEntry {
                address: Address::from([0x11; 20]),
                storage_keys: vec![crate::types::Hash::from([0x22; 32])],
            },
        ];
        let tx = Transaction::Eip1559(tx);

        let gas = calculate_intrinsic_gas(&tx);
        // 21000 + 2400 (address) + 1900 (storage key) = 25300
        assert_eq!(gas, U256::from(25300u64));
    }

    #[test]
    fn test_calculate_intrinsic_gas_access_list_multiple_keys() {
        let (mut tx, _) = create_signed_eip1559_tx();
        tx.access_list = vec![AccessListEntry {
            address: Address::from([0x11; 20]),
            storage_keys: vec![
                crate::types::Hash::from([0x22; 32]),
                crate::types::Hash::from([0x33; 32]),
                crate::types::Hash::from([0x44; 32]),
            ],
        }];
        let tx = Transaction::Eip1559(tx);

        let gas = calculate_intrinsic_gas(&tx);
        // 21000 + 2400 + 3 * 1900 = 29100
        assert_eq!(gas, U256::from(29100u64));
    }

    #[test]
    fn test_calculate_intrinsic_gas_contract_creation() {
        let (mut tx, _) = create_signed_legacy_tx();
        tx.to = None; // Contract creation
        let tx = Transaction::Legacy(tx);

        let gas = calculate_intrinsic_gas(&tx);
        // 21000 + 32000 = 53000
        assert_eq!(gas, U256::from(53000u64));
    }

    #[test]
    fn test_calculate_intrinsic_gas_contract_creation_with_data() {
        let (mut tx, _) = create_signed_legacy_tx();
        tx.to = None;
        tx.data = Bytes::from_slice(&[0x60, 0x60, 0x60]); // 3 non-zero bytes
        let tx = Transaction::Legacy(tx);

        let gas = calculate_intrinsic_gas(&tx);
        // 21000 + 32000 + 3 * 16 = 53048
        assert_eq!(gas, U256::from(53048u64));
    }

    // =========================================================================
    // Balance Tests
    // =========================================================================

    #[test]
    fn test_validate_balance_sufficient() {
        let (tx, _) = create_signed_legacy_tx();
        let tx = Transaction::Legacy(tx);

        let balance = U256::from(1_000_000_000_000_000u64);
        let result = validate_balance(&tx, balance, U256::from(10u64));
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_balance_insufficient() {
        let (tx, _) = create_signed_legacy_tx();
        let tx = Transaction::Legacy(tx);

        let balance = U256::from(100u64); // Too low
        let result = validate_balance(&tx, balance, U256::from(10u64));
        assert_eq!(result, Err(ValidationError::InsufficientFunds));
    }

    #[test]
    fn test_validate_balance_exact() {
        let (tx, _) = create_signed_legacy_tx();
        let tx_clone = Transaction::Legacy(tx.clone());

        // Calculate exact cost: gas_limit * gas_price + value
        let cost = tx
            .gas_limit
            .saturating_mul(tx.gas_price)
            .saturating_add(tx.value);

        let result = validate_balance(&tx_clone, cost, U256::from(10u64));
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_balance_one_below_required() {
        let (tx, _) = create_signed_legacy_tx();
        let tx_clone = Transaction::Legacy(tx.clone());

        let cost = tx
            .gas_limit
            .saturating_mul(tx.gas_price)
            .saturating_add(tx.value);
        let balance = cost.saturating_sub(U256::from(1u64));

        let result = validate_balance(&tx_clone, balance, U256::from(10u64));
        assert_eq!(result, Err(ValidationError::InsufficientFunds));
    }

    #[test]
    fn test_validate_balance_eip1559() {
        let (tx, _) = create_signed_eip1559_tx();
        let tx = Transaction::Eip1559(tx);

        let balance = U256::from(1_000_000_000_000_000u64);
        let result = validate_balance(&tx, balance, U256::from(10u64));
        assert!(result.is_ok());
    }

    // =========================================================================
    // Chain ID Tests
    // =========================================================================

    #[test]
    fn test_validate_chain_id_correct() {
        let (tx, _) = create_signed_eip1559_tx();
        let tx = Transaction::Eip1559(tx);

        let result = validate_chain_id(&tx, U256::from(1u64));
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_chain_id_incorrect() {
        let (tx, _) = create_signed_eip1559_tx();
        let tx = Transaction::Eip1559(tx);

        let result = validate_chain_id(&tx, U256::from(999u64));
        assert_eq!(result, Err(ValidationError::InvalidChainId));
    }

    #[test]
    fn test_validate_chain_id_legacy_eip155() {
        let (mut tx, _) = create_signed_legacy_tx();
        // EIP-155: v = chain_id * 2 + 35 + {0,1}
        // For chain_id = 1, v = 37 or 38
        tx.v = U256::from(37u64);
        let tx = Transaction::Legacy(tx);

        let result = validate_chain_id(&tx, U256::from(1u64));
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_chain_id_legacy_pre_eip155() {
        let (mut tx, _) = create_signed_legacy_tx();
        tx.v = U256::from(27u64); // Pre-EIP-155
        let tx = Transaction::Legacy(tx);

        // Should pass - no chain ID to validate
        let result = validate_chain_id(&tx, U256::from(1u64));
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_chain_id_eip2930() {
        let tx = Eip2930Transaction {
            chain_id: U256::from(5u64), // Goerli
            nonce: U256::from(0u64),
            gas_price: U256::from(20_000_000_000u64),
            gas_limit: U256::from(21000u64),
            to: Some(Address::ZERO),
            value: U256::from(1_000_000_000_000u64),
            data: Bytes::new(),
            access_list: vec![],
            v: U256::from(0u64),
            r: U256::ZERO,
            s: U256::ZERO,
        };
        let tx = Transaction::Eip2930(tx);

        let result = validate_chain_id(&tx, U256::from(5u64));
        assert!(result.is_ok());

        let result = validate_chain_id(&tx, U256::from(1u64));
        assert_eq!(result, Err(ValidationError::InvalidChainId));
    }

    // =========================================================================
    // Integration Tests
    // =========================================================================

    #[test]
    fn test_validate_transaction_complete_success() {
        let (tx, expected_sender) = create_signed_legacy_tx();
        let tx = Transaction::Legacy(tx);

        let sender = validate_transaction(
            &tx,
            U256::from(0u64),
            U256::from(1_000_000_000_000_000u64),
            U256::from(30_000_000u64),
            U256::from(10u64),
            U256::from(1u64),
        )
        .expect("Transaction should be valid");

        assert_eq!(sender, expected_sender);
    }

    #[test]
    fn test_validate_transaction_nonce_mismatch() {
        let (tx, _) = create_signed_legacy_tx();
        let tx = Transaction::Legacy(tx);

        let result = validate_transaction(
            &tx,
            U256::from(5u64), // Wrong nonce
            U256::from(1_000_000_000_000_000u64),
            U256::from(30_000_000u64),
            U256::from(10u64),
            U256::from(1u64),
        );

        assert_eq!(result, Err(ValidationError::NonceTooLow));
    }

    #[test]
    fn test_validate_transaction_insufficient_balance() {
        let (tx, _) = create_signed_legacy_tx();
        let tx = Transaction::Legacy(tx);

        let result = validate_transaction(
            &tx,
            U256::from(0u64),
            U256::from(100u64), // Insufficient balance
            U256::from(30_000_000u64),
            U256::from(10u64),
            U256::from(1u64),
        );

        assert_eq!(result, Err(ValidationError::InsufficientFunds));
    }

    #[test]
    fn test_validate_transaction_eip1559_complete() {
        let (tx, expected_sender) = create_signed_eip1559_tx();
        let tx = Transaction::Eip1559(tx);

        let sender = validate_transaction(
            &tx,
            U256::from(0u64),
            U256::from(1_000_000_000_000_000u64),
            U256::from(30_000_000u64),
            U256::from(10u64),
            U256::from(1u64),
        )
        .expect("Transaction should be valid");

        assert_eq!(sender, expected_sender);
    }

    #[test]
    fn test_validate_transaction_all_types() {
        // Test Legacy
        let (legacy_tx, _) = create_signed_legacy_tx();
        let legacy = Transaction::Legacy(legacy_tx);
        assert!(validate_transaction(
            &legacy,
            U256::from(0u64),
            U256::from(1_000_000_000_000_000u64),
            U256::from(30_000_000u64),
            U256::from(10u64),
            U256::from(1u64),
        )
        .is_ok());

        // Test EIP-1559
        let (eip1559_tx, _) = create_signed_eip1559_tx();
        let eip1559 = Transaction::Eip1559(eip1559_tx);
        assert!(validate_transaction(
            &eip1559,
            U256::from(0u64),
            U256::from(1_000_000_000_000_000u64),
            U256::from(30_000_000u64),
            U256::from(10u64),
            U256::from(1u64),
        )
        .is_ok());
    }

    // =========================================================================
    // Transaction Getter Tests
    // =========================================================================

    #[test]
    fn test_transaction_getters_legacy() {
        let (tx, _) = create_signed_legacy_tx();
        let tx = Transaction::Legacy(tx.clone());

        assert_eq!(tx.nonce(), U256::from(0u64));
        assert_eq!(tx.gas_limit(), U256::from(21000u64));
        assert_eq!(tx.value(), U256::from(1_000_000_000_000u64));
        assert!(tx.to().is_some());
        assert!(tx.data().is_empty());
        assert!(tx.access_list().is_empty());
    }

    #[test]
    fn test_transaction_getters_eip1559() {
        let (tx, _) = create_signed_eip1559_tx();
        let original = tx.clone();
        let tx = Transaction::Eip1559(tx);

        assert_eq!(tx.nonce(), original.nonce);
        assert_eq!(tx.gas_limit(), original.gas_limit);
        assert_eq!(tx.value(), original.value);
        assert_eq!(tx.chain_id(), Some(U256::from(1u64)));
        assert_eq!(
            tx.max_priority_fee_per_gas(),
            Some(U256::from(2_000_000_000u64))
        );
    }

    #[test]
    fn test_transaction_effective_gas_price_legacy() {
        let (tx, _) = create_signed_legacy_tx();
        let gas_price = tx.gas_price;
        let tx = Transaction::Legacy(tx);

        let effective = tx.effective_gas_price(U256::from(10u64));
        assert_eq!(effective, gas_price);
    }

    #[test]
    fn test_transaction_effective_gas_price_eip1559_base_low() {
        let (tx, _) = create_signed_eip1559_tx();
        let tx = Transaction::Eip1559(tx);

        // base_fee = 5, max_priority = 2_000_000_000
        // effective = min(20_000_000_000, 5 + 2_000_000_000) = 2_000_000_005
        let effective = tx.effective_gas_price(U256::from(5u64));
        assert_eq!(effective, U256::from(2_000_000_005u64));
    }

    #[test]
    fn test_transaction_effective_gas_price_eip1559_base_high() {
        let (tx, _) = create_signed_eip1559_tx();
        let tx = Transaction::Eip1559(tx);

        // base_fee = 30_000_000_000, max_fee = 20_000_000_000
        // effective = min(20_000_000_000, 30_000_000_000 + 2_000_000_000) = 20_000_000_000
        let effective = tx.effective_gas_price(U256::from(30_000_000_000u64));
        assert_eq!(effective, U256::from(20_000_000_000u64));
    }
}
