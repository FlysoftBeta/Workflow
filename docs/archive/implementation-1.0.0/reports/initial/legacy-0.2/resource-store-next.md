# 统一工作区资源存储：下一阶段设计

> 已被 0.2 默认自管架构取代。下文保留为 0.1 的历史方案，不是当前实施方向：编辑器、本地 PTY、本机 Codex 现在共同访问应用拥有的 `.workspace`，无需把默认文件迁移到 Termux/远端。下述 Source ID/乐观版本机制仅在未来显式增加外部资源提供者时适用。

状态：**设计提案，尚未实现**。当前 Android 的文件编辑、Session 和 Working Resource 位于应用私有 `filesDir/.workspace`；Termux daemon 的进程与文件位于 Termux 用户的 `~/.workspace`。当前只有显式添加到对话的附件上传到 daemon。不能把这两处目录描述为已经同步，AI 在 Termux 中修改的文件不会自动更新 Android 本地文件。

## 目标与边界

编辑器、终端和 Codex 对同一资源的访问最终应落在 daemon 的 canonical workspace。Android 私有存储承担启动配对、UI 状态缓存、离线快照与未保存草稿。现有 WorkspaceRepository 的 Session/Working Resource 语义继续保留；不通过定时复制整个 `.workspace` 来模拟一致性。

不做没有版本前置条件的双向同步。尤其禁止把 Android 含配对密钥的 `config.json` 整份发送到 daemon，也不允许首次连接就以空目录覆盖远端。

## 资源身份

为每个 source 分配稳定且随机的 `sourceId`，与运行进程的 instance ID 分离。daemon 重启不改变 sourceId；重装并新建工作区必须生成新 sourceId。重新配对时客户端先比较 sourceId，不用 endpoint 或目录字符串判断“仍是同一工作区”。

资源标识至少包含 `sourceId + resourceId`。`resourceId` 由 source 分配并持久化；相对路径是可变属性，不是唯一主键。rename 后打开面板、草稿与对话附件引用仍指向同一资源。现有按路径资源迁移时创建映射表，保留旧 resource ID 别名，避免丢失 Session 引用。

## 接口草案

```kotlin
interface ResourceStore {
    val sourceId: SourceId
    suspend fun list(directory: ResourceId?): DirectorySnapshot
    suspend fun stat(id: ResourceId): ResourceMetadata
    suspend fun read(id: ResourceId): VersionedBytes
    suspend fun create(parent: ResourceId, name: String, bytes: ByteArray): VersionedResource
    suspend fun write(id: ResourceId, bytes: ByteArray, expectedSha256: String): WriteResult
    suspend fun rename(id: ResourceId, parent: ResourceId, name: String, expectedVersion: String): ResourceMetadata
    suspend fun events(after: ChangeCursor?): ChangeBatch
}
```

`VersionedBytes` 返回真实字节、SHA-256、resourceId、sourceId、path 与内容类型。`WriteResult` 区分成功、内容冲突、资源删除、权限拒绝和连接失败。对不存在文件的创建使用独立 create 语义，避免用 `expectedSha256=null` 意外获得无条件覆盖能力。上传完成后以服务返回的版本为准。

事件包括 create/write/rename/delete 与可恢复 cursor。终端或 Codex 在文件 API 之外修改文件时，服务通过文件监视或打开/保存时重新计算摘要检测变化；不能仅依赖 API 自己发出的事件。cursor 过期必须返回显式断档，客户端重扫 metadata。

## Working Resource 与缓存

未保存草稿始终先保存在 Android 本地，键为稳定 source/resource ID，包含 `baseSha256`、完整草稿或可恢复的编辑日志、修改时间。界面必须区分“草稿已保留在本机”和“已经保存到工作区”。Session 仅引用资源，不持有文本。

- 打开：校验 source ID，读取远端版本到缓存；有本地草稿时保留草稿，比较 baseSha256 后标记是否存在外部变化。
- 编辑：后台串行、原子保存 Working Resource；无需网络，不能因 Session 切换而取消最后一份草稿。
- 保存：提交草稿与 baseSha256。远端成功并返回新 SHA 后才清除 Working Resource、更新缓存版本。
- 冲突：保留本地草稿和远端快照，提供对比、另存、采用远端、明确覆盖四种选择。明确覆盖仍应携带用户看到的远端版本，避免确认后再次被外部更新。
- 删除：不能把远端消失等同空文件；草稿继续存在，用户可另存。

缓存不是 source of truth。不得使用最后同步时间替代内容版本，也不得以本地缓存删除推断用户希望删除远端。

## 配置分离

Android bootstrap 配置保存 endpoint、认证凭据、最近 sourceId、必要的系统授权引导状态。认证凭据不进入 canonical workspace、不进云备份、不作为普通文件暴露给 Codex。将它与可编辑的应用/工作区配置拆分，是启用统一 ResourceStore 的前置迁移。

canonical workspace 的 `config.json` 保存可共享的应用行为；`container.json` 保存 declarative runtime 环境。修改 container.json 后 daemon 校验与 reconcile，并独立上报“配置已保存 / 正在应用 / 应用失败 / 当前生效版本”，文件成功保存不等于环境已应用。

Session manifest、对话资源与 Working Resource 的服务端/本地放置策略需要 schema 明确：至少必须有一个权威写入方，并允许客户端持有可恢复缓存；不允许两个旧版本 manifest 在重连时互相覆盖。

## 断连、重连与迁移

断连后保留已打开内容与编辑能力；远程列表和未缓存文件标记不可用，不伪装空目录。保存操作可保留为待提交意图，但不自动无条件重试写入。重连先检查 sourceId 和最新版本，再逐个恢复；出现新的 sourceId 时提示重新关联，不迁移草稿到同名路径。

迁移采用显式事务：备份旧 manifest → 建立 source/resource 映射 → 为已打开文件比较本地/远端 SHA → 持久化新 schema → 保留回滚标记。相同路径但不同内容视作冲突，不选择更新较新的文件自动覆盖。迁移未完成时原本地工作区仍可读取。

## 附件与路径

对话附件使用 source/resource ID、选取时的版本及展示名。若 Codex 与 resource source 相同，发送经过验证的 canonical 工作区路径；跨 source 的附件先复制到 daemon 的受控 `attachments/` 路径，返回 server-assigned path 和 SHA，再构造 turn 输入。

图片应按 App Server 当前 schema 发送 `localImage` 等相应输入项，不能仅通过文本文件名假装附上了图像。拖入终端的路径必须相对当前 terminal source 转换并进行 shell quoting；本机 Android 私有路径不能直接交给 Termux。

## 必须验证的情境

在实施阶段补充针对语义的集成测试：离线编辑后重连、Codex 并发修改导致保存冲突、保存响应丢失后重试、rename 期间保留面板/草稿、source ID 改变、服务重启 cursor 断档、草稿存在时远端删除、配置迁移中断恢复、图片附件上传失败，以及 Android 进程被杀后的 Working Resource 恢复。完整通过后才将“文件/终端/对话共享一个工作区”标记为已完成。
