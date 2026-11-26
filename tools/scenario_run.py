#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/scenario_run.py —— S2 oracle 场景采集框架（驱动 + DSL 唯一解析器）。

职责：
1. 解析场景 DSL（oracle/scenarios/*.txt）——**DSL 语义只在本文件解析**（无第二套解析器）；
2. 生成 analysis/_gen/<name>.lua = PLAN 表字面量 + tools/scenario_capture.lua 执行器；
3. 驱动 Mesen2 2.1.1 采集：
   - Windows：`Mesen --testRunner <lua> <rom> --timeout=N --enableStdout
     --debug.scriptWindow.allowIoOsAccess=true`（headless：无窗口、Lua 自动执行、
     emu.stop 后进程自退；2026-10-14 实测）；
   - Unix：自管 Xvfb + `Mesen rom <lua> /DoNotSaveSettings`（装载用法已验证；
     emu.stop 不退进程 ⇒ 驱动等 DONE 标记后 SIGTERM 收尾）；
4. 校验产物并落 run 清单（可复现证据）。

用法：
    python3 tools/scenario_run.py list
    python3 tools/scenario_run.py gen <name>            # 只生成 lua（调试）
    python3 tools/scenario_run.py run <name|路径> [--timeout 秒]
    python3 tools/scenario_run.py run-all [--timeout 秒]

帧计数约定（与 tools/scenario_capture.lua 一致，差分复现靠它锁死）：
- frame 在 inputPolled 回调递增：第 N 次 poll = 帧 N，schedule[N] 的按键在该帧注入；
- press <f> <btn> = 帧 f..f+press_len-1 按住（press_len 默认 2），之后释放；
- hold <a-b> <btn> = 帧 a..b（含）按住，帧 b+1 释放；
- endFrame（帧 N 渲染完成后）写 trace 行 / 截图 / RAM dump；frame == frames 时写 DONE 并停机。

输入单源（task-2 anti-drift 纪律）：press/hold/press_len/对向键的**逐帧 joy1 展开只有
expand_schedule() 一处实现**（frame→joy1，1-based，非零帧才入表）。Rust 侧（.sched）
与 Lua 侧（PLAN.schedule 预展开嵌入）同消费同一产物；scenario_capture.lua 不得再展开。
对向键（left/right、up/down）同帧双真=**后写覆盖**（holds 先于 presses、按钮列序后者胜）。

输出（均可用 --out/--shots 覆盖，默认 analysis/ 与 shots/）：
    analysis/traces/<name>.frames.jsonl    逐帧状态向量（w=watch 字节/range hex，可选 ram 全量）
    analysis/traces/<name>.captures.jsonl  采集点索引（截图路径 + RAM hex 文件 + cpu 寄存器）
    analysis/traces/<name>_fNNNNN_<tag>.ram.hex  采集点全量 2KB RAM（hex，4096 字符）
    shots/<name>_fNNNNN_<tag>.png          截图（emu.takeScreenshot）
    analysis/traces/<name>.run.json        运行清单（sha256 + 计数 + 路径）
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import os
import shutil
import signal
import subprocess
import sys
import time
from pathlib import Path

IS_WINDOWS = os.name == "nt"
if IS_WINDOWS:
    import msvcrt
else:
    import fcntl

# Windows 无 SIGKILL：退化为 SIGTERM（调用侧 Windows 路径实际走 taskkill /F，语义为强杀）
SIGKILL = getattr(signal, "SIGKILL", signal.SIGTERM)

ROOT = Path(__file__).resolve().parent.parent
MESEN_DIR = ROOT / "tools" / "mesen2"


def find_mesen() -> Path:
    """Mesen2 可执行定位（BYO 工具，tools/mesen2/ 不入库，见 docs/methodology.md）。

    GUNNAC_MESEN 环境变量可覆盖；Windows 认 Mesen.exe/Mesen，Unix 认 Mesen。
    缺失不在 import 期报错（list/gen 不需要 Mesen），run 前显式 fail-loud。"""
    env = os.environ.get("GUNNAC_MESEN")
    if env:
        p = Path(env)
        return p if p.is_absolute() else (ROOT / p)
    names = ("Mesen.exe", "Mesen") if IS_WINDOWS else ("Mesen",)
    for n in names:
        p = MESEN_DIR / n
        if p.is_file():
            return p
    return MESEN_DIR / names[0]


def _abs(env: str, default: str) -> Path:
    v = os.environ.get(env)
    q = Path(v) if v else Path(default)
    return q if q.is_absolute() else (ROOT / q)
ROM = _abs("GUNNAC_ROM", "rom/Metroid (USA).nes")
TRACES_DIR = _abs("GUNNAC_TRACES", "analysis/traces")
SHOTS_DIR = _abs("GUNNAC_SHOTS", "shots")
HARNESS = ROOT / "tools" / "scenario_capture.lua"
SCENARIO_DIR = ROOT / "oracle" / "scenarios"
GEN_DIR = ROOT / "analysis" / "_gen"

BUTTONS = {"a", "b", "start", "select", "up", "down", "left", "right"}
# 按钮位映射（game-platform PadState / hardware.asm:89-96——A=bit7 … Right=bit0）。
# 唯一定义处（task-2）：expand_schedule 用它合成 joy1；Lua 侧经 PLAN.btn_bits 下发同表。
BUTTON_BITS = {
    "right": 0x01, "left": 0x02, "down": 0x04, "up": 0x08,
    "start": 0x10, "select": 0x20, "b": 0x40, "a": 0x80,
}
PRESS_LEN_DEFAULT = 2


class ScenarioError(Exception):
    """DSL 冲突消解规则（2026-10-19 lead 裁定）：①互斥键表（LEFT/RIGHT、UP/DOWN）同帧
双真=后写覆盖（btnOrder 确定性序）②同键多源 OR 保持③冲突消解显式（scenario_capture.lua
互斥段）。DSL 解析/校验错误（带 文件:行号）。"""


def pick_display() -> str:
    """选空闲 X display（:99 起，按锁文件/socket 跳过占用）。"""
    for n in range(99, 200):
        if not Path(f"/tmp/.X{n}-lock").exists() and not Path(f"/tmp/.X11-unix/X{n}").exists():
            return f":{n}"
    raise ScenarioError("无空闲 X display（/tmp/.X11-unix 全占）")


def wait_for_x(display: str, xvfb: subprocess.Popen, timeout_s: float = 10.0) -> None:
    """等 X socket 就绪；Xvfb 起不来即报错（fail-closed）。"""
    sock = Path(f"/tmp/.X11-unix/X{display.lstrip(':')}")
    t0 = time.time()
    while not sock.exists():
        if xvfb.poll() is not None:
            raise ScenarioError(f"Xvfb {display} 启动失败（exit {xvfb.returncode}）")
        if time.time() - t0 > timeout_s:
            raise ScenarioError(f"Xvfb {display} socket 超时未就绪")
        time.sleep(0.1)


def relp(path: Path) -> str:
    """ROOT 内路径显示为相对 POSIX；ROOT 外（如 /tmp 探索场景）原样显示。"""
    try:
        return Path(path).resolve().relative_to(ROOT).as_posix()
    except ValueError:
        return str(path)


# ---------------------------------------------------------------- DSL 解析

def parse_range(text: str) -> tuple:
    """<起>-<止> 区间解析（hold/nav 共用）。"""
    a_s, b_s = text.split("-", 1)
    if not (a_s.isdigit() and b_s.isdigit()) or int(a_s) > int(b_s):
        raise ValueError("区间必须是 起<=止 的非负整数")
    return int(a_s), int(b_s)


def parse_scenario(path: Path) -> dict:
    """解析场景 DSL → PLAN dict。所有语义判定唯一入口。
    scenario 编辑 lint（教训四犯升级——全清正写强制项机械拦）：长场景无菜单起点
    且无 bundle_load=警（菜单误删三例实录——press 540 前缀滤波误删）。"""
    menu_ok = False
    bundle_ok = False
    path = Path(path)
    endframes: list[int] = []  # task-2 语义决议：endFrame → 终端采集点（解析后统一消解去重）
    plan: dict = {
        "name": path.stem,
        "frames": None,
        "press_len": PRESS_LEN_DEFAULT,
        "presses": [],
        "navs": [],
        "triggers": [],
        "holds": [],
        "input_log": None,  # 录像逐帧输入表（tools/movie_import.py 产物；绝对优先覆盖）
        "captures": [],
        "capture_every": None,
        "watch": [],
        "sample_point": None,  # task-24 3b: spin-exit | 缺省 endFrame（混态期 legacy）
        "fixtures": [],  # task-28 ①: {frame, addr, value, persistence}——F_a 注入（设计 v9/v11）
        "bundle_capture": None,  # task-32 收官: {frame, out}——MSS+三直映采集
        "bundle_load": None,     # task-32 收官: {path}——任意起点装载
        "resume_at": None,       # task-45: 装载后把 harness 帧计数器置到该帧（续跑）
        "watch_ranges": [],
        "ramframe": False,
        "coverage": False,
        "coverage_pc": [],
        "cputrace_addrs": [],
        "cputrace_read_addrs": [],
        "cputrace_write_addrs": [],
        "cputrace_window": None,
        "probe_mem": [],
        "probe_dispatch": [],
        "entry_probe": [],
        "vram_snap": [],  # GGR 移植件2：中帧 VRAM 快照（扫描线级，如 vram_snap 64,128,192）
        "vrsnap_poll": None,  # vram_snap 轮询面（oam|zp，缺省 oam）
        "write_hooks": [],  # GGR 移植件3：NT/CHR/OAM 写钩（write_hooks nt|chr|oam，可多次）
    }
    for lineno, raw in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        line = raw.split("#", 1)[0].strip()
        if not line:
            continue
        toks = line.split()
        op, args = toks[0], toks[1:]

        def err(msg: str) -> ScenarioError:
            return ScenarioError(f"{path}:{lineno}: {msg}（行: {raw.strip()!r}）")

        if op == "name":
            if len(args) != 1:
                raise err("name 需要 1 个参数")
            plan["name"] = args[0]
        elif op == "frames":
            _frames_val = int(args[0])
            if len(args) != 1 or not args[0].isdigit() or int(args[0]) <= 0:
                raise err("frames 需要正整数")
            plan["frames"] = int(args[0])
        elif op == "press_len":
            if len(args) != 1 or not args[0].isdigit() or int(args[0]) <= 0:
                raise err("press_len 需要正整数")
            plan["press_len"] = int(args[0])
        elif op == "trigger":
            # sc-loop v3 触发式：trigger <f1-f2> <reg> <op> <val> <hold|press> <按钮...>
            # 条件（RAM 检测）触发按键——复现确定性（时窗→状态触发）
            if len(args) < 6:
                raise err("trigger 用法: trigger <起>-<止> <reg> <!|==|>|< > <val> <hold|press> <按钮...>")
            try:
                lo, hi = parse_range(args[0])
            except ValueError as e:
                raise err(f"trigger 区间错: {e}")
            reg = int(args[1], 16)
            op_ = args[2]
            if op_ not in ("!=", "==", ">", "<"):
                raise err(f"trigger op 未知: {op_}")
            val = int(args[3], 0)
            mode = args[4]
            if mode not in ("hold", "press"):
                raise err(f"trigger mode 未知: {mode}")
            btns = args[5:]
            plan["triggers"].append({"from": lo, "to": hi, "reg": reg, "op": op_,
                                     "val": val, "mode": mode, "buttons": btns})
        elif op == "nav":
            # sc-loop 反馈制导航：nav <f1-f2> <x|mapx|jump> <target> [按钮...]
            # 实时读 RAM（ScrX=$51/ScrY=$52/MapX=$50）→条件按键（G 系共用底）
            if len(args) < 3:
                raise err("nav 用法: nav <起>-<止> <x|mapx|jump> <目标> [按钮...]")
            try:
                lo, hi = parse_range(args[0])
            except ValueError as e:
                raise err(f"nav 区间错: {e}")
            kind = args[1]
            if kind not in ("x", "mapx", "jump"):
                raise err(f"nav 种类未知: {kind}（x/mapx/jump）")
            target = int(args[2], 0)
            btns = args[3:] if len(args) > 3 else (["a"] if kind == "jump" else ["left", "right"])
            plan["navs"].append({"from": lo, "to": hi, "kind": kind,
                                 "target": target, "buttons": btns})
        elif op == "press":
            if len(args) < 2 or not args[0].isdigit():
                raise err("press 用法: press <帧号> <按钮...>")
            btns = [b.lower() for b in args[1:]]
            for b in btns:
                if b not in BUTTONS:
                    raise err(f"未知按钮 {b!r}（可选: {sorted(BUTTONS)}）")
            plan["presses"].append({"frame": int(args[0]), "buttons": btns})
        elif op == "press_every":
            # press_every <N> <起>-<止> <按钮...>：每 N 帧点按一次（解析期展开为 presses，无新执行语义）
            if len(args) < 3 or not args[0].isdigit() or int(args[0]) <= 0 or "-" not in args[1]:
                raise err("press_every 用法: press_every <N> <起>-<止> <按钮...>")
            a_s, b_s = args[1].split("-", 1)
            if not (a_s.isdigit() and b_s.isdigit()) or int(a_s) > int(b_s):
                raise err("press_every 区间必须是 起<=止 的非负整数")
            btns = [b.lower() for b in args[2:]]
            for b in btns:
                if b not in BUTTONS:
                    raise err(f"未知按钮 {b!r}（可选: {sorted(BUTTONS)}）")
            for f in range(int(a_s), int(b_s) + 1, int(args[0])):
                plan["presses"].append({"frame": f, "buttons": btns})
        elif op == "hold":
            if len(args) < 2 or "-" not in args[0]:
                raise err("hold 用法: hold <起>-<止> <按钮...>")
            a_s, b_s = args[0].split("-", 1)
            if not (a_s.isdigit() and b_s.isdigit()) or int(a_s) > int(b_s):
                raise err("hold 区间必须是 起<=止 的非负整数")
            btns = [b.lower() for b in args[1:]]
            for b in btns:
                if b not in BUTTONS:
                    raise err(f"未知按钮 {b!r}（可选: {sorted(BUTTONS)}）")
            plan["holds"].append({"from": int(a_s), "to": int(b_s), "buttons": btns})
        elif op == "inputlog":
            # 录像输入面：inputlog <路径>（相对仓库根；逐帧表由 tools/movie_import.py 生成）。
            # 语义=该表对 1..N 帧**绝对优先**（覆盖 press/hold 的合成结果）——录像即权威输入。
            if len(args) != 1:
                raise err("inputlog 用法: inputlog <路径>")
            plan["input_log"] = args[0]
        elif op == "capture":
            if len(args) != 2 or not args[0].isdigit():
                raise err("capture 用法: capture <帧号> <tag>")
            plan["captures"].append({"frame": int(args[0]), "tag": args[1]})
        elif op == "capture_every":
            if len(args) != 2 or not args[0].isdigit() or int(args[0]) <= 0:
                raise err("capture_every 用法: capture_every <N> <tag>")
            plan["capture_every"] = {"n": int(args[0]), "tag": args[1]}
        elif op == "watch":
            # watch <hex 地址> [标签] —— 标签缺省=地址本身（语义命名必须有台账证据，见 AGENTS.md §3）
            if len(args) not in (1, 2):
                raise err("watch 用法: watch <hex 地址> [标签]")
            addr = args[0].lower()
            if len(addr) != 4 or any(c not in "0123456789abcdef" for c in addr):
                raise err(f"地址须为 4 位 hex（如 0320），实际 {args[0]!r}")
            plan["watch"].append({"addr": addr, "label": args[1] if len(args) == 2 else addr})
        elif op == "probe_mem":
            if len(args) != 2:
                raise err("probe_mem 用法: probe_mem <lo> <hi>（16 进制，NMI 级写点插桩）")
            plan["probe_mem"].append([args[0], args[1]])
        elif op == "vram_snap":
            # GGR 移植件2：中帧 VRAM 快照（扫描线级热写轮询——NT+CHR 各一拍/帧/扫描线）。
            # vram_snap <sl>[,<sl>...]（0-261，逗号分隔；可多次）。源手法=
            # GGR oracle/probes/ppu_corridor_probe.lua:8-15（sl 64/128/192 热写轮询+
            # 「N-1 OAM 代位 vs 帧内 $2007 写流」二择判别器的捕获半）。
            # 【本仓实测扩展 0→261】Metroid 主循环=NMI 标志旋等（$C0D3 lda/bne），
            # 游戏写流集中 NMI 后 vblank（scanline 240-260）——GGR 可见区拍点
            # （64/128/192）在本仓结构性空窗（vrsnap-smoke/-zp 首跑实录：仅 boot
            # init f0-f5 命中），NMI 后窗才是本仓「中帧」价值窗。261=PAL 全界，
            # NTSC 有效 0-260。
            if not args:
                raise err("vram_snap 用法: vram_snap <sl>[,<sl>...]（0-261）")
            sls = []
            for tok in " ".join(args).split(","):
                tok = tok.strip()
                if not tok.isdigit():
                    raise err(f"vram_snap 扫描线须为整数: {tok!r}")
                sl = int(tok)
                if not 0 <= sl <= 261:
                    raise err(f"vram_snap 扫描线超界 0-261: {sl}")
                sls.append(sl)
            for sl in sls:
                if sl not in plan["vram_snap"]:
                    plan["vram_snap"].append(sl)
        elif op == "vrsnap_poll":
            # vram_snap 轮询面选择（可选，缺省 oam）：
            #   oam=$0200-$02FF 写（GGR 源手法面；Metroid 实测中帧命中稀——写流集中
            #   vblank 附近，见 vrsnap-smoke 首跑实录）；
            #   zp=$0000-$00FF 写（游戏主循环逐扫描线写零页——中帧命中密；getState
            #   开销换节奏，验证实测 wall 可承受则留）。
            if len(args) != 1 or args[0] not in ("oam", "zp"):
                raise err("vrsnap_poll 用法: vrsnap_poll oam|zp")
            plan["vrsnap_poll"] = args[0]
        elif op == "write_hooks":
            # GGR 移植件3：NT/CHR/OAM 写钩三件套（$2006 锁存+$2007→.ntwrites.jsonl /
            # $2007 v2 ctrl bit2 1/32 增量→.chrwrites.jsonl 含 PC / $0200-$02FF+PC→
            # .oamwrites.jsonl）。write_hooks nt|chr|oam（可多次）。
            # 【GGR v1 教训对照】MMIO 写回调须枚举 emu.callbackType.write 注册
            # （字符串 "write" 静默落 read 域=盲区——本仓 gotcha
            # gotcha-mesen2-callbacktype-string-enum-split.md / T1 定谳同根因）。
            if len(args) != 1 or args[0] not in ("nt", "chr", "oam"):
                raise err("write_hooks 用法: write_hooks nt|chr|oam")
            if args[0] not in plan["write_hooks"]:
                plan["write_hooks"].append(args[0])
        elif op == "fixture":
            # task-28 ①：夹具状态设置（state-fixture 设计 §3 F_a 注入模型）——
            # fixture <帧号> <hex地址> <hex值> [invariant|evolving]
            # （persistence 语义：invariant=窗口末读回断言；evolving=演化观测面）
            if len(args) not in (3, 4):
                raise err("fixture 用法: fixture <帧号> <hex地址> <hex值> [invariant|evolving]")
            if not args[0].isdigit():
                raise err(f"fixture 帧号须为非负整数: {args[0]!r}")
            for a in args[1:3]:
                if a.lower() in ("a", "x", "y", "p"):
                    continue  # CPU 态 register 名（注入径修（register set API））
                if len(a) not in (2, 4) or any(c not in "0123456789abcdef" for c in a.lower()):
                    raise err(f"fixture 地址/值须为 hex（2/4 位地址、2 位值）或 register 名（a/x/y/p）: {a!r}")
            pers = args[3].lower() if len(args) == 4 else "invariant"
            if pers not in ("invariant", "evolving"):
                raise err(f"fixture persistence 须为 invariant|evolving: {args[3]!r}")
            plan["fixtures"].append({"frame": int(args[0]), "addr": args[1].lower(),
                                     "value": args[2].lower(), "persistence": pers})
        elif op == "bundle_capture":
            # task-32 收官：bundle 采集——bundle_capture <帧> <输出前缀>
            # （exec 回调上下文 createSavestate→<前缀>.mss + <前缀>.json 三直映）
            if len(args) != 2 or not args[0].isdigit():
                raise err("bundle_capture 用法: bundle_capture <帧> <输出前缀>")
            plan["bundle_capture"] = {"frame": int(args[0]), "out": args[1]}
        elif op == "resume_at":
            # task-45：续跑帧号——`resume_at <F>` 与 `snapshot <前缀>` 配对使用：
            # 装载第 F 帧末存档后把 harness 帧计数器置 F，schedule 从 F+1 继续喂。
            if len(args) != 1 or not args[0].isdigit():
                raise err("resume_at 用法: resume_at <帧号>")
            plan["resume_at"] = int(args[0])
        elif op in ("snapshot", "bundle_load"):
            # task-2 语义决议：snapshot <前缀> 为规范名，≡ bundle_load（别名保留防破坏
            # 既有 scenario）——三要素②「待加载存档」声明（首 spin 出口 loadSavestate）。
            # `snapshot none`=显式无存档（从上电起跑，cases/case-001 实用形态）。
            if len(args) != 1:
                raise err(f"{op} 用法: {op} <前缀|none>")
            plan["bundle_load"] = None if args[0].lower() == "none" else {"path": args[0]}
        elif op == "endFrame":
            # task-2 语义决议：endFrame <帧> 定死=终端采集点（≡ capture <帧> end；
            # 同帧已有 capture 则去重不重复采集）——解析后统一消解（与行序无关）
            if len(args) != 1 or not args[0].isdigit():
                raise err("endFrame 用法: endFrame <帧号>")
            endframes.append(int(args[0]))
        elif op == "sample_point":
            # task-24 3b：L3 采样点声明——spin-exit=$C0D3 @exitLoop_waitNMIEnd
            # （m1disasm prg7_engine.asm:218）；缺省=endFrame（legacy/混态期）。
            if len(args) != 1 or args[0] not in ("spin-exit", "endFrame"):
                raise err("sample_point 用法: sample_point spin-exit|endFrame")
            plan["sample_point"] = args[0]
        elif op == "coverage_pc":
            # r3 decode-sweep 动态收口：pc 过滤型指令入口覆盖（execute 回调 + cpu.pc==地址
            # 才记——精确指令入口观测；已知盲点=中断入口首指令，见 scenario_capture.lua 注）。
            # 可多次，区间针对待查段（勿全 bank 撒网：回调热度）。
            if len(args) != 1 or "-" not in args[0]:
                raise err("coverage_pc 用法: coverage_pc <hex起>-<hex止>（可多次）")
            a_s, b_s = (s.lower() for s in args[0].split("-", 1))
            if not (len(a_s) == 4 and len(b_s) == 4):
                raise err("coverage_pc 区间须为 4 位 hex")
            if int(a_s, 16) > int(b_s, 16):
                raise err("coverage_pc 起点必须 <= 止点")
            plan["coverage_pc"].append([a_s, b_s])
        elif op == "probe_dispatch":
            # r3：分发器入口探针（pc 过滤命中）——记 A/site/target + $8000 窗口指纹。
            # site 由栈返回址-1 反推（site = ret-1 - 2），target = word(site+3+2*A)。
            if len(args) != 1:
                raise err("probe_dispatch 用法: probe_dispatch <hex 地址>（如 C27C）")
            ad = args[0].lower()
            if len(ad) != 4 or any(c not in "0123456789abcdef" for c in ad):
                raise err("probe_dispatch 地址须为 4 位 hex")
            if ad not in plan["probe_dispatch"]:
                plan["probe_dispatch"].append(ad)
        elif op == "entry_probe":
            # task-12：执行路径调用图——函数级入口全量探针（callbackType.exec 真执行）。
            # entry_probe <path>：path=JSON 数组（4 位 hex 字符串清单，tools/call_graph.py
            # anchors 子命令生成）；登记后每入口记执行序/首帧/调用者边。
            if len(args) != 1:
                raise err("entry_probe 用法: entry_probe <anchors.json 路径>")
            ap = Path(args[0])
            if not ap.is_file():
                ap = ROOT / args[0]
            if not ap.is_file():
                raise err(f"entry_probe 文件不存在: {args[0]}")
            try:
                lst = json.loads(ap.read_text(encoding="utf-8"))
            except json.JSONDecodeError as e:
                raise err(f"entry_probe JSON 解析失败: {e}")
            if not isinstance(lst, list) or not lst:
                raise err("entry_probe 文件须为非空 JSON 数组")
            for ad in lst:
                if not isinstance(ad, str) or len(ad) != 4 or \
                        any(c not in "0123456789abcdef" for c in ad.lower()):
                    raise err(f"entry_probe 元素须为 4 位 hex 字符串: {ad!r}")
            plan["entry_probe"] = [str(a).lower() for a in lst]
        elif op == "watch_range":
            # watch_range <hex 起>-<hex 止> [标签]
            if len(args) not in (1, 2) or "-" not in args[0]:
                raise err("watch_range 用法: watch_range <hex起>-<hex止> [标签]")
            a_s, b_s = (s.lower() for s in args[0].split("-", 1))
            if not (len(a_s) == 4 and len(b_s) == 4):
                raise err("watch_range 地址须为 4 位 hex")
            if int(a_s, 16) > int(b_s, 16):
                raise err("watch_range 起点必须 <= 止点")
            label = args[1] if len(args) == 2 else f"{a_s}-{b_s}"
            # 【吞吐卡分片（2026-11-02）】wram 大区间 shard=4（每 4 帧写一次）——IPC
            # 10240→1024 次/帧（10x（read32 4x+分片 2.5x））——PLAN 携带（executor/
            # diff_traces 同源消费（分片镜像））。
            shard = 4 if label == "wram" else 1
            plan["watch_ranges"].append({"from": a_s, "to": b_s, "label": label,
                                         "shard": shard})
        elif op == "ramframe":
            if len(args) != 1 or args[0] not in ("on", "off"):
                raise err("ramframe 用法: ramframe on|off")
            plan["ramframe"] = args[0] == "on"
        elif op == "coverage":
            # 覆盖采集（缺口左移机制③）：execute 回调记录执行过的地址 → <name>.coverage.jsonl
            if len(args) > 1 or (args and args[0] not in ("on", "off")):
                raise err("coverage 用法: coverage [on|off]")
            plan["coverage"] = (args[0] if args else "on") != "off"
        elif op == "cputrace":
            # 取指探针（exec→<name>.cputrace.log，scenario_capture.lua:321）
            if len(args) != 1:
                raise err("cputrace 用法: cputrace <4hex>")
            ad = args[0].lower()
            if len(ad) != 4 or any(c not in "0123456789abcdef" for c in ad):
                raise err("cputrace 地址须为 4 位 hex")
            ai = int(ad, 16)  # lua format('%04X', a) 需整数（2026-11-11 修：字符串致 lua:342 类型错）
            if ai not in plan["cputrace_addrs"]:
                plan["cputrace_addrs"].append(ai)
        elif op == "ctread":
            # 总线读探针（read 回调→<name>.ctread.log，scenario_capture.lua:347）
            if len(args) != 1:
                raise err("ctread 用法: ctread <4hex>")
            ad = args[0].lower()
            if len(ad) != 4 or any(c not in "0123456789abcdef" for c in ad):
                raise err("ctread 地址须为 4 位 hex")
            ai = int(ad, 16)  # lua format('%04X', a) 需整数（2026-11-11 修：字符串致 lua:342 类型错）
            if ai not in plan["cputrace_read_addrs"]:
                plan["cputrace_read_addrs"].append(ai)
        elif op == "ctwrite":
            # 数据写探针（write 回调→<name>.ctwrite.log，scenario_capture.lua:419）
            if len(args) != 1:
                raise err("ctwrite 用法: ctwrite <4hex>")
            ad = args[0].lower()
            if len(ad) != 4 or any(c not in "0123456789abcdef" for c in ad):
                raise err("ctwrite 地址须为 4 位 hex")
            ai = int(ad, 16)  # lua format('%04X', a) 需整数（2026-11-11 修：字符串致 lua:342 类型错）
            if ai not in plan["cputrace_write_addrs"]:
                plan["cputrace_write_addrs"].append(ai)
        elif op == "ctwindow":
            # 探针窗（十进制帧闭区间；trace 行计数语义 scenario_capture.lua:324）
            if len(args) != 2:
                raise err("ctwindow 用法: ctwindow <起帧> <止帧>")
            try:
                w0, w1 = int(args[0]), int(args[1])
            except ValueError:
                raise err("ctwindow 须为十进制整数")
            if w0 > w1:
                raise err("ctwindow 起点必须 <= 止点")
            plan["cputrace_window"] = [w0, w1]
        else:
            raise err(f"未知 DSL 指令 {op!r}")

    if plan["frames"] is None:
        raise ScenarioError(f"{path}: 缺 frames 指令")
    frames = plan["frames"]
    # endFrame → 终端采集点消解（≡ capture <帧> end，同帧已有 capture 去重——与行序无关）
    cap_frames = {c["frame"] for c in plan["captures"]}
    for ef in endframes:
        if ef not in cap_frames:
            plan["captures"].append({"frame": ef, "tag": "end"})
            cap_frames.add(ef)
    seen = set()
    for e in plan["presses"]:
        if not 1 <= e["frame"] <= frames:
            raise ScenarioError(f"{path}: press 帧 {e['frame']} 超出 1..{frames}")
        key = (e["frame"], tuple(e["buttons"]))
        if key in seen:
            raise ScenarioError(f"{path}: 重复 press {key}")
        seen.add(key)
    for e in plan["holds"]:
        if not 1 <= e["from"] <= e["to"] <= frames:
            raise ScenarioError(f"{path}: hold 区间 {e['from']}-{e['to']} 超出 1..{frames}")
    for e in plan["captures"]:
        if not 1 <= e["frame"] <= frames:
            raise ScenarioError(f"{path}: capture 帧 {e['frame']} 超出 1..{frames}")
    # task-45：续跑帧号只与存档装载配对使用；越界/孤立一律 fail-loud（防「以为续跑了、
    # 其实还是从第 1 帧喂输入」的静默错位）。
    if plan.get("resume_at") is not None:
        if not plan.get("bundle_load"):
            raise ScenarioError(f"{path}: resume_at 需要 `snapshot <前缀>` 指定存档（任意起点）")
        if not 1 <= plan["resume_at"] < frames:
            raise ScenarioError(
                f"{path}: resume_at {plan['resume_at']} 越界：应落在 1..{frames - 1}（末帧 {frames}）")
    # scenario 编辑 lint（教训四犯——机械拦）：长场景须有菜单起点或 bundle_load
    # （任意起点案）——防前缀滤波误删菜单。
    # 【启发式放宽（2026-11-12，WARN 假阳修正）】旧形硬找 `press 540 start` 恰一帧——
    # morphball/scrollseam 等用 310/350 双 start（更早标题流程，f320/f420 锚 0px 证输入
    # 流正确）被误 WARN。不变量=「前 700 帧内至少一次 start press」（菜单必在早期发生）；
    # 零 start 仍 WARN（真误删形态）。非门禁弱化：WARN 非阻塞、判据收紧为不变量级。
    if plan.get("frames", 0) > 500 and not plan.get("bundle_load"):
        # 【2026-11-18 扩口径】除 `press` 外也认 `inputlog` 带来的 Start 位——门过渡机制场景
        # 直接用录像前 600 帧当输入（作者在 f22/f26 按 Start），静态数 `presses` 会误 WARN。
        # 判据仍是机械的：**展开后的前 700 帧里至少有一次 Start 按下**（press 与 inputlog 同一口径）。
        _menu = any(p.get("frame", 10**9) <= 700 for p in plan.get("presses", [])
                    if any(str(b).lower() == "start" for b in p.get("buttons", [])))
        if not _menu and plan.get("input_log"):
            try:
                _pads = expand_pads(plan)[0]
                _menu = any((bits >> 4) & 1 for f, bits in _pads.items() if f <= 700)
            except ScenarioError:
                _menu = False
        if not _menu:
            print(f"[scenario-lint] WARN {path.name}: 长场景前 700 帧无 start press——"
                  f"疑菜单误删（教训四犯机械拦）——正写全清核查", file=sys.stderr)
    return plan


# ---------------------------------------------------------------- 输入展开（唯一源）

def expand_schedule(plan: dict) -> dict[int, int]:
    """逐帧 **1P** 输入展开（向后兼容门面——唯一实现在 :func:`expand_pads`）。"""
    return expand_pads(plan)[0]


def expand_pads(plan: dict) -> tuple[dict[int, int], dict[int, int]]:
    """逐帧输入展开**唯一实现**（task-2 输入单源——anti-drift 硬判据的根）。

    返回 `(joy1, joy2)`：1P 来自 DSL（press/hold）与 `inputlog` 的 p1 列；2P **只来自
    `inputlog` 的 p2 列**（Metroid 的 2P 仅在「Up+A 放弃进度回密码屏」这类原版 quirk 上
    使用，手写 DSL 不造第二套语法——需要 2P 就写录像）。

    2P 语义的 asm 锚：`PauseMode ; 07:C9B1` 读 `Joy2Status($15)` 的 `A|UP`。两侧同源：
    emu 腿 `emu.setInput(btn, 1)`、Rust 腿 `FrameMemory.joy2`——`ReadJoyPads ; 07:C215`
    本来就读双端口，缺的只是把值喂进来。

    展开 DSL 静态输入段 → `{frame: joy1}`（1-based，非零帧才入表，返回按帧号升序）：
      · hold <a-b> <btn>：帧 a..b（含）按住；
      · press <f> <btn>：帧 f..f+press_len-1 按住（press_len 默认 PRESS_LEN_DEFAULT）；
      · 同帧多按钮 OR 成 joy1（位映射 BUTTON_BITS）；
      · 对向键（left/right、up/down）同帧双真=**后写覆盖**：holds 先于 presses 依序登记，
        同段内按按钮列序，后登记者胜（确定性——spec 冲突消解规则 ①）。

    Rust 侧（tools/l4_diff_run.py 写 .sched）与 Lua 侧（generate_lua 嵌入 PLAN.schedule）
    同消费本函数产物——**禁止第二处展开**（两处实现漂移=两侧跑不同输入=假对拍）。
    trigger/nav 为运行时条件输入（依赖 RAM 读），不在静态展开面（scenario_capture.lua 运行时叠加）。
    """
    press_len = int(plan.get("press_len") or PRESS_LEN_DEFAULT)
    # 每帧记录 (frame -> {btn: 登记序}) 以复现「后写覆盖」冲突消解
    per_frame: dict[int, dict[str, int]] = {}
    seq = 0

    def add(a: int, b: int, btns: list[str]) -> None:
        nonlocal seq
        for f in range(a, b + 1):
            d = per_frame.setdefault(f, {})
            for btn in btns:
                seq += 1
                d[btn] = seq

    for e in plan.get("holds", []):
        add(int(e["from"]), int(e["to"]), e["buttons"])
    for e in plan.get("presses", []):
        add(int(e["frame"]), int(e["frame"]) + press_len - 1, e["buttons"])

    # 录像输入表（inputlog）：逐帧绝对优先——录像即权威输入，press/hold 只用于
    # 「无录像的用例」。表为空/未声明的用例字节不变（锚不漂移）。
    log_path = plan.get("input_log")
    log_table: dict[int, tuple[int, int, int]] = {}
    if log_path:
        sys.path.insert(0, str(Path(__file__).resolve().parent))
        import movie_import  # 单源：.inputlog 读写语义只在 movie_import

        lp = Path(log_path)
        if not lp.is_absolute():
            lp = ROOT / lp
        log_table = movie_import.read_inputlog(lp)

    # 【#111 注入相位根因】录像**复位帧必须被消费**：inputlog 的 cmd bit0/bit1 是软/硬复位
    # （tools/movie_import.py 写出端，FCEUX 影片首行恒为复位）。FCEUX 播片时该帧只做复位、
    # 不产生一帧游戏输入，后续录像帧整体后移一帧；旧实现把复位帧当普通输入帧喂进去
    # （`_cmd` 读而不用），于是整卷输入相对影片**晚一帧**。
    # A 级证据（docs/findings/tas-injection-phase.md §11）：lordtom 100% 首伤
    # emu f1759 / FCEUX f2190；消费复位帧后 emu 首伤 f2192（Δ2=采样点约定），且
    # f4000 前不再死亡（旧行为 f3889 即死）。
    reset_shift = 0
    if log_table:
        shifted: dict[int, tuple[int, int, int]] = {}
        for f in sorted(log_table):
            p1, p2, cmd = log_table[f]
            if cmd & 0x03:
                reset_shift += 1
                continue
            shifted[f + reset_shift] = (p1, p2, cmd)
        if reset_shift:
            log_table = shifted

    joy: dict[int, int] = {}
    for f in sorted(per_frame):
        d = per_frame[f]
        # 对向键后写覆盖（登记序小者剔除）
        if d.get("left") and d.get("right"):
            d.pop("left" if d["left"] < d["right"] else "right")
        if d.get("up") and d.get("down"):
            d.pop("up" if d["up"] < d["down"] else "down")
        bits = 0
        for btn in d:
            bits |= BUTTON_BITS[btn]
        if bits:
            joy[f] = bits
    for f, (p1, _p2, _cmd) in log_table.items():
        if p1:
            joy[f] = p1
        else:
            joy.pop(f, None)

    # 2P：只认录像 p2 列（非零帧才入表；与 1P 同「非零才落行」口径——sched 字节最小化）
    joy2: dict[int, int] = {}
    for f, (_p1, p2, _cmd) in log_table.items():
        if p2:
            joy2[f] = p2
    return joy, joy2


# ---------------------------------------------------------------- Lua 生成

def lua_str(s: str) -> str:
    out = s.replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n").replace("\r", "\\r")
    return f'"{out}"'


def to_lua(v) -> str:
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, int):
        return str(v)
    if isinstance(v, str):
        return lua_str(v)
    if isinstance(v, list):
        return "{" + ",".join(to_lua(x) for x in v) + "}"
    if isinstance(v, dict):
        # int 键（如 schedule 的帧号）直接作 Lua 数字键；str 键走字符串键形
        return "{" + ",".join(
            f"[{k if isinstance(k, int) else lua_str(k)}]={to_lua(val)}" for k, val in v.items()
        ) + "}"
    raise TypeError(f"不可序列化为 Lua: {v!r}")


def _lint_lua_text(text: str, label: str) -> None:
    """task-38 ③+采集截取缝契约：机械防（清单单源=tools/lint_mesen_lua.py）。

    2026-11-03 fade-blink-capture-race：endFrame 直读截图=渲染面翻转竞态窗（淡入窗
    FC&1 翻格根）——SEAM 契约（requestShot 登记+fulfillShot 缝兑现）机械强制，
    legacy 直读径拒跑（fail-loud，不静默降级）。
    """
    import lint_mesen_lua as _lint
    hits = _lint.lint_text(text, label) + _lint.lint_seam(text, label)
    if hits:
        raise SystemExit(f"[lua lint] 拒跑：\n  " + "\n  ".join(hits))


def generate_lua(plan: dict, scenario_path: Path) -> tuple[str, str]:
    """返回 (lua 源码, lua sha256)。PLAN 前置 + 执行器整拼（无 dofile 依赖）。

    task-2 输入单源：逐帧 joy1 由 **expand_schedule 唯一展开**后以 PLAN.schedule 嵌入
    （Rust 侧 .sched 同源同产物）；PLAN.btn_bits=按钮位表（同源下发，Lua 不复制第二份）。
    raw presses/holds **不进 Lua**——Lua 物理上不可二次展开（anti-drift 结构保证）。
    """
    # None 字段（缺省 DSL 项如 capture_every）不进 Lua 表（r4 工程卫生：缺省场景必须可跑）
    plan_lua = {k: v for k, v in plan.items() if v is not None}
    # raw 输入段不进 Lua（唯一展开源=expand_schedule——防二次展开漂移假对拍）
    plan_lua.pop("presses", None)
    plan_lua.pop("holds", None)
    joy1, joy2 = expand_pads(plan)
    plan_lua["schedule"] = joy1
    if joy2:  # 无 2P 的用例不落键（PLAN 字节不漂移）
        plan_lua["schedule2"] = joy2
    plan_lua["btn_bits"] = BUTTON_BITS
    plan_lua["out_dir"] = str((TRACES_DIR).resolve())
    plan_lua["shots_dir"] = str((SHOTS_DIR).resolve())
    # task-32：bundle 输出前缀解析绝对路径（Mesen CWD≠仓库根——相对路径 io.open
    # 静默失败崩回调实录）+ 建父目录
    if plan_lua.get("bundle_capture"):
        out = Path(plan_lua["bundle_capture"]["out"])
        if not out.is_absolute():
            out = ROOT / out
        out.parent.mkdir(parents=True, exist_ok=True)
        plan_lua["bundle_capture"]["out"] = str(out.resolve())
    if plan_lua.get("bundle_load"):
        lp = Path(plan_lua["bundle_load"]["path"])
        if not lp.is_absolute():
            lp = ROOT / lp
        plan_lua["bundle_load"]["path"] = str(lp.resolve())
    harness = HARNESS.read_text(encoding="utf-8")
    src = (
        f"-- 生成文件：勿手改。来源 {relp(scenario_path)}"
        f"（由 tools/scenario_run.py 生成）\n"
        f"local PLAN = {to_lua(plan_lua)}\n"
        + harness
    )
    return src, hashlib.sha256(src.encode("utf-8")).hexdigest()


# ---------------------------------------------------------------- 运行

def scenario_path_of(name_or_path: str) -> Path:
    p = Path(name_or_path)
    if p.is_file():
        return p
    cand = SCENARIO_DIR / f"{name_or_path}.txt"
    if cand.is_file():
        return cand
    raise ScenarioError(f"找不到场景 {name_or_path!r}（试过 {p} 与 {cand}）")


def archive_existing(name: str) -> str | None:
    """产物已存在 ⇒ 按时间戳归档（t24 事故：重跑踩踏他人在用的 trace；归档=run 隔离）。"""
    stamp = time.strftime("%Y%m%d-%H%M%S")
    dst = TRACES_DIR / ".archive" / f"{name}-{stamp}"
    moved = False
    # 类级修复（task-15 实测）：按 `<name>.*` 全量归档——旧 6 族白名单漏了
    # probe/entries/edges/nmi/dispatch/window/exec_cov 等新产物族（run1 被覆盖案）。
    for src in sorted(TRACES_DIR.glob(f"{name}.*")):
        if src.is_file() and not re.search(r"\.run[12]\.", src.name):
            dst.mkdir(parents=True, exist_ok=True)
            shutil.move(str(src), str(dst / src.name))
            moved = True
    for src in (TRACES_DIR).glob(f"{name}_f*.ram.hex"):
        dst.mkdir(parents=True, exist_ok=True)
        shutil.move(str(src), str(dst / src.name))
        moved = True
    for src in (SHOTS_DIR).glob(f"{name}_f*.png"):
        shots_dst = dst / "shots"
        shots_dst.mkdir(parents=True, exist_ok=True)
        shutil.move(str(src), str(shots_dst / src.name))
        moved = True
    return dst.relative_to(ROOT).as_posix() if moved else None



def historical_first_sample_s(name: str, first_sample_frame: int) -> list[float]:
    """历史 run 的『启动→首采样帧』秒数样本：新字段 first_sample_s 直读；旧 run 以
    wall_s × 首采样帧/总帧 估算（同一驱动/机器，比例模型足够做 3 倍告警基线）。"""
    samples: list[float] = []
    for d in sorted((TRACES_DIR / ".archive").glob(f"{name}-*")):
        mp = d / f"{name}.run.json"
        if not mp.is_file():
            continue
        try:
            m = json.loads(mp.read_text(encoding="utf-8"))
        except Exception:
            continue
        if "first_sample_s" in m and m["first_sample_s"]:
            samples.append(float(m["first_sample_s"]))
        elif m.get("wall_s") and m.get("frames"):
            samples.append(float(m["wall_s"]) * first_sample_frame / max(1, int(m["frames"])))
    return samples


def normalize_paths_in_outputs(name: str) -> None:
    """产物 JSON 禁绝对路径（可移植性）：把 ROOT 前缀归一为仓库相对路径。

    Lua 落盘需要绝对目录（PLAN.shots_dir），但 captures.jsonl/run.json 对外
    暴露的路径必须相对——2026-10-14 用户裁定。
    Windows 适配：ROOT 前缀有 4 种形态（正/反斜杠 × Lua 原样/JSON 反斜杠转义），
    全部剥除，产物 JSON 在两个平台字节面一致。
    """
    root = str(ROOT)
    esc = root.replace("\\", "\\\\")
    prefixes = (root + "/", root + "\\", esc + "/", esc + "\\")
    for f in [TRACES_DIR / f"{name}.captures.jsonl", TRACES_DIR / f"{name}.run.json"]:
        if not f.is_file():
            continue
        txt = f.read_text(encoding="utf-8")
        orig = txt
        for prefix in prefixes:
            txt = txt.replace(prefix, "")
        if txt != orig:
            f.write_text(txt, encoding="utf-8", newline="\n")


class ScenarioLock:
    """串行锁（t24 事故）：同一时刻只允许一个 Mesen 场景运行。

    Unix=fcntl.flock；Windows=msvcrt.locking 锁文件首字节（空文件先垫 1 行）。
    两侧语义一致：非阻塞获取失败 ⇒ 已有场景在跑。"""

    def __init__(self, path: Path):
        self.path = path
        self.f = None

    def acquire(self) -> None:
        self.f = open(self.path, "a+")
        try:
            if IS_WINDOWS:
                self.f.seek(0, os.SEEK_END)
                if self.f.tell() == 0:
                    self.f.write("lock\n")
                    self.f.flush()
                self.f.seek(0)
                msvcrt.locking(self.f.fileno(), msvcrt.LK_NBLCK, 1)
            else:
                fcntl.flock(self.f.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)  # Linux 名（LK_EX/LK_NB=BSD）
        except OSError:
            self.f.close()
            self.f = None
            raise ScenarioError("另一场景运行中（串行纪律）：analysis/.scenario.lock 被占用")

    def release(self) -> None:
        if self.f is None:
            return
        try:
            if IS_WINDOWS:
                self.f.seek(0)
                msvcrt.locking(self.f.fileno(), msvcrt.LK_UNLCK, 1)
            else:
                fcntl.flock(self.f.fileno(), fcntl.LOCK_UN)  # Linux 名（LK_UN=BSD）
        finally:
            self.f.close()
            self.f = None



def mesen_home() -> Path:
    """Mesen2 用户配置目录（【2026-10-27 并发安全件·隔离根治（照 Guardic 模式移植）】
    GUNNAC_MESEN_HOME > ~/.config/Mesen2-metroid（**独立 HOME**——配置零共享：
    单实例劫持=共享配置域互扰根因（mesen.lock 隔离（不同配置域（不同锁）））
    + RecentGames/<rom>.rgd resume 状态跨 run/跨项目污染源。
    首用迁移：隔离目录 settings.json 缺→共享域 ~/.config/Mesen2 拷入（确定性键随迁）。

    POSIX 下 Mesen 配置解析=$HOME/.config/Mesen2（XDG）；隔离域=sibling（Mesen2-metroid），
    由 mesen_isolated_home() fake HOME 注入子进程 env 使 Mesen 用隔离配置域。
    参考：/home/peng/workspace/GuardicGaidenRemake/tools/scenario_run.py（mesen_home（563-582 行））
    """
    isolated = Path.home() / ".config" / "Mesen2-metroid"
    shared = Path.home() / ".config" / "Mesen2"
    # 首用迁移（dated）：隔离域 settings 缺→共享域拷入（确定性键随迁）
    if not isolated.exists() and shared.exists():
        try:
            isolated.parent.mkdir(parents=True, exist_ok=True)
            shutil.copytree(shared, isolated)
        except OSError:
            pass
    return Path(os.environ.get("GUNNAC_MESEN_HOME", str(isolated)))


def clear_mesen_resume_state(rom: Path, xdg_config: Path | None = None) -> list:
    """清除 Mesen2 的 recent-game resume 状态（跨 run/跨项目污染源）。

    Mesen2 GUI 退出时把 resume state 内嵌写进 `RecentGames/<rom>.rgd`（内含 Savestate.mss），
    下次启动同一 ROM 可能自动恢复该状态 ⇒ trace 第二跑/跨模式 sha 分歧（Guardic task-133
    实测：title-menu run1≠run2；删除 .rgd 后恢复一致）。采集前显式删除，不碰 settings.json；
    返回被删除文件（报告留痕）。删除失败 fail-closed 抛错（不许静默放过污染源）。
    参考：GuardicGaidenRemake/tools/scenario_run.py（clear_mesen_resume_state（585-600 行））
    """
    home = (xdg_config / "Mesen2") if xdg_config is not None else mesen_home()
    removed = []
    for pat in (home / "RecentGames" / f"{rom.stem}.rgd",):
        if pat.is_file():
            try:
                pat.unlink()
            except OSError as exc:
                raise ScenarioError(
                    f"resume 状态清理失败（fail-closed，防 auto-resume 跨 run 污染）：{pat}：{exc}")
            removed.append(str(pat))
    return removed


def mesen_isolated_home() -> Path:
    """fake HOME（【2026-10-27 隔离根治】把隔离配置域注入 Mesen 子进程 env 的机制）。

    Mesen2 POSIX 配置解析=$HOME/.config/Mesen2（XDG）——共享域 ~/.config/Mesen2=跨项目
    污染源（mesen.lock 单实例劫持+RecentGames resume 污染）。隔离机制=设 HOME=<fake>，
    使 <fake>/.config/Mesen2→symlink→mesen_home()（隔离域 Mesen2-metroid）——Mesen 的
    $HOME/.config/Mesen2 解析落到隔离域（配置零共享（与 Guardic Mesen2-ggr/其他项目互不干扰））。
    """
    fake = Path.home() / ".config" / "Mesen2-metroid-home"
    link = fake / ".config" / "Mesen2"
    isolated = mesen_home()
    fake.mkdir(parents=True, exist_ok=True)
    (fake / ".config").mkdir(parents=True, exist_ok=True)
    if not link.is_symlink() and not link.exists():
        try:
            link.symlink_to(isolated)
        except OSError:
            # 【WinError 1314 回退】客户侧无符号链接特权（DSH 沙箱实证）——隔离径
            # 退化为独立空配置目录（Mesen 自建配置）——隔离性保持（跨项目零共享），
            # 仅失配置持久化（可接受——采集产物才是契约面）。
            link.mkdir(parents=True, exist_ok=True)
    return fake


# ---------------------------------------------------------------- 每 run 环境域（2026-11-24 #26）
RUNS_ROOT = ROOT / "analysis" / ".mesen-runs"


def run_id_of(name: str) -> str:
    """本 run 的唯一 id（场景名 + 驱动 pid）——并发两实例各自独立，互不覆盖。"""
    return f"{name}-{os.getpid()}"


def prepare_run_dirs(run_id: str) -> dict:
    """为本 run 建独立 HOME / XDG_CONFIG_HOME / XDG_CACHE_HOME / TMPDIR。

    三个共享面（各自踩踏的根因）：
    - `$XDG_CONFIG_HOME/Mesen2/mesen.lock`：Mesen2 的单实例锁按配置域落盘——共享配置域 =
      后启进程把参数转发给先启进程后 exit_group(0)（rc=0、无产物），即「静默退出」现象；
    - `/tmp/CoreFxPipe_<固定 GUID>`：Mono 命名管道按 TMPDIR 落盘——同 TMPDIR = 同一根管道
      （后连者 `Pipe hasn't been connected yet` → InvalidOperationException → 退出）；
    - RecentGames/<rom>.rgd：resume 快照跨 run 污染。

    settings.json 从模板域（mesen_home()）拷入：空配置域会弹「initial setup」向导并**阻塞**
    （实测：Mesen 窗口 400x515 停在 Configuration 对话框，0 帧、无日志）。
    """
    root = RUNS_ROOT / run_id
    home = root / "home"
    xdg = root / "xdg-config"
    cache = root / "xdg-cache"
    tmp = root / "tmp"
    for d in (home, xdg / "Mesen2", cache, tmp):
        d.mkdir(parents=True, exist_ok=True)
    tpl = mesen_home() / "settings.json"
    dst = xdg / "Mesen2" / "settings.json"
    if tpl.is_file() and not dst.exists():
        shutil.copy2(tpl, dst)
    return {"run_id": run_id, "root": root, "home": home, "xdg_config": xdg,
            "xdg_cache": cache, "tmp": tmp, "template": mesen_home()}


def inject_run_env(env: dict, dirs: dict) -> dict:
    """把 per-run 环境注入子进程（POSIX 与 Windows 同口径；Windows 用 TEMP/TMP）。"""
    env["HOME"] = str(dirs["home"])
    env["XDG_CONFIG_HOME"] = str(dirs["xdg_config"])
    env["XDG_CACHE_HOME"] = str(dirs["xdg_cache"])
    env["XDG_DATA_HOME"] = str(dirs["home"] / ".local" / "share")
    env["TMPDIR"] = str(dirs["tmp"])
    env["TMP"] = str(dirs["tmp"])
    env["TEMP"] = str(dirs["tmp"])
    # 自有标记：孤儿清扫只认同一 run id 的进程（不误杀并发实例/他人 worktree）
    env["MESEN_RUN_ID"] = str(dirs["run_id"])
    return env


def _runtime_version() -> str:
    """运行时版本（bad_cast 这类平台故障的第一手信息）：mono 优先，其次 dotnet。"""
    for cmd in (["mono", "--version"], ["dotnet", "--list-runtimes"]):
        try:
            r = subprocess.run(cmd, capture_output=True, text=True, timeout=10)
        except (OSError, subprocess.SubprocessError):
            continue
        out = (r.stdout or "").strip()
        if r.returncode == 0 and out:
            return out.splitlines()[0]
    return "未找到 mono/dotnet"


def _gl_info() -> str:
    """显示/GL 渲染器（平台故障第二手信息）；无 glxinfo 时退化为 libGL 存在性。"""
    disp = os.environ.get("DISPLAY") or "（未设置）"
    try:
        r = subprocess.run(["glxinfo", "-B"], capture_output=True, text=True, timeout=10)
        line = next((ln.strip() for ln in (r.stdout or "").splitlines()
                     if "OpenGL renderer" in ln), "")
        if line:
            return f"DISPLAY={disp}；{line}"
    except (OSError, subprocess.SubprocessError):
        pass
    libs = sorted(Path("/usr/lib/x86_64-linux-gnu").glob("libGL.so*"))
    tail = f"libGL={libs[0].name}" if libs else "libGL 未找到"
    return f"DISPLAY={disp}；（无 glxinfo）{tail}"


def run_diag(dirs: dict, log_path: Path, rc, timed_out: bool) -> str:
    """失败路径诊断块（不再 rc=0 静默）：退出码 / 配置域 / 管道 / 日志尾。"""
    log_tail = ""
    try:
        lines = log_path.read_text(encoding="utf-8", errors="replace").splitlines()
        log_tail = "\n".join("    " + ln for ln in lines[-8:])
    except OSError:
        pass
    pipes = sorted(p.name for p in dirs["tmp"].glob("CoreFxPipe_*"))
    hint = ""
    if "PassArgumentsToFirstInstance" in log_tail or "Pipe hasn't been connected" in log_tail:
        hint = ("\n    诊断：命中 Mesen 单实例转发路径（配置域/管道未隔离）——"
                "检查 XDG_CONFIG_HOME/TMPDIR 是否按 run 分开")
    elif rc == 0 and not timed_out:
        hint = "\n    诊断：rc=0 但无 DONE = 静默退出（多半是单实例握手/启动期崩溃，看日志尾）"
    return (f"  [诊断] rc={rc} timed_out={timed_out}\n"
            f"    运行时: {_runtime_version()}\n"
            f"    显示/GL: {_gl_info()}\n"
            f"    XDG_CONFIG_HOME={dirs['xdg_config']}\n"
            f"    TMPDIR={dirs['tmp']}\n"
            f"    Mesen 管道: {pipes or '（未创建）'}\n"
            f"    日志尾:\n{log_tail or '    （空）'}{hint}")


def kill_orphan_mesen(mesen: Path, run_id: str) -> int:
    """t54 F3 根因防护：清扫驱动锁罩不到的孤儿 Mesen（驱动崩溃/被强杀后残留，
    与后续运行争写同一批产物）。返回清除数。

    Unix：pgrep -f + SIGKILL（含 rounds/r2 F2 孤儿 Xvfb——匹配本驱动特征性几何参数；
    同参数他用 Xvfb 为已知附带风险）。
    Windows：PowerShell CIM 按命令行过滤（Mesen 进程名 + mesen2/自定义路径特征）
    + taskkill /T /F；自定义 GUNNAC_MESEN 路径若不含 mesen2 特征则靠精确路径匹配，
    过宽匹配的附带风险与 Unix 侧 pgrep 同级。"""
    if IS_WINDOWS:
        pat = str(mesen).replace("'", "''")
        script = (
            "Get-CimInstance Win32_Process | Where-Object { $_.Name -like 'Mesen*' -and "
            "($_.CommandLine -like '*mesen2*' -or $_.CommandLine -like '*" + pat + "*') } | "
            "Select-Object -ExpandProperty ProcessId"
        )
        out = subprocess.run(["powershell", "-NoProfile", "-Command", script],
                             capture_output=True, text=True,
                             encoding="utf-8", errors="replace")
        pids = [t for t in (out.stdout or "").split() if t.isdigit()]
        for pid in pids:
            subprocess.run(["taskkill", "/PID", pid, "/T", "/F"],
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        return len(pids)
    pids = subprocess.run(["pgrep", "-f", "tools/mesen2/Mesen"],
                          capture_output=True, text=True,
                          encoding="utf-8", errors="replace").stdout.split()
    # rounds/r2 F2：孤儿 Xvfb 同样清扫（匹配本驱动特征性几何参数；同参数他用 Xvfb 为已知附带风险）
    pids += subprocess.run(
        ["pgrep", "-f", r"Xvfb :[0-9]+ -screen 0 1280x1024x24 -nolisten tcp"],
        capture_output=True, text=True, encoding="utf-8", errors="replace").stdout.split()

    # 【F5 整改（2026-11-12）】worktree 隔离：pgrep -f 无差别匹配会误杀其它 worktree
    # （git worktree 各有独立 tools/mesen2/Mesen 绝对路径 cmdline；Xvfb 靠 cwd 归属）——
    # r9 实测：复核 worktree /tmp/verify-r9 的 Mesen 被主树清扫 SIGKILL（互杀产品=
    # blocked 4/flaky 10 假象）。只杀 cmdline 含本 worktree 根（Mesen）或 cwd 在本
    # worktree（Xvfb）的进程；/proc 不可读时保守跳过（宁可漏杀不误杀）。
    # 【2026-11-24 #26 并发改造】按 MESEN_RUN_ID 归属清扫，不再按 worktree 根：
    # 同 worktree 的并发实例 cmdline 完全一样，按 root 匹配会互杀；按 run id 只清
    # 本 run 自己的残留（env 是进程私有，不会被别的实例继承）。
    marker = f"MESEN_RUN_ID={run_id}".encode()
    scoped = []
    for pid in pids:
        try:
            envb = open(f"/proc/{pid}/environ", "rb").read()
            cl = open(f"/proc/{pid}/cmdline", "rb").read().decode("utf-8", "replace")
            cwd = os.path.realpath(f"/proc/{pid}/cwd")
        except OSError:
            continue  # 进程已退或 /proc 不可用——跳过，不杀
        if marker in envb and (str(ROOT) in cl or cwd.startswith(str(ROOT))):
            scoped.append(pid)
    n = 0
    for pid in scoped:
        try:
            os.kill(int(pid), SIGKILL)
            n += 1
        except (ProcessLookupError, ValueError):
            pass
    return n


def run_scenario(name_or_path: str, timeout_s: float = 1800.0, keep_gen: bool = True) -> dict:
    spath = scenario_path_of(name_or_path)
    plan = parse_scenario(spath)
    name = plan["name"]
    mesen = find_mesen()
    if not mesen.is_file():
        raise ScenarioError(f"Mesen2 未就绪: {mesen}（BYO 工具，见 docs/methodology.md；GUNNAC_MESEN 可覆盖）")
    # 串行锁（t24 事故）：同一时刻只允许一个 Mesen 场景运行（Unix flock / Windows msvcrt）
    # 【2026-11-24 #26】锁粒度从「全仓一把」改为「按场景名一把」：同名场景仍串行
    # （archive_existing/产物名相同，必须互斥）；不同场景各写各的 name.* 产物，可并发。
    slug = re.sub(r"[^A-Za-z0-9._-]+", "_", name)
    lock = ScenarioLock(ROOT / "analysis" / f".scenario-{slug}.lock")
    lock.acquire()
    # t54 F3 根因防护：串行锁只罩驱动进程——驱动崩溃/被强杀后 Mesen/Xvfb 孤儿仍在跑，
    # 与后续运行争显示/写同一批产物（实测 trace 交错、DONE 不落、30x 变慢）。启动前显式清扫。
    run_id = run_id_of(name)
    dirs = prepare_run_dirs(run_id)
    n_stale = kill_orphan_mesen(mesen, run_id)
    if n_stale:
        print(f"  [清场] 清除孤儿 Mesen/Xvfb {n_stale} 个（驱动侧锁覆盖不到的残留）")
    archived = archive_existing(name)
    (TRACES_DIR).mkdir(parents=True, exist_ok=True)
    (SHOTS_DIR).mkdir(parents=True, exist_ok=True)
    GEN_DIR.mkdir(parents=True, exist_ok=True)
    lua_src, lua_sha = generate_lua(plan, spath)
    lua_path = GEN_DIR / f"{name}.lua"
    # newline="\n"：生成物字节面跨平台一致（lua_sha 按 LF 计，禁 CRLF 翻译）
    _lint_lua_text(lua_src, str(lua_path))  # task-38 ③：禁用面机械防（挂死面拒跑）
    lua_path.write_text(lua_src, encoding="utf-8", newline="\n")

    # t54：观测/基线设置全部在 Mesen 启动前完成（此段崩溃不得孤儿化模拟器）
    cap_frames = [int(c["frame"]) for c in plan.get("captures", []) if "frame" in c]
    every = plan.get("capture_every")
    if isinstance(every, dict) and every.get("n"):
        cap_frames.append(int(every["n"]))
    first_sample_frame = min(cap_frames) if cap_frames else 60
    history = historical_first_sample_s(name, first_sample_frame)
    median_s = sorted(history)[len(history) // 2] if history else None
    # 地板 15s 吸收 Mesen 启动开销（比例模型不含启动段；真 first_sample_s 样本入史后中位数自校准）
    warn_at = max((3.0 * median_s) if median_s else 120.0, 15.0)
    first_sample_s: float | None = None
    next_progress = 60.0
    probe_globs = [
        TRACES_DIR / f"{name}_f*.ram.hex",
        SHOTS_DIR / f"{name}_f*.png",
    ]

    done_path = TRACES_DIR / f"{name}.done"
    trace_path = TRACES_DIR / f"{name}.frames.jsonl"
    if done_path.exists():
        done_path.unlink()

    # 【2026-10-27 隔离根治】跨 run/跨项目 resume 污染防护：删 Mesen2 recent-game
    # resume 状态（RecentGames/<rom>.rgd（内含 Savestate.mss（auto-resume 污染源）））。
    removed_states = clear_mesen_resume_state(ROM, dirs["xdg_config"])
    if removed_states:
        print(f"  [清场] 删除 Mesen2 resume 状态: {', '.join(Path(p).name for p in removed_states)}")

    t0 = time.time()
    mesen_log = open(TRACES_DIR / f"{name}.mesen.log", "w", encoding="utf-8")
    # 自管 Xvfb（t24 实测：xvfb-run 包装层偶发提前退出 → Xvfb 随之死 → Mesen 冻在中途帧、
    # DONE 不落）。直接拉 Xvfb + DISPLAY 注入，Mesen 为直接子进程；进程组清理见 finally。
    # 场景仍必须**串行**运行（run-all 已是串行；勿并行多个 Mesen）。
    display = None if IS_WINDOWS else pick_display()
    xvfb: subprocess.Popen | None = None
    proc: subprocess.Popen | None = None
    timed_out = False
    dead_since: float | None = None
    try:
        # rounds/r2 F2：Xvfb/Mesen 创建全部收进 try，任一启动失败不泄漏 Xvfb。
        # Windows：--testRunner headless（Mesen2 2.1.1 TestRunner.cs 语义，2026-10-14 实测）：
        #   无窗口（ConsoleMode，UI 不启动）、Lua 自动执行、emu.stop 后进程自退（退出码=stop code）、
        #   自动 DisableSaveSettings；--debug.scriptWindow.allowIoOsAccess=true 解锁脚本 io
        #   （默认沙箱禁写）；Mesen 启动后把 CWD 切到 HomeFolder ⇒ Lua 产物必须绝对路径（PLAN 已是）。
        #   /DoNotSaveSettings、Xvfb/DISPLAY 仅 GUI 模式需要（Unix 侧沿用已验证路径）。
        env = dict(os.environ)
        # 【2026-11-24 #26】per-run 四个域全隔离（配置域/缓存/TMPDIR/标记）：
        # 配置域决定 mesen.lock（单实例），TMPDIR 决定 CoreFxPipe（命名管道）。
        # 旧实现只把 HOME 指向 fake home，但 Mesa/Mono 用的是 XDG_CONFIG_HOME——
        # 宿主的 XDG_CONFIG_HOME 一继承，两个实例仍然共享 ~/.config/Mesen2/mesen.lock。
        env = inject_run_env(env, dirs)
        env["GUNNAC_MESEN_HOME"] = str(dirs["template"])
        if IS_WINDOWS:
            proc = subprocess.Popen(
                [str(mesen), "--testRunner", str(lua_path), str(ROM),
                 f"--timeout={int(timeout_s)}", "--enableStdout",
                 "--debug.scriptWindow.allowIoOsAccess=true"],
                cwd=ROOT, stdout=mesen_log, stderr=subprocess.STDOUT, text=True, env=env,
            )
        else:
            xvfb = subprocess.Popen(
                ["Xvfb", display, "-screen", "0", "1280x1024x24", "-nolisten", "tcp"],
                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, start_new_session=True,
                env=env,
            )
            wait_for_x(display, xvfb)
            env["DISPLAY"] = display
            proc = subprocess.Popen(
                [str(mesen), str(ROM), str(lua_path), "/DoNotSaveSettings"],
                cwd=ROOT, stdout=mesen_log, stderr=subprocess.STDOUT, text=True,
                env=env, start_new_session=True,
            )
        while not done_path.exists():
            now = time.time()
            el = now - t0
            if now - t0 > timeout_s:
                timed_out = True
                break
            if proc.poll() is not None:
                # 包装层退出 ≠ 采集完成（孤儿 Mesen 可能仍在写盘）；给 20s 宽限等 DONE
                if dead_since is None:
                    dead_since = now
                elif now - dead_since > 20:
                    break
            if first_sample_s is None:
                if any(any(g.parent.glob(g.name)) for g in probe_globs):
                    first_sample_s = round(el, 1)
                    print(f"  首采样帧（~f{first_sample_frame}）耗时 {first_sample_s}s"
                          f"（历史中位 {('%.1f' % median_s) if median_s else '无'}s）")
                elif el > next_progress:
                    print(f"  [时长告警] 已 {el:.0f}s 未见首采样产物（基线 {warn_at:.0f}s）"
                          "——疑似挂起/负载异常，检查 Mesen/Xvfb/系统负载", flush=True)
                    next_progress = el + 60.0
            time.sleep(0.5)
    finally:
        def kill_group(sig) -> None:
            if proc is None:
                return
            if IS_WINDOWS:
                # Windows 无进程组/SIGKILL 语义：taskkill /T /F = 整树 TerminateProcess
                subprocess.run(["taskkill", "/PID", str(proc.pid), "/T", "/F"],
                               stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                return
            try:
                os.killpg(os.getpgid(proc.pid), sig)
            except (ProcessLookupError, PermissionError, OSError):
                pass

        if proc is not None and (proc.poll() is None or not done_path.exists()):
            kill_group(signal.SIGTERM)
        if proc is not None:
            try:
                proc.wait(timeout=10)
            except subprocess.TimeoutExpired:
                kill_group(SIGKILL)
                proc.wait(timeout=10)
        mesen_log.close()
        if xvfb is not None:
            try:
                xvfb.terminate()
                xvfb.wait(timeout=5)
            except Exception:
                try:
                    xvfb.kill()
                except Exception:
                    pass
        # 【批内逐案资源自清（2026-11-02（裁定触发件（最薄实现）））】每案毕释放本
        # run 自有 Xvfb 的 socket/lock——**归属判**（本 run Popen 子进程（wait/kill 后
        # 进程已死=归属死判（非裸 PID 启发（liveness 精判约束不变（裁定③））））——
        # 批跑（run-all 12 case×Xvfb）socket 累积⇒后段全占/连死显示⇒采点缺（blocked
        # 波动）根因面。
        if display and not IS_WINDOWS:
            n = display.lstrip(":")
            for stale in (Path(f"/tmp/.X{n}-lock"), Path(f"/tmp/.X11-unix/X{n}")):
                try:
                    stale.unlink()
                except OSError:
                    pass
    wall = round(time.time() - t0, 1)

    done_info = None
    if done_path.exists():
        done_info = json.loads(done_path.read_text(encoding="utf-8").strip().splitlines()[-1])
    n_lines = 0
    if trace_path.exists():
        with open(trace_path, encoding="utf-8") as f:
            n_lines = sum(1 for _ in f)
    shots = sorted(p.relative_to(ROOT).as_posix() for p in (SHOTS_DIR).glob(f"{name}_f*.png"))
    rams = sorted(p.relative_to(ROOT).as_posix() for p in (TRACES_DIR).glob(f"{name}_f*.ram.hex"))

    manifest = {
        "scenario": relp(spath),
        "scenario_sha256": hashlib.sha256(spath.read_bytes()).hexdigest(),
        "generated_lua": lua_path.relative_to(ROOT).as_posix() if keep_gen else None,
        "generated_lua_sha256": lua_sha,
        "frames": plan["frames"],
        "sample_point": plan.get("sample_point") or "endFrame",
        "mesen_run": {"run_id": run_id, "xdg_config": str(dirs["xdg_config"].relative_to(ROOT)),
                      "tmp": str(dirs["tmp"].relative_to(ROOT))},
        "fixtures": plan.get("fixtures") or [],
        "bundle_capture": plan.get("bundle_capture"),
        "bundle_load": plan.get("bundle_load"),
        "trace_lines": n_lines,
        "done": done_info,
        "timed_out": timed_out,
        "wall_s": wall,
        "first_sample_s": first_sample_s,
        "duration_warning": (
            f"首采样 {first_sample_s}s 超历史中位 {median_s:.1f}s 的 3 倍——疑似挂起/负载异常"
            if first_sample_s is not None and median_s is not None and first_sample_s > 3.0 * median_s
            else None
        ),
        "outputs": {
            "trace": trace_path.relative_to(ROOT).as_posix(),
            "captures": f"analysis/traces/{name}.captures.jsonl",
            "shots": shots,
            "ram_dumps": rams,
        },
    }
    mpath = TRACES_DIR / f"{name}.run.json"
    mpath.write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")

    lock.release()
    if archived:
        print(f"  已归档旧产物: {archived}（run 隔离，防踩踏）")
    ok = done_info is not None and not timed_out and n_lines > 0
    print(f"== scenario run: {name} ==")
    print(f"  DSL: {manifest['scenario']} sha256={manifest['scenario_sha256'][:16]}…")
    normalize_paths_in_outputs(name)
    print(f"  frames: {plan['frames']} trace_lines: {n_lines} captures: {len(rams)} shots: {len(shots)} wall: {wall}s")
    if done_info:
        print(f"  DONE: {json.dumps(done_info, ensure_ascii=False)}")
    print(f"  trace: {manifest['outputs']['trace']}")
    print(f"  run清单: {mpath.relative_to(ROOT).as_posix()}")
    if not ok:
        rc = proc.returncode if proc is not None else None
        print(f"  !! 未完成（timed_out={timed_out}, done={done_info is not None}, trace_lines={n_lines}, rc={rc}）")
        print(run_diag(dirs, TRACES_DIR / f"{name}.mesen.log", rc, timed_out))
    return manifest


def cmd_list(_args) -> int:
    rows = []
    for p in sorted(SCENARIO_DIR.glob("*.txt")):
        try:
            plan = parse_scenario(p)
            rows.append((plan["name"], plan["frames"], len(plan["presses"]), len(plan["holds"]), len(plan["captures"])))
        except ScenarioError as exc:
            rows.append((p.name, "解析失败", str(exc), "", ""))
    print(f"{'场景':<24} {'帧数':>6} {'press':>6} {'hold':>5} {'capture':>8}")
    for r in rows:
        print(f"{r[0]:<24} {str(r[1]):>6} {str(r[2]):>6} {str(r[3]):>5} {str(r[4]):>8}")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(prog="tools/scenario_run.py", description="S2 oracle 场景采集驱动（DSL 唯一解析器）")
    sub = ap.add_subparsers(dest="cmd", required=True)
    sub.add_parser("list", help="列出 oracle/scenarios/ 场景")
    p_gen = sub.add_parser("gen", help="只生成 lua（调试）")
    p_gen.add_argument("scenario")
    p_run = sub.add_parser("run", help="运行单个场景")
    p_run.add_argument("scenario")
    p_run.add_argument("--timeout", type=float, default=1800.0)
    p_all = sub.add_parser("run-all", help="顺序运行全部场景")
    p_all.add_argument("--timeout", type=float, default=1800.0)
    args = ap.parse_args()

    try:
        if args.cmd == "list":
            return cmd_list(args)
        if args.cmd == "gen":
            spath = scenario_path_of(args.scenario)
            plan = parse_scenario(spath)
            GEN_DIR.mkdir(parents=True, exist_ok=True)
            src, sha = generate_lua(plan, spath)
            out = GEN_DIR / f"{plan['name']}.lua"
            out.write_text(src, encoding="utf-8", newline="\n")
            print(f"生成 {out.relative_to(ROOT)} sha256={sha}")
            return 0
        if args.cmd == "run":
            run_scenario(args.scenario, args.timeout)
            return 0
        if args.cmd == "run-all":
            failed = []
            for p in sorted(SCENARIO_DIR.glob("*.txt")):
                m = run_scenario(p, args.timeout)
                if not (m["done"] and not m["timed_out"]):
                    failed.append(m["scenario"])
            if failed:
                print(f"RUN-ALL: 未完成 {failed}")
                return 1
            print("RUN-ALL: 全部场景完成")
            return 0
    except ScenarioError as exc:
        print(f"场景错误: {exc}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
