# Android native engine：接口、实验闸门与验收规格

状态：**设计与宿主探针；自研 loader/ptrace engine 尚未实现、不可用**。2026-09-26。
本方案对应原始 brief 的 container engine；`engine:proot` 仍是独立兼容后端。
本文件不改变现有 `core/`、runtime、资源仓库边界；实际接入须同时遵守
[架构契约](architecture.md)、[ResourceStore 方案](resource-store-next.md) 和
[当前 runtime 契约](runtime.md)。

## 先做哪些实验

在写完整 syscall 表前依序通过 G0–G2；任一项失败，记录明确不支持的平台，不能退回 host shell 并报告成功。

| 闸门 | 最小实验与必留证据 | 通过条件 / 失败后的动作 |
| --- | --- | --- |
| G0 执行归属 | 在**应用自身 UID**下记录 nativeLibraryDir、实际 loader 路径/ELF、数据根、SELinux context、API、targetSdk、ABI、kernel、page size；安装/更新后重新取路径 | PM 确实提取 loader，应用子进程可执行；只在 adb shell/root 下成功不算。验证 app 私有 rootfs 与 Termux 私有 rootfs 是两个 owner |
| G1 加载链 | PM loader → 私有目录中同架构 guest ELF 的 PT_INTERP → guest `/bin/true`；记录每段 mmap/mprotect errno、auxv、退出码；另测 libc/dlopen 的 executable mappings | 真正执行 Debian linker 与 guest，不能只打印 hello 的 Android ELF。PM loader 可执行**不等于** guest 文件映射 `PROT_EXEC` 被允许；失败时不可用，不降低 targetSdk/关闭 SELinux掩盖问题 |
| G2 tracing | 自己创建的 child 的 TRACEME、entry/exit、exec、fork/vfork/clone、信号/重启、tracee 与 tracer 异常退出 | 每个 tid 的状态一致；信号只投递一次；无悬挂子进程。分别测纯 PTRACE_SYSCALL 与可选 seccomp 路径；SETOPTIONS 成功不代替事件实测 |
| G3 文件身份 | xattr regular/dir/symlink、硬链接、rename、unlink-open-fd、并发 chmod、重启与恢复 | guest `stat`/`fstat`/`statx`/access 一致；xattr 不支持时同样有明确 durable store；属性不可静默丢失 |
| G4 Debian 工作负载 | work、sudo su、shell pipeline/job control、git、编译、Python/Node、动态库、终端 resize、SIGINT | 结果与本机同版本 Debian 对照；记录不支持 syscall 及调用栈，不伪造系统权限成功 |
| G5 lifecycle/接入 | stop、应用进程死亡、升级、断电式安装中止、资源文件与 PTY/Codex 指向同一执行 root | 清理可验证；恢复不误杀复用的 PID；迁移/回滚后资源 ID 和草稿仍有效 |

0.2 已在真机 API28 的应用 UID 中验证 PackageManager 提取的静态 Codex、原生 PTY 和 Mihomo只读执行，证明部分 G0 执行归属；自研 loader → guest PT_INTERP 的 G1 仍未实现/验收。宿主探针与这些静态程序的成功均不能用来打开 Debian native-engine availability。
Android 10 对 target API 29+ 的可写应用数据目录执行有限制；官方文档并不保证任意自定义 ELF 映射策略可行。
[Android 执行限制](https://developer.android.com/about/versions/10/behavior-changes-10#execute-permission)

## 执行 owner 与 PackageManager loader

**实验版决策：tracer、loader、rootfs 全部先在 Workflow 的 Android UID 下运行。**
默认运行进程与文件现在均归 Workflow UID，使用 `filesDir/.workspace`，不需要 Termux。下列同 UID 方向已经用于本地 PTY/Codex；它仍未代替 guest loader 的 G1 验证：

- Android native runtime：同 UID 的服务拥有自己的 root descriptor，并提供文件、进程、PTY 和 Codex。guest rootfs 后续也应归同一个执行 owner；不再引入另一个默认工作区或外部服务激活门槛。
- 旧 Termux-owner 候选已撤回。跨 package 访问只作为未来显式外部兼容情境，不是当前架构前提。

NDK 原型产出 `libworkflow_loader.so`（可 `execve` 的 ELF，不是依赖 JNI `dlopen` 的普通 shared library）和 tracer。
ABI 首批 `arm64-v8a`，`x86_64` 用于模拟器；不承诺 32-bit guest。
APK/AAB 必须启用对应版本 AGP 的 native library 提取设置，并在最终 APK 与实际安装上确认结果；仅设置 manifest 不算证明。
通过 `ApplicationInfo.nativeLibraryDir` 发现每次安装后的路径，不硬编码 `/data/app` 或把 loader 复制到可写数据根执行。
PM 管理的程序文件是安装代码；所有可变状态、image、日志、socket、probe 结果仍归配置的 `.workspace`。
[提取选项](https://developer.android.com/guide/topics/manifest/application-element#extractNativeLibs)、
[nativeLibraryDir API](https://developer.android.com/reference/android/content/pm/ApplicationInfo#nativeLibraryDir)

加载流程的责任必须分开：

1. `ElfInspector` 只读有界输入，校验 magic/class/endian/machine、program header 范围、段大小与溢出、PT_INTERP 长度/终止、page alignment、ET_EXEC/ET_DYN、GNU_STACK/RELRO/property。跨 ABI 或不支持的 execstack 显式拒绝。
2. `ExecPlanner` 在 **guest** 路径空间解析脚本 interpreter / PT_INTERP，再经 bind/path resolver 得到 host FD；不使用 Android linker 解析 glibc。初始 ELF 与 linker 都验证；PT_INTERP 的 guest 绝对路径不能被当成 host 绝对路径。
3. tracer 为这次 exec 生成有界、带版本的 `LoadPlan`：映射、零填充、初始栈、argv/env/auxv、入口、运行时 page size。采用带有效入口的独立 freestanding loader；具体 ET_DYN/static PIE 布局先由 G1 验证，不能直接套固定虚拟地址。
4. 子进程在 tracing 下 exec PM loader；loader 将 ELF/PT_INTERP 的 PT_LOAD 段映射到检查过的地址，按 `p_memsz-p_filesz` 清零 BSS，建立 ABI 对齐的栈，从 guest linker entry 开始执行。动态库后续映射仍受 syscall 代理覆盖。
5. `AT_PHDR/PHENT/PHNUM/ENTRY` 描述 guest 主程序，`AT_BASE` 指向 guest interpreter；`AT_EXECFN` 是 guest 名称，保留/生成有效 `AT_RANDOM`，检查 `AT_PAGESZ`、uid/gid、`AT_SECURE`、vDSO 与 ABI 一致。虚拟 sudo/set-id 的 credential 与 secure-exec 语义必须专测，不能随意复制 loader 的 auxv。
6. successful exec 才提交新 image/cwd/fd-CLOEXEC/credential 状态；失败时保留旧进程。无 PT_INTERP 的静态 ELF、shebang 递归、execveat 的 FD/AT_EMPTY_PATH 单列 case。不得把 `execveat` 全部等价为 pathname execve。

不要预设 anonymous executable copy、memfd 或改 `+x` 就能绕过实际平台限制；这些若成为备选，必须有独立 G1 结果、W^X/更新一致性设计及内存成本记录。默认方案不要求 root。

## 可实现的接口（提案，不是现有 HTTP API）

现有 `GET /container` 的 native unavailable 保持。新增 native 服务前先实现纯数据模型与独立原生测试驱动：

```text
RuntimeRoot { runtimeId, ownerUid, workspaceId, canonicalPrivateRoot }
EngineProbe { engineBuild, abi, api, targetSdk, kernel, pageSize,
              nativeLibraryDir, ptraceFeatures, mappingFeatures, metadataFeatures,
              passedGates[], unavailableReasons[] }
InstallRequest { operationId, sourceStream, trustedSha256, imageType, architecture,
                 expectedCurrentGeneration?, expandedByteLimit, entryLimit }
InstallResult { generationId, metadata, rootfsDigest, attributeSchema }
StartRequest { operationId, generationId, argv[], environment{}, guestCwd,
               credentials:{uid,gid,groups}, binds[], binfmtRules[], ptySpec? }
RunHandle { runId, engineInstanceId, startedAt, generationId }
RunEvent { sequence, runId, type:started|output|exit|unsupported|failure, details }
StopRequest { runId, signal, graceMillis }
RemoveRequest { generationId }  // 有活动 run 引用就返回 busy
```

- Kotlin adapter/daemon 只传结构化值，不拼 shell 字符串；native IO 在独立服务/后台执行，不放主线程、不放进 `core/`。
- 每次启动锁定已验证 generation 与配置 fingerprint；重复 operationId 返回同一结果。错误包含 stage、errno、guestPath、ABI/syscall，日志隐藏环境密钥；失败不能报 `ready`。
- `runId` 是逻辑身份；native 持有 tid/start-time 或 pidfd（可用时），持久文件只用于恢复审计，不凭旧 PID 直接 kill。EXITKILL 与每-task parent-death/lifecycle 行为均实测。
- 停止先发预期信号并等待，超时杀掉本次 run 所有被跟踪任务并 reap；PTY/管道/FD 关闭完成后才发 stopped。进程组不足以覆盖自行 setsid 的后代。
- 能力按实验结果声明。`ptraceSupported`、`guestElfPassed`、`metadataVirtualized`、`readOnlyBindSupported` 分开，不能以 `su`/二进制存在或开关已启用代替。

## ptrace scheduler 与 syscall 覆盖

一个 wait-loop 管理全部 tid；每个 `TaskState` 至少有 ABI、tgid/tid、entry/exit 阶段、原始/改写寄存器、pending result、signal info、credential、exec generation、shared fs/fd/vm 引用和 loader phase。
`clone` 标志决定 `CLONE_FILES/FS/VM/THREAD` 的共享；fork 拷贝逻辑视图，vfork 处理父进程暂停；非 leader exec 后 tid 变更必须使用 exec event 信息重新关联。
新 task 的第一次 stop 与 fork event 到达顺序均要容忍。一次 exec 不能让全局 entry/exit toggle 错位。

基线先只用 `PTRACE_SYSCALL` + `TRACESYSGOOD` + FORK/VFORK/CLONE/EXEC/EXIT；新内核实际支持时用 `GET_SYSCALL_INFO`，旧内核用每 ABI 的 GETREGSET/SETREGSET 与经过信号/exec 校正的状态机。
register layout、syscall number、stat/stack/time ABI 都按架构固定表，不能把宿主 `struct stat` 写到另一 ABI。
启动对最小无害 tracee 做事件能力探测；不依据 uname 单独决定支持。

seccomp tracing 只是第二阶段的可选提速：实测 syscall-entry/seccomp/exit 的次序与 GETEVENTMSG 内容；Android 已有 seccomp/SELinux 拒绝不会因为自己再安装 filter 消失。
必须处理 `SIGSYS`/被拒 syscall；不能假装已撤掉系统 seccomp。无加速时仍应正确，只是变慢。
Termux 的最新修复显示部分 4.14 内核 event message 会丢失，说明仅有设置成功不足以证明运行正确。
[Termux event loop](https://github.com/termux/proot/blob/d4d2a19081c3c07f75250e4ce2980b9fa2f5720f/src/tracee/event.c)

信号分为 syscall-stop、ptrace event、signal-delivery-stop、group-stop；只把真正 delivery 信号及原 siginfo 投递一次。
对 `SIGSTOP/CONT`、`SIGINT`、`SIGCHLD`、`SIGPIPE`、实时信号、EINTR/SA_RESTART、restart_syscall 建回归。
G2 使用 TRACEME；后续若改用 SEIZE/LISTEN，要单独验证 job control，不把 group-stop 一律吞掉。
[Linux ptrace 语义](https://man7.org/linux/man-pages/man2/ptrace.2.html)

| syscall 类别 | 首轮实现面 | 必须保持的语义 / 暂不支持边界 |
| --- | --- | --- |
| 路径与 FD | open/openat/openat2、*stat/statx、readlink*、getcwd/chdir/fchdir、rename/link/unlink/symlink、mkdir/rmdir、dup/fcntl/close/close_range | dirfd、AT_EMPTY_PATH、follow/no-follow、绝对/相对 symlink、删除后仍开放的 FD、CLOEXEC；旧内核替代不能丢失 resolve flags |
| 执行/地址空间 | execve/execveat、brk、mmap/mprotect/munmap/mremap、arch/TLS 必需调用 | loader 的 syscall 与 guest syscall 有显式 phase；JIT、RELRO、page size、非 PIE 分开验收；exec failure 不提交新状态 |
| 身份/属性 | get/set*uid/gid、groups、umask、chmod/chown 家族、access/faccessat2、xattr 家族 | guest credential 与 host UID 永远不同层；errno/返回结构一致；sudo set-id 仅虚拟改变身份 |
| 进程/通信 | fork/vfork/clone/clone3、wait、kill/tgkill、signal、futex、socket/connect/bind/sendmsg | pathname Unix sockets 与 SCM_RIGHTS 需要 FD/path 登记；不能只代理普通文件 open。clone3 不可简单无条件降级 clone |
| 不支持或有副作用 | mount/namespaces、capabilities、真实设备管理、嵌套 ptrace、io_uring 路径操作等 | 先建立 syscall inventory。会绕过路径/身份代理的调用显式 ENOSYS/EPERM；不要默认透传未知调用并声称隔离 |

host pass-through 仅针对已审计、无需路径/身份转换的 syscall。unsupported 列表是版本化能力的一部分。
这仍是同 UID 用户空间兼容环境，**不是对恶意 guest 的安全沙箱**；没有 namespace/内核隔离保证。

## UID/GID/mode 的 durable 语义

沿用 image 中 `user.workflow.uid/gid/mode` 的 ASCII 十进制约定作为 v1 迁移输入；mode 保存权限与特殊位，文件类型来自真实 inode 或显式虚拟节点描述。
host 目录需 owner 可遍历、host 文件需引擎可读写，host setuid/setgid 不启用；guest exec/access 根据**虚拟** `+x` 判断。仅写 xattr 不会改变 syscall 所见 UID/mode。

`AttributeStore` 操作使用 root/generation + 稳定 object identity + 打开的 FD；xattr 支持的普通 inode 优先读写 xattr。symlink 的 `user.*` 常不支持，不能对 symlink 使用会跟随目标的 setxattr；用 l*语义/单独记录。
当前 path-keyed `attributes.json` 是安装索引，尚不能充当运行时数据库：rename、硬链接、unlink-open-FD、inode 复用会令路径快照失效。

计划用版本化 journal 保存 objectId、generation、路径链接集合及变化事务；能写 `user.workflow.objectId` 的 inode 以其关联，不能写 xattr 的对象由链接/生命周期索引管理。
所有 create/link/rename/unlink/metadata 更新同步这个索引；没有 generation/生命周期校验时，`st_dev:st_ino` 不能永久作主键。外部文件 API 写入也必须更新它或触发显式重新导入。

- `chmod/chown` 先校验 guest 权限，持久化成功后再返回成功；`stat/fstat/statx`、`access`、`exec` 使用同一 attribute view。
- mkdir/create 结合 guest umask；硬链接共享属性；rename 保留 objectId；open-unlink 后 fstat 仍有效；最后 FD/链接释放才回收。
- 虚拟 root 只拥有 guest 语义；不能 mknod 真设备、操作 host mount 或授予 Android 权限。`sudo su` 的 passwd/group、PAM、tty、set-id、AT_SECURE 联合测试不可略过。
- guest xattr API 隐藏/保护 engine 保留名字，并真实返回其它命名空间不支持的错误；不能把 `security.capability` 写失败伪造成真正授予 kernel capability。
- xattr 与 journal 无天然跨文件事务：必须定义 write-ahead/recovery 次序、fsync 边界、损坏时只读或 unavailable。G3 用故障注入覆盖每个提交点。

本机探针确认 regular xattr 随硬链接/rename 保留，symlink xattr 返回 EPERM；这只是该宿主文件系统的结果，Android ext4/f2fs 要重测。
Termux `USERLAND` fake-id 分支使用独立 metadata 文件，其 statx 分支还有 `TODO: USERLAND`；不能把参考实现当作现成 xattr 全覆盖方案。
[属性参考](https://github.com/termux/proot/blob/d4d2a19081c3c07f75250e4ce2980b9fa2f5720f/src/extension/fake_id0/stat.c)

## image 安装、bind 与 binfmt

继续使用 zstd tar 内 `metadata.json + rootfs/` 的私有格式；type=`debian-trixie`，work=1000:1000。
解包前验证 trusted SHA-256、版本/variant/ABI；限制压缩输入、展开总量、entry 数量/单文件大小与元数据长度；拒绝重复条目、绝对路径、`..`、危险 hardlink、解包时 symlink 穿越。保留普通 guest symlink 文本，device node 只记虚拟描述。
构建、下载、staging、journal 在 `.workspace/container/` 内。

下一版安装以 `generations/<id>/` 为原子单位，同时包含 rootfs、metadata、属性与校验结果；完成 fsync 后原子替换小的 `current.json` 指针，再同步父目录。
现在 `image.py` 对 rootfs/metadata/attributes 的三次 replace 不等价于完整 generation 提交，迁移前必须有断电恢复测试。运行中 generation 不删除、不原地替换；配置 rebuild 成功才切换，失败保留旧 generation。
大 image/源代码 checkout/实验输出放 ignored `artifacts/` 或 `runtime/artifacts/`。

bind 是用户空间 guest→owner-root 路径映射，不是真 mount。结构包含 source ResourceRef/允许的 owner root、guest target、读写策略、符号链接策略、mappingId；最长组件前缀匹配，路径按 guest root 逐段解析，绝对 symlink 从 guest `/` 重起。解析必须覆盖 dirfd、cwd、`/proc/self/fd` 与删后 FD，并防止 TOCTOU 把已验证对象换掉。

`readOnly` 初期继续拒绝：只有 open flags 过滤不足以覆盖现存可写 FD、mmap(MAP_SHARED)、rename/unlink/link、truncate、metadata、SCM_RIGHTS。全部通路有一致拒绝行为后才可声明语义支持，也不宣称安全沙箱。
`/dev` 采用明确设备白名单；`/proc` 做 guest 身份/path 呈现，不能默认整个 host `/proc` 就是隔离过的 guest `/proc`。

binfmt 先实现 guest shebang（长度、参数、递归上限、错误码）和 native ELF；其它 ABI 默认 ENOEXEC。
之后若接 QEMU，使用 engine 自己的 magic/mask/offset → interpreter/argv 规则，不写 host `/proc/sys/fs/binfmt_misc/register`，不声称支持完整 kernel binfmt flags；runner 也必须通过 G1/G2，且其 syscall/guest FD 映射另验。
[内核 binfmt 参考](https://docs.kernel.org/admin-guide/binfmt-misc.html)

## Android 与 kernel 验收矩阵

所有 native 行均为**待验证**。API、内核、页大小不能互相推断；报告记录实际值。

| 目标 | 关键区别 | 必跑集合 |
| --- | --- | --- |
| Android 9/API 28 x86_64 模拟器，SELinux enforcing、普通 app UID | minSdk 真基线；不能使用 adb root 成功替代 | G0–G5，4K，纯 syscall tracing，离线安装 |
| Android 9/API 28 arm64 真机，厂商 4.x 内核 | ARM 寄存器/TLS、旧内核/backport、f2fs/ext4 | G0–G5、fallback register 读取、无新 syscall、信号/进程压力 |
| Android 10/11 arm64，API 29/30，target 保持项目设置 | 可写数据执行限制、Android seccomp | G0/G1 首先，exec/mmap 实际 errno，再 G2–G5 |
| 新 Android arm64，实际 5.10/5.15/6.x 内核设备 | GET_SYSCALL_INFO/pidfd/openat2/clone3 的实际支持 | G0–G5、能力启用/禁用各一轮、seccomp 加速开关一致性 |
| 新 Android arm64，16 KiB 页设备或可用镜像 | ELF p_align、mapping 边界、NDK linker/page-size | G0/G1、BSS/RELRO/dlopen/JIT、安装库提取；不能硬编码 4096 |
| 同一支持设备上的 release APK、升级、重启、省电/后台终止 | debug 路径不等于发行路径；PM loader 路径会变 | G0/G5、service death、活动 guest 清理、配置/资源恢复 |

通过 G4 的最低工作负载：`id` 为 work；`sudo su` 后 guest uid=0、真实 Android UID 不变；运行 git/init/status、make 编译、Python venv/uv、Node/nvm、Unicode/空格路径、symlink/硬链接、数百次 fork/exec、多线程、shell pipelines、job control、Ctrl-C、终端 resize。
每项报告 command、环境 fingerprint、退出状态、guest/host 预期与实测、stderr、持续时间；不得仅凭无 crash 或编译成功打勾。
新增 syscall 被版本化 inventory 标注 supported/emulated/rejected/unverified，并与负面用例共同评审。

## 本轮实际证据与来源固定

独立探针不链接/复制 PRoot 代码，不修改 app/runtime，不操作模拟器：

```bash
python3 tools/engine-probes/run.py
```

2026-09-26 宿主 x86_64 Linux `7.2.7-200.fc44.x86_64`、uid 1000：
TRACEME + syscall-stop 80 次、GET_SYSCALL_INFO 80 次、fork event 1、exec event 1、SIGUSR1 delivery 1、两个进程正确退出（0/23）。
读取 ELF64 的 PT_INTERP/LOAD 成功；**由内核普通 exec 执行，不是自研 loader**。
regular xattr/硬链接/rename 成功且 host UID/mode 不变；symlink xattr errno=1。
输出 `artifacts/engine-probes/result.json`；trace stop 次数可随宿主 libc 变化，不以 80 为固定断言。
未验证：Android PM、guest linker、自定义寄存器/syscall 改写、execveat、vfork/clone、group-stop、seccomp、新旧 Android kernel、崩溃恢复。

以下均为 2026-09-26 实际读取的官方源代码；本地 shallow checkout 位于 ignored `artifacts/engine-references/`，包含上游许可证。本轮仅阅读与引用，未 vendoring；若后续复用源码，保留其 GPL-2.0-or-later 版权/许可与发行义务，不能改名后当作全新实现。

| 源 | 固定 revision / 本轮使用处 |
| --- | --- |
| proot-me/proot | [`2265984687bf354ff35a0b8b1887fc6f5b5518b7`](https://github.com/proot-me/proot/tree/2265984687bf354ff35a0b8b1887fc6f5b5518b7)，commit 2026-09-20；`src/execve/enter.c` 解析 PT_LOAD/PT_INTERP、选择 loader，`src/loader/loader.c` 与 `script.h` 的映射/栈契约 |
| termux/proot | [`d4d2a19081c3c07f75250e4ce2980b9fa2f5720f`](https://github.com/termux/proot/tree/d4d2a19081c3c07f75250e4ce2980b9fa2f5720f)，commit 2026-09-24；`src/tracee/event.c` 事件兼容，`src/syscall/enter.c` execveat 边界，`src/extension/fake_id0/` 属性语义，`src/path/binding.c` 路径映射 |

进一步可逐段对照的源码：
[上游 exec planner](https://github.com/proot-me/proot/blob/2265984687bf354ff35a0b8b1887fc6f5b5518b7/src/execve/enter.c)、
[上游 load script](https://github.com/proot-me/proot/blob/2265984687bf354ff35a0b8b1887fc6f5b5518b7/src/loader/script.h)、
[Termux loader/auxv](https://github.com/termux/proot/blob/d4d2a19081c3c07f75250e4ce2980b9fa2f5720f/src/loader/loader.c)、
[Termux syscall dispatcher](https://github.com/termux/proot/blob/d4d2a19081c3c07f75250e4ce2980b9fa2f5720f/src/syscall/enter.c)、
[Termux bind resolver](https://github.com/termux/proot/blob/d4d2a19081c3c07f75250e4ce2980b9fa2f5720f/src/path/binding.c)。
