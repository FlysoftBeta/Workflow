# 工作区状态：Session、布局与 Working Resource

用户可见行为以 [product.md](product.md) §2–§6 为准，视觉与尺寸以 [ui.md](ui.md) 为准。本文件规定 `:core` 中的状态模型、不变量、`LayoutOp` 语义、持久化格式。代码位于 `core/src/main/kotlin/top/flysoftbeta/workflow/core/`：

| 包 | 内容 |
| --- | --- |
| `io` | 文件系统与监视端口、路径规则 |
| `layout` | Workbench、Panel、Stack、LayoutOp |
| `session` | Session 与排序、归档策略 |
| `resource` | 草稿与冲突 |
| `config` | config.json 与 Launcher |
| `store` | WorkspaceStore、持久化 |

## 1. 路径

- 跨模块的路径一律是**工作区相对路径**：`/` 分隔，无前导 `/`，没有 `.` 与 `..` 段；`""` 表示根。
- 实现按规范路径解析，经符号链接逃出工作区的路径视为外部路径并拒绝。
- 保留路径不能作为用户文件打开、保存或附加：
  - `.workflow/**`，但 `.workflow/proxy/**` 除外（代理配置经显式动作打开）；
- 文件浏览器始终隐藏 `.workflow`。

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

`Workbench.apply(op)` 是纯函数，也是全函数：

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

## 5. WorkspaceStore

**执行模型**

- 进程内唯一，由 `AppGraph.workspaceStore(context)` 提供。它是 `.workflow/state` 的唯一写入者，也是 Working Resource 的唯一拥有者。
- 命令在一个串行 actor 上逐个执行，IO 在 IO 调度器上完成，调用方线程从不阻塞。
- `state: StateFlow<WorkspaceState>` 是唯一的快照，没有变化的部分保持原实例。
- `layout(...)`、`editFile(...)` 立即返回，其他命令是 `suspend` 并返回结果。

**持久化**

- 按文件防抖：会话 500ms，草稿 400ms；持续变化时最迟 2s 也会写入。
- 写入失败时记入 `writeError`，按 1s/5s/15s/60s 重试。
- 应用进入后台时调用 `flush()`。
- 输入文字只重写该路径的草稿文件，不重写会话清单。

**保存**

- 保存前重新读取磁盘版本并比较哈希，磁盘相对草稿基准有变化时返回 `Conflict`，绝不覆盖。
- 保存前校验：config.json 始终校验；其他路径可经 `StoreOptions.validators` 追加，例如 container.json。

**外部修改**

- `FileWatcher` 监视所有被跟踪文件的父目录。被跟踪的文件包括：有草稿的文件、未归档 Session 中打开的文件、config.json。事件防抖 150ms，再读取版本确认。
- 干净文件只更新 `disk`，编辑器据此刷新内容。
- 有草稿时保留基准，状态变为冲突，由用户选择：
  - **保留我的**：把基准改为当前磁盘版本；
  - **使用磁盘版本**：丢弃草稿；
  - **对比**：打开 `Diff` Panel。
- 外部写入的内容恰好等于草稿时，草稿自动消失。
- 恢复到前台时调用 `runMaintenance()`：自动归档并重新检查被跟踪的文件。

**归档**

- 自动归档：临时 Session 1 天、持久 Session 7 天未使用即归档。每个有未保存内容的资源，只有引用它的**最新**未归档 Session 受保护：比较最后使用时间，再比较创建时间，最后比较 id。
- 手动归档时，只要涉及未保存内容，就返回 `NeedsDecision`，由用户选择一项：
  - `SAVE_ALL`：保存全部文件，对话输入保留。任一文件冲突时整体不执行，返回 `SaveConflict`。
  - `KEEP_DRAFTS`：草稿全部保留。
  - `DISCARD`：丢弃文件草稿；对话输入清空并推进修订号。

**配置**

- config.json 的写入保留未知键，采用 2 空格缩进。
- 解析失败时继续使用最后一次有效的配置（`state/config.last-good.json`），在 `configProblem` 中给出带键路径的原因，并拒绝设置界面的写入（`Blocked`），不覆盖用户的文件。
- 文件的各个键：
  - `appearance`：theme、density、fontScale、monoFontSize；
  - `agent`：backend、permissions、`backends.<id>` 下的 model 与 effort；
  - `overlay`：enabled、extraApps；
  - `launcher`：内置 id 或 `包名[/Activity]` 的有序数组，内置应用不可移除；
  - `terminal`：extraKeysPinned。

## 6. 持久化格式（`.workspace/.workflow/state/`）

| 文件 | 内容 | 格式 |
| --- | --- | --- |
| `sessions.json` (+`.bak`) | 全部 Session（含已归档）、`activeSessionId`、`pinned` | `format: 1` |
| `drafts/<sha256(path)[0:40]>.json` | 单个文件草稿（路径、文本、基准、编辑时间、修订号） | `format: 1` |
| `composers/<sha256(id)[0:40]>.json` | 单个对话的输入与附件（含空墓碑） | `format: 1` |
| `conversations.json` | 对话索引（由 agent 层编解码，WorkspaceStore 统一写盘） | `format: 1` |
| `config.last-good.json` | 最后一次有效的 config.json 原文 | — |
| `corrupt/` | 被隔离的损坏文件原样字节 | — |

**写入**

- 先写入 `.workflow/tmp` 中的临时文件，fsync 后原子重命名，再同步目录；Android 上用 `Os.fsync`。
- 用户目录中不出现临时文件或 `.bak`。启动时只清理 10 分钟以前的临时文件。
- `sessions.json` 在替换前把上一份已知有效的内容写入 `.bak`。

**版本**

- 读到更高的 `format` 时不改写该文件：会话以只读方式保持原样，并给出 `NEWER_FORMAT` 通知。
- 未知的 Panel 类型会被丢弃，其余内容照常读取。
- 数据格式与应用版本号独立；不隐式转换其他格式。

**损坏恢复**

- `sessions.json` 损坏：改用 `.bak`。两者都不可读时从空会话开始，损坏字节移入 `corrupt/`，并给出 `RECOVERED` 通知。
- 单个草稿或对话输入文件损坏：该文件移入 `corrupt/`，其余照常读取。
- Session 的布局损坏：重置为空 Workbench，Session 本身保留。

## 7. 格式边界

1.0.0 不包含旧版数据迁移。只读取当前 `.workflow/state/` 格式，不扫描、导入、搬移或删除 `.state`、旧对话、旧草稿和旧备份。`config.json` 必须明确指定 `version: 2`；旧格式与无版本文件显示配置错误，不自动升级或另存副本。

Explorer 的 `showHidden` 随 Session 布局保存；切换或重启后保持。编辑器标签拖入对话或终端时传递文件，拖到 Tab 行时移动面板；拖放源的无障碍操作菜单提供对应动作。
