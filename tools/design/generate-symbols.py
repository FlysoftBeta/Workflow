#!/usr/bin/env python3
"""Converts the Material Symbols Rounded glyphs used by ui.design into Android vector drawables.

Source: google/material-design-icons at a pinned commit (Apache-2.0), optical size 20, weight 400,
grade 0; fill 0 by default and fill 1 for the glyphs listed in FILLED (active states, docs/ui.md §1.3).
Downloads are cached in third_party/.cache/material-symbols/<commit>/ and recorded with SHA-256 in
third_party/material-symbols/manifest.json. A cached SVG whose digest differs from the manifest fails
the run (delete it to re-fetch). Outputs:
  app/src/main/res/drawable/sym_<name>[_fill].xml
  app/src/main/java/top/flysoftbeta/workflow/ui/design/icons/Symbols.kt
  third_party/material-symbols/{manifest.json,LICENSE}

Usage: python3 tools/design/generate-symbols.py [--offline]
Add a glyph: append its Material Symbols name to GLYPHS (and FILLED if it needs an active state).
"""
from __future__ import annotations

import hashlib
import json
import re
import sys
import urllib.request
from pathlib import Path

COMMIT = "bd8cb85bd4bad964fe6918f79665bb40c3a8efef"  # google/material-design-icons, 2026-09-25 "Update Symbols"
OPSZ = 20
RAW = f"https://raw.githubusercontent.com/google/material-design-icons/{COMMIT}"

# Material Symbols name -> purpose (kept terse; the purpose is emitted as KDoc).
GLYPHS: dict[str, str] = {
    # Corner clusters and tool rows
    "home": "Home / Launcher", "left_panel_open": "Side bar toggle (closed)", "left_panel_close": "Side bar toggle (open)",
    "right_panel_open": "Aux region toggle (closed)", "right_panel_close": "Aux region toggle (open)",
    "settings": "Settings", "more_horiz": "More menu", "keyboard_arrow_down": "Expand / overflow", "keyboard_arrow_up": "Collapse",
    "chevron_right": "Disclosure / submenu", "arrow_drop_down": "Chip dropdown", "close": "Close", "add": "Add / new",
    "check": "Confirm / selected", "check_box": "Menu toggle on", "check_box_outline_blank": "Menu toggle off",
    "arrow_back": "Back (settings drill-in)",
    # Editor and resource actions
    "save": "Save", "undo": "Undo", "redo": "Redo", "search": "Find", "add_comment": "Attach to conversation",
    "wrap_text": "Word wrap", "content_copy": "Copy", "account_tree": "Reveal in file tree", "open_in_new": "Open with",
    "delete": "Delete / discard", "share": "Share", "fit_screen": "Fit / original size",
    # Layout
    "open_in_full": "Maximize / make primary", "close_fullscreen": "Restore", "splitscreen_right": "Split right",
    "splitscreen_bottom": "Split down", "move_item": "Move to", "tab_close": "Close others",
    # Session
    "library_add": "New session", "edit": "Rename", "swap_horiz": "Switch paradigm", "history": "All sessions",
    # Terminal
    "terminal": "Terminal", "clear_all": "Clear", "restart_alt": "Restart", "stop_circle": "End process", "keyboard": "Keyboard",
    # Proxy
    "bolt": "Latency test / speed tier", "speed": "Latency", "edit_document": "Edit config", "receipt_long": "Logs",
    "lan": "Connections", "language": "Proxy group", "power_settings_new": "Start / stop", "shield": "Proxy (built-in app)",
    # Files
    "note_add": "New file", "create_new_folder": "New folder", "photo_camera": "Camera", "photo_library": "Gallery",
    "upload_file": "Device files", "filter_list": "Filter", "unfold_less": "Collapse all", "refresh": "Refresh / retry",
    "visibility": "Show hidden", "drive_file_move": "Move file",
    # Conversation
    "fork_right": "Fork", "markdown": "Copy as Markdown", "archive": "Archive", "unarchive": "Restore",
    "arrow_upward": "Send", "arrow_downward": "Jump to bottom", "stop": "Stop", "edit_square": "New conversation",
    "lightbulb": "Thinking", "attach_file": "Attachment", "checklist": "Plan",
    "hexagon": "Codex backend badge", "asterisk": "Claude Code backend badge", "key": "API key login",
    # Status
    "warning": "Warning / full access", "error": "Error", "info": "Details", "block": "Invalid drop target",
    "check_circle": "Success", "cancel": "Remove / failed",
    # Overlay and launcher
    "lock": "Lock screen", "contrast": "Grayscale", "light_mode": "Brightness", "apps": "Apps",
    "drag_indicator": "Drag handle", "schedule": "Time", "link": "Link",
    "space_dashboard": "Workbench (built-in app)", "chat": "Conversation panel", "do_not_disturb_on": "Remove from Apps",
    # File types (by extension, ~12)
    "draft": "Generic file", "description": "Text file", "article": "Markdown", "code": "Source code",
    "javascript": "JavaScript / TypeScript", "html": "HTML / XML", "css": "CSS", "data_object": "JSON / YAML / TOML",
    "image": "Image", "picture_as_pdf": "PDF", "folder_zip": "Archive file", "table": "CSV / TSV",
    "audio_file": "Audio", "video_file": "Video", "folder": "Folder", "folder_open": "Open folder",
}
# Glyphs that also get a fill-1 variant (active state).
FILLED = [
    "home", "settings", "left_panel_close", "right_panel_close", "folder", "folder_open", "terminal", "lightbulb",
    "bolt", "check_circle", "error", "warning", "info", "lock", "contrast", "light_mode", "description", "image",
    "draft", "article", "stop_circle",
]

ROOT = Path(__file__).resolve().parents[2]
CACHE = ROOT / "third_party/.cache/material-symbols" / COMMIT
DRAWABLES = ROOT / "app/src/main/res/drawable"
KOTLIN = ROOT / "app/src/main/java/top/flysoftbeta/workflow/ui/design/icons/Symbols.kt"
NOTICE = ROOT / "third_party/material-symbols"


def fetch(relative: str, expected: str | None, offline: bool) -> bytes:
    cached = CACHE / relative
    if cached.is_file():
        data = cached.read_bytes()
        if expected and hashlib.sha256(data).hexdigest() != expected:
            sys.exit(f"{cached} does not match the pinned SHA-256; inspect and delete it to re-fetch")
        return data
    if offline:
        sys.exit(f"missing {cached} (offline)")
    with urllib.request.urlopen(f"{RAW}/{relative}", timeout=30) as response:
        data = response.read()
    if expected and hashlib.sha256(data).hexdigest() != expected:
        sys.exit(f"{relative}: downloaded content does not match the pinned SHA-256")
    cached.parent.mkdir(parents=True, exist_ok=True)
    cached.write_bytes(data)
    return data


def camel(name: str) -> str:
    return "".join(part.capitalize() for part in name.split("_"))


def to_vector(svg: str, source: str) -> str:
    if 'viewBox="0 -960 960 960"' not in svg:
        raise ValueError(f"{source}: unexpected viewBox")
    paths = re.findall(r'<path d="([^"]+)"', svg)
    if not paths:
        raise ValueError(f"{source}: no path")
    body = "\n".join(f'        <path android:fillColor="#FF000000" android:pathData="{d}" />' for d in paths)
    return (
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n"
        f"<!-- Material Symbols Rounded (Apache-2.0), {source} @ {COMMIT[:12]}. Generated by tools/design/generate-symbols.py. -->\n"
        "<vector xmlns:android=\"http://schemas.android.com/apk/res/android\"\n"
        "    android:width=\"24dp\" android:height=\"24dp\"\n"
        "    android:viewportWidth=\"960\" android:viewportHeight=\"960\">\n"
        "    <group android:translateY=\"960\">\n"
        f"{body}\n"
        "    </group>\n"
        "</vector>\n"
    )


def main() -> None:
    offline = "--offline" in sys.argv
    manifest_file = NOTICE / "manifest.json"
    pinned: dict[str, str] = {}
    if manifest_file.is_file():
        old = json.loads(manifest_file.read_text())
        if old.get("commit") == COMMIT:
            pinned = {entry["file"]: entry["sha256"] for entry in old.get("files", [])}

    unknown = [name for name in FILLED if name not in GLYPHS]
    if unknown:
        sys.exit(f"FILLED names not in GLYPHS: {unknown}")

    for stale in DRAWABLES.glob("sym_*.xml"):
        stale.unlink()
    files = []
    for name in GLYPHS:
        variants = [("", f"{name}_{OPSZ}px.svg")]
        if name in FILLED:
            variants.append(("_fill", f"{name}_fill1_{OPSZ}px.svg"))
        for suffix, file_name in variants:
            relative = f"symbols/web/{name}/materialsymbolsrounded/{file_name}"
            data = fetch(relative, pinned.get(relative), offline)
            (DRAWABLES / f"sym_{name}{suffix}.xml").write_text(to_vector(data.decode(), file_name))
            files.append({"file": relative, "sha256": hashlib.sha256(data).hexdigest(), "bytes": len(data)})

    license_bytes = fetch("LICENSE", pinned.get("LICENSE"), offline)
    NOTICE.mkdir(parents=True, exist_ok=True)
    (NOTICE / "LICENSE").write_bytes(license_bytes)
    manifest = {
        "name": "material-symbols-rounded",
        "license": "Apache-2.0",
        "source": "https://github.com/google/material-design-icons",
        "commit": COMMIT,
        "style": {"family": "Material Symbols Rounded", "opticalSize": OPSZ, "weight": 400, "grade": 0, "fill": [0, 1]},
        "packagedAs": "app/src/main/res/drawable/sym_*.xml (vector drawables, only the glyphs listed here)",
        "files": files + [{"file": "LICENSE", "sha256": hashlib.sha256(license_bytes).hexdigest(), "bytes": len(license_bytes)}],
    }
    manifest_file.write_text(json.dumps(manifest, indent=2) + "\n")

    lines = [
        "// GENERATED by tools/design/generate-symbols.py — do not edit by hand.",
        f"// Material Symbols Rounded (Apache-2.0), google/material-design-icons @ {COMMIT}, opsz {OPSZ}, wght 400.",
        "package top.flysoftbeta.workflow.ui.design.icons",
        "",
        "import androidx.annotation.DrawableRes",
        "import top.flysoftbeta.workflow.R",
        "",
        "/** Drawable ids of the bundled Material Symbols. `*Fill` variants are the fill-1 (active) glyphs. */",
        "object Sym {",
    ]
    for name, purpose in GLYPHS.items():
        lines.append(f"    /** {purpose}. */")
        lines.append(f"    @DrawableRes val {camel(name)}: Int = R.drawable.sym_{name}")
        if name in FILLED:
            lines.append(f"    @DrawableRes val {camel(name)}Fill: Int = R.drawable.sym_{name}_fill")
    lines.append("")
    lines.append("    /** Every bundled glyph by Material Symbols name (design gallery, tests). */")
    lines.append("    val all: List<Pair<String, Int>> by lazy {")
    lines.append("        listOf(")
    for name in GLYPHS:
        lines.append(f"            \"{name}\" to {camel(name)},")
        if name in FILLED:
            lines.append(f"            \"{name} (fill)\" to {camel(name)}Fill,")
    lines.append("        )")
    lines.append("    }")
    lines.append("}")
    KOTLIN.parent.mkdir(parents=True, exist_ok=True)
    KOTLIN.write_text("\n".join(lines) + "\n")
    print(f"{len(files)} glyph files, {len(GLYPHS)} names ({len(FILLED)} with fill variants)")


if __name__ == "__main__":
    main()
