# 实施与验收记录

2026-09-26 从 Android Compose 模板开始实现。以下先记录本轮 0.2 自管架构，再保留 0.1 的历史验收，不能混为当前完成度。

## 0.2 真机迭代

- 已撤销 Termux/HTTP 配对激活门槛，工作台和文件可独立使用。默认编辑器、JNI PTY、本机 Codex 共享应用私有 `.workspace`。
- 已打包官方 ARM64 Codex 0.157.0 与 Mihomo 1.19.31，由 PackageManager 管理可执行代码；加入本地进程、前台保活、账户 UI 和本机代理配置/进程管理。
- 真机 API28 / ARM64 / kernel 4.14.98，在 UID10087、SELinux `untrusted_app` 中完成 9 项后台 instrumentation：PTY 双向文件/TTY/resize/Ctrl-C/跨块 UTF-8/退出回收，Codex 原生与实际管理通道的初始化、停止/重启，鉴权 CONNECT/系统 CA，Mihomo 只读版本与配置检查。
- 另有 2 项真机协议回放通过，验证跨 Session 后台回复归属、未知服务器请求与已解决审批清理；未调用模型。
- 外部 TLS 探针通过证书和主机名验证，到 `auth.openai.com` 得到 HTTP 403；证明 DNS/TCP/TLS 通道可达，**不代表登录或模型调用已通过**。没有发起认证或模型回合。
- 本地文件监听会刷新干净编辑器；草稿保留打开时版本，含首次键入前外部修改、保存期间继续输入、空文件被删等冲突保护。JVM 测试现有核心 19、代理解析 13、UTF-8 6 项通过（另有原模板测试）。
- 已按用户授权解锁真机，并通过 scrcpy/触屏完成首轮实际交互。已验证文件创建/保存、重名冲突提示、附件、竖屏菜单、多标签及关闭后草稿恢复。实际发现并修复了嵌入式 WebView 高度为零导致的终端黑屏/越界，以及侧边 Chat 被输入法遮挡；新包中的终端首开、触屏 `pwd` 和侧边输入法避让已复验。
- 后续真机润色以实际触控复验为准；未通过的项目不计为完成，不能以构建或后台测试替代。
- 0.2.1 实际复验：撤销/重做、Files + IME 约 6 行编辑空间、Chat 主模式输入框与发送按钮、跨 Session 草稿隔离均通过。最终包中应用搜索键盘避让、树→Chat 附件、拖放取消、标签排序、树→终端路径和 `cat` 读取、快速输入/退格/真实中文输入均通过；正文与附件重启后保留。
- 按原始需求在开发机使用现有 Codex 执行过一次无工具模型回合，返回中文 Markdown 与 `$a^2+b^2=c^2$`，CLI JSON 记录位于 `artifacts/physical-review/host-model-probe.jsonl`。没有向手机复制账户认证；这不替代真机登录/模型回合验收。
- 0.2.1 已通过真机独立 Root 代理 fixture：运行实际 manager/guardian/Mihomo、控制接口鉴权、三种模式和代理组回读、DIRECT/REJECT 本机转发、停止/重启及进程/端口/前台服务清理。生产配置未变，原有 Meta TUN 与接口/规则/路由完全未变。本测试未启 TUN，不能代替 Workflow 的实际 TUN 路由验收。
- 自研 Debian loader/ptrace 引擎仍未实现；当前本地终端是 Android 系统 shell。没有改变系统 DNS 或信任证书。
- 最终安装包的 9 项设备回归通过，包含真实嵌入式 WebView、快速 IME 与立即 Enter、重复字母、原生 PTY 输入顺序、协议与文件共享。68 项 JVM 测试、15 项 Web 测试和 lint 通过。终端重复输入根因和版本兼容策略见 [输入回归记录](android-input-validation.md)。

证据和详细边界：[本地运行时](android-local-runtime.md)、[本地代理](android-local-proxy.md)、`artifacts/physical-review/`。

## 0.1 历史工程状态

| 范围 | 已实现 / 已验证 | 尚未完成或验证 |
| --- | --- | --- |
| Session / Working Resource | 独立草稿、原子落盘、损坏恢复、外部文件冲突；1/7 天归档、最新 dirty 引用保护、手动归档决策；14 项核心测试通过 | 多端/多进程同时编辑的锁与恢复机制 |
| Launcher / Apps | 仅内置应用默认出现、手工注册外部应用、独立 Session 入口、Home/Back；响应式 Material 3 Expressive | 实体设备默认 Launcher 行为 |
| Files / Chat Paradigm | 主/侧面板切换，资源引用与布局独立；旋转时 ViewModel 保留状态，进程重建恢复导航 | 任意 Stack 拆分、任意 Panel 拖放仍不完整 |
| 文件编辑 | Sora + 六种预载 TextMate 语法、保存/撤销/重做、后台草稿写入、图片预览；文件/相册/拍照共用导入 | 文件当前由 Android 本地仓库管理，尚非统一远程 ResourceStore |
| 拖放 | 文件 tab 排序、文件进入对话附件、外部 URI 导入和目标提示 | 跨所有 Panel/Stack 的完整布局拖放 |
| 终端 | 真实 PTY、xterm、控制键、resize、链接回调、UTF-8 分块解码；Android 9 原始 WebView 66 增量渲染测试通过 | 远程终端文件链接与本地编辑器的统一解析；Android 进程死亡后的 PTY 历史重接 |
| Codex | 0.157.0 全量实验 schema（167 客户端方法 / 11 服务器请求 / 83 通知）、双向透传；聊天流、模型/推理滑块、审批/输入与通用 RPC；真实握手与模型列表通过 | 全部方法的专用 UI、完整 thread/read 历史重建、真实模型回合端到端验收；透传不等于全部界面完成 |
| Markdown / 数学 | 本地 marked + KaTeX + DOMPurify、流式更新、代码块保护、链接交接、渲染进程退出后的重载；3 项 JS 测试和 2 项设备测试通过 | 多种实体设备 WebView/大内容性能矩阵 |
| 授权与激活 | 同一能力中心提供设置入口与首次工作台激活；认证 health 成功才激活；Codex 可后装；APK 内置可导出的 runtime 包 | Android→宿主 daemon 鉴权已测；Termux 实机安装全流程仍需验证 |
| 悬浮窗 | 真实系统 overlay/FGS、拖动贴边、外部点击收起、已添加应用切换、亮度、DeviceAdmin/root 锁屏、root 灰度实现 | overlay、亮度、DeviceAdmin 锁屏在 API28 实测；root 灰度和 OEM 后台限制未实测 |
| Mihomo | 1.19.31 双架构内核/源码/许可预载，REST 模式/组选择、独立安装包与 root 启动脚本；API28 root TUN 建立、路由、控制与停止清理实测通过 | App 内完整内核安装/启停生命周期；ARM64 实体手机 root/TUN |
| 声明式环境 | config schema、文件 watcher、明确选择 proot 时的 reconcile；native 不可用会明确拒绝 | 自研引擎、构建失败回滚/原子切换、Android 上实际环境变更 |
| 私有 image | Debian 13 扁平 image.tar.zst、版本 metadata、校验、安装与虚拟属性 xattr；amd64 约 259 MiB，27,873 项属性安装验证；work/sudo/Node/Python 运行验证 | ARM64 image 构建与设备 guest 执行 |
| 自研 loader / ptrace | 架构边界和不可用状态已明确 | **尚未实现**；不能以宿主 shell、OCI 测试或可选 proot 代替 |

## 0.1 历史验证

- `./gradlew :app:assembleDebug :app:testDebugUnitTest :app:lintDebug :app:assembleDebugAndroidTest`：通过。核心 14 项测试通过（另有原模板示例测试）；lint 0 errors，仍有资源、样式、国际化等 warnings。
- `PYTHONPATH=runtime python3 -m unittest discover -s runtime/tests -v`：12 项通过，包含鉴权、路径、PTY、双向协议、事件断档恢复与镜像安全安装。
- `npm --prefix web test`：3 项通过。
- Android 9 / API28 / WebView 66：6 项 instrumentation 通过，其中 5 项针对离线渲染、协议回放和 runtime 打包，1 项为原模板检查。
- `runtime/reference/smoke-result.json`：真实 Codex 初始化与模型列表；没有发起模型回合。
- `runtime/reference/image-result.json`：完整 amd64 image 安装和用户/语言环境验证。
- `runtime/reference/mihomo-emulator-result.json`：真实 root TUN 与停止后路由恢复。
- [交互验收记录](android-ui-validation.md)：截图、页面与系统动作检查，随最终设备验收更新。

构建产物：`app/build/outputs/apk/debug/app-debug.apk`。运行包与代理包位于 `runtime/artifacts/`，预构建 amd64 镜像位于 `runtime/artifacts/amd64/`。这些大产物和验收截图均在项目内，但不进入源码版本控制。

## 0.1 历史后续方案（统一资源项已由 0.2 自管方案取代）

1. 按 [ResourceStore 方案](resource-store-next.md) 统一 Android 编辑器、Codex、终端的资源身份与文件服务，避免不可靠的双向目录镜像。
2. 按 [自研引擎设计与验收门槛](native-engine-design.md) 完成 PackageManager loader、ptrace 与 syscall/属性语义。宿主探针已验证 ptrace 与普通文件 xattr；Android 应用 UID、跨应用存储归属及新旧 kernel 矩阵仍需逐项验证。
3. 从协议库存逐项扩展专用 Codex UI、历史恢复、跨进程终端恢复及通用 Panel/Stack 布局能力。
4. 以 Workflow 应用自管的 ARM64 工作区完成环境变更、代理生命周期与权限失效的组合验收；Termux 已不是交付前提。

以上待办是完整需求的一部分，不应把此轮可运行基线称为整份需求已经完成。
