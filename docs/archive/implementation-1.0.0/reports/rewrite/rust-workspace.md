# Rust Workspace Engine 交付与验证

日期：2026-10-03。范围：`engine/server/**`、协议补充、`docs/workspace-engine.md`。
初始 1.0.0 checkpoint 未改写；没有旧仓库导入、跨版本适配、host agent/terminal fallback 或 SSH 实现。

## 已交付实现

- `workflow-engine serve` 是真实的双向 JSONL JSON-RPC Server；APK 名称由集成层使用 `libworkflow-engine.so`。
  首条 hello 必须精确匹配 `workflow.workspace/1`。批处理、重复 JSON 键、损坏 JSON、超长帧被拒绝；
  未知方法 -32601、参数错误 -32602、业务错误 -32000 + `data.kind`。id 按 raw JSON 返回，已测试字符串与 120-bit 整数。
- Workspace 进程独占锁、串行权威 mutation、revision/watch、原子快照与备份；未知格式只读，损坏数据保留后恢复。
  磁盘变更轮询只重读发生 stat 变化的已跟踪文件；每小时运行归档/垃圾桶维护。
- Rust 布局 reducer/normalize 与当前 Kotlin oracle 对齐；会话、file draft、composer tombstone、冲突、归档保护、
  trash/restore、目录操作与权限保留均在 Server。命令返回权威 StateCodec 形状，客户端无需预计算布局。
- 二进制文件分块读/上传；上传不覆盖竞态中出现的目标文件。路径拒绝遍历、symlink 和私有状态目录。
  大小限制：单帧32MiB、原始块64KiB、文本16MiB、状态24MiB、单上传8GiB。
- Opaque documents 与 `services.<serviceId>/<key>` 原文件映射；纯文本服务配置和编辑器共享文件，
  CAS sidecar 仅记录 hash/revision。network 实测上报只生成 guest resolver；没有改 Android/主机 DNS 或路由。
- 环境后台构建直接调用指定 Rust runtime。APK 图片资产使用 pinned zip 8.6.0 解包并按 image sha 缓存；
  只接受 `workflow-image/2`、`workspace`、`debian-trixie/1` 的定制镜像。
- Python/Node 数组逐项安装，verify-many 实测后才执行 post_scripts/激活。环境配置变化期间到来的新配置重新 reconcile；
  失败指纹不会自动反复执行，显式 retry 可重试。旧 root generation、持久 home 和工具链/cache 保留。
- post_scripts 使用0700 home暂存；成功时只应用脚本 delta，CAS 检查并保留构建期间无关的 live home 编辑，
  用 Linux/Android renameat2 exchange 原子切换完整 home。成功/失败清理暂存，绝不把暂存纳入日志或发布产物。
- process.spawn/read/write/resize/stop/wait 均真实实现。PTY 使用 openpty；stdio 两个流各有4MiB ring，旧 offset 显式裁剪。
  父死亡信号绑定持续存在的 reaper 线程；显式 restart 只在存在 verified pending 时停止进程组并有界等待。
  Server 既不解析 vendor 协议，也不回答审批。用户环境变量通过 guest `/usr/bin/env` 应用，不注入 host runtime loader。

## 实际线格式

| 调用 | result 或 workspace.command 的 value |
| --- | --- |
| hello | `{protocol,engineVersion:"1.0.0",workspaceRoot,capabilities}` |
| workspace.snapshot/watch | `{revision,state}`；watch 最长30000ms，等待不持有状态锁 |
| workspace.command | `{revision,state,value}` |
| enterWorkbench/createSession/openInSeparateSession | session id 字符串 |
| activateSession/restoreSession/renameSession/pinSession/unpinSession | boolean |
| archiveSession | `kind=archived{savedPaths}/needsDecision{resources}/saveConflict{paths}/invalid{path,message}/failed{message}/notFound` |
| applyLayout | 完整 Workbench 或 null；placement/drop 的标签键为 `type` |
| openFile | `{path,disk,diskText,binary,tooLarge,draft}` |
| saveFile | `kind=saved{version,text}/unchanged{version}/conflict{disk}/invalid{message}/failed{message}` |
| editFile/resolveConflict/discardDraft/runMaintenance/dismissNotice/removeConversation | null |
| movePath/copyPath/deletePath/createFile/createDirectory | `kind=done/failed{message}` |
| trashPath / restoreFromTrash | `trashed{entry:{id,path,trashedAt,isDirectory}}` / `restored{path}`，均可 failed |
| listDirectory / purgeTrash / flush | FileEntry数组 / 清理条目数 / true |
| editComposer/acknowledgeComposer/discardComposer | 当前 composer `{format:1,conversationId,revision,text,attachments}` |
| updateConfig | `kind=updated{config}/blocked{problem}/failed{message}/conflict{revision}` |
| files.read | `{data,nextOffset,eof,size}`，data为base64 |
| files.upload.begin/chunk/commit/cancel | `{uploadId}` / `{nextOffset}` / file-op结果 / `{cancelled:true}` |
| documents.read/write | `{document,revision}` / `{revision}`，文档revision按键独立 |
| services.report/status | `{revision,serviceId,state}` / `{services:{id:{format,state,reportedAt}},desired}` |
| client.config | `{revision,config:{appearance,overlay,launcher,terminal}}` |
| environment.status/reconcile/restart | `phase,usable,progress,stage,error,runningProcesses,architecture` + active/pending/previous等记录 |
| process.spawn/read | `{processId}` / `{data,startOffset,nextOffset,eof,exitCode?}` |
| process.write/resize/stop/wait | `{written}` / `{rows,columns}` / `{stopping:true}` / `{running,exitCode?}` |

未知 workspace.command/LayoutOp 返回参数错误。布局中的缺失引用/非法目标保留原布局，遵循原 reducer 的 total-operation 约定。
服务路径的 namespace 必须为 `services.<安全标识>`，key 为单段安全文件名；示例：
`documents.write {namespace:"services.proxy",key:"config.yaml",document:"mode: direct\n",expectedRevision:0}`。

## 可重复验证及结果

所有 Cargo/Gradle 命令通过 `flock artifacts/.gradle.lock`，Cargo `-j 2`，Rust 1.93.1。

1. `engine/server/tools/layout-oracle.sh 400`：使用已编译 Kotlin `LayoutPropertyTest.randomOp`、
   `LayoutReducer`、`LayoutInvariants`、`StateCodec`，输出400个seed × 80步 = **32,000**个独立 before/op/after。
   `WORKFLOW_LAYOUT_ORACLE=$PWD/artifacts/workspace-engine/layout-oracle.jsonl cargo test ...`：
   **19/19通过**；逐条比较完整 Workbench（浮点容差1e-8），并检查 normalize 幂等。
   默认测试还内置54个分层抽取fixture，不依赖 Kotlin 编译产物。
2. `flock artifacts/.gradle.lock cargo test --manifest-path engine/Cargo.toml -p workflow-engine -j 2`：
   当前源码 **19 passed, 0 failed**。覆盖草稿跨重启/首键外部冲突、归档全部预检、最近引用保护、composer CAS/ack、
   路径/symlink保护、不覆盖文件、trash/move、损坏/未知schema、外部配置竞争、权限保留、orphan draft保护、
   环境schema、home delta冲突、无关live编辑保留及directory→symlink安全替换、多级服务资产安全路径。
3. `python3 engine/server/tools/test-protocol.py engine/target/debug/workflow-engine --runtime artifacts/engine/rust-host/engine --loader artifacts/engine/rust-host/loader --rootfs artifacts/engine/rust-host/rootfs`：
   **全部通过**。真实子进程验证 hello拒绝、未知method、raw id、并发watch、blob回读/上传竞态、documents CAS、
   原文件服务文档与编辑器一致、DNS IP验证、跨进程草稿恢复；lifecycle fixture验证慢构建期间会话响应、重试与spawn/restart竞争。
   同一脚本还使用**真正 Rust runtime**验证原始UTF-8/vendor审批形状字节不变、PTY初始/动态尺寸、cwd、私有目录mask、
   5MiB输出裁剪、stop/wait。fixture结果不冒充真实容器结果。
4. `python3 -u engine/server/tools/test-environment.py engine/target/debug/workflow-engine engine/target/release/workflow-runtime artifacts/engine/rust-host/loader artifacts/image/amd64/image.tar.zst artifacts/image/amd64/image.json`：
   **真实定制镜像完整生命周期通过**。Python **3.14.7**、Node **24.21.0**；work脚本写入`~/.config`在正式进程可见；
   失败脚本的home变更不发布；node=[]返回127且不落到系统Node；活跃进程使新generation pending，显式restart停止它再切换；
   Server重启保持generation、home及工具链缓存。全过程仅操作临时工作区和guest，没有设备或系统设置操作。

证据（ignored）：`artifacts/workspace-engine/{unit-final.log,unit-current.log,protocol-final.log,real-environment.log,layout-oracle.jsonl}`。

Oracle SHA256：`3b033a7f0d656b7920f9aaa10a1e220d07e77dff49ccc35e725234ce1afecb32`。
Kotlin LayoutReducer SHA256：`30e7be8ce6168fcf44ef754706f92ade388bf2eebfffec15179c31b9f01b0a26`；
LayoutInvariants：`477bedb2db3d4670ec51b5b0adbc5b13571b66958fe6095db7c05755fc8bb997`。
真实环境完整run使用的host Server SHA256：`e1250835ba4338a9153368776190846a67db459fb705c4ac016f2c8f52a145ee`；
最终仅放开显式服务资产分块上传后的Server SHA256：`26f46b08447a4b89327c58bdfd95d8b0358a905bf8dc43a95da644e589d9dddb`，
对应JSONL/服务资产回归见 `artifacts/workspace-engine/service-assets.log`（通过）；
Rust runtime：`c3cee7bd56baac53ad66dbf69a994a8aba06e72086f1b87b7f77900018b2f855`；
定制amd64镜像：`5d49db08277ba4af6ab4fe72ecb5713ee845cff8b3971e452e874a8f442e4d60`。

## 已审查修复与集成边界

独立只读审查发现并修复了：spawn/restart锁顺序死锁、原子写丢可执行位、move吞掉目标未保存draft、
持久化失败导致building永久卡住，以及post_scripts的home被持久home覆盖的问题；相关回归均列入测试。

本报告不把host测试当设备验收：完整Android连接恢复、服务能力与真实Codex/Claude会话由root集成验收。
Server没有实现远程transport、generation自动垃圾回收或任意大小快照流式化；旧generation保守保留。
超过32MiB的组合snapshot/openFile响应明确报too_large，较大二进制仍可通过files分块API处理。
