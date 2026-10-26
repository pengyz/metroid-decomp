#!/usr/bin/env python3
"""baseline 上调工具（rust_test_count 584→587（PENDING（现值高于基线（+3 测试））））——user ruling（工具建（tools/（tool 缺（待建））））

Task 6 扩展：L4 逐采集点 ratchet 基线上调面
    python3 tools/baseline_update.py l4 --reason "…" [--matrix shots_rust/matrix.json]
        # 唯一写入口 data/status/l4_ratchet_baseline.json（gates/cli.py l4 门禁消费）
        # 只升不降（match 集合只增不减 + blocked 只减不增——非聚合率）+ 强制 --reason 留痕
        # 判定单源=gates/cli.py l4_ratchet_violations（工具不持判定权——importlib 引用）
"""
import importlib.util, json, sys
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BASELINE = Path("data/status/baselines/rust_test_count.json")
L4_RATCHET = Path("data/status/l4_ratchet_baseline.json")


def update(new_count: int) -> None:
    old = json.loads(BASELINE.read_text()) if BASELINE.exists() else {"rust_test_count": 584}
    old_count = old.get("rust_test_count", 584)
    if new_count < old_count:
        print(f"ERR: 基线只升不降（{old_count}→{new_count}）——拒绝")
        sys.exit(1)
    old["rust_test_count"] = new_count
    BASELINE.write_text(json.dumps(old, indent=2) + "\n")
    print(f"baseline 上调：rust_test_count {old_count}→{new_count}（PENDING（现值高于基线））")


def _gates_cli():
    """gates/cli.py 装载（判定逻辑单源=门禁侧——本工具不复制 ratchet 判定，AGENTS：工具自身
    不持有判定权）。"""
    spec = importlib.util.spec_from_file_location(
        "gates_cli_baseline_update", ROOT / "gates" / "cli.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def l4_update(matrix_path: Path, baseline_path: Path, reason: str) -> dict:
    """L4 逐采集点 ratchet 基线唯一写入口（Task 6）：只升不降 + 强制 reason 留痕。

    - 矩阵必须是 run-all 产物（generated_by 契约，锚不可信拒更新）；
    - 旧基线存在 ⇒ 先过 l4_ratchet_violations（单源=gates/cli.py）：任何回退
      （match 缩水/新增 blocked/rom 锚漂移）⇒ 拒绝；缺旧基线=genesis 引导；
    - states 重锚**当前矩阵逐采集点**——工具授权的更新永不丢 match 锚（缩水即拒），
      防「删采集点逃逸」；人工重定义采集点须在 reason 写明确认无回退的证据；
    - history 追加 {ts, reason, match/diff/blocked/cells_total} 留痕（基线说明面）。
    """
    reason = (reason or "").strip()
    if not reason:
        raise ValueError("基线更新必须 --reason 留说明"
                         "（AGENTS.md §2：更新基线需写明『确认无回退』的证据）")
    gates_cli = _gates_cli()
    matrix_path = Path(matrix_path)
    baseline_path = Path(baseline_path)
    matrix = json.loads(matrix_path.read_text(encoding="utf-8"))
    if (not isinstance(matrix, dict)
            or matrix.get("generated_by") != "tools/l4_diff_run.py run-all"):
        raise ValueError(f"{matrix_path} 非 run-all 矩阵产物（拒更新——锚不可信）")
    old = None
    if baseline_path.is_file():
        old = gates_cli.load_l4_ratchet_baseline(baseline_path)
        viol = gates_cli.l4_ratchet_violations(old, matrix)
        if viol:
            raise gates_cli.RatchetError(
                "l4 ratchet 基线拒绝下调（只升不降）: " + "；".join(viol))
    states = gates_cli.l4_matrix_states(matrix)
    row_scen = {str(r.get("case")): r.get("scenario_sha256")
                for r in (matrix.get("rows") or [])}
    doc = {
        "schema": "l4-ratchet/v1",
        "doc": ("L4 逐采集点 ratchet 锚（Task 6）——match 集合只增不减 + blocked 只减不增"
                "（**非**一致率百分比：9/10→9/10 逐采集点换坏必触发）。消费面=gates/cli.py "
                "gate_l4（run-all 矩阵 vs 本锚逐采集点对账）；唯一写入口=tools/baseline_update.py "
                "l4（只升不降 + 强制 --reason）；diff=现状参考（改善 diff→match 欢迎，"
                "→blocked 由『新增 blocked』规则拦截）；白名单例外=gates/visual_whitelist.json。"),
        "rom_sha256": matrix.get("rom_sha256"),
        "cases": {case: {"scenario_sha256": row_scen.get(case), "states": dict(st)}
                  for case, st in states.items()},
        "history": list((old or {}).get("history", [])),
    }
    record = {
        "ts": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        "reason": reason,
        "match": matrix.get("match"), "diff": matrix.get("diff"),
        "blocked": matrix.get("blocked"), "cells_total": matrix.get("cells_total"),
    }
    doc["history"].append(record)
    baseline_path.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n",
                             encoding="utf-8", newline="\n")
    return record


def _main() -> None:
    if len(sys.argv) > 1 and sys.argv[1] == "l4":
        import argparse
        ap = argparse.ArgumentParser(
            prog="baseline_update.py l4",
            description="L4 逐采集点 ratchet 基线上调（只升不降 + 强制 --reason）")
        ap.add_argument("--reason", required=True, help="必须写明『确认无回退』的证据")
        ap.add_argument("--matrix", default="shots_rust/matrix.json")
        ap.add_argument("--baseline", default=str(L4_RATCHET))
        args = ap.parse_args(sys.argv[2:])
        try:
            rec = l4_update(Path(args.matrix), Path(args.baseline), args.reason)
        except Exception as exc:  # noqa: BLE001 —— 拒绝原因显式打印，禁裸 traceback
            print(f"L4 基线更新拒绝: {exc}")
            sys.exit(1)
        print("L4 RATCHET UPDATE: " + json.dumps(rec, ensure_ascii=False))
        print(f"（写入 {args.baseline}；commit 必须含 Evidence/Confidence——data/status/ 敏感路径）")
        return
    update(int(sys.argv[1]) if len(sys.argv) > 1 else 587)


if __name__ == "__main__":
    _main()
