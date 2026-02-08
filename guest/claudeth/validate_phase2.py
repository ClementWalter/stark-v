#!/usr/bin/env -S uv run
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///

"""
Phase 2 (Partial MPT) Validation Script

Validates that Phase 2 implementation meets all requirements:
- All required files exist
- Tests pass (minimum 145 tests)
- Zero clippy warnings
- All modules properly integrated
"""

import subprocess
import sys
from pathlib import Path

CLAUDETH_DIR = Path(__file__).parent
MANIFEST_PATH = CLAUDETH_DIR / "Cargo.toml"

# Required files for Phase 2
REQUIRED_FILES = [
    "src/state/mod.rs",
    "src/state/account.rs",
    "src/state/storage.rs",
    "src/state/partial_mpt/mod.rs",
    "src/state/partial_mpt/node.rs",
    "src/state/partial_mpt/trie.rs",
    "src/state/partial_mpt/root.rs",
    "src/state/partial_mpt/proof.rs",
]

MIN_TESTS = 145  # 30+40+20+30+25 from Phase 2 tasks


def check_files() -> bool:
    """Check that all required files exist."""
    print("📁 Checking required files...")
    all_exist = True
    for file in REQUIRED_FILES:
        path = CLAUDETH_DIR / file
        if path.exists():
            print(f"  ✅ {file}")
        else:
            print(f"  ❌ {file} - MISSING")
            all_exist = False
    return all_exist


def run_tests() -> tuple[bool, int]:
    """Run tests in release mode and count passing tests."""
    print("\n🧪 Running tests in --release mode...")
    result = subprocess.run(
        ["cargo", "test", "--manifest-path", str(MANIFEST_PATH), "--release"],
        capture_output=True,
        text=True,
    )

    # Count passing tests
    test_count = 0
    for line in result.stdout.split("\n"):
        if "test result: ok." in line:
            # Extract number: "test result: ok. 385 passed"
            parts = line.split()
            if len(parts) >= 4:
                test_count += int(parts[3])

    success = result.returncode == 0
    if success:
        print(f"  ✅ All tests passed ({test_count} tests)")
        if test_count >= MIN_TESTS:
            print(f"  ✅ Test count requirement met ({test_count} >= {MIN_TESTS})")
        else:
            print(f"  ⚠️  Test count below minimum ({test_count} < {MIN_TESTS})")
            success = False
    else:
        print(f"  ❌ Tests failed")
        print(result.stdout)
        print(result.stderr)

    return success, test_count


def run_clippy() -> bool:
    """Run clippy with strict warnings."""
    print("\n🔍 Running clippy with --tests -D warnings...")
    result = subprocess.run(
        [
            "cargo",
            "clippy",
            "--manifest-path",
            str(MANIFEST_PATH),
            "--tests",
            "--",
            "-D",
            "warnings",
        ],
        capture_output=True,
        text=True,
    )

    success = result.returncode == 0
    if success:
        print("  ✅ Zero clippy warnings")
    else:
        print("  ❌ Clippy warnings found:")
        print(result.stdout)
        print(result.stderr)

    return success


def main():
    """Run all validation checks."""
    print("=" * 60)
    print("Phase 2 (Partial MPT) Validation")
    print("=" * 60)

    files_ok = check_files()
    tests_ok, test_count = run_tests()
    clippy_ok = run_clippy()

    print("\n" + "=" * 60)
    print("Validation Summary")
    print("=" * 60)
    print(f"Files:  {'✅ PASS' if files_ok else '❌ FAIL'}")
    print(f"Tests:  {'✅ PASS' if tests_ok else '❌ FAIL'} ({test_count} tests)")
    print(f"Clippy: {'✅ PASS' if clippy_ok else '❌ FAIL'}")

    all_ok = files_ok and tests_ok and clippy_ok

    if all_ok:
        print("\n🎉 Phase 2 validation: ALL CHECKS PASSED")
        return 0
    else:
        print("\n❌ Phase 2 validation: SOME CHECKS FAILED")
        return 1


if __name__ == "__main__":
    sys.exit(main())
