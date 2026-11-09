"""Core admits only its listed dependencies, from the public registries only."""
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
import check_dependencies  # noqa: E402

CRATE = "crates/allowit-core/Cargo.toml"


class Dependencies(unittest.TestCase):
    def setUp(self):
        self.root = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.root)
        for name in ("Cargo.toml", "Cargo.lock", "package.json", "pnpm-lock.yaml", CRATE,
                     "scripts/locked-crates.txt"):
            (self.root / name).parent.mkdir(parents=True, exist_ok=True)
            shutil.copy(ROOT / name, self.root / name)

    def append(self, name, text):
        with open(self.root / name, "a") as f:
            f.write(text)

    def replace(self, name, old, new):
        path = self.root / name
        text = path.read_text()
        self.assertIn(old, text)
        path.write_text(text.replace(old, new, 1))

    def errors(self):
        return check_dependencies.check_root(self.root)[0]

    def test_repository_passes(self):
        self.assertEqual(check_dependencies.check_root(ROOT)[0], [])

    def test_unlisted_dependency_rejects(self):
        self.append(CRATE, 'ethers = "2"\n')
        self.assertIn("dependency is not admitted", "\n".join(self.errors()))

    def test_renamed_dependency_rejects(self):
        self.append(CRATE, 'serde2 = { package = "alloy", version = "1" }\n')
        self.assertIn("dependency is not admitted", "\n".join(self.errors()))

    def test_listed_name_with_path_source_rejects(self):
        self.replace(CRATE, 'url = { version = "2.5.7"', 'url = { path = "../allowit-engine"')
        self.assertIn("'path' sources", "\n".join(self.errors()))

    def test_git_source_rejects(self):
        self.replace(CRATE, 'url = { version = "2.5.7"',
                     'url = { git = "https://github.com/allowit-hq/allowit-engine"')
        self.assertIn("'git' sources", "\n".join(self.errors()))

    def test_target_dev_dependency_rejects(self):
        self.append(CRATE, '\n[target.\'cfg(unix)\'.dev-dependencies]\nrevm = "1"\n')
        self.assertIn("revm: dependency is not admitted", "\n".join(self.errors()))

    def test_source_override_rejects(self):
        self.append("Cargo.toml", '\n[patch.crates-io]\nserde = { path = "../serde" }\n')
        self.assertIn("[patch] source overrides", "\n".join(self.errors()))

    def test_locked_git_source_rejects(self):
        self.replace("Cargo.lock", 'source = "registry+https://github.com/rust-lang/crates.io-index"',
                     'source = "git+https://github.com/allowit-hq/allowit-engine#abc"')
        self.assertIn("only crates.io sources", "\n".join(self.errors()))

    def test_locked_local_package_rejects(self):
        self.append("Cargo.lock", '\n[[package]]\nname = "bridge"\nversion = "0.1.0"\n')
        self.assertIn("bridge: local package is not a Core member", "\n".join(self.errors()))

    def test_unlisted_locked_crate_rejects(self):
        self.append("Cargo.lock", '\n[[package]]\nname = "revm"\nversion = "1.0.0"\n'
                    'source = "registry+https://github.com/rust-lang/crates.io-index"\n')
        self.assertIn("revm: locked crate is not admitted", "\n".join(self.errors()))

    def test_npm_unlisted_or_remote_dependency_rejects(self):
        self.replace("package.json", '"typescript": "5.9.3"',
                     '"typescript": "github:microsoft/TypeScript", "web3": "1.0.0"')
        errors = "\n".join(self.errors())
        self.assertIn("devDependencies.web3: dependency is not admitted", errors)
        self.assertIn("devDependencies.typescript: only registry versions", errors)

    def test_locked_npm_package_rejects(self):
        self.append("pnpm-lock.yaml", "\n  web3@1.0.0: {}\n")
        self.assertIn("web3: locked npm package is not admitted", "\n".join(self.errors()))

    def test_locked_npm_remote_resolution_rejects(self):
        self.replace("pnpm-lock.yaml", "resolution: {integrity:", "resolution: {tarball: x, other:")
        self.assertIn("only registry resolutions", "\n".join(self.errors()))

    def test_npm_overrides_reject(self):
        self.replace("package.json", '"devDependencies"', '"overrides": {}, "devDependencies"')
        self.assertIn("'overrides' is not permitted", "\n".join(self.errors()))


if __name__ == "__main__":
    unittest.main()
