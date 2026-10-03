# 容器引擎（workflow-engine）

`native/engine` 实现工作区 Debian 环境的运行时：以普通应用 UID 运行 glibc rootfs 的用户态兼容层（ptrace 系统调用转换 + 自有 ELF 加载器 + xattr 元数据）。**它不是安全沙箱**：客体与应用同 UID，没有命名空间隔离，恶意客体不在考虑范围内。镜像格式、`container.json`、reconcile 与数据布局总览见 [environment.md](environment.md)；本文件是引擎自身的设计与对 `:app` 的接口契约。完成度与测试证据见 `docs/report/initial/w2-engine.md`；应用集成验收见 `docs/report/initial/w10-engine-integration.md`。

## 1. 约束（规划决定）

| 项 | 决定 |
| --- | --- |
| 实现 | 自有 C 代码，不含 proot 代码；参考其边界情况清单 |
| 架构 | 客体架构 = 宿主架构（arm64-v8a 设备 / x86_64 模拟器与宿主测试），不支持 32 位客体 |
| 可执行位置 | 引擎与加载器都放在 `nativeLibraryDir`（API 29+ 只允许从这里 exec） |
| 元数据 | `user.*` xattr；硬链接一律模拟（Android 应用目录 `link()` 返回 EACCES） |
| 身份 | 默认用户 `work`（1000:1000）；`sudo su` 得到虚拟 root，宿主 UID 不变 |
| 异构二进制 | 通用 binfmt 表，不随包附带 QEMU |
| io_uring | ENOSYS（其 SQE 中的路径绕过 ptrace） |
| 实例 | 每个进程树一个引擎进程；多个实例可同时使用同一 rootfs，共享状态以文件锁保护 |

## 2. 组成

| 文件 | 作用 |
| --- | --- |
| `libworkflow-engine.so` | 引擎（PIE 可执行文件，按 .so 命名以便 PackageManager 解压）：CLI、tracer、分发器、安装器 |
| `libworkflow-loader.so` | 独立加载器：static-pie、无 libc、无重定位（构建时校验）、16 KiB 段对齐 |
| `src/tracer.c` | 单线程 `waitpid(-1, __WALL)` 调度器；`PTRACE_SEIZE` + `EXITKILL`；fork/vfork/clone/exec 事件；组停止用 `PTRACE_LISTEN`；seccomp 快速路径 |
| `src/path.c`、`guest.c` | 路径解析（chroot 语义、绑定表、隐藏路径、`/proc` 魔法链接） |
| `src/sys.c` | 系统调用分发：路径翻译、虚拟 DAC、元数据、硬链接、getdents/xattr 过滤、AF_UNIX、x86_64 旧调用改写、SIGSYS |
| `src/meta.c` | xattr 元数据、权限判断、硬链接存储、崩溃恢复、`fsck` |
| `src/ident.c` | 虚拟凭据与能力集 |
| `src/exec.c`、`loader/` | exec 规划（shebang、execveat、binfmt、set-id）与加载 |
| `src/install.c` | 镜像安装器与 generation 生命周期（zstd 解码器 `third_party/zstd-1.5.7`，BSD） |
| `src/sysinv.c`、`gen/` | 生成的系统调用清单（Linux v7.2.8 表，按摘要固定），见 §7 |

### 2.1 exec 与加载

每次客体 `execve`/`execveat` 都被改写为对加载器的真实内核 exec（argv/envp 原样），因此新地址空间、CLOEXEC、线程回收、信号重置都由内核完成，`/proc/self/cmdline` 正确。引擎在 exec 入口按内核规则规划：解析路径、权限（虚拟 mode）、binfmt 规则、shebang 链（最多 5 层，第 6 层 ELOOP）、ELF 架构检查、`PT_INTERP` 在客体内解析、set-id（虚拟 mode，`no_new_privs` 时忽略）。加载器用标记系统调用（参数带魔数的 `getpid`）取回计划，映射可执行文件与解释器，构建客体栈（auxv 含 `AT_SECURE`、虚拟 `AT_*UID/GID`、`AT_EXECFN`、`AT_RANDOM`），设置任务名，再报告每进程 4 MiB 暂存区（每线程 16 KiB 槽，放翻译后的路径）。

段映射：能用文件映射时共享页缓存（段与运行时页大小同余且互不共页）；否则整体匿名映射、逐段复制，每页取所有相关段权限的并集。页大小一律取自 `AT_PAGESZ`。

## 3. CLI 契约（`:app` 的 ProcessLauncher / EnvironmentEngine）

所有子命令都通过 `ProcessBuilder(nativeLibraryDir/libworkflow-engine.so, …)` 启动；加载器默认取同目录的 `libworkflow-loader.so`。

### 3.1 run

```
libworkflow-engine.so run --root GEN/rootfs
    [--bind HOST:GUEST]... [--hide GUESTPATH]... [--cwd GUESTDIR]
    [--user work|root | --uid N --gid N] [--socket-dir DIR]
    [--binfmt RULE|@FILE]... [--no-guest-binfmt] [--no-seccomp] [--wait-all]
    [--no-default-binds] [--loader FILE] [-v|-vv] -- CMD [ARG...]
```

应用启动环境实例（终端、代理后端、reconcile 步骤）时的标准绑定（[environment.md](environment.md) §2.6）：

| 选项 | 值 |
| --- | --- |
| `--root` | `.workflow/engine/generations/<n>/rootfs`（`current.json` 指向的 generation） |
| `--bind` | `.workspace:/workspace` + `--hide /workspace/.workflow`；`engine/home/work:/home/work`；`engine/toolchains:/opt/toolchains`；应用写好的 `resolv.conf:/etc/resolv.conf`（设备当前 DNS）；APK 内固定的静态 musl `libcodex.so:/opt/workflow/bin/codex` |
| `--socket-dir` | `cacheDir/s`（须短：AF_UNIX 路径上限 108 字节，见 §6.3） |
| `--cwd` | `/workspace`（终端）或调用方指定 |
| `--user` | 缺省 `work`；reconcile 的 root 步骤用 `--user root` |

绑定进来的对象对客体呈现宿主权限位；应用进程的 umask 是 077，因此应用写给客体读的文件（如 `resolv.conf`）须显式设为 0644——apt 以 `_apt` 身份解析域名。绑定的挂载点若在 rootfs 中不存在，引擎按需创建（root 所有，0755；文件绑定建空文件）。自动加入的绑定：`/proc`、`/sys`（隐藏 `/sys/fs/selinux`）、`/dev/{null,zero,full,random,urandom,tty,ptmx}`、`/dev/pts`；Android 上还有 `/system`、`/apex`、`/vendor`、`/product`、`/system_ext`、`/odm`、`/dev/__properties__`、`/dev/socket` 与文件 `/linkerconfig/ld.config.txt`（应用可读该文件但不能 stat 目录），让 bionic 程序（`/system/bin/linker64`、系统属性、netd DNS 代理）在客体内照常运行；Debian 不使用这些路径。

- **argv**：`CMD` 若不含 `/`，按客体 `PATH` 查找；`argv[0]` 保持原样。
- **环境**：引擎把自己的环境原样交给客体，只去掉 `WORKFLOW_ENGINE_*`。调用方应 `env -i` 式地给出完整环境：镜像 `metadata.environment`（`PATH`、`LANG`、`NVM_DIR`、`UV_*`）+ `container.json` 的 `env` + `HOME=/home/work USER LOGNAME SHELL=/bin/bash TERM TZ`（设备时区）。
- **PTY**：终端会话把 JNI PTY 的从端作为 stdin/stdout/stderr 与控制终端交给引擎（`setsid` + `TIOCSCTTY`）；引擎不做任何终端处理，作业控制（Ctrl-C/Z、`fg`）、`SIGWINCH`/`TIOCSWINSZ` 由内核在客体进程组上直接生效。代理后端用管道。
- **停止**：向引擎发 `SIGTERM`：转发给整个客体树，`WORKFLOW_ENGINE_STOP_GRACE` 秒（缺省 3）后 `SIGKILL`。引擎被杀时 `PTRACE_O_EXITKILL` 使客体随之终止；引擎是 child subreaper，孤儿进程不会逃出。
- **实例的结束**：主进程（`CMD`）退出时，实例随之结束——其余客体进程（后台作业、守护进程）先收到 `SIGTERM`，宽限期后 `SIGKILL`，引擎以主进程的状态退出，因此 `awaitExit` 不会被残留进程拖住。`--wait-all` 改为等待整棵树全部退出。
- **退出码**：客体主进程的退出码（0–255，信号为 128+N）；125 引擎错误；126 不可执行；127 命令不存在；2 用法错误。
- **日志**：stderr，前缀 `[engine:级别:pid]`；`WORKFLOW_ENGINE_LOG=0..4`（缺省 1=警告；3 记录绑定表与 SIGSYS 处理；4 逐个系统调用及其路径解析）；`WORKFLOW_ENGINE_STATS=1` 在结束时输出停止次数与快速路径状态。其他测试开关：`WORKFLOW_ENGINE_SECCOMP=0`、`WORKFLOW_ENGINE_CRASH_AT=<点>`、`WORKFLOW_ENGINE_CHECK_TOGGLE=1`、`WORKFLOW_ENGINE_TEST_LEGACY=sigsys`、`--test-app-filter`、`--test-page-size 16384`、`--no-filemap`。

### 3.2 生命周期子命令

| 命令 | 行为 | 输出 |
| --- | --- | --- |
| `install --image FILE\|- (--index image.json \| --sha256 HEX) --target GEN [--profile workspace\|base\|any] [--quiet]` | 按 environment.md §2.5 校验并流式安装到新目录 `GEN`（必须不存在；reconcile 传 `<n>.partial`，成功后自行重命名）；`-` 从 stdin 读（可直接转发 `AssetManager` 流）；任何失败都删除 `GEN` | stdout 一行 JSON（rows、members、hardlinks、seedEntries、bytes、sha256、seconds）；进度写 stderr |
| `clone --from GEN --to GEN2` | 复制 rootfs（能 reflink 则 reflink）、xattr、硬链接存储；不含 seeds | JSON（entries、seconds） |
| `verify --generation GEN` | 每个对象都有元数据，硬链接存储一致 | JSON；0 通过，1 有问题 |
| `remove --generation GEN` | 删除（不跟随符号链接）；有实例在用则拒绝 | 0；3 正在使用 |
| `fsck --root ROOTFS [--repair]` | 硬链接存储检查；`--repair` 仅在没有其他实例时进行 | 0 一致；1 有问题；3 正在使用 |
| `probe` | `{"arch","pageSize"}` | JSON |

安装器退出码（sysexits）：0 成功；65 镜像数据不合格（格式、不变式、摘要、tar 类型）；66 输入缺失；73 无法创建目标；74 I/O 错误；75 空间不足。

## 4. 数据布局

generation 之外的布局见 [environment.md](environment.md) §4。引擎在每个 rootfs 内使用一个对客体隐藏的目录（getdents、路径解析、listxattr 均不可见）：

```
generations/<n>/
  rootfs/                     客体 /
    .workflow-engine/
      links/<id>              硬链接对象（内容 + 元数据；名字是指向它的 stub）
      links/journal-<id>      转换中的日志（崩溃恢复用）
      meta.lock               元数据读改写与硬链接计数的锁
      instances.lock          运行实例持共享锁；fsck --repair / remove 需独占
      fifo/<dev>-<ino>        FIFO 占位对应的真实 FIFO（按需创建）
  seeds/<store>/…             安装器输出的 store 种子（由 reconcile 移走）
```

对象元数据：xattr `user.workflow.meta` = `1 UID GID MODE NLINK MAJ,MIN`（MODE 为八进制、含类型位）。宿主权限位：文件至少 `0600`、目录至少 `0700`，从不含 set-id；真实 mode 只在 xattr 中。

## 5. 元数据与硬链接

- **读取**：rootfs 内对象读 xattr；缺失时视为 `0:0` + 宿主权限位。**绑定目录不保存元数据**（`/workspace`、`/home/work`、`/opt/toolchains`）：属主呈现为绑定的属主（work），mode 即宿主权限位；在其中 `chown` 被接受但无效果，`chmod` 改宿主位，设备节点与硬链接返回 EPERM。从绑定移入 rootfs 的对象记录其呈现属主。
- **写入**：创建（open/mkdir/mknod）在退出时写入 `fsuid`、父目录 setgid 继承、umask 后的 mode；chmod/chown 在 `meta.lock` 下读改写（多实例安全）。内核 umask 始终是客体 umask `& 077`，保证引擎总能读写自己创建的文件。
- **权限**：路径逐级搜索权限、open 读写、创建/删除（含 sticky）都按虚拟属主与能力集判断（generic_permission 的顺序，`CAP_DAC_OVERRIDE`/`READ_SEARCH`/`FOWNER`/`FSETID`/`CHOWN`/`MKNOD`）；`access(2)` 用真实 ID。
- **特殊文件**：字符/块设备、FIFO、socket（mknod）在宿主上是普通占位文件，类型与设备号在 xattr 中（内核拒绝在 FIFO 上设置 `user.*`）。打开设备占位打开白名单内的宿主设备（null/zero/full/random/urandom/tty/ptmx），打开 FIFO 占位打开 `.workflow-engine/fifo/` 中的真实 FIFO。
- **硬链接**：第一次 `link` 把文件移入 `links/<id>`（有日志），原名与新名都成为文本为 `/.workflow-engine/links/<id>` 的符号链接（客体绝对路径，rootfs 可整体搬移）；对客体它们是普通文件（stat 给出对象的 inode 与 NLINK，getdents 报 DT_REG，readlink 为 EINVAL）。删除/覆盖一个名字使 NLINK 减一，归零时删除对象。硬链接名移出 rootfs 返回 EXDEV（`mv` 改为复制）。
- **崩溃一致性**：转换用日志，启动时恢复；其余窗口只可能让 NLINK **偏大**（泄漏），绝不提前删除内容；`fsck --repair` 按实际名字重算。
- **隐藏**：`user.workflow.*` 不出现在 listxattr，读取为 ENODATA，写/删为 EPERM；`security.capability` 写入返回 EOPNOTSUPP（不支持文件能力）。宿主的 SELinux 标签（`security.selinux`）同样不可见、不可写，`/sys/fs/selinux` 被隐藏：客体按无 LSM 的 Debian 运行（否则 `ls -l` 会显示 Android 标签的 `.` 标记）。

## 6. 身份、exec、系统调用

### 6.1 凭据

每个任务有虚拟 ruid/euid/suid/fsuid、对应 gid、附加组、能力集（eff/prm/inh/amb/bnd）、securebits（KEEPCAPS）、`no_new_privs`、dumpable 与 umask，按内核规则变化：setuid 家族的权限判断，commoncap 的 setxuid/setfsuid/exec 能力转换，capget/capset，`PR_CAPBSET_*`、`PR_CAP_AMBIENT`、`PR_SET_SECUREBITS`、`PR_SET_KEEPCAPS`。初始凭据取 `--user`，附加组来自客体 `/etc/group`。`PR_SET_DUMPABLE 0` 只记录不下发（不可 dump 的进程内存对 tracer 关闭）。set-id 程序（sudo、su、passwd、chage、unix_chkpwd）因虚拟 mode 而生效，`AT_SECURE` 相应为 1；`sudo` 检查 `/etc/sudoers` 属主与 mode 时看到的也是虚拟值。审计 netlink socket 返回 EPROTONOSUPPORT。

### 6.2 binfmt

引擎自有的规则表（从不使用宿主 `binfmt_misc`）：`--binfmt` 注册串（`:name:M|E:offset:magic:mask:interpreter:flags`）或 `@文件`，加上客体 `/usr/lib/binfmt.d`、`/etc/binfmt.d` 的 `*.conf`（同名 `/etc` 优先，按文件名排序）。先于 shebang/ELF 匹配；支持 `P`，接受 `O`/`C`/`F`（解释器以路径参数拿到程序）。没有规则的外来 ELF 返回 ENOEXEC。不附带 QEMU。

### 6.3 系统调用覆盖策略

- **清单**：`gen/syscalls.tsv` 为 Linux v7.2.8 两个 ABI 的每个系统调用给出类别（EXEC/ENOSYS/EPERM/PATH/ID/META/SOCK/PROC/FD/PASS），生成 `src/sysinv.c`；`gen/check.sh` 校验表摘要、确定性与 NDK 头文件。新内核的系统调用必须先分类（默认拒绝）。
- **缺省**：没有专门处理的调用中，PASS/FD/PROC/SOCK/ID 放行，EPERM 类返回 EPERM，其余（含清单外的号码）返回 ENOSYS。
- **拒绝**：io_uring、openat2、clone3、uselib → ENOSYS（glibc 回退）；mount 家族、chroot、pivot_root、swap、acct、ptrace、fanotify、open_by_handle_at → EPERM；name_to_handle_at → EOPNOTSUPP。客体内的 strace/gdb 因此得到明确的 EPERM。
- **Android 应用 seccomp**：应用过滤器拦截的调用以 SIGSYS 到达。x86_64 旧调用（open、stat、dup2、pipe、poll、select、getdents、alarm、time 等 31 个）改写为 `*at`/`*2` 等价调用：≥ 4.8 内核在入口改写（seccomp 在 ptrace 入口停止之后检查新号码），4.4 内核在 SIGSYS 处以新号码重新发起，出口恢复客体全部参数寄存器。引擎完全模拟的调用（set*id 家族、setgroups、capset、链接模拟、占位文件、各类拒绝）在 SIGSYS 处直接给出模拟结果——应用过滤器的 TRAP 优先于快速路径的 TRACE，4.4 内核又在入口停止之前拦截，这些调用到不了入口处理。其余被拦截的调用答 ENOSYS（glibc 有回退：statx、faccessat2、fchmodat2、rseq、set_robust_list、clone3、close_range…）。aarch64 没有旧调用。宿主测试用 `--test-app-filter` 在子进程首次 exec 前装入实测的应用过滤器（`include/engine/appfilter.h`，与 zygote 的位置相同），在两种内核顺序下复现应用沙箱。
- **seccomp 快速路径**（内核 ≥ 4.8 缺省开启，`--no-seccomp` 关闭）：客体首进程安装 RET_TRACE 过滤器，只有需要处理的调用停止（PASS/FD/PROC 与 recv* 直接执行；加载器标记按参数匹配）；只在入口改动了东西时才请求出口停止；外来架构调用（x86 `int 0x80`）直接 ENOSYS。与应用过滤器叠加时 TRAP 优先于 TRACE，SIGSYS 重新发起的调用改用 `PTRACE_SYSCALL` 取得停止。4.4 内核或安装失败时退回逐调用跟踪。
- **AF_UNIX**：bind/connect/sendto/sendmsg 翻译路径；超过 108 字节的宿主路径经 `--socket-dir` 下以目录哈希命名的符号链接别名（确定性，各实例一致）；getsockname/getpeername/accept 映射回客体路径；抽象地址不变。

## 7. 已知限制

- 不是隔离：客体可以经由指向客体之外的 fd、非客体进程的 `/proc` 魔法链接等触及应用 UID 可达的宿主路径；这是兼容层，不是边界。
- 路径解析与内核之间有 TOCTOU 窗口；并发改名的竞争不保证与内核一致。
- 16 KiB 页内核上，为 4 KiB 链接的 glibc 库被 glibc 自己的 ld.so 拒绝（"ELF load command address/offset not page-aligned"）：Debian amd64 在 16 KiB x86_64 模拟器上只能运行静态程序；Debian arm64 按 64 KiB 对齐，不受影响。
- 占位特殊文件的 `d_type` 是 DT_REG（`find -type p/c` 看不到；`stat`/`ls -l` 正确）。
- 绑定目录中硬链接返回 EPERM（uv/git 会回退到复制/改名）；`O_TMPFILE` 文件不能再 `linkat` 成名字。
- 未翻译：recvfrom/recvmsg 返回的 AF_UNIX 发送方地址、sendmmsg 的目标地址、inotify 事件中的名字以外的路径信息；`/proc/<pid>/status` 等显示宿主 UID 与真实能力。
- 虚拟 ID 不影响内核对信号、ptrace、共享内存等的判断（全部同一宿主 UID）。
- 文件能力（`security.capability`）不支持；`CAP_*` 只作用于引擎模拟的检查。
- 设备：API 29+ 真机、平板（f2fs、arm64、快速路径开启）尚待验证；见报告中的矩阵。

## 8. 测试

| 命令 | 内容 |
| --- | --- |
| `native/engine/test-host.sh [m0 m1 m2 m3 m4 plain legacy]` | 宿主套件：tracer、加载器（含 16 KiB 模拟）、路径/元数据/身份三组与真实内核（podman）逐字节一致的 oracle、崩溃注入、多实例、binfmt、快速路径开关、x86_64 旧调用（两种内核顺序）；`ENGINE_ORACLE=1` 重跑 podman |
| `native/engine/test/accept-host.sh [install workload pty lifecycle soak]` | 真实工作区镜像的验收（research-engine.md §4.6）：安装、apt/dpkg、gcc/cmake、uv/npm、git、AF_UNIX、PTY 作业控制、进程压力、嵌套 ptrace、io_uring、停止/EXITKILL/断电安装、长时间运行 |
| `native/engine/gen/check.sh` | 系统调用清单 |
| `native/engine/device/run.sh SERIAL [cases/m5.json]` | 在真实应用进程（zygote 派生、应用 seccomp 与 SELinux 域）中运行；`WORKSPACE_IMAGE=1` 上传镜像供安装器用例使用；AVD 由 `device/avd.sh` 管理 |
