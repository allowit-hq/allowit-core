"""Exercise actual dependency declarations that default feature trees can hide."""
import io
import json
import sys
import tarfile
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
import check_core_boundaries as boundary

PACKAGE = '[package]\nname="core_fixture"\nversion="0.1.0"\nedition="2024"\n'


class CoreBoundaryTests(unittest.TestCase):
    def test_optional_aliases_and_every_cargo_dependency_table_are_checked(self):
        declarations = [
            '[dependencies]\nbridge={package="soroban-sdk",version="1",optional=true}\n',
            '[dev-dependencies]\nbridge={package="stellar-xdr",version="1"}\n',
            '[build-dependencies]\nbridge={package="alloy-primitives",version="1"}\n',
            '[target.\'cfg(target_arch="wasm32")\'.dependencies]\nbridge={package="solana-program",version="1",optional=true}\n',
            '[workspace.dependencies]\nunused={package="anchor-lang",version="1"}\n',
            '[patch.crates-io]\nbridge={package="soroban-sdk",git="https://example.invalid/native"}\n',
        ]
        for declaration in declarations:
            with self.subTest(declaration=declaration), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                (root / "Cargo.toml").write_text(PACKAGE + declaration)
                self.assertTrue(boundary.check_root(root))

    def test_workspace_inheritance_resolved_path_names_and_symlink_escapes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / "core"
            root.mkdir()
            (root / "member").mkdir()
            (root / "Cargo.toml").write_text(PACKAGE + '[workspace]\nmembers=["member"]\n[workspace.dependencies]\nbridge={package="stellar-strkey",version="1"}\n')
            (root / "member/Cargo.toml").write_text(PACKAGE.replace('core_fixture', 'member') + '[dependencies]\nbridge={workspace=true,optional=true}\n')
            self.assertTrue(boundary.check_root(root))
            (root / "Cargo.toml").write_text(PACKAGE + '[dependencies]\nbridge={path="member"}\n')
            (root / "member/Cargo.toml").write_text(PACKAGE.replace('core_fixture', 'solana-sdk'))
            self.assertTrue(boundary.check_root(root))
            (root / "member/Cargo.toml").unlink()
            (root / "member").rmdir()
            outside = Path(directory) / "outside"
            outside.mkdir()
            (outside / "Cargo.toml").write_text(PACKAGE)
            (root / "member").symlink_to(outside, target_is_directory=True)
            self.assertTrue(any("escapes Core" in error for error in boundary.check_root(root)))

    def test_locked_transitives_and_git_credentials_do_not_hide_native_sources(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "Cargo.toml").write_text(PACKAGE)
            (root / "Cargo.lock").write_text('version=4\n[[package]]\nname="revm"\nversion="1.0.0"\nsource="registry+https://github.com/rust-lang/crates.io-index"\n')
            self.assertTrue(boundary.check_root(root))
            (root / "Cargo.lock").unlink()
            # Dummy credentials verify diagnostics do not echo a source URL.
            secret = "not" + "-a-real-password"
            url = "https://reader:" + secret + "@github.com/allowit-hq/vendor-example.git"
            (root / "Cargo.toml").write_text(PACKAGE + '[dependencies]\nbridge={git=' + json.dumps(url) + '}\n')
            errors = boundary.check_root(root)
            self.assertTrue(errors)
            self.assertNotIn(secret, str(errors))
            self.assertNotIn(url, str(errors))

    def test_npm_aliases_optional_declarations_workspaces_and_archive_identity(self):
        manifests = [
            {"dependencies": {"bridge": "npm:@stellar/stellar-sdk@1"}},
            {"optionalDependencies": {"bridge": "npm:ethers@6"}},
            {"devDependencies": {"@solana/kit": "1"}},
            {"peerDependencies": {"bridge": "github:coral-xyz/anchor"}},
            {"overrides": {"tool": {"dependency": "npm:@solana/kit@1"}}},
            {"pnpm": {"overrides": {"tool@1": "npm:ethers@6"}}},
            {"pnpm": {"packageExtensions": {"tool@1": {"optionalDependencies": {"bridge": "npm:@stellar/stellar-sdk@1"}}}}},
        ]
        for manifest in manifests:
            with self.subTest(manifest=manifest), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                (root / "Cargo.toml").write_text(PACKAGE)
                (root / "package.json").write_text(json.dumps(manifest))
                self.assertTrue(boundary.check_root(root))
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "Cargo.toml").write_text(PACKAGE)
            (root / "packages/adapter").mkdir(parents=True)
            (root / "package.json").write_text(json.dumps({"workspaces": ["packages/*"]}))
            (root / "packages/adapter/package.json").write_text(json.dumps({"name": "@solana/kit"}))
            self.assertTrue(boundary.check_root(root))
            (root / "package.json").write_text(json.dumps({"dependencies": {"bridge": "file:adapter.tgz"}}))
            with tarfile.open(root / "adapter.tgz", "w:gz") as archive:
                payload = json.dumps({"name": "@stellar/stellar-sdk"}).encode()
                entry = tarfile.TarInfo("package/package.json")
                entry.size = len(payload)
                archive.addfile(entry, io.BytesIO(payload))
            self.assertTrue(boundary.check_root(root))

    def test_workspace_inspection_does_not_execute_hooks_and_rejects_unknown_yaml(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "Cargo.toml").write_text(PACKAGE)
            (root / "package.json").write_text('{"name":"safe-wrapper","version":"1.0.0"}')
            (root / ".pnpmfile.cjs").write_text(
                "require('node:fs').writeFileSync(require('node:path').join(__dirname,'hook-ran'),'yes'); module.exports={};"
            )
            workspace = root / "pnpm-workspace.yaml"
            workspace.write_text("packages:\n  - '.'\n")
            self.assertEqual(boundary.check_root(root), [])
            self.assertFalse((root / "hook-ran").exists())
            (root / "packages/native").mkdir(parents=True)
            (root / "packages/native/package.json").write_text('{"name":"@solana/kit"}')
            workspace.write_text("packages:\n  - 'packages/*'\n")
            self.assertTrue(boundary.check_root(root))
            workspace.write_text("packages: []\ncatalog:\n  bridge: 'npm:@solana/kit@1'\n")
            self.assertTrue(boundary.check_root(root))
            self.assertFalse((root / "hook-ran").exists())

    def test_safe_archive_name_does_not_hide_nested_dependency_sources(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "Cargo.toml").write_text(PACKAGE)
            (root / "package.json").write_text('{"dependencies":{"bridge":"file:wrapper.tgz"}}')
            for declaration, rejected in [
                ({"dependencies": {"bridge": "npm:@noble/ed25519@2"}}, False),
                ({"dependencies": {"bridge": "npm:@stellar/stellar-sdk@1"}}, True),
                ({"pnpm": {"overrides": {"tool": {"bridge": "npm:@solana/kit@1"}}}}, True),
                ({"dependencies": {"bridge": "file:uninspectable.tgz"}}, True),
            ]:
                with self.subTest(declaration=declaration):
                    payload = json.dumps({"name": "safe-wrapper", **declaration}).encode()
                    with tarfile.open(root / "wrapper.tgz", "w:gz") as archive:
                        entry = tarfile.TarInfo("package/package.json")
                        entry.size = len(payload)
                        archive.addfile(entry, io.BytesIO(payload))
                    self.assertEqual(bool(boundary.check_root(root)), rejected)

    def test_external_metadata_symlinks_are_rejected_before_open(self):
        documents = {
            "Cargo.toml": PACKAGE,
            "Cargo.lock": "version=4\n",
            "package.json": "{}",
            "pnpm-workspace.yaml": "packages:\n  - '.'\n",
            "member/Cargo.toml": PACKAGE,
            "wrapper.tgz": "not read",
        }
        for filename, text in documents.items():
            with self.subTest(filename=filename), tempfile.TemporaryDirectory() as directory:
                root = Path(directory) / "core"
                root.mkdir()
                (root / "Cargo.toml").write_text(PACKAGE + '[dependencies]\nbridge={path="member"}\n' if filename.startswith("member/") else PACKAGE)
                if filename == "wrapper.tgz":
                    (root / "package.json").write_text('{"dependencies":{"bridge":"file:wrapper.tgz"}}')
                external = Path(directory) / "external"
                external.write_text(text)
                target = root / filename
                target.parent.mkdir(parents=True, exist_ok=True)
                target.unlink(missing_ok=True)
                target.symlink_to(external)
                opened = []
                original = Path.open
                def observe(path, *args, **kwargs):
                    if path.resolve() == external.resolve():
                        opened.append(filename)
                    return original(path, *args, **kwargs)
                with patch.object(Path, "open", observe), patch.object(boundary.tarfile, "open", side_effect=AssertionError("archive must not open")):
                    errors = boundary.check_root(root)
                self.assertTrue(any("escapes Core" in error for error in errors))
                self.assertEqual(opened, [])

    def test_generic_primitives_and_unreferenced_fixture_manifests_remain_allowed(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            dependencies = '\n'.join(f'{name}="1"' for name in ["sha2", "ed25519-dalek", "ring", "ruint", "reqwest", "anchor"])
            (root / "Cargo.toml").write_text(PACKAGE + '[dependencies]\n' + dependencies)
            (root / "package.json").write_text(json.dumps({"devDependencies": {"typescript": "5.9.3", "@types/node": "24"}, "dependencies": {"@noble/ed25519": "2"}}))
            (root / "node_modules/foreign").mkdir(parents=True)
            (root / "node_modules/foreign/package.json").write_text(json.dumps({"name": "@stellar/stellar-sdk"}))
            (root / "tests/fixtures").mkdir(parents=True)
            (root / "tests/fixtures/Cargo.toml").write_text(PACKAGE.replace("core_fixture", "soroban-sdk"))
            self.assertEqual(boundary.check_root(root), [])


if __name__ == "__main__":
    unittest.main()
