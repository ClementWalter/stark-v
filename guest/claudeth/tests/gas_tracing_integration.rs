//! Integration test for EVM gas tracing
//!
//! This test demonstrates how to use the gas tracer to debug gas consumption.
//! Run with: cargo test --release --features evm-trace gas_tracing_integration

#[cfg(feature = "evm-trace")]
#[test]
fn test_gas_tracing_simple_execution() {
    use claudeth::state::InMemoryState;

    // Simple bytecode: PUSH1 0x02 PUSH1 0x03 ADD STOP
    let code = vec![0x60, 0x02, 0x60, 0x03, 0x01, 0x00];

    // Create EVM with tracing enabled
    let state = InMemoryState::new();
    let mut evm =
        claudeth::evm::interpreter::Evm::new(code, 100000, state, claudeth::evm::NullHost)
            .with_tracing();

    // Execute
    let result = evm.run().expect("execution should succeed");

    // Check execution succeeded
    assert!(result.success);
    assert_eq!(result.gas_used, 9); // PUSH1(3) + PUSH1(3) + ADD(3) + STOP(0) = 9

    // Get the tracer
    let tracer = evm.tracer().expect("tracer should be present");

    // Verify trace entries
    let entries = tracer.entries();
    assert_eq!(entries.len(), 4, "should have 4 opcodes");

    // Verify first PUSH1
    assert_eq!(entries[0].opcode, 0x60);
    assert_eq!(entries[0].name, "PUSH1");
    assert_eq!(entries[0].gas_cost, 3);

    // Verify second PUSH1
    assert_eq!(entries[1].opcode, 0x60);
    assert_eq!(entries[1].name, "PUSH1");
    assert_eq!(entries[1].gas_cost, 3);

    // Verify ADD
    assert_eq!(entries[2].opcode, 0x01);
    assert_eq!(entries[2].name, "ADD");
    assert_eq!(entries[2].gas_cost, 3);

    // Verify STOP
    assert_eq!(entries[3].opcode, 0x00);
    assert_eq!(entries[3].name, "STOP");
    assert_eq!(entries[3].gas_cost, 0);

    // Print trace for debugging (commented out to avoid spamming CI)
    // tracer.print();
}

#[cfg(feature = "evm-trace")]
#[test]
fn test_gas_tracing_memory_operations() {
    use claudeth::evm::NullHost;
    use claudeth::evm::interpreter::Evm;
    use claudeth::state::InMemoryState;

    // PUSH1 0x42 PUSH1 0x00 MSTORE STOP
    let code = vec![0x60, 0x42, 0x60, 0x00, 0x52, 0x00];

    let state = InMemoryState::new();
    let mut evm = Evm::new(code, 100000, state, NullHost).with_tracing();

    let result = evm.run().expect("execution should succeed");
    assert!(result.success);

    let tracer = evm.tracer().expect("tracer should be present");
    let entries = tracer.entries();

    // Should have 4 operations: PUSH1, PUSH1, MSTORE, STOP
    assert_eq!(entries.len(), 4);

    // MSTORE should have higher gas cost due to memory expansion
    let mstore_entry = &entries[2];
    assert_eq!(mstore_entry.opcode, 0x52);
    assert_eq!(mstore_entry.name, "MSTORE");
    // Base cost 3 + memory expansion cost
    assert!(
        mstore_entry.gas_cost > 3,
        "MSTORE should include memory expansion gas"
    );
}

#[cfg(not(feature = "evm-trace"))]
#[test]
fn test_tracing_not_available_without_feature() {
    // When feature is not enabled, tracing methods should not be available
    // This test just ensures the code compiles without the feature
    use claudeth::evm::NullHost;
    use claudeth::evm::interpreter::Evm;
    use claudeth::state::InMemoryState;

    let code = vec![0x60, 0x42, 0x00];
    let state = InMemoryState::new();
    let mut evm = Evm::new(code, 100000, state, NullHost);

    let result = evm.run().expect("execution should succeed");
    assert!(result.success);

    // Tracer methods are not available without feature flag
    // This is compile-time checked, so this test just ensures it compiles
}
