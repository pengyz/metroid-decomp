#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/l4_conclusion_layer.py —— L4 结论层（pass/fail（清晰结论））。

职责：
1. 汇总对拍结果（hash 对比（像素对比））
2. 给出结论（pass/fail（清晰结论））
3. 保存结果（result.json（diff_report.txt））

用法：
    python3 tools/l4_conclusion_layer.py <scenario> <comparison_results.json>
    python3 tools/l4_conclusion_layer.py game-start analysis/l4_auto/game-start/comparison_results.json

输出：
    analysis/l4_auto/<scenario>/result.json  # 对拍结论（pass/fail）
    analysis/l4_auto/<scenario>/diff_report.txt  # 差异报告

设计原则（不过度）：
    - 工具与问题成正比（解决核心问题（汇总结果（给出结论）））
    - 不过度设计（恢复功能（记账（后续增加）））
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Dict, List

ROOT = Path(__file__).resolve().parent.parent
OUTPUT = ROOT / "analysis" / "l4_auto"


def generate_conclusion(scenario: str, comparison_results: List[Dict]) -> Dict:
    """生成结论（pass/fail（清晰结论））"""
    total = len(comparison_results)
    passed = sum(1 for r in comparison_results if r["hash_match"])
    failed = total - passed
    
    conclusion = {
        "scenario": scenario,
        "total_frames": total,
        "passed_frames": passed,
        "failed_frames": failed,
        "result": "PASS" if failed == 0 else "FAIL",
        "comparison_results": comparison_results,
    }
    
    return conclusion


def save_conclusion(scenario: str, conclusion: Dict) -> None:
    """保存结论（result.json（diff_report.txt））"""
    output_dir = OUTPUT / scenario
    output_dir.mkdir(parents=True, exist_ok=True)
    
    # 保存 result.json
    result_file = output_dir / "result.json"
    with open(result_file, "w") as f:
        json.dump(conclusion, f, indent=2)
    
    # 生成 diff_report.txt
    report_file = output_dir / "diff_report.txt"
    with open(report_file, "w") as f:
        f.write(f"L4 自动对拍报告：{scenario}\n")
        f.write(f"{'='*60}\n")
        f.write(f"总帧数：{conclusion['total_frames']}\n")
        f.write(f"通过帧数：{conclusion['passed_frames']}\n")
        f.write(f"失败帧数：{conclusion['failed_frames']}\n")
        f.write(f"结论：{conclusion['result']}\n\n")
        
        for result in conclusion["comparison_results"]:
            status = "PASS" if result["hash_match"] else "FAIL"
            f.write(f"[{status}] {Path(result['golden']).name}\n")
            f.write(f"  golden_hash: {result['golden_hash'][:16]}...\n")
            f.write(f"  shots_hash:  {result['shots_hash'][:16]}...\n")
            if not result["hash_match"] and "pixel_diff" in result:
                f.write(f"  pixel_diff:  {result['pixel_diff']}/{result['pixel_total']} ({result['pixel_diff_ratio']*100:.1f}%)\n")
            f.write(f"\n")
    
    print(f"[*] 结果文件：{result_file}")
    print(f"[*] 差异报告：{report_file}")


def main():
    parser = argparse.ArgumentParser(description="L4 结论层（pass/fail（清晰结论））")
    parser.add_argument("scenario", help="场景名称")
    parser.add_argument("comparison_results", help="对拍结果 JSON 文件路径")
    
    args = parser.parse_args()
    
    # 读取对拍结果
    with open(args.comparison_results, "r") as f:
        comparison_results = json.load(f)
    
    # 生成结论
    conclusion = generate_conclusion(args.scenario, comparison_results)
    
    # 保存结论
    save_conclusion(args.scenario, conclusion)
    
    # 输出结论
    print(f"\n[*] 对拍结论：{conclusion['result']}")
    print(f"[*] 总帧数：{conclusion['total_frames']}")
    print(f"[*] 通过帧数：{conclusion['passed_frames']}")
    print(f"[*] 失败帧数：{conclusion['failed_frames']}")


if __name__ == "__main__":
    main()
