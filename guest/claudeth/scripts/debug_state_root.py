#!/usr/bin/env -S uv run
# /// script
# requires-python = ">=3.11"
# dependencies = ["eth-hash[pycryptodome]"]
# ///
"""
Debug state root mismatches by analyzing EELS test fixtures.
Compares expected post-state with what claudeth likely computed.
"""

import json
import sys
from pathlib import Path
from eth_hash.auto import keccak


def load_test(test_path: str) -> dict:
    """Load an EELS test fixture."""
    with open(test_path) as f:
        data = json.load(f)
    # EELS fixtures have nested structure: {"testName": {...}}
    return next(iter(data.values()))


def hex_to_int(h: str) -> int:
    """Convert hex string to integer."""
    if h.startswith("0x"):
        h = h[2:]
    return int(h, 16) if h else 0


def hex_to_bytes(h: str) -> bytes:
    """Convert hex string to bytes."""
    if h.startswith("0x"):
        h = h[2:]
    if len(h) % 2:
        h = "0" + h
    return bytes.fromhex(h)


def analyze_state_diff(test_name: str, test_data: dict):
    """Analyze state differences between pre and post state."""
    print(f"\n{'='*80}")
    print(f"Test: {test_name}")
    print(f"{'='*80}\n")

    pre_state = test_data.get("pre", {})
    post_state = test_data.get("postState", {})

    # Find all addresses involved
    all_addresses = set(pre_state.keys()) | set(post_state.keys())

    for addr in sorted(all_addresses):
        pre_acc = pre_state.get(addr, {})
        post_acc = post_state.get(addr, {})

        # Check if account changed
        balance_changed = pre_acc.get("balance") != post_acc.get("balance")
        nonce_changed = pre_acc.get("nonce") != post_acc.get("nonce")
        code_changed = pre_acc.get("code") != post_acc.get("code")

        pre_storage = pre_acc.get("storage", {})
        post_storage = post_acc.get("storage", {})
        storage_changed = pre_storage != post_storage

        if balance_changed or nonce_changed or code_changed or storage_changed:
            print(f"\nAddress: {addr}")
            print("-" * 80)

            if balance_changed:
                print(f"  Balance:  {pre_acc.get('balance', '0x0')} -> {post_acc.get('balance', '0x0')}")
                pre_bal = hex_to_int(pre_acc.get('balance', '0x0'))
                post_bal = hex_to_int(post_acc.get('balance', '0x0'))
                delta = post_bal - pre_bal
                print(f"            Delta: {delta:+d} ({delta:+#x})")

            if nonce_changed:
                print(f"  Nonce:    {pre_acc.get('nonce', '0x0')} -> {post_acc.get('nonce', '0x0')}")

            if code_changed:
                pre_code = pre_acc.get('code', '0x')
                post_code = post_acc.get('code', '0x')
                print(f"  Code:     {len(hex_to_bytes(pre_code))} bytes -> {len(hex_to_bytes(post_code))} bytes")
                if post_code != '0x':
                    code_bytes = hex_to_bytes(post_code)
                    code_hash = keccak(code_bytes)
                    print(f"            Code hash: 0x{code_hash.hex()}")

            if storage_changed:
                all_slots = set(pre_storage.keys()) | set(post_storage.keys())
                if all_slots:
                    print(f"  Storage changes:")
                    for slot in sorted(all_slots):
                        pre_val = pre_storage.get(slot, '0x0')
                        post_val = post_storage.get(slot, '0x0')
                        if pre_val != post_val:
                            print(f"    Slot {slot}:")
                            print(f"      {pre_val} -> {post_val}")

    # Show transaction details
    blocks = test_data.get("blocks", [])
    if blocks:
        block = blocks[0]
        txs = block.get("transactions", [])
        print(f"\n\nTransactions ({len(txs)}):")
        print("-" * 80)
        for i, tx in enumerate(txs):
            print(f"\nTx {i}:")
            print(f"  From:       {tx.get('sender', 'N/A')}")
            print(f"  To:         {tx.get('to', '(contract creation)')}")
            print(f"  Value:      {tx.get('value', '0x0')}")
            print(f"  Gas limit:  {tx.get('gasLimit', 'N/A')}")
            print(f"  Gas price:  {tx.get('gasPrice', 'N/A')}")

            data = tx.get('data', ['0x'])[0] if isinstance(tx.get('data'), list) else tx.get('data', '0x')
            data_bytes = hex_to_bytes(data)
            print(f"  Data:       {len(data_bytes)} bytes")

            # Show access list if present
            access_list = tx.get('accessList', [])
            if access_list:
                print(f"  Access list ({len(access_list)} entries):")
                for entry in access_list:
                    addr = entry.get('address')
                    keys = entry.get('storageKeys', [])
                    print(f"    {addr}: {len(keys)} storage keys")


def main():
    if len(sys.argv) < 2:
        print("Usage: debug_state_root.py <test_name>")
        print("\nAvailable tests:")
        print("  - optionsTest")
        print("  - shanghaiExample")
        print("  - basefeeExample")
        print("  - tloadDoesNotPersistAcrossBlocks")
        print("  - tloadDoesNotPersistCrossTxn")
        print("  - transStorageBlockchain")
        sys.exit(1)

    test_name = sys.argv[1]

    # Map test names to file paths
    test_files = {
        "optionsTest": "tests/eels/BlockchainTests/ValidBlocks/bcExample/optionsTest.json",
        "shanghaiExample": "tests/eels/BlockchainTests/ValidBlocks/bcExample/shanghaiExample.json",
        "basefeeExample": "tests/eels/BlockchainTests/ValidBlocks/bcExample/basefeeExample.json",
        "tloadDoesNotPersistAcrossBlocks": "tests/eels/BlockchainTests/ValidBlocks/bcEIP1153-transientStorage/tloadDoesNotPersistAcrossBlocks.json",
        "tloadDoesNotPersistCrossTxn": "tests/eels/BlockchainTests/ValidBlocks/bcEIP1153-transientStorage/tloadDoesNotPersistCrossTxn.json",
        "transStorageBlockchain": "tests/eels/BlockchainTests/ValidBlocks/bcEIP1153-transientStorage/transStorageBlockchain.json",
    }

    if test_name not in test_files:
        print(f"Unknown test: {test_name}")
        print(f"Available tests: {', '.join(test_files.keys())}")
        sys.exit(1)

    test_path = Path(__file__).parent.parent / test_files[test_name]
    if not test_path.exists():
        print(f"Test file not found: {test_path}")
        print("Run scripts/fetch_eels_tests.py first")
        sys.exit(1)

    test_data = load_test(str(test_path))
    analyze_state_diff(test_name, test_data)


if __name__ == "__main__":
    main()
