from __future__ import annotations

from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "check-docs.py"


class CheckDocsTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="check-docs-")
        self.root = Path(self.temporary.name)
        (self.root / "tools").mkdir()
        shutil.copy(SCRIPT, self.root / "tools/check-docs.py")
        (self.root / "docs").mkdir()
        (self.root / "docs/README.md").write_text("See [guide](guide.md).\n")
        (self.root / "docs/guide.md").write_text("A guide.\n")

    def tearDown(self):
        self.temporary.cleanup()

    def check(self):
        return subprocess.run([sys.executable, str(self.root / "tools/check-docs.py")], capture_output=True, text=True)

    def test_nested_checkouts_are_not_checked_as_this_checkout(self):
        for name, marker in [("linked", "file"), ("cloned", "directory")]:
            nested = self.root / ".claude/worktrees" / name
            (nested / "docs").mkdir(parents=True)
            if marker == "file": (nested / ".git").write_text("gitdir: /elsewhere\n")
            else: (nested / ".git").mkdir()
            (nested / "docs/README.md").write_text("Its own [missing](absent.md) link.\n")
        result = self.check()
        self.assertEqual(result.returncode, 0, result.stdout)
        self.assertIn("Checked 2 maintained", result.stdout)

    def test_broken_link_in_this_checkout_still_fails(self):
        (self.root / ".claude/worktrees/linked").mkdir(parents=True)
        (self.root / ".claude/worktrees/linked/.git").write_text("gitdir: /elsewhere\n")
        (self.root / "docs/guide.md").write_text("A [broken](absent.md) link.\n")
        result = self.check()
        self.assertEqual(result.returncode, 1)
        self.assertIn("docs/guide.md: missing link absent.md", result.stdout)


if __name__ == "__main__":
    unittest.main()
