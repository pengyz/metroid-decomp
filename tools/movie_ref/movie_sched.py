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
