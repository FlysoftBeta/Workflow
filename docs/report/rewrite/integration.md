# 1.0.0 Rust Workspace 集成交付

日期：2026-10-03（Asia/Hong_Kong）。初始 Claude 接续基线保留于 `artifacts/checkpoints/initial-1.0.0/`；本报告对应其后的 Rust 架构重写，不把初始基线结果冒充新架构验收。

## 最终实现

- Rust Server 是会话、面板布局、文件、草稿、配置、环境及服务状态的唯一写入者。Android 只保存连接配置和 `client.config` 下发的本地偏好缓存；其余数据通过严格版本 JSONL 协议访问。首启显式选择工作区，断线关闭工作区操作，重连读取权威快照。
- 配置和私有状态位于 `<workspace-root>/.workspace/`。没有旧数据迁移、跨版本协议适配或宿主 terminal/agent fallback。当前仅实现 Embedded bootstrap；远程/SSH 保留实际被使用的 bootstrap/transport 抽象，没有虚假的远程连接实现。
- APK 各自打包 ARM64/amd64 定制镜像以及 Rust Server/runtime/loader。镜像预置 Python、Node、uv、nvm 和开发工具；env.json 的语言数组、额外包、变量与 post_scripts 由 Server 构建和验证。
- 首次使用环境后，Server 自动监测保存及外部修改的声明；只用文件/代理服务的连接不会解压未使用的环境。构建期间旧环境可用；成功后等待运行进程退出或用户确认重启。终端和设置均提供明确确认。重启恢复终端原目录，保留面板和草稿；失败脚本的 home 变更不发布。
- 对话改为原生 Compose，支持流式 Markdown、公式、表格、代码、选择复制和长历史滚动。Chat 与账户设置通过注入端口取得服务；WebView 仅保留离线 xterm。
- UI 修复包含窄屏标题/菜单、最大化侧栏、整片编辑器边缘拖放、排序及无障碍尺寸调整。编辑器恢复逻辑行、自动换行列、部分行和水平偏移；处理 IME/尺寸变化时光标与阅读位置不同的情况；首屏颜色及 Joni Markdown 语法已修复。
- 原 Kotlin writer、宿主代理 launcher 和模拟面板仅留在测试源码；旧 C runtime、WebView chat 与过时设计归档。原生容器运行时实际为 Rust，生产不编译或调用旧 C 引擎。

## 集成修复

真实 app UID 验收发现并修复：上传缺少 size；Android 拒绝硬链接提交；配置 CAS conflict 值处理；Codex guest home 未初始化；连接/终端缓存未随连接退休；异步后端卸载与重新安装竞态；窄屏首次创建及 Ctrl+N 不显示文件树；保存 env.json 没有触发构建；后台索引刷新覆盖已确认重命名/归档；快速断开时前台恢复读取失效连接；Android 9 在 FGS 尚未确认启动前被停止而终止应用。后两项通过失效帧/连接检查、合并服务启动请求、每次启动确认前台状态及延后释放修复。

新文件、上传、移动和垃圾桶恢复使用 `renameat2(RENAME_NOREPLACE)`，竞态中出现的同名文件不会被覆盖。索引刷新在同一 edit mutex 内读取并合并，延迟 Engine 确认的回归测试验证用户标题和归档状态存续。

## 验证

所有重型构建共用 Gradle/Cargo 锁；AVD 统一经 `tools/with-emulator.sh`，一次一个、1536 MiB、磁盘临时层，不使用日常平板。源码中的测试不会把 reference store 或 fixture process 当作真实 Engine。

| 范围 | 结果及证据 |
| --- | --- |
| JVM | **432/432**：core 225、agent 48、proxy 83、app 76，零失败/跳过。`artifacts/rewrite-integration/final-checks.log`、`final-junit/`、`jvm-results.json` |
| Android lint | **0 errors, 32 warnings**；未通过禁用检查掩盖错误。临时 applied Kotlin dependency script 导致的 lint 崩溃通过直接声明同版本依赖修复 |
| Rust Server | **21/21** 单元与真实 JSONL/进程契约通过，证据见 `server-final-verified.log`；含 no-overwrite、CAS、损坏状态、布局 oracle、失败构建/重启及懒加载环境 |
| Rust runtime | [运行时报告](rust-runtime.md)、[搬迁后的 68/68 回归](repository-cleanup.md)；Android app sandbox 24 + 29 oracle，两个 ABI 构建 |
| 定制镜像 | [两个架构的真实工具链/编译器/多版本/空列表/profile 检查](custom-images.md)，27 项 image 单元测试 |
| Web | 仅 terminal 的 17 项测试通过；旧 chat/Markdown Web 依赖和资产不进入 APK |
| API 28 工作台 | 最终包三种尺寸默认创建流程通过：1920×1080@306、1920×1200@261、800×1280@320。无预置文件或 saveFile 绕行 |
| API 28 运行环境 | 实际 Rust 包、UID、Python/Node、sudo、HTTPS、Codex app-server/models/account、Claude 下载/摘要/版本、PTY、pending、确认重启、脚本失败回滚、Server 重启恢复全部通过 |
| API 28 UI | 文件/会话/设置/导入/xterm/原生聊天/控件 **33/33**；真实 Editor、连接退休/重连各 1；实际工作台自动构建/终端重连与带空格路径粘贴各 1 |

完整 UI/后端矩阵的快照为 `artifacts/rewrite-integration/apk-final/`，分组日志与截图位于 `run-final/`。之后的最终快照 `apk-final4/` 修正环境按需启动和快速重连/FGS 生命周期；与服务最终验收 `apk-v6/` 的两个 APK 逐字节一致。此最终快照再次通过界面内自动构建/重启、连续 **5/5** 次连接失败/断开/重连、**15/15** 服务测试、另 **5/5** 次悬浮窗旋转/重启/重连，以及真实 app force-stop 清理。位置检查保留原容差，并读取实际 window frame。逐项独立验收见 [服务报告](service-acceptance.md)、[客户端边界](client-boundary.md)、[编辑器](editor-repair.md)、[工作台](workbench-qa.md)、[原生聊天](native-chat.md)。

中间失败保留供定位：早期生命周期测试错误地把可编辑 `.workspace` 配置也当作隐藏目录；上传竞态测试曾在 begin 之前创建目标；参考 UI fixture 仍使用旧 trash 路径；这些测试问题均已校正。一个合并运行同时保留多个完整测试工作区导致 AVD 空间不足，最终脚本在独立测试组间清除仅该 disposable AVD 的应用数据。没有降低运行时磁盘检查或强行把失败环境标为可用。

## 发布与范围

版本固定 **1.0.0 / 10000**。输出目录 `artifacts/delivery/1.0.0/` 的 APK 经 V1/V2/V3 验证、minSdk 28、非 debuggable、单一目标 ABI；定制镜像的 profile/大小/SHA-256 和三个 Rust 可执行文件均被检查，且不包含旧 WebView chat 或 base bootstrap 资产。

ARM64 与 x86_64 的包分别用于实际设备与模拟器。API 28 x86_64 的设备结果不冒充物理 ARM64、API 29+、OEM su/黑白模式成功授权、真实 16 KiB 动态库或真实账户模型轮次。无登录凭据时验证了真实后端初始化和协议调用；真实账户模型轮次未测试。

低层 runtime 仍有 unsafe Rust；ptrace 兼容层不是恶意代码安全隔离。旧 generation 保守保留、尚无自动垃圾回收；amd64 4 KiB 链接 glibc 的真实 16 KiB 页限制、聚合状态/文本大小上限均保留在 [状态](../../status.md) 和子系统文档中。远程连接没有实现，符合本轮仅预留接口的要求。

## 最后一次服务复测说明

综合矩阵曾在悬浮球重启位置断言失败。独立复现对照 `dumpsys window` 和截图证明旋转前后的物理 window frame 一致，而 Android 9 AccessibilityNodeInfo 仍返回旧方向的缓存坐标；测试改为读取并等待实际 overlay window bounds 稳定，保留 `<8 px` 原容差。15 项服务测试和 5 次重复均通过，未改生产几何逻辑。详见服务报告 `overlay-regression/final6-v1/`。

最终连接压力结果在 `artifacts/rewrite-integration/run-final4/connection-{1..5}.log`，同组 `rebuild.log` 验证了自动构建、用户确认、终端 cwd/面板/草稿恢复。服务最终证据在 `artifacts/service-acceptance/overlay-regression/final6-v1/`。初始的失败日志保留，最终结果均有明确源码/产物对应。

最终 Kotlin 状态读取检查通过（`lint-complete.log`），为 0 errors / 32 warnings；发布编译及签名验证记录于 `release-complete.log`。两个 ABI 的 client DEX 一致，x86_64 Release 的三个 Rust 二进制与实际设备验收 Debug APK 一致；具体 SHA-256、大小、签名报告位于 `artifacts/delivery/1.0.0/`。

最终 Release 复核通过：首次连接配置、Launcher、设置、定制环境就绪、强制停止后的自动连接恢复，以及 1.0.0/10000 非 debuggable 清单；日志 `artifacts/rewrite-integration/release-smoke-complete.log`，实际截图在 `release-smoke/`。源码和对应发布副本冻结于 `artifacts/checkpoints/rust-1.0.0/`，不包含签名私钥、设备工作区或任何 transcript。最终分项机器清单为 `artifacts/delivery/1.0.0/acceptance.json`。
