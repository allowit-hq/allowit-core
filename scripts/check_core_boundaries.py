#!/usr/bin/env python3
"""Reject declared or locked native rail dependencies in Core."""
import json
import re
import sys
import subprocess
import tarfile
import tomllib
from pathlib import Path
from urllib.parse import urlsplit

RUST_PREFIXES = (
    "soroban-", "stellar-", "solana-", "spl-", "ethers-", "alloy-", "revm-", "evm-",
    "anchor-lang-", "anchor-spl-", "anchor-client-", "anchor-attribute-", "anchor-derive-",
    "allowit-stellar", "allowit-solana", "allowit-tempo", "allowit-vendor-",
)
RUST_NAMES = {"soroban", "stellar", "solana", "ethers", "alloy", "revm", "evm", "web3",
              "anchor-lang", "anchor-spl", "anchor-client", "anchor-syn",
              "allowit-sdk-engine", "allowit-engine", "allowit-server", "allowit-agent", "allowit-cli"}
NPM_PREFIXES = ("@stellar/", "@solana/", "@ethersproject/", "@ethereumjs/", "@allowit/vendor-")
NPM_NAMES = {"ethers", "web3", "viem", "@coral-xyz/anchor", "@project-serum/anchor", "@allowit/vendor",
             "stellar-sdk", "stellar-base", "soroban-client", "@allowit/ui", "@allowit/app",
             "@allowit/agent", "@allowit/server", "@allowit/cli", "@allowit/sdk-engine"}
DEP_TABLES = ("dependencies", "build-dependencies", "dev-dependencies")
NPM_TABLES = ("dependencies", "devDependencies", "optionalDependencies", "peerDependencies")


def safe(value):
    return re.sub(r"[^A-Za-z0-9@_./-]", "?", str(value))[:100]


def native_rust(name):
    name = name.lower().replace("_", "-")
    return name in RUST_NAMES or name.startswith(RUST_PREFIXES)


def native_npm(name):
    name = name.lower()
    return name in NPM_NAMES or name.startswith(NPM_PREFIXES)


def native_repository(source):
    if str(source).startswith("registry+"):
        return False
    source = str(source).removeprefix("git+")
    if source.startswith("github:"):
        source = "https://github.com/" + source[len("github:"):]
    elif re.match(r"^[^/@]+@[^/:]+:", source):
        source = "ssh://" + source.replace(":", "/", 1)
    try:
        parsed = urlsplit(source)
    except ValueError:
        return True
    pieces = parsed.path.rstrip("/").lower().removesuffix(".git").split("/")
    if len(pieces) < 2:
        return False
    owner, repo = pieces[-2:]
    if owner == "allowit-hq" and repo != "allowit-core":
        return True
    repo = re.sub(r"^(rs|js)-", "", repo)
    return (repo.startswith(("vendor-", "allowit-contracts-", "allowit-vendor-"))
            or native_rust(repo)
            or (owner, repo) in {("coral-xyz", "anchor"), ("project-serum", "anchor"),
                                 ("anza-xyz", "agave"), ("gakonst", "ethers-rs")}
            or owner == "alloy-rs")


def check_root(root):
    root = Path(root).resolve()
    errors, seen_cargo, seen_npm = [], set(), set()

    def problem(manifest, item, reason):
        try:
            path = manifest.relative_to(root)
        except ValueError:
            path = "external manifest"
        errors.append(f"{safe(path)} [{safe(item)}]: {reason}")

    def inside(path, manifest, item):
        try:
            resolved = path.resolve()
            resolved.relative_to(root)
            return resolved
        except (ValueError, OSError, RuntimeError):
            problem(manifest, item, "local dependency or member escapes Core")
            return None

    def read(manifest, kind):
        try:
            text = manifest.read_text(encoding="utf-8")
            result = tomllib.loads(text) if kind == "toml" else json.loads(text)
            if not isinstance(result, dict):
                raise ValueError("manifest must be an object")
            return result
        except (OSError, ValueError):
            problem(manifest, "manifest", f"cannot read valid {kind} manifest")
            return {}

    cargo_root = root / "Cargo.toml"
    root_doc = read(cargo_root, "toml")
    workspace = root_doc.get("workspace", {})
    inherited = workspace.get("dependencies", {})

    def dependency(manifest, alias, spec):
        if isinstance(spec, str):
            spec = {}
        if not isinstance(spec, dict):
            problem(manifest, alias, "invalid dependency declaration")
            return
        from_workspace = spec.get("workspace") is True
        if from_workspace:
            base = inherited.get(alias)
            if base is None:
                problem(manifest, alias, "workspace dependency is not declared")
                return
            if set(spec) - {"workspace", "features", "default-features", "optional"}:
                problem(manifest, alias, "workspace source identity cannot be overridden")
            spec = {} if isinstance(base, str) else base
        actual = spec.get("package", alias)
        if native_rust(actual):
            problem(manifest, alias, "native rail package cannot enter Core")
        if "git" in spec and native_repository(spec["git"]):
            problem(manifest, alias, "vendor or implementation repository cannot enter Core")
        if "path" in spec:
            origin = root if from_workspace else manifest.parent
            target = inside(origin / spec["path"] / "Cargo.toml", manifest, alias)
            if target:
                cargo_manifest(target)

    def cargo_manifest(manifest):
        manifest = inside(manifest, manifest, "package")
        if manifest is None or manifest in seen_cargo:
            return
        seen_cargo.add(manifest)
        doc = root_doc if manifest == cargo_root else read(manifest, "toml")
        if doc.get("patch") or doc.get("replace"):
            problem(manifest, "source override", "Core dependency source overrides are not permitted")
        name = doc.get("package", {}).get("name")
        if name and native_rust(name):
            problem(manifest, name, "resolved path package is a native rail package")
        for table in DEP_TABLES:
            for alias, spec in doc.get(table, {}).items():
                dependency(manifest, alias, spec)
        for tables in doc.get("target", {}).values():
            for table in DEP_TABLES:
                for alias, spec in tables.get(table, {}).items():
                    dependency(manifest, alias, spec)

    cargo_manifest(cargo_root)
    for alias, spec in inherited.items():
        dependency(cargo_root, alias, spec)
    excluded = {p.resolve() for pattern in workspace.get("exclude", []) for p in root.glob(pattern)}
    for pattern in workspace.get("members", []):
        matches = list(root.glob(pattern))
        if not matches:
            problem(cargo_root, "workspace member", "declared member does not exist")
        for member in matches:
            if member.resolve() not in excluded:
                cargo_manifest(member / "Cargo.toml")
    lock = root / "Cargo.lock"
    if lock.exists():
        for package in read(lock, "toml").get("package", []):
            if native_rust(package["name"]) or native_repository(package.get("source", "")):
                problem(lock, package["name"], "locked native or vendor package cannot enter Core")

    def npm_source(manifest, alias, source):
        if not isinstance(source, str):
            problem(manifest, alias, "invalid npm dependency declaration")
            return
        actual = alias.rsplit(">", 1)[-1].strip()
        marker = actual.find("@", 1) if actual.startswith("@") else actual.find("@")
        if marker >= 0:
            actual = actual[:marker]
        if source.startswith("npm:"):
            match = re.fullmatch(r"npm:(@[^/@]+/[^@]+|[^@]+)(?:@.*)?", source)
            if not match:
                problem(manifest, alias, "cannot inspect npm alias identity")
                return
            actual = match[1]
        if native_npm(actual) or native_repository(source):
            problem(manifest, alias, "native or vendor npm package cannot enter Core")
        if source.startswith("catalog:"):
            problem(manifest, alias, "catalog alias requires an inspectable package identity")
        if source.startswith(("file:", "link:", ".", "/")):
            local = source.split(":", 1)[1] if source.startswith(("file:", "link:")) else source
            target = inside(manifest.parent / local, manifest, alias)
            if target and target.is_file():
                try:
                    with tarfile.open(target) as archive:
                        info = json.load(archive.extractfile("package/package.json"))
                    if native_npm(info.get("name", "")):
                        problem(manifest, alias, "resolved archive is a native rail package")
                except (OSError, ValueError, KeyError, tarfile.TarError):
                    problem(manifest, alias, "cannot inspect local npm archive identity")
            elif target:
                npm_manifest(target / "package.json")

    def npm_overrides(manifest, overrides):
        for alias, source in overrides.items():
            if isinstance(source, dict):
                npm_source(manifest, alias, "0")
                npm_overrides(manifest, source)
            else:
                npm_source(manifest, alias, source)

    def npm_manifest(manifest):
        manifest = inside(manifest, manifest, "package")
        if manifest is None or manifest in seen_npm:
            return
        seen_npm.add(manifest)
        doc = read(manifest, "json")
        if native_npm(doc.get("name", "")):
            problem(manifest, "package", "resolved package is a native rail package")
        for table in NPM_TABLES:
            for alias, source in doc.get(table, {}).items():
                npm_source(manifest, alias, source)
        npm_overrides(manifest, doc.get("overrides", {}))
        pnpm = doc.get("pnpm", {})
        npm_overrides(manifest, pnpm.get("overrides", {}))
        for selector, extension in pnpm.get("packageExtensions", {}).items():
            npm_source(manifest, selector, "0")
            for table in NPM_TABLES:
                for alias, source in extension.get(table, {}).items():
                    npm_source(manifest, alias, source)
        members = doc.get("workspaces", [])
        if isinstance(members, dict):
            members = members.get("packages", [])
        for pattern in members:
            for member in manifest.parent.glob(pattern):
                if "node_modules" not in member.parts and (member / "package.json").is_file():
                    npm_manifest(member / "package.json")

    package = root / "package.json"
    if package.exists():
        npm_manifest(package)
    pnpm_workspace = root / "pnpm-workspace.yaml"
    if pnpm_workspace.exists():
        try:
            result = subprocess.run(
                ["pnpm", "list", "--recursive", "--depth", "-1", "--json"], cwd=root,
                capture_output=True, text=True, timeout=30, check=True,
            )
            for item in json.loads(result.stdout):
                npm_manifest(Path(item["path"]) / "package.json")
            # Catalog aliases need their resolved identities, not their display keys.
            if any(line.strip().startswith(("catalog:", "catalogs:")) for line in pnpm_workspace.read_text().splitlines()):
                problem(pnpm_workspace, "catalog", "catalog sources need explicit dependency identities for inspection")
        except (OSError, ValueError, KeyError, subprocess.SubprocessError):
            problem(pnpm_workspace, "workspace", "cannot inspect declared pnpm package paths")
    return sorted(set(errors))


if __name__ == "__main__":
    errors = check_root(Path(sys.argv[1]) if len(sys.argv) > 1 else Path(__file__).resolve().parent.parent)
    if errors:
        print("Core dependency boundary failed:")
        for error in errors:
            print(f"- {error}")
        raise SystemExit(1)
    print("Core dependency boundary passes")
