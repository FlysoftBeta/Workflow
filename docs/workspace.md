# 工作区状态：Session、布局与 Working Resource

用户可见行为以 [product.md](product.md) §2–§6 为准，视觉与尺寸以 [ui.md](ui.md) 为准。本文件规定共享状态模型、不变量与布局语义。Rust `engine/server` 执行并持久化所有命令；`:core` 提供客户端协议/模型与参考布局纯函数。旧 `ReferenceWorkspaceStore` 和环境 writer 仅保留在 test fixtures。

## 1. 路径与所有权

- 跨模块使用工作区相对路径：`/` 分隔，无前导 `/`，没有 `.` 与 `..` 段；空字符串表示根。绝对路径、父级跳转与 symlink 穿越被拒绝。
- 用户文件位于工作区根，配置和私有状态位于 `<workspace-root>/.workspace/`；文件浏览器始终隐藏该内部目录。
- 文件 API 只允许显式访问 `.workspace/config.json`、`.workspace/env.json` 和安全服务文件路径；其余私有状态不可打开、保存或附加。guest 内隐藏状态、environment、documents、上传暂存、corrupt、trash 与 Engine 锁；显式配置/服务路径由 Engine 管理。
- 连接内的 `WorkspaceStore` 是命令/快照接口；Android 不另存会话、草稿或布局。RPC 编码、失败与 CAS 语义见 [protocol.md](protocol.md)。

## 2. 模型

**Session**

- 由 id、名称、创建与最后使用时间、使用统计、归档时间和 Workbench 组成。
- **有名称即持久，无名称即临时**；命名会把临时 Session 转为持久，空名称被拒绝。
- Session 不含任何未保存内容。

**使用统计**

- 以"使用片段"计数：激活 Session，或间隔 ≥30 分钟后再次交互，各算一次。
- 另存一份指数衰减计数（半衰期 7 天）。快速来回切换不会抬高频率。

**持久 Session 排序**

- 分数 = 0.65 × 最近度 + 0.35 × 频率。
  - 最近度：1 小时内为 1；之后按对数衰减，到 7 天为 0。
  - 频率：`ln(1+衰减计数)` 除以列表中的最大值，即归一化。
- 分数相同时依次比较最后使用时间、id，结果确定。
- 用户拖动过的项放入置顶组（`pinned`），按拖动顺序排在自动排序之前。

**临时 Session 时间轴**

- 按最后使用时间所在的日历日分组：今天 / 昨天 / 本周（2–6 天前）/ 更早。时区由调用方传入。
- 每组带 `fade` 与 `maxResources`，界面据此逐级淡化并减少显示的信息。

**搜索**

- 查询按空白切分成词，每个词都必须命中名称或某个资源：文件路径与文件名，或调用方给出的标题，例如对话标题。
- 名称命中的排在前面，其次是未归档的，再按最后使用时间排序。

**Working Resource**

- 文件草稿按规范路径存放，每个路径一份，所有 Session 共享。草稿包含文本、基准磁盘版本 `DiskVersion(exists, size, mtime, sha256)`、编辑时间和修订号。
- 对话输入框按对话 id 存放文本与待发送附件（工作区路径）。修订语义：
  - 只有内容变化才递增修订号；
  - 发送后的确认必须与所有者、修订号和内容完全一致才清空；
  - 过时的快照会被拒绝；
  - 清空后以空的"墓碑"保存，修订号永不回退。
- 文件状态是推导出来的：
  - 无草稿 → CLEAN；
  - 磁盘等于基准 → DIRTY；
  - 磁盘已变 → CONFLICT；
  - 磁盘上已删除 → DELETED。

## 3. Workbench（布局 v2）

一个 Session 只有一个 Workbench，它同时保存三种 Paradigm 的排布。

**Panel**

- 由稳定 id、目标和视图状态组成。
- 目标类型：文件、图片、对比、终端、对话、代理（概览 / 日志 / 连接）、设置。
- 视图状态：滚动锚点与偏移、光标、`extras`（图片缩放、设置分类等）。
- 每个目标有一个 `key`，文件与图片共用 `file:<path>`。

**Stack**

- 一组有序 Panel 加上激活的那一个。
- 编辑区是 Stack 的 n 叉拆分树，节点有方向与权重。
- `bottom`（终端）与 `aux`（对话）是两个固定的单一 Stack，不能拆分，也不会被删除。
- 文件浏览器是固定区域，不是 Panel。

**各 Paradigm 的独立状态**

- **Files**：浏览器、辅助区、底部区域各自的尺寸与收起状态；焦点；最大化的 Stack。
  - 侧区尺寸单位为 dp，底部为列高的比例；按窗口的限制在渲染时处理。
- **Chat**：对话列表栏、侧区、侧区内的树列；侧区显示哪个 Stack；焦点；被提升对话的来源 `promotedFrom`。
- **Solo**：显示的 Panel，以及离开后回到哪个 Paradigm。

**跨 Paradigm 共享**

- Panel 与 Stack 为所有 Paradigm 共用。
- `lastEditorStack` 是最近获得焦点的编辑 Stack：打开文件的默认位置，也是 Chat 侧区默认显示的 Stack。
- 浏览器状态：展开的目录与选中项。
- MRU：按最近焦点排列的 Panel。

**不变量**（`Workbench.violations()`，测试逐条检查）

- 每个 Panel 恰在一个 Stack 中；同一 `key` 在一个 Session 内至多打开一次。
- 编辑树至少有一个 Stack，且每个编辑 Stack 只出现一次。
- `bottom` 与 `aux` 存在，并且不在编辑树中。
- 非空 Stack 有激活项，空 Stack 没有。
- 除两种情况外，不存在空的编辑 Stack：它是唯一的编辑 Stack，或是被提升对话的来源。
- 拆分节点至少有两个子节点，不直接嵌套同方向的拆分；权重有限、每个 ≥0.05（或 1/n）、总和为 1。
- 各焦点、Chat 侧区 Stack、最大化 Stack、Solo 与 `promotedFrom` 都引用有效对象。
- 处于 Files 时没有 `promotedFrom`。

## 4. LayoutOp

Server 的布局 reducer 与 Kotlin `Workbench.apply(op)` 参考语义一致，是纯函数，也是全函数：

- 引用不存在或非法的对象时原样返回同一实例，从不抛出。
- 每次结果都经过规范化，并且规范化是幂等的。
- 拖放在松手时只提交一个 op。
- 新 id 由 `nextSeq` 确定性生成。终端与对话的 id 由各自的拥有者提供。

| Op | 语义 |
| --- | --- |
| `Open(target, placement, focus)` | 已打开则聚焦，不重复打开；未打开则按放置规则插入到激活项之后（见下文）。 |
| `Focus` / `FocusStack` | 激活 Panel，聚焦其所在 Stack，并让它可见：展开被收起的底部区或辅助区；聚焦其他 Stack 时取消最大化；Chat 中把侧区切到该 Stack。在 Solo 中聚焦其他 Panel 会离开 Solo。 |
| `Close(ids)` | 不影响草稿。Stack 改为激活 MRU 中最近的剩余项。空的编辑 Stack 被移除，拆分随之合并，焦点按 MRU 转移。终端全部关闭时底部区收起，焦点回到编辑区。 |
| `Move(id, Center\|Tab\|Edge\|EditorEdge)` | `Tab` 的索引按当前显示计算，包含被拖动的 Tab。`Edge` 在目标的该边拆出新 Stack，同方向时并入父节点并平分目标的权重；拖到 bottom/aux 的边等同于中心。`EditorEdge` 在整个编辑区外缘新建一列或一行。 |
| `SplitStack(stack, edge)` | 把激活项移入新拆分，要求至少两个 Panel。 |
| `ResizeSplit` / `ResetSplit` / `ResizeRegion` / `SetRegionCollapsed` / `ToggleRegion` / `SetMaximized` | 拆分权重规范化并钳制；区域尺寸限制在各区域的上下限内；最大化同时聚焦该 Stack。 |
| `SwitchParadigm(FILES\|CHAT)` | 进入 Chat 时，侧区显示 `lastEditorStack`（如果它为空，改用最近有内容的编辑 Stack），焦点在主列。回到 Files 时恢复 Files 的全部排布，包括焦点。 |
| `PromoteConversation(id)` | 对话若不在 aux，先移入 aux，并记录来源（Stack、位置、aux 原来的激活项）。来源 Stack 在 Chat 期间即使为空也保留。然后进入 Chat。 |
| `ReturnToFiles` | 被提升的对话回到原来的位置，aux 的激活项复原。若该对话已关闭或已被移走，则不再归位。 |
| `EnterSolo(target)` | 只显示一个 Panel，并记录来源 Paradigm。打开或聚焦其他内容时回到来源 Paradigm（新建的 Solo Session 回到 Files），原 Panel 成为一个 Tab。 |
| `SetChatSideStack` | 侧区的 Stack 切换器，只接受编辑 Stack 或 bottom。 |
| `Retarget(id, target)` | 原地换成另一个目标（对话切换器）。目标已在别处打开时改为聚焦那里。 |
| `UpdateView` / `UpdateExplorer` | 只改视图状态，不改变焦点或 MRU。 |
| `RenamePath(from, to)` | 文件与目录改名后，面板目标和浏览器路径跟随；发生冲突时保留被改名的那个 Panel。 |

**`Open` 的放置规则**

- `Auto`（默认）：
  - 文件、图片、对比、代理、设置 → 当前焦点所在的编辑 Stack。
    - Files 中若焦点不在编辑 Stack，使用 `lastEditorStack`。
    - Chat 中使用侧区的 Stack。
  - 终端 → `bottom`。
  - 对话 → `aux`。
- `InStack`：放入指定的 Stack。
  - `replaceActive` 只替换同类型的激活项，用于对话切换器。
  - `moveExisting` 把已在别处打开的目标移过来。
  - `LayoutOp.showConversation(id)` 就是用这两项打开到 `aux`。
- `SplitEdge`：在指定 Stack 的某一边拆出新 Stack 并放入。
- 在 Solo 中打开其他目标会先转为 Files。

## 5. 命令、持久化与恢复

Server 是唯一权威状态源。每个变更串行计算，原子持久化成功后确认新 revision；客户端以返回快照更新可丢弃的 `StateFlow`。读流、watch、环境构建和等待进程在锁外进行，不阻塞文件与会话命令。断开连接后不再提交工作区修改，重连取新快照。

会话、布局、文件草稿、composer 共存在 `.workspace/state/workspace.json`；后端索引使用 opaque documents。Server 持有工作区进程锁，写临时文件、fsync、原子 rename 并同步目录；保留有效备份，损坏原文放入 `.workspace/corrupt/`。未知格式保留只读，不做迁移。外部配置无效时保留最后有效配置并报告 `configProblem`，不得覆盖用户原文。

保存先检查当前磁盘版本和草稿基准；冲突返回明确结果。外部更改的干净文件刷新磁盘投影；有草稿则保留正文并显示冲突。保留我的更新基准，使用磁盘丢弃草稿，对比打开 Diff 面板。Server 定期检查跟踪文件并运行维护；不依赖 Android FileObserver 作为第二状态来源。

自动归档阈值仍为临时 1 天、持久 7 天；每个脏资源最新的未归档引用受保护。手动归档必须选择 save_all/keep_drafts/discard；save_all 先检查所有文件，任一冲突均不执行归档，对话输入保留。composer 的发送确认必须匹配 owner、revision、正文与附件，空墓碑保持 revision 单调。

`.workspace/config.json` 是当前 `version: 2` 的工作区设置，包含 appearance、agent、overlay、launcher、terminal；更新保留未知键并使用 expectedRevision。`.workspace/env.json` 单独定义完整环境，见 [environment.md](environment.md)。没有旧状态扫描、导入、搬移或跨版本适配。

Explorer 的 showHidden、阅读位置与光标随 Session 布局保存。编辑器 Tab 拖入聊天或终端传递文件，拖到 Tab 行移动面板；可访问的操作菜单提供等价动作。所有文件读写、拖入导入与附件读取经 Engine 服务端口。

具体容量、原子上传、文档 CAS 和恢复规则见 [workspace-engine.md](workspace-engine.md)；测试证据见 [status.md](status.md)。
