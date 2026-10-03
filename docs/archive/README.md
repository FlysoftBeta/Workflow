# 历史归档

这里保存已退出生产的实现与设计，不作为当前行为的来源；当前文档从 [README](../../README.md) 进入。

- [initial](initial/)：重写前的长久文档快照，路径/所有权已经被 Rust Workspace Engine 取代。
- [native-engine](native-engine/ARCHIVE.md)：原 C 容器/loader/生成器及许可证的完整源码快照；当前实现为 `engine/{runtime,loader}`。
- [web-chat](web-chat/README.md)：原 WebView 聊天/Markdown 源码、资产、npm lockfile 与退役依赖许可证；当前聊天为原生 Compose。
- `legacy-runtime/proxy/`：无生产调用的旧代理示例配置。
- `android-workspace/AndroidFileWatcher.kt`：不再使用的客户端文件监视器。Android 文件系统适配仅被 instrumented reference-store 测试使用，位于 `app/src/androidTest/`。

归档文件中的相对路径、章节引用和命令记录原始位置；复现实验应在隔离 checkout 恢复原目录，不能加入生产构建。现行验收 oracles 与 harness 已迁至 `engine/runtime/tests/`。过程证据继续位于 [report](../report/README.md)，初始交付 checkpoint 位于被忽略的 `artifacts/checkpoints/initial-1.0.0/`。
