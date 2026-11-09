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

    def test_host_compile_rejects_native_string_misuse_that_the_gate_admits(self):
        # Each body passes the gate and fails the native `String` build with this error code.
        rejected = {
            "let n: i128 = allowit::param(ctx, &key);": "E0308",
            "let n: i128 = allowit::arg(ctx, &key);": "E0308",
            "let n = ctx.params[&key].clone();": "E0308",
            "let n = ctx.params.contains(&key);": "E0308",
            "let n = ctx.request.args.contains(&key);": "E0308",
            "let n: Option<i128> = allowit::get_state(ctx, &key)?;": "E0308",
            "allowit::set_state(ctx, &key, 1i128)?;": "E0308",
            "let n: i128 = allowit::param(ctx, &*key);": "E0614",
            'panic!("over limit for {}", key);': "E0277",
            'let n = key == "limit";': "E0308",
            'let n = key + "_x";': "E0369",
            "let n: &str = key.as_ref();": "E0599",
            "let n = allowit::get_state::<String>(ctx, \"seen\")?.unwrap_or_default();": "E0277",
        }
        accepted = ('let other: String = allowit::param(ctx, "other_key"); '
                    'let seen: Option<String> = allowit::get_state(ctx, "seen")?; '
                    'if key < other || seen == Some(key.clone()) { panic!("repeated {:?}", key); } '
                    'allowit::set_state(ctx, "seen", key)?;')
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "policy.rs"
            for body, code in list(rejected.items()) + [(accepted, None)]:
                source = ("use allowit::prelude::*;\npub fn execute(ctx: &AllowitContext) -> allowit::Result {\n"
                          'let key: String = allowit::param(ctx, "limit_key");\n' + body + "\nOk(())\n}\n")
                path.write_text(source)
                with self.subTest(body=body):
                    policy_source.checked_code(source)
                    if code is None:
                        policy_source.host_compile(path)
                    else:
                        with self.assertRaisesRegex(ValueError, r"error\[" + code + r"\]"):
                            policy_source.host_compile(path)


if __name__ == "__main__":
    unittest.main()
