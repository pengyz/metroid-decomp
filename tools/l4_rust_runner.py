#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/l4_rust_runner.py —— L4 Rust 实现执行（game-core（生成 shots/））。

职责：
1. 运行 Rust 实现（game-core（生成 shots/））
2. 消费统一输入格式（scenario DSL（两边消费））
3. 生成截图（shots/（用于对拍））

用法：
    python3 tools/l4_rust_runner.py <scenario> [--timeout 秒]
    python3 tools/l4_rust_runner.py game-start --timeout 30

输出：
    shots/<scenario>_f<N>_<tag>.png  # 截图（用于对拍）

设计原则（不过度）：
    - 工具与问题成正比（解决核心问题（Rust 实现执行（生成截图）））
    - 不过度设计（恢复功能（记账（后续增加）））
"""
from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SHOTS = ROOT / "shots"


def run_rust_implementation(scenario: str, timeout: int) -> dict:
    """运行 Rust 实现（生成截图）"""
    # TODO: 实现 Rust 实现执行（game-core（生成 shots/））
    # 当前：模拟 Rust 实现执行（生成截图）
    
    output_dir = SHOTS
    output_dir.mkdir(parents=True, exist_ok=True)
    
    # 模拟 Rust 实现执行（生成截图）
    print(f"[*] 运行 Rust 实现：{scenario}（timeout={timeout}）")
    
    return {
        "scenario": scenario,
        "timeout": timeout,
        "status": "TODO",  # TODO: 实现 Rust 实现执行
    }


def main():
    parser = argparse.ArgumentParser(description="L4 Rust 实现执行")
    parser.add_argument("scenario", help="场景名称")
    parser.add_argument("--timeout", type=int, default=30, help="超时时间（秒（默认 30））")
    
    args = parser.parse_args()
    
    result = run_rust_implementation(args.scenario, args.timeout)
    print(f"[*] 执行结果：{result}")


if __name__ == "__main__":
    main()
