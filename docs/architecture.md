# 架构

用户行为见 [product.md](product.md)，跨组件调用以 [protocol.md](protocol.md) 为准。Rust Workspace Engine 是工作区唯一写入者，Android 负责连接、界面和本机能力执行。

## 模块

| 目录 / 模块 | 职责 |
| --- | --- |
| `engine/server` | JSONL Workspace Server；会话、布局、文件、草稿、配置、服务文档、environment 与进程生命周期 |
| `engine/runtime` | Rust 容器 CLI、ptrace、路径与身份转换、镜像安装、generation 元数据 |
| `engine/loader` | Rust no_std ELF 加载器 |
| `:core` | 纯 JVM 的工作区协议、数据模型、布局语义与终端流工具；旧写入器仅作 test fixtures |
| `:agent` | 纯 JVM 的 Codex/Claude 协议适配器、中立事件模型与审批约束 |
| `:proxy` | 纯 JVM 的 Mihomo 配置/控制与 guardian 协议 |
| `:app` | Android 连接 shell、独立 feature、Compose/Sora/xterm 界面与本机能力执行器 |
| `native/` | Android PTY/JNI 与 Root proxy guardian；容器实现不在这里 |
| `web/` | 离线 xterm 资源、Android 输入适配与测试 |
| `image/`、`third_party/` | 定制镜像工具、固定供应链清单与许可证 |

`:core`、`:agent`、`:proxy` 均不依赖 Android；`:agent` 与 `:proxy` 彼此独立。`feature.*` 不互相引用，跨功能动作通过注入的服务端口和面板导航契约传递。`AppGraph` 组装连接、可丢弃快照投影和平台执行器；ViewModel 不拥有持久工作区状态。

## 数据布局

```text
<workspace-root>/             用户文件；guest /workspace
  .workspace/                 Engine 内部目录；普通文件树隐藏
    config.json               工作区设置
    env.json                  完整环境的声明
    state/workspace.json      Session、布局、文件草稿、composer
    environment/              generation、home、toolchains、环境记录
    services/<serviceId>/     服务配置、资产及脱敏日志
    documents/                opaque 后端索引等文档
    trash/                    可撤销删除
    corrupt/                  损坏原文保留
```

具体磁盘记录和恢复规则由 [workspace-engine.md](workspace-engine.md) 与 Server 定义。配置与服务文件只经显式动作开放；其他内部状态不可通过文件 API 或 guest 访问。Android 缓存可丢弃；连接中断不能退回本地文件仓库。应用数据不参与系统备份。

## 状态与连接

启动先选择并保存连接配置，再建立 bootstrap/transport、执行精确版本 hello、获取权威快照。当前只有本机 Embedded 实现；远程/SSH 仅预留抽象与配置类型，未实现路径明确失败。

所有工作区变更由 Server 串行提交，原子持久化后返回 revision 与完整权威状态。客户端 `StateFlow` 是投影，不是第二写入者。布局变更由 Server 运行 reducer；Kotlin 模型与布局纯函数保留协议编码和测试 oracle。断线禁用修改，重连重新取快照，已提交草稿由 Engine 保留。

Session 布局与 Working Resources 分离。归档保护、composer CAS/ack、外部冲突和原子保存规则见 [workspace.md](workspace.md)。环境构建、watch、进程等待和流读取不占用状态提交锁。

## 进程与渲染

APK 解压三个可执行文件：`libworkflow-engine.so` 是 Server，`libworkflow-runtime.so` 是 Rust 容器，`libworkflow-loader.so` 是加载器。Server 调用 runtime 在已验证 generation 内启动所有终端与代理进程；PTY/stdio 数据通过进程 API 传递。没有 Android shell、宿主 agent 或旧仓库回退。Server 不解析 vendor 审批，审批仍由 `:agent` 保真传递给用户。

环境镜像、home、工具链与重启见 [environment.md](environment.md)；runtime 的兼容边界见 [container-runtime.md](container-runtime.md)。RuntimeService 维持本机连接和能力执行生命周期。Root Mihomo 是独立本机执行器，配置及发布状态仍归 Engine，见 [proxy.md](proxy.md)。

聊天全部使用 Compose：CommonMark 解析，原生文本、表格、代码、Canvas 数学公式；滚动位置经面板命令保存到 Engine。xterm 独占 WebView，离线资源转译到 Android 9 Chromium 66，URL/文件链接交给本机导航。

旧 C 实现、旧 WebView 对话与旧设计在 [archive](archive/README.md)；当前生产构建不能读取它们。历史 Kotlin writer 只在 test fixtures 中参与参考测试。验证方法与实测边界见 [testing.md](testing.md)、[status.md](status.md)。
