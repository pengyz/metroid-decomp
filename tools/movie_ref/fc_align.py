"""按游戏时间（FrameCount）配对两腿，并给出「FC 对齐后」的像素差。

背景：原机在主循环单次迭代超出帧预算时会丢显示帧（`$002D FrameCount` 推进慢于视频帧），
Rust 无 cycle 模型因而永不丢帧 ⇒ 按视频帧号比像素必然错位（三区 7 房 11–18K px）。

判据只借游戏时间、不借像素：
  1. 配对帧 g：令 `(FC, ScrollX)` 在 emu 轨迹上与 rust 该帧相同；必须**唯一**，否则 fail-closed。
  2. 相位：`emu(g-1) == emu(g)`（g 是重复帧对的第二帧＝该迭代超预算、显示面沿用上一帧）
     ⇒ 取 **live**（工作后活体）产物；载房窗内一律取 **seam**（`room_load_phase==1` 时
     live 会画出「活体已推进、显示未推进」的错帧）；其余取 **seam**。
     该规律在 Kraid 31 的 21 帧上 9/10 命中、唯一例外正是载房窗（见
     docs/superpowers/plans/2026-11-23-fc-align-p1-implementation.md P1.0f）。
  3. 位移量**只来自 `$002D`**：本模块从不做 `min(diff)` / ±1 搜索 / 像素平移。

用法（一次性核对某房）：
    python3 -m tools.movie_ref.fc_align --case case-x --rust-state /tmp/s.state.jsonl \
        --seam-dir shots_rust --live-dir /tmp/live --emu-dir shots --out /tmp/fc.json
"""
from __future__ import annotations

import argparse
import glob
import json
from dataclasses import dataclass
from pathlib import Path

TOOLS = Path(__file__).resolve().parent.parent.parent
SHOTS = TOOLS / "shots"
SHOTS_RUST = TOOLS / "shots_rust"
TRACES = TOOLS / "analysis" / "traces"

FC_ADDR = 0x2D

# `--state-out` 的 `rl` 顶层键 → 该 rust 帧是否落在载房窗（相位 1）。见 rust_screenshot::state_row。
ROOM_LOAD: dict[int, int] = {}
SCROLL_X_ADDR = 0xFD


@dataclass(frozen=True)
class State:
    """一帧的判定用状态（只两列，够配对）。"""

    frame: int
    fc: int
    scroll_x: int

    def key(self) -> tuple[int, int]:
        return (self.fc, self.scroll_x)


def parse_emu_trace(path: Path, fc_label: str = "FC", sx_label: str = "ScrollX") -> dict[int, State]:
    """读 `analysis/traces/<case>.frames.jsonl`（每行 {"f":N,"w":{标签:值}}）。"""
    out: dict[int, State] = {}
    with path.open() as fh:
        for line in fh:
            line = line.strip()
            if not line:
                continue
            row = json.loads(line)
            w = row.get("w") or {}
            if fc_label not in w or sx_label not in w:
                continue
            out[int(row["f"])] = State(int(row["f"]), int(w[fc_label]), int(w[sx_label]))
    return out


def parse_rust_state(path: Path) -> dict[int, State]:
    """读 `rust_screenshot --state-out`：每行 `{"f":N,"w":{"zp":"<512 hex 字符>",…}}`。

    `zp` 是 `$0000-$00FF` 的 hex 串，`$2D` 的字节在 `zp[0x2D*2:0x2D*2+2]`。
    """
    out: dict[int, State] = {}
    with path.open() as fh:
        for line in fh:
            line = line.strip()
            if not line:
                continue
            row = json.loads(line)
            zp = (row.get("w") or {}).get("zp")
            if not zp:
                continue
            if len(zp) != 512:
                raise ValueError(f"zp 长度应为 512（$0000-$00FF 全零页），实得 {len(zp)}，帧 {row.get('f')}")
            fc = int(zp[FC_ADDR * 2 : FC_ADDR * 2 + 2], 16)
            sx = int(zp[SCROLL_X_ADDR * 2 : SCROLL_X_ADDR * 2 + 2], 16)
            st = State(int(row["f"]), fc, sx)
            out[st.frame] = st
            if row.get("rl") is not None:
                ROOM_LOAD[st.frame] = int(row["rl"])
    return out


def is_duplicate_second(g: int, emu_png: dict[int, Path]) -> bool:
    """g 是否为「重复帧对的第二帧」：emu 的 g 帧与 g-1 帧**画面完全相同**。

    重复帧对＝该次主循环迭代超出帧预算，显示面沿用上一帧（`FrameCount` 仍会 +1，
    所以**不能**用 `(FC,ScrollX)` 判重——实测 emu 1725=(164,240) 与 1726=(165,240)
    FC 已推进而画面逐字节相同，见 P1.0f 表）。

    这里用的是 **emu 自己的相邻显示帧**是否相同，不是「在多个配对里挑像素最小的」——
    不参与配对，只决定拿 rust 的哪一份产物去比，因此不构成按像素挑对齐。
    """
    a, b = emu_png.get(g - 1), emu_png.get(g)
    if a is None or b is None:
        return False
    return a.read_bytes() == b.read_bytes()


def pair_frame(emu: dict[int, State], rust: State, horizon: int = 40) -> int | None:
    """等 `(FC, ScrollX)` 唯一确定 emu 帧 g（g >= rust.frame）。

    返回 None 表示无候选；抛 ValueError 表示多候选（fail-closed，不做「取最小」）。
    """
    cands = [
        g
        for g in range(rust.frame, rust.frame + horizon + 1)
        if (s := emu.get(g)) is not None and s.key() == rust.key()
    ]
    if not cands:
        return None
    if len(cands) > 1:
        raise ValueError(f"多候选配对 c={rust.frame} key={rust.key()} cands={cands}")
    return cands[0]


def choose_phase(g: int, emu_png: dict[int, Path], room_load: bool = False) -> str:
    """相位三分支（见模块 docstring 规则 2）。"""
    if room_load:
        return "seam"
    return "live" if is_duplicate_second(g, emu_png) else "seam"


def _png(dir_path: Path, case: str, frame: int, tag: str) -> Path | None:
    """按**精确 tag** 取产物；同帧多 tag（`f{c}`/`prev{c}`/`prevend` 历史上会撞帧）不得歧义。

    只认判定采点 `f{c}`：`pre`/`prev*`/`prevend` 是参考帧，`room_sweep.py:792-793` 明确
    不参与判定；早期版本用 `tag=None` 的 glob 会把它们当判定点，导致 `ok` 全来自参考帧。
    """
    hits = sorted(
        p
        for p in glob.glob(str(dir_path / f"{case}_f{frame:05d}_{tag}.png"))
        if ".diff." not in p
    )
    if not hits:
        return None
    if len(hits) > 1:
        raise ValueError(f"同帧多产物，tag 歧义：{hits}")
    return Path(hits[0])


def diff_pixels(a: Path, b: Path) -> int:
    import numpy as np
    from PIL import Image

    x = np.asarray(Image.open(a).convert("RGB")).astype(np.int16)
    y = np.asarray(Image.open(b).convert("RGB")).astype(np.int16)
    if x.shape != y.shape:
        raise ValueError(f"尺寸不一致 {a.name} {x.shape} vs {b.name} {y.shape}")
    return int(np.any(x != y, axis=2).sum())


def align_case(
    case: str,
    rust_state: dict[int, State],
    emu: dict[int, State],
    seam_dir: Path,
    live_dir: Path,
    emu_dir: Path,
    room_load_frames: set[int] | None = None,
    horizon: int = 40,
) -> dict:
    """对一房逐帧算「FC 对齐后的像素差」。room_load_frames 里的 rust 帧按载房窗处理。"""
    load = set(room_load_frames or ()) | {f for f, ph in ROOM_LOAD.items() if ph == 1}
    # 目录一次索引（按 `_fNNNNN_<tag>.png` 解析），避免逐帧 glob（实测占 62.8s 中的绝大部分）。
    emu_png: dict[int, Path] = {}
    seam_png: dict[int, Path] = {}
    for p in sorted(emu_dir.glob(f"{case}_f*.png")):
        if ".diff." in p.name:
            continue
        rest = p.name[len(case) + 3 :]
        digits, _, tagp = rest.partition("_")
        if digits.isdigit() and tagp.startswith("f"):
            emu_png[int(digits)] = p
    for p in sorted(seam_dir.glob(f"{case}_f*.png")):
        if ".diff." in p.name:
            continue
        rest = p.name[len(case) + 3 :]
        digits, _, tagp = rest.partition("_")
        if digits.isdigit() and tagp.startswith("f"):
            seam_png[int(digits)] = p
    live_png: dict[int, Path] = {}
    for p in sorted(live_dir.glob(f"{case}_f*.png")):
        if ".diff." in p.name:
            continue
        rest = p.name[len(case) + 3 :]
        digits, _, tagp = rest.partition("_")
        if digits.isdigit() and tagp.startswith("f"):
            live_png[int(digits)] = p
    rows = []
    for c in sorted(rust_state):
        if c not in seam_png:
            continue  # 该帧不是判定采点（只认 f{c}）
        rs = rust_state[c]
        try:
            g = pair_frame(emu, rs, horizon)
        except ValueError as exc:
            rows.append({"c": c, "g": None, "phase": None, "diff": None, "error": str(exc)})
            continue
        if g is None:
            rows.append({"c": c, "g": None, "phase": None, "diff": None, "error": "no_candidate"})
            continue
        phase = choose_phase(g, emu_png, room_load=c in load)
        ep = emu_png.get(g)
        rp = (seam_png if phase == "seam" else live_png).get(c)
        if ep is None or rp is None:
            rows.append({"c": c, "g": g, "phase": phase, "diff": None, "error": "missing_png"})
            continue
        rows.append({"c": c, "g": g, "phase": phase, "diff": diff_pixels(ep, rp), "error": None})
    diffs = [r["diff"] for r in rows if r["diff"] is not None]
    errors = [r for r in rows if r["error"]]
    return {
        "case": case,
        "frames": len(rows),
        "ok": sum(1 for d in diffs if d == 0),
        "nonzero": sum(1 for d in diffs if d),
        "errors": len(errors),
        "max_diff": max(diffs) if diffs else None,
        "rows": rows,
    }


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description="FC 对齐（游戏时间配对）后的像素差")
    ap.add_argument("--case", required=True)
    ap.add_argument("--rust-state", required=True)
    ap.add_argument("--seam-dir", default=str(SHOTS_RUST))
    ap.add_argument("--live-dir", required=True)
    ap.add_argument("--emu-dir", default=str(SHOTS))
    ap.add_argument("--trace", default=None, help="默认 analysis/traces/<case>.frames.jsonl")
    ap.add_argument("--room-load", default="", help="逗号分隔的 rust 帧号（载房窗内，强制 seam）")
    ap.add_argument("--out", default=None)
    args = ap.parse_args(argv)

    trace = Path(args.trace) if args.trace else TRACES / f"{args.case}.frames.jsonl"
    emu = parse_emu_trace(trace)
    if not emu:
        raise SystemExit(f"emu 轨迹为空或缺 FC/ScrollX 列：{trace}（watch 里要加 002D/00FD）")
    rust = parse_rust_state(Path(args.rust_state))
    load = {int(x) for x in args.room_load.split(",") if x.strip()}
    res = align_case(
        args.case,
        rust,
        emu,
        Path(args.seam_dir),
        Path(args.live_dir),
        Path(args.emu_dir),
        room_load_frames=load,
    )
    text = json.dumps(res, ensure_ascii=False, indent=1)
    if args.out:
        Path(args.out).write_text(text + "\n")
    print(
        f"{res['case']}: frames={res['frames']} ok={res['ok']} nonzero={res['nonzero']} "
        f"errors={res['errors']} max={res['max_diff']}"
    )
    for r in res["rows"]:
        if r["error"]:
            print(f"  c={r['c']} g={r['g']} ERROR {r['error']}")
        elif r["diff"]:
            print(f"  c={r['c']} g={r['g']} phase={r['phase']} diff={r['diff']}")
    # 判定面「看得见全部失配」：有任何 error 或非零差都以非零码退出，避免恒 0 静默通过。
    return 1 if (res["errors"] or res["nonzero"]) else 0


if __name__ == "__main__":
    raise SystemExit(main())
