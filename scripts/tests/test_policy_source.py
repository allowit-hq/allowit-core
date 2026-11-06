"""Check the shared corpus and the actual direct-rustc host build boundary."""
import importlib.util
import json
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

    def test_only_exact_registered_host_calls_extend_the_server_source_gate(self):
        file = ROOT / "crates/allowit-core/tests/fixtures/host-call-vectors.json"
        for vector in json.loads(file.read_text()):
            body = vector["body"]
            source = ("use allowit::prelude::*; pub fn execute(ctx: &AllowitContext) -> allowit::Result {"
                      "allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?; "
                      + body + " Ok(()) }")
            with self.subTest(body=body, calls=vector["calls"]):
                if vector["accepted"]:
                    self.assertEqual(policy_source.standard_prefix(source, host_calls=vector["calls"]),
                                     ["payment_limit"])
                    self.assertFalse(policy_source.writes_state(source, host_calls=vector["calls"]))
                else:
                    with self.assertRaises(ValueError):
                        policy_source.standard_prefix(source, host_calls=vector["calls"])
                if body.startswith("vendor::"):
                    with self.assertRaises(ValueError):
                        policy_source.standard_prefix(source)
        with self.assertRaises(ValueError):
            policy_source.standard_prefix(vectors.CANONICAL, host_calls=[("vendor", "quote")] * 129)
        oversized = "// " + "é" * 8000 + "\n" + vectors.CANONICAL
        policy_source.standard_prefix(oversized)
        with self.assertRaises(ValueError):
            policy_source.standard_prefix(oversized, host_calls=[("vendor", "quote")])

    def test_host_compile_uses_the_portable_std_backend(self):
        policy_source.host_compile(ROOT / "crates/allowit-core/tests/fixtures/portable-policy.rs")
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "policy.rs"
            path.write_text(vectors.CANONICAL.replace("ctx: &AllowitContext", "ctx: &mut AllowitContext"))
            with self.assertRaisesRegex(ValueError, "fails the std-only compile"):
                policy_source.host_compile(path)


if __name__ == "__main__":
    unittest.main()
