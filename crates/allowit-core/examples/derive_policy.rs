//! Print the on-chain part of a policy source file, then what only the server enforces.
use allowit_core::policy_source::derive;

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: derive_policy POLICY.rs");
    let source = std::fs::read_to_string(path).expect("read the policy source");
    match derive(&source) {
        Ok(derived) => {
            print!("{}", derived.source);
            eprintln!("left: {:?}", derived.left);
            eprintln!("kept: {:?}", derived.kept);
            let figures: Vec<&str> = derived.figures.iter().map(|figure| figure.key).collect();
            eprintln!("figures: {figures:?}");
            for check in derived.server_only {
                eprintln!(
                    "server only: line {} {} ({})",
                    check.line,
                    check.check,
                    check.cause.code()
                );
            }
        }
        Err(error) => {
            eprintln!("refused: {error}");
            std::process::exit(1);
        }
    }
}
