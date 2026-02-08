#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.14"
# dependencies = ["typer>=0.12.0"]
# ///
"""
Ralph loop runner (Python version of the shell template).

Shell template:

  prompt="your prompt here"
  for i in $(seq 1 100); do
    claude --dangerously-skip-permissions -p "$prompt"
  done

Usage:
  uv run --script ralph.py                    # reads PROMPT.md

  uv run --script ralph.py "your prompt here" --iterations 50
  uv run --script ralph.py "your prompt here" --no-change-exit-after 10

  ./ralph.py "your prompt here"      # after: chmod +x ralph.py

Stops early if there are no git changes (no HEAD change and no `git diff` change)
for N consecutive iterations.
"""

from __future__ import annotations

import hashlib
import os
import subprocess
import sys
from pathlib import Path
from typing import Optional

import typer  # pyright: ignore[reportMissingImports]

DEFAULT_ITERATIONS = 50
DEFAULT_NO_CHANGE_EXIT_AFTER = 10
DEFAULT_PROMPT_FILE = "PROMPT.md"


def _repo_dir() -> Path:
    # `ralph.py` lives in the repo root; anchor git commands here (not the caller's CWD).
    return Path(__file__).resolve().parent


def _resolve_prompt_file(prompt_file: Path, cwd: Path) -> Path:
    if prompt_file.is_absolute():
        return prompt_file
    return cwd / prompt_file


def _load_prompt(prompt: Optional[str], prompt_file: Path, cwd: Path) -> str:
    if prompt is not None:
        if not prompt.strip():
            typer.echo("PROMPT argument cannot be empty.", err=True)
            raise typer.Exit(code=2)
        return prompt

    path = _resolve_prompt_file(prompt_file, cwd)
    try:
        contents = path.read_text(encoding="utf-8")
    except FileNotFoundError:
        typer.echo(
            f"Prompt file not found: {path}\n"
            "Create PROMPT.md in the repo root, or pass a PROMPT argument.",
            err=True,
        )
        raise typer.Exit(code=2) from None
    except OSError as e:
        typer.echo(f"Failed to read prompt file {path}: {e}", err=True)
        raise typer.Exit(code=2) from None

    if not contents.strip():
        typer.echo(f"Prompt file {path} is empty.", err=True)
        raise typer.Exit(code=2)

    return contents


def _git_bytes(cwd: Path, *args: str) -> bytes:
    try:
        completed = subprocess.run(
            ["git", *args],
            cwd=cwd,
            check=False,
            capture_output=True,
        )
    except FileNotFoundError:
        typer.echo('Could not find "git" on PATH.', err=True)
        raise typer.Exit(code=127) from None

    if completed.returncode != 0:
        stderr = completed.stderr.decode(errors="replace")
        typer.echo(f"git command failed: git {' '.join(args)}\n{stderr}", err=True)
        raise typer.Exit(code=2)

    return completed.stdout


def _head_sha(cwd: Path) -> Optional[str]:
    try:
        completed = subprocess.run(
            ["git", "rev-parse", "--verify", "HEAD"],
            cwd=cwd,
            check=False,
            capture_output=True,
            text=True,
        )
    except FileNotFoundError:
        typer.echo('Could not find "git" on PATH.', err=True)
        raise typer.Exit(code=127) from None

    if completed.returncode != 0:
        return None
    return completed.stdout.strip()


def _git_state_fingerprint(cwd: Path) -> str:
    """
    Content-sensitive fingerprint of the git working state.

    Includes:
    - tracked unstaged diff (git diff)
    - tracked staged diff (git diff --cached)
    - untracked files + their content hashes
    """
    h = hashlib.sha256()

    h.update(b"unstaged\0")
    h.update(
        _git_bytes(
            cwd,
            "diff",
            "--no-ext-diff",
            "--binary",
            "--no-color",
        )
    )

    h.update(b"\0staged\0")
    h.update(
        _git_bytes(
            cwd,
            "diff",
            "--cached",
            "--no-ext-diff",
            "--binary",
            "--no-color",
        )
    )

    h.update(b"\0untracked\0")
    untracked_raw = _git_bytes(cwd, "ls-files", "--others", "--exclude-standard", "-z")
    untracked = [p for p in untracked_raw.split(b"\0") if p]
    for rel_b in sorted(untracked):
        h.update(rel_b)
        h.update(b"\0")

        rel = os.fsdecode(rel_b)
        path = cwd / rel
        try:
            if path.is_file():
                h.update(hashlib.sha256(path.read_bytes()).digest())
            else:
                h.update(b"\0" * 32)
        except OSError:
            h.update(b"\0" * 32)
        h.update(b"\0")

    return h.hexdigest()


def cli(
    prompt: Optional[str] = typer.Argument(
        None,
        help=f"Prompt to pass to Claude. If omitted, reads {DEFAULT_PROMPT_FILE}.",
    ),
    iterations: int = typer.Option(
        DEFAULT_ITERATIONS,
        "--iterations",
        "-n",
        min=1,
        help="Maximum number of iterations.",
        show_default=True,
    ),
    claude_bin: str = typer.Option(
        "claude",
        "--claude-bin",
        envvar="CLAUDE_BIN",
        help='Claude executable (default: "claude", or $CLAUDE_BIN).',
        show_default=True,
    ),
    no_change_exit_after: int = typer.Option(
        DEFAULT_NO_CHANGE_EXIT_AFTER,
        "--no-change-exit-after",
        min=0,
        help=(
            "Stop early if there are no git changes (no HEAD change and no `git diff` change) "
            "for N consecutive iterations. Set to 0 to disable."
        ),
        show_default=True,
    ),
) -> None:
    """
    Run Claude in a loop.
    """

    cwd = Path(__file__).resolve().parent
    resolved_prompt = _load_prompt(prompt, Path(DEFAULT_PROMPT_FILE), cwd)
    previous_head = _head_sha(cwd) if no_change_exit_after else None
    previous_fingerprint = _git_state_fingerprint(cwd) if no_change_exit_after else ""
    no_change_streak = 0

    try:
        for i in range(1, iterations + 1):
            typer.echo(f"Iteration {i} of {iterations}", err=True)
            try:
                typer.echo(f"Running Claude with prompt: {resolved_prompt}", err=True)
                completed = subprocess.run(
                    ["claude", "--dangerously-skip-permissions", "-p", resolved_prompt,  "--output-format", "stream-json", "--verbose"],
                    cwd=cwd,
                    check=False,
                )
                typer.echo(f"Claude exited with status {completed.returncode}", err=True)
                typer.echo(f"Running Codex with prompt: {resolved_prompt}", err=True)
                completed = subprocess.run(
                    ["codex", "exec", resolved_prompt],
                    cwd=cwd,
                    check=False,
                )
                typer.echo(f"Codex exited with status {completed.returncode}", err=True)
            except FileNotFoundError:
                typer.echo(
                    f'Could not find "{claude_bin}". Is the Claude CLI installed and on PATH? '
                    "You can also set $CLAUDE_BIN.",
                    err=True,
                )
                raise typer.Exit(code=127) from None

            if completed.returncode != 0:
                typer.echo(
                    f"Claude exited with status {completed.returncode}.",
                    err=True,
                )

            if no_change_exit_after:
                current_head = _head_sha(cwd)
                current_fingerprint = _git_state_fingerprint(cwd)

                # NOTE: We do NOT stop when HEAD changes; a HEAD change resets the streak.
                if current_head == previous_head and current_fingerprint == previous_fingerprint:
                    no_change_streak += 1
                    if no_change_streak >= no_change_exit_after:
                        typer.echo(
                            f"No git change for {no_change_streak} consecutive iterations; stopping."
                        )
                        return
                else:
                    previous_head = current_head
                    previous_fingerprint = current_fingerprint
                    no_change_streak = 0
    except KeyboardInterrupt:
        # Match typical shell behavior for Ctrl+C.
        typer.echo("", err=True)
        raise typer.Exit(code=130) from None


if __name__ == "__main__":
    typer.run(cli)
