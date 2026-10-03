# 代理

Rust Workspace Engine 拥有代理配置和已发布状态；`ProxyService` 是当前连接的本机执行器，
经 `AppGraph.proxy` 提供 `:proxy` 的 `ProxyApi`。代理使用单独的 Root Mihomo 进程，
不在开发容器里运行。打开面板、读配置、
校验配置和查询状态都不会申请 Root；只有明确的启动或确认停止操作可以使用 Root。

## 配置与界面

配置只在工作区根的 `.workspace/services/proxy/config.yaml`。导入、首次创建与读取走
`documents`（namespace=`services.proxy`、key=`config.yaml`），编辑器通过 Engine 打开同一文件；
版本冲突明确失败。Android 仅把 Engine 返回的字节暂存到 `cache/proxy-executors/<connection>/`，
Mihomo 的工作目录也在此缓存中。配置读取失败时不能用缓存启动。File provider 的相对路径在同一工作区代理目录下解析，
校验、启动和显式 provider 更新前由 Engine 有界读取并暂存；缺失或读取失败时不使用旧缓存。
用户直接编辑 YAML；应用不做配置覆写、订阅账户管理或旧版本配置迁移。首次“编辑配置”
创建一次安全模板：TUN 关闭、仅本机控制接口、随机 secret、独立端口和 TUN 路由范围。
导入是明确确认后的原子替换；运行中的内核在重启后读取新配置。

代理面板提供启停、规则/全局/直连、分组节点选择、组/节点测速和实时上下行速率。
停止后从配置显示禁用的模式和节点；同一配置最后一次实际选择可保留供查看。
日志与连接在当前 Stack 打开独立 Panel，关闭面板不结束内核。日志按级别着色、可选择复制；
连接显示目的地、路由链、流量和存续时间，每秒更新。连接列表只读。
“更新订阅与规则”调用配置已有的文件/HTTP providers，报告每项失败并重新读取节点。

## 生命周期与所有权

启动先用无特权 `mihomo -t` 校验配置，再检查本机端口、其他 VPN/TUN 和配置变化。
没有真实 `uid 0` 结果时不显示 Root 已授权；可执行文件存在也不代表内核或 TUN 已运行。
运行状态分别记录控制接口可达性、TUN 网卡出现与路由规则检查结果。执行器先以
`services.report` 提交实测状态，核对 Engine 回执后才发布 UI 投影；报告不携带配置密钥、
订阅地址或测速 URL。连接失败显示未确认，不能显示为已停止或继续启动。断开连接仅关闭
自己的 guardian 管道，不自动探测 Root、不按旧 PID 发信号。

Root guardian 只监督自己 fork 的内核。控制命令携带本次 run UUID、PID 和 `/proc` 启动时间；
内核身份验证、`waitid(WNOWAIT)` 和未回收的组长防止 PID 复用。`.process.json` 是缓存中的临时核对记录，
不授权按旧 PID 杀进程。前台服务 lease 随本次 guardian 存亡保留/释放。

应用死亡时控制管道 EOF 触发 guardian 向自己的进程组发送 SIGTERM，2 秒后才升级 SIGKILL。
明确设备名并开启 `auto-route` 时，guardian 接收本次路由表和规则范围，启动前通过 netlink
确认它们为空。内核退出后仅删除该范围内匹配本次表/设备的完整规则记录；不 flush 表、
不删除其他接口，不修改系统 DNS。Mihomo 正常退出负责其余资源，非持久 TUN 随描述符关闭销毁。
停止前若发现本次范围内出现不同目标或接口的规则，guardian 直接终止自己的内核后做精确清理，
防止 Mihomo 的优雅退出按优先级范围删掉后来出现的规则；不能读取规则时也不允许该范围清理。

guardian 自己被 SIGKILL 时内核由 PDEATHSIG 杀死，但 guardian 无法再执行清理；同范围残留规则
会阻止重启，应用不假定它们属于自己并擅自回收。自动设备名、auto-redirect/nftables 及外部
Root 管理器强杀进程树不具备同等的强制退出清理保证；安全模板使用明确设备名且关闭
`auto-redirect`。真实设备 Root 管理器仍须独立验收。

## 共存与日志

检测到其他 VPN/TUN 时拒绝启动 TUN，不提供“仍然启动”。Root 获准后重新检查冲突；任何
占用目标表或规则范围的既有规则均阻止启动，读取失败也阻止启动。建议采用模板里的
`workflow-tun`、表/优先级 `9500`（含随后 10 个优先级）；绝不自动更改其他代理。
`include-package`/`exclude-package`、UID 范围和 DNS 策略均由 Mihomo 的 YAML 配置控制。
无 carrier 的点对点 TUN 仍视为已有所有者，不因 Android 暂时报告 `isUp=false` 而放行。
Root 获准后通过 `ip -o link show` 重新检查，包括 Java 网卡枚举可能遗漏的无地址网卡；
命令失败、输出为空或不可解析时拒绝启动 TUN。

本机缓存 `runtime.log` 超过 1 MiB 后轮换为 `runtime.previous.log`。日志面板通过 Engine
`services.proxy/runtime.log` 提交有界、已脱敏的日志副本，编辑器打开同一服务文件。
这些缓存可丢弃，不作为工作区状态的后备来源。写入、校验诊断、实时日志和
provider 错误先隐藏控制 secret、节点认证字段与配置订阅 URL（包括 URL 编码和跨读取块的值）。
配置本身含敏感值，不进入日志、备份或测试证据。应用不会记录完整 controller 请求/响应。

## 验证

固定内核和源归档信息见 `third_party/mihomo/manifest.json`。JVM 测试运行 `:proxy:test`；
guardian 宿主测试运行 `native/test-host.sh proxy-guard`。Android 9 模拟器的
`ProxyExecutableTest`、`ProxyPanelEmulatorTest` 和 `ProxyEmulatorRootTest` 覆盖真实打包内核、
面板、Root TUN、provider 更新与清理。后两者要求显式 instrumentation 参数和
ranchu/goldfish + `adb root` 三重门控。另有 host 观察实际 app force-stop、进程退出及外部规则
保留的场景。验收证据和未覆盖矩阵在 `docs/report/initial/w9-proxy.md`。
