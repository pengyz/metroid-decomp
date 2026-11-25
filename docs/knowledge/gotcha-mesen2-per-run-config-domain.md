---
type: gotcha
title: Mesen2 GUI 模式的两个「共享域」——配置域锁与 TMPDIR 管道，任一共享都会静默劫持
date: 2026-11-24
status: confirmed
---

## 现象

同一台机器上并发起两个 Mesen2（GUI 模式，自管 Xvfb）时，后启的实例**不报错也不出产物**：

- 表现 A：进程 rc=0，`analysis/traces/<case>.mesen.log` 只到 `scenario … started`，没有
  `DONE`、没有 trace、没有截图；
- 表现 B：`mesen.log` 里 `Error: System.InvalidOperationException: Pipe hasn't been connected yet.
  at Mesen.Utilities.SingleInstance.PassArgumentsToFirstInstance(String[] args)`，随后退出。

两种表现都会让上层驱动（`tools/scenario_run.py`）把它误读成「模拟器跑完但没写 trace」。

## 为什么

Mesen2 的单实例判定与命名管道各有自己的域，二者**不由 HOME 决定**：

1. **单实例锁按配置域落盘**：`$XDG_CONFIG_HOME/Mesen2/mesen.lock`。只隔离 HOME、让
   `XDG_CONFIG_HOME` 继承宿主 `~/.config` 时，两个实例仍然抢同一把锁 —— 后启者把参数
   `sendto` 给先启者后 `exit_group(0)`（strace 实测：`connect(CoreFxPipe)+sendto(rom路径)+exit_group(0)`）。
   本仓库旧实现 `mesen_isolated_home()` 只把 `HOME` 指向 fake home，正是踩这一条。
2. **命名管道按 TMPDIR 落盘**：`$TMPDIR/CoreFxPipe_<固定 GUID>`（GUID 与实例无关，实测
   `CoreFxPipe_a46696b7-2d1c-4cc5-a52f-43bcaf094aef`）。两个实例共用 `/tmp` 就共用同一根管道。
3. **空配置域会卡在初始向导**：隔离出一个全新的 `XDG_CONFIG_HOME` 但没放 `settings.json` 时，
   Mesen 会起一个 400x515 的 `Mesen - Emulator Configuration / initial setup` 对话框等人点
   CONFIRM —— 0 帧、无日志、进程长活，看起来像「挂起」。必须从模板域拷一份 `settings.json`
   进 per-run 域。

## 何时使用

- 并发跑两个及以上 Mesen2 采集（两卡并行、批量 run-all 提速）；
- 看到「rc=0 但无 DONE / 无 trace」或 `PassArgumentsToFirstInstance` 字样时；
- 给采集驱动加隔离能力、或排查「同一场景单独跑过、并发跑就缺产物」的假 flaky。

正确做法（已落在 `tools/scenario_run.py`）：每次 run 建独立
`analysis/.mesen-runs/<name>-<pid>/{home,xdg-config,xdg-cache,tmp}`，注入
`HOME/XDG_CONFIG_HOME/XDG_CACHE_HOME/XDG_DATA_HOME/TMPDIR/TMP/TEMP`，并从模板域拷
`xdg-config/Mesen2/settings.json`；孤儿清扫只认 `/proc/<pid>/environ` 里带同一
`MESEN_RUN_ID` 的进程（按 worktree 根匹配会误杀同 worktree 的并发实例与队友实例）。

## 附：`--testRunner` 在 Linux 上必崩（同一家族的另一条）

`./tools/mesen2/Mesen --testRunner <lua> <rom>` 在本机 2.1.1 构建上 rc=134、
`terminate called after throwing an instance of 'std::bad_cast'`。gdb `catch throw` 栈：

```
__cxa_throw ← __cxa_bad_cast ← std::use_facet<std::collate<char>>
← _BracketMatcher<char>::_M_apply ← _M_ready
← _Compiler::_M_insert_character_class_matcher ← _Compiler ← basic_regex::_M_compile
← _GLOBAL__sub_I_Base6502Assembler.cpp ← call_init ← dlopen(MesenCore.so)
```

即 `MesenCore.so` 的静态初始化（汇编器基类的 `std::regex` 字符类编译）在 dlopen 时抛
`bad_cast`（缺 collate facet），异常逃出 `dlopen` → terminate。strace 显示 testRunner 在
SDL/X11 之前就 dlopen（line 182 的 `openat(MesenCore.so)`），GUI 模式要等 SDL/X11 之后
（line 796）才读同一文件；gdb 在 GUI 模式跑 25s 零次命中 Catchpoint、无 bad_cast。
四种 locale（C.UTF-8 / en_US.UTF-8 / C / zh_CN.UTF-8）与干净 HOME/XDG/TMPDIR 均复现 ⇒
与 locale/环境无关，是本构建 testRunner 路径的初始化顺序缺陷。**结论：Linux 上别把
`--testRunner` 当可用后端**（仓库 Unix 路径本来就只用 GUI 模式）。

## 证据指针

- `tools/scenario_run.py`（2026-11-24 改造）：`RUNS_ROOT`/`run_id_of`/`prepare_run_dirs`/
  `inject_run_env`/`run_diag`，`kill_orphan_mesen(mesen, run_id)`，锁改
  `analysis/.scenario-<slug>.lock`。
- 并发实测（本卡 #26 A）：两场景各 1200 帧并发 ×3 轮，六份 trace 各 1200 行、均 `DONE`、rc=0。
- 失败诊断实测：`GUNNAC_MESEN=/bin/true python3 tools/scenario_run.py run …` →
  `!! 未完成（… rc=0）` + `[诊断]` 块。
- bad_cast：`gdb -batch -ex 'catch throw' -ex run --args ./tools/mesen2/Mesen --testRunner …`
  的栈；`strace -f -e trace=openat` 对照 GUI 与 testRunner 的 dlopen 行号。
