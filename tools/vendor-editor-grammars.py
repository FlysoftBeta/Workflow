#!/usr/bin/env python3
"""Fetch pinned upstream grammar assets; no runtime CDN requests are needed."""
import hashlib
import json
from pathlib import Path
from urllib.request import Request, urlopen

ROOT = Path(__file__).resolve().parents[1]
TARGET = ROOT / "app/src/main/assets/textmate"
VSCODE = "https://raw.githubusercontent.com/microsoft/vscode/529ee19061e6723e0a640fe432e57c69d50a4f4f/"
KOTLIN = "https://raw.githubusercontent.com/fwcd/vscode-kotlin/4a7c1538754828c1d22a8bee8ff3400045b4352a/"
SOURCES = {
    "json.json": VSCODE + "extensions/json/syntaxes/JSON.tmLanguage.json",
    "python.json": VSCODE + "extensions/python/syntaxes/MagicPython.tmLanguage.json",
    "javascript.json": VSCODE + "extensions/javascript/syntaxes/JavaScript.tmLanguage.json",
    "typescript.json": VSCODE + "extensions/typescript-basics/syntaxes/TypeScript.tmLanguage.json",
    "markdown.json": VSCODE + "extensions/markdown-basics/syntaxes/markdown.tmLanguage.json",
    "kotlin.json": KOTLIN + "syntaxes/kotlin.tmLanguage.json",
    "LICENSE-vscode.txt": VSCODE + "LICENSE.txt",
    "NOTICE-vscode.txt": VSCODE + "ThirdPartyNotices.txt",
    "LICENSE-kotlin.txt": KOTLIN + "LICENSE",
}


def patch_markdown_for_joni(data):
    # Joni (Sora's Android TextMate regex engine) cannot put a variable-length
    # backreference in lookbehind. Test the same underscore/word boundary before
    # consuming the closing delimiter, leaving all three capture groups intact.
    old = rb"(\\1)(?!(?<=_\\1)\\w)"
    new = rb"(?!(?<=_)\\1\\w)(\\1)"
    if data.count(old) != 1:
        raise ValueError("Pinned Markdown strikethrough rule changed; review the Joni patch")
    return data.replace(old, new)


def main():
    TARGET.mkdir(parents=True, exist_ok=True)
    manifest = []
    for name, url in SOURCES.items():
        with urlopen(Request(url, headers={"User-Agent": "Workflow-asset-builder"}), timeout=60) as response:
            data = response.read()
        entry = {"file": name, "source": url}
        if name == "markdown.json":
            entry["upstream_sha256"] = hashlib.sha256(data).hexdigest()
            entry["patches"] = ["joni-strikethrough-lookahead"]
            data = patch_markdown_for_joni(data)
        entry["sha256"] = hashlib.sha256(data).hexdigest()
        if name.endswith(".json"):
            entry["scope"] = json.loads(data)["scopeName"]
        (TARGET / name).write_bytes(data)
        manifest.append(entry)
        print(name, entry.get("scope", "license"))
    (TARGET / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    for dark in (False, True):
        colors = ["#EAE4F0", "#A097AC", "#D1BBFF", "#9BD5B1", "#F1C789", "#A6C8F1"] if dark else ["#28222F", "#84798C", "#705299", "#38724D", "#925812", "#3966A3"]
        theme = {"name": "Workflow Dark" if dark else "Workflow Light", "settings": [
            {"settings": {"foreground": colors[0], "background": "#1C1921" if dark else "#FFFBFF"}},
            *[{"scope": scope, "settings": {"foreground": color}} for scope, color in zip(
                ["comment", "keyword, storage", "string", "constant.numeric, constant.language", "entity.name, support.function"], colors[1:])]
        ]}
        (TARGET / ("theme-dark.json" if dark else "theme-light.json")).write_text(json.dumps(theme, indent=2) + "\n")


if __name__ == "__main__":
    main()
