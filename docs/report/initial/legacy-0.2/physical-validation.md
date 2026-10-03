# 0.2 真机验收进度

设备：IFLYTEK CB-C6-STU，Android 9 / API28，ARM64，kernel 4.14.98。保留设备原有 density override 261、旋转、Gboard、字体和锁屏凭据。scrcpy 不录音、不录屏、不自动同步剪贴板。

## 已验证

- 真机应用 UID 10087、SELinux `untrusted_app` 下的 9 项后台测试通过：真正 PTY、相同工作区双向文件可见、resize、Ctrl-C、跨块 UTF-8、退出与回收；原生 Codex 和 UI 使用的进程通道可初始化、读取账户状态、停止并重新启动；网络桥鉴权/二进制转发/半关闭/清理/系统 CA；Mihomo 可执行版本与隔离配置检查、代理进程管理组件拒绝非 root 运行。
- 2 项协议回放通过：后台回复不会串到当前 Session；未知服务器请求保留 ID/参数并且不会自动批准，resolved 通知能清除对应待处理项。
- 独立外部 TLS 探针通过系统证书与主机名验证，从 `auth.openai.com` 收到 HTTP 403。没有携带账号认证，也没有请求模型；这只能证明网络通道和 TLS 校验，不是登录成功证明。
- 最终 0.2.1：68 项 JVM 测试（核心 37、代理 13、UTF-8 6、终端输出窗口 8、增量渲染 3、模板 1）和 15 项 Web 测试通过；构建与 lint 无错误。原生 PTY 宿主测试 7 组，代理进程管理组件宿主测试 13 项通过。
- 最终安装包 9 项真机回归通过：离线 Markdown/数学、嵌入式 WebView 高度/缩放/历史保留、快速 IME 重复字母与回车顺序、跨 Session 协议回放、真实 PTY 与 UI→IO 写入顺序。另有 1 项独立 Root 代理 fixture 通过，边界见下文。

本轮实际修复了 Android ICU 在内部缓存 UTF-8 前缀时被错误重建 decoder 导致的 emoji 乱码，保留了能在真机复现的回归测试。也修复了首次编辑前的外部变化、保存途中继续输入和源文件删除时的草稿版本问题。

## 真机交互与润色

用户授权解锁后，已保留原显示密度完成第一轮触屏操作。没有修改锁屏凭据，也没有通过注入激活状态来伪装使用成功。以下均为本轮实体设备结果：

- 文件创建、保存、重名错误、文件附件、横竖屏菜单、多标签、关闭标签后重新打开未保存草稿通过。
- 终端初版首开黑屏且越界。真机 WebView 为 137.0.7151.72，问题来自 Compose 中 WebView 的 `WRAP_CONTENT` 导致页面高度为零；修复原生布局参数、裁剪及 viewport 同步后，安装包首开正常，Gboard 触屏输入 `pwd` 正确返回共享 `.workspace` 路径。
- 侧边 Chat 键盘避让、Files + IME 约 6 行编辑空间、Chat 主模式收起终端并保留进程均已实际复验；主输入正文和发送按钮可触达。
- 第二文件标签的撤销/重做实际回滚通过。不同 Session 输入草稿已隔离：新 Session 为空，切回原 Session 恢复原输入。草稿正文与附件均按对话资源持久化，归档保护覆盖未发送内容。
- 快速输入的重复后缀已通过真机 DOM trace 和实际 WebView 重放定位到 xterm 229 fallback 排程。安装包已通过重复字母与立即 Enter 回归；详见 [输入回归记录](android-input-validation.md)。

Launcher 内手动添加外部应用、启动、Back 返回已通过；最终包中添加应用的搜索框与结果在 Gboard 展开时可见。文件树拖入聊天输入区与空白区均添加真实附件，取消拖放会清除高亮，之后正常触摸输入可用；标签拖放重排通过。文件拖入终端生成正确引用的绝对路径，执行 `cat` 读到编辑器保存的 `hello`。

快速 `echo FAST_FINAL` 无重复；`RETURNN` 经退格变为 `RETURN`；实际逐键 Gboard 拼音 `nihao` 选择“你好”后回车，终端正确输出中文。对话原输入 `session_A_only` 和两个附件在重启后保留。测试误注入到 `qa-second.md` 的尾部字符串已通过 UI 精确删除，保存回测试前内容 `second_draft`。

悬浮窗授权、展开、点击空白收起、拖动贴边已通过。文件抽屉、进一步重启隔离和设备控制仍在最终检查；完成后更新本段。

开发机已按原始需求完成一次无工具 Codex 模型回合，返回中文 Markdown 和数学公式。手机上没有复制桌面认证，真实账户登录及模型回合仍未验收。

首次独立 Root 代理 fixture 在 `root-start` 阶段失败，后续脱敏诊断确认为 `Permission denied`。在 Magisk UI 中确认 Workflow 单独被拒绝，按用户授权仅将该应用改为允许后，最终 fixture 通过：实际 Root 内核启动、鉴权、三模式/代理组、DIRECT/REJECT 本机转发、停止/重启、新进程身份、端口与服务清理全部验证。生产配置未变，临时 fixture 已删除。

前后 `ip link` / `ip rule` / 全部路由完全一致。设备原有 `Meta` TUN 与 table 2022 属于现有网络，不作为 Workflow 的验收证据。测试没有启用 Workflow TUN 或改系统 DNS/证书。

自动化证据：`0.2.1-final-device-regression.log`、`0.2.1-root-proxy-final-result.json`、`0.2.1-web-tests.log`、`0.2.1-build.log`。

证据：`artifacts/physical-review/`。0.2.1 已装到设备，归档 APK、SHA-256 和报告位于 `artifacts/delivery/0.2.1/`。0.2.0 目录保留的是前一修复包，不应替代本次结果。
