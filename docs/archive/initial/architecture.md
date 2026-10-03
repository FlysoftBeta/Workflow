# 架构

用户可见行为以 [product.md](product.md) 为准；本文件规定模块边界、数据归属与进程模型。各子系统细节：[ui.md](ui.md)、[agents.md](agents.md)、[engine.md](engine.md)、[proxy.md](proxy.md)；验证方法见 [testing.md](testing.md)，当前完成度见 [status.md](status.md)。

## 1. 模块

```
:core    纯 Kotlin/JVM   工作区、Session、布局（Paradigm/Stack/Panel）、Working Resource、配置、container.json、归档策略、终端流工具
:agent   纯 Kotlin/JVM   对话后端统一接口；Codex App-Server 与 Claude Code 适配器；协议 transport；覆盖清单
:proxy   纯 Kotlin/JVM   Mihomo 配置解析、控制接口客户端、guardian 协议
:app     Android         AppGraph、导航、feature 界面、platform 实现（PTY、进程、root、服务、能力探测）
native/  CMake           pty、proxy-guard、engine（loader + tracer）；各自带宿主测试
web/     npm             Markdown/数学渲染器、xterm 终端页；构建到 app assets
image/                   工作区镜像定义、打包与参考安装器
third_party/             预构建二进制的 manifest、许可与校验和；二进制本体由构建任务按摘要获取，不进入源码
```

依赖方向（由模块图强制）：

1. `:core` 不依赖任何内部模块，不引用 Android、WebView、OkHttp。
2. `:agent`、`:proxy` 只依赖 `:core`，彼此独立。平台能力通过端口进入：`ProcessLauncher`、`RootShell`、`TerminalBackend`、`GuardianLauncher`。
3. `:app/platform` 实现这些端口；`:app/feature.*` 之间不互相引用，跨功能动作（“附加到对话”“在编辑器打开路径”）经由 store 命令或导航契约。
4. 长生命周期对象（`WorkspaceStore`、`RuntimeHost`、`AgentHub`、`ProxyService`）在进程级 `AppGraph` 中；ViewModel 不持有持久状态。

`:app` 包结构：`app`（Application、AppGraph、MainActivity、导航）、`feature.{launcher, sessions, workbench, files, editor, terminal, chat, proxy, settings, overlay}`、`platform.{pty, process, engine, root, network, service, capabilities, filewatch}`、`ui.{design, web}`。

## 2. 数据布局

所有持久数据位于同一个 `.workspace`（Android 上为 `filesDir/.workspace`），不存在多项目或第二个默认工作区。

```
.workspace/                 文件浏览器根；环境内挂载为 /workspace
  config.json               应用配置（用户可直接编辑）
  container.json            声明式环境（用户可直接编辑）
  …                         用户文件
  .workflow/                应用内部数据；浏览器默认隐藏；对环境内进程不可见
    state/                  sessions、layout、working resources、composers、launcher、对话索引
    engine/                 镜像、rootfs 各代、元数据库、home/work、日志
    proxy/                  config.yaml、providers、运行状态与日志
    trash/                  删除的文件，7 天后清理
    logs/
```

- `.workflow/` 在环境内被屏蔽，代理后端执行的命令无法读写 Session、草稿与密钥。代理后端自身的登录状态存放在环境的 home 中。
- 代理配置等内部文件只能通过所属应用的显式动作（如代理页“编辑配置”）在编辑器中打开；把 `.workflow/` 内文件作为附件发送前需要确认。
- 应用数据不参与系统备份与设备迁移。

## 3. 状态与持久化

模型、不变量、`LayoutOp` 语义、存储格式见 [workspace.md](workspace.md)。

- `WorkspaceStore` 是 `.workflow/state` 的唯一写入者：串行命令、内存中的权威状态、`StateFlow` 输出、防抖后原子替换写盘。UI 不持有第二份快照，主线程不做文件 IO。
- Session 状态（Paradigm、Stacks、布局树、焦点、阅读位置）随每次修改自动持久化；Working Resource 独立存放，按资源 ID 共享。
- 布局修改是纯函数 `LayoutOp`（打开、关闭、移动到 Stack 中心或某边、排序、调整比例、切换 Paradigm、提升对话），在 `:core` 中完整测试；拖放在松手时提交一个 `LayoutOp`。
- 声明式文件（config.json、container.json）解析失败时保留最后一次有效版本，并在对应位置提示错误。
- 文件写入使用可恢复的原子替换；外部修改通过 file watcher 进入 store，按 [product.md](product.md) §4 处理冲突。

## 4. 进程模型

- `RuntimeService`（前台服务）持有所有长期进程：终端、代理后端、环境引擎、代理内核 guardian；UI 被销毁时进程与输出不丢失，重新进入即可重接。
- 终端：JNI PTY 分配终端，子进程为环境内的登录 shell（用户 `work`，工作目录 `/workspace`）。
- 环境：`native/engine` 编译为 `lib*.so`，由 PackageManager 解压到 nativeLibraryDir 后作为可执行文件启动；它加载 guest ELF 的解释器并以 ptrace 转换系统调用。见 [engine.md](engine.md)。
- 代理后端：Codex App-Server 与 Claude Code 在环境内运行，因此其命令与用户终端处于同一 Debian 环境。进程只由容器的 `EngineProcessLauncher` 启动，stdio 承载协议；没有宿主后端、Android shell 回退或演示模式入口。
- 代理内核：Mihomo 由 root guardian 启动与回收，见 [proxy.md](proxy.md)。

## 5. 协议与渲染

- 对话协议帧使用保真 JSON：未知方法、通知和字段原样保留并以通用形式呈现；服务器请求 ID 原样回传；任何审批都必须来自用户操作。
- Markdown 与终端使用离线 WebView 资源（marked、KaTeX、DOMPurify、xterm.js），不访问 CDN；HTML 经过清理，外部链接交给系统，本地路径交给文件服务。聊天内容不能调用桥接 JavaScript。

## 6. 约束

- minSdk 28；Android 9 WebView 为真实兼容目标。
- 依赖使用精确版本，不用动态版本；预发布依赖只用于必需能力（Material 3 Expressive、sora-editor）。
- 能力状态以实际探测为准：开关、已安装的包或存在 `su` 都不等于系统动作已成功。
- 大型生成物放在被忽略的 `artifacts/`、各模块 `build/` 或 `third_party/` 缓存中。
