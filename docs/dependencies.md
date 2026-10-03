# 固定依赖与离线资源

基础依赖核对日期：2026-09-26；原生对话依赖以重写交付的固定坐标为准。使用官方 Google Maven / Maven Central / npm registry 的实际 metadata 核对，不使用动态版本。

| 组件 | 版本 | 说明 |
| --- | --- | --- |
| Gradle / AGP | 9.8.0 / 9.4.1 | 官方当前稳定版，wrapper 固定 SHA-256 |
| Kotlin Compose plugin | 2.4.20 | 官方 Maven 当前稳定版 |
| Compose BOM | 2026.09.00 | 稳定 BOM |
| Material 3 | 1.5.0-alpha29 | Expressive 所需预发布 API，显式 opt-in |
| AndroidX core / activity / lifecycle | 1.19.1 / 1.13.0 / 2.11.0 | 当前稳定版，minSdk 28 |
| AndroidX WebKit | 1.17.1 | 本地 asset loader |
| Sora editor | 0.24.6 | 上游预发布，坐标 `io.github.rosemoe:editor-bom` |
| xterm / fit / web-links | 6.0.0 / 0.11.0 / 0.12.0 | npm 精确锁版本 |
| CommonMark / GFM tables / strikethrough | 0.30.0 | 原生 Compose 聊天的 Markdown AST |
| JLaTeXMath Android | 0.2.0 | 原生 Canvas 数学渲染；精确坐标与许可位于 `app/build.gradle.kts` 和 assets/notices |
| jsdom（测试） | 30.1.1 | 仅开发依赖，不进入 APK |
| esbuild / core-js-bundle | 0.28.2 / 3.50.0 | 构建转译到 Chromium 66 / 运行时内建函数兼容层 |
| Android NDK / CMake | 30.0.15729638 / 4.3.0 | 本地 PTY 与 root 进程管理组件；当前原生构建脚本面向 Linux |
| OkHttp / SnakeYAML | 5.5.0 / 2.7 | 本机代理控制器与安全配置解析，版本已核对 Maven metadata |
| Codex / Mihomo（arm64、x86_64） | 0.157.1 / 1.19.31 | 官方原始可执行文件，固定摘要，由 PackageManager 提取管理；不携带账号认证 |

仅终端的前端资源已复制到 `app/src/main/assets/web/vendor/`，包含许可证与 SHA-256 清单。首次运行不请求 CDN。重新生成：

```sh
cd web
npm ci --ignore-scripts
npm run vendor
npm test
```

前端 JS 在预载时转译到 Chromium 66，且附带 core-js 内建函数兼容层，以覆盖 Android 9 原始 WebView。终端真实设备测试位于 `OfflineRendererTest.kt`，原生对话在 `NativeTranscriptTest.kt`，不能仅凭 Node 测试确认 Android 兼容性。

Sora TextMate 语法（JSON、Python、JavaScript、TypeScript、Markdown、Kotlin）也已预载。`tools/vendor-editor-grammars.py` 固定上游 commit，附带来源 manifest、许可证及第三方声明，不在启动时下载。配色主题为本项目自有定义。

一手参考：

- [AndroidX 发布版本](https://developer.android.com/jetpack/androidx/versions)
- [Gradle 9.8 发布说明](https://docs.gradle.org/9.8.0/release-notes.html)
- [Compose BOM 映射](https://developer.android.com/develop/ui/compose/bom/bom-mapping)
- [Material 3 发布说明](https://developer.android.com/jetpack/androidx/releases/compose-material3)
- [Sora 0.24.6](https://github.com/Rosemoe/sora-editor/releases/tag/0.24.6)
- [xterm.js](https://github.com/xtermjs/xterm.js)
- [CommonMark Java](https://github.com/commonmark/commonmark-java)
- [JLaTeXMath Android](https://github.com/noties/jlatexmath-android)
