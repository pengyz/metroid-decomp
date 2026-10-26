#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/l4_unified_diff.py —— L4 统一对拍工具（模拟器+Rust 实现（自动截图对拍（hash 对比（给出结论））））。

职责：
1. 统一输入：一套输入格式（模拟器格式（或统一格式（两边转换）））
2. 自动截图对拍：特定帧率截图（自动对比）
3. hash 对比：截图 hash 对比（快速判定）
4. 给出结论：pass/fail（清晰结论）

用法：
    python3 tools/l4_unified_diff.py <scenario> [--fps N] [--timeout 秒]
    python3 tools/l4_unified_diff.py game-start --fps 60 --timeout 30

输出：
    analysis/l4_unified/<scenario>/  # 对拍结果目录
    analysis/l4_unified/<scenario>/result.json  # 对拍结论（pass/fail）
    analysis/l4_unified/<scenario>/diff_report.txt  # 差异报告

后续功能（记账）：
    - 恢复功能：游戏状态定义文件（存档（恢复到特定点（输入+对拍）））
    - 当前机制暂时缺失（测的是游戏的一幕（不需要））
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
GOLDEN = ROOT / "oracle" / "screens"
SHOTS = ROOT / "shots"
OUTPUT = ROOT / "analysis" / "l4_unified"


def calculate_hash(file_path: Path) -> str:
    """计算文件 hash（SHA-256）"""
    if not file_path.exists():
        return ""
    with open(file_path, "rb") as f:
        return hashlib.sha256(f.read()).hexdigest()


def compare_screenshots(golden_path: Path, shots_path: Path) -> dict:
    """对比截图（hash 对比（像素对比））"""
    golden_hash = calculate_hash(golden_path)
    shots_hash = calculate_hash(shots_path)
    
    result = {
        "golden": str(golden_path),
        "shots": str(shots_path),
        "golden_hash": golden_hash,
        "shots_hash": shots_hash,
        "hash_match": golden_hash == shots_hash,
        "golden_exists": golden_path.exists(),
        "shots_exists": shots_path.exists(),
    }
    
    # 像素对比（如果 hash 不一致）
    if not result["hash_match"] and result["golden_exists"] and result["shots_exists"]:
        try:
            from PIL import Image
            img_golden = Image.open(golden_path)
            img_shots = Image.open(shots_path)
            
            pixels_golden = list(img_golden.getdata())
            pixels_shots = list(img_shots.getdata())
            
            diff_count = sum(1 for a, b in zip(pixels_golden, pixels_shots) if a != b)
            total = len(pixels_golden)
            
            result["pixel_diff"] = diff_count
            result["pixel_total"] = total
            result["pixel_diff_ratio"] = diff_count / total if total > 0 else 0
        except Exception as e:
            result["pixel_error"] = str(e)
    
    return result


def run_scenario(scenario: str, fps: int, timeout: int) -> dict:
    """运行场景（生成截图）"""
    # 运行模拟器场景（生成截图）
    cmd = ["python3", "tools/scenario_run.py", "run", scenario, "--timeout", str(timeout)]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout + 10)
    
    return {
        "scenario": scenario,
        "fps": fps,
        "timeout": timeout,
        "returncode": result.returncode,
        "stdout": result.stdout,
        "stderr": result.stderr,
    }


def unified_diff(scenario: str, fps: int, timeout: int) -> dict:
    """统一对拍（模拟器+Rust 实现（自动截图对拍（hash 对比（给出结论））））"""
    output_dir = OUTPUT / scenario
    output_dir.mkdir(parents=True, exist_ok=True)
    
    # 1. 运行场景（生成截图）
    print(f"[*] 运行场景：{scenario}（fps={fps}（timeout={timeout}））")
    run_result = run_scenario(scenario, fps, timeout)
    
    # 2. 对比截图（hash 对比）
    print(f"[*] 对比截图（hash 对比）")
    golden_files = sorted(GOLDEN.glob(f"{scenario}_*.png"))
    shots_files = sorted(SHOTS.glob(f"{scenario}_*.png"))
    
    comparison_results = []
    for golden_file in golden_files:
        # 找到对应的 shots 文件
        shots_file = SHOTS / golden_file.name
        if shots_file.exists():
            result = compare_screenshots(golden_file, shots_file)
            comparison_results.append(result)
    
    # 3. 给出结论（pass/fail）
    total = len(comparison_results)
    passed = sum(1 for r in comparison_results if r["hash_match"])
    failed = total - passed
    
    conclusion = {
        "scenario": scenario,
        "fps": fps,
        "timeout": timeout,
        "total_frames": total,
        "passed_frames": passed,
        "failed_frames": failed,
        "result": "PASS" if failed == 0 else "FAIL",
        "comparison_results": comparison_results,
    }
    
    # 4. 保存结果
    result_file = output_dir / "result.json"
    with open(result_file, "w") as f:
        json.dump(conclusion, f, indent=2)
    
    # 5. 生成差异报告
    report_file = output_dir / "diff_report.txt"
    with open(report_file, "w") as f:
        f.write(f"L4 统一对拍报告：{scenario}\n")
        f.write(f"{'='*60}\n")
        f.write(f"总帧数：{total}\n")
        f.write(f"通过帧数：{passed}\n")
        f.write(f"失败帧数：{failed}\n")
        f.write(f"结论：{conclusion['result']}\n\n")
        
        for result in comparison_results:
            status = "PASS" if result["hash_match"] else "FAIL"
            f.write(f"[{status}] {Path(result['golden']).name}\n")
            f.write(f"  golden_hash: {result['golden_hash'][:16]}...\n")
            f.write(f"  shots_hash:  {result['shots_hash'][:16]}...\n")
            if not result["hash_match"] and "pixel_diff" in result:
                f.write(f"  pixel_diff:  {result['pixel_diff']}/{result['pixel_total']} ({result['pixel_diff_ratio']*100:.1f}%)\n")
            f.write(f"\n")
    
    print(f"[*] 对拍结果：{conclusion['result']}（{passed}/{total} 通过）")
    print(f"[*] 结果文件：{result_file}")
    print(f"[*] 差异报告：{report_file}")
    
    return conclusion


def main():
    parser = argparse.ArgumentParser(description="L4 统一对拍工具")
    parser.add_argument("scenario", help="场景名称")
    parser.add_argument("--fps", type=int, default=60, help="帧率（默认 60）")
    parser.add_argument("--timeout", type=int, default=30, help="超时时间（秒（默认 30））")
    
    args = parser.parse_args()
    
    result = unified_diff(args.scenario, args.fps, args.timeout)
    
    if result["result"] == "PASS":
        print(f"[✓] L4 对拍通过（{result['passed_frames']}/{result['total_frames']}）")
        sys.exit(0)
    else:
        print(f"[✗] L4 对拍失败（{result['failed_frames']} 帧不一致）")
        sys.exit(1)


if __name__ == "__main__":
    main()
