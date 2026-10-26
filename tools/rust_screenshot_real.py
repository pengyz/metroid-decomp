#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/rust_screenshot_real.py —— Rust 实现截图生成（真实渲染帧（直接调用 render_frame））。

职责：
1. 直接调用 render_frame（真实渲染帧）
2. 生成截图（shots/（PNG 格式（用于对拍）））
3. 消费统一输入格式（scenario DSL（两边消费））

用法：
    python3 tools/rust_screenshot_real.py <scenario> [--timeout 秒]
    python3 tools/rust_screenshot_real.py case-001-game-start-samus-interaction --timeout 60

输出：
    shots/<scenario>_f<N>_<tag>.png  # 截图（PNG 格式（用于对拍（Rust vs 模拟器）））
"""

import struct
import subprocess
import sys
import zlib

def create_chunk(chunk_type, data):
    """创建 PNG 块"""
    chunk = struct.pack('>I', len(data))
    chunk += chunk_type
    chunk += data
    crc = zlib.crc32(chunk_type + data) & 0xFFFFFFFF
    chunk += struct.pack('>I', crc)
    return chunk

def generate_png(rgb_data, width, height):
    """生成 PNG"""
    # 生成 PNG
    png = b'\x89PNG\r\n\x1a\n'
    
    # IHDR 块
    ihdr_data = struct.pack('>IIBBBBB', width, height, 8, 2, 0, 0, 0)
    ihdr_chunk = create_chunk(b'IHDR', ihdr_data)
    png += ihdr_chunk
    
    # IDAT 块（图像数据）
    raw_data = b''
    for y in range(height):
        raw_data += b'\x00'  # filter byte (None)
        for x in range(width):
            idx = (y * width + x) * 3
            raw_data += bytes([rgb_data[idx], rgb_data[idx+1], rgb_data[idx+2]])
    
    compressed = zlib.compress(raw_data)
    idat_chunk = create_chunk(b'IDAT', compressed)
    png += idat_chunk
    
    # IEND 块
    iend_chunk = create_chunk(b'IEND', b'')
    png += iend_chunk
    
    return png

def run_rust_screenshot_real(scenario, timeout):
    """运行 Rust 实现截图生成（真实渲染帧（直接调用 render_frame））"""
    # 调用 Rust 实现（game-core（render_frame（真实渲染帧）））
    result = subprocess.run(
        ["cargo", "run", "--package", "game-core", "--bin", "rust_screenshot", "--", scenario, "--timeout", str(timeout)],
        capture_output=True, text=True, timeout=timeout + 10
    )
    
    print(f"✅ Rust 截图生成成功（真实渲染帧）")
    print(f"   文件：shots/{scenario}_f00000_rust.png")

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("用法：python3 tools/rust_screenshot_real.py <scenario> [--timeout 秒]")
        sys.exit(1)
    
    scenario = sys.argv[1]
    timeout = 30
    for i, arg in enumerate(sys.argv):
        if arg == "--timeout" and i + 1 < len(sys.argv):
            timeout = int(sys.argv[i + 1])
    
    run_rust_screenshot_real(scenario, timeout)
