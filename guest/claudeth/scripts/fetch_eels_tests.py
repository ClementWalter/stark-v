#!/usr/bin/env -S uv run
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///

"""
Fetch Ethereum execution-spec-tests JSON fixtures.

This script clones the ethereum/tests repository to access test fixtures
for integration testing with claudeth.
"""

import subprocess
import sys
from pathlib import Path

# Test repository configuration
TESTS_REPO_URL = "https://github.com/ethereum/tests.git"
TESTS_DIR = Path(__file__).parent.parent / "tests" / "eels"


def clone_tests_repo():
    """Clone the ethereum/tests repository."""
    if TESTS_DIR.exists():
        print(f"Tests directory already exists at {TESTS_DIR}")
        print("Pulling latest changes...")
        try:
            subprocess.run(
                ["git", "pull"],
                cwd=TESTS_DIR,
                check=True,
                capture_output=True,
            )
            print("  ✓ Updated to latest")
        except subprocess.CalledProcessError as e:
            print(f"Warning: git pull failed: {e.stderr.decode()}")
            print("Continuing with existing tests...")
        return

    print(f"Cloning ethereum/tests repository...")
    print(f"  URL: {TESTS_REPO_URL}")
    print(f"  Destination: {TESTS_DIR}")
    print("  (This may take a few minutes...)")

    try:
        # Use --depth=1 for shallow clone to save time/space
        subprocess.run(
            ["git", "clone", "--depth=1", TESTS_REPO_URL, str(TESTS_DIR)],
            check=True,
            capture_output=True,
        )
        print("  ✓ Cloned successfully")
    except subprocess.CalledProcessError as e:
        print(f"Error cloning repository: {e.stderr.decode()}", file=sys.stderr)
        sys.exit(1)


def analyze_tests(tests_dir: Path):
    """
    Analyze available tests and categorize them.

    Currently we focus on:
    - BlockchainTests: Full block processing
    - GeneralStateTests: Transaction execution and state transitions
    """
    print("\nAnalyzing test fixtures...")

    # Count test files by category
    blockchain_tests = list(tests_dir.glob("BlockchainTests/**/*.json"))
    state_tests = list(tests_dir.glob("GeneralStateTests/**/*.json"))

    print(f"  Found {len(blockchain_tests)} blockchain tests")
    print(f"  Found {len(state_tests)} state tests")

    # Sample some test paths to understand structure
    if blockchain_tests:
        print("\nSample blockchain tests:")
        for test in blockchain_tests[:3]:
            rel_path = test.relative_to(tests_dir)
            print(f"  - {rel_path}")

    if state_tests:
        print("\nSample state tests:")
        for test in state_tests[:3]:
            rel_path = test.relative_to(tests_dir)
            print(f"  - {rel_path}")

    return {
        "blockchain": blockchain_tests,
        "state": state_tests,
    }


def main():
    """Main entry point."""
    print("="*60)
    print("Fetching Ethereum Test Fixtures")
    print("="*60)

    # Clone or update the tests repository
    clone_tests_repo()

    # Analyze available tests
    print("\n" + "="*60)
    analyze_tests(TESTS_DIR)

    print("\n" + "="*60)
    print("✓ Test fixtures ready!")
    print(f"\nTests located at: {TESTS_DIR}")
    print("\nNext steps:")
    print("1. Review test structure in tests/eels/")
    print("2. Build Rust test harness to parse JSON fixtures")
    print("3. Execute tests against claudeth STF")


if __name__ == "__main__":
    main()
