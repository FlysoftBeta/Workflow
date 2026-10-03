# W9 代理交付与验收

2026-10-03；接续 Claude Code 中断的 W9b。实现包含 backend、三个正式面板及安全生命周期，
下列 JVM、native 与 Android 9 模拟器验收均已通过。当前内核固定为 Mihomo 1.19.31；未更换供应链版本。

## 完成内容

- `:proxy`：受限 YAML 读取、安全模板、配置节点预览、typed controller、模式/节点读回确认、
  节点与组测速、traffic/log 流、只读连接、HTTP/File provider 更新与健康检查。
- `ProxyService`：唯一进程级所有者、真实 Root 检查、前台 lease、内核/controller/TUN 分别验证。
  Root 代理保持独立系统功能；没有旧配置/状态迁移，配置仅使用当前工作区目录。
- 正式代理面板：48dp 状态条、规则/全局/直连、可折叠自适应组、节点选择/测速/延迟色阶、
  provider 更新、导入与直接编辑配置；工具行只留测速，启停在状态条。
- 日志/连接在同 Stack 中打开独立面板。日志跟随、选择/复制、打开编辑器；连接每秒刷新，
  显示流量、路由链、目标和时长。读取失败与无连接分别显示，不用空白屏代替错误。

## 接续时修复的问题

1. 原 backend/UI 允许在发现其他 VPN/TUN 后选择“仍然启动”。现在所有调用都拒绝覆盖；
   旧 `overrideConflict` 参数及兼容调用已删除。Root 获准后再检查一次，规则读取失败关闭启动路径。
2. 原规则检测把相同范围/表的规则当作“自己残留”而放行。现在既有规则总是冲突。
   guardian 对明确设备名 + auto-route 做独立 netlink 预检查，既有规则或目标表已有路由均阻止启动。
3. 原 guardian 依赖 Mihomo 的 SIGTERM 清理；内核直接崩溃/SIGKILL 可遗留策略规则。
   现在 reap 自己的子进程后只删除目标范围内匹配自己表/设备的完整规则消息，保留不同目标/接口。
   app EOF、正常停止、内核崩溃均走此路径；失败会在状态中明确报告。
4. 原 provider 更新忽略 healthcheck HTTP 失败；现在失败进入对应 provider 结果，UI 防止重复提交。
5. 原日志只隐藏 controller secret；现在还隐藏配置中的节点认证、私钥、订阅 URL/query，
   包括 percent encoding 与跨读块的秘密。循环 YAML 集合不会使收集器无限递归。
6. W9b 原 UI 测试直接调 Activity Back dispatcher 关闭 Popup，实际会返回 Launcher，
   导致连接面板测试超时。现在通过真实“连接”菜单完成导航。
7. API 28 AVD 只有 `/dev/tun`，其 `ip tuntap` 期待 `/dev/net/tun`。
   测试创建并还原这一临时别名，同时检查创建假 TUN 的退出码，避免无效的冲突验收。
8. 最终 UI 验收发现 API 28 的 Java 网卡枚举会漏掉无地址、无 carrier 的已有 `Meta`。
   现在保留点对点网卡所有权，并在 Root 获准后通过 `ip -o link show` 重新检查；
   读取失败、空输出或格式未知均拒绝启动。增加网卡解析、遗漏网卡与读取失败回归测试。
9. 真实 app force-stop 发现 Mihomo 优雅退出会删除同优先级范围内后来加入的外部规则。
   guardian 停止前重新检查；发现不同目标/接口或读取失败时仅强制结束自己的内核，
   再精确删除其规则。普通停止和 app 死亡均验证范围内、范围外外部规则保留。

## 验证命令与结果

| 检查 | 证据与结果 |
| --- | --- |
| `flock artifacts/.gradle.lock ./gradlew :proxy:test` | 71/71 通过，0 skipped；controller/config/runtime/redaction/conflict suites |
| `native/test-host.sh proxy-guard` | 13 个真实子进程测试 + identity/rule parser 断言通过；`artifacts/proxy-final/native-host.log` |
| `native/proxy-guard/build.sh` | Android arm64-v8a 与 x86_64 编译通过，16 KiB 最大页对齐 |
| `:app:assembleEmulatorDebug :app:assembleEmulatorDebugAndroidTest` | 构建通过；`artifacts/proxy-final/final-build.log` |
| `ProxyExecutableTest` | 最终 2/2 通过：实际打包 kernel/guardian、模板 TUN on/off 校验、refresh 不使用 Root |
| `ProxyPanelEmulatorTest` | 最终 4/4 通过：启停、模式、节点、测速、traffic、连接/日志、Root 拒绝、空状态、冲突与端口错误、深色 Solo；16 张截图已检查 |
| `ProxyEmulatorRootTest` | 最终 1/1 通过：真实 Root TUN、TEST-NET 捕获、controller/provider、普通停止、EOF、child SIGKILL、再启动、占用表及外部 TUN 拒绝；普通停止和 child SIGKILL 保留同范围外部规则 |
| 实际 Android app `am force-stop` | 独立 host 观察通过：app、guardian、Mihomo 均退出，TUN 与自己的 IPv4/IPv6 规则消失；`9505→9700` 和 `9700→9700` 两条外部哨兵规则均保留 |

最终合并 instrumentation 为 **7/7**（49.805 秒，0 skipped），见
`artifacts/proxy-final/instrumentation.log`；Root 分步结果见 `root-result.json`，截图见
`artifacts/proxy-final/screenshots/`。实际 app 死亡场景不计入 JUnit 通过数：instrumentation
被外部终止是预期结果，断言由 host 执行，见 `app-death-result.txt` 及对应 rules/ready 记录。

可复核的完整验收脚本保存在 `artifacts/proxy-final/`：

```sh
WORKFLOW_EMULATOR_PORT=5840 tools/with-emulator.sh bash artifacts/proxy-final/final-accept.sh
```

全部运行经 `tools/with-emulator.sh` 与共享 `artifacts/.device.lock`，使用独占 AVD
`workflow-tablet-api28`、端口 5840、read-only、1536 MiB 内存；临时映像写到磁盘目录
`artifacts/emulator-tmp/run.*`，退出时关闭模拟器并删除 overlay。安装使用带 180 秒超时的
`adb install --no-streaming`，避免大 APK streaming 安装停滞。最终模拟器已停止。
不读取/复制宿主账户凭据，不接触用户日常平板或宿主路由/DNS。测试网络全部为本机 fixture，
TUN 捕获使用 TEST-NET 地址与本地 HTTP responder。

## 已知边界

- guardian 自身被 SIGKILL 后无法运行清理，PDEATHSIG 只能保证内核结束。残留规则会阻止再启动；
  不根据旧 PID 或熟悉的规则外观强杀/回收。此边界没有包装成成功恢复。
- guardian 的强制清理覆盖明确 `tun.device` 的 auto-route 策略规则；自动设备名、
  auto-redirect/nftables、Root 管理器强杀整棵进程树仍需独立设备验收。安全模板不启用 auto-redirect。
- 未在用户平板执行 Root/TUN，也没有测试 Magisk/KernelSU 授权界面。模拟器使用受门控的 root shell
  注入，guardian/kernel/协议/controller 和 UI 为真实生产实现。
