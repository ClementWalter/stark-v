#!/usr/bin/env python3
# /// script
# requires-python = ">=3.10"
# ///
"""
Pre-commit hook: fail if a workspace member contains orphan Rust source files.

Definition (pragmatic):
- A `.rs` file under a package's `src/` directory must be reachable from at least one
  crate root (any target whose `src_path` is under that package's `src/`) by following
  `mod foo;` declarations (and nested inline `mod foo { ... }` blocks).

This catches the common footgun: adding `src/foo.rs` (or `src/foo/mod.rs`) but
forgetting to add `mod foo;` somewhere, so the file is never compiled.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Optional


@dataclass(frozen=True)
class ModuleFile:
    path: Path
    module_name: Optional[str]  # None for crate roots
    is_crate_root: bool


def _run_cargo_metadata(workspace_root: Path) -> dict:
    proc = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--no-deps"],
        cwd=str(workspace_root),
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    return json.loads(proc.stdout)


def _is_ident_start(ch: str) -> bool:
    return ch == "_" or ("a" <= ch <= "z") or ("A" <= ch <= "Z")


def _is_ident_continue(ch: str) -> bool:
    return _is_ident_start(ch) or ("0" <= ch <= "9")


def _skip_line_comment(code: str, i: int) -> int:
    # assumes code[i:i+2] == "//"
    j = code.find("\n", i + 2)
    return len(code) if j == -1 else j + 1


def _skip_block_comment(code: str, i: int) -> int:
    # Rust supports nested block comments.
    # assumes code[i:i+2] == "/*"
    depth = 1
    j = i + 2
    n = len(code)
    while j < n and depth > 0:
        if code.startswith("/*", j):
            depth += 1
            j += 2
        elif code.startswith("*/", j):
            depth -= 1
            j += 2
        else:
            j += 1
    return j


def _skip_string_literal(code: str, i: int) -> int:
    # Handles: "...", '...', r#"..."#, br#"..."#
    n = len(code)
    # Raw string?
    if code[i] in ("r", "b") and i + 1 < n:
        # raw byte string can be "br" or "rb" (accept both)
        if code.startswith("br", i) or code.startswith("rb", i):
            j = i + 2
            hashes = 0
            while j < n and code[j] == "#":
                hashes += 1
                j += 1
            if j < n and code[j] == '"':
                j += 1
                end = '"' + ("#" * hashes)
                k = code.find(end, j)
                return n if k == -1 else k + len(end)
        if code[i] == "r":
            j = i + 1
            hashes = 0
            while j < n and code[j] == "#":
                hashes += 1
                j += 1
            if j < n and code[j] == '"':
                j += 1
                end = '"' + ("#" * hashes)
                k = code.find(end, j)
                return n if k == -1 else k + len(end)

    # Normal string literal
    if code[i] == '"':
        j = i + 1
        while j < n:
            ch = code[j]
            if ch == "\\":
                j += 2
            elif ch == '"':
                return j + 1
            else:
                j += 1
        return n

    # Char literal (or byte char b'...')
    if code[i] == "'":
        j = i + 1
        while j < n:
            ch = code[j]
            if ch == "\\":
                j += 2
            elif ch == "'":
                return j + 1
            else:
                j += 1
        return n

    # byte string b"..."
    if code[i] == "b" and i + 1 < n and code[i + 1] == '"':
        return _skip_string_literal(code, i + 1)

    # byte char b'...'
    if code[i] == "b" and i + 1 < n and code[i + 1] == "'":
        return _skip_string_literal(code, i + 1)

    return i + 1


def _parse_attribute_block(code: str, i: int) -> tuple[str, int]:
    # assumes code[i:i+2] == "#["
    j = i + 2
    n = len(code)
    depth = 1
    buf: list[str] = []
    while j < n and depth > 0:
        if code.startswith("#[", j):
            # Nested attribute blocks are not expected; treat as raw text.
            buf.append(code[j])
            j += 1
            continue

        ch = code[j]
        if ch == "]":
            depth -= 1
            j += 1
            break
        if ch == '"':
            k = _skip_string_literal(code, j)
            buf.append(code[j:k])
            j = k
            continue
        if code.startswith("//", j):
            j = _skip_line_comment(code, j)
            continue
        if code.startswith("/*", j):
            j = _skip_block_comment(code, j)
            continue

        buf.append(ch)
        j += 1

    return ("".join(buf).strip(), j)


def _try_extract_path_attr(attr_inner: str) -> Optional[str]:
    # Only accept a plain `path = "..."` attribute (avoid matching cfg_attr etc).
    # Examples:
    #   #[path = "foo.rs"]
    #   #[path="dir/foo.rs"]
    s = attr_inner.strip()
    if not s.startswith("path"):
        return None
    s = s[len("path") :].lstrip()
    if not s.startswith("="):
        return None
    s = s[1:].lstrip()
    if not s.startswith('"'):
        return None
    # naive but sufficient for simple path attrs
    end = s.find('"', 1)
    if end == -1:
        return None
    return s[1:end]


def _module_base_dir(module_file: ModuleFile) -> Path:
    # Crate roots (targets) always resolve `mod foo;` relative to their directory.
    # File modules resolve:
    # - `.../mod.rs` => base dir is the file's directory
    # - `.../<name>.rs` => base dir is `<dir>/<name>/`
    if module_file.path.name == "mod.rs":
        return module_file.path.parent
    if module_file.is_crate_root:
        return module_file.path.parent
    if module_file.module_name is None:
        # Should not happen for non-crate-root modules, but be defensive.
        return module_file.path.parent
    return module_file.path.parent / module_file.module_name


def _resolve_mod_file(
    *,
    module_name: str,
    current_base_dir: Path,
    current_file_dir: Path,
    path_override: Optional[str],
) -> Optional[Path]:
    candidates: list[Path] = []
    if path_override:
        rel = Path(path_override)
        # Try both interpretations; rustc uses one, but this avoids brittle false positives.
        candidates.append((current_file_dir / rel).resolve())
        candidates.append((current_base_dir / rel).resolve())
    else:
        candidates.append((current_base_dir / f"{module_name}.rs").resolve())
        candidates.append((current_base_dir / module_name / "mod.rs").resolve())

    existing = [p for p in candidates if p.exists() and p.is_file()]
    if not existing:
        return None
    # If multiple exist, rustc would error; still pick the first deterministically.
    return existing[0]


def _discover_child_modules(code: str, module_file: ModuleFile, package_dir: Path) -> list[ModuleFile]:
    current_file_dir = module_file.path.parent.resolve()
    base_dir_stack: list[Path] = [_module_base_dir(module_file).resolve()]
    brace_stack: list[bool] = []  # True if this '{' started an inline module
    pending_path: Optional[str] = None

    out: list[ModuleFile] = []
    i = 0
    n = len(code)

    while i < n:
        ch = code[i]

        # Whitespace
        if ch.isspace():
            i += 1
            continue

        # Comments
        if code.startswith("//", i):
            i = _skip_line_comment(code, i)
            continue
        if code.startswith("/*", i):
            i = _skip_block_comment(code, i)
            continue

        # Strings / chars
        is_raw_str = code.startswith('r"', i) or code.startswith("r#", i)
        is_raw_byte_str = (
            code.startswith('br"', i)
            or code.startswith("br#", i)
            or code.startswith('rb"', i)
            or code.startswith("rb#", i)
        )
        if ch in ('"', "'") or code.startswith('b"', i) or code.startswith("b'", i) or is_raw_str or is_raw_byte_str:
            i = _skip_string_literal(code, i)
            continue

        # Attributes
        if code.startswith("#[", i):
            attr_inner, j = _parse_attribute_block(code, i)
            maybe_path = _try_extract_path_attr(attr_inner)
            if maybe_path is not None:
                pending_path = maybe_path
            i = j
            continue

        # Braces
        if ch == "{":
            brace_stack.append(False)
            i += 1
            continue
        if ch == "}":
            if brace_stack:
                is_inline_mod = brace_stack.pop()
                if is_inline_mod and len(base_dir_stack) > 1:
                    base_dir_stack.pop()
            i += 1
            continue

        # Identifiers / keywords
        if _is_ident_start(ch):
            start = i
            i += 1
            while i < n and _is_ident_continue(code[i]):
                i += 1
            ident = code[start:i]

            if ident != "mod":
                # Be conservative: clear pending `#[path]` if we see any real token that is
                # not a visibility prefix, to avoid accidentally leaking it.
                if ident not in ("pub", "crate", "super", "self", "in"):
                    pending_path = None
                continue

            # Parse module name
            while i < n and code[i].isspace():
                i += 1
            if i >= n or not _is_ident_start(code[i]):
                pending_path = None
                continue
            name_start = i
            i += 1
            while i < n and _is_ident_continue(code[i]):
                i += 1
            mod_name = code[name_start:i]

            # Determine mod kind (';' or inline '{')
            while i < n and code[i].isspace():
                i += 1
            if i >= n:
                pending_path = None
                continue

            if code[i] == ";":
                resolved = _resolve_mod_file(
                    module_name=mod_name,
                    current_base_dir=base_dir_stack[-1],
                    current_file_dir=current_file_dir,
                    path_override=pending_path,
                )
                pending_path = None
                i += 1

                if resolved is None:
                    continue
                try:
                    resolved_rel = resolved.resolve()
                except FileNotFoundError:
                    continue
                if package_dir.resolve() not in resolved_rel.parents and resolved_rel != package_dir.resolve():
                    continue
                out.append(ModuleFile(path=resolved_rel, module_name=mod_name, is_crate_root=False))
                continue

            if code[i] == "{":
                # Inline module: update base-dir context for nested `mod foo;` resolutions.
                base_dir_stack.append((base_dir_stack[-1] / mod_name).resolve())
                brace_stack.append(True)
                pending_path = None
                i += 1
                continue

            pending_path = None
            continue

        # Other punctuation
        i += 1

    return out


def _reachable_rs_files_for_package(pkg: dict, workspace_root: Path) -> set[Path]:
    manifest_path = Path(pkg["manifest_path"]).resolve()
    package_dir = manifest_path.parent
    src_dir = (package_dir / "src").resolve()

    roots: list[ModuleFile] = []
    for target in pkg.get("targets", []):
        src_path = Path(target["src_path"]).resolve()
        # Only crate roots under `src/` affect reachability of `src/**/*.rs` files.
        if src_dir in src_path.parents or src_path == src_dir:
            roots.append(ModuleFile(path=src_path, module_name=None, is_crate_root=True))

    visited: set[Path] = set()
    reachable: set[Path] = set()
    queue: list[ModuleFile] = list(roots)

    while queue:
        mf = queue.pop()
        p = mf.path.resolve()
        if p in visited:
            continue
        visited.add(p)
        if not p.exists() or not p.is_file():
            continue
        try:
            code = p.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            # Rust sources are utf-8; if this happens, ignore file rather than blocking commits.
            continue

        reachable.add(p)
        for child in _discover_child_modules(code, mf, package_dir):
            if child.path not in visited:
                queue.append(child)

    return reachable


def main() -> int:
    workspace_root = Path(os.getcwd()).resolve()
    metadata = _run_cargo_metadata(workspace_root)

    workspace_member_ids = set(metadata.get("workspace_members", []))
    packages = [p for p in metadata.get("packages", []) if p.get("id") in workspace_member_ids]

    # `cargo metadata` may treat vendored path dependencies (like git submodules under `external/`)
    # as "workspace members". We typically don't want to enforce repo-specific hygiene rules on
    # third-party code, so exclude anything living under `external/`.
    filtered_packages: list[dict] = []
    for p in packages:
        manifest_path = Path(p["manifest_path"]).resolve()
        try:
            rel = manifest_path.relative_to(workspace_root)
        except ValueError:
            # If it's outside the repo root, skip.
            continue
        if rel.parts and rel.parts[0] == "external":
            continue
        filtered_packages.append(p)
    packages = filtered_packages

    failures: list[str] = []
    for pkg in sorted(packages, key=lambda p: p.get("name", "")):
        manifest_path = Path(pkg["manifest_path"]).resolve()
        package_dir = manifest_path.parent
        src_dir = package_dir / "src"
        if not src_dir.exists():
            continue

        all_rs = {p.resolve() for p in src_dir.rglob("*.rs") if p.is_file()}
        reachable = _reachable_rs_files_for_package(pkg, workspace_root)

        orphans = sorted(p for p in all_rs if p not in reachable)
        if not orphans:
            continue

        rel_orphans = [str(p.relative_to(workspace_root)) for p in orphans if workspace_root in p.parents]
        if not rel_orphans:
            rel_orphans = [str(p) for p in orphans]

        failures.append(
            "\n".join(
                [
                    f"- package `{pkg.get('name')}` has orphan Rust files under `src/`:",
                    *[f"  - {rp}" for rp in rel_orphans],
                ]
            )
        )

    if failures:
        msg = "\n".join(
            [
                "Found Rust source files that are not reachable from any crate root via `mod ...;`.",
                "",
                *failures,
                "",
                "Fix: add the missing `mod <name>;` in the appropriate parent `mod.rs`/`lib.rs`,",
                "or delete/relocate the unused file.",
            ]
        )
        print(msg, file=sys.stderr)
        return 1

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
