# Pinned dependencies and offline assets

Dependency versions are exact repository inputs, not a claim about the latest upstream release. The Gradle version catalog, wrapper properties, module build files, npm lockfile, Cargo lockfile, and third-party manifests are the source of truth. Stable releases are preferred; Material 3 Expressive and Sora are the explicit prerelease exceptions. The application has minSdk 28 and must work with Android 9's original terminal WebView.

| Component | Pinned version | Location or purpose |
| --- | --- | --- |
| Gradle / Android Gradle Plugin | 9.8.0 / 9.4.1 | Wrapper SHA256 and `gradle/libs.versions.toml` |
| Kotlin / Compose plugin | 2.4.20 | Version catalog |
| Compose BOM | 2026.09.00 | Version catalog |
| Material 3 | 1.5.0-alpha29 | Expressive APIs with explicit opt-in |
| AndroidX core / activity / lifecycle | 1.19.1 / 1.13.0 / 2.11.0 | Android shell and Compose lifecycle |
| AndroidX WebKit | 1.17.1 | Local asset loading |
| Sora | 0.24.6 | `io.github.rosemoe:editor-bom` |
| kotlinx serialization / coroutines | 1.11.0 / 1.9.0 | Shared JVM modules; coroutines matches the app's resolved baseline |
| xterm / fit / web-links | 6.0.0 / 0.11.0 / 0.12.0 | Exact npm dependencies |
| CommonMark / GFM tables / strikethrough | 0.30.0 | Native chat Markdown model |
| JLaTeXMath Android | 0.2.0 | Native Canvas formulas; notice files under app assets |
| jsdom | 30.1.1 | Test-only web dependency |
| esbuild / core-js-bundle | 0.28.2 / 3.50.0 | Chromium 66 transpilation and built-in compatibility |
| Android NDK / CMake | 30.0.15729638 / 4.3.0 | Native adapters; build scripts currently target Linux hosts |
| OkHttp / SnakeYAML | 5.5.0 / 2.7 | Proxy controller and configuration parsing |
| Codex / Mihomo | 0.157.1 / 1.19.31 | SHA-256-pinned guest CLI / Android local proxy executable |
| Eclipse Temurin JRE | 17.0.20.1+1 | Pinned Linux amd64/arm64 guest runtime in `third_party/jre/manifest.json` |
| Claude Code | 2.1.283 | Pinned optional Engine-managed binary in `third_party/claude-code/manifest.json` |
| zstd-sys | 2.0.16 | Rust runtime compression library, containing zstd 1.5.7 |

Android and JVM libraries belong to `app/android`, `app/client` and `app/proxy`; terminal JavaScript dependencies belong to `app/web`. Rust dependencies are pinned in the Engine workspace and lockfile. Codex, optional Claude, and the temporary JRE/chat payload belong to Environment and Chat; Mihomo belongs to the local App proxy executor. The JRE pin remains required until the [Rust Chat cutover gate](../engine/chat.md#rust-port-and-cutover-gate) passes.

Prebuilt executables are declared in `third_party/*/manifest.json`, downloaded to the ignored `third_party/.cache/`, and verified before packaging. Android's PackageManager extracts native Engine/local-service executables. Codex, the Linux JRE and chat service are instead carried in the verified Engine tools archive under APK assets; Engine validates and binds them into the guest. Optional Claude installation is also Engine-owned. Upstream licenses and provenance stay with their manifests; account credentials do not accompany binaries. Customized environment image inputs have their own pins in `image/versions.env`, described in the [environment document](../engine/environment.md).

Only terminal frontend assets are copied into `app/android/src/main/assets/web/vendor/`. Their generated manifest records SHA256 and licenses, and first launch does not request a CDN. Regenerate and test them from `app/web/`:

```sh
npm ci --ignore-scripts
npm run vendor
npm test
```

Vendoring transpiles JavaScript for Chromium 66 and includes core-js built-ins needed by the Android 9 WebView. The real terminal compatibility test is `OfflineRendererTest`; passing Node tests alone is insufficient. Chat's CommonMark and JLaTeXMath dependencies support native Compose rendering and do not restore a browser chat runtime.

Sora TextMate grammars for JSON, Python, JavaScript, TypeScript, Markdown, and Kotlin are also bundled. `tools/vendor-editor-grammars.py` uses pinned upstream commits and carries source manifests, licenses, and third-party notices. Color themes are project-owned definitions. No syntax grammar is downloaded during application startup.

The UI bundles JetBrains Mono NL 2.304 from `third_party/jetbrains-mono/manifest.json` under OFL-1.1. Material Symbols Rounded vectors come from the commit and individual digests in `third_party/material-symbols/manifest.json`; `tools/design/generate-symbols.py` regenerates the vector resources. They are neither the old `material-icons-extended` artifact nor a runtime font/CDN dependency. Theme generation uses the pinned design tooling to derive the tonal palette described in the [Workbench document](../app/workbench.md).

When updating a dependency, update its exact source of truth, generated assets where applicable, hashes, and notices together, then run the checks appropriate to the affected Android and offline paths. Current build and device evidence belongs in [status](../status.md) and its linked reports, rather than in an assertion that an upstream version is current.
