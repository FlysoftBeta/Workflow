# W10：工作环境接入

接续 Claude Code 中断的 W10，实现事实以当前代码和下列验收为准。

## 实现

- `platform/engine`：APK 镜像流式安装、原生 metadata verify、generation/reconcile、进程级 health、前台服务租约、DNS 文件与 network callback、`container.json` watcher、终端和代理的 activation lease、重启。
- 终端用 JNI PTY 启动环境内 `work` 的登录 bash，默认 `/workspace`。`.workflow` 隐藏，home 与 toolchains 是独立持久 store。Codex 经环境启动 APK 固定二进制；Claude Code 按需下载 2.1.283，固定 SHA-256、版本执行探测、可见进度/失败/重试。无宿主凭据。
- 按最新产品要求，生产不再调用 Android shell / native agent / debug demo 绕行；环境不可用即显示失败。旧版 `container.json` 和 Codex login-state 迁移移除。
- 修复安装进度为 null 时的 UI 类型转换崩溃；home/rootfs 路径映射必须先匹配更具体的 bind；清除 idle activation race；确保实例状态先记录再交给调用方；重启在 IO dispatcher 执行、等待真实退出；代理在每次启动读取所持 activation 的环境变量。
- 修复 device base APK 缺少整棵 `guest/` 脚本的问题（Gradle Directory 被 isFile 过滤），改为递归文件输入。基础镜像按阶段恢复 provision/pack 后进入同一 install/reconcile 路径。
- 首次 base 实测又发现打包器遍历 Android `/proc`、`/sys` 绑定而受 SELinux 拒绝。打包专用 `GuestRun` 关闭默认运行时绑定，保留脚本、输出、DNS 和 null/zero/random/urandom；终端和代理的运行时绑定不受影响。
- MainActivity 在前台、workspace ready 后恢复已启用的 overlay；foreground service 包含 environment 租约。

## 验证

2026-10-02：

- `native/engine/test-host.sh`：68/68。首次发现宿主 locale 使 mkdir 错误中的引号不同；probe 同原行设置 `LC_ALL=C`，不改 oracle、不改行号，再跑全套通过。日志 `artifacts/w10-native-host.log`。
- `native/engine/gen/check.sh`：通过，三个宿主变体、NDK arm64 compile/link、生成表同步；日志 `artifacts/w10-syscall-check.log`。
- `python3 -m unittest discover -s image/tests -v`：27/27；日志 `artifacts/w10-image-tests.log`。
- 旧 60 分钟 soak 的实际日志已查：13634 轮，engine RSS 2528 → 2384 KiB，零孤儿，2/2；证据 `artifacts/engine/accept/soak-run.log` 和 `soak.log`。不是本次重跑。

API 28 app-sandbox 验收：使用独占、read-only `workflow-tablet-api28` AVD / emulator-5830，测试入口 `platform.engine.EngineIntegrationTest`。2026-10-03 的新运行统一由 `WORKFLOW_EMULATOR_PORT=5830 tools/with-emulator.sh bash artifacts/w10/base-accept.sh` 持有 device 锁、限制 1536 MiB 内存、使用磁盘临时目录，并自动停止。

- 完整 workspace APK：2/2，132.397 秒，日志 `artifacts/w10-instrumented2.log`。覆盖 Claude Code 2.1.283 下载/摘要/环境内版本执行，以及 Debian work/root、PTY、共享工作区、隐藏内部目录、Python/Node、HTTPS、apt、Codex app-server/model list、真实 watcher/reconcile、重启后 Node 22/env/home 保留。不使用宿主凭据或付费模型请求。
- base 第一次完整实测：provision 成功，但打包失败，256.088 秒；`artifacts/w10-base-instrumented4.log`、`w10-base-bootstrap4.log` 保留真实错误。关闭运行时绑定后的宿主定向验证：捕获 28445 条目，未遍历 proc/sys/dev/tmp 子内容；`envctl state` 正确读到 Python 3.14.7 / Node 24.21.0（`artifacts/w10-pack-capture-check.log`）。
- 修复后的 base APK 从清空的 Workflow 数据首次启动：**1/1 通过，427.073 秒**（`artifacts/w10-base-instrumented5.log`）。执行真实 provision → 环境内 pack/verify → 标准镜像安装/激活，再通过 Debian/PTY/HTTPS/apt/Codex/watcher/reconcile/restart 全流程。生成 `89e867df5295…`、280464960 字节的 workspace 镜像；metadata 确认为 `builder=engine`，provision 摘要与源码一致，bootstrap 暂存目录已清理。证据：`artifacts/w10-base-bootstrap5.log`、`w10-base-local-image5.json`、`w10-base-current5.json`、`w10-base-verified5.log`。
- APK 资产检查：实际 amd64 base 和 ARM64 device APK 的镜像索引、大小、SHA-256、全部 22 个 bootstrap 文件均与源文件一致；ARM64 engine/loader/Codex 和 Claude 2.1.283 manifest 均在包中（`artifacts/w10-assets-verified2.log`）。ARM64 base 摘要 `044cc47f9d94d8af2295d8a5f525b2d94ca82197180f54f951e3e7ec6beecede`；这不是 ARM64 真机首次启动验收。
- 默认构建已恢复 emulator workspace，并成功构建 device base；构建和快照复制在同一个 Gradle 锁区间内完成（`artifacts/w10-default-restored-build2.log`）。API 28 流式 APK 安装曾在 54.4% 停滞，主动终止自己的 adb 客户端后由 wrapper 清理；重试脚本改为 `timeout 180 adb install --no-streaming`，不把安装停滞算成应用测试结果。

限制：ARM64 日用平板未运行本轮首次配置测试；API 29+、ARM64/f2fs 与 16 KiB 真机矩阵仍按引擎报告列为未验收，不能由模拟器或构建成功替代。
