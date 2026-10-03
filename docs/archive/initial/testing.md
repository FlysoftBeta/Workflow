# 验证

测试结果与硬件边界见 [status.md](status.md)；逐次证据保存在 `docs/report/` 与被忽略的 `artifacts/`。编译通过不代表设备验收。

## 宿主检查

要求 JDK 25、Android SDK/build-tools 37、项目固定 NDK/CMake、Node.js、Python 3 与 zstd。`local.properties` 配置 SDK；依赖版本见 [dependencies.md](dependencies.md)。

```sh
mkdir -p artifacts
flock artifacts/.gradle.lock ./gradlew :core:test :agent:test :proxy:test \
  :app:testEmulatorDebugUnitTest :app:lintEmulatorDebug
native/test-host.sh
native/engine/test-host.sh
native/engine/gen/check.sh
python3 -m unittest discover -s image/tests -v
(cd web && npm ci --ignore-scripts && npm test)
```

引擎宿主测试需要 `artifacts/engine/rootfs-amd64` 与 `artifacts/image/amd64/image.tar.zst`。每轮使用独立副本；`ENGINE_HOST_BUILD` 可改输出目录。`native/engine/test/accept-host.sh` 的 workload/soak 属于另外的验收，不用短时测试替代。

## 构建快照与内存

- 所有 Gradle 命令使用同一把 `artifacts/.gradle.lock`。worker 数固定为 1，Kotlin 编译器在 Gradle 进程内运行，构建结束退出单次 daemon。
- 设备验证只允许一个 AVD 同时运行。使用 `tools/with-emulator.sh`，它持有 `artifacts/.device.lock`，限制 1536 MiB/2 核、禁用快照，并自动停止自己创建的模拟器。
- 模拟器临时文件放在 `artifacts/emulator-tmp`；必须设置 **ANDROID_TMP**。仅设置 TMPDIR 不会移动只读 qcow 临时镜像，在 tmpfs 上会占用主机内存。
- 排队或运行中的验收脚本不原地修改；修复后使用新的脚本快照或等旧进程退出再改。
- 构建与复制验收 APK 必须在同一 Gradle 锁内。不同 `workflow.image.*` 参数会复用同一产物路径，锁外复制会拿到别人的镜像版本。

```sh
mkdir -p artifacts/qa/apk
flock artifacts/.gradle.lock bash -c '
  ./gradlew :app:assembleEmulatorDebug :app:assembleEmulatorDebugAndroidTest &&
  cp app/build/outputs/apk/emulator/debug/app-emulator-debug.apk artifacts/qa/apk/app.apk &&
  cp app/build/outputs/apk/androidTest/emulator/debug/app-emulator-debug-androidTest.apk artifacts/qa/apk/test.apk
'
WORKFLOW_EMULATOR_PORT=5860 tools/with-emulator.sh bash artifacts/qa/accept.sh
```

`accept.sh` 使用 wrapper 导出的 `ANDROID_SERIAL` 安装快照，然后运行 `am instrument`。默认 AVD 为 `workflow-tablet-api28`（API 28、x86_64、1920×1200、261 dpi）；用 `WORKFLOW_AVD` 选择另一份已有 AVD。

## 设备矩阵

| 范围 | 入口 / 要点 |
| --- | --- |
| 工作台 | `app.ShellNavigationTest`：Home/Back、Session、范式、Tab 排序/拆分 |
| 文件与输入 | `feature.files.FilesFeatureTest`、`feature.terminal.TerminalFeatureTest`、`platform.importer.ImportServiceTest`；真实 WebView、Sora、草稿、冲突、回收站、附件、路径链接、特殊键、重接 |
| 流式对话 | `feature.chat.TranscriptWebViewTest`、`OfflineRendererTest`：Android 9 WebView、增量数学/Markdown、sanitize、桥接白名单、IME |
| 环境 | `platform.engine.EngineIntegrationTest`：真实 app UID 安装、用户身份、PTY、网络、语言版本、Codex handshake/model list、Claude 按需安装、声明式重建与重启、`.workflow` 隐藏 |
| 首次构建 | 同一环境套件分别对 `-Pworkflow.image.emulator=workspace` 和 `base` 执行；安装前检查 APK 内 `image.json` 的 profile；ARM64 首次 provision 不以 x86_64 结果冒充 |
| 代理 | `ProxyExecutableTest`、`ProxyPanelEmulatorTest`、`ProxyEmulatorRootTest`；先 TUN-off，再只在模拟器上 root/TUN、冲突拒绝、停止/进程死亡后的规则恢复 |
| 设置与悬浮窗 | `SettingsFeatureTest`、`OverlayEmulatorTest`；紧凑/标准、横竖屏、权限实际读回、浮窗拖动/旋转/关闭、错误状态 |

代理测试必须显式传 `-e proxyEmulator true -e proxyEmulatorRoot true`，且测试自身检查 emulator hardware 与 adb uid 0。不要把这些参数用于真实平板。

设备操作一律指向明确 serial。日常平板不能卸载应用、输入锁屏 PIN、改其他应用、路由、DNS、SELinux 或系统设置。破坏性和 Root/TUN 测试只在隔离 AVD 上执行。

## 发布验证

```sh
tools/init-release-key.sh     # 仅首次创建；不会替换已有私钥
tools/build-release.sh
```

发布脚本生成 ARM64 与 x86_64 的 `1.0.0` Release APK，并检查 V1/V2/V3、唯一目标 ABI、环境镜像摘要/大小、base provision 脚本、引擎/loader/PTY/代理二进制和离线页面。输出在 `artifacts/delivery/1.0.0/`，附 `SHA256SUMS`、签名报告和 `release.json`。

本地无口令 PKCS#12 位于 `artifacts/signing/Workflow-release.p12`（权限 0600，不入库）。Release APK 还必须在 AVD 上启动检查；Debug instrumentation 不能代替 Release 启动验证。真实账户登录和模型调用、真实 ARM64 平板结果分别记录，不得以演示后端或协议回放冒充。
