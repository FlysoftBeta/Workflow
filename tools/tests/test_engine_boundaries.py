"""Crate and IO ownership gates complement the Engine's behavioral Rust suites."""
from pathlib import Path
import re
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[2]
ENGINE = ROOT / "engine"


def production(path):
    # Unit fixtures may use direct IO/JSON to construct malformed external input.
    if path.name == "tests.rs" or "tests" in path.parts:
        return ""
    return path.read_text().split("#[cfg(test)]", 1)[0]


class EngineBoundaryTests(unittest.TestCase):
    def test_domains_depend_only_on_environment(self):
        workspace = tomllib.loads((ENGINE / "Cargo.toml").read_text())
        self.assertEqual({"environment", "environment/runtime", "environment/loader", "workspace",
                          "filework", "terminal", "server", "chat"}, set(workspace["workspace"]["members"]))
        for name in ("workspace", "filework", "terminal", "chat"):
            manifest = tomllib.loads((ENGINE / name / "Cargo.toml").read_text())
            internal = {key for key in manifest.get("dependencies", {}) if key.startswith("workflow-")}
            self.assertEqual({"workflow-environment"}, internal, name)
        env = tomllib.loads((ENGINE / "environment/Cargo.toml").read_text())
        self.assertFalse(any(key.startswith("workflow-") for key in env["dependencies"]))
        server = tomllib.loads((ENGINE / "server/Cargo.toml").read_text())
        self.assertEqual("workflow-server", server["package"]["name"])
        self.assertEqual("workflow-engine", server["bin"][0]["name"])

    def test_no_private_path_construction_outside_environment(self):
        for name in ("workspace", "filework", "terminal", "chat", "server"):
            for path in (ENGINE / name / "src").rglob("*.rs"):
                source = production(path)
                self.assertNotRegex(source, r'\.join\s*\(\s*"\.workspace(?:/|"|\b)', str(path))
                if name in {"workspace", "terminal"} or (name == "server" and "contracts" not in path.parts):
                    self.assertNotRegex(source, r'\b(?:std::)?fs::(?:read|write|rename|create_dir|remove|metadata)', str(path))

    def test_guest_process_and_pty_access_remains_in_runtime_api(self):
        for name in ("workspace", "filework", "terminal", "chat", "server"):
            for path in (ENGINE / name / "src").rglob("*.rs"):
                self.assertNotRegex(production(path), r'\bCommand::new\s*\(|libc::(?:openpty|fork|exec|kill|prctl|ioctl)', str(path))

    def test_known_domain_documents_are_not_raw_json(self):
        for name in ("workspace", "filework", "terminal", "server", "environment"):
            for path in (ENGINE / name / "src").rglob("*.rs"):
                if path.name == "json.rs":
                    continue  # The explicit opaque/strict JSON implementation is the sole owner.
                source = production(path)
                self.assertNotRegex(source, r'\bjson!\s*\(', str(path))
                self.assertNotRegex(source, r'\bserde_json::Value\b|\bValue\s+as\s+V\b', str(path))

    def test_chat_is_rust_only_and_runs_through_the_environment_runtime(self):
        for retired in ("chat/build.gradle.kts", "chat/src/main/kotlin", "chat/src/test/kotlin"):
            self.assertFalse((ENGINE / retired).exists(), retired)
        self.assertFalse((ENGINE.parent / "agent").exists())
        self.assertFalse((ENGINE.parent / "third_party/jre").exists())
        self.assertNotRegex((ENGINE.parent / "settings.gradle.kts").read_text(), r'":agent"|":engine-chat"')
        self.assertTrue((ENGINE / "server/src/chat.rs").is_file())
        self.assertIn("spawn_piped", production(ENGINE / "server/src/chat.rs"))
        self.assertFalse((ENGINE / "runtime").exists())
        self.assertFalse((ENGINE / "loader").exists())


if __name__ == "__main__":
    unittest.main()
