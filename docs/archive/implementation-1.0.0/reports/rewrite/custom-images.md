# 1.0.0 定制镜像交付与验证

日期：2026-10-03。范围：`image/**`、构建工具 `third_party/qemu-user/**` 与环境文档；Rust runtime/server、Android、Gradle 由各自负责人集成。两种架构均已交付真正预配置的 workspace 镜像，外层 formatVersion 2、debian-trixie typeVersion 1 未改变。

## 交付产物

| 架构 | 文件 | 字节数 | SHA256 |
| --- | --- | ---: | --- |
| amd64 | `artifacts/image/amd64/image.tar.zst` | 189461736 | `5d49db08277ba4af6ab4fe72ecb5713ee845cff8b3971e452e874a8f442e4d60` |
| arm64 | `artifacts/image/arm64/image.tar.zst` | 177861645 | `63d5d1a4cd549a573639b709ce45d7c5b21a39a3c862b149a31c3f82f87ab92a` |

每个目录同时包含可信索引 `image.json`、`image.tar.zst.sha256`、`image.packages.tsv`、`image.state.json`、`image.architecture.json`、`image.build.log` 和 `rewrite-roundtrip.log`。amd64 203 个 Debian 包、28475 属性行、899778068 普通文件字节；arm64 202 个 Debian 包、28581 属性行、874894791 普通文件字节。差异来自目标架构的软件包与二进制；完整包版本在对应 TSV 中。

两镜像均包含：work 1000:1000、登录 Bash、无密码 sudo、en_US.UTF-8、curl/wget/git/nano/unzip/zip/rsync/CA/gnupg/locales/tzdata/build-essential/cmake/xz-utils/zstd/jq，uv 0.12.19、nvm 0.40.8、Python 3.14.7、Node 24.21.0（含 npm）。默认规格为 Python `3.14`、Node `24`，完整默认 profile 为 `py3.14.7-node24.21.0`。镜像内 `/usr/local/bin/codex` 指向 `/opt/workflow/bundled/libcodex.so`；实际 Codex 二进制由 APK/Server 挂载提供。

## 多版本与激活契约

`envctl python SPEC`、`node SPEC` 只安装到持久 store，已有匹配版本不下载，均不改变 active。nvm 在独立子 shell 中加载，安装过程中临时的 nvm use 不影响运行环境。`verify-many PY_JSON NODE_JSON [PKG...]` 校验数组、全部选定解释器与软件包，输出完整已解析数组及不可变 profile。

非空首项是默认；全部选择都能经 `pythons/X.Y.Z/bin/python3`、`nodes/A.B.C/bin/node` 运行。多版本、次序、次要版本及空列表均参与 profile 身份计算；多/空变体使用 SHA256 名，单对版本保留可读名称。空语言入口包含退出码 127 的 guard，即使系统 PATH 存在解释器也不会默认回退。jq 属于镜像固定包，因此 JSON 解析不依赖任何启用的 Python/Node。`activate` 才原子切换 active；`state` 保留当前标量默认并报告 `installedPython`/`installedNode`。

缺省列表取镜像默认，显式空数组禁用托管语言。`.workspace/env.json`、post_scripts 顺序/记录/重试、generation 及运行实例归 Rust Server；envctl 不执行 post_scripts。镜像内基础包清单用于防止 reconcile 删除固定镜像包。

## 构建路径与隔离

执行：

```sh
image/build.sh --arch amd64 --keep
image/build.sh --arch arm64 --keep
```

arm64 使用 `image/with-cross.sh`，在 `podman unshare` 的 rootless user namespace 内创建私有 mount namespace，再挂载该命名空间专属 binfmt_misc。静态 QEMU 来自 Debian qemu-user `1:10.0.13+ds-0+deb13u1`，包 SHA256 `ca6ede739327a20ae5a3498230c8a3a9a072c586e131bc6bcc1201eb1725e336`；下载固定在 manifest，构建前验证 SHA256 后提取，并保留组件版权和 GPL 文本。主机需要 Linux >=6.7 的 user-namespace binfmt 支持。没有全局 binfmt 注册、主机系统/网络设置变化或平板操作。

构建后主机 `/proc/sys/fs/binfmt_misc` 仍只有原有 windows/windowsPE 和 register/status。QEMU 仅位于忽略的缓存，镜像扫描确认没有带入 `/usr/bin/qemu-aarch64`。所有镜像压缩最多两线程并持共享 `artifacts/.gradle.lock`。

两种架构均实际 provision 两遍：amd64 第一遍 61 秒、第二遍 0 秒；arm64 第一遍 306 秒、第二遍 5 秒。第二遍没有下载，完整工具链/包状态与第一遍逐字节相同。随后交叉核对客体 manifest 与导出 tar，再按标准格式打包、完整校验摘要/属性/顺序/架构。该证据证明声明状态幂等；这些发布文件的准确内容以所列 SHA256 为准。

## 验证结果

```sh
PYTHONPATH=image python3 -m unittest discover -s image/tests -v
image/tests/podman-roundtrip.sh
image/with-cross.sh arm64 image/tests/podman-roundtrip.sh artifacts/image/arm64/image.tar.zst --skip-repack
python3 image/tests/inspect-artifacts.py artifacts/image/amd64/image.tar.zst
python3 image/tests/inspect-artifacts.py artifacts/image/arm64/image.tar.zst
```

- 27 个宿主格式/属性/错误拒绝/脚本测试通过；日志 `artifacts/image/rewrite-unit-tests.log`。
- amd64、arm64 导入容器后的完整 smoke 与 envctl 流程均 `0 failure(s)`：work uid/gid/home、sudo su、setuid 模式、locale、默认 Python/Node/npm、uv/nvm、sudo PATH、C 编译并运行、uv venv 与项目版本 pin 全部通过。
- 两架构都真实下载并运行 Python 3.13.15、Node 22.23.3；重复安装返回 installed=false。安装和 verify 保持原 active，显式 activate 切换正确，旧 profile 回滚成功。
- 多版本全部可运行；次要版本移除、排序改变会产生不同 profile；验证不改变 active。Python 空、Node 空、两者空都通过，且测试额外注入系统解释器后仍返回 127，不会偷偷回退。重复/非法/未安装规格和缺失包均拒绝，失败不改变 active。
- apt-install 可重复、额外 ripgrep 可移除、镜像内 git 保留；所有临时工具链目录最终为空。
- amd64 在客体内重新打包后，attributes.tsv 和除 createdAt/builder 外的 metadata 与原镜像相同。arm64 已完成客体配置/执行、host 打包与完整标准验证；仅显式跳过慢速 QEMU 内再次 zstd 压缩。
- 完整镜像 ELF 扫描：amd64 的 949 个 ELF 全部 e_machine=62；arm64 的 944 个 ELF 全部 e_machine=183。扫描包括 Bash、uv、Python、Node；Codex 符号链接目标正确，镜像中的 envctl 与源码 SHA256 均为 `8a2e566634190fb296b5f742f79b2cf6c71d0ebd0fd9e8fbc8d53855d02b8158`。

本报告验证的是发布镜像与实际 Linux/QEMU 客体行为。Android APK 集成、Rust runtime 运行及设备接受由主任务的独立证据覆盖，未用容器结果替代设备接受。
