# Workflow

Workflow 1.0.0 面向 Android 9+：Launcher 是工作入口，同一 Session 可在文件工作台与对话之间切换。Sora 编辑器、原生 Compose 对话、xterm 终端、Codex/Claude Code、代理与设置共享 Rust Workspace Engine。

Engine 是会话、布局、文件、草稿、环境与服务状态的唯一拥有者。Android 通过连接读取权威快照；关闭面板或切换会话不会丢弃已提交草稿。终端和编码代理只在定制的 Debian 环境内运行。远程/SSH 仅预留接口，当前实现为本机连接。

## 构建

要求 JDK 25、Rust 1.93.1、Android SDK 37 与固定 NDK/CMake；在 `local.properties` 配置 SDK。预编译依赖按 `third_party/*/manifest.json` 校验摘要。两个 ABI 均使用完整定制镜像，镜像构建见 [环境](docs/environment.md)。

```sh
mkdir -p artifacts
flock artifacts/.gradle.lock ./gradlew :app:assembleEmulatorDebug
tools/init-release-key.sh
tools/build-release.sh
```

- `device` 为 ARM64，`emulator` 为 x86_64；两者都打包 `workspace` 镜像、Rust Server/runtime/loader。
- 工作区根保存用户文件；配置与内部状态位于 `<workspace-root>/.workspace/`。客户端仅保存连接配置与工作区下发的本地配置缓存。
- 版本为 1.0.0；没有旧数据迁移、跨版本协议适配或宿主进程回退。
- 签名包、摘要与检查结果输出到 `artifacts/delivery/1.0.0/`；实际范围见 [集成交付报告](docs/report/rewrite/integration.md)。

## 文档与目录

[产品](docs/product.md) · [UI](docs/ui.md) · [架构](docs/architecture.md) · [协议](docs/protocol.md) · [会话与资源](docs/workspace.md) · [Workspace Server](docs/workspace-engine.md) · [容器运行时](docs/container-runtime.md) · [环境](docs/environment.md) · [对话后端](docs/agents.md) · [代理](docs/proxy.md) · [测试](docs/testing.md) · [完成状态](docs/status.md)

`engine/{server,runtime,loader}` 为 Rust 工作区服务与运行时；`:core`、`:agent`、`:proxy` 是纯 JVM 契约/模型与适配器；`:app` 是 Android shell。`native/` 保留 PTY/JNI 与 proxy guardian，`web/` 只维护离线 xterm 资源，`image/` 定义两个 ABI 的定制镜像，`third_party/` 保存供应链清单与许可证。

历史实现与旧设计在 [docs/archive](docs/archive/README.md)，验收与过程记录在 [docs/report](docs/report/README.md)。旧 Kotlin 状态写入器仅保留在 test fixtures。生成物与本地签名文件位于被忽略的 `artifacts/`，不会进入生产源码或 APK。
