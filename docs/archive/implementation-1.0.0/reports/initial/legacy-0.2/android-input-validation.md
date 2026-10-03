# Android 终端输入回归

2026-09-26，IFLYTEK CB-C6-STU / API 28 / WebView 137.0.7151.72。

真机 Gboard 输入连接实际产生 `keydown(229) → beforeinput(insertText) → input → keyup(229)`。预载 xterm 6.0.0 的 `CompositionHelper` 为每次 229 按键保存旧 textarea，再在零延迟定时器中读取新值。多个事件先于定时器执行时，旧快照会重复消费同一个最终值。

在真实 WebView 同步重放该事件序列，`bookkeeper` 被扩大为 `bookkeeperookkeeperokkeeperkkeeperkeepereepereperpererr`。因此问题不只是截图、键盘显示或 PTY 回显。证据见 `artifacts/physical-review/terminal-ime-trace.json` 与 `terminal-burst-before.json`。

`app/src/main/assets/web/android-input.js` 仅调整该 fallback 的排程：同一 helper 保留一个待处理快照，在下次 keydown 或 compositionstart 前处理，最后一个输入由定时器收尾。保留上游字符差异、删除与替换规则，不在 `onData` 按字符内容去重。重复字母、输入后立即回车和中文组合不能因修复而丢失。

兼容层以固定版本的私有 helper 形状与方法指纹确认适用性，失败时保留上游行为并记录错误；升级 xterm 时必须重新检查指纹与测试。未修改上游 bundle 或其许可证；版本和 SHA-256 仍在 `app/src/main/assets/web/vendor/manifest.json`。兼容层通过公开 addon 生命周期清理定时器。

另将 PTY 写锁移到切换 IO dispatcher 之前，保证从 UI 顺序进入的字符先排队，再写入 native PTY；这处理潜在重排，与上述重复后缀问题是两个独立边界。

验证：

- `web/terminal-input.test.mjs` 使用实际预载 Terminal，12 项覆盖旧实现确切反例、连续输入/重复字母、删除、同长度替换、中文组合、emoji、Enter 顺序、重复安装、未知版本与释放清理；连同原 Markdown 测试共 15 项通过。
- 真机热加载兼容层后，同样事件序列准确输出 `bookkeeper`，记录为 `terminal-burst-after-hotfix.json`。热加载不单独算安装包验收。
- APK 通过 `OfflineRendererTest.androidImeBurstPreservesRepeatedLettersAndEnterOrdering` 检查真实 WebView 的整个事件路径，通过 `LocalRuntimeInstrumentedTest.rapidUiInputKeepsItsOrderAcrossTheIoDispatcher` 检查实际 PTY 写入顺序。最终结果以 `0.2.1-final-device-regression.log` 为准。

调试事件只记录合成测试输入，监听器和临时 adb 转发随后移除，不在产品中记录用户键入内容。
