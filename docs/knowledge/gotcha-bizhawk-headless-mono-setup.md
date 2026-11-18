---
type: gotcha
title: BizHawk 2.10(mono) Linux 无头三坑与配方——OpenAL 卡死/X11 BadMatch/退出不干净，ALSOFT_DRIVERS=null 全解
date: 2026-11-18
status: confirmed
---

# gotcha：BizHawk 2.10（mono）在 Linux 无头跑录像的三坑与配方

## 现象

在 Ubuntu 22.04 用 mono 6.8 跑 `/tmp/bh/EmuHawkMono.sh`（BizHawk 2.10，`--chromeless --lua …
--movie … <rom>`）：

1. **启动即卡死**：进程不退出、**Lua 一行都不执行**、轨迹文件不出现；`--help` 却正常。
   `strace` 显示主线程在 X socket 上忙轮询。
2. 加 `xvfb-run -a` 之后仍打印 **X11 报错两遍**（见下「为什么」2）。
3. **退出码是 137**：Lua 写完产物并 `client.exit()` 后，日志出现
   `BizHawk has completed its shutdown routines, killing process...`，紧接着 shell 打印
   `Killed`，退出码 **137**——看起来像"被 timeout 杀掉/崩了"，其实是 BizHawk 在 shutdown
   末尾**自己 SIGKILL 本进程**（20 帧实测 wall **1.9 s** 就已经 137，`timeout 300` 根本没到期）。
4. **`--userdata` / `client.getuserdata` 路径挂死**：`--help` 确认参数格式
   `--userdata k1:v1;k2:v2`，但带 `--userdata frames:50` 运行 180 s 内既不产轨迹也不退出。

## 为什么

1. **OpenAL 初始化阻塞主线程**：`ALSOFT_DRIVERS` 未指定时 BizHawk 走默认音频后端，
   在无音频设备的环境里卡在设备探测（表象是"Lua 没执行"，实际是根本没进到脚本）。
   解法：`ALSOFT_DRIVERS=null`（并配 `SDL_AUDIODRIVER=dummy` 掐掉 SDL 侧音频）。
2. **X11 `BadMatch` 来自 WinForms 布局，不是致命错误**：
   ```
   X11 Error encountered:
     Error: BadMatch (invalid parameter attributes)
     Request:     12 (0)  Resource ID: 0x20007A
     Control:     BizHawk.Bizware.Graphics.Controls.OpenGLControl
     at BizHawk.Client.EmuHawk.MainForm.SetStatusBar ()
     at BizHawk.Client.EmuHawk.MainForm.MainForm_Load (...)
     at BizHawk.Client.EmuHawk.Program.SubMain (System.String[] args)
   ```
   无窗管的 Xvfb 下 `XGetWMNormalHints`/`SetWindowMinMax` 失败；每次启动 2 次，**不影响**
   Lua 执行、录像播放与产物落盘（实测全片逐帧轨迹完整写出）。
3. **137 是自杀不是崩溃**：`client.exit()` 触发 shutdown routines，收尾末尾 BizHawk 自行
   SIGKILL（日志原话 `killing process`）⇒ 退出码 137，与 timeout/异常无关；
   `timeout` 只是保险。⇒ **判成功看产物行数/内容（`frames+1` 行），不看退出码**。
   （早期把 137 误判成"被 timeout 杀"，20 帧 1.9 s 计时实测证伪。）
4. **`client.getuserdata` 疑与 `comm.socket*` IPC 绑定**：未开 `--socket-ip/--socket-port` 时调用会挂；
   未进一步定位。⇒ 参数传递改用**环境变量**（`os.getenv`）而不是 `--userdata`。

## 何时使用

- Linux 无头录 BizHawk 轨迹/录像回放，一律用本配方（仓库内封装：
  `tools/movie_ref/run_bizhawk_trace.sh`）：
  ```bash
  ALSOFT_DRIVERS=null SDL_AUDIODRIVER=dummy \
  timeout 240 xvfb-run -a /tmp/bh/EmuHawkMono.sh --chromeless \
    --lua <绝对路径>.lua --movie <movie>.bk2 "<rom 绝对路径>"
  ```
- **产物路径必须写绝对路径**：`EmuHawkMono.sh` 会先 `cd` 到 `/tmp/bh`，Lua 里的相对路径会落错地方
  （与 Mesen2 的 CWD 偷换同族坑，见 `gotcha-mesen2-windows-headless-testrunner.md`）。
- 帧数/其它参数用环境变量传（`BH_FRAMES=28408`），**不要用 `--userdata`**。
- 判"脚本是否真执行了"：先看产物是否出现；纯 `client.exit()` 探针秒退=执行了。
  禁凭"没看到输出/进程没退"下结论——OpenAL 坑会把它伪装成"脚本坏了"。
- 通过率估算：约 950 行/分（全片 28408 帧 ≈30 min），脚本每 100 帧 `flush` ⇒ 可边跑边看行数。

## 证据指针

- `docs/findings/movie-bizhawk-6725m.md` §2/§3（核心选择证据 + 无头运行实录与错误原文）。
- 实测：`BH_FRAMES=1200` 产出 `analysis/traces/bizhawk-6725M.jsonl` **1201 行**（`f=0..1200`）；
  同命令跑两次产物 **逐字节相同**（`diff -q` 通过）。
- 日志自证（`/tmp/bh-full.log` 第 5/12 行）：`quickerNES: Booted with Mapper #1 "mmc1"`、
  `Core reported BoardID: "mmc1"`；核心选择键 `/tmp/bh/config.ini:11` `"NES": "quickerNES"`。
- 脚本：`tools/movie_ref/bizhawk_ref.lua`（逐帧 RAM 面）、`tools/movie_ref/run_bizhawk_trace.sh`（配方封装）。
