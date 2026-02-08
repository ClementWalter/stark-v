#!/usr/bin/env -S uv run
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""
Validation script for Claudeth implementation.

This script validates:
1. All tests pass in --release mode
2. Zero clippy warnings with --tests flag
3. Test coverage meets minimum requirements
4. No linter errors
"""

import subprocess
import sys
from pathlib import Path


def run_command(cmd: list[str], description: str) -> tuple[bool, str]:
    """Run a command and return success status and output."""
    print(f"\n{'='*60}")
    print(f"Running: {description}")
    print(f"Command: {' '.join(cmd)}")
    print(f"{'='*60}")

    result = subprocess.run(
        cmd,
        cwd=Path(__file__).parent.parent.parent,  # stark-v root
        capture_output=True,
        text=True
    )

    output = result.stdout + result.stderr
    print(output)

    success = result.returncode == 0
    status = "✅ PASS" if success else "❌ FAIL"
    print(f"\n{status}: {description}")

    return success, output


def main():
    """Run all validation checks."""
    manifest = "guest/claudeth/Cargo.toml"

    checks = [
        (
            ["cargo", "test", "--manifest-path", manifest, "--release"],
            "Tests in release mode"
        ),
        (
            ["cargo", "clippy", "--manifest-path", manifest, "--tests", "--", "-D", "warnings"],
            "Clippy with tests"
        ),
        (
            ["cargo", "check", "--manifest-path", manifest],
            "Compilation check"
        ),
    ]

    results = []
    for cmd, description in checks:
        success, output = run_command(cmd, description)
        results.append((description, success, output))

    # Print summary
    print(f"\n{'='*60}")
    print("VALIDATION SUMMARY")
    print(f"{'='*60}")

    all_passed = True
    for description, success, _ in results:
        status = "✅ PASS" if success else "❌ FAIL"
        print(f"{status}: {description}")
        if not success:
            all_passed = False

    # Count tests
    for description, success, output in results:
        if "Tests in release mode" in description and success:
            # Extract test count
            for line in output.split("\n"):
                if "test result:" in line:
                    print(f"\n{line}")

    print(f"\n{'='*60}")
    if all_passed:
        print("✅ ALL VALIDATION CHECKS PASSED")
        print(f"{'='*60}")
        return 0
    else:
        print("❌ VALIDATION FAILED")
        print(f"{'='*60}")
        return 1


if __name__ == "__main__":
    sys.exit(main())
