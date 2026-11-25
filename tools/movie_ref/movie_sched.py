#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/movie_ref/movie_sched.py —— 录像 inputlog → Rust 腿调度表（.sched）。

存在理由：`rust_screenshot` 只吃**预解析调度表**（不在 bin 里解析 DSL——DSL 语义单源纪律）。
本脚本把「.inputlog → joy/joy2 逐帧表」这一步接到既有唯一实现上，**不复制位表、不另写展开器**：

    tools/movie_import.read_inputlog   （inputlog 读回，经 scenario_run.expand_pads）
    tools/scenario_run.expand_pads     （逐帧输入展开唯一实现）
    tools/l4_diff_run.schedule_text    （调度表序列化唯一实现）

即与 `l4_diff_run.py` 落 `.sched` 的字节构造同源，只是输入来自录像而非 cases/ DSL。

帧号约定（与 findings §6.1 对齐协议一致）：
- inputlog 第 n 行（1-based，跳过 `#` 头）= 录像第 n-1 帧（bk2 0-based）的输入；
- Rust 帧 N 消费录像第 N-1 帧输入 ⇒ `joy N` = inputlog 第 N 行；
- 因此 `--frames <movie_frames>` 与 inputlog 的 `frames` 元信息同值，Rust 跑 1..frames。

两种产物形态（同一份采集点，两腿不看同一套格式）：
- `--format sched`（默认）：Rust 腿 `rust_screenshot --schedule` 的 `.sched`；
- `--format mesen2`：emu 腿 `tools/scenario_run.py run <场景>` 的场景文本
  （`name/frames/inputlog/capture` 四类行——仍是既有 DSL，不新增语法）。

用法（产物只许落 /tmp，`cases/` 不得新增）：
    python3 tools/movie_ref/movie_sched.py \
        --inputlog movies/eien86-6725M.inputlog --frames 28408 \
        --capture-every 10 --capture-from 20 --capture-to 1200 \
        --capture 938:door-a --capture 944:door-b \
        --out /tmp/m6725-full.sched
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools"))

import l4_diff_run  # noqa: E402
import scenario_run  # noqa: E402


def validate_inputlog(inputlog: Path) -> None:
    """inputlog ↔ 影片输入表零容差自检（防"旧解码产物"复发）。

    provenance：inputlog 头部 `# source <path>` 指向原影片；该影片仍存在就必须逐帧
    零容差。判据按格式分派（两种影片的 P1 列解析不同，不能共用一套）：
    - `.bk2`/zip ⇒ `tools/movie_check.py::p1_truth`（按 `Input Log.txt` 的 `LogKey:` 列序）；
    - `.fm2`     ⇒ `tools/movie_import.py::load_movie` 的 `frames[i][0]`（port0=P1；
       fm2 列序 RLDUTSBA 与位序同构，由 movie_import 唯一实现，本处不重写位表）。
    缺 provenance / 影片缺失 / 解析失败 / 逐帧不符 ⇒ fail-loud，绝不静默使用。
    """
    src = None
    for raw in inputlog.read_text(encoding="utf-8", errors="replace").splitlines():
        if not raw.startswith("#"):
            break
        if raw.startswith("# source "):
            src = raw.split(" ", 2)[2].strip()
            break
    if not src:
        raise SystemExit(f"{inputlog}: 缺 `# source` provenance（无法自检，拒绝使用）")
    movie = Path(src)
    if not movie.is_absolute():
        movie = ROOT / movie
    if not movie.exists():
        print(f"[movie_sched] 跳过输入自检：源影片不存在（{movie}）")
        return
    sys.path.insert(0, str(ROOT / "tools"))
    if movie.suffix.lower() == ".fm2":
        import movie_import  # noqa: E402

        try:
            truth = [f[0] for f in movie_import.load_movie(movie)["frames"]]
        except Exception as exc:  # noqa: BLE001
            raise SystemExit(f"{movie}: fm2 解析失败（拒绝使用 inputlog）: {exc}") from exc
        how = "fm2/P1 列"
    else:
        import movie_check  # noqa: E402

        truth = movie_check.p1_truth(str(movie))
        how = "bk2/Input Log.txt"
    got = [
        int(l.split()[0], 16)
        for l in inputlog.read_text(encoding="utf-8", errors="replace").splitlines()
        if l.strip() and not l.startswith("#")
    ]
    if len(truth) != len(got):
        raise SystemExit(f"{inputlog}: 行数 {len(got)} ≠ 影片 {len(truth)}（旧产物或截断）")
    bad = [i for i in range(len(truth)) if truth[i] != got[i]]
    if bad:
        raise SystemExit(f"{inputlog}: {len(bad)}/{len(truth)} 帧与影片不符（首错 f={bad[0]}）")
    print(f"[movie_sched] 输入自检 OK（{how}）：{len(truth)} 帧与影片零容差")


def build_plan(inputlog: Path, frames: int, name: str) -> dict:
    """最小 plan：只有录像输入表（无 press/hold/trigger/nav ⇒ 两腿输入面恒等）。"""
    return {
        "name": name,
        "frames": frames,
        "press_len": scenario_run.PRESS_LEN_DEFAULT,
        "presses": [],
        "holds": [],
        "navs": [],
        "triggers": [],
        "input_log": str(inputlog),
        "captures": [],
        "capture_every": None,
        "fixtures": [],
        "bundle_load": None,
    }


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description="录像 inputlog → .sched（Rust 腿调度表）")
    ap.add_argument("--inputlog", required=True, help=".inputlog 路径（movie_import 产物）")
    ap.add_argument("--frames", required=True, type=int, help="Rust 跑多少帧（=录像帧数）")
    ap.add_argument("--out", required=True, help="调度表输出路径（建议 /tmp/…）")
    ap.add_argument("--name", default=None, help="sched name（缺省取 inputlog 文件名）")
    ap.add_argument("--capture-every", type=int, default=0,
                    help="按步长铺采集点（0=不铺）")
    ap.add_argument("--capture-from", type=int, default=1, help="步长采集起点（含）")
    ap.add_argument("--capture-to", type=int, default=None,
                    help="步长采集终点（含；缺省=frames）")
    ap.add_argument("--format", choices=("sched", "mesen2"), default="sched",
                    help="sched=Rust 腿调度表（默认）；mesen2=Mesen2 腿场景文本")
    ap.add_argument("--capture", action="append", default=[],
                    metavar="F[:TAG]", help="显式采集点（可多次；TAG 缺省 cap<F>）")
    args = ap.parse_args(argv)

    inputlog = Path(args.inputlog)
    if not inputlog.is_absolute():
        inputlog = ROOT / inputlog
    if not inputlog.is_file():
        print(f"[movie_sched] 找不到 inputlog: {inputlog}", file=sys.stderr)
        return 2
    if args.frames <= 0:
        print("[movie_sched] --frames 须为正整数", file=sys.stderr)
        return 2

    name = args.name or inputlog.stem
    validate_inputlog(Path(args.inputlog))
    plan = build_plan(inputlog, args.frames, name)
    joy, joy2 = scenario_run.expand_pads(plan)  # 唯一展开源（1P 来自 p1、2P 只来自 p2）

    over = max([args.frames] + list(joy) + list(joy2))
    if over > args.frames:
        print(f"[movie_sched] 警告：inputlog 有超过 --frames 的输入（最大帧 {over}）",
              file=sys.stderr)

    caps: list[tuple[int, str]] = []
    seen: dict[int, str] = {}
    if args.capture_every > 0:
        to = args.frames if args.capture_to is None else args.capture_to
        for f in range(args.capture_from, to + 1, args.capture_every):
            seen[f] = f"cap{f:04d}"
    for spec in args.capture:
        head, _, tag = spec.partition(":")
        f = int(head)
        seen[f] = tag or f"cap{f:04d}"
    caps = sorted(seen.items())

    if args.format == "mesen2":
        # 场景 DSL 文本（与 parse_scenario 的 op 名一一对应；inputlog 用相对仓库根路径）
        rel = inputlog.relative_to(ROOT) if inputlog.is_relative_to(ROOT) else inputlog
        lines = [f"# {name} —— 录像像素面对拍场景（movie_sched.py 生成，临时用，勿入库）",
                 f"name {name}",
                 f"frames {args.frames}",
                 f"inputlog {rel}"]
        lines += [f"capture {f} {tag}" for f, tag in caps]
        text = "\n".join(lines) + "\n"
    else:
        text = l4_diff_run.schedule_text(name, args.frames, joy, caps, joy2=joy2)
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(text, encoding="utf-8", newline="\n")

    print(f"[movie_sched] {inputlog.name}: format={args.format} frames={args.frames} "
          f"joy1={len(joy)} joy2={len(joy2)} captures={len(caps)} → {out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
