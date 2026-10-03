# 验证

实际完成度见 [status.md](status.md)；每次运行的证据放在 `docs/report/` 与忽略的 `artifacts/`。编译、host 回放、Android instrumented 测试和真实账户/设备验收必须分别记录。

## 宿主检查

要求 JDK 25、Rust 1.93.1、Android SDK/build-tools 37、固定 NDK/CMake、Node.js、Python 3 与 zstd。SDK 路径配置在 `local.properties`；固定版本见 [dependencies.md](dependencies.md)。

```sh
mkdir -p artifacts
flock artifacts/.gradle.lock ./gradlew :core:test :agent:test :proxy:test \
  :app:testEmulatorDebugUnitTest :app:lintEmulatorDebug
flock artifacts/.gradle.lock cargo test --manifest-path engine/Cargo.toml -j 2
python3 engine/runtime/tools/generate-syscalls.py --check
engine/runtime/test-host.sh
native/test-host.sh
PYTHONPATH=image python3 -m unittest discover -s image/tests -v
(cd web && npm ci --ignore-scripts && npm run build && npm test)
```

Runtime host 套件需要 `artifacts/engine/rootfs-amd64` 与 `artifacts/image/amd64/image.tar.zst`，在独立副本运行；`ENGINE_HOST_BUILD` 可选择独立输出目录。当前入口只构建 Rust。`engine/runtime/tests/native/accept-host.sh` 的 workload/PTY/lifecycle/soak 是额外的真实镜像验收，不能用短测试替代。

Server 还有独立的协议、布局 oracle 与真实镜像生命周期测试：`engine/server/tools/test-protocol.py`、`layout-oracle.sh`、`test-environment.py`；完整参数和已用产物见 [Rust Server 报告](report/rewrite/rust-workspace.md)。fixture 进程只验证协议边界，必须与真实 runtime 测试区分。

`:core` 的 `testFixtures` 保留旧 Kotlin writer 和环境 planner 的参考测试；这些结果不证明生产 Rust Server 已验收。原生 chat 由 app 单元/设备测试验证，`web` 的 17 个测试仅验证 terminal 输入与链接。

## 构建锁与设备

- 所有重型 Cargo/Gradle 构建共享 `artifacts/.gradle.lock`，Cargo jobs ≤ 2。Rust build 脚本自行取锁；已持锁的调用方传 `WORKFLOW_BUILD_LOCK_HELD=1`，避免嵌套死锁。
- Gradle worker 固定为 1，Kotlin 编译器在 Gradle 进程内运行。APK 构建和快照复制在同一锁内，避免复制到不同镜像/源码的产物。
- AVD 只经 `tools/with-emulator.sh`：一把 `.device.lock`、1536 MiB/2 核、禁用快照、`ANDROID_TMP` 指向磁盘目录，命令结束清理自己创建的 emulator。
- 正在运行或排队的验收脚本不原地修改。修复使用新的脚本快照，或等旧进程结束。

```sh
mkdir -p artifacts/qa/apk
flock artifacts/.gradle.lock bash -c '
  ./gradlew :app:assembleEmulatorDebug :app:assembleEmulatorDebugAndroidTest &&
  cp app/build/outputs/apk/emulator/debug/app-emulator-debug.apk artifacts/qa/apk/app.apk &&
  cp app/build/outputs/apk/androidTest/emulator/debug/app-emulator-debug-androidTest.apk artifacts/qa/apk/test.apk
'
WORKFLOW_EMULATOR_PORT=5860 tools/with-emulator.sh bash artifacts/qa/accept.sh
```

`accept.sh` 使用 wrapper 导出的明确 `ANDROID_SERIAL`，安装快照并执行 instrumentation。默认为 `workflow-tablet-api28`，API 28 x86_64、1920×1200、261 dpi。`WORKFLOW_AVD` 选择另一份已有 AVD。容器专用 app-sandbox 入口为 `engine/runtime/test-android.sh`，内部同样经过 wrapper；harness 源码位于 `engine/runtime/tests/android-harness/`。

## Android 验收矩阵

| 范围 | 入口与判据 |
| --- | --- |
| 连接和工作区 | `WorkbenchEngineAcceptanceTest`：真实 Server、文件传输、草稿恢复、布局、拖放/菜单、终端；不能以 reference-store fixture 替代 |
| Shell / 文件 / 编辑器 | `ShellNavigationTest`、`FilesFeatureTest`、`TerminalFeatureTest`、`ImportServiceTest`：注明每项使用真实连接还是参考 store |
| 原生对话 | `NativeTranscriptTest`：Compose streaming Markdown/公式/表格/代码、长文、滚动、选择复制；单独检查实际截图 |
| xterm | `OfflineRendererTest`：Android 9 原始 WebView、IME、特殊键、选择与链接 |
| 环境 | `EngineIntegrationTest`：定制镜像、真实 guest UID/PTY/网络/语言、后端启动、配置构建/失败/重启与私有目录隐藏 |
| Runtime | `engine/runtime/test-android.sh`：app seccomp sandbox、固定基础/M5 oracles、Rust runtime/loader 摘要 |
| 代理 | `ProxyExecutableTest`、`ProxyPanelEmulatorTest`、`ProxyEmulatorRootTest`：先 TUN-off，再隔离 emulator 上的 root/TUN 与清理 |
| 设置/悬浮窗 | `SettingsFeatureTest`、`DeviceControlsEmulatorTest`、`OverlayEmulatorTest`：权限实际读回、旋转/关闭、失败状态 |

两个 ABI 都必须使用定制 `workspace` 镜像；`base` 仅供镜像/低层测试，不是发布或应用回退。x86_64 emulator 结果不能冒充 ARM64 设备、Android API 29+ 或真实 16 KiB 动态库验收。

代理测试需要显式 `-e proxyEmulator true -e proxyEmulatorRoot true`，并同时通过 emulator hardware 与 adb uid 0 门控。日常平板不得卸载应用、输入 PIN、更改其他应用、路由、DNS、SELinux 或系统设置；Root/破坏性测试只在隔离 AVD 运行。

## 发布

```sh
tools/init-release-key.sh
tools/build-release.sh
```

输出 `artifacts/delivery/1.0.0/` 包含两个 ABI 的签名 APK、SHA256SUMS、签名报告与 `release.json`。检查 V1/V2/V3、唯一 ABI、完整定制镜像摘要/大小、Rust Server/runtime/loader、PTY/guardian/Mihomo/Codex 及离线 terminal 资产。旧 WebView chat 资产不进入 APK。

本地无口令 PKCS#12 位于 `artifacts/signing/Workflow-release.p12`，权限 0600，不入库。Release 还需独立 AVD 启动检查；Debug instrumentation 不替代 Release 接受。真实账户登录/模型轮次、ARM64 真机、OEM Root 授权边界必须另列证据。
