//! EVM Stack implementation
//!
//! This module provides a bounded stack for the Ethereum Virtual Machine.
//! The EVM stack has a maximum size of 1024 items, each of which is a 256-bit unsigned integer.
//!
//! ## Specification
//!
//! - Maximum stack size: 1024 items
//! - Each item is a U256 value
//! - Operations: push, pop, peek, swap, dup
//! - All operations check for overflow and underflow

#[cfg(target_arch = "riscv32")]
extern crate alloc;

#[cfg(not(target_arch = "riscv32"))]
use std::vec::Vec;

#[cfg(target_arch = "riscv32")]
use alloc::vec::Vec;

use core::fmt;

use crate::types::U256;

// =============================================================================
// Constants
// =============================================================================

/// Maximum stack size (1024 items as per EVM specification)
pub const MAX_STACK_SIZE: usize = 1024;

// =============================================================================
// Error Types
// =============================================================================

/// Errors that can occur during stack operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackError {
    /// Stack overflow (attempted to push to a full stack)
    Overflow,
    /// Stack underflow (attempted to pop from an empty stack)
    Underflow,
    /// Invalid index (attempted to access an out-of-bounds position)
    InvalidIndex,
}

impl fmt::Display for StackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StackError::Overflow => write!(f, "Stack overflow: maximum size of {MAX_STACK_SIZE} items reached"),
            StackError::Underflow => write!(f, "Stack underflow: attempted to pop from empty stack"),
            StackError::InvalidIndex => write!(f, "Invalid index: position is out of bounds"),
        }
    }
}

// =============================================================================
// Stack Implementation
// =============================================================================

/// EVM stack with bounded size
///
/// The stack stores U256 values and enforces a maximum size of 1024 items.
/// All operations return Result types to handle overflow/underflow conditions.
#[derive(Debug, Clone)]
pub struct Stack {
    /// Stack items (top of stack is at the end of the vector)
    items: Vec<U256>,
}

impl Stack {
    /// Create a new empty stack
    pub fn new() -> Self {
        Stack {
            items: Vec::new(),
        }
    }

    /// Push a value onto the stack
    ///
    /// Returns an error if the stack is full (1024 items).
    pub fn push(&mut self, value: U256) -> Result<(), StackError> {
        if self.items.len() >= MAX_STACK_SIZE {
            return Err(StackError::Overflow);
        }
        self.items.push(value);
        Ok(())
    }

    /// Pop a value from the stack
    ///
    /// Returns an error if the stack is empty.
    pub fn pop(&mut self) -> Result<U256, StackError> {
        self.items.pop().ok_or(StackError::Underflow)
    }

    /// Peek at a value on the stack without removing it
    ///
    /// Index 0 is the top of the stack, index 1 is one below the top, etc.
    /// Returns an error if the index is out of bounds.
    pub fn peek(&self, index: usize) -> Result<&U256, StackError> {
        if index >= self.items.len() {
            return Err(StackError::InvalidIndex);
        }
        // Top of stack is at the end, so we need to index from the back
        Ok(&self.items[self.items.len() - 1 - index])
    }

    /// Swap the top stack item with the n-th item
    ///
    /// For EVM compatibility:
    /// - n=1 swaps top with second item (SWAP1)
    /// - n=2 swaps top with third item (SWAP2)
    /// - etc.
    ///
    /// Returns an error if n is 0 or if there aren't enough items on the stack.
    pub fn swap(&mut self, n: usize) -> Result<(), StackError> {
        if n == 0 {
            return Err(StackError::InvalidIndex);
        }
        let len = self.items.len();
        if n >= len {
            return Err(StackError::Underflow);
        }
        // Swap top (len - 1) with n-th item (len - 1 - n)
        self.items.swap(len - 1, len - 1 - n);
        Ok(())
    }

    /// Duplicate the n-th item and push it to the top
    ///
    /// For EVM compatibility:
    /// - n=1 duplicates the top item (DUP1)
    /// - n=2 duplicates the second item (DUP2)
    /// - etc.
    ///
    /// Returns an error if n is 0, if there aren't enough items on the stack,
    /// or if the stack is full.
    pub fn dup(&mut self, n: usize) -> Result<(), StackError> {
        if n == 0 {
            return Err(StackError::InvalidIndex);
        }
        let len = self.items.len();
        if n > len {
            return Err(StackError::Underflow);
        }
        if len >= MAX_STACK_SIZE {
            return Err(StackError::Overflow);
        }
        // Duplicate n-th item (len - n) and push to top
        let value = self.items[len - n];
        self.items.push(value);
        Ok(())
    }

    /// Get the number of items on the stack
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Check if the stack is empty
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl Default for Stack {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Basic operations tests

    #[test]
    fn test_new_stack_is_empty() {
        let stack = Stack::new();
        assert!(stack.is_empty());
        assert_eq!(stack.len(), 0);
    }

    #[test]
    fn test_push_and_len() {
        let mut stack = Stack::new();
        assert_eq!(stack.push(U256::from_u64(1)), Ok(()));
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.push(U256::from_u64(2)), Ok(()));
        assert_eq!(stack.len(), 2);
    }

    #[test]
    fn test_pop() {
        let mut stack = Stack::new();
        stack.push(U256::from_u64(1)).unwrap();
        stack.push(U256::from_u64(2)).unwrap();

        assert_eq!(stack.pop(), Ok(U256::from_u64(2)));
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.pop(), Ok(U256::from_u64(1)));
        assert_eq!(stack.len(), 0);
    }

    #[test]
    fn test_peek() {
        let mut stack = Stack::new();
        stack.push(U256::from_u64(1)).unwrap();
        stack.push(U256::from_u64(2)).unwrap();
        stack.push(U256::from_u64(3)).unwrap();

        // Peek at top (index 0)
        assert_eq!(stack.peek(0), Ok(&U256::from_u64(3)));
        // Peek at second (index 1)
        assert_eq!(stack.peek(1), Ok(&U256::from_u64(2)));
        // Peek at third (index 2)
        assert_eq!(stack.peek(2), Ok(&U256::from_u64(1)));

        // Stack should be unchanged
        assert_eq!(stack.len(), 3);
    }

    // Stack overflow tests

    #[test]
    fn test_overflow_on_full_stack() {
        let mut stack = Stack::new();
        // Fill the stack to maximum capacity
        for i in 0..MAX_STACK_SIZE {
            assert_eq!(stack.push(U256::from_u64(i as u64)), Ok(()));
        }
        // Next push should fail
        assert_eq!(stack.push(U256::ZERO), Err(StackError::Overflow));
    }

    #[test]
    fn test_dup_overflow_on_full_stack() {
        let mut stack = Stack::new();
        // Fill the stack to maximum capacity
        for i in 0..MAX_STACK_SIZE {
            stack.push(U256::from_u64(i as u64)).unwrap();
        }
        // DUP should fail on full stack
        assert_eq!(stack.dup(1), Err(StackError::Overflow));
    }

    // Stack underflow tests

    #[test]
    fn test_underflow_on_empty_pop() {
        let mut stack = Stack::new();
        assert_eq!(stack.pop(), Err(StackError::Underflow));
    }

    #[test]
    fn test_underflow_on_empty_peek() {
        let stack = Stack::new();
        assert_eq!(stack.peek(0), Err(StackError::InvalidIndex));
    }

    #[test]
    fn test_swap_underflow_insufficient_items() {
        let mut stack = Stack::new();
        stack.push(U256::from_u64(1)).unwrap();
        // Only 1 item, can't swap with position 1
        assert_eq!(stack.swap(1), Err(StackError::Underflow));
    }

    #[test]
    fn test_dup_underflow_insufficient_items() {
        let mut stack = Stack::new();
        stack.push(U256::from_u64(1)).unwrap();
        // Only 1 item, can't dup position 2
        assert_eq!(stack.dup(2), Err(StackError::Underflow));
    }

    // Swap operations (SWAP1-SWAP16 tests)

    #[test]
    fn test_swap1() {
        let mut stack = Stack::new();
        stack.push(U256::from_u64(1)).unwrap();
        stack.push(U256::from_u64(2)).unwrap();

        assert_eq!(stack.swap(1), Ok(()));
        assert_eq!(stack.pop(), Ok(U256::from_u64(1)));
        assert_eq!(stack.pop(), Ok(U256::from_u64(2)));
    }

    #[test]
    fn test_swap2() {
        let mut stack = Stack::new();
        stack.push(U256::from_u64(1)).unwrap();
        stack.push(U256::from_u64(2)).unwrap();
        stack.push(U256::from_u64(3)).unwrap();

        assert_eq!(stack.swap(2), Ok(()));
        assert_eq!(stack.pop(), Ok(U256::from_u64(1)));
        assert_eq!(stack.pop(), Ok(U256::from_u64(2)));
        assert_eq!(stack.pop(), Ok(U256::from_u64(3)));
    }

    #[test]
    fn test_swap16() {
        let mut stack = Stack::new();
        // Push 17 items (1-17)
        for i in 1..=17 {
            stack.push(U256::from_u64(i)).unwrap();
        }

        // SWAP16 swaps top (17) with 16th item down (1)
        assert_eq!(stack.swap(16), Ok(()));
        assert_eq!(stack.pop(), Ok(U256::from_u64(1)));
        // Next should be 16
        for _ in 0..15 {
            stack.pop().unwrap();
        }
        assert_eq!(stack.pop(), Ok(U256::from_u64(17)));
    }

    #[test]
    fn test_swap_zero_invalid() {
        let mut stack = Stack::new();
        stack.push(U256::from_u64(1)).unwrap();
        assert_eq!(stack.swap(0), Err(StackError::InvalidIndex));
    }

    // Dup operations (DUP1-DUP16 tests)

    #[test]
    fn test_dup1() {
        let mut stack = Stack::new();
        stack.push(U256::from_u64(1)).unwrap();

        assert_eq!(stack.dup(1), Ok(()));
        assert_eq!(stack.len(), 2);
        assert_eq!(stack.pop(), Ok(U256::from_u64(1)));
        assert_eq!(stack.pop(), Ok(U256::from_u64(1)));
    }

    #[test]
    fn test_dup2() {
        let mut stack = Stack::new();
        stack.push(U256::from_u64(1)).unwrap();
        stack.push(U256::from_u64(2)).unwrap();

        assert_eq!(stack.dup(2), Ok(()));
        assert_eq!(stack.len(), 3);
        assert_eq!(stack.pop(), Ok(U256::from_u64(1)));
        assert_eq!(stack.pop(), Ok(U256::from_u64(2)));
        assert_eq!(stack.pop(), Ok(U256::from_u64(1)));
    }

    #[test]
    fn test_dup16() {
        let mut stack = Stack::new();
        // Push 16 items (1-16)
        for i in 1..=16 {
            stack.push(U256::from_u64(i)).unwrap();
        }

        // DUP16 duplicates the 16th item (1) to top
        assert_eq!(stack.dup(16), Ok(()));
        assert_eq!(stack.len(), 17);
        assert_eq!(stack.pop(), Ok(U256::from_u64(1)));
    }

    #[test]
    fn test_dup_zero_invalid() {
        let mut stack = Stack::new();
        stack.push(U256::from_u64(1)).unwrap();
        assert_eq!(stack.dup(0), Err(StackError::InvalidIndex));
    }

    // Edge cases and boundary conditions

    #[test]
    fn test_peek_out_of_bounds() {
        let mut stack = Stack::new();
        stack.push(U256::from_u64(1)).unwrap();
        stack.push(U256::from_u64(2)).unwrap();

        assert_eq!(stack.peek(2), Err(StackError::InvalidIndex));
        assert_eq!(stack.peek(100), Err(StackError::InvalidIndex));
    }

    #[test]
    fn test_operations_preserve_other_values() {
        let mut stack = Stack::new();
        stack.push(U256::from_u64(10)).unwrap();
        stack.push(U256::from_u64(20)).unwrap();
        stack.push(U256::from_u64(30)).unwrap();

        // Peek should not modify stack
        stack.peek(1).unwrap();
        assert_eq!(stack.len(), 3);

        // Swap should preserve all values
        stack.swap(1).unwrap();
        assert_eq!(stack.len(), 3);
    }

    #[test]
    fn test_large_u256_values() {
        let mut stack = Stack::new();
        stack.push(U256::MAX).unwrap();
        assert_eq!(stack.pop(), Ok(U256::MAX));
    }

    #[test]
    fn test_full_stack_operations() {
        let mut stack = Stack::new();
        // Fill stack completely
        for i in 0..MAX_STACK_SIZE {
            stack.push(U256::from_u64(i as u64)).unwrap();
        }

        // Verify we can still perform read operations
        assert_eq!(stack.peek(0), Ok(&U256::from_u64((MAX_STACK_SIZE - 1) as u64)));
        assert_eq!(stack.len(), MAX_STACK_SIZE);

        // Verify we can swap on full stack
        assert_eq!(stack.swap(1), Ok(()));

        // Pop one and verify we can push again
        stack.pop().unwrap();
        assert_eq!(stack.push(U256::ZERO), Ok(()));
    }

    #[test]
    fn test_multiple_operations_sequence() {
        let mut stack = Stack::new();

        // Push some values
        stack.push(U256::from_u64(1)).unwrap();
        stack.push(U256::from_u64(2)).unwrap();
        stack.push(U256::from_u64(3)).unwrap();

        // DUP1 (duplicate top)
        stack.dup(1).unwrap();
        assert_eq!(stack.peek(0), Ok(&U256::from_u64(3)));
        assert_eq!(stack.len(), 4);

        // SWAP2 (swap top with third)
        stack.swap(2).unwrap();
        assert_eq!(stack.peek(0), Ok(&U256::from_u64(2)));

        // Pop and verify
        assert_eq!(stack.pop(), Ok(U256::from_u64(2)));
        assert_eq!(stack.pop(), Ok(U256::from_u64(3)));
    }

    #[test]
    fn test_default_stack() {
        let stack = Stack::default();
        assert!(stack.is_empty());
        assert_eq!(stack.len(), 0);
    }

    #[test]
    fn test_error_display() {
        // Test error display messages
        let overflow = StackError::Overflow;
        let underflow = StackError::Underflow;
        let invalid = StackError::InvalidIndex;

        assert!(format!("{overflow}").contains("overflow"));
        assert!(format!("{underflow}").contains("underflow"));
        assert!(format!("{invalid}").contains("Invalid"));
    }
}
