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
PRESS_LEN_DEFAULT = 2


class ScenarioError(Exception):
    """DSL 解析/校验错误（带 文件:行号）。"""


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

def parse_scenario(path: Path) -> dict:
    """解析场景 DSL → PLAN dict。所有语义判定唯一入口。"""
    path = Path(path)
    plan: dict = {
        "name": path.stem,
        "frames": None,
        "press_len": PRESS_LEN_DEFAULT,
        "presses": [],
        "holds": [],
        "captures": [],
        "capture_every": None,
        "watch": [],
        "sample_point": None,  # task-24 3b: spin-exit | 缺省 endFrame（混态期 legacy）
        "fixtures": [],  # task-28 ①: {frame, addr, value, persistence}——F_a 注入（设计 v9/v11）
        "bundle_capture": None,  # task-32 收官: {frame, out}——MSS+三直映采集
        "bundle_load": None,     # task-32 收官: {path}——任意起点装载
        "watch_ranges": [],
        "ramframe": False,
        "coverage": False,
        "coverage_pc": [],
        "probe_mem": [],
        "probe_dispatch": [],
        "entry_probe": [],
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
            if len(args) != 1 or not args[0].isdigit() or int(args[0]) <= 0:
                raise err("frames 需要正整数")
            plan["frames"] = int(args[0])
        elif op == "press_len":
            if len(args) != 1 or not args[0].isdigit() or int(args[0]) <= 0:
                raise err("press_len 需要正整数")
            plan["press_len"] = int(args[0])
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
        elif op == "fixture":
            # task-28 ①：夹具状态设置（state-fixture 设计 §3 F_a 注入模型）——
            # fixture <帧号> <hex地址> <hex值> [invariant|evolving]
            # （persistence 语义：invariant=窗口末读回断言；evolving=演化观测面）
            if len(args) not in (3, 4):
                raise err("fixture 用法: fixture <帧号> <hex地址> <hex值> [invariant|evolving]")
            if not args[0].isdigit():
                raise err(f"fixture 帧号须为非负整数: {args[0]!r}")
            for a in args[1:3]:
                if len(a) not in (2, 4) or any(c not in "0123456789abcdef" for c in a.lower()):
                    raise err(f"fixture 地址/值须为 hex（2/4 位地址、2 位值）: {a!r}")
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
        elif op == "bundle_load":
            # task-32 收官：任意起点装载——bundle_load <前缀>（首 spin 出口装载）
            if len(args) != 1:
                raise err("bundle_load 用法: bundle_load <前缀>")
            plan["bundle_load"] = {"path": args[0]}
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
            plan["watch_ranges"].append({"from": a_s, "to": b_s, "label": label})
        elif op == "ramframe":
            if len(args) != 1 or args[0] not in ("on", "off"):
                raise err("ramframe 用法: ramframe on|off")
            plan["ramframe"] = args[0] == "on"
        elif op == "coverage":
            # 覆盖采集（缺口左移机制③）：execute 回调记录执行过的地址 → <name>.coverage.jsonl
            if len(args) > 1 or (args and args[0] not in ("on", "off")):
                raise err("coverage 用法: coverage [on|off]")
            plan["coverage"] = (args[0] if args else "on") != "off"
        else:
            raise err(f"未知 DSL 指令 {op!r}")

    if plan["frames"] is None:
        raise ScenarioError(f"{path}: 缺 frames 指令")
    frames = plan["frames"]
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
    return plan


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
        return "{" + ",".join(f"[{lua_str(k)}]={to_lua(val)}" for k, val in v.items()) + "}"
    raise TypeError(f"不可序列化为 Lua: {v!r}")


def _lint_lua_text(text: str, label: str) -> None:
    """task-38 ③：禁用面机械防（与 tools/lint_mesen_lua.py 同源清单）。"""
    import re as _re
    for pat in (r"emu\.createSavestate\s*\(\s*\)", r"emu\.loadSavestate\s*\(\s*\)"):
        if _re.search(pat, text):
            raise SystemExit(f"[task-38 lint] {label}: 禁用面命中 {pat}——挂死面拒跑")


def generate_lua(plan: dict, scenario_path: Path) -> tuple[str, str]:
    """返回 (lua 源码, lua sha256)。PLAN 前置 + 执行器整拼（无 dofile 依赖）。"""
    # None 字段（缺省 DSL 项如 capture_every）不进 Lua 表（r4 工程卫生：缺省场景必须可跑）
    plan_lua = {k: v for k, v in plan.items() if v is not None}
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


def kill_orphan_mesen(mesen: Path) -> int:
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
    n = 0
    for pid in pids:
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
    lock = ScenarioLock(ROOT / "analysis" / ".scenario.lock")
    lock.acquire()
    # t54 F3 根因防护：串行锁只罩驱动进程——驱动崩溃/被强杀后 Mesen/Xvfb 孤儿仍在跑，
    # 与后续运行争显示/写同一批产物（实测 trace 交错、DONE 不落、30x 变慢）。启动前显式清扫。
    n_stale = kill_orphan_mesen(mesen)
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
        print(f"  !! 未完成（timed_out={timed_out}, done={done_info is not None}, trace_lines={n_lines}）")
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
