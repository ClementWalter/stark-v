#!/usr/bin/env -S uv run
# /// script
# requires-python = ">=3.11"
# dependencies = ["eth-hash[pycryptodome]"]
# ///
"""
Debug state root mismatches by comparing expected vs computed state.
"""

import json
import sys
from eth_hash.auto import keccak

def hex_to_int(h):
    """Convert hex string to int."""
    if isinstance(h, str) and h.startswith('0x'):
        return int(h, 16)
    return int(h)

def analyze_test(test_path, test_name):
    """Analyze a specific test case."""
    with open(test_path, 'r') as f:
        data = json.load(f)

    # Find the test by partial name match
    matching_tests = [k for k in data.keys() if test_name in k]
    if not matching_tests:
        print(f"Error: No test found matching '{test_name}'")
        print(f"Available tests: {list(data.keys())}")
        return

    test_key = matching_tests[0]
    test = data[test_key]

    print(f"=== Analyzing {test_key} ===\n")

    # Get sender from transaction
    block = test['blocks'][0]
    tx = block['transactions'][0]

    # Print transaction details
    print("TRANSACTION:")
    print(f"  To: {tx.get('to', 'CREATE')}")
    print(f"  Value: {tx.get('value', '0x0')}")
    print(f"  Gas limit: {tx['gasLimit']}")
    print(f"  Data length: {len(tx.get('data', '0x')) // 2 - 1} bytes")

    # Extract sender from v,r,s signature
    sender = tx.get('sender')
    if not sender:
        # Try to derive from secretKey if present
        print("  Sender: <unknown>")
    else:
        print(f"  Sender: {sender}")

    print()

    # Compare pre and post state
    pre = test['pre']
    post = test['postState']

    all_addresses = sorted(set(list(pre.keys()) + list(post.keys())))

    print("STATE CHANGES:")
    for addr in all_addresses:
        pre_acc = pre.get(addr, {})
        post_acc = post.get(addr, {})

        # Check if there are any changes
        pre_balance = hex_to_int(pre_acc.get('balance', '0x0'))
        post_balance = hex_to_int(post_acc.get('balance', '0x0'))
        pre_nonce = hex_to_int(pre_acc.get('nonce', '0x0'))
        post_nonce = hex_to_int(post_acc.get('nonce', '0x0'))
        pre_code = pre_acc.get('code', '0x')
        post_code = post_acc.get('code', '0x')
        pre_storage = pre_acc.get('storage', {})
        post_storage = post_acc.get('storage', {})

        has_changes = (
            pre_balance != post_balance or
            pre_nonce != post_nonce or
            pre_code != post_code or
            pre_storage != post_storage
        )

        if has_changes:
            print(f"\n{addr}:")
            if pre_balance != post_balance:
                print(f"  Balance: {pre_balance} -> {post_balance} ({post_balance - pre_balance:+d})")
            if pre_nonce != post_nonce:
                print(f"  Nonce: {pre_nonce} -> {post_nonce} ({post_nonce - pre_nonce:+d})")
            if pre_code != post_code:
                print(f"  Code: {len(pre_code)//2-1} bytes -> {len(post_code)//2-1} bytes")
                print(f"    Pre:  {pre_code[:60]}...")
                print(f"    Post: {post_code[:60]}...")
            if pre_storage != post_storage:
                all_keys = sorted(set(list(pre_storage.keys()) + list(post_storage.keys())))
                storage_changes = []
                for key in all_keys:
                    pre_val = pre_storage.get(key, '0x0')
                    post_val = post_storage.get(key, '0x0')
                    if pre_val != post_val:
                        storage_changes.append((key, pre_val, post_val))

                if storage_changes:
                    print(f"  Storage changes ({len(storage_changes)} slots):")
                    for key, pre_val, post_val in storage_changes[:5]:
                        print(f"    {key}: {pre_val} -> {post_val}")
                    if len(storage_changes) > 5:
                        print(f"    ... and {len(storage_changes) - 5} more")

    print()
    print("EXPECTED ROOTS:")
    header = block['blockHeader']
    print(f"  State root: {header['stateRoot']}")
    print(f"  Tx root: {header['transactionsRoot']}")
    print(f"  Receipt root: {header['receiptsRoot']}")
    print(f"  Gas used: {header['gasUsed']}")

if __name__ == '__main__':
    if len(sys.argv) < 2:
        print("Usage: debug_state_mismatch.py <test_name>")
        print("Example: debug_state_mismatch.py optionsTest_Prague")
        sys.exit(1)

    test_name = sys.argv[1]

    # Map test names to files
    test_files = {
        'optionsTest': 'tests/eels/BlockchainTests/ValidBlocks/bcExample/optionsTest.json',
        'shanghaiExample': 'tests/eels/BlockchainTests/ValidBlocks/bcExample/shanghaiExample.json',
        'basefeeExample': 'tests/eels/BlockchainTests/ValidBlocks/bcExample/basefeeExample.json',
        'mergeExample': 'tests/eels/BlockchainTests/ValidBlocks/bcExample/mergeExample.json',
    }

    # Find matching file
    test_file = None
    for key, path in test_files.items():
        if key in test_name:
            test_file = path
            break

    if not test_file:
        print(f"Error: Unknown test '{test_name}'")
        print(f"Known tests: {', '.join(test_files.keys())}")
        sys.exit(1)

    analyze_test(test_file, test_name)
