---
type: gotcha
title: Mesen2 Windows 采集三坑与 headless 配方——io 沙箱/CWD 偷换/弹窗扰民，--testRunner 全解
date: 2026-10-14
status: confirmed
---

# gotcha：Mesen2 Windows 自动化采集的三个坑与 `--testRunner` headless 配方

## 现象

用 GUI 方式在 Windows 驱动 Mesen2 2.1.1 做自动化采集：模拟器窗口弹到前台扰民；
Lua 脚本看似"没执行"（无日志、无产物）；产物相对路径全部落空。逐项排查后发现三个独立坑，
而正解是官方 help 里一行没展开的 **`--testRunner`**（用户点破"应该有 headless 参数"）。

## 为什么

1. **io 沙箱默认锁死**：`ScriptWindowConfig.AllowIoOsAccess` 默认 false，`io.open` 不可写；
   采集 Lua 全部产物靠 io ⇒ `assert(io.open(...))` 静默炸掉、脚本死在注册回调之前。
   解锁：`--debug.scriptWindow.allowIoOsAccess=true`（`ConfigManager.ProcessSwitch` 用
   IgnoreCase 反射按 `a.b.c=value` 直设任意配置项，不必改 settings.json）。
2. **CWD 被偷换**：`Program.Main` 里 `Environment.CurrentDirectory = ConfigManager.HomeFolder`
   （`Documents\Mesen2`）——脚本里的相对路径产物全部落错地方且 `io.open` 失败被 `if f then` 吞掉。
   产物路径必须绝对（本仓 PLAN.out_dir/shots_dir 已是绝对路径，勿改相对）。
3. **GUI 弹窗 + 假死残留**：GUI 模式窗口会抢焦点；驱动误判进程退出后 Mesen 还活着（孤儿），
   与后续 run 争产物。headless 下无窗口、`emu.stop` 后进程自退（退出码=stop code），无孤儿。

`--testRunner <lua> <rom> --timeout=N` 的权威语义（Mesen2 2.1.1 `UI/Utilities/TestRunner.cs`）：
UI 初始化前分叉、`ConsoleMode` 无窗口、`DisableSaveSettings` 自动、Lua 自动执行
（`DebugApi.LoadScript`，GUI 模式的 AutoStartScriptOnLoad=true 也自动执行——"没执行"是 io 坑的假象）、
`MaximumSpeed` 全速、超时或脚本停机后退出。**跨模式/跨平台字节确定性已实证**：
Windows testRunner 重采集的 boot-title 300 帧 trace 与 Linux Xvfb GUI 记录的 golden
sha256 逐字节一致（`4655a986…`）。

## 何时使用

- Windows 采集一律 headless：`Mesen.exe --testRunner <lua绝对路径> <rom绝对路径>
  --timeout=N --enableStdout --debug.scriptWindow.allowIoOsAccess=true`
  （本仓 `tools/scenario_run.py` IS_WINDOWS 分支即此配方）。
- 任何"脚本没执行"的判断先做**二分探针**：纯 `emu.stop()` 秒退=执行了（否则跑满 timeout）；
  再单独测 io（绝对路径落盘）。禁凭"没看到输出"下结论——沙箱/路径/CWD 三坑都会伪装成"没执行"。
- 查权威语义回 2.1.1 tag 源码（`UI/Utilities/CommandLineHelper.cs`、`TestRunner.cs`、
  `UI/Config/ConfigManager.cs`），勿猜开关名。
- Unix 侧沿用 Xvfb+GUI 已验证路径不变；两侧 trace 字节面由本条目的确定性证据钉住。

## 证据指针

- 二分探针实录（2026-10-14）：probe3 纯 `emu.stop()` ⇒ `exit=0 wall=0.6s`（脚本执行✓）；
  probe2 加 `--debug.scriptWindow.allowIoOsAccess=true` + 绝对路径 ⇒ `io_out=True`（io 解锁✓）；
  同探针相对路径 ⇒ `io_out=False`（CWD 坑复现✓）。
- headless 首战：`python tools/scenario_run.py run boot-title --timeout 300` ⇒
  300 帧/7 captures/2.0s，`sha256=4655a986…` 与 `oracle/traces/l3_manifest.json` 锚点逐字节一致；
  `gates/cli.py trace` PASS（L3 裁定 1 用例 0 失败）。
- 源码锚：[SourMesen/Mesen2 `2.1.1` tag](https://github.com/SourMesen/Mesen2/tree/2.1.1)
  `UI/Utilities/TestRunner.cs`（Run 语义）、`UI/Utilities/CommandLineHelper.cs`
  （`--testRunner`/`--timeout`/`--enableStdout`/`ProcessSwitch`）、`UI/Config/ConfigManager.cs`
  （`settings.json` 路径与 HomeFolder 切换）。
