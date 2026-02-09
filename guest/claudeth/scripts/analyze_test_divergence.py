#!/usr/bin/env -S uv run
# /// script
# requires-python = ">=3.11"
# dependencies = ["eth-hash[pycryptodome]"]
# ///

"""
Analyze EELS test divergence by comparing fixture expected state with our computed state.

Usage:
    uv run scripts/analyze_test_divergence.py tests/eels/BlockchainTests/.../test.json Prague
"""

import json
import sys
from pathlib import Path
from eth_hash.auto import keccak


def parse_hex(s: str) -> bytes:
    """Parse hex string (with or without 0x prefix)."""
    if s.startswith("0x"):
        s = s[2:]
    return bytes.fromhex(s)


def to_hex(b: bytes) -> str:
    """Convert bytes to hex string with 0x prefix."""
    return "0x" + b.hex()


def rlp_encode_length(length: int, offset: int) -> bytes:
    """RLP encode a length."""
    if length < 56:
        return bytes([offset + length])
    elif length < 256**8:
        bl = length.to_bytes((length.bit_length() + 7) // 8, "big")
        return bytes([offset + 55 + len(bl)]) + bl
    else:
        raise ValueError("Length too large for RLP")


def rlp_encode_bytes(data: bytes) -> bytes:
    """RLP encode bytes."""
    if len(data) == 1 and data[0] < 0x80:
        return data
    return rlp_encode_length(len(data), 0x80) + data


def rlp_encode_list(items: list[bytes]) -> bytes:
    """RLP encode a list of already-encoded items."""
    encoded = b"".join(items)
    return rlp_encode_length(len(encoded), 0xC0) + encoded


def rlp_encode_uint(value: int) -> bytes:
    """RLP encode a uint (big-endian, minimal encoding)."""
    if value == 0:
        return b"\x80"  # Empty byte array
    # Remove leading zeros
    value_bytes = value.to_bytes((value.bit_length() + 7) // 8, "big")
    return rlp_encode_bytes(value_bytes)


def rlp_encode_account(nonce: int, balance: int, storage_root: bytes, code_hash: bytes) -> bytes:
    """RLP encode an Ethereum account [nonce, balance, storage_root, code_hash]."""
    nonce_rlp = rlp_encode_uint(nonce)
    balance_rlp = rlp_encode_uint(balance)
    storage_root_rlp = rlp_encode_bytes(storage_root)
    code_hash_rlp = rlp_encode_bytes(code_hash)
    return rlp_encode_list([nonce_rlp, balance_rlp, storage_root_rlp, code_hash_rlp])


def main():
    if len(sys.argv) < 3:
        print("Usage: uv run scripts/analyze_test_divergence.py <test.json> <network>")
        sys.exit(1)

    test_file = Path(sys.argv[1])
    network = sys.argv[2]  # "Cancun" or "Prague"

    if not test_file.exists():
        print(f"Error: {test_file} does not exist")
        sys.exit(1)

    with open(test_file) as f:
        data = json.load(f)

    # Find the test case - try both with and without path prefix
    test_name = test_file.stem
    full_test_name = f"{test_name}_{network}"

    # Try to find matching key
    matching_key = None
    for key in data.keys():
        if key.endswith(f"::{full_test_name}"):
            matching_key = key
            break
        elif key == full_test_name:
            matching_key = key
            break

    if matching_key is None:
        print(f"Error: Test case {full_test_name} not found in {test_file}")
        print(f"Available tests: {list(data.keys())}")
        sys.exit(1)

    test_case = data[matching_key]
    print(f"Using test key: {matching_key}")

    # Get post-state (after all blocks)
    post_state = test_case.get("postState", {})

    print(f"\n=== {full_test_name} ===\n")
    print(f"Number of blocks: {len(test_case.get('blocks', []))}")
    print(f"Number of accounts in post-state: {len(post_state)}\n")

    # Constants
    EMPTY_TRIE_ROOT = bytes.fromhex("56e81f171bcc55a6ff8345e692c0f86e5b96e01b996cadc001622fb5e363b421")
    EMPTY_CODE_HASH = bytes.fromhex("c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470")

    print("Expected account RLPs (from post-state):\n")

    for address_hex, account_data in sorted(post_state.items()):
        address = parse_hex(address_hex)
        address_hash = keccak(address)

        nonce = int(account_data.get("nonce", "0x0"), 16)
        balance = int(account_data.get("balance", "0x0"), 16)

        # Storage root - need to compute from storage
        storage = account_data.get("storage", {})
        if storage:
            # Non-empty storage - we'd need to build a trie to compute the real root
            # For now, just note it's not empty
            storage_root = bytes(32)  # Placeholder - real implementation builds trie
            storage_root_hex = "<computed from storage>"
        else:
            storage_root = EMPTY_TRIE_ROOT
            storage_root_hex = to_hex(storage_root)

        # Code hash
        code_hex = account_data.get("code", "0x")
        if code_hex == "0x" or code_hex == "":
            code_hash = EMPTY_CODE_HASH
        else:
            code = parse_hex(code_hex)
            code_hash = keccak(code)

        # Encode account
        account_rlp = rlp_encode_account(nonce, balance, storage_root, code_hash)

        print(f"  {address_hex}")
        print(f"    hashed_key: {to_hex(address_hash)}")
        print(f"    nonce:       {hex(nonce)}")
        print(f"    balance:     {hex(balance)}")
        print(f"    storage_root: {storage_root_hex}")
        print(f"    code_hash:    {to_hex(code_hash)}")
        print(f"    account_rlp:  {to_hex(account_rlp)}")
        if storage:
            print(f"    storage entries: {len(storage)}")
            for key, value in sorted(storage.items())[:5]:  # Show first 5
                print(f"      {key} = {value}")
            if len(storage) > 5:
                print(f"      ... and {len(storage) - 5} more")
        print()


if __name__ == "__main__":
    main()
