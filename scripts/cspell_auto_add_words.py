#!/usr/bin/env python3
# /// script
# requires-python = ">=3.10"
# ///
"""
Pre-commit helper: auto-allow all words flagged by cspell.

Workflow:
1) Run trunk check --filter=cspell --index (with --cache=false).
2) If it fails with "Unknown word (...)" issues, extract those words.
3) Lowercase and collect unique words, merge into cspell.yaml under words: (only new words).
4) Keep the words: list sorted (casefold, then original string).
5) Optionally git add cspell.yaml and re-run the check; succeed only if clean.

No third-party deps; stdlib only. Invoke with: uv run scripts/cspell_auto_add_words.py
"""

from __future__ import annotations

import os
import re
import subprocess
import sys
from pathlib import Path
from typing import Iterable

# Match "Unknown word (word)" from cspell/trunk output.
# Strip ANSI escapes before searching.
UNKNOWN_WORD_RE = re.compile(r"Unknown word [\\]?\(([^)]+)[\\]?\)")
ANSI_ESCAPE_RE = re.compile(r"\x1B\[[0-?]*[ -/]*[@-~]")


def _repo_root() -> Path:
    proc = subprocess.run(
        ["git", "rev-parse", "--show-toplevel"],
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    return Path(proc.stdout.strip()).resolve()


def _run(cmd: list[str], *, cwd: Path, check: bool) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        cmd,
        cwd=str(cwd),
        check=check,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        env={**os.environ, "NO_COLOR": "1"},
    )


def _trunk_cspell_index(*, cwd: Path) -> subprocess.CompletedProcess[str]:
    return _run(
        [
            "trunk",
            "check",
            "--filter=cspell",
            "--index",
            "--cache=false",
            "--no-progress",
            "--color=false",
        ],
        cwd=cwd,
        check=False,
    )


def _extract_unknown_words(output: str) -> set[str]:
    output = ANSI_ESCAPE_RE.sub("", output)
    words: set[str] = set()
    for m in UNKNOWN_WORD_RE.finditer(output):
        w = m.group(1).strip().lower()
        if w:
            words.add(w)
    return words


def _is_top_level_key(line: str) -> bool:
    if not line or line[0].isspace() or line.startswith("#"):
        return False
    return bool(re.match(r"^[A-Za-z0-9_-]+\s*:", line))


def _parse_word_item(line: str) -> str | None:
    s = line.strip()
    if not s.startswith("- "):
        return None
    item = s[2:].strip()
    if not item:
        return None
    if (item.startswith("'") and item.endswith("'") and len(item) >= 2) or (
        item.startswith('"') and item.endswith('"') and len(item) >= 2
    ):
        item = item[1:-1]
    return item


def _yaml_emit_word(word: str) -> str:
    if re.fullmatch(r"[A-Za-z0-9_./+-]+", word) and not re.fullmatch(
        r"(?i:true|false|null|yes|no|on|off)", word
    ):
        return word
    escaped = word.replace("'", "''")
    return f"'{escaped}'"


def _update_cspell_yaml(cspell_path: Path, new_words: Iterable[str]) -> int:
    """Merge new_words into words:, sort full list by casefold then original. Return count added."""
    text = cspell_path.read_text(encoding="utf-8")
    lines = text.splitlines(keepends=True)

    words_key_idx: int | None = None
    for i, line in enumerate(lines):
        if line.strip() == "words:" and not line.startswith(" "):
            words_key_idx = i
            break
    if words_key_idx is None:
        raise RuntimeError(f"Could not find top-level `words:` in {cspell_path}")

    block_end = words_key_idx + 1
    while block_end < len(lines) and not _is_top_level_key(lines[block_end].rstrip("\n")):
        block_end += 1

    existing_casefold: set[str] = set()
    existing_words: list[str] = []
    for line in lines[words_key_idx + 1 : block_end]:
        item = _parse_word_item(line)
        if item is not None and item.casefold() not in existing_casefold:
            existing_casefold.add(item.casefold())
            existing_words.append(item)

    to_add: list[str] = []
    for w in new_words:
        if w.casefold() in existing_casefold:
            continue
        existing_casefold.add(w.casefold())
        to_add.append(w)

    if not to_add:
        return 0

    # Full list = existing + new, sorted by (casefold(), original)
    all_words = existing_words + to_add
    all_words.sort(key=lambda w: (w.casefold(), w))

    block_lines = ["words:\n"] + [f"  - {_yaml_emit_word(w)}\n" for w in all_words]
    new_content = "".join(lines[:words_key_idx]) + "".join(block_lines) + "".join(lines[block_end:])
    if not new_content.endswith("\n"):
        new_content += "\n"
    cspell_path.write_text(new_content, encoding="utf-8")
    return len(to_add)


def main() -> int:
    root = _repo_root()
    cspell_path = root / "cspell.yaml"

    first = _trunk_cspell_index(cwd=root)
    if first.returncode == 0:
        return 0

    unknown = _extract_unknown_words(first.stdout)
    if not unknown:
        sys.stderr.write(first.stdout)
        sys.stderr.write("\n")
        sys.stderr.write("cspell failed, but no `Unknown word (...)` patterns were found to auto-add.\n")
        return 1

    added = _update_cspell_yaml(cspell_path, unknown)
    if added == 0:
        sys.stderr.write(first.stdout)
        sys.stderr.write("\n")
        sys.stderr.write("No new words were added to cspell.yaml, but cspell still fails.\n")
        return 1

    _run(["git", "add", str(cspell_path)], cwd=root, check=True)

    second = _trunk_cspell_index(cwd=root)
    if second.returncode == 0:
        sys.stdout.write(f"cspell: auto-added {added} word(s) to cspell.yaml\n")
        return 0

    sys.stderr.write(second.stdout)
    sys.stderr.write("\n")
    sys.stderr.write("cspell still failing after auto-adding words; please inspect output above.\n")
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
