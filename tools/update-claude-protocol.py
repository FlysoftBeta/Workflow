#!/usr/bin/env python3
"""Extract the Claude Code stream-json / control protocol inventory from the Agent SDK typings.

  tools/update-claude-protocol.py <path/to/@anthropic-ai/claude-agent-sdk/package>

Reads sdk.d.ts + package.json and writes agent/src/main/resources/protocol/claude/inventory.json:
  - controlSubtypes: every `subtype` of SDKControlRequestInner (both directions; the direction is
    classified in ClaudeCoverage.kt, not here)
  - messages: every (type, subtype) of the stdout message union (SDKMessage + control envelopes)
The output is sorted and deterministic. ClaudeCoverageTest fails until new entries are classified.
"""
import json
import re
import sys
from pathlib import Path

root = Path(__file__).resolve().parents[1]
out = root / "agent" / "src" / "main" / "resources" / "protocol" / "claude" / "inventory.json"


def main(package: Path) -> None:
    dts = (package / "sdk.d.ts").read_text()
    version = json.loads((package / "package.json").read_text())["version"]

    def body(name: str) -> str | None:
        m = re.search(r"(?:export )?declare type " + re.escape(name) + r" = ", dts)
        if not m:
            return None
        start = m.end()
        depth, i = 0, start
        while i < len(dts):
            c = dts[i]
            if c in "{(<[":
                depth += 1
            elif c in "})>]":
                depth -= 1
            elif c == ";" and depth == 0:
                return dts[start:i]
            i += 1
        return None

    def union(name: str) -> list[str]:
        text = body(name) or ""
        return [p.strip() for p in re.split(r"\|", re.sub(r"\s+", " ", text)) if p.strip()]

    def literal(text: str, field: str) -> list[str]:
        m = re.search(r"^\s{4}" + field + r": ([^;]+);", text, re.M)
        if not m:
            return []
        return re.findall(r"'([^']+)'", m.group(1))

    subtypes = set()
    for member in union("SDKControlRequestInner"):
        text = body(member)
        if text:
            subtypes.update(literal(text, "subtype"))

    messages = set()

    def visit(name: str, seen: set) -> None:
        name = name.replace("coreTypes.", "")
        if name in seen:
            return
        seen.add(name)
        text = body(name)
        if text is None:
            return
        if text.lstrip().startswith("{"):
            types = literal(text, "type")
            subs = literal(text, "subtype") or [None]
            for t in types:
                for s in subs:
                    messages.add((t, s))
        else:
            for member in union(name):
                if re.fullmatch(r"[A-Za-z_.]+", member):
                    visit(member, seen)

    seen: set = set()
    for member in union("StdoutMessage"):
        visit(member, seen)

    inventory = {
        "generated": True,
        "sdkVersion": version,
        "source": "@anthropic-ai/claude-agent-sdk sdk.d.ts (SDKControlRequestInner, StdoutMessage)",
        "controlSubtypes": sorted(subtypes),
        "messages": [{"type": t, "subtype": s} for t, s in sorted(messages, key=lambda x: (x[0], x[1] or ""))],
    }
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(inventory, indent=2) + "\n")
    print(f"claude-agent-sdk {version}: {len(subtypes)} control subtypes, {len(messages)} message kinds -> {out.relative_to(root)}")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    main(Path(sys.argv[1]))
