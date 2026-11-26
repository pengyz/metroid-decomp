#!/usr/bin/env python3
"""Stage A 通关验收面：节点判定表（静态链路 + 动态到达表）。

两个面，各自独立可跑，都不改代码：

1. `links`：静态校验「进度链的每条连接点」在 Rust 侧是否真的存在——对每个连接点
   声明它必须在哪个文件里出现哪几个模式（写点/读点/置位点）。缺一个就判**断链**并
   报 `file:line`。这把 docs/spec/game-progress.md §4 的差异表变成机械可复算的检查。

2. `trace`：拿一份逐帧状态轨迹（`{f, w:{标签:值}}` JSONL，生产者可以是
   `tools/scenario_run.py`（Mesen watch）或任何等价导出），按节点序求「首次满足帧」，
   输出到达表；第一个未到达的节点即**第一个断点**。

3. `report`：把两面合成一句判决「当前能通关到哪一步」。

设计取舍（详见 docs/design/stage-a-gate.md）：
- 本工具**不是**第四个回放器：输入格式与 `tools/scenario_run.py` / `tools/tas_replay_diff.py`
  的逐帧产物对齐，只做「节点判定」这一层；按键序列/仿真由既有管线负责。
- 节点谓词用受限表达式（只允许寄存器名、整数、位运算），不引入图灵完备的规则引擎。
- 静态面只认「模式是否出现在声明文件内」，不判断语义正确；语义正确仍由对拍/单测裁定。

用法：
    python3 tools/stage_a_nodes.py links
    python3 tools/stage_a_nodes.py trace --trace analysis/traces/x.frames.jsonl
    python3 tools/stage_a_nodes.py trace --trace x.jsonl --json
    python3 tools/stage_a_nodes.py report [--trace x.jsonl]
    python3 tools/stage_a_nodes.py selftest
    python3 tools/stage_a_nodes.py nodes          # 打印节点表（供文档/复核引用）

退出码：0=全部通过；1=存在断链或存在未到达节点；2=输入/用法错误。
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# ---------------------------------------------------------------------------
# 节点表（顺序 = 必经顺序；cited = 证据指针）
# confidence: A=ROM 字节/汇编逐行可复现；B=代码语境推断
# pred: 受限表达式；可用名见 WATCH_KEYS
# require_seen: 先决条件（该键必须先出现过满足条件的值，避免「一直是 0」被误判为达成）
# ---------------------------------------------------------------------------
WATCH_KEYS = {
    "kraid_status": "$687B KraidStatueStatus（击杀 bit0 / 升起 bit7）",
    "ridley_status": "$687C RidleyStatueStatus（击杀 bit1 / 升起 bit7）",
    "bridge": "$0054 StatuesBridgeIsSpawned",
    "in_area": "$0074 InArea（低 4 位=区号，3=Tourian）",
    "zeb0": "$0758+0 Zebetites[0].status",
    "zeb1": "$0758+8 Zebetites[1].status",
    "zeb2": "$0758+16 Zebetites[2].status",
    "zeb3": "$0758+24 Zebetites[3].status",
    "zeb4": "$0758+32 Zebetites[4].status",
    "mb_status": "$0098 MotherBrainStatus",
    "end_timer": "$010A EndTimer",
    "at_ending": "$6883 AtEnding",
    "in_tourian": "$0074 InArea 低 4 位 == 3（派生键：这一帧是否在 Tourian）",
}

NODES = [
    dict(id="S1", title="首个 Boss 击破（在 Boss 区）",
         pred="((kraid_status & 0x01) and (in_area & 0x0F) == 2) or ((ridley_status & 0x02) and (in_area & 0x0F) == 4)",
         cited="docs/spec/game-progress.md §0#1, §1.4（LDD75）；区号 2=Kraid、4=Ridley（crates/game-core/src/world/mainloop.rs 五区表）",
         confidence="A"),
    dict(id="S2", title="两个 Boss 均击破", pred="(kraid_status & 0x01) and (ridley_status & 0x02)",
         cited="docs/spec/game-progress.md §0#1（须在 S1 之后，故至少一次是真击杀）", confidence="A"),
    dict(id="S3", title="两座雕像升起（在 Brinstar 雕像房）",
         pred="(kraid_status & 0x80) and (ridley_status & 0x80) and (in_area & 0x0F) == 0",
         cited="docs/spec/game-progress.md §0#4, §1.5（命中才升；区号 0=Brinstar）", confidence="A"),
    dict(id="S4", title="通路桥生成", pred="bridge == 1",
         cited="docs/spec/game-progress.md §0#5, §1.6", confidence="A"),
    dict(id="S5", title="进入 Tourian", pred="(in_area & 0x0F) == 3",
         cited="docs/spec/game-progress.md §0#6, §2", confidence="A"),
    dict(id="S6", title="Zebetite 清空", pred="zeb0 == 0 and zeb1 == 0 and zeb2 == 0 and zeb3 == 0 and zeb4 == 0",
         require_seen={"in_tourian": "gt0", "zeb0": "gt0", "zeb1": "gt0",
                       "zeb2": "gt0", "zeb3": "gt0", "zeb4": "gt0"},
         cited="docs/spec/game-progress.md §3.3（5 座各 8 击）；区域门：#98 §5（未进 Tourian 时 Zebetite 根本没生成，全 0 不是「已清空」）", confidence="B"),
    dict(id="S7", title="母脑击破", pred="mb_status == 7",
         cited="docs/spec/game-progress.md §3.2（11 态表；7=死亡态待核）", confidence="B"),
    dict(id="S8", title="逃逸计时耗尽", pred="end_timer == 0", require_seen={"end_timer": "gt0"},
         cited="docs/spec/game-progress.md §3.4", confidence="A"),
    dict(id="S9", title="结局触发", pred="at_ending != 0",
         cited="docs/spec/game-progress.md §0#7, §3.5", confidence="A"),
]

# 静态链路：每条声明「该连接点必须在哪个文件的哪几处体现」。
# expect 里任一模式在文件内找不到 ⇒ 断链（报该条第一个缺失模式）。
LINKS = [
    dict(id="L1", title="Boss 击杀结算写雕像状态（prevStatus bit7 → LDD75）",
         file="crates/game-core/src/enemy/framework.rs",
         expect=["0x687B|KRAID_STATUE_STATUS", "0x687C|RIDLEY_STATUE_STATUS"],
         cited="docs/spec/game-progress.md §1.4（07:DD75）", confidence="A"),
    dict(id="L2", title="Boss 房重生门读雕像状态（GetEnemyType bit6 跳生成）",
         file="crates/game-core/src/world/room_draw.rs",
         expect=["0x687B|KRAID_STATUE_STATUS", "0x687C|RIDLEY_STATUE_STATUS"],
         cited="docs/spec/game-progress.md §1.2", confidence="A"),
    dict(id="L3", title="雕像升起（命中计数 + 逐帧上移 + 置 bit7）",
         file="crates/game-core/src/world/statue.rs",
         expect=["0x80", "update_statue_raise"], cited="docs/spec/game-progress.md §1.5", confidence="A"),
    dict(id="L4", title="桥生成（两座雕像 bit7 ⇒ StatuesBridgeIsSpawned）",
         file="crates/game-core/src/world/statue.rs",
         expect=["STATUES_BRIDGE_SPAWNED|0x0054"], cited="docs/spec/game-progress.md §1.6", confidence="A"),
    dict(id="L5", title="结局电梯触发（etype 0x8F ⇒ AtEnding）",
         file="crates/game-core/src/world/elevator.rs",
         expect=["AT_ENDING|0x6883", "0x8F"], cited="docs/spec/game-progress.md §3.5", confidence="A"),
]


# 逐帧状态还可以是「整块内存 hex」形态（`tas_replay_diff.py` 的 Rust 腿产物
# `rust.state.jsonl`：w={zp,iram,wram,pw}）。这里给每个 watch 键声明它在哪块、
# 偏移多少，缺命名键时按此换算——让节点表能同时对 emu 腿与 Rust 腿求值。
WATCH_ADDR = {
    "kraid_status": ("wram", 0x687B - 0x6000),
    "ridley_status": ("wram", 0x687C - 0x6000),
    "bridge": ("zp", 0x0054),
    "in_area": ("zp", 0x0074),
    "mb_status": ("zp", 0x0098),
    # 【iram 是 $0100-$07FF 的 1792 B 视图（不是 $0000-$07FF）】Rust 腿
    # `rust.state.jsonl` 的 `w.iram` 由 `memmap/observe/frame_face.rs` 导出，
    # 起点是 $0100、长度 0x700。因此这里的偏移一律写 `addr - 0x0100`；
    # 写绝对地址会把 $010A 读成 $020A、把 $0758 读成越界（旧实现静默跳过
    # ⇒ zeb0..4 键永不存在 ⇒ S6 永远不可能成立，见 #98 §4 / #99 G6）。
    "end_timer": ("iram", 0x010A - 0x0100),
    "at_ending": ("wram", 0x6883 - 0x6000),
    "zeb0": ("iram", 0x0758 - 0x0100),
    "zeb1": ("iram", 0x0760 - 0x0100),
    "zeb2": ("iram", 0x0768 - 0x0100),
    "zeb3": ("iram", 0x0770 - 0x0100),
    "zeb4": ("iram", 0x0778 - 0x0100),
}

# 每个整块 blob 的 CPU 地址起点（`iram` 不是从 $0000 起）。
BLOB_BASE = {"zp": 0x0000, "iram": 0x0100, "wram": 0x6000, "pw": 0x6988}
BLOB_LEN = {"zp": 0x100, "iram": 0x700, "wram": 0x2000, "pw": 0x2C}

# 派生键：不进 WATCH_ADDR（没有独立地址），由其它键现算。
# in_tourian = (in_area & 0x0F) == 3 —— S6 的区域门用它（#98 §5 假阳性）。
DERIVED_KEYS = {"in_tourian"}


class NodeError(RuntimeError):
    pass


def _decode_blobs(w: dict) -> dict:
    """从整块内存 hex 里取出 watch 键；越界**fail-loud**（不静默跳过）。

    旧实现在 `i + 2 > len(blob)` 时静默 continue，导致 `zeb0..4`（$0758，
    相对 $0100 是 0x658）在唯一的 Rust 轨迹上永远取不到 ⇒ S6 恒不可达。
    现在：块存在但键越界 ⇒ 抛 NodeError，报文带键名、绝对地址、块容量，
    让「谁的生产者少导了一段」立刻可见（#99 G6）。
    """
    out = {}
    for key, (space, off) in WATCH_ADDR.items():
        blob = w.get(space)
        if not isinstance(blob, str):
            continue
        cap = len(blob) // 2
        if off + 1 > cap:
            base = BLOB_BASE.get(space, 0)
            raise NodeError(
                f"watch 键 {key} 越界：地址 ${base + off:04X}"
                f"（{space} 偏移 {off:#06x}）超出块容量 {cap} B"
                f"（{space} 期望 {BLOB_LEN.get(space, cap)} B）——"
                f"检查导出方是否少导了这段内存"
            )
        i = off * 2
        try:
            out[key] = int(blob[i:i + 2], 16)
        except ValueError as e:
            raise NodeError(f"watch 键 {key} 的字节不是合法 hex（{space}[{off}]）：{e}") from e
    return _derive(out)


def _derive(vals: dict) -> dict:
    """派生键：只要 in_area 在，就能算出 in_tourian（0/1）。"""
    if "in_area" in vals:
        vals["in_tourian"] = 1 if (int(vals["in_area"]) & 0x0F) == 3 else 0
    return vals


_ALLOWED = re.compile(r"^[\s0-9a-zA-Z_|&()<>=!+\-]*$")


def eval_pred(expr: str, values: dict) -> bool:
    """受限表达式求值：只允许标识符/整数/位运算/比较/布尔。缺键按 0。"""
    if not _ALLOWED.match(expr):
        raise NodeError(f"谓词含不允许的字符：{expr!r}")
    env = {"__builtins__": {}}
    for name in WATCH_KEYS:
        v = values.get(name, 0)
        env[name] = int(v) if isinstance(v, (int, float)) else 0
    try:
        return bool(eval(expr, env))  # noqa: S307 —— 已字符级白名单 + 只有整数变量
    except Exception as e:  # pragma: no cover - 谓词写错属工具缺陷
        raise NodeError(f"谓词求值失败 {expr!r}: {e}")


def load_trace(path: Path) -> list[dict]:
    out = []
    for idx, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        line = line.strip()
        if not line:
            continue
        try:
            rec = json.loads(line)
        except json.JSONDecodeError as e:
            raise NodeError(f"{path}:{idx} 不是合法 JSON：{e}")
        if "f" not in rec or "w" not in rec:
            raise NodeError(f"{path}:{idx} 缺 f/w 字段（期望 {{f,w}}）")
        out.append(rec)
    if not out:
        raise NodeError(f"{path} 没有任何帧记录")
    return out


def _needed_keys(node: dict) -> set[str]:
    keys = set(re.findall(r"[a-zA-Z_][a-zA-Z0-9_]*", node["pred"])) & set(WATCH_KEYS)
    keys |= set(node.get("require_seen") or {}) & set(WATCH_KEYS)
    return keys


def eval_trace(rows: list[dict]) -> dict:
    """按节点序求首次满足帧；返回 {arrived:[{id,frame}], first_break:{...}|None}。

    判据：一个节点只在**它引用的所有 watch 键都在该帧出现**时才可判定——避免
    「没采集的键默认 0」把未观测误判为已达成（例如 Zebetite 全 0 与根本没 watch）。
    """
    arrived: list[dict] = []
    seen_ok = {k: False for node in NODES for k in (node.get("require_seen") or {})}
    gate_reason = ""
    pending = list(NODES)
    for rec in rows:
        raw = rec["w"]
        vals = {k: int(v) for k, v in raw.items() if isinstance(v, (int, float))}
        for k, v in _decode_blobs(raw).items():
            vals.setdefault(k, v)
        _derive(vals)
        for k in seen_ok:
            if not seen_ok[k] and vals.get(k, 0) > 0:
                seen_ok[k] = True
        while pending:
            node = pending[0]
            if not _needed_keys(node) <= set(vals):
                break
            if not eval_pred(node["pred"], vals):
                break
            req = node.get("require_seen") or {}
            missing = [k for k in req if not seen_ok.get(k)]
            if missing:
                gate_reason = f"先决门未满足：{', '.join(missing)}（要求先出现过满足 {req[missing[0]]} 的值）"
                break
            arrived.append(dict(id=node["id"], title=node["title"], frame=rec["f"]))
            pending.pop(0)
        if not pending:
            break
    first_break = None
    if pending:
        n = pending[0]
        first_break = dict(id=n["id"], title=n["title"], pred=n["pred"],
                           cited=n["cited"], confidence=n["confidence"],
                           gate=gate_reason)
    return dict(frames=len(rows), arrived=arrived, first_break=first_break)


def check_links() -> list[dict]:
    rows = []
    for link in LINKS:
        p = ROOT / link["file"]
        if not p.exists():
            rows.append(dict(**link, ok=False, reason=f"文件不存在：{link['file']}", line=0))
            continue
        text = p.read_text(encoding="utf-8")
        missing = [pat for pat in link["expect"] if not re.search(pat, text)]
        if missing:
            rows.append(dict(**link, ok=False, reason=f"缺模式：{missing[0]}", line=0))
        else:
            m = re.search(link["expect"][0], text)
            line = text[: m.start()].count("\n") + 1 if m else 0
            rows.append(dict(**link, ok=True, reason="", line=line))
    return rows


def cmd_links(args) -> int:
    rows = check_links()
    bad = [r for r in rows if not r["ok"]]
    if args.json:
        print(json.dumps(dict(links=rows, ok=not bad), ensure_ascii=False, indent=1))
    else:
        print(f"[链路] 进度链静态校验：{len(rows)} 条，断链 {len(bad)} 条")
        for r in rows:
            mark = "OK  " if r["ok"] else "断链"
            tail = f"{r['file']}:{r['line']}" if r["ok"] else r["reason"]
            print(f"  [{mark}] {r['id']} {r['title']} —— {tail}")
            if not r["ok"]:
                print(f"          依据 {r['cited']}")
    return 0 if not bad else 1


def cmd_trace(args) -> int:
    path = Path(args.trace)
    if not path.exists():
        print(f"[错误] 轨迹文件不存在：{path}", file=sys.stderr)
        return 2
    res = eval_trace(load_trace(path))
    if args.json:
        print(json.dumps(res, ensure_ascii=False, indent=1))
    else:
        print(f"[轨迹] {path} —— 帧数 {res['frames']}")
        print("  节点          到达帧   说明")
        hit = {a["id"]: a["frame"] for a in res["arrived"]}
        for n in NODES:
            f = hit.get(n["id"])
            print(f"  {n['id']} {n['title']:<12} {('—' if f is None else f):>8}   ({n['confidence']})")
        if res["first_break"]:
            b = res["first_break"]
            print(f"  第一个断点：{b['id']} {b['title']}")
            print(f"    谓词 {b['pred']}")
            if b.get("gate"):
                print(f"    {b['gate']}")
            print(f"    依据 {b['cited']}")
        else:
            print("  全部节点按序到达。")
    return 0 if not res["first_break"] else 1


def cmd_report(args) -> int:
    links = check_links()
    bad = [r for r in links if not r["ok"]]
    res = None
    if args.trace:
        res = eval_trace(load_trace(Path(args.trace)))
    print("== Stage A 通关验收面 ==")
    print(f"静态链路：{len(links)} 条，断链 {len(bad)} 条" + (f"（{', '.join(r['id'] for r in bad)}）" if bad else ""))
    if res:
        hit = {a["id"]: a["frame"] for a in res["arrived"]}
        reached = [n["id"] for n in NODES if n["id"] in hit]
        print(f"动态轨迹：{args.trace} 帧数 {res['frames']}，到达 {len(reached)}/{len(NODES)} 个节点"
              + (f"（{reached[-1]} 为止）" if reached else ""))
        if res["first_break"]:
            b = res["first_break"]
            print(f"第一个断点：{b['id']} {b['title']}（{b['cited']}）")
            if b.get("gate"):
                print(f"  {b['gate']}")
    if bad:
        print("判决：**不通关** —— 静态链路断在 "
              + "、".join(f"{r['id']}（{r['file']}）" for r in bad))
        return 1
    if res and res["first_break"]:
        print(f"判决：本次轨迹未通关 —— 停在 {res['first_break']['id']} {res['first_break']['title']}")
        return 1
    if res:
        print("判决：本次轨迹走完全部节点。")
    return 0


def cmd_nodes(args) -> int:
    print("节点表（顺序=必经顺序）：")
    for n in NODES:
        print(f"  {n['id']} {n['title']}  [{n['confidence']}]  pred: {n['pred']}")
        print(f"      依据 {n['cited']}")
    print("\n静态链路：")
    for l in LINKS:
        print(f"  {l['id']} {l['title']}  [{l['confidence']}]  {l['file']}")
        print(f"      expect: {l['expect']}  依据 {l['cited']}")
    print("\nwatch 键（生产者需按此命名逐帧导出）：")
    for k, v in WATCH_KEYS.items():
        print(f"  {k:<14} {v}")
    return 0


def check_convert(trace_path: Path, coverage_path: Path | None = None) -> dict:
    """换算断言：`iram` 是 $0100 起的 1792 B 视图，$0400 必须落在偏移 0x300。

    返回 `{frame, iram_0x300, cov_rust, ok}`；`ok` 要求
    `iram[$0400-$0100] == coverage.first_divergence.rust`（同一帧同一字节）。
    """
    cov = None
    if coverage_path and coverage_path.exists():
        cov = json.loads(coverage_path.read_text(encoding="utf-8")).get("first_divergence")
    res = dict(frame=None, iram_0x300=None, cov_rust=None, ok=False, iram_0x400=None)
    target = (cov or {}).get("frame")
    for rec in load_trace(trace_path):
        if target is not None and rec["f"] != target:
            continue
        iram = (rec.get("w") or {}).get("iram")
        if not isinstance(iram, str):
            break
        raw = bytes.fromhex(iram)
        res["frame"] = rec["f"]
        res["iram_0x300"] = raw[0x300] if len(raw) > 0x300 else None
        res["iram_0x400"] = raw[0x400] if len(raw) > 0x400 else None
        res["cov_rust"] = (cov or {}).get("rust")
        res["ok"] = res["iram_0x300"] == res["cov_rust"] is not None
        break
    return res


def cmd_verify_convert(args) -> int:
    trace = Path(args.trace)
    if not trace.exists():
        print(f"[错误] 轨迹文件不存在：{trace}", file=sys.stderr)
        return 2
    r = check_convert(trace, Path(args.coverage) if args.coverage else None)
    print(f"[换算] 帧 {r['frame']}：iram[0x0400-0x0100]=iram[0x300]={r['iram_0x300']}，"
          f"coverage.rust={r['cov_rust']}（iram[0x400]={r['iram_0x400']}）")
    if not r["ok"]:
        print("[FAIL] iram 偏移换算与 coverage 不一致（期望 $0400 落在 0x300）", file=sys.stderr)
        return 1
    print("[PASS] iram 偏移换算与 coverage 一致")
    return 0


def cmd_selftest(args) -> int:
    fails = []

    def ok(name, cond, detail=""):
        if not cond:
            fails.append(f"{name}: {detail}")

    # 1) 谓词求值
    ok("kraid bit0", eval_pred("kraid_status & 0x01", {"kraid_status": 0x01}))
    ok("kraid bit7 不算击破", not eval_pred("kraid_status & 0x01", {"kraid_status": 0x80}))
    ok("缺键按 0", not eval_pred("kraid_status & 0x01", {}))
    try:
        eval_pred("__import__('os').system('true')", {})
        ok("拒非法字符", False, "未拒绝")
    except NodeError:
        ok("拒非法字符", True)

    # 2) 完整链路轨迹 ⇒ 全到达
    zeb = dict(zeb0=0, zeb1=0, zeb2=0, zeb3=0, zeb4=0)
    full = [
        dict(f=10, w=dict(kraid_status=0x00, ridley_status=0x00, in_area=0x02)),
        dict(f=20, w=dict(kraid_status=0x01, ridley_status=0x00, in_area=0x02)),
        dict(f=25, w=dict(kraid_status=0x01, ridley_status=0x02, in_area=0x04)),
        dict(f=30, w=dict(kraid_status=0x81, ridley_status=0x82, in_area=0x00)),
        dict(f=40, w=dict(kraid_status=0x81, ridley_status=0x82, bridge=1)),
        dict(f=50, w=dict(bridge=1, in_area=0x03)),
        dict(f=52, w=dict(in_area=0x03, zeb0=8, zeb1=8, zeb2=8, zeb3=8, zeb4=8)),
        dict(f=55, w=dict(in_area=0x03, **zeb)),
        dict(f=70, w=dict(in_area=0x03, mb_status=7)),
        dict(f=80, w=dict(end_timer=120)),
        dict(f=90, w=dict(end_timer=0)),
        dict(f=100, w=dict(end_timer=0, at_ending=1)),
    ]
    r = eval_trace(full)
    ok("完整链路全到达", r["first_break"] is None and len(r["arrived"]) == len(NODES),
       f"arrived={[a['id'] for a in r['arrived']]} break={r['first_break']}")
    ok("到达帧正确", [a["frame"] for a in r["arrived"]] == [20, 25, 30, 40, 50, 55, 70, 90, 100],
       str([(a["id"], a["frame"]) for a in r["arrived"]]))

    # 3) 雕像始终没升起 ⇒ 断在 S3（桥那帧就算 bridge=1 也不会越过 S3 顺序）
    broken = [
        dict(f=20, w=dict(kraid_status=0x01, ridley_status=0x00, in_area=0x02)),
        dict(f=25, w=dict(kraid_status=0x01, ridley_status=0x02, in_area=0x04)),
        dict(f=40, w=dict(kraid_status=0x01, ridley_status=0x02, bridge=1, in_area=0x00)),
        dict(f=100, w=dict(at_ending=1, in_area=0x00)),
    ]
    r2 = eval_trace(broken)
    ok("缺雕像升起 ⇒ 断在 S3", r2["first_break"] and r2["first_break"]["id"] == "S3", str(r2["first_break"]))

    # 4) Zebetite 从未被 watch ⇒ 不算达成（键缺失不作为证据）
    nowatch = [dict(f=b["f"], w={k: v for k, v in b["w"].items() if not k.startswith("zeb")}) for b in full]
    r3 = eval_trace(nowatch)
    ok("Zebetite 未采集 ⇒ 断在 S6", r3["first_break"] and r3["first_break"]["id"] == "S6", str(r3["first_break"]))

    # 4b) 整块内存 hex 形态（Rust 腿 rust.state.jsonl）能按偏移取出同一批键。
    #     iram 是 $0100-$07FF 的 1792 B 视图：$010A ⇒ 偏移 0x0A、$0758 ⇒ 偏移 0x658。
    zp = bytearray(0x100); iram = bytearray(0x700); wram = bytearray(0x2000)
    zp[0x74] = 0x11; zp[0x54] = 1; zp[0x98] = 7
    iram[0x010A - 0x0100] = 0xF0
    iram[0x0758 - 0x0100] = 3
    wram[0x687B - 0x6000] = 0x81; wram[0x687C - 0x6000] = 0x82; wram[0x6883 - 0x6000] = 1
    dec = _decode_blobs(dict(zp=zp.hex(), iram=iram.hex(), wram=wram.hex(), pw="00" * 16))
    ok("blob 解码 in_area", dec.get("in_area") == 0x11, str(dec))
    ok("blob 解码 kraid/ridley", dec.get("kraid_status") == 0x81 and dec.get("ridley_status") == 0x82, str(dec))
    ok("blob 解码 at_ending/bridge/mb", dec.get("at_ending") == 1 and dec.get("bridge") == 1 and dec.get("mb_status") == 7, str(dec))
    # 换算锁死：iram 偏移必须按 addr-$0100，$010A 读 iram[0x0A]、$0758 读 iram[0x658]
    ok("iram 换算 $010A→0x0A", dec.get("end_timer") == 0xF0, str(dec.get("end_timer")))
    ok("iram 换算 $0758→0x658", dec.get("zeb0") == 3, str(dec.get("zeb0")))
    # 反例：把 $010A 当绝对偏移(0x10A)读，在 1792 B 块里会越过 $0758 区（这里直接 fail-loud）
    try:
        _decode_blobs(dict(iram=bytearray(0x700).hex()))
        ok("右尺寸 iram 块可取到全部键", True)
    except NodeError as e:
        ok("右尺寸 iram 块可取到全部键", False, str(e))
    # 越界必须 fail-loud（旧实现静默跳过 ⇒ S6 恒不可达）
    try:
        _decode_blobs(dict(iram=bytearray(0x100).hex()))
        ok("iram 越界 fail-loud", False, "未抛 NodeError")
    except NodeError as e:
        ok("iram 越界 fail-loud", "越界" in str(e) and "zeb0" in str(e) or "zeb" in str(e), str(e))
    # 派生键：in_tourian 只在 in_area 低 4 位 ==3 时为 1
    dec3 = _decode_blobs(dict(zp=(lambda b: (b.__setitem__(0x74, 0x03), b.hex())[1])(bytearray(0x100))))
    ok("派生 in_tourian 区号3=1", dec3.get("in_tourian") == 1, str(dec3.get("in_tourian")))
    dec0 = _decode_blobs(dict(zp=(lambda b: (b.__setitem__(0x74, 0x00), b.hex())[1])(bytearray(0x100))))
    ok("派生 in_tourian 区号0=0", dec0.get("in_tourian") == 0, str(dec0.get("in_tourian")))

    # 4c) 真产物上的换算断言（有则锁死；无则显式 SKIP，不静默）
    real_trace = ROOT / "analysis" / "tas-main-f22895" / "rust" / "full.state.jsonl"
    real_cov = ROOT / "analysis" / "tas-main-f22895" / "coverage.json"
    if real_trace.exists() and real_cov.exists():
        rc = check_convert(real_trace, real_cov)
        ok("真产物换算 f=22895", rc["ok"], str(rc))
    else:
        print(f"SELFTEST SKIP：无真产物（{real_trace}）——换算断言由 `verify-convert` 单独跑")

    # 5) end_timer 一直为 0 不算达成（require_seen）
    flat = [dict(f=b["f"], w=dict(b["w"])) for b in full]
    for b in flat:
        b["w"]["end_timer"] = 0
    r4 = eval_trace(flat)
    ok("end_timer 未曾>0 ⇒ 不算达成", r4["first_break"] and r4["first_break"]["id"] == "S8", str(r4["first_break"]))

    # 5b) S6 区域门：Zebetite 全 0 但从未进 Tourian ⇒ 不得判到达（#98 §5 假阳性）
    no_tourian = [
        dict(f=20, w=dict(kraid_status=0x01, ridley_status=0x00, in_area=0x02)),
        dict(f=25, w=dict(kraid_status=0x01, ridley_status=0x02, in_area=0x04)),
        dict(f=30, w=dict(kraid_status=0x81, ridley_status=0x82, in_area=0x00)),
        dict(f=40, w=dict(kraid_status=0x81, ridley_status=0x82, bridge=1, in_area=0x00)),
        dict(f=50, w=dict(bridge=1, in_area=0x00, **zeb)),
        dict(f=60, w=dict(in_area=0x00, **zeb)),
    ]
    r5 = eval_trace(no_tourian)
    ok("未进 Tourian ⇒ S6 不判到达", r5["first_break"] and r5["first_break"]["id"] == "S5",
       str(r5["first_break"]))
    # 进过 Tourian 但 Zeb 一直是 0（从未生成）⇒ 曾非零门挡住，不得判「已清空」
    entered_zero = no_tourian[:-1] + [dict(f=55, w=dict(in_area=0x03, **zeb))]
    r6 = eval_trace(entered_zero)
    ok("Zeb 从未非零 ⇒ S6 仍不判到达", not any(a["id"] == "S6" for a in r6["arrived"]), str(r6["arrived"]))
    # 进过 Tourian 且 Zeb 曾非零、随后归零 ⇒ S6 到达
    entered_ok = no_tourian[:-1] + [
        dict(f=55, w=dict(in_area=0x03, in_tourian=None, zeb0=8, zeb1=8, zeb2=8, zeb3=8, zeb4=8)),
        dict(f=60, w=dict(in_area=0x03, **zeb)),
    ]
    entered_ok = [{k: v for k, v in x["w"].items() if v is not None} and dict(f=x["f"], w={k: v for k, v in x["w"].items() if v is not None}) for x in entered_ok]
    r7 = eval_trace(entered_ok)
    ok("Zeb 曾非零后归零 ⇒ S6 到达", any(a["id"] == "S6" for a in r7["arrived"]), str(r7["arrived"]))

    # 6) 静态链路：真仓库上跑通（不强求全绿，只求不抛异常且结构完整）
    rows = check_links()
    ok("静态链路条目数", len(rows) == len(LINKS), str(len(rows)))
    for r4 in rows:
        ok(f"链路字段完整 {r4['id']}", {"ok", "file", "reason", "line"} <= set(r4), str(sorted(r4)))

    # 7) 静态链路能识别缺失
    fake = ROOT / "tools" / "_stage_a_selftest_missing.txt"
    fake.write_text("nothing here\n", encoding="utf-8")
    try:
        saved = LINKS[0]["file"]
        LINKS[0]["file"] = "tools/_stage_a_selftest_missing.txt"
        rr = check_links()[0]
        ok("静态链路能判断链", rr["ok"] is False, str(rr))
        LINKS[0]["file"] = saved
    finally:
        fake.unlink(missing_ok=True)

    if fails:
        print("SELFTEST FAIL:")
        for f in fails:
            print("  -", f)
        return 1
    print(f"SELFTEST: 全部通过（断言组 6，节点 {len(NODES)}，链路 {len(LINKS)}）")
    return 0


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description="Stage A 通关验收面：节点判定表（静态链路 + 动态到达表）")
    sub = ap.add_subparsers(dest="cmd", required=True)

    p = sub.add_parser("links", help="静态校验进度链连接点是否存在")
    p.add_argument("--json", action="store_true")
    p.set_defaults(func=cmd_links)

    p = sub.add_parser("trace", help="按节点序评估一份逐帧轨迹")
    p.add_argument("--trace", required=True)
    p.add_argument("--json", action="store_true")
    p.set_defaults(func=cmd_trace)

    p = sub.add_parser("report", help="静态链路 + 动态轨迹 ⇒ 一句判决")
    p.add_argument("--trace", default=None)
    p.set_defaults(func=cmd_report)

    p = sub.add_parser("nodes", help="打印节点表 / 链路表 / watch 键")
    p.set_defaults(func=cmd_nodes)

    p = sub.add_parser("verify-convert", help="校验 iram 偏移换算与 coverage 一致（$0400 ⇒ 0x300）")
    p.add_argument("--trace", required=True)
    p.add_argument("--coverage", default=None)
    p.set_defaults(func=cmd_verify_convert)

    p = sub.add_parser("selftest", help="自测（不依赖仓库状态以外的输入）")
    p.set_defaults(func=cmd_selftest)

    args = ap.parse_args(argv)
    try:
        return args.func(args)
    except NodeError as e:
        print(f"[错误] {e}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
