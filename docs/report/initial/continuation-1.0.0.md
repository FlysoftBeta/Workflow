# Claude Code 接续：1.0.0 基线

本轮只读取 Claude Code 的工作记录和仓库；未读取旧 Codex transcript。最新用户约束：移除所有 Legacy 迁移、版本 1.0.0、Codex/Claude/终端强依赖容器。后续 Rust 统一工作区架构与 UI 改造另立阶段，不能混淆验收。

## 接续修复

- 旧状态迁移器、fixture仓库、自动备份搬移、旧config与container解析分支删除。旧文件不被扫描或转换；无版本app配置拒绝写回。
- 生产 native Codex launcher、Android网络桥、debug demo入口和Android shell fallback删除；PTY隔离测试用的shell adapter移到androidTest。
- 对话索引写入回归WorkspaceStore唯一actor，统一防抖、flush、错误可见性和重试；并发重命名/归档/模型设置不会覆盖彼此，新版格式不改写。
- 对话菜单补充压缩、用量/后端状态、显式高级控制台（Claude定位当前会话）、确认删除。删除移除所有Session引用和本对话草稿，不影响其他草稿。
- Codex queue变化读取完整分页，更新排序/内容并保留在途提交；用真实submission id取消。修复无乐观消息时TurnStarted丢失client id的问题。
- 文件标签拖入对话/终端变为附件/转义路径；标签行仍执行面板移动。隐藏文件偏好持久化，并修复规范化和重命名时丢失该字段。
- 拖放源补充无障碍动作菜单。Activity恢复时明确的destination优先于恢复页面，消费后不重复打开。
- 无口令本地PKCS12（0600、ignored）与V1/V2/V3 release构建/校验脚本；版本1.0.0/code10000。

## 验收记录

- 单元：core 212、agent 48、proxy 71、app 56，全部通过；lint无错误。证据 `artifacts/completion/initial-final-checks.log` 和 `unit-summary.json`。
- Web：36/36；image格式27/27；native PTY/guardian通过。engine完整host 68/68（详W10）。
- 综合设备回归揭露一个已废弃且未被生产引用的 AssetRenderer/TerminalContent 还按旧终端 API 调用 append。已归档并删除孤立适配器，将同样的增量、真实WebView IME和Compose内嵌resize测试转到实际 TerminalWebView；没有通过跳过旧失败用例代替生产验证。
- API28：完整环境2/2、base首次全流程1/1（427.073秒）、设置/overlay13/13且零跳过、综合回归22/22（40.024秒）。综合回归包括实际 TerminalWebView 的IME/增量/内嵌resize；外部settings intent也由uiautomator断言与截图通过。代理最终7/7与真实App强制停止后外部规则保留通过，见W9。
- 两ABI Release的V1/V2/V3、versionName/code、唯一ABI、镜像摘要和离线资产核验通过。x86_64 Release在全新安装后可从外部intent打开设置和Workbench，非debuggable、无Fatal；证据 artifacts/completion/release-smoke/。最终guardian安全修复已重建并核验所有签名/资产。

## 主机资源事故与修正

三个read-only AVD的临时qcow写入/tmp tmpfs，叠加Gradle/Kotlin daemon耗尽内存和swap，导致应用重启与一次构建失败。已终止本轮遗留daemon、删除确认无进程持有的临时qcow；Gradle worker=1、Kotlin in-process；AVD单实例锁、1536MiB/2核，设置ANDROID_TMP到磁盘。TMPDIR单独设置无效，已通过实际/proc/fd定位验证。失败/中断运行不计为通过。

## 后续用户任务

先保留本基线，再完成统一Rust Engine、workspace root/.workspace配置、定制预置镜像、多工具链env.json、连接配置启动页/远程接口抽象（不实现SSH连接）、聊天插件解耦、UI/AVD体验完善与文件归档。Environment指完整运行环境，不是镜像与配置文件的机械组合。

后续补充：Chat transcript 改用 Compose 原生，保留流式、公式、Markdown、表格和代码块；xterm终端保留既有WebView适配器。
