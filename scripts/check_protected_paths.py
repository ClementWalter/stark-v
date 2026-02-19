#!/usr/bin/env -S uv run
# /// script
# requires-python = ">=3.11"
# dependencies = ["pathspec"]
# ///

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

import pathspec

PROTECTED_PATTERNS = """
# exact files
guest/claudeth/README.md

# directories
crates/sdk/
crates/runner/
crates/prover/
crates/bench-cli/
crates/debug-utils/
crates/stwo-macros/
external/stwo/
"""


def _run_git(args: list[str]) -> bytes:
    return subprocess.check_output(["git", *args])


def _repo_root() -> Path:
    root = _run_git(["rev-parse", "--show-toplevel"]).decode().strip()
    return Path(root)


def _split_z(output: bytes) -> list[str]:
    if not output:
        return []
    items = [item for item in output.split(b"\0") if item]
    return [item.decode() for item in items]


def _collect_changed_paths() -> set[str]:
    staged = _split_z(_run_git(["diff", "--name-only", "--cached", "-z"]))
    unstaged = _split_z(_run_git(["diff", "--name-only", "-z"]))
    untracked = _split_z(_run_git(["ls-files", "--others", "--exclude-standard", "-z"]))
    return set(staged) | set(unstaged) | set(untracked)


def _normalize_paths(paths: set[str], root: Path) -> list[str]:
    normalized: list[str] = []
    for path in paths:
        rel = Path(path)
        if rel.is_absolute():
            try:
                rel = rel.relative_to(root)
            except ValueError:
                continue
        normalized.append(rel.as_posix())
    return sorted(set(normalized))


def _load_patterns() -> list[str]:
    patterns: list[str] = []
    for line in PROTECTED_PATTERNS.splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        patterns.append(stripped)
    return patterns


def main() -> int:
    root = _repo_root()
    paths = _normalize_paths(_collect_changed_paths(), root)
    if not paths:
        return 0

    patterns = _load_patterns()
    spec = pathspec.PathSpec.from_lines("gitwildmatch", patterns)
    protected = [path for path in paths if spec.match_file(path)]

    if not protected:
        return 0

    print("Protected paths were modified (staged or unstaged):")
    for path in protected:
        print(f"- {path}")
    print("\nRemove these changes before committing.")
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
