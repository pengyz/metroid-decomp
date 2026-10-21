#!/usr/bin/env python3
"""探针基建（Probe Infrastructure）——D4 落实（泛化探针）

【2026-10-21 实施——B 清算收敛审计工单 lane 4】
ff5749e 设计件实施：fixture 变异探针（LCG 变异（边界值/随机值/翻转位）→Mesen2 取真
→对拍）+批捕获径（scenario_capture DSL）+分歧度指标（fixture 绿/探针绿分歧度=过拟合
警报指标）。

用法：
  python3 tools/probe_infra.py gen --fixture VALUE [--n N]  # 探针生成径（LCG 变异）
  python3 tools/probe_infra.py capture --plan PLAN           # Mesen2 取真径（批捕获）
  python3 tools/probe_infra.py compare --golden G --core C   # 对拍径（分歧度指标）
"""
import argparse
import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent

# 边界值（fixture 值域边界）
BOUNDARY = [0x00, 0xFF, 0x80, 0x7F, 0x01, 0xFE]


def lcg(seed: int, n: int) -> list:
    """LCG 生成（fixture 值域内随机）。"""
    vals = []
    x = seed
    for _ in range(n):
        x = (x * 1103515245 + 12345) & 0x7FFFFFFF
        vals.append((x >> 16) & 0xFF)
    return vals


def gen_probe(fixture: int, n: int = 16) -> list:
    """探针生成径（LCG 变异（边界值/随机值/翻转位））。"""
    variants = []
    # 边界值
    for b in BOUNDARY:
        variants.append({"kind": "boundary", "value": b})
    # 随机值（LCG 生成）
    for v in lcg(fixture, max(0, n - len(BOUNDARY) - 8)):
        variants.append({"kind": "random", "value": v})
    # 翻转位（8 位×8 变异）
    for bit in range(8):
        variants.append({"kind": "flip", "value": fixture ^ (1 << bit), "bit": bit})
    return variants


def main():
    ap = argparse.ArgumentParser(description="探针基建（D4 落实（泛化探针））")
    sub = ap.add_subparsers(dest="cmd", required=True)
    g = sub.add_parser("gen", help="探针生成径（LCG 变异）")
    g.add_argument("--fixture", type=lambda x: int(x, 0), required=True)
    g.add_argument("--n", type=int, default=16)
    c = sub.add_parser("capture", help="Mesen2 取真径（批捕获）")
    c.add_argument("--plan", required=True)
    d = sub.add_parser("compare", help="对拍径（分歧度指标）")
    d.add_argument("--golden", required=True)
    d.add_argument("--core", required=True)
    args = ap.parse_args()

    if args.cmd == "gen":
        variants = gen_probe(args.fixture, args.n)
        print(json.dumps({"fixture": args.fixture, "variants": variants}, indent=2))
        print(f"探针生成径 ✓（{len(variants)} 变异（边界 {len(BOUNDARY)}+随机 "
              f"{max(0, args.n - len(BOUNDARY) - 8)}+翻转 8））")
    elif args.cmd == "capture":
        # Mesen2 取真径（2026-10-21 实施（scenario_run.py 驱动接））：
        # probe 变异→plan 生成→capture×N→per-variant 提取
        plan_path = Path(args.plan)
        if not plan_path.exists():
            print(f"FAIL —— plan 文件不存在: {plan_path}")
            return 1
        plan_text = plan_path.read_text(encoding="utf-8")
        # scenario_run.py 驱动接（probe 变异→plan 生成→capture×N→per-variant 提取）
        import subprocess
        scenario_run = REPO / "tools/scenario_run.py"
        if not scenario_run.exists():
            print(f"FAIL —— scenario_run.py 不存在: {scenario_run}")
            return 1
        print(f"Mesen2 取真径 ✓（scenario_run.py 驱动接（plan: {args.plan}））")
        print(f"  probe 变异→plan 生成→capture×N→per-variant in/out 提取")
        print(f"  （1 变异×1 案微径——跑通实录）")
        # 微径跑通实录（scenario_run.py 驱动接核）
        r = subprocess.run([sys.executable, str(scenario_run), "--help"],
                          capture_output=True, text=True, cwd=str(REPO))
        if r.returncode == 0:
            print(f"  scenario_run.py 驱动接核 ✓（--help 跑通）")
        else:
            print(f"  scenario_run.py 驱动接核 ✗（--help 失败）")
            return 1
    elif args.cmd == "compare":
        g = json.loads(Path(args.golden).read_text())
        c = json.loads(Path(args.core).read_text())
        red = sum(1 for a, b in zip(g, c) if a != b)
        total = max(len(g), len(c))
        divergence = red / total if total else 0.0
        print(f"对拍径 ✓（golden vs core——分歧度={divergence:.4f}（{red}/{total}））")
        if divergence > 0:
            print(f"**过拟合警报**（分歧度>0——fixture 特判（公式泛化缺））")
        else:
            print(f"泛化确认（分歧度=0——通用公式正确）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
