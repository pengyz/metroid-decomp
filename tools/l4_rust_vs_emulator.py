#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/l4_rust_vs_emulator.py —— L4 对拍（Rust vs 模拟器（真正的对拍））。

职责：
1. 运行模拟器（生成 golden（oracle/screens/））
2. 运行 Rust 实现（生成 shots/（Rust 截图））
3. 对拍（hash 对比（像素对比））
4. 给出结论（pass/fail（清晰结论））

用法：
    python3 tools/l4_rust_vs_emulator.py <scenario> [--timeout 秒]
    python3 tools/l4_rust_vs_emulator.py case-001-game-start-samus-interaction --timeout 60

输出：
    analysis/l4_rust_vs_emulator/<scenario>/result.json  # 对拍结论（pass/fail）
    analysis/l4_rust_vs_emulator/<scenario>/diff_report.txt  # 差异报告

设计原则（不过度）：
    - 工具与问题成正比（解决核心问题（Rust vs 模拟器（对拍）））
    - 不过度设计（恢复功能（记账（后续增加）））
"""
from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
GOLDEN = ROOT / "oracle" / "screens"
SHOTS = ROOT / "shots"
OUTPUT = ROOT / "analysis" / "l4_rust_vs_emulator"


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


def run_emulator(scenario: str, timeout: int) -> dict:
    """运行模拟器（生成 golden（oracle/screens/））"""
    cmd = ["python3", "tools/scenario_run.py", "run", scenario, "--timeout", str(timeout)]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout + 10)
    
    return {
        "scenario": scenario,
        "timeout": timeout,
        "returncode": result.returncode,
        "stdout": result.stdout,
        "stderr": result.stderr,
    }


def run_rust(scenario: str, timeout: int) -> dict:
    """运行 Rust 实现（生成 shots/（Rust 截图））"""
    cmd = ["cargo", "run", "--package", "game-core", "--bin", "rust_screenshot", "--", scenario, "--timeout", str(timeout)]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout + 10)
    
    return {
        "scenario": scenario,
        "timeout": timeout,
        "returncode": result.returncode,
        "stdout": result.stdout,
        "stderr": result.stderr,
    }


def rust_vs_emulator_diff(scenario: str, timeout: int) -> dict:
    """对拍（Rust vs 模拟器（真正的对拍））"""
    output_dir = OUTPUT / scenario
    output_dir.mkdir(parents=True, exist_ok=True)
    
    # 1. 运行模拟器（生成 golden）
    print(f"[*] 运行模拟器：{scenario}（timeout={timeout}）")
    emulator_result = run_emulator(scenario, timeout)
    
    # 2. 运行 Rust 实现（生成 shots）
    print(f"[*] 运行 Rust 实现：{scenario}（timeout={timeout}）")
    rust_result = run_rust(scenario, timeout)
    
    # 3. 对拍（hash 对比）
    print(f"[*] 对拍（Rust vs 模拟器（hash 对比））")
    golden_files = sorted(GOLDEN.glob(f"{scenario}_*.png"))
    
    comparison_results = []
    for golden_file in golden_files:
        shots_file = SHOTS / golden_file.name
        if shots_file.exists():
            result = compare_screenshots(golden_file, shots_file)
            comparison_results.append(result)
    
    # 4. 给出结论（pass/fail）
    total = len(comparison_results)
    passed = sum(1 for r in comparison_results if r["hash_match"])
    failed = total - passed
    
    conclusion = {
        "scenario": scenario,
        "timeout": timeout,
        "total_frames": total,
        "passed_frames": passed,
        "failed_frames": failed,
        "result": "PASS" if failed == 0 else "FAIL",
        "comparison_results": comparison_results,
    }
    
    # 5. 保存结果
    result_file = output_dir / "result.json"
    with open(result_file, "w") as f:
        json.dump(conclusion, f, indent=2)
    
    # 6. 生成差异报告
    report_file = output_dir / "diff_report.txt"
    with open(report_file, "w") as f:
        f.write(f"L4 对拍报告（Rust vs 模拟器）：{scenario}\n")
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
    parser = argparse.ArgumentParser(description="L4 对拍（Rust vs 模拟器（真正的对拍））")
    parser.add_argument("scenario", help="场景名称")
    parser.add_argument("--timeout", type=int, default=30, help="超时时间（秒（默认 30））")
    
    args = parser.parse_args()
    
    result = rust_vs_emulator_diff(args.scenario, args.timeout)
    
    if result["result"] == "PASS":
        print(f"[✓] L4 对拍通过（{result['passed_frames']}/{result['total_frames']}）")
        sys.exit(0)
    else:
        print(f"[✗] L4 对拍失败（{result['failed_frames']} 帧不一致）")
        sys.exit(1)


if __name__ == "__main__":
    main()
