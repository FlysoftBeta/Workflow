# 验收与过程记录

[当前状态](../status.md) 是完成度索引；本目录保留当时的命令、产物摘要、测试结果和未覆盖边界。历史报告不自动证明后来源码已通过同样检查。

- [rewrite/integration.md](rewrite/integration.md)：本轮最终整合、修复、源码与签名包验收。
- [rewrite/client-boundary.md](rewrite/client-boundary.md)：严格协议客户端、上传、连接退休与恢复。
- [rewrite/editor-repair.md](rewrite/editor-repair.md)：真实 Engine + Sora 的主题、语法与阅读位置恢复。
- [rewrite/service-acceptance.md](rewrite/service-acceptance.md)：Engine 配置/实测状态下的 Root 代理、悬浮窗及清理。
- [initial](initial/)：初始 1.0.0 接续、研究与设备验收；对应 frozen checkpoint。
- [rewrite/rust-workspace.md](rewrite/rust-workspace.md)：Rust Server、协议、权威持久化与环境生命周期。
- [rewrite/rust-runtime.md](rewrite/rust-runtime.md)：Rust runtime/loader、host oracle、app sandbox 与 ABI 构建。
- [rewrite/custom-images.md](rewrite/custom-images.md)：两个 ABI 的定制镜像与工具链。
- [rewrite/native-chat.md](rewrite/native-chat.md)：原生 Compose Markdown/数学/表格/流式渲染。
- [rewrite/client-services.md](rewrite/client-services.md)：Android 服务与配置归属。
- [rewrite/workbench-qa.md](rewrite/workbench-qa.md)：工作台实际交互验收及已报告的集成问题。
- [rewrite/repository-cleanup.md](rewrite/repository-cleanup.md)：源码边界、资产与文档清理。

路径迁移说明：报告中的旧 `native/engine/test-host.sh` 对应现行 `engine/runtime/test-host.sh`；
`native/engine/{test,device,android-harness}` 对应 `engine/runtime/tests/{native,device,android-harness}`。
旧 C 实现整体在 `docs/archive/native-engine/`，原 WebView 对话在 `docs/archive/web-chat/`。证据中的旧文件名和原始摘要保持原样。
