#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/l4_diff_layer.py —— L4 对拍层（hash 对比（像素对比））。

职责：
1. hash 对比：截图 hash 对比（快速判定）
2. 像素对比：像素差异统计（差异比例（差异分布））
3. 给出结论：pass/fail（清晰结论）

用法：
    python3 tools/l4_diff_layer.py <golden_path> <shots_path>
    python3 tools/l4_diff_layer.py oracle/screens/game-start_f00300_title.png shots/game-start_f00300_title.png

输出：
    对拍结果（pass/fail（hash 对比（差异统计）））

设计原则（不过度）：
    - 工具与问题成正比（解决核心问题（hash 对比（像素对比）））
    - 不过度设计（恢复功能（记账（后续增加）））
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Dict


def calculate_hash(file_path: Path) -> str:
    """计算文件 hash（SHA-256）"""
    if not file_path.exists():
        return ""
    with open(file_path, "rb") as f:
        return hashlib.sha256(f.read()).hexdigest()


def compare_screenshots(golden_path: Path, shots_path: Path) -> Dict:
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


def main():
    parser = argparse.ArgumentParser(description="L4 对拍层（hash 对比（像素对比））")
    parser.add_argument("golden_path", help="金面截图路径（oracle/screens/）")
    parser.add_argument("shots_path", help="复采截图路径（shots/）")
    
    args = parser.parse_args()
    
    golden_path = Path(args.golden_path)
    shots_path = Path(args.shots_path)
    
    result = compare_screenshots(golden_path, shots_path)
    
    print(f"对拍结果：")
    print(f"  golden: {result['golden']}")
    print(f"  shots:  {result['shots']}")
    print(f"  golden_hash: {result['golden_hash'][:16]}...")
    print(f"  shots_hash:  {result['shots_hash'][:16]}...")
    print(f"  hash_match:  {result['hash_match']}")
    
    if not result["hash_match"] and "pixel_diff" in result:
        print(f"  pixel_diff:  {result['pixel_diff']}/{result['pixel_total']} ({result['pixel_diff_ratio']*100:.1f}%)")
    
    if result["hash_match"]:
        print(f"\n[✓] 对拍通过（hash 一致）")
    else:
        print(f"\n[✗] 对拍失败（hash 不一致）")


if __name__ == "__main__":
    main()
