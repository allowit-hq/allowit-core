"""Check the shared corpus and the actual direct-rustc host build boundary."""
import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
import policy_source

SPEC = importlib.util.spec_from_file_location("source_vectors", ROOT / "scripts/policy-source-vectors.py")
vectors = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(vectors)


class PolicySourceTests(unittest.TestCase):
    def test_gate_vectors_match_policy_source(self):
        self.assertEqual(vectors.GATE_FILE.read_text(), vectors.dump(vectors.gate_vectors()))

    def test_host_compile_uses_the_portable_std_backend(self):
        policy_source.host_compile(ROOT / "tests/fixtures/portable-policy.rs")
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "policy.rs"
            path.write_text(vectors.CANONICAL.replace("ctx: &AllowitContext", "ctx: &mut AllowitContext"))
            with self.assertRaisesRegex(ValueError, "fails the std-only compile"):
                policy_source.host_compile(path)


if __name__ == "__main__":
    unittest.main()
