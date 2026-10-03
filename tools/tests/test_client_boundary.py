"""Source-level architecture gates for the Android presentation/Engine boundary.

These complement runtime tests: moving a server writer behind an innocently named
Android wrapper must not restore production access to its module or executable.
Instrumentation fixtures deliberately have a different dependency boundary.
"""
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[2]
APP_SRC = ROOT / "app/src"
MAIN = APP_SRC / "main"

# Preserve literals (including URLs) while removing comments before checking code.
TOKENS = re.compile(r'"""[\s\S]*?"""|"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\'|//[^\n]*|/\*[\s\S]*?\*/')


def code(text):
    return TOKENS.sub(lambda match: "" if match.group().startswith(("//", "/*")) else match.group(), text)


def production_dependencies(text):
    dependencies = set()
    for configuration, module in re.findall(
        r'\b(\w+)\s*\(\s*project\s*\(\s*(?:path\s*=\s*)?["\'](:[\w:-]+)["\']\s*\)', code(text)
    ):
        name = configuration.lower()
        if "test" not in name and name.endswith(("implementation", "api", "runtimeonly", "compileonly")):
            dependencies.add(module)
    return dependencies


class ClientBoundaryTests(unittest.TestCase):
    def sources(self):
        return [(path.relative_to(ROOT), code(path.read_text())) for path in APP_SRC.rglob("*")
                if path.suffix in {".kt", ".java"} and "test" not in path.relative_to(APP_SRC).parts[0].lower()]

    def test_production_dependency_graph_cannot_reach_agent_or_chat_service(self):
        # Resolve every reached module, so a newly introduced intermediary cannot hide a server dependency.
        aliases = {":engine-chat": ROOT / "engine/chat/build.gradle.kts"}
        pending = [":app"]
        visited = set()
        while pending:
            module = pending.pop()
            if module in visited:
                continue
            visited.add(module)
            self.assertNotIn(module, {":agent", ":engine-chat", ":engine:chat"},
                             f"Android production reaches Engine implementation module {module}")
            build = aliases.get(module, ROOT / module.lstrip(":").replace(":", "/") / "build.gradle.kts")
            self.assertTrue(build.is_file(), f"Unresolved production module {module}; extend boundary resolution explicitly")
            pending.extend(production_dependencies(build.read_text()))
        self.assertIn(":agent-model", visited, "Android must consume the shared wire/model module")

    def test_vendor_adapters_and_process_launchers_are_not_android_production_code(self):
        forbidden = re.compile(
            r'\btop\.flysoftbeta\.workflow\.(?:agent\.(?:codex|claude|process|transport)(?:\.|\b)|'
            r'agent\.(?:AgentBackend|AgentStateStore|ThreadOptions|UnknownRequestPolicy)\b|engine\.chat(?:\.|\b))'
        )
        offenders = [str(path) for path, text in self.sources() if forbidden.search(text)]
        self.assertEqual([], offenders, "Vendor adapters/process machinery belong to the Engine distribution")

    def test_android_has_no_vendor_install_or_private_home_contract(self):
        forbidden = re.compile(
            r'libcodex\.so|(?:/|\$\{?HOME\}?/?)\.(?:codex|claude)\b|'
            r'\b(?:CODEX|CLAUDE|JRE)_(?:VERSION|BINARY|EXECUTABLE|HOME|CONFIG(?:_DIR)?)\b|downloads\.claude\.ai|'
            r'/(?:opt|home)/[^"\s]*(?:codex|claude)|'
            r'https://[^"\s]*(?:openai/codex|claude-code)[^"\s]*'
        )
        offenders = [str(path) for path, text in self.sources() if forbidden.search(text)]
        self.assertEqual([], offenders, "Vendor paths/releases/downloads must remain Engine implementation details")
        chat_sources = [(path, text) for path, text in self.sources() if "/platform/agent/" in str(path)]
        downloads = re.compile(r'\b(?:ProcessBuilder|URL|HttpClient|HttpURLConnection)\s*\(|\b(?:curl|wget)\s+-')
        self.assertEqual([], [str(path) for path, text in chat_sources if downloads.search(text)],
                         "Chat clients may request tool installation, never run download/installer commands")

    def test_host_pty_is_instrumentation_only(self):
        forbidden = re.compile(r'\b(?:NativePty|PtyTerminalProcess|AndroidShellBackend|LocalRuntime)\b|'
                               r'System\.loadLibrary\s*\(\s*"workflow_pty"')
        self.assertEqual([], [str(path) for path, text in self.sources() if forbidden.search(text)])
        for name in ("NativePty.kt", "PtyTerminalProcess.kt", "AndroidShellBackend.kt"):
            self.assertTrue((ROOT / "app/src/androidTest/java/top/flysoftbeta/workflow/platform/pty" / name).is_file())

    def test_terminal_client_has_no_process_or_reference_reaping_policy(self):
        forbidden = re.compile(r'"process\.(?:spawn|stop|wait)"|\b(?:EnvironmentRestartListener|beforeRestart|afterRestart|referencedTerminals)\b|\breap\s*\(')
        paths = [path for path in MAIN.rglob("*.kt") if "/feature/terminal/" in str(path)
                 or path.name in {"EngineTerminalBackend.kt", "EngineController.kt"}]
        self.assertEqual([], [str(path.relative_to(ROOT)) for path in paths if forbidden.search(code(path.read_text()))],
                         "Terminal process/restart/reference lifecycle belongs to Engine terminal resources")

    def test_connection_projection_does_not_expose_a_host_workspace_file(self):
        path = MAIN / "java/top/flysoftbeta/workflow/platform/connection/WorkspaceConnectionManager.kt"
        source = code(path.read_text())
        session = re.search(r'data\s+class\s+WorkspaceConnectionSession\s*\((.*?)\n\)', source, re.S)
        self.assertIsNotNone(session)
        self.assertNotRegex(session.group(1), r'\b(?:File|Path)\b', "Host root is an embedded bootstrap detail")
        self.assertFalse((MAIN / "java/top/flysoftbeta/workflow/platform/workspace/WorkspaceLocation.kt").exists())

    def test_codex_is_not_packaged_as_android_jni(self):
        build = code((ROOT / "app/build.gradle.kts").read_text())
        packages = re.search(r'val\s+prebuiltPackages\s*=\s*listOf\((.*?)\)', build, re.S)
        self.assertIsNotNone(packages, "Inspect the JNI prebuilt packaging contract when replacing its implementation")
        self.assertNotRegex(packages.group(1), r'["\'](?:codex|claude-code|jre)["\']')
        self.assertNotRegex(build, r'libcodex\.so')
        for path in (ROOT / "app/src").glob("*/jniLibs/**/*"):
            self.assertFalse(path.is_file() and "codex" in path.name.lower(), str(path))

    def test_dependency_gate_distinguishes_test_fixtures(self):
        self.assertEqual({":agent-model", ":core"}, production_dependencies('''
            implementation(project(":agent-model"))
            debugImplementation(project(path = ":core"))
            testImplementation(project(":agent"))
            androidTestImplementation(project(":agent"))
            // implementation(project(":agent"))
        '''))


if __name__ == "__main__":
    unittest.main()
