#!/usr/bin/env python3
"""baseline 上调工具（rust_test_count 584→587（PENDING（现值高于基线（+3 测试））））——user ruling（工具建（tools/（tool 缺（待建））））"""
import json, sys
from pathlib import Path

BASELINE = Path("data/status/baselines/rust_test_count.json")

def update(new_count: int) -> None:
    old = json.loads(BASELINE.read_text()) if BASELINE.exists() else {"rust_test_count": 584}
    old_count = old.get("rust_test_count", 584)
    if new_count < old_count:
        print(f"ERR: 基线只升不降（{old_count}→{new_count}）——拒绝")
        sys.exit(1)
    old["rust_test_count"] = new_count
    BASELINE.write_text(json.dumps(old, indent=2) + "\n")
    print(f"baseline 上调：rust_test_count {old_count}→{new_count}（PENDING（现值高于基线））")

if __name__ == "__main__":
    update(int(sys.argv[1]) if len(sys.argv) > 1 else 587)
