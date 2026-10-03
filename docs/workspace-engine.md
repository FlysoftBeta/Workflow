# Rust Workspace Engine

Engine 是工作区唯一写入者；Android 通过 `docs/protocol.md` 的双向 JSONL 连接持有可丢弃投影。
本机进程由 `workflow-engine serve --root <用户文件根>` 启动，APK 中的可执行文件名为
`libworkflow-engine.so`。运行时、loader 和定制镜像通过 CLI 显式传入；没有 host exec 或旧仓库回退。

## 所有权与恢复

工作区配置为 `.workspace/config.json` 和 `.workspace/env.json`，会话/布局、文件草稿、输入框草稿
位于 `.workspace/state/workspace.json`。Engine 持有独占进程锁，串行提交状态、fsync 临时文件与父目录、
原子 rename 后才确认新 revision。最新有效快照保留一份备份；损坏内容移入 `.workspace/corrupt`。
未知 state/draft/composer 格式保留只读，不迁移。环境记录损坏不阻止会话与文件访问。

布局事务在 Server 计算，沿用当前 StateCodec 线格式与 LayoutReducer 不变量。
关闭面板不删除 Working Resources；自动归档仅保护引用同一脏资源的最近 live session。
手动归档需要 save_all/keep_drafts/discard，保存前检查全部文件版本。输入框 acknowledgment 只有
owner/revision/正文/附件全部匹配时才清空，空 tombstone 保持 revision 单调。

用户文件路径拒绝绝对路径、父级跳转和 symlink 穿越；内部状态不可通过文件 API 访问。
显式配置文件和 `.workspace/services/<serviceId>/<key>` 允许编辑。移动文件同步重定位所有会话、草稿、
附件；目标已有未保存草稿时拒绝移动。原子保存保留已有文件权限。上传暂存后用不覆盖目标的原子链接提交。

## 环境与进程

环境采用经过校验的定制镜像，每次配置构建生成独立 generation；既有可用 generation、home 和
side-by-side 工具链/cache 均保留。缺省 Python/Node 使用镜像 defaults，空数组禁用，首项为默认。
Server 调用镜像内 envctl 安装各版本、verify-many 验证全部语言和包，成功后才执行有序 post_scripts。
脚本失败不激活新环境。home 暂存为 0700，成功激活时检查并合并脚本变更，失败/成功均清理暂存。
运行中的旧环境不被后台构建替换；显式 restart 处理已经验证的 pending generation。

构建、等待进程、watch 与流读写独立于状态提交。终端使用 PTY，其他进程使用双向 stdio；
所有命令均通过指定 Rust runtime 和 generation root 运行。调用方环境变量由 guest `/usr/bin/env`
应用，不能注入 host runtime 的动态加载器。stdout/stderr 按字节传递，不解析或回答 vendor 审批。
Engine 终止、显式 stop/restart 均清理拥有的进程组；父死亡信号由持续存活的 reaper 线程拥有。

## 边界与容量

单帧 32MiB；blob 原始块 64KiB；可编辑文本/服务文档 16MiB；权威状态文档最多 24MiB。
大于单帧的组合响应明确返回 too_large，二进制内容使用 files.read/upload。
每个进程的两个输出流分别保留 4MiB，旧 offset 返回实际 startOffset；退出前所有 vendor 字节保持原样。
最多 128 个待处理长请求、128 个活动进程、16 个未完成上传；上传上限 8GiB。

后端索引使用 opaque documents；本机服务报告和期望配置由 Engine 持有，Android 只执行实测能力操作。
`services.<id>` 文本文档与文件编辑器共享唯一文件，revision sidecar 只记录内容 hash。
本机 DNS 上报只生成 guest 的 resolver 绑定，不改变主机设置。远程 bootstrap/transport 仅为客户端抽象接口。

验证命令和本轮实际证据见 `docs/report/rewrite/rust-workspace.md`；host 测试不代表 Android 设备验收。
