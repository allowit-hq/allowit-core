//! Rust and Python must reject the same nonportable source and derive the same prefix.
use allowit_core::policy_source::source_shape;
use serde_json::Value;

const CANONICAL: &str = include_str!("fixtures/portable-policy.rs");

fn vectors(text: &str) -> Vec<Value> {
    serde_json::from_str(text).expect("vector file is JSON")
}

#[test]
fn source_gate_matches_policy_source_py() {
    let vectors = vectors(include_str!("fixtures/policy-source-vectors.json"));
    assert!(vectors.len() > 300);
    let mut accepted = 0;
    for v in &vectors {
        let head = v["head"].as_u64().unwrap() as usize;
        let tail = v["tail"].as_u64().unwrap() as usize;
        let source = format!(
            "{}{}{}",
            &CANONICAL[..head],
            v["middle"].as_str().unwrap(),
            &CANONICAL[CANONICAL.len() - tail..]
        );
        let shape = source_shape(&source);
        if v["accepted"].as_bool().unwrap() {
            accepted += 1;
            let shape =
                shape.unwrap_or_else(|e| panic!("Python accepts, Rust rejects: {e}\n{source}"));
            let prefix: Vec<String> = serde_json::from_value(v["prefix"].clone()).unwrap();
            assert_eq!(shape.prefix, prefix, "{source}");
            assert_eq!(
                shape.writes_state,
                v["writes_state"].as_bool().unwrap(),
                "{source}"
            );
        } else {
            assert!(
                shape.is_err(),
                "Python rejects ({}), Rust accepts:\n{source}",
                v["error"]
            );
        }
    }
    assert!(accepted > 100 && accepted < vectors.len());
}

#[test]
fn packaged_resources_compile_and_reject_against_their_own_portable_library() {
    use allowit_core::policy_source::{
        PORTABLE_CORE_RS, PORTABLE_LIB_RS, PORTABLE_TERMS, PYTHON_HELPER, PYTHON_HELPER_TERMS,
    };
    use std::{fs, process::Command, time::SystemTime};

    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("allowit-resource-{}-{nonce}", std::process::id()));
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(&directory).unwrap();
    struct RemoveDirectory(std::path::PathBuf);
    impl Drop for RemoveDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = RemoveDirectory(directory.clone());
    for subdirectory in ["scripts", "crates/allowit-core/src/portable"] {
        fs::create_dir_all(directory.join(subdirectory)).unwrap();
    }
    for (path, bytes) in [
        ("scripts/policy_source.py", PYTHON_HELPER),
        ("scripts/LICENSE", PYTHON_HELPER_TERMS),
        ("crates/allowit-core/src/portable/lib.rs", PORTABLE_LIB_RS),
        ("crates/allowit-core/src/portable/core.rs", PORTABLE_CORE_RS),
        ("crates/allowit-core/src/portable/LICENSE", PORTABLE_TERMS),
        ("policy.rs", CANONICAL),
    ] {
        fs::write(directory.join(path), bytes).unwrap();
    }
    fs::write(
        directory.join("invalid.rs"),
        CANONICAL.replace("ctx: &AllowitContext", "ctx: &mut AllowitContext"),
    )
    .unwrap();
    let output = Command::new("python3")
        .arg("-c")
        .arg(
            r#"import sys
from pathlib import Path
root = Path(sys.argv[1])
sys.path.insert(0, str(root / 'scripts'))
import policy_source
policy_source.standard_prefix((root / 'policy.rs').read_text())
policy_source.host_compile(root / 'policy.rs')
try:
    policy_source.host_compile(root / 'invalid.rs')
except ValueError:
    pass
else:
    raise AssertionError('invalid source compiled')
"#,
        )
        .arg(&directory)
        .output()
        .expect("the packaged source check requires python3 and rustc");
    assert!(
        output.status.success(),
        "packaged helper failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
