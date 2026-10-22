#!/usr/bin/env python3
"""rewrite_coverage.py —— 改写覆盖率统计工具（自动化统计（default() 返回（TODO 注释（逻辑））））。

统计口径（三分类）：
- 真实现：有逻辑（无 default()（无 TODO））
- 部分真实现：有逻辑（但仍有 default()）
- 空壳：default() 返回+TODO 注释+无逻辑

用法：python3 tools/rewrite_coverage.py [根目录（默认 crates/game-core/src）]
输出：逐文件分类+汇总计数（真实现/部分真实现/空壳）——实测输出原文（A 级）。
"""
import os
import re
import sys

def classify(path: str):
    """分类单个 .rs 文件（真实现/部分真实现/空壳）。"""
    src = open(path, encoding='utf-8', errors='replace').read()
    lines = src.splitlines()
    # default() 返回形：fn ... -> T { default() } 或 return default() 或 Default::default()
    default_re = re.compile(r'default\(\)|Default::default\(\)')
    todo_re = re.compile(r'TODO|todo!|unported!')
    # 逻辑行：非空非注释非大括号行
    logic_lines = [l for l in lines
                   if l.strip() and not l.strip().startswith('//')
                   and not l.strip().startswith('/*') and l.strip() not in ('{', '}', '});')]
    has_default = any(default_re.search(l) for l in lines)
    has_todo = any(todo_re.search(l) for l in lines)
    has_logic = len(logic_lines) > 20  # 阈值：>20 逻辑行=有实质逻辑
    if has_logic and not has_default and not has_todo:
        return 'real', len(logic_lines), has_default, has_todo
    if has_logic and has_default:
        return 'partial', len(logic_lines), has_default, has_todo
    return 'shell', len(logic_lines), has_default, has_todo

def main():
    root = sys.argv[1] if len(sys.argv) > 1 else 'crates/game-core/src'
    counts = {'real': 0, 'partial': 0, 'shell': 0}
    rows = []
    for dirpath, _, filenames in os.walk(root):
        for fn in sorted(filenames):
            if not fn.endswith('.rs'):
                continue
            p = os.path.join(dirpath, fn)
            kind, n, hd, ht = classify(p)
            counts[kind] += 1
            rows.append((p.replace(root + '/', ''), kind, n, hd, ht))
    # 输出（实测输出原文（A 级））
    print('=== 改写覆盖率统计（rewrite_coverage）===')
    print(f'根目录: {root}')
    print(f'{"文件":<48} {"分类":<10} {"逻辑行":>6} {"default":>8} {"TODO":>5}')
    for name, kind, n, hd, ht in rows:
        print(f'{name:<48} {kind:<10} {n:>6} {str(hd):>8} {str(ht):>5}')
    total = sum(counts.values())
    print(f'--- 汇总 ---')
    print(f'真实现（real）:      {counts["real"]}')
    print(f'部分真实现（partial）: {counts["partial"]}')
    print(f'空壳（shell）:       {counts["shell"]}')
    print(f'总计:              {total}')
    if total:
        print(f'真实现占比: {counts["real"]}/{total} = {counts["real"]/total:.1%}')
    return 0

if __name__ == '__main__':
    sys.exit(main())
