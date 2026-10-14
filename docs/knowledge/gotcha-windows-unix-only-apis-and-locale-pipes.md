---
type: gotcha
title: Windows 工具链双坑——Unix-only 进程 API 直接断跑 + 管道按 locale 解码把真因换成二次 TypeError
date: 2026-10-14
status: confirmed
---

# gotcha：Unix-only 进程 API 与 locale 管道解码（Windows 断跑实录）

## 现象

同一套工具链在 Linux 全绿，在中文 Windows 上两种死法：

1. **import 即死**：`tools/scenario_run.py` 无条件 `import fcntl` ⇒ `ModuleNotFoundError`；
   即便过了 import，`pgrep`/`os.killpg`/`signal.SIGKILL`/Xvfb `/tmp/.X11-unix` 全部 Unix-only。
2. **解码崩溃掩盖真因**：`python gates/cli.py status` 崩在
   `UnicodeDecodeError: 'gbk' codec can't decode byte 0xbb`（reader 线程），
   紧接着 `TypeError: can only concatenate str (not "NoneType") to str`——
   用户看到的是 TypeError，真正的编码错误被折叠掉了。

## 为什么

- `subprocess.run(capture_output=True, text=True)` 缺 `encoding` 时按**locale 预ferred编码**
  解码子进程输出（中文 Windows=GBK），而 cargo/Rust 子进程按 UTF-8 输出字节 ⇒ 解码必炸；
  Python 子进程又随 locale 走 GBK——**管道两端编码无约定**是根因。
- reader 线程炸掉后 `proc.stdout` 变 `None`，`(proc.stdout + proc.stderr)` 抛出二次 TypeError，
  **第二个异常不是真因**；排障必须回到 traceback 的第一现场。
- `fcntl`/`pgrep`/`killpg`/`SIGKILL`/X11 是平台 API 不是"Python 没装好"，跨平台工具链必须
  在 `os.name` 边界显式分支，而不是祈祷运行环境像 CI。

## 何时使用

- Python 工具链跨 Windows：**子进程管道三件套**——子进程 `PYTHONUTF8=1`、
  父侧 `encoding="utf-8", errors="replace"`、拼接用 `(proc.stdout or "")`。
- 平台 API 显式分支（本次适配的对照表）：`msvcrt.locking` ↔ `fcntl.flock`（串行锁）、
  `taskkill /T /F` ↔ `os.killpg`（进程树清理）、PowerShell CIM 查询 ↔ `pgrep -f`（孤儿清扫）、
  ~~Windows 原生 GUI 直跑 ↔ 自管 Xvfb+DISPLAY~~、`getattr(signal, "SIGKILL", signal.SIGTERM)`。
  > 【2026-10-14 更正】"Windows 原生 GUI 直跑"当日即被 `--testRunner` headless 配方取代
  > （弹窗扰民 + io 沙箱 + CWD 偷换三坑，见
  > [gotcha-mesen2-windows-headless-testrunner.md](gotcha-mesen2-windows-headless-testrunner.md)）；
  > 对照表该行以 headless 为准。
- Lua 侧同理：文本产物 `io.open(p, "wb")` 写 `"\n"`，C 运行时文本模式的 `\n→\r\n` 翻译会
  让 trace 字节面随平台漂移（oracle_sha256 锚定破裂）。
- 看到"第二现场异常"（TypeError/AttributeError on None）先回头找第一个异常。

## 证据指针

- 修复前崩溃 traceback（2026-10-14 `python gates/cli.py status`，GBK `UnicodeDecodeError`
  → `gates/cli.py` `_gate_runner_dispatch` 处 `TypeError`）与
  `ModuleNotFoundError: No module named 'fcntl'`（`tools/scenario_run.py:34`）。
- 修复后负向验证（**不带** PYTHONUTF8 环境变量）：`python gates/cli.py trace` 正常出裁定行
  （FAIL 语义=BYO 工具缺失的响亮报错"Mesen2 未就绪…BYO 工具，见 docs/methodology.md"，
  不再崩溃）；`python gates/cli.py selftest` 37/37。
- 修复面：`tools/scenario_run.py`（ScenarioLock/kill_orphan_mesen/find_mesen/kill_group 分支）、
  `tools/scenario_capture.lua`（文本产物 wb）、`tools/diff_traces.py`、`gates/cli.py`
  （`_child_env` + UTF-8 解码，2026-10-14 fix(tools)/fix(gates) 提交）。
