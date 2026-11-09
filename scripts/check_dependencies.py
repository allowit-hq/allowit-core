#!/usr/bin/env python3
"""Admit only Core's listed dependencies, from the public registries only.

Core depends on no other Allowit repository and on no rail package. An allowlist enforces this
without naming any rail: a new dependency needs an explicit entry here. The vendor-boundary eval
separately rejects rail names in every file, including the lockfiles.
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
NPM_FORBIDDEN = ("workspaces", "overrides", "resolutions", "bundleDependencies", "bundledDependencies")
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


def check_lock(path, doc, members):
    errors = []
    for package in doc.get("package", []):
        source = package.get("source")
        if source is None:
            if package.get("name") not in members:
                errors.append(f"{path} {package.get('name')}: local package is not a Core member")
        elif source != REGISTRY:
            errors.append(f"{path} {package.get('name')}: only crates.io sources are permitted")
    return errors


def check_npm(path, doc):
    errors = [f"{path}: '{key}' is not permitted" for key in NPM_FORBIDDEN if key in doc]
    if "overrides" in doc.get("pnpm", {}):
        errors.append(f"{path}: 'pnpm.overrides' is not permitted")
    for table in NPM_TABLES:
        for name, spec in doc.get(table, {}).items():
            if name not in NPM:
                errors.append(f"{path} {table}.{name}: dependency is not admitted")
            elif not isinstance(spec, str) or not NPM_VERSION.match(spec):
                errors.append(f"{path} {table}.{name}: only registry versions are permitted")
    return errors


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
        errors += check_lock("Cargo.lock", tomllib.loads((root / "Cargo.lock").read_text()), members)
    errors += check_npm("package.json", json.loads((root / "package.json").read_text()))
    if (root / "pnpm-workspace.yaml").exists():
        errors.append("pnpm-workspace.yaml: Core is a single npm package")
    return errors


def main():
    root = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(__file__).resolve().parent.parent
    errors = check_root(root)
    for error in errors:
        print(error, file=sys.stderr)
    print(f"check_dependencies: {len(errors)} errors")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
