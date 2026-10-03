# Engine：工作区服务与容器

生产 Engine 由三个 Rust 组件组成，不能把 Server 与容器运行时视为同一个进程。

| 组件 | 源码 | Android 可执行文件 | 契约 |
| --- | --- | --- | --- |
| Workspace Server | `engine/server` | `libworkflow-engine.so` | [协议](protocol.md)、[状态与生命周期](workspace-engine.md) |
| Container runtime | `engine/runtime` | `libworkflow-runtime.so` | [运行时](container-runtime.md) |
| ELF loader | `engine/loader` | `libworkflow-loader.so` | 由 runtime 调用的 no_std 加载器 |

Android 通过双向 JSONL 连接 Server。Server 拥有会话、布局、文件、草稿、配置、环境与服务状态，并用独立 argv 调用 runtime。终端与 Codex/Claude 只能在环境内启动；不存在宿主 fallback。

Environment 是完整运行时。两个 ABI 均打包定制 workspace 镜像；`env.json` 描述多版本工具链、包、变量和构建后脚本。Server 验证成功才发布 generation，有运行进程时等待显式重启。详见 [environment.md](environment.md)。

runtime 与 guest 同应用 UID，是用户态兼容层，不是安全沙箱。ARM64 与 x86_64 各有同架构 runtime/loader；未知系统调用默认拒绝。4 KiB 链接的 amd64 glibc 动态库在真实 16 KiB 页环境仍有限制，不能以静态 ELF 模拟测试声称完整兼容。

当前验证入口在 [testing.md](testing.md)。原 C 源码、原生成器和许可证已归档至 [archive/native-engine](archive/native-engine/ARCHIVE.md)，旧设计在 [archive/initial/engine.md](archive/initial/engine.md)。Rust 的独立 syscall 清单位于 `engine/runtime/inventory/`；共享验收 probes/oracles 位于 `engine/runtime/tests/`。生产构建和当前验收入口不依赖归档实现。
