#!/usr/bin/env python3
"""Admit only Core's listed dependencies, from the public registries only.

Core depends on no other Allowit repository and on no rail package. Allowlists enforce this
without naming any rail: a new direct dependency needs an entry in RUST or NPM, and a new locked
crate needs an entry in scripts/locked-crates.txt. `--write-locked` rewrites that file from
Cargo.lock for review.
"""
import json
import re
import sys
import tomllib
from pathlib import Path

RUST = frozenset((
    "serde", "serde_json", "sha2", "libc", "url", "reqwest", "zeroize", "time",
    "serde_json_canonicalizer",
))
NPM = frozenset(("typescript",))
REGISTRY = "registry+https://github.com/rust-lang/crates.io-index"
TABLES = ("dependencies", "dev-dependencies", "build-dependencies")
SOURCE_KEYS = ("path", "git", "registry", "registry-index", "branch", "tag", "rev")
NPM_TABLES = ("dependencies", "devDependencies", "optionalDependencies", "peerDependencies")
NPM_FORBIDDEN = ("workspaces", "overrides", "resolutions", "bundleDependencies", "bundledDependencies",
                 "pnpm")
RESOLUTION = re.compile(r"^\s+resolution: \{integrity: [A-Za-z0-9+/=-]+\}$")
NPM_VERSION = re.compile(r"^[\^~]?\d+(\.\d+){0,2}(-[0-9A-Za-z.-]+)?$")


def tables(doc):
    """Yield (where, table) for every dependency table of one Cargo manifest."""
    for name in TABLES:
        if name in doc:
            yield name, doc[name]
    for target, body in doc.get("target", {}).items():
        for name in TABLES:
            if name in body:
                yield f"target.{target}.{name}", body[name]
    if "dependencies" in doc.get("workspace", {}):
        yield "workspace.dependencies", doc["workspace"]["dependencies"]


def check_cargo(path, doc):
    errors = []
    for key in ("patch", "replace"):
        if key in doc or key in doc.get("workspace", {}):
            errors.append(f"{path}: [{key}] source overrides are not permitted")
    for where, table in tables(doc):
        for alias, spec in table.items():
            name = spec.get("package", alias) if isinstance(spec, dict) else alias
            if name not in RUST:
                errors.append(f"{path} {where}.{alias}: dependency is not admitted")
            if isinstance(spec, dict):
                for key in SOURCE_KEYS:
                    if key in spec:
                        errors.append(f"{path} {where}.{alias}: '{key}' sources are not permitted")
    return errors


def check_lock(path, doc, members, locked):
    errors = []
    for package in doc.get("package", []):
        source = package.get("source")
        if package.get("name") not in members and package.get("name") not in locked:
            errors.append(f"{path} {package.get('name')}: locked crate is not admitted")
        if source is None:
            if package.get("name") not in members:
                errors.append(f"{path} {package.get('name')}: local package is not a Core member")
        elif source != REGISTRY:
            errors.append(f"{path} {package.get('name')}: only crates.io sources are permitted")
    return errors


def check_npm(path, doc):
    errors = [f"{path}: '{key}' is not permitted" for key in NPM_FORBIDDEN if key in doc]
    for table in NPM_TABLES:
        for name, spec in doc.get(table, {}).items():
            if name not in NPM:
                errors.append(f"{path} {table}.{name}: dependency is not admitted")
            elif not isinstance(spec, str) or not NPM_VERSION.match(spec):
                errors.append(f"{path} {table}.{name}: only registry versions are permitted")
    return errors


def check_pnpm_lock(path, text):
    """Every locked npm package must be a listed dependency resolved from the registry."""
    errors, section = [], None
    for line in text.splitlines():
        if line and not line.startswith(" "):
            section = line.rstrip(":")
        elif section in ("packages", "snapshots") and re.match(r"^  \S", line):
            key = line.strip().rstrip(":").split(":")[0].strip("'\"")
            name = key[: key.index("@", 1)] if "@" in key[1:] else key
            if name not in NPM:
                errors.append(f"{path} {name}: locked npm package is not admitted")
        elif section == "packages" and "resolution:" in line and not RESOLUTION.match(line):
            errors.append(f"{path}: only registry resolutions are permitted")
    return errors


def locked_names(root, members):
    doc = tomllib.loads((root / "Cargo.lock").read_text())
    return sorted({p["name"] for p in doc.get("package", []) if p["name"] not in members})


def read_locked(root):
    path = root / "scripts" / "locked-crates.txt"
    return frozenset(path.read_text().split()) if path.exists() else frozenset()


def check_root(root):
    root = Path(root)
    errors = []
    workspace = tomllib.loads((root / "Cargo.toml").read_text())
    errors += check_cargo("Cargo.toml", workspace)
    members = set()
    for member in workspace.get("workspace", {}).get("members", []):
        if any(c in member for c in "*?[") or ".." in Path(member).parts:
            errors.append(f"Cargo.toml: member '{member}' must be a literal path inside Core")
            continue
        manifest = f"{member}/Cargo.toml"
        doc = tomllib.loads((root / manifest).read_text())
        members.add(doc["package"]["name"])
        errors += check_cargo(manifest, doc)
    if (root / "Cargo.lock").exists():
        errors += check_lock("Cargo.lock", tomllib.loads((root / "Cargo.lock").read_text()), members,
                             read_locked(root))
    errors += check_npm("package.json", json.loads((root / "package.json").read_text()))
    if (root / "pnpm-lock.yaml").exists():
        errors += check_pnpm_lock("pnpm-lock.yaml", (root / "pnpm-lock.yaml").read_text())
    if (root / "pnpm-workspace.yaml").exists():
        errors.append("pnpm-workspace.yaml: Core is a single npm package")
    return errors, members


def main():
    args = [a for a in sys.argv[1:] if a != "--write-locked"]
    root = Path(args[0]) if args else Path(__file__).resolve().parent.parent
    errors, members = check_root(root)
    if "--write-locked" in sys.argv:
        (root / "scripts" / "locked-crates.txt").write_text("\n".join(locked_names(root, members)) + "\n")
        errors, members = check_root(root)
    for error in errors:
        print(error, file=sys.stderr)
    print(f"check_dependencies: {len(errors)} errors")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
