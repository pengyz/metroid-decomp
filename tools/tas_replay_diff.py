#!/usr/bin/env python3
"""TAS M1：inputlog → 双侧逐帧报告（emu Mesen 腿 + Rust 腿）。

只出报告、不做判定：判等粒度（整片/断点帧/关卡出口）留在工具外，由调用方按报告里的
`first_divergence` 与逐帧表自行裁定。

用法（详见 docs/design/tas-replay-pipeline.md）：

    python3 tools/tas_replay_diff.py replay \
        --inputlog movies/eien86-6725M.inputlog --frames 1200 \
        --out analysis/tas-m1/eien86-1200

自测（不启 emu）：

    python3 tools/tas_replay_diff.py selftest
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import time
import sys
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
sys.path.insert(0, str(ROOT / "tools" / "movie_ref"))


class ReplayError(RuntimeError):
    """fail-loud：缺帧/超时/rc 非 0/字段缺失一律抛，调用方按 rc=2 退出。"""


def sha256_bytes(b: bytes) -> str:
    return hashlib.sha256(b).hexdigest()


def sha256_file(p: Path) -> str:
    return sha256_bytes(p.read_bytes())


def parse_range(spec: str) -> tuple[int, int]:
    m = re.fullmatch(r"([0-9A-Fa-f]{4})-([0-9A-Fa-f]{4})", spec.strip())
    if not m:
        raise ReplayError(f"--range 需形如 0400-04FF，收到 {spec!r}")
    lo, hi = int(m.group(1), 16), int(m.group(2), 16)
    if hi < lo:
        raise ReplayError(f"--range 起止颠倒: {spec}")
    return lo, hi


def build_scenarios(inputlog: Path, frames: int, name: str, out_dir: Path,
                    rng: tuple[int, int], tag: str,
                    captures: list[str]) -> tuple[Path, Path]:
    """两条腿各自的调度面（**同源**：都来自 movie_sched 的同一 inputlog 展开）。

    - emu 腿：`--format mesen2`（场景内带 `inputlog`，由 scenario_run 展开）+ watch 观察面；
    - rust 腿：`--format sched`（展开成逐帧 `joy` 行——rust parser 不认 `inputlog` 指令）。
    """
    lo, hi = rng
    out_dir.mkdir(parents=True, exist_ok=True)
    mesen = out_dir / "scenario-mesen.txt"
    sched = out_dir / "scenario-rust.sched"
    for fmt, dest in (("mesen2", mesen), ("sched", sched)):
        cmd = [sys.executable, str(ROOT / "tools" / "movie_ref" / "movie_sched.py"),
               "--inputlog", str(inputlog), "--frames", str(frames), "--format", fmt,
               "--name", name]
        for c in captures:
            cmd += ["--capture", c]
        cmd += ["--out", str(dest)]
        r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True,
                           encoding="utf-8", errors="replace")
        if r.returncode != 0:
            raise ReplayError(f"movie_sched({fmt}) 失败 rc={r.returncode}: {r.stderr[-800:]}")
        if not dest.exists():
            raise ReplayError(f"movie_sched({fmt}) 未产出: {dest}")
    extra = f"watch_range {lo:04X}-{hi:04X} {tag}\nwatch 0014 Joy1Status\n"
    text = mesen.read_text(encoding="utf-8")
    if not text.endswith("\n"):
        text += "\n"
    mesen.write_text(text + extra, encoding="utf-8")
    return mesen, sched


def run_emu(scen: Path, timeout: float, traces: Path, shots: Path,
            expect: Path | None = None) -> None:
    """跑 emu 腿。产物一律落 `--out`（`GUNNAC_TRACES`/`GUNNAC_SHOTS` 覆盖），
    不往仓库的 `analysis/traces`、`shots/` 写——卡面边界要求不碰 shots/。"""
    traces.mkdir(parents=True, exist_ok=True)
    shots.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ, GUNNAC_TRACES=str(traces), GUNNAC_SHOTS=str(shots))
    cmd = [sys.executable, str(ROOT / "tools" / "scenario_run.py"), "run", str(scen),
           "--timeout", str(timeout)]
    r = subprocess.run(cmd, cwd=ROOT, env=env, capture_output=True, text=True,
                       encoding="utf-8", errors="replace")
    if r.returncode != 0:
        raise ReplayError(f"emu 腿 rc={r.returncode}\nstdout:{r.stdout[-800:]}\nstderr:{r.stderr[-800:]}")
    # scenario_run 在串行锁被占等情况下可能「打一行错、退出码 0」，不校验产物就会
    # 变成上层一句没信息的「缺产物」。这里把它的输出带上，锁冲突才能被重试逻辑识别。
    if expect is not None and not expect.exists():
        raise ReplayError(f"emu 腿没产出 {expect.name}\nstdout:{r.stdout[-800:]}\nstderr:{r.stderr[-800:]}")


def run_rust(scen: Path, out_dir: Path, state_out: Path) -> None:
    out_dir.mkdir(parents=True, exist_ok=True)
    cmd = ["cargo", "run", "-q", "-p", "game-core", "--bin", "rust_screenshot", "--",
           "--schedule", str(scen), "--out", str(out_dir), "--state-out", str(state_out)]
    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, encoding="utf-8", errors="replace")
    if r.returncode != 0:
        raise ReplayError(f"rust 腿 rc={r.returncode}\n{r.stderr[-1200:]}")


def count_inputlog_frames(path: Path) -> int:
    """inputlog 行数（跳过 `#` 头与空行）——用于 --frames 越界早失败。"""
    n = 0
    for line in path.read_text(encoding="utf-8").splitlines():
        t = line.strip()
        if t and not t.startswith("#"):
            n += 1
    if n == 0:
        raise ReplayError(f"{path} 没有输入行")
    return n


def iter_jsonl(path: Path):
    """流式逐行解析（长片 state.jsonl 可达 GB 级：read_text 会把整文件读进内存）。"""
    if not path.exists():
        raise ReplayError(f"缺产物: {path}")
    with path.open(encoding="utf-8") as fh:
        for ln, line in enumerate(fh, 1):
            line = line.strip()
            if not line:
                continue
            try:
                yield json.loads(line)
            except json.JSONDecodeError as exc:
                raise ReplayError(f"{path}:{ln} JSON 解析失败: {exc}") from exc


def load_jsonl(path: Path) -> list[dict]:
    rows = list(iter_jsonl(path))
    if not rows:
        raise ReplayError(f"{path} 无记录（空跑不得绿）")
    return rows


def emu_frames(path: Path, tag: str, joy_key: str, n: int) -> dict[int, dict[str, bytes]]:
    out: dict[int, dict[str, bytes]] = {}
    for d in load_jsonl(path):
        f = d.get("f")
        w = d.get("w") or {}
        if not isinstance(f, int):
            raise ReplayError(f"{path} 行缺 f: {d}")
        if tag not in w:
            raise ReplayError(f"emu 帧 {f} 缺观察面 {tag!r}（场景未挂 watch_range？）")
        try:
            raw = bytes.fromhex(w[tag])
        except ValueError as exc:
            raise ReplayError(f"emu 帧 {f} 面 {tag} 非 hex: {exc}") from exc
        joy = w.get(joy_key)
        if joy is None:
            raise ReplayError(f"emu 帧 {f} 缺按键面 {joy_key!r}")
        if isinstance(joy, int):  # `watch` 单字节面落整数
            joy = bytes([joy & 0xFF])
        else:
            joy = bytes.fromhex(str(joy))
        out[f] = {"range": raw, "tag": tag, joy_key: joy}
    missing = [f for f in range(1, n + 1) if f not in out]
    if missing:
        raise ReplayError(f"emu 腿缺帧 {len(missing)} 个（前 5: {missing[:5]}）——fail-loud")
    return {f: out[f] for f in range(1, n + 1)}


def rust_frames(path: Path, lo: int, hi: int, n: int) -> dict[int, dict[str, bytes]]:
    out: dict[int, dict[str, bytes]] = {}
    for d in load_jsonl(path):
        f = d.get("f")
        w = (d.get("w") or {})
        iram = w.get("iram")
        zp = w.get("zp")
        if not isinstance(f, int) or not isinstance(iram, str) or not isinstance(zp, str):
            raise ReplayError(f"{path} 行缺 f/iwram 面")
        ir = bytes.fromhex(iram)  # $0100-$07FF
        if lo < 0x100 or hi > 0x7FF:
            raise ReplayError("本工具只支持 $0100-$07FF 范围的判等面")
        out[f] = {"range": ir[lo - 0x100: hi - 0x100 + 1],
                  "Joy1Status": bytes.fromhex(zp)[0x14:0x15]}
    # Rust 腿在 span/载房窗帧**不跑 tick**（引擎一跳多帧），这些帧没有状态行是引擎行为，
    # 不是工具故障：记进 coverage.rust_missing 并在报告里显式列出，绝不静默当成一致。
    return out


def compare(emu: dict[int, dict[str, bytes]], rust: dict[int, dict[str, bytes]],
            lo: int, n: int) -> dict:
    per = []
    first = None
    common = sorted(set(emu) & set(rust))
    if not common:
        raise ReplayError("两侧无共同帧——空跑不得绿")
    for f in common:
        e, r = emu[f]["range"], rust[f]["range"]
        if len(e) != len(r):
            raise ReplayError(f"帧 {f} 两侧判等面长度不等 {len(e)} vs {len(r)}")
        eq = e == r
        # 按键面两侧采样时刻不同：emu 的 Joy1Status[f] 是**应用前**（== inputlog 第 f-1 行），
        # rust 的 zp[0x14][f] 是**应用后**（== 第 f 行）。故对齐比 emu[f] 与 rust[f-1]，
        # 原样比只保留在 joy_raw_equal 里作溯源，不参与「一致」判断。
        prev = rust.get(f - 1)
        je = None if prev is None else emu[f]["Joy1Status"] == prev["Joy1Status"]
        row = {"frame": f, "equal": eq,
               "sha256_emu": sha256_bytes(e), "sha256_rust": sha256_bytes(r),
               "joy_emu": emu[f]["Joy1Status"].hex(), "joy_rust": rust[f]["Joy1Status"].hex(),
               "joy_equal": je,
               "joy_raw_equal": emu[f]["Joy1Status"] == rust[f]["Joy1Status"]}
        if not eq:
            row["diff_bytes"] = sum(1 for a, b in zip(e, r) if a != b)
            if first is None:
                off = next(i for i, (a, b) in enumerate(zip(e, r)) if a != b)
                first = {"frame": f, "field": f"${lo + off:04X}",
                         "offset": off, "emu": e[off], "rust": r[off],
                         "diff_bytes": row["diff_bytes"]}
                row["first_field"] = first["field"]
        per.append(row)
    return {"frames": per, "first_divergence": first,
            "diverge_frames": sum(1 for r in per if not r["equal"]),
            "compared": len(common),
            "range": f"${lo:04X}-${lo + len(emu[common[0]]['range']) - 1:04X}"}


def shot_report(emu_shots: dict[int, Path], rust_shots: dict[int, Path]) -> list[dict]:
    """采集点画面：给两侧文件 sha 与**像素**比对。

    两侧 PNG 编码器不同（Mesen 截图 vs Rust png crate）⇒ 文件 sha 不等也可能像素全同，
    故 `equal` 以像素为准；`sha256_*` 只作溯源。像素尺寸不一致直接 fail-loud。
    """
    from PIL import Image  # 延迟导入：只有比画面时的依赖

    out = []
    for f in sorted(set(emu_shots) | set(rust_shots)):
        e, r = emu_shots.get(f), rust_shots.get(f)
        if e is None or r is None:
            raise ReplayError(f"采集帧 {f} 单侧缺图（emu={bool(e)} rust={bool(r)}）——fail-loud")
        ia, ib = Image.open(e).convert("RGB"), Image.open(r).convert("RGB")
        if ia.size != ib.size:
            raise ReplayError(f"采集帧 {f} 画面尺寸不等 {ia.size} vs {ib.size}")
        pa, pb = ia.tobytes(), ib.tobytes()
        diff = sum(1 for i in range(0, len(pa), 3) if pa[i:i + 3] != pb[i:i + 3])
        out.append({"frame": f, "sha256_emu": sha256_file(e), "sha256_rust": sha256_file(r),
                    "pixel_diff": diff, "equal": diff == 0})
    return out


def collect_shots(prefix: str, pattern_dirs: list[Path]) -> dict[int, Path]:
    got: dict[int, Path] = {}
    for d in pattern_dirs:
        for p in sorted(d.glob(f"*_f[0-9]*.png")):
            m = re.search(r"_f(\d{5})", p.name)
            if m:
                got[int(m.group(1))] = p
    return got


def write_report(out: Path, payload: dict) -> None:
    out.mkdir(parents=True, exist_ok=True)
    canon = json.dumps(payload, ensure_ascii=False, sort_keys=True, indent=2) + "\n"
    (out / "report.json").write_text(canon, encoding="utf-8")
    lines = ["# TAS 回放对拍报告", "",
             f"- inputlog: `{payload['input']['inputlog']}`",
             f"- frames: {payload['input']['frames']}",
             f"- 判等面: `{payload['compare']['range']}`（逐帧 sha256）",
             f"- 首分歧: {payload['compare']['first_divergence'] or '无'}",
             f"- 分歧帧数: {payload['compare']['diverge_frames']}",
             f"- 覆盖: emu {payload['coverage']['emu_frames']} 帧 / rust {payload['coverage']['rust_frames']} 帧 / "
             f"可比 {payload['coverage']['compared']} 帧",
             f"- rust 未 tick 帧（span/载房窗，一跳多帧；**记为不可比、不算一致**）: "
             f"{len(payload['coverage']['rust_missing'])} 个"
             + ("（本次以 --allow-rust-missing 放行）" if payload['coverage']['rust_missing'] else ""), "",
             "> 按键列：emu 采的是**应用前**（inputlog 第 f−1 行），rust 是**应用后**（第 f 行）；"
             "表中已按 emu[f] ↔ rust[f−1] 对齐，`joy_raw_equal` 只作溯源、不是判据。", "",
             "| 帧 | 内存一致 | emu sha | rust sha | 按键对齐一致 |", "|---|---|---|---|---|"]
    for row in payload["compare"]["frames"]:
        lines.append(f"| {row['frame']} | {'✅' if row['equal'] else '❌'} | "
                     f"`{row['sha256_emu'][:12]}` | `{row['sha256_rust'][:12]}` | "
                     f"{'—' if row['joy_equal'] is None else ('✅' if row['joy_equal'] else '❌')} |")
    if payload.get("shots"):
        lines += ["", "## 采集点画面", "", "| 帧 | 像素一致 | 差异像素 | emu sha | rust sha |",
                  "|---|---|---|---|---|"]
        for s in payload["shots"]:
            lines.append(f"| {s['frame']} | {'✅' if s['equal'] else '❌'} | {s['pixel_diff']} | "
                         f"`{s['sha256_emu'][:12]}` | `{s['sha256_rust'][:12]}` |")
    lines += ["", "## 已知限制", "",
              "- 端口 2（`joy2`）在 Mesen 腿不可驱动（`tools/scenario_capture.lua:1109-1120`），"
              "含 2P 输入的片段只能比 1P 面。",
              "- 判等粒度不在本工具：报告只给逐帧事实与首分歧，整片/断点帧/出口由调用方裁。",
              "- 画面 sha 只覆盖场景里 `capture` 声明的帧（逐帧截图成本高），内存面逐帧全覆盖。"]
    (out / "report.md").write_text("\n".join(lines) + "\n", encoding="utf-8")


def rebuild_report(out: Path, args) -> int:
    """只用已有的 `coverage.json` 重写报告（不跑 emu/rust）。

    用途：长片整跑几十分钟，报告口径（如「首个真实分歧」标注）改进后不该为了几行表格
    重跑一遍。读入既有 entries，按当前口径重算并覆盖 `coverage.json` / `coverage.md`。
    """
    path = out / "coverage.json"
    if not path.exists():
        raise ReplayError(f"没有可重建的报告: {path}（先跑一次 --segments）")
    old = json.loads(path.read_text(encoding="utf-8"))
    inputlog = old["input"]["inputlog"]
    il = Path(inputlog)
    il = il if il.is_absolute() else ROOT / il
    write_coverage(out, il, old["total_frames"], old["segment_size"], args,
                   old["segments"], old["ran_frames"], old["input"]["movie_frames"])
    new = json.loads(path.read_text(encoding="utf-8"))
    fd = new.get("first_divergence")
    print(f"[tas] 报告已重建: {path} / {out / 'coverage.md'}"
          + (f"；首个真实分歧 f{fd['frame']} {fd['field']}" if fd else "；无分歧"))
    return 0


def cmd_replay(args) -> int:
    inputlog = Path(args.inputlog)
    if not inputlog.is_absolute():
        inputlog = ROOT / inputlog
    if not inputlog.exists():
        raise ReplayError(f"inputlog 不存在: {inputlog}")
    rng = parse_range(args.range)
    have = count_inputlog_frames(inputlog)
    frames = args.frames or have
    if frames < 1 or frames > have:
        raise ReplayError(f"--frames {frames} 越界：inputlog 只有 {have} 帧（movie 长度上限）")
    if getattr(args, "report_only", False):
        out = Path(args.out)
        out = out if out.is_absolute() else ROOT / out
        return rebuild_report(out, args)
    if getattr(args, "segments", 0) and args.segments > 0:
        if args.start != 1 or (args.chunk or 0) > 0:
            print("[tas] --segments 模式下 --start/--chunk 不适用（分段本身即窗口化），已忽略",
                  file=sys.stderr)
        return run_segments(args, inputlog, have, frames, args.start)
    start = args.start
    if not (1 <= start <= frames):
        raise ReplayError(f"--start {start} 越界：应在 1..{frames}")
    out = Path(args.out)
    if not out.is_absolute():
        out = ROOT / out
    name = args.name or f"tas-{inputlog.stem}-{frames}"
    scen, sched = build_scenarios(inputlog, frames, name, out, rng,
                                  args.tag, args.capture)
    # emu 产物必须先落**仓库内**：scenario_run 用 relative_to(ROOT) 记 run.json 的 shots 列表，
    # 直接指到 /tmp 会炸。故先 staging 到 analysis/.tas-m1-runs/<name>，比完再搬进 --out。
    stage = ROOT / "analysis" / ".tas-m1-runs" / name
    emu_traces, emu_shots_dir = stage / "emu-traces", stage / "emu-shots"
    if stage.exists():
        shutil.rmtree(stage)
    # finally 包住**整个** replay 主体：emu 腿之后的 rust 腿、fail-loud、报告写入任一处抛错，
    # staging 都不会留在 analysis/.tas-m1-runs/ 里（#32 复核抓到的缺陷）。
    try:
        run_emu(scen, args.timeout, emu_traces, emu_shots_dir)
        emu_jsonl = emu_traces / f"{name}.frames.jsonl"
        emu_shots = collect_shots(name, [emu_shots_dir])
        run_rust(sched, out / "rust-shots", out / "rust.state.jsonl")
        rust_jsonl = out / "rust.state.jsonl"
        emu = emu_frames(emu_jsonl, args.tag, "Joy1Status", frames)
        rust = rust_frames(rust_jsonl, *rng, frames)
        cmp_ = compare(emu, rust, rng[0], frames)
        missing = [f for f in range(1, frames + 1) if f not in rust]
        if missing and not args.allow_rust_missing:
            raise ReplayError(
                f"rust 腿 {len(missing)}/{frames} 帧未 tick（前 8: {missing[:8]}）。"
                "卡面要求覆盖双侧各 N 帧：默认 fail-loud；确认是 span/载房窗跳帧、"
                "接受把这些帧记为「不可比」时，显式加 --allow-rust-missing 重跑。")
        # 采集点画面：emu 侧 shots/<name>_f*_{tag}.png，rust 侧 out/rust-shots
        emu_caps = {f: p for f, p in emu_shots.items() if f in {int(c.split(":")[0]) for c in args.capture}}
        rust_caps = {f: p for f, p in collect_shots(name, [out / "rust-shots"]).items()
                     if f in emu_caps}
        payload = {
            "input": {"inputlog": str(inputlog.relative_to(ROOT)), "frames": frames,
                      "start": start, "range": args.range, "tag": args.tag,
                      "chunk": args.chunk or 0},
            "compare": cmp_,
            "coverage": {"emu_frames": len(emu), "rust_frames": len(rust),
                         "compared": len(set(emu) & set(rust)),
                         "rust_missing": missing,
                         "rust_missing_policy": "listed-not-compared" if missing else "none",
                         "frame_window": [start, frames] if start > 1 else [1, frames],
                         "start_note": ("--start 只裁剪比较窗口；emu 无法寻址，回放仍从第 1 帧跑"
                                        "（耗时≈frames，不是窗口长度）") if start > 1 else ""},
            "shots": shot_report(emu_caps, rust_caps) if emu_caps else [],
            "deterministic": True,
        }
        write_report(out, payload)
        write_parts(out, payload, start)
        # staging → --out（/tmp 也可用）：产物随报告走，仓库里不留残渣
        for src in (emu_traces, emu_shots_dir):
            dst = out / src.name
            if dst.exists():
                shutil.rmtree(dst)
            shutil.move(str(src), str(dst))
    finally:
        shutil.rmtree(stage, ignore_errors=True)
    print(f"[tas] 报告: {out / 'report.json'} / {out / 'report.md'}")
    print(f"[tas] 分歧帧 {cmp_['diverge_frames']}/{frames}（比较 {cmp_['compared']} 帧）；首分歧 "
          f"{cmp_['first_divergence'] or '无'}")
    return 0


def write_parts(out: Path, payload: dict, start: int) -> None:
    """`--chunk N`：把同一份逐帧表切成每 N 帧一个 part 文件，便于分段审阅与跨进程比对。

    注意：切的是**报告**，不是回放。Mesen 无法从任意帧起播（CLI 无 load-savestate），
    分段回放只能每段从第 1 帧重放，总耗时≈Σ段末帧，因此不提供那种「假分段」。
    """
    chunk = payload["input"].get("chunk") or 0
    if chunk <= 0:
        return
    frames = payload["compare"]["frames"]
    d = out / "parts"
    d.mkdir(parents=True, exist_ok=True)
    idx = []
    for lo in range(start, payload["input"]["frames"] + 1, chunk):
        hi = min(lo + chunk - 1, payload["input"]["frames"])
        sel = [r for r in frames if lo <= r["frame"] <= hi]
        part = {"input": payload["input"], "coverage": payload["coverage"],
                "window": [lo, hi], "frames": sel,
                "diverge_frames": sum(1 for r in sel if not r["equal"]),
                "first_divergence": next((r["frame"] for r in sel if not r["equal"]), None),
                "deterministic": True}
        fn = f"part-{lo:05d}-{hi:05d}.json"
        (d / fn).write_text(json.dumps(part, ensure_ascii=False, sort_keys=True, indent=2) + "\n",
                            encoding="utf-8")
        idx.append({"window": [lo, hi], "file": f"parts/{fn}", "frames": len(sel),
                    "diverge_frames": part["diverge_frames"],
                    "first_divergence": part["first_divergence"]})
    (d / "index.json").write_text(json.dumps({"chunk": chunk, "parts": idx}, ensure_ascii=False,
                                             sort_keys=True, indent=2) + "\n", encoding="utf-8")


# ---------------------------------------------------------------- 分段链（task-45）
#
# 为什么不用「先单独跑一遍落所有检查点」：`scenario_capture.lua` 每次 run 只落一个
# `bundle_capture`，且 §实验（task-45）已证「spin-exit 缝落档 → 续跑逐字节一致」。
# 于是让**每段顺带落下一段入口**：
#   段 k [a,b)：emu `frames=b` + (`snapshot cp_a` + `resume_at a`，k>0) + `bundle_capture b cp_b`
#              rust 相对帧调度表（joy 行 1..b-a 取自影片 a+1..b）+ `snapshot cp_a`
# ⇒ 整片总耗时 ≈ 一遍全长；中断后 `--resume` 从 coverage.json 里第一个未 pass 的段继续。
#
# rust 腿为什么必须用相对帧表：`rust_screenshot` 的调度表没有帧偏移指令、主循环恒
# `for frame in 1..=plan.frames` 且按 `joy[frame]` 取键（crates/ 卡面禁改）⇒ 只能把
# 「段内第 i 帧」当 1 起编号，比较时按 `abs = from + i` 映射（同 findings/
# bundle-first-arbitrary-start.md §1 的「ff 行 k ↔ 连续 F_a+k」口径）。


def _run_movie_sched(inputlog: Path, frames: int, fmt: str, name: str, dest: Path) -> None:
    dest.parent.mkdir(parents=True, exist_ok=True)
    cmd = [sys.executable, str(ROOT / "tools" / "movie_ref" / "movie_sched.py"),
           "--inputlog", str(inputlog), "--frames", str(frames), "--format", fmt,
           "--name", name, "--out", str(dest)]
    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True,
                       encoding="utf-8", errors="replace")
    if r.returncode != 0:
        raise ReplayError(f"movie_sched({fmt}) 失败 rc={r.returncode}: {r.stderr[-800:]}")
    if not dest.exists():
        raise ReplayError(f"movie_sched({fmt}) 未产出: {dest}")


def build_segment_emu_scenario(inputlog: Path, to: int, name: str, dest: Path,
                               cp_from: Path | None, cp_to: Path | None,
                               from_: int = 0, rng: tuple[int, int] = (0x400, 0x4FF),
                               tag: str = "ens") -> None:
    """emu 场景：绝对帧号（`frames=to`）；k>0 时装载 cp_from 并把帧号续到 from_，
    段末把状态落成 cp_to（下一段的入口）。"""
    _run_movie_sched(inputlog, to, "mesen2", name, dest)
    text = dest.read_text(encoding="utf-8")
    if not text.endswith("\n"):
        text += "\n"
    text += "sample_point spin-exit\n"
    lo, hi = rng
    text += f"watch_range {lo:04X}-{hi:04X} {tag}\nwatch 0014 Joy1Status\n"
    if cp_from is not None:
        text += f"snapshot {cp_from}\nresume_at {from_}\n"
    if cp_to is not None:
        text += f"bundle_capture {to - 1} {cp_to}\n"
    dest.write_text(text, encoding="utf-8")


def build_segment_rust_sched(inputlog: Path, from_: int, to: int, name: str,
                             dest: Path, cp_from: Path | None) -> None:
    """rust 场景：相对帧表（joy 1..to-from_ = 影片 from_+1..to 的输入）。"""
    full = dest.with_suffix(".full.sched")
    _run_movie_sched(inputlog, to, "sched", name, full)
    out = [f"name {name}", f"frames {to - from_}"]
    for raw in full.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        parts = line.split()
        if parts[0] in ("joy", "joy2") and len(parts) >= 3:
            f = int(parts[1])
            if f > from_:
                out.append(f"{parts[0]} {f - from_} {parts[2]}")
        elif parts[0] == "name" or parts[0] == "frames":
            continue  # 已重写
    if cp_from is not None:
        out.append(f"snapshot {cp_from}")
    dest.write_text("\n".join(out) + "\n", encoding="utf-8")
    full.unlink(missing_ok=True)


def emu_rows_abs(path: Path, tag: str, joy_key: str) -> dict[int, dict[str, bytes]]:
    """emu 逐帧面（不要求 1..n 连号——段跑只覆盖自己的窗口）。"""
    out: dict[int, dict[str, bytes]] = {}
    for d in load_jsonl(path):
        f = d.get("f")
        w = d.get("w") or {}
        if not isinstance(f, int):
            raise ReplayError(f"{path} 行缺 f: {d}")
        if tag not in w:
            raise ReplayError(f"emu 帧 {f} 缺观察面 {tag!r}（场景未挂 watch_range？）")
        try:
            raw = bytes.fromhex(w[tag])
        except ValueError as exc:
            raise ReplayError(f"emu 帧 {f} 面 {tag} 非 hex: {exc}") from exc
        joy = w.get(joy_key)
        if isinstance(joy, int):
            joy = bytes([joy & 0xFF])
        elif isinstance(joy, str):
            joy = bytes.fromhex(joy)
        out[f] = {"range": raw, "Joy1Status": joy or b""}
    return out


def rust_rows_rel(path: Path, lo: int, hi: int) -> dict[int, dict[str, bytes]]:
    """rust 逐帧面（相对帧号 1..len）；返回 {rel_frame: {range, Joy1Status}}。"""
    out: dict[int, dict[str, bytes]] = {}
    for d in iter_jsonl(path):
        f = d.get("f")
        w = (d.get("w") or {})
        iram, zp = w.get("iram"), w.get("zp")
        if not isinstance(f, int) or not isinstance(iram, str) or not isinstance(zp, str):
            raise ReplayError(f"{path} 行缺 f/iwram 面")
        ir = bytes.fromhex(iram)
        if lo < 0x100 or hi > 0x7FF:
            raise ReplayError("本工具只支持 $0100-$07FF 范围的判等面")
        out[f] = {"range": ir[lo - 0x100: hi - 0x100 + 1],
                  "Joy1Status": bytes.fromhex(zp)[0x14:0x15]}
    return out


def compare_window(emu: dict[int, dict[str, bytes]], rust_rel: dict[int, dict[str, bytes]],
                   from_: int, lo: int, rust_base: int | None = None) -> dict:
    """段内逐帧比：rust 行号 r ↔ 绝对帧 rust_base+r（emu 侧已是绝对帧）。

    `rust_base` 缺省 = `from_`（分段相对表）；rust 腿若为全片连续跑（行号即绝对帧），
    传 `rust_base=0`，否则会把绝对行号再叠一次偏移，整段全红（实测踩过）。
    """
    if rust_base is None:
        rust_base = from_
    per, first = [], None
    common = sorted({rust_base + r for r in rust_rel} & set(emu))
    if not common:
        raise ReplayError(f"段内无共同帧（emu={len(emu)} rust={len(rust_rel)}）——空跑不得绿")
    for ap in common:
        e = emu[ap]["range"]
        r = rust_rel[ap - rust_base]["range"]
        if len(e) != len(r):
            raise ReplayError(f"帧 {ap} 两侧判等面长度不等 {len(e)} vs {len(r)}")
        eq = e == r
        row = {"frame": ap, "equal": eq,
               "sha256_emu": sha256_bytes(e), "sha256_rust": sha256_bytes(r)}
        if not eq:
            row["diff_bytes"] = sum(1 for a, b in zip(e, r) if a != b)
            if first is None:
                off = next(i for i, (a, b) in enumerate(zip(e, r)) if a != b)
                first = {"frame": ap, "field": f"${lo + off:04X}", "offset": off,
                         "emu": e[off], "rust": r[off], "diff_bytes": row["diff_bytes"]}
        per.append(row)
    return {"frames": per, "first_divergence": first,
            "compared": len(common),
            "diverge_frames": sum(1 for r in per if not r["equal"]),
            "sha256_agg_emu": sha256_bytes(b"".join(emu[f]["range"] for f in common)),
            "sha256_agg_rust": sha256_bytes(b"".join(rust_rel[f - rust_base]["range"] for f in common))}


def _lua_tail(log_path: Path, n: int = 6) -> str:
    if not log_path.exists():
        return ""
    lines = [ln for ln in log_path.read_text(encoding="utf-8", errors="replace").splitlines() if ln.strip()]
    return " | ".join(lines[-n:])


def _run_emu_wait_lock(scen: Path, timeout: float, traces: Path, shots: Path,
                       *, polls: int = 60, wait_s: float = 20.0,
                       expect: Path | None = None) -> None:
    """跑一段 emu；撞上仓库级串行锁（`analysis/.scenario.lock` 被别的 agent 占）时排队重试。

    长片整跑要几十分钟，别人一次几十秒的场景就会把整链打断；这里只对「串行锁被占用」
    这一类错误重试（其它错误立即抛，不许被重试掩盖）。上限 = polls*wait_s（默认 20 分钟）。
    """
    last: ReplayError | None = None
    for i in range(polls):
        try:
            run_emu(scen, timeout, traces, shots, expect=expect)
            if i:
                print(f"[tas] 串行锁等待 {i} 轮后取得，继续")
            return
        except ReplayError as exc:
            msg = str(exc)
            if "被占用" not in msg and "串行纪律" not in msg and "另一场景运行中" not in msg:
                raise
            last = exc
            print(f"[tas] 串行锁被占用（第 {i + 1}/{polls} 轮），{wait_s:.0f}s 后重试")
            time.sleep(wait_s)
    raise ReplayError(f"串行锁等待超时（{polls} 轮 × {wait_s:.0f}s）：{last}")


def run_segments(args, inputlog: Path, have: int, frames: int, seed: int) -> int:
    rng = parse_range(args.range)
    out = Path(args.out) if Path(args.out).is_absolute() else ROOT / args.out
    out.mkdir(parents=True, exist_ok=True)
    seg = args.segments
    bounds = list(range(0, frames, seg)) + [frames]
    cps = out / "checkpoints"
    cps.mkdir(parents=True, exist_ok=True)
    cov_path = out / "coverage.json"
    prior = {}
    if args.resume and cov_path.exists():
        try:
            old = json.loads(cov_path.read_text(encoding="utf-8"))
            # 已完成 = pass 或 diverge（分歧是合法结论，段本身已跑完并落了下一段检查点）；
            # 只有 fail/缺条目才需要重跑。否则中断一次就要把几十分钟的分段重跑一遍。
            prior = {e["index"]: e for e in old.get("segments", [])
                     if e.get("status") in ("pass", "diverge")}
        except (json.JSONDecodeError, KeyError) as exc:
            raise ReplayError(f"coverage.json 不可解析，拒绝 --resume（{exc}）") from exc
    # rust 腿**不分段**：`restore_mss` 只恢复 ram/wram/regs，实测从第 700 帧存档续跑后
    # 第 922 帧起与连续跑分叉（476/697 帧不一致，见 docs/design/tas-replay-pipeline.md
    # 「已知限制」）⇒ 分段只作用于 emu 腿；rust 全片连续跑一次（成本低），按绝对帧对齐。
    rust_all = out / "rust" / "full.state.jsonl"
    rust_all.parent.mkdir(parents=True, exist_ok=True)
    if not (args.resume and rust_all.exists() and rust_all.stat().st_size > 0):
        rust_sched_all = out / "rust" / "full.sched"
        _run_movie_sched(inputlog, frames, "sched", f"tas-rust-full-{frames}", rust_sched_all)
        print(f"[tas] rust 腿：全片连续跑 {frames} 帧（不分段，见 coverage.md 已知限制）")
        t_rust = time.time()
        run_rust(rust_sched_all, out / "rust" / "shots", rust_all)
        print(f"[tas] rust 腿完成 {round(time.time() - t_rust, 1)}s → {rust_all}")
    rust_rows = rust_rows_rel(rust_all, *rng)

    entries: list[dict] = []
    ran_to = 0
    for idx in range(len(bounds) - 1):
        a, b = bounds[idx], bounds[idx + 1]
        if idx in prior:
            entries.append(prior[idx])
            ran_to = b
            print(f"[tas] seg {idx} [{a},{b}) 已完成（{prior[idx]['status']}，--resume 跳过）")
            continue
        if a > 0 and not (cps / f"cp-{a:06d}.mss").exists():
            raise ReplayError(f"缺段入口检查点 {cps / f'cp-{a:06d}'}.mss；"
                              f"先跑段 {idx-1}（或去掉 --resume 从头跑）——不静默回退上电起跑")
        name = f"tas-seg{idx:03d}-{a}to{b}"
        work = out / "work" / name
        work.mkdir(parents=True, exist_ok=True)
        emu_scen = work / "scenario-mesen.txt"
        rust_sched = work / "scenario-rust.sched"
        cp_in = cps / f"cp-{a:06d}" if a > 0 else None
        cp_out = cps / f"cp-{b:06d}"
        t0 = time.time()
        entry = {"index": idx, "from": a, "to": b, "status": "fail", "attempts": 1,
                 # --out 允许绝对路径（单段模式历来如此，/tmp 常见）；分段这边也要能算，
                 # 算不出相对路径就存绝对路径，别在 1.9s 就 ValueError 崩掉。
                 "checkpoint_out": (str(cp_out.relative_to(ROOT))
                                    if str(cp_out).startswith(str(ROOT)) else str(cp_out))}
        try:
            # 边界帧 b 的 spin 出口在 frames=b 时刚好落在收尾之后（实测 capture 不触发）
            # ⇒ emu 腿多跑 1 帧，让 checkpoints/cp-<b> 真的落盘；比较窗口仍是 [a,b]。
            emu_end = b + 1 if cp_out is not None else b
            build_segment_emu_scenario(inputlog, emu_end, name, emu_scen, cp_in, cp_out,
                                       from_=a, rng=rng, tag=args.tag)
            stage = ROOT / "analysis" / ".tas-m1-runs" / name
            if stage.exists():
                shutil.rmtree(stage)
            emu_traces, emu_shots = stage / "emu-traces", stage / "emu-shots"
            try:
                _run_emu_wait_lock(emu_scen, args.timeout, emu_traces, emu_shots,
                                   expect=emu_traces / f"{name}.frames.jsonl")
                emu_jsonl = emu_traces / f"{name}.frames.jsonl"
                emu = emu_rows_abs(emu_jsonl, args.tag, "Joy1Status")
                cmp_ = compare_window(emu, rust_rows, a, rng[0], rust_base=0)
                (out / "emu").mkdir(exist_ok=True)
                shutil.copy2(emu_jsonl, out / "emu" / f"seg-{a:06d}-{b:06d}.jsonl")
                entry.update({"status": "pass" if cmp_["diverge_frames"] == 0 else "diverge",
                              "emu_rows": len(emu), "rust_rows": len(rust_rows),
                              "compared": cmp_["compared"],
                              "diverge_frames": cmp_["diverge_frames"],
                              "first_divergence": cmp_["first_divergence"],
                              "sha256_agg_emu": cmp_["sha256_agg_emu"],
                              "sha256_agg_rust": cmp_["sha256_agg_rust"]})
            finally:
                shutil.rmtree(stage, ignore_errors=True)
        except ReplayError as exc:
            log_tail = _lua_tail(work / "emu-traces" / f"{name}.lua.log")
            reached = 0
            trace = work / "emu-traces" / f"{name}.frames.jsonl"
            if trace.exists():
                try:
                    reached = max((d.get("f", 0) for d in load_jsonl(trace)), default=0)
                except ReplayError:
                    reached = 0
            entry.update({"failure": {"frame": reached or a,
                                      "reason": str(exc)[:400],
                                      "lua_log_tail": log_tail}})
        entry["wall_s"] = round(time.time() - t0, 1)
        entries.append(entry)
        ran_to = b if entry["status"] != "fail" else a
        write_coverage(out, inputlog, frames, seg, args, entries, ran_to, have)
        print(f"[tas] seg {idx} [{a},{b}) {entry['status']} {entry['wall_s']}s"
              + (f" 首分歧 {entry.get('first_divergence')}" if entry.get("first_divergence") else "")
              + (f" 失败 {entry['failure']['reason'][:120]}" if entry.get("failure") else ""))
        if entry["status"] == "fail":
            print("[tas] 该段失败：后续段依赖它的检查点，停止推进（可用 --resume 重试本段）")
            break
    write_coverage(out, inputlog, frames, seg, args, entries, ran_to, have)
    # 「问题段」只算 fail（段没跑完）；diverge 是合法结论——分叉一旦发生不会自愈，
    # 后续段全都 diverge，按 fail 统计会把 13/13 报成「13 个问题」。
    fails = [e for e in entries if e["status"] == "fail"]
    diverged = [e for e in entries if e["status"] == "diverge"]
    fd = next((e["first_divergence"] for e in entries if e.get("first_divergence")), None)
    print(f"[tas] 覆盖率报告: {out / 'coverage.json'} / {out / 'coverage.md'}")
    print(f"[tas] 段 {len(entries)}/{len(bounds) - 1} 已跑；到帧 {ran_to}/{frames}；"
          f"失败段 {len(fails)}；分歧段 {len(diverged)}"
          + (f"；首个真实分歧 f{fd['frame']} {fd['field']}" if fd else "；无分歧"))
    # 退出码只看「有没有段失败」：diverge 是合法结论（分叉不会自愈，后续段必然也 diverge），
    # 不能当失败；ran_to 只在有段失败时才不等于 frames，故失败即 rc=2。
    return 0 if not fails else 2


def _env_fingerprint() -> dict:
    """记录报告产出时的代码环境，便于判断两份 coverage 是否同口径。

    2026-11-25（#72）：`git_head` 由 12 位短 sha 改为 **40 位提交号**，并补 `at`（ISO 时间）——
    短 sha 无法判断「这份报告是哪棵树跑的」，lane 树与主线在前 12 位之外分叉时尤其危险。
    规则见 `docs/design/evidence-binding.md`。
    """
    import hashlib
    import subprocess
    env: dict = {}
    try:
        env["git_head"] = subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT,
                                         capture_output=True, text=True,
                                         timeout=10).stdout.strip()
        env["at"] = datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")
        env["git_dirty_crates"] = subprocess.run(
            ["git", "status", "--porcelain", "--", "crates/"], cwd=ROOT,
            capture_output=True, text=True, timeout=10).stdout.strip().splitlines()
    except (OSError, subprocess.SubprocessError):
        env["git_head"] = None
    bin_ = ROOT / "target" / "debug" / "rust_screenshot"
    if bin_.exists():
        env["rust_screenshot_sha256"] = hashlib.sha256(bin_.read_bytes()).hexdigest()[:16]
        env["rust_screenshot_mtime"] = int(bin_.stat().st_mtime)
    return env


def write_coverage(out: Path, inputlog: Path, frames: int, seg: int, args,
                   entries: list[dict], ran_to: int, have: int) -> None:
    # 分段各自报的「首分歧帧」在后继段里只会重复同一个已存在分歧（一旦分叉就不会自愈），
    # 直接用逐段数字当结论会误报成 N 个独立缺陷。这里标出全局首个真实分歧，并把其后的
    # 段标为 post_divergence（承接已分歧后的漂移），只作覆盖推进与漂移量参考。
    global_fd = None
    for e in entries:
        fd = e.get("first_divergence")
        if fd and (global_fd is None or fd["frame"] < global_fd["frame"]):
            global_fd = fd
    for e in entries:
        fd = e.get("first_divergence")
        e["post_divergence"] = bool(global_fd and fd and fd["frame"] > global_fd["frame"])

    payload = {
        "first_divergence": global_fd,
        # 环境指纹：逐帧 sha 依赖 rust 二进制与树内代码，工作树被别人改过就会变
        # （实测：同一 HEAD、工作树带别人 WIP 时 sha 从 1bb9ddea 变成 a81abf70）。
        # 记下 HEAD 与二进制 sha，才能判断两次报告是不是同一口径。
        "env": _env_fingerprint(),
        "input": {"inputlog": str(inputlog.relative_to(ROOT)) if str(inputlog).startswith(str(ROOT)) else str(inputlog),
                  "movie_frames": have, "total_frames": frames, "segment_size": seg,
                  "range": args.range, "tag": args.tag},
        "total_frames": frames, "segment_size": seg, "ran_frames": ran_to,
        "coverage_pct": round(100.0 * ran_to / frames, 2),
        "segments": entries,
        "all_pass": all(e["status"] == "pass" for e in entries) and ran_to == frames,
        "deterministic": True,
    }
    (out / "coverage.json").write_text(
        json.dumps(payload, ensure_ascii=False, sort_keys=True, indent=2) + "\n", encoding="utf-8")
    lines = ["# TAS 长片分段回放覆盖率报告", "",
             f"- 影片: `{payload['input']['inputlog']}`（{have} 帧）",
             f"- 目标帧数: {frames}；段长: {seg}",
             f"- 已跑: {ran_to}/{frames}（{payload['coverage_pct']}%）",
             f"- 判等面: `{args.range}`（逐帧 sha256 聚合）", "",
             "| 段 | 区间 | 判定 | 比较帧 | 分歧帧 | emu sha | rust sha | 耗时(s) |",
             "|---|---|---|---|---|---|---|---|"]
    for e in entries:
        mark = "（承接已分歧）" if e.get("post_divergence") else ""
        lines.append(f"| {e['index']} | [{e['from']},{e['to']}) | {e['status']}{mark} | "
                     f"{e.get('compared', 0)} | {e.get('diverge_frames', 0)} | "
                     f"`{str(e.get('sha256_agg_emu', '-'))[:12]}` | `{str(e.get('sha256_agg_rust', '-'))[:12]}` | "
                     f"{e.get('wall_s', '-')} |")
    if global_fd:
        lines += ["", "## 首个真实分歧", "",
                  f"- 帧 {global_fd['frame']} 字段 `{global_fd['field']}`：emu={global_fd['emu']:02X} / "
                  f"rust={global_fd['rust']:02X}（{global_fd['diff_bytes']} 字节）",
                  "- 该帧之前的全部段逐帧一致；其后各段的分歧帧数只表示漂移规模，不是新增缺陷。"]
    fails = [e for e in entries if e.get("failure")]
    if fails:
        lines += ["", "## 失败点", ""]
        for e in fails:
            f = e["failure"]
            lines.append(f"- 段 {e['index']} [{e['from']},{e['to']})：停在帧 {f['frame']}；原因 `{f['reason']}`")
            if f.get("lua_log_tail"):
                lines.append(f"  - lua.log: `{f['lua_log_tail']}`")
    lines += ["", "## 已知限制", "",
              "- 段间靠 savestate 续跑；task-45 实测 600→1199 共 595 帧逐字节一致。",
              "- rust 腿是相对帧表（`abs = from + rel`）；比较按该映射对齐。",
              "- 段失败时后续段停止（下一段入口检查点由上一段产出）。"]
    (out / "coverage.md").write_text("\n".join(lines) + "\n", encoding="utf-8")


def cmd_selftest(_args) -> int:
    """差异检测自测：合成两份逐帧面，注入一字节差异，必须被指出。"""
    n = 60
    lo = 0x400
    base = {f: {"range": bytes((f + i) & 0xFF for i in range(16)), "Joy1Status": bytes([0x09])}
            for f in range(1, n + 1)}
    good = compare(dict(base), dict(base), lo, n)
    if good["first_divergence"] is not None or good["diverge_frames"] != 0:
        raise ReplayError(f"selftest 阴性样本误报: {good['first_divergence']}")
    bad = {f: {k: bytes(v) for k, v in d.items()} for f, d in base.items()}
    bad[33]["range"] = bytes(list(bad[33]["range"][:5]) + [bad[33]["range"][5] ^ 0x01]
                             + list(bad[33]["range"][6:]))
    rep = compare(dict(bad), dict(base), lo, n)
    fd = rep["first_divergence"]
    if not fd or fd["frame"] != 33 or fd["field"] != f"${lo + 5:04X}":
        raise ReplayError(f"selftest 未指出注入差异: {fd}")
    print(f"[selftest] OK 阴性 0 分歧；阳性首分歧 f{fd['frame']} {fd['field']} "
          f"emu={fd['emu']:02X} rust={fd['rust']:02X}")
    return 0


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description="TAS M1 双侧逐帧对拍（只出报告，不做判定）")
    sub = ap.add_subparsers(dest="cmd", required=True)
    r = sub.add_parser("replay")
    r.add_argument("--inputlog", required=True)
    r.add_argument("--frames", type=int, default=None,
                   help="回放帧数；省略=取 inputlog 全长")
    r.add_argument("--start", type=int, default=1,
                   help="比较窗口起点（1 基）；回放仍从第 1 帧跑，见文档「分段」节")
    r.add_argument("--report-only", dest="report_only", action="store_true",
                   help="只按现有 coverage.json 重写报告（不跑 emu/rust）")
    r.add_argument("--segments", type=int, default=0,
                   help="段长（帧）；>0 时走分段链：每段 emu 跑完顺带落下一段检查点，"
                        "rust 走相对帧表，产出 coverage.json/md")
    r.add_argument("--resume", action="store_true",
                   help="按已有 coverage.json 跳过已完成的段（pass 或 diverge），"
                        "从第一个 fail/缺条目继续，不重跑已完成段")
    r.add_argument("--chunk", type=int, default=0,
                   help="把逐帧表切成每 N 帧一个 parts/part-*.json（0=不切）")
    r.add_argument("--out", required=True)
    r.add_argument("--range", default="0400-04FF")
    r.add_argument("--tag", default="ens")
    r.add_argument("--name")
    r.add_argument("--capture", action="append", default=[])
    r.add_argument("--timeout", type=float, default=600)
    r.add_argument("--allow-rust-missing", action="store_true",
                   help="允许 rust 腿在 span/载房窗未 tick 的帧只记「不可比」而不 fail-loud")
    r.set_defaults(func=cmd_replay)
    st = sub.add_parser("selftest")
    st.set_defaults(func=cmd_selftest)
    args = ap.parse_args(argv)
    try:
        return args.func(args)
    except ReplayError as exc:
        print(f"[tas] FAIL: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
