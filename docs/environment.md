# 环境：定制镜像与 env.json

Environment 是完整运行时：镜像、软件包、多版本工具链、变量、挂载、进程与生命周期均归 Rust Workspace Engine。Android 仅显示 Server 发布的实测状态；跨组件边界见 [protocol.md §4](protocol.md#4-运行环境与进程)。工作区配置为 `<workspace-root>/.workspace/env.json`，不读取或迁移 container.json、旧 `.workflow` 或旧运行时状态。

## 1. 原则

- 两个 ABI 都随 APK 交付预配置 `workspace` 镜像，首次启动无需联网安装默认工具链。`base` 只用于构建和引擎测试，不能作为应用环境或发布回退。
- 镜像预装用户、shell、开发工具、uv、nvm、Python、Node 与 envctl；APK 提供 Codex。用户要求的额外版本或软件包才需要下载。
- 只有 Server 拥有期望配置、generation、profile、激活及进程。首次使用环境时请求构建；之后保存或外部修改 env.json，Server 自动检查并开始构建；完整验证及 post_scripts 成功后才提交。失败保留旧环境并报告失败步骤和退出状态，重试必须显式请求。
- 工具链在持久 `/opt/toolchains` 并存；home、用户文件不随 generation 重建。运行进程引用的旧 generation/profile 保留，有运行进程时需显式重启才能切换。

## 2. 镜像格式（formatVersion 2）

### 2.1 文件

- `image.tar.zst`：一个 zstd 流（可多帧），每帧带内容校验，窗口不超过 128 MiB（windowLog ≤ 27，解码器无需额外参数）。解压后是 POSIX tar（ustar + PAX）。
- `image.json`：与镜像一同打包的索引，`{"sha256", "size", "metadata"}`，`metadata` 是 `metadata.json` 的副本，便于不解压即可检查。APK 已签名，其中的 `image.json` 即可信摘要。
- tar 成员顺序固定：`metadata.json`、`attributes.tsv`、rootfs 成员（顺序见 §2.4）。不允许其他成员。

### 2.2 tar 成员

- 类型只允许 `0` 普通文件、`5` 目录、`2` 符号链接，以及修饰下一个成员的 PAX `x` 头。禁止 `g` 全局头、GNU `L`/`K`/`S` 扩展、`1` 硬链接和 `3`/`4`/`6` 特殊文件——它们由 `attributes.tsv` 表达。
- PAX 关键字只允许 `path`、`linkpath`、`size`、`mtime`，值为 UTF-8。
- 名字：根目录为 `rootfs`，其余为 `rootfs/` 加去掉开头 `/` 的客体路径；读取时去掉目录名末尾的一个 `/`。路径必须是合法 UTF-8，不含空分量、`.`、`..`。
- uid/gid 写 0，uname/gname 为空；mode 为 `权限 & 0777`，只供人读，安装器不得据此设置属性。mtime 为客体 mtime（整数秒），安装器必须保留（pyc、make 依赖它）。
- 符号链接目标原样保存，可以是客体绝对路径；安装时绝不跟随。
- 上限：`metadata.json` ≤ 64 KiB，`attributes.tsv` ≤ 64 MiB。

### 2.3 metadata.json

UTF-8 JSON 对象。读取方忽略未知字段；只增加字段不提升 `formatVersion`，不兼容的修改必须提升。

| 字段 | 值 | 含义 |
| --- | --- | --- |
| `format` | `"workflow-image"` | 魔数 |
| `formatVersion` | `2` | 本节定义的外层格式。`1` 是 0.2.x 旧格式（属主写在 tar 头中），不再接受 |
| `type` | `"debian-trixie"` | 变体，选择引擎与 reconcile 的变体适配器 |
| `typeVersion` | `1` | 变体内约定的版本（§2.6） |
| `profile` | `"workspace"` \| `"base"` | `workspace` 是可交付环境；`base` 是未配置的 Debian，只用于构建与引擎测试，应用不得将其激活为工作环境 |
| `architecture` | `"arm64"` \| `"amd64"` | Debian 架构名，必须与设备 ABI（arm64-v8a / x86_64）一致 |
| `createdAt` | RFC 3339 UTC | 构建时间 |
| `base` | `{reference, indexDigest, manifestDigest, platform}` | 固定摘要的 OCI 基础镜像 |
| `rootfs` | `{entries, members, regularBytes}` | `attributes.tsv` 行数、rootfs tar 成员数、普通文件字节总和（不含硬链接重复） |
| `attributes` | `{path: "attributes.tsv", version: 1, rows, size, sha256}` | 属性表及其摘要 |
| `requires` | string[] | 安装器与引擎必须实现的能力；遇到未知或未实现的能力，必须在解压前拒绝 |
| `stores` | `{guestPath: {store, seed}}` | 持久 store 覆盖的客体目录（仅 workspace）。其下内容不进入 generation，只作为 store 的种子：`seed` 为 `if-absent`（store 不存在时整体放入）或 `merge`（补入 store 缺少的条目，已有条目优先） |
| `user` | `{name, uid, gid, home, shell}` | 默认用户（仅 workspace） |
| `environment` | object | 非登录进程的默认环境变量（`PATH`、`LANG`、`NVM_DIR`…）（仅 workspace） |
| `toolchains` | `{uv, nvm, python, node, profile}` | 镜像种子中的实际版本与 profile 名（仅 workspace） |
| `defaults` | `{python, node}` | `env.json` 缺省语言列表的单元素版本规格（仅 workspace） |
| `provision` | `{sha256, builder}` | provision 脚本摘要与构建途径（`podman` / `engine`） |

`requires` 取值：

| 能力 | 含义 |
| --- | --- |
| `virtual-ownership` | uid/gid 只存在于属性表，由引擎呈现给客体 |
| `virtual-mode` | 完整 mode（含 setuid/setgid/sticky 与 `+x`）只存在于属性表 |
| `hardlink-emulation` | 存在 `h` 行（Android 应用目录中 `link()` 返回 EACCES） |
| `virtual-special-files` | 存在 `c`/`b`/`p` 行 |
| `store-seeds` | 存在 `stores`（§2.5 第 4 条） |

前两项总是出现；其余仅在内容需要时出现。

### 2.4 attributes.tsv（属性表）

ASCII 文本，行以 `\n` 结束。第一行为 `#workflow-attributes 1`，之后每个 rootfs 条目一行，字段以单个制表符分隔：

```
path	type	uid	gid	mode	extra
```

| 字段 | 规则 |
| --- | --- |
| `path` | 客体绝对路径（根为 `/`）。按字节百分号编码：0x21–0x7E 中除 `%` 外原样，其余字节（空格、`%`、控制字符、所有非 ASCII）写作大写 `%XX` |
| `type` | `d` 目录、`f` 普通文件、`l` 符号链接、`h` 硬链接、`c` 字符设备、`b` 块设备、`p` FIFO |
| `uid` `gid` | 十进制，0–4294967294 |
| `mode` | 四位八进制权限位 `0000`–`7777`，不含类型位；`l` 行固定 `0777` |
| `extra` | `h`：主路径（同样编码）；`c`/`b`：`major,minor`（十进制）；其他：`-` |

不变式（安装器逐条校验，任一不满足即拒绝整个镜像）：

1. 行按解码后路径的字节序严格递增，无重复；第一行是 `/`（`d`）。因此父目录总在子项之前。
2. 每个非根路径的父路径是之前出现的 `d` 行。
3. `d`/`f`/`l` 行与 rootfs tar 成员一一对应、顺序相同、类型一致；`h`/`c`/`b`/`p` 行没有 tar 成员。
4. `h` 行的主路径是之前出现的 `f` 行，也是该组中字节序最小的路径；组内共享主路径的内容，`h` 行重复写出与主路径相同的 uid/gid/mode，不一致即拒绝。不存在指向目录、符号链接或特殊文件的硬链接。
5. 行数、字节数与 sha256 与 metadata 一致。
6. `stores` 的每个前缀是 `d` 行；前缀之下（不含前缀本身）只有 `d`/`f`/`l` 行，属主为默认用户，无 setuid/setgid/sticky；没有 `h` 行位于 store 之下或指向 store 之内。

socket 无法表示，打包时丢弃。v1 不携带其他扩展属性；文件 capability（`security.capability`）不受支持，引擎对其如实报错。

### 2.5 安装器要求

1. 解压前：校验 metadata（`format`、`formatVersion == 2`、`type` 已知且 `typeVersion` 受支持、`profile` 符合用途、`architecture` 与本机一致、`requires` 全部实现）；读取并校验属性表（头、不变式 1/2/4/5）；按 `rootfs.regularBytes`/`entries` 检查可用空间与上限。
2. 流式解压到 staging generation，按不变式 3 逐个比对 tar 成员；多余、缺失、乱序或类型不符都使整代失败。
3. 只相对已创建的目录 fd 操作（`openat` + `O_NOFOLLOW|O_EXCL`、`mkdirat`、`symlinkat`），从不跟随归档创建的符号链接；不创建设备节点与 FIFO；宿主文件上从不设置 setuid/setgid/sticky。宿主权限由引擎决定，只须保证引擎可读写、目录可遍历。
4. uid/gid/mode 与特殊文件写入引擎的属性存储（`user.workflow.*` xattr 与 journal），`h` 行交给硬链接模拟；保留 mtime。store 前缀之下的条目不进入 rootfs 与属性存储，而是作为普通文件写入 `<generation>/seeds/<store>/<相对路径>`，宿主权限位取 `mode & 0777`（目录另加 owner rwx）；前缀目录本身留在 rootfs 中作挂载点。种子由 Server 合并进 store（§4）。
5. 整个流的 sha256 与 `image.json` 一致后才提交为 generation（§5.4）；否则删除 staging，当前 generation 不受影响。

### 2.6 变体 debian-trixie（typeVersion 1）

| 项 | 约定 |
| --- | --- |
| 基础 | `docker.io/library/debian:13-slim`，按摘要固定（`image/versions.env`） |
| 软件包 | sudo curl wget git nano unzip zip rsync ca-certificates gnupg locales tzdata build-essential cmake，另加 xz-utils（nvm 下载格式）、zstd（环境内打包）、jq（envctl 的独立 JSON 解析）。生成 `en_US.UTF-8` |
| 用户 | `work` 1000:1000，home `/home/work`，shell `/bin/bash`，无密码；`/etc/sudoers.d/workflow` 授予 `NOPASSWD: ALL`，`sudo su` 无需密码进入虚拟 root |
| uv / nvm 程序 | `/usr/local/bin/uv`、`uvx`；`/usr/local/lib/nvm/nvm.sh`（交互 shell 中 `nvm` 按需加载） |
| 工具链 store | `/opt/toolchains`（属于 `work`）：`uv/python/cpython-X.Y.Z-*`（`UV_PYTHON_INSTALL_DIR`）、`nvm/versions/node/vX.Y.Z`（`NVM_DIR=/opt/toolchains/nvm`）、`profiles/<identity>/{python,node}`（默认版本）、`pythons/<全版本>` 与 `nodes/<全版本>`（全部所选版本）、`active` → `profiles/<name>` |
| 默认解释器 | `UV_PYTHON_PREFERENCE=system`：`uv venv`/`uv run` 默认使用 PATH 上的 active Python，项目的 `.python-version`/`requires-python` 仍优先 |
| PATH | `/opt/toolchains/active/python/bin:/opt/toolchains/active/node/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin`（profile.d、sudo `secure_path`、`metadata.environment` 一致） |
| stores | `/home/work` → `home/work`（`if-absent`），`/opt/toolchains` → `toolchains`（`merge`） |
| envctl | `/usr/local/libexec/workflow/envctl`，reconcile 在客体内执行的步骤接口（§5） |
| Codex 入口 | `/usr/local/bin/codex` → `/opt/workflow/bundled/libcodex.so`；二进制由已签名 APK 提供，Server 挂载 |
| 占位文件 | `/etc/hosts`（localhost）、`/etc/hostname`（`workflow`）、空 `/etc/resolv.conf` |

引擎运行时提供：`/workspace` ← 工作区根（屏蔽 `.workspace` 内的私有状态子目录，显式配置文件按协议开放编辑）、`/home/work` ← `.workspace/environment/stores/home/work`、`/opt/toolchains` ← `.workspace/environment/stores/toolchains`、`/dev` 白名单、`/proc`；生成 `/etc/resolv.conf`（设备当前 DNS）；`TZ` 可在 env.json 中声明；未指定时使用镜像默认时区。bind 进来的文件没有属性存储：对客体呈现为 `work` 所有，权限位即宿主权限位（不含 set-id）。

## 3. env.json（version 1）

```json
{
  "version": 1,
  "python": ["3.14", "3.13"],
  "node": ["24", "22"],
  "packages": ["ripgrep"],
  "env": {"PIP_INDEX_URL": "https://mirrors.example/pypi/simple"},
  "post_scripts": [
    {"id": "prepare", "run": "mkdir -p \"$HOME/.config/example\"", "user": "work"}
  ]
}
```

| 字段 | 语义 |
| --- | --- |
| `version` | 当前格式为 1；不提供旧格式适配 |
| `python` | `3`、`3.14` 或 `3.14.7` 规格的数组；缺省取 `metadata.defaults.python`，空数组禁用托管 Python |
| `node` | `24`、`24.1` 或 `24.1.0` 规格的数组；缺省取 `[metadata.defaults.node]`，空数组禁用托管 Node |
| `packages` | 额外 Debian 包名数组；缺省 `[]`；重复或无效包名报错，镜像自带包是不可移除基线 |
| `env` | 客体环境变量，键须是合法变量名，值须是无 NUL 的字符串；具体进程可覆盖同名值，不注入宿主 runtime/loader |
| `post_scripts` | `{id,run,user:"work"|"root"}` 数组，缺省 `[]`；按序执行，root 是虚拟客体 root |

语言数组顺序有意义：首项为默认，其余同时安装且可以通过 profile 中的完整版本路径运行；重复规格或非法规格报错。匹配按点分前缀：已装 `3.14.7` 满足 `3.14`，不会只因上游补丁发布重新下载。未指定与空数组不同，禁用语言不得静默回退到系统解释器；profile 的默认命令以明确错误和退出码 127 遮挡系统 PATH。

## 4. 持久状态与 Reconcile

Server 在 `.workspace/environment/` 下拥有 generations、stores/toolchains、stores/home、日志与已验证激活。构建先准备 generation/工具链，安装额外包，逐项安装请求版本，调用 `verify-many`，再按配置顺序执行 post_scripts；全部成功才发布新 activation。post_scripts 在候选 generation 内从 `/` 执行，用于配置运行环境；构建时不挂载用户工作区。work 用户的 home 使用私有暂存副本，成功激活时按内容版本合并；失败或冲突不发布 home 变更。工具链缓存可复用。脚本的顺序、失败重试与 generation 隔离由 Server 负责，envctl 不执行它们。

安装/验证工具链不会改 active。运行实例使用已验证 profile；Server 在合适的生命周期边界执行显式 activate。没有运行进程时直接切换，有进程时保留旧实例并等待重启。构建期间旧环境始终可用；终端与设置中的重启入口均说明会停止进程，并等待用户确认。镜像环境变量作为已验证 generation 的默认值保留，env.json 覆盖同名默认值。任何失败保持旧 activation；相同失败输入不自动无限重试。远程连接只预留抽象接口，不实现 SSH，也不提供宿主 shell/agent 回退。

镜像 `stores` 保持 formatVersion 2/typeVersion 1：home 仅在不存在时播种，toolchains 合并缺少项且不覆盖已有 active。镜像默认 profile 和用户选择的额外 profile 都包含不可变身份文档；全部当前/待激活/运行实例引用消失后，Server 才能回收其版本。

## 5. envctl（客体步骤接口）

所有命令的进度写 stderr，stdout 只携带机器 JSON（不返回数据的步骤为空）。程序路径为 `/usr/local/libexec/workflow/envctl`。

| 命令 | 用户 | 行为 |
| --- | --- | --- |
| `apt-install PKG…` | root | 缺少才 update/install，标记 manual，清理 apt 缓存与索引 |
| `apt-remove PKG…` | root | 删除额外包并 autoremove；`/usr/local/share/workflow/base-packages` 中所有包保留 |
| `python SPEC` | work | uv 并存安装，已有满足版本时无下载；输出 `{python,installed}`，不激活 |
| `node SPEC` | work | 在隔离子 shell 中加载 `nvm.sh --no-use` 并安装；输出 `{node,installed}`，不激活 |
| `verify-many PY_JSON NODE_JSON [PKG…]` | work | 每个规格取最新已装版本，检查全部解释器及包，创建/验证不可变 profile；返回 `{profile,python:[全版本…],node:[全版本…],packages:{名称:版本}}` |
| `verify PY NODE [PKG…]` | work | 镜像构建便捷命令，两个单元素列表，返回标量 python/node；Server 不使用 |
| `activate PROFILE` | work | 显式原子切换 active；仅在 Server 已决定激活或镜像 provision 时调用 |
| `state [PKG…]` | 任意 | `{profile,python,node,installedPython:[…],installedNode:[…],packages:{…}}`；python/node 为当前默认或 null，installed 数组列出 store 中全部版本 |

Profile 身份由完整、有序的已解析 Python/Node 数组决定；单个 Python+Node 名为 `pyX.Y.Z-nodeA.B.C`，多版本或空变体名为 `set-<规范数组 SHA256>`。更换次要版本、次序或空状态必定产生不同身份。`toolchains.json` 记录数组；`python`/`node` 为默认入口，`pythons/X.Y.Z/bin/python3` 与 `nodes/A.B.C/bin/node` 提供所有指定版本。校验失败退出非零，不能把缺包或不可运行的版本报告为成功。

## 6. 构建与验证

- `image/versions.env` 固定 Debian OCI 摘要、uv/nvm 版本和下载 SHA256、语言默认规格与基础包集合。`guest/provision.sh` 在客体中配置两遍并比较 state，确认幂等；`guest/envctl` 是运行时接口。
- `image/build.sh --arch amd64` 和 `image/build.sh --arch arm64` 都生成完整镜像；`--profile base` 只生成测试/构建基底。输出为 `artifacts/image/<arch>/image.tar.zst`、`image.json`、SHA256、包清单、state 与构建日志。
- x86_64 主机使用 `image/with-cross.sh arm64 COMMAND…` 执行 arm64 构建/验收。它下载 `third_party/qemu-user/manifest.json` 固定 SHA256 的 Debian 静态 QEMU，进入 `podman unshare` 和私有 mount namespace，再挂载仅该 user namespace 生效的 binfmt_misc（Linux ≥6.7）。不写宿主全局 binfmt，不修改网络/系统设置，不使用平板。QEMU 与许可证只属于构建工具，不进入镜像/APK。
- 导出后交叉比对客体 manifest、tar 和属性表；打包器检查镜像格式与架构。zstd 最多两线程，压缩持 `artifacts/.gradle.lock` 与重构的重型编译互斥。
- `PYTHONPATH=image python3 -m unittest discover -s image/tests -v` 验证格式及错误拒绝；`image/tests/podman-roundtrip.sh` 验证实际用户、sudo、shell、解释器、编译器与重新打包。`image/tests/toolchain-profiles.sh CONTAINER` 检查多版本/空列表/激活边界，容器必须是可丢弃的测试实例；`python3 image/tests/inspect-artifacts.py IMAGE` 扫描实际 ELF 架构、envctl 源码摘要、Codex 链接并确认构建用 QEMU 不在镜像中。
- APK 按 ABI 打包且强制 workspace profile，校验索引、架构、大小和 SHA256；不允许 vanilla Debian 回退。具体设备接受状态以 [status.md](status.md) 与 `docs/report/rewrite/` 的证据为准，镜像构建成功不等于设备接受。
