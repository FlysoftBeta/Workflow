#!/usr/bin/env python3
"""Check maintained Markdown without rewriting historical originals or upstream licenses."""
from pathlib import Path
import re
import os
import sys
from urllib.parse import unquote

ROOT = Path(__file__).resolve().parents[1]
IGNORED = {".git", ".gradle", ".kotlin", ".idea", "build", "target", "node_modules", "__pycache__", ".cache", "artifacts"}
LINK = re.compile(r"\[[^\]]*\]\(([^\n]+?)\)")


def maintained(path):
    relative = path.relative_to(ROOT)
    if any(part in IGNORED for part in relative.parts): return False
    if relative.parts[:2] == ("docs", "archive"):
        return path.name == "README.md" and relative in {
            Path("docs/archive/README.md"), Path("docs/archive/implementation-1.0.0/README.md"),
            Path("docs/archive/experiments/README.md")}
    return True


def main():
    failures = []; count = 0
    paths = []
    for directory, dirs, files in os.walk(ROOT):
        parent = Path(directory)
        # A nested checkout, such as a linked worktree under .claude/worktrees, has its own documents and root.
        dirs[:] = [name for name in dirs if name not in IGNORED and not (parent / name / ".git").exists()]
        relative = parent.relative_to(ROOT)
        if relative == Path("docs/archive"):
            dirs[:] = [name for name in dirs if name in {"implementation-1.0.0", "experiments"}]
        elif relative.parts[:2] == ("docs", "archive"):
            dirs[:] = []
        paths.extend(parent / name for name in files if name.endswith(".md"))
    for path in sorted(paths):
        if not maintained(path): continue
        count += 1; text = path.read_text(encoding="utf-8")
        if text.count("```") % 2: failures.append(f"{path.relative_to(ROOT)}: unmatched fenced block")
        prose = re.sub(r"```.*?```", "", text, flags=re.S)
        prose = re.sub(r"`[^`]+`", "", prose)
        # Literal UI labels may be quoted; surrounding maintained prose should be English.
        prose = re.sub(r'“[^”]*”|「[^」]*」|"[^"\n]*"', "", prose)
        if re.search(r"[\u4e00-\u9fff]", prose): failures.append(f"{path.relative_to(ROOT)}: non-English maintained prose")
        for target in LINK.findall(text):
            target = target.strip().split(' "', 1)[0].strip("<>")
            if re.match(r"[a-zA-Z][a-zA-Z0-9+.-]*:", target) or target.startswith("#"): continue
            raw = unquote(target.split("#", 1)[0])
            if not raw: continue
            candidate = (path.parent / raw).resolve()
            if not candidate.exists(): failures.append(f"{path.relative_to(ROOT)}: missing link {target}")
    if failures:
        print("\n".join(failures)); return 1
    print(f"Checked {count} maintained English documents and their local links")
    return 0


if __name__ == "__main__": raise SystemExit(main())
