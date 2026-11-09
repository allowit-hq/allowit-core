"""Reject retired policy rule names and 'cap' wording in tracked files."""
import re
import subprocess
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SKIPPED = {"Cargo.lock", "pnpm-lock.yaml"}
RETIRED = [
    # The word "cap" or "caps" in any case, also inside snake_case or kebab-case names.
    re.compile(r"(?i)(?<![a-z])caps?(?![a-z])"),
    # "Cap" inside a camel-case name, such as `PaymentCapExceeded`.
    re.compile(r"(?<=[a-z])Caps?(?![a-z])"),
    # Retired param keys and their constants. Adopted names: destinations, expires_at, operations.
    re.compile(r"""(?i)["'`](recipients|expiry|ops)["'`]"""),
    re.compile(r"\b(RECIPIENTS|EXPIRY|OPS)\b"),
    re.compile(r"(?i)\bexpiry\b(?=\s*(rule|param|key|label|limit))"),
]


def tracked_text_files():
    names = subprocess.run(["git", "ls-files", "-z"], cwd=ROOT, check=True, capture_output=True).stdout
    for name in names.decode().split("\0"):
        path = ROOT / name
        if not name or Path(name).name in SKIPPED or not path.is_file():
            continue
        try:
            yield name, path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            continue


def hits(text):
    for number, line in enumerate(text.splitlines(), 1):
        if any(pattern.search(line) for pattern in RETIRED):
            yield number, line.strip()


class RuleNameTests(unittest.TestCase):
    def test_patterns_catch_retired_names_and_skip_unrelated_words(self):
        for line in ("Cap every payment", "transaction_cap", "DAY_TOTAL_CAP", "spending caps",
                     "PaymentCapExceeded", '"vendor_cap"', "cap-key", 'params["ops"]', "EXPIRY",
                     '"label": "Expiry"', "the expiry param"):
            with self.subTest(line=line):
                self.assertTrue(list(hits(line)))
        for line in ("capability", "capture", "capacity", "escape", "std::ops::Deref", "day_total",
                     "DAY_SPENT", "payment_limit", "expires_at", "operations"):
            with self.subTest(line=line):
                self.assertFalse(list(hits(line)))

    def test_tracked_files_use_adopted_rule_names(self):
        found = [f"{name}:{number}: {line[:120]}"
                 for name, text in tracked_text_files() if name != "scripts/tests/test_rule_names.py"
                 for number, line in hits(text)]
        self.assertEqual(found, [], "Retired rule name or 'cap' wording:\n" + "\n".join(found))


if __name__ == "__main__":
    unittest.main()
