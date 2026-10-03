# 状态

当前交付版本为 **1.0.0 Rust Workspace Engine 重写**：Server 拥有工作区状态，Android 是连接 shell，对话已改为原生 Compose。初始接续基线冻结在 `artifacts/checkpoints/initial-1.0.0/`；其 [历史验收](report/initial/continuation-1.0.0.md) 不替代重写后的验证。

| 范围 | 已记录证据与边界 |
| --- | --- |
| Rust runtime / loader | host 优化/Debug 各 68/68；API 28 x86_64 app sandbox 基础 24/24 + M5 29/29；两个 Android ABI 构建。见 [运行时报告](report/rewrite/rust-runtime.md) |
| Workspace Server | 权威状态、布局 oracle、协议/文件/文档 CAS、真实定制镜像生命周期；host 结果与 Android 集成区分。见 [Server 报告](report/rewrite/rust-workspace.md) |
| 定制环境 | ARM64 与 amd64 完整镜像，多版本工具链、验证与 envctl；`base` 不是产品 fallback。见 [镜像报告](report/rewrite/custom-images.md) |
| 原生对话 | Compose Markdown、公式、表格、代码、流式长文与滚动；单元测试及 API 28 截图/交互证据见 [chat 报告](report/rewrite/native-chat.md) |
| 客户端服务 | 工作区配置/代理文档与状态经 Engine；Android 只执行本机能力。见 [服务报告](report/rewrite/client-services.md) |
| 工作台 | 真实 Engine 交互、菜单、拖放、终端和已发现的集成问题分别记录；不能以参考 store 测试冒充。见 [工作台报告](report/rewrite/workbench-qa.md) |
| 仓库边界 | Kotlin writer 仅在 test fixtures；C runtime、旧 chat WebView、旧设计已归档；生产资源仅保留 xterm。见 [清理报告](report/rewrite/repository-cleanup.md) |

最终整合与源码/签名产物对应关系见 [集成交付报告](report/rewrite/integration.md)。432 项 JVM 测试通过，Android lint 为 0 errors / 32 warnings；实际 API 28 矩阵覆盖首启连接、工作台三种尺寸、原生聊天、环境构建/重启/失败回滚、客户端恢复及 Engine 所有权下的本机服务。各子报告仍保留其当时的源码/产物快照，不从历史结果推定最终验收。未知协议版本明确拒绝，没有旧数据迁移、跨版本适配、宿主进程 fallback 或远程/SSH 实现。

旧 generation 保守保留，暂不自动回收。仍需独立记录真实账户模型轮次、ARM64 日常平板/OEM Root、API 29+ 与真实 16 KiB 设备验收。4 KiB 链接的 amd64 glibc 动态库存在 16 KiB 页限制；Rust 的 unsafe 低层兼容实现也不等同于内存安全重构。所有破坏性/Root 验证仅使用 wrapper 管理的隔离 AVD，不改用户日常设备。
