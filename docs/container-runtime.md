# Rust 容器运行时

Workspace Engine Server 调用 `workflow-runtime` 管理容器进程和镜像 generation。Android 中对应 `libworkflow-runtime.so`；`libworkflow-engine.so` 由 Server 使用。运行时是普通应用 UID 上的兼容层，**不是安全隔离边界**。它不使用宿主 agent/terminal 作为产品回退；无 root 的 `run -- CMD` 只保留给原生测试。

## 实现边界

- `engine/runtime`：Rust CLI、ptrace 调度器、路径解析、虚拟身份/权限、xattr 元数据、硬链接日志/fsck、exec 规划和流式安装器。平台专用模块显式保留 Linux x86_64、Android x86_64、Android aarch64 的 libc/寄存器布局。低层指针操作仍是 unsafe Rust；Rust 重写不意味着已经完成内存安全改造。
- `engine/runtime/src/sysinv.rs`：共享、只读的系统调用分类；`inventory/` 固定 Linux v7.2.8 源文件摘要。未知号码、空洞和 x32 号码仍拒绝。`tools/generate-syscalls.py --check` 检查摘要、完整分类和生成文件。
- `engine/loader`：Rust `no_std` ELF 加载器；仅系统调用入口及最终切换栈使用汇编。无 libc、无动态解释器、无运行时重定位，段按 16 KiB 对齐。实际页大小来自 `AT_PAGESZ`；保留文件映射和匿名复制两条路径。
- 运行时仅链接操作系统 libc 与固定 `zstd-sys = 2.0.16`（zstd 1.5.7）压缩库；不链接、不调用原 C 引擎。原 C 实现与许可证已归档到 `docs/archive/native-engine/`，生产构建入口不使用它们。

## CLI 与状态

`run`、`install`、`clone`、`verify`、`remove`、`fsck`、`probe` 的参数、输出与退出码延续冻结的运行时契约；Server 用独立 argv 调用。`run` 支持 `--root`、重复 `--bind HOST:GUEST` / `--hide`、`--cwd`、虚拟 `--user work|root` / `--uid` / `--gid`、`--loader`、`--socket-dir`、binfmt、seccomp 与进程树策略。客体与宿主 ABI 必须相同。

运行时默认绑定 `/proc`、`/sys`、必要设备以及 Android bionic 路径。`--no-default-binds` 只供镜像打包等显式调用：仅保留调用方传入的 binds；普通终端/agent 保持默认绑定。工作区私有状态由 Server 逐个 `--hide` 隐藏；配置/服务文件保留显式访问路径。

每次客体 exec 都真实执行加载器，保留 CLOEXEC、线程回收、信号重置和 cmdline 语义。加载计划保留虚拟 UID/GID、`AT_SECURE`、shebang/binfmt 和 argv。x86_64 在 Android 应用 SIGSYS 下仍执行旧调用改写；虚拟身份及拒绝类调用直接模拟。

rootfs 内 `.workflow-engine/` 继续保存隐藏的元数据锁、实例锁、硬链接对象/日志和 FIFO 后备对象；这不是工作区配置目录。所有 Workspace 状态和运行环境 generation 的外层所有权由 Server 持有，见 [protocol.md](protocol.md)。退出默认收敛整个客体树；`--wait-all` 保留显式等待模式。

安装器保留流式 zstd/tar、摘要与属性验证、fd 相对提取、失败清理、种子目录和 generation 使用锁。退出码：65 数据无效，66 输入缺失，73 无法创建目标，74 I/O，75 空间不足；remove/fsck 修复遇到使用中的 generation 返回 3。

## 构建与验证

- `engine/runtime/build-host.sh OUT`：输出 `OUT/engine`、`OUT/loader`；`ENGINE_RUST_PROFILE=debug` 可启用 Rust 运行时检查。
- `ENGINE_ANDROID_OUT=OUT engine/runtime/build-android.sh [arm64-v8a x86_64]`：输出每 ABI 的 `libworkflow-runtime.so`、`libworkflow-loader.so`，API 28。脚本共享 `artifacts/.gradle.lock` 且 Cargo jobs=2；调用方已持锁时设置 `WORKFLOW_BUILD_LOCK_HELD=1`。
- `ENGINE_HOST_BUILD=OUT engine/runtime/test-host.sh`：运行保留的 68 项验收，唯一运行时为 Rust；probes 与 oracles 位于 `engine/runtime/tests/native/`。
- `engine/runtime/test-android.sh`：通过 `tools/with-emulator.sh` 持有设备锁、在 emulator-5830 的真实 app sandbox 中运行 24 项基础用例与 29 项 M5 用例。测试暂存目录为冻结 harness 文件名建立 Rust 二进制副本，校验包内摘要。

已验证矩阵与原生兼容限制记录于 [Rust 运行时报告](report/rewrite/rust-runtime.md)。Android arm64 构建不等于 arm64 设备验收；16 KiB 上 4 KiB 链接的 amd64 glibc 动态库限制仍存在。
