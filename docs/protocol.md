# Workspace Engine 协议

此文件与 `engine/protocol/contract.json` 是本轮 Server/Android 并行实现的固定边界。应用版本维持 1.0.0。所有新工作区配置和状态位于 **工作区根/.workspace/**；不读取或迁移旧 `.workflow`、旧根 `.workspace`、container.json。

## 1. 连接

本轮实现本机进程的双向 stdio；只定义将来的远程 bootstrap/transport 接口，不实现 SSH。借鉴 [VS Code Remote SSH](https://code.visualstudio.com/docs/remote/ssh) 的客户端/工作区服务分离，业务代码不得依赖远程方式。

UTF-8 JSON-RPC 2.0，一行一个对象，不支持批处理。请求 id 必须原样返回；stdin/stdout 仅用于协议，诊断写 stderr，禁止凭据。单帧32MiB，二进制按64KiB原始块base64传输。未知方法返回-32601，参数错误-32602，业务错误-32000及结构化data.kind；不得静默成功。首条必须 `hello {protocol:"workflow.workspace/1",clientId:"..."}`，返回同protocol、engineVersion、workspaceRoot、capabilities。不匹配立即拒绝，不做跨版本兼容。

CLI：`workflow-engine serve --root <用户文件根> [--runtime <workflow-runtime>] [--loader <workflow-loader>] [--apk <Android APK>] [--native-dir <nativeLibraryDir>] [--image <image.tar.zst> --image-index <image.json>]`。Android包中分别为 `libworkflow-engine.so`（Server）、`libworkflow-runtime.so`（Rust容器）、`libworkflow-loader.so`。运行时CLI继续支持现有run/install/verify/fsck/probe子命令语义，便于复用验收oracle。

## 2. 权威状态

`workspace.snapshot {}` -> `{revision,state}`。
`workspace.watch {afterRevision,timeoutMs}` -> 同形状，timeoutMs限0..30000；等待期间不得持有状态锁或阻塞其他请求。
`workspace.command {name,args}` -> `{revision,state,value}`。所有变更由Server串行提交，提交成功后返回权威快照；文件IO不在Android主线程。客户端只维护可丢弃的StateFlow投影，不再用Kotlin仓库落盘。

state字段：`status`（ready/failed）、`failure`、`sessions`、`activeSessionId`、`pinned`、`drafts`（path→对象）、`composers`（resource id→对象）、`disk`（path→DiskVersion）、`config`（完整配置文档）、`configProblem`、`notices`、`writeError`。Session、Workbench、PanelTarget/View、DiskVersion、draft/composer的对象形状以当前 `core/.../store/StateCodec.kt` 的编码形状为本轮固定线格式；这是新协议的定义，不是旧数据导入。只接受当前格式。后端插件的opaque文档另走documents，不把Codex/Claude协议或实现塞入Server core。

命令name与args（返回value沿用同名Kotlin WorkspaceStore方法的含义；下面是协议标识，不通过反射执行任意方法）：

| name | args |
| --- | --- |
| enterWorkbench | {} |
| createSession | {name?} |
| openInSeparateSession | {target} |
| activateSession / restoreSession / unpinSession | {id} |
| renameSession | {id,name} |
| archiveSession | {id,decision?}，decision=save_all/keep_drafts/discard；有草稿且未决定返回needsDecision |
| pinSession | {id,index} |
| runMaintenance / flush | {} |
| dismissNotice | {id} |
| applyLayout | {sessionId?,op}，缺省当前Session |
| openFile | {path} |
| editFile | {path,text,shown} |
| saveFile | {path,text?} |
| resolveConflict | {path,resolution}，keep_mine/use_disk |
| discardDraft / deletePath / trashPath / createDirectory | {path} |
| movePath / copyPath | {from,to} |
| restoreFromTrash | {id} |
| purgeTrash | {} |
| createFile | {path,data}，base64；大文件用upload |
| listDirectory | {path,showHidden} |
| editComposer | {draft,expectedRevision} |
| acknowledgeComposer | {submitted} |
| discardComposer / removeConversation | {conversationId}；移除必须来自用户确认 |
| updateConfig | {config,expectedRevision}，版本竞争返回conflict，客户端重新应用用户transform后重试 |

结果对象用 `kind` 标签：save= saved{version,text}/unchanged{version}/conflict{disk}/invalid{message}/failed{message}；archive= archived{savedPaths}/needsDecision{resources}/saveConflict{paths}/invalid{path,message}/failed{message}/notFound；file操作=done/failed{message}；trash=trashed{entry}/failed；restore=restored{path}/failed；config=updated{config}/conflict{revision}/blocked{problem}/failed；conflict 仍是成功 command envelope，客户端先接收其权威快照，再对最新配置重新应用 transform。ResourceRef编码为 `{kind:"file",path}` 或 `{kind:"conversation",id}`。openFile返回 `{path,disk,diskText,binary,tooLarge,draft}`；listDirectory为 `{path,isDirectory,size,modifiedAt}` 数组。

LayoutOp 的 `type` 为现有类名的小驼峰（open/focus/focusStack/close/move/splitStack/resizeSplit/resetSplit/resizeRegion/setRegionCollapsed/toggleRegion/setMaximized/switchParadigm/promoteConversation/returnToFiles/enterSolo/setChatSideStack/retarget/updateView/updateExplorer/renamePath）；字段与类构造参数同名。枚举小写。placement标签为auto/inStack/splitEdge，drop标签为center/tab/edge/editorEdge。必须保持当前布局不变量和草稿归档保护，不以客户端预先计算的布局充当权威结果。

## 3. 文件与服务文档

- `files.read {path,offset,length}` -> `{data,nextOffset,eof,size}`，长度上限65536；默认禁止逃出根和内部私有目录，仅显式配置编辑允许 `.workspace/config.json`、`.workspace/env.json`、服务配置。
- upload.begin `{path,size}` -> `{uploadId}`；chunk `{uploadId,offset,data}` -> `{nextOffset}`；commit `{uploadId}` -> file操作结果；cancel `{uploadId}`。原子提交；断线未提交文件留在受限暂存目录并清理，不能覆盖同名用户文件。客户端导入端口显式接收长度和 InputStream 工厂；已知字节/暂存文件直接提供长度，以 64KiB 块流式传输，源长度变化、异常或取消均取消 upload，不在客户端另存工作区正文。
- documents.read `{namespace,key}` -> `{document:字符串或null,revision}`；write `{namespace,key,document,expectedRevision?}` -> `{revision}`；quarantine同键。路径由Server构造，namespace/key限安全标识，禁止任意路径。聊天索引使用namespace=chat,key=conversations；Local proxy状态也记录为服务文档。配置和索引IO由Engine拥有。
- services.report `{serviceId,state}` 为本机能力执行器的实测结果；Server拥有发布/持久化的状态，App不能把开关当成功。proxy为明确的本机服务，工作区配置是唯一配置源。
- client.config `{clientId}` 返回工作区下发的本地配置（外观、Launcher、悬浮窗等）；App仅可缓存这份配置与连接配置。

## 4. 运行环境与进程

Environment是完整运行环境，包含工具链/包/变量/挂载/进程与生命周期；env.json是声明手段。配置位于 `.workspace/env.json`：`{version:1,python:["3.14"],node:["24"],packages:[],env:{},post_scripts:[]}`。缺省语言列表用定制镜像defaults；空列表表示不启用该语言；非空列表首项为默认，其余同装。禁止重复/无效版本。post_scripts为`{id,run,user:"work"|"root"}`，按顺序、仅新配置构建执行，成功后才激活；失败保持旧环境，并可显式重试。default root是虚拟guest root，不是设备root。

首次使用环境时请求 reconcile；之后 Server 自动监测已保存或外部修改的 env.json，并在重连后检查变更。仅文件/代理服务连接不会自动解压未使用的环境；失败输入不会自动重复运行。`environment.status` 返回健康状态；`environment.reconcile {retry?}`启动构建；`environment.restart {}`仅切换已经验证成功的generation。长任务必须发出可查询进度，不阻塞文件/Session。保留home、用户文件、工具链缓存及旧可用generation；有运行进程时待用户重启，无进程时直接激活。

所有终端/编码代理进程由Engine管理且强制在environment运行，不能提供host exec fallback。

- process.spawn `{argv?,cwd:"/workspace",env:{},terminal:false,rows:24,columns:80,label?}` -> `{processId}`。terminal=true分配PTY；argv缺省登录bash/work。使用APK传入的runtime/loader路径，无shell拼接。
- process.read `{processId,stream:"stdout"|"stderr",offset,maxBytes,waitMs}` -> `{data,startOffset,nextOffset,eof,exitCode?}`；有界ring buffer，旧offset明确报告裁剪起点；waitMs≤1000。
- process.write `{processId,data}`；resize `{processId,rows,columns}`；stop `{processId,force?}`；wait `{processId,timeoutMs}` -> `{running,exitCode?}`。stdio数据原样传输，不消费/自动批准vendor协议请求。

## 5. 连接接口边界

Android持有 `WorkspaceConnectionConfig`、`WorkspaceTransport` 与 `WorkspaceBootstrapper`。本轮只有Embedded实现；远程仅有接口与配置类型，调用未实现的transport明确unsupported，不伪造可用连接。启动必须先选择并保存连接配置，再启动Engine/获取workspace状态。断开后工作区变更禁用，草稿已提交的部分由Engine保留；重连拿权威快照。没有静默本地仓库回退。连接失败会清除当前连接、取消整组会话消费者并释放对应前台服务租约；旧 projection 标记 failed，禁止后续变更。Embedded bootstrap 必须通过同一 WorkspaceBootstrapper 接口提供 transport。客户端缓存仅保存 `client.config` 下发的 appearance/launcher/overlay/terminal，与连接 id 绑定；连接页可使用这些外观设置，不能据此重建会话、草稿或工作区配置。

### 服务纯文本文档

`documents.*` 的 namespace 为 `services.<serviceId>` 时，key 是安全的单段文件名，
对应唯一原文件 `.workspace/services/<serviceId>/<key>`，例如
`{namespace:"services.proxy",key:"config.yaml"}` 与文件编辑器路径
`.workspace/services/proxy/config.yaml` 指向同一内容；`runtime.log` 同理。
Server 只保存 hash/revision sidecar，不另复制文档正文。读取或 CAS 写入前检查原文件，
编辑器/外部更新使旧 expectedRevision 失效。内容为最多 16MiB UTF-8 纯文本，格式由服务解释；
日志写入方必须先脱敏。serviceId/key 均为安全标识，不接受斜线和 `.`/`..`。
显式文件 API 另允许 `.workspace/services/<serviceId>/<安全相对路径>` 的多级文本/二进制服务资产；
每段均为安全标识，仍拒绝 symlink 与父级跳转。documents 的 key 保持单段。

本机网络执行器上报 `services.report {serviceId:"network",state:{dnsServers:[IP字符串],connected:boolean}}`。
Server 验证 IPv4/IPv6，写工作区内部 0644 resolv.conf，并仅绑定到 guest `/etc/resolv.conf`；
不修改 Android/主机的系统 DNS、路由或其他应用。

`environment.status` 提供 `phase`（not_installed/installing/building/ready/needs_restart/failed）、
`usable`、`progress`（0..1 或 null）、`stage`、`error`、`runningProcesses`、`architecture`。
已有经过验证的 pending generation 时，显式 restart 先停止 Engine 管理的进程组，
最多等待 10 秒再切换；没有 pending 时返回当前状态且不停止进程。
post_scripts 的 home 写入私有 0700 暂存；仅成功激活时按内容版本检查合并，保留构建期间
未被脚本修改的用户 home 变更。冲突保留旧 generation 并报告失败；暂存不进入备份/发布产物。
