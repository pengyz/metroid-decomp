#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/l4_unified_format.py —— L4 统一输入格式定义（scenario DSL（两边消费））。

对拍用例定义：
    对拍用例 = ① 输入脚本 + ② 待加载存档 + ③ 对拍时长
    输出 = 对拍结果（pass/fail（hash 对比（差异统计）））

设计要素：
    ① 输入脚本：特定操作（scenario DSL（模拟器格式））
    ② 待加载存档：游戏状态定义文件（load_snapshot（缺口））
    ③ 对拍时长：特定时长（可由输入脚本定义）

统一输入格式（scenario DSL）：
    # 注释
    frames <N>          # 总帧数（对拍时长）
    snapshot <path>     # 待加载存档（load_snapshot（缺口））
    press <frame> <btn> # 按键（帧号（按钮））
    hold <start-end> <btn>  # 按住（帧范围（按钮））
    endFrame <frame>    # 结束帧（对拍点）

示例（game-start.txt）：
    frames 700
    press 540 start     # 按 start 键（帧 540）
    hold 540-541 start  # 按住 start 键（帧 540-541）
    endFrame 699        # 结束帧（对拍点）
"""
from __future__ import annotations

import re
from pathlib import Path
from typing import Dict, List, Optional


class ScenarioDSL:
    """scenario DSL 解析器（统一输入格式（两边消费））"""
    
    def __init__(self, path: Path):
        self.path = path
        self.frames: int = 0
        self.snapshot: Optional[str] = None
        self.inputs: List[Dict] = []
        self.end_frame: int = 0
        
    def parse(self) -> "ScenarioDSL":
        """解析 scenario DSL"""
        with open(self.path, "r") as f:
            lines = f.readlines()
        
        for line in lines:
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            
            # frames <N>
            match = re.match(r"frames\s+(\d+)", line)
            if match:
                self.frames = int(match.group(1))
                continue
            
            # snapshot <path>
            match = re.match(r"snapshot\s+(.+)", line)
            if match:
                self.snapshot = match.group(1)
                continue
            
            # press <frame> <btn>
            match = re.match(r"press\s+(\d+)\s+(\w+)", line)
            if match:
                self.inputs.append({
                    "type": "press",
                    "frame": int(match.group(1)),
                    "button": match.group(2),
                })
                continue
            
            # hold <start-end> <btn>
            match = re.match(r"hold\s+(\d+)-(\d+)\s+(\w+)", line)
            if match:
                self.inputs.append({
                    "type": "hold",
                    "start": int(match.group(1)),
                    "end": int(match.group(2)),
                    "button": match.group(3),
                })
                continue
            
            # endFrame <frame>
            match = re.match(r"endFrame\s+(\d+)", line)
            if match:
                self.end_frame = int(match.group(1))
                continue
        
        return self
    
    def to_simulator_format(self) -> str:
        """转换为模拟器格式（scenario_run.py 消费）"""
        lines = []
        lines.append(f"frames {self.frames}")
        for inp in self.inputs:
            if inp["type"] == "press":
                lines.append(f"press {inp['frame']} {inp['button']}")
            elif inp["type"] == "hold":
                lines.append(f"hold {inp['start']}-{inp['end']} {inp['button']}")
        lines.append(f"endFrame {self.end_frame}")
        return "\n".join(lines)
    
    def to_rust_format(self) -> str:
        """转换为 Rust 格式（game-core 消费）"""
        # TODO: 实现 Rust 格式转换
        return self.to_simulator_format()
    
    def __str__(self) -> str:
        return f"ScenarioDSL(frames={self.frames}, snapshot={self.snapshot}, inputs={len(self.inputs)}, end_frame={self.end_frame})"


def parse_scenario(path: Path) -> ScenarioDSL:
    """解析 scenario DSL（统一输入格式）"""
    return ScenarioDSL(path).parse()


if __name__ == "__main__":
    import sys
    if len(sys.argv) < 2:
        print("用法：python3 tools/l4_unified_format.py <scenario>")
        sys.exit(1)
    
    path = Path(sys.argv[1])
    dsl = parse_scenario(path)
    print(f"解析结果：{dsl}")
    print(f"\n模拟器格式：")
    print(dsl.to_simulator_format())
    print(f"\nRust 格式：")
    print(dsl.to_rust_format())
