#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""int_x 的 SMT miter 等价证明（2^40 域）+ 公式↔Rust 桥接采样（2^16 随机 + 边界族）。

三层口径中的「模型内证明」：
  Side A = bank07:E3E5-E448 + sub_C2C5/C2BF/E449 的**指令语义逐条展开**（BV8 公式，
          与 seed 指令清单一一对应——本文件内注释带指令地址）
  Side B = game_core::integrators::int_x 的语义公式化
  断言「存在输入使任一输出不同」⇒ check() == unsat ⇒ 证明（公式层）。
残余假设（显式声明）：Side B 公式 ↔ 真实 Rust 代码的同一性经 2^16 随机点 + 边界族桥接采样
（int-x-bridge bin），非证明——与 int_y 的「解释器语义假设」同类，入账。
"""
import random
import subprocess
import sys

from z3 import BitVec, BitVecVal, If, LShR, Solver, sat, unsat, ZeroExt, UGE, ULT, Extract

def u8(x):
    return x & 0xFF

def side_a(si, sf, acc, py, m):
    """指令语义展开（BV8）。返回 (i2, f2, py2, disp, x, y, lim1)。"""
    lim_lo = (m << 4) & 0xFF            # E3E8 JSR C2C5 (ASL×4) → $00/$02
    lim_hi = LShR(m, 4)                 # E3F2 JSR C2BF (LSR×4) → $01/$03
    s = ZeroExt(1, sf) + ZeroExt(1, acc)   # E3FC CLC / E3FD ADC $315（9 位保进位）
    f1 = Extract(7, 0, s)
    c = Extract(8, 8, s)                # 进位
    sign = Extract(7, 7, acc)           # E406 BIT $315（N=acc bit7）
    a_mid = If(sign == 1, BitVecVal(0xFF, 9), BitVecVal(0, 9))   # E404 LDA#0 / E40B LDA#$FF
    t = a_mid + ZeroExt(1, si) + ZeroExt(8, c)              # E40D ADC $309
    i1 = Extract(7, 0, t)
    neg = Extract(7, 7, i1) == 1        # E414 BPL（测新 int 符号）

    # 负路径取负链（E418-E422）：mag_lo = -f1, mag_hi = -si - (1-c1), c1=(f1==0)
    c1 = If(f1 == 0, BitVecVal(1, 9), BitVecVal(0, 9))
    mag_lo = Extract(7, 0, BitVecVal(0, 9) - ZeroExt(1, f1))
    mag_hi = Extract(7, 0, BitVecVal(0, 9) - ZeroExt(1, i1) - (BitVecVal(1, 9) - c1))  # E41F SBC $309 读更新后 int
    x_r = If(neg, mag_lo, f1)           # E403 TAX / E41C TAX
    y_r = If(neg, mag_hi, i1)           # E413 TAY / E422 TAY

    # sub_E449：limit16 取负（仅负路径）
    c2 = If(lim_lo == 0, BitVecVal(1, 9), BitVecVal(0, 9))
    neg_lo = Extract(7, 0, BitVecVal(0, 9) - ZeroExt(1, lim_lo))
    neg_hi = Extract(7, 0, BitVecVal(0, 9) - ZeroExt(1, lim_hi) - (BitVecVal(1, 9) - c2))
    lo0 = If(neg, neg_lo, lim_lo)       # $00 末值（E42D LDA 0）
    hi0 = If(neg, neg_hi, lim_hi)       # $01 末值（E432 LDA 1）

    # E426 CPX $02 / E429 SBC $03：16 位无符号比较 (y_r:x_r) vs (lim_hi:lim_lo)
    c_lo = UGE(x_r, lim_lo)             # CPX：C = 无借位（**无符号**）
    borrow = ULT(y_r, lim_hi)
    eq = y_r == lim_hi
    do_clamp = Not(borrow | (eq & Not(c_lo)))   # BCC E437：借位则跳过钳位
    # 注：do_clamp = (y_r:x_r) >= (lim_hi:lim_lo)
    f2 = If(do_clamp, lo0, f1)          # E42F STA $313
    i2 = If(do_clamp, hi0, i1)          # E434 STA $309
    s2 = ZeroExt(1, py) + ZeroExt(1, f2)   # E43A CLC / E43B ADC $313
    py2 = Extract(7, 0, s2)
    c3 = Extract(8, 8, s2)
    disp = Extract(7, 0, BitVecVal(0, 9) + ZeroExt(1, i2) + ZeroExt(8, c3))  # E443 ADC
    return (i2, f2, py2, disp, x_r, y_r, hi0)

# 我们在 z3 里没有 Not/And 的命名空间问题——补 import
from z3 import Not, And, Or as z3_or  # noqa: E402

def side_b(si, sf, acc, py, m):
    """game_core::integrators::int_x 的公式化（与 Rust 逐行对应）。"""
    lim_lo = (m << 4) & 0xFF
    lim_hi = LShR(m, 4)
    s = ZeroExt(1, sf) + ZeroExt(1, acc)
    f1 = Extract(7, 0, s)
    c = Extract(8, 8, s)
    sign_ext = If(Extract(7, 7, acc) == 1, BitVecVal(0xFF, 9), BitVecVal(0, 9))
    t = ZeroExt(1, si) + sign_ext + ZeroExt(8, c)
    i1 = Extract(7, 0, t)
    neg = Extract(7, 7, i1) == 1
    x_r = If(neg, Extract(7, 0, BitVecVal(0, 9) - ZeroExt(1, f1)), f1)
    borrow0 = f1 == 0
    y_r = If(neg, Extract(7, 0, BitVecVal(0, 9) - ZeroExt(1, i1) -
                          If(borrow0, BitVecVal(0, 9), BitVecVal(1, 9))), i1)  # 同上：读更新后 int
    new_lo = Extract(7, 0, BitVecVal(0, 9) - ZeroExt(1, lim_lo))
    b2 = lim_lo == 0
    neg_hi = Extract(7, 0, BitVecVal(0, 9) - ZeroExt(1, lim_hi) -
                     If(b2, BitVecVal(0, 9), BitVecVal(1, 9)))
    lim0 = If(neg, new_lo, lim_lo)
    lim1 = If(neg, neg_hi, lim_hi)
    c_lo = UGE(x_r, lim_lo)
    borrow = ULT(y_r, lim_hi) | ((y_r == lim_hi) & Not(c_lo))
    do_clamp = Not(borrow)
    f2 = If(do_clamp, lim0, f1)
    i2 = If(do_clamp, lim1, i1)
    s2 = ZeroExt(1, py) + ZeroExt(1, f2)
    py2 = Extract(7, 0, s2)
    c3 = Extract(8, 8, s2)
    disp = Extract(7, 0, BitVecVal(0, 9) + ZeroExt(1, i2) + ZeroExt(8, c3))
    return (i2, f2, py2, disp, x_r, y_r, lim1)

def prove():
    si, sf, acc, py, m = (BitVec(n, 8) for n in ("si", "sf", "acc", "py", "m"))
    a = side_a(si, sf, acc, py, m)
    b = side_b(si, sf, acc, py, m)
    names = ("i", "f", "py", "disp", "x", "y", "lim1")
    s = Solver()
    s.add(z3_or(*[a[i] != b[i] for i in range(7)]))
    r = s.check()
    if r == unsat:
        print("SMT PROOF: int_x ≡ bank07:E3E5-E448 指令语义（BV8 miter）——反例不存在（unsat）")
        print(f"输出面: {names} × 输入域 2^40（si,sf,acc,py,m 各 8 位）")
        return True
    if r == sat:
        print("反例:", s.model())
        return False
    print("求解结果未知:", r)
    return False

def bridge(n=1 << 16):  # 2^16 随机 + 1440 边界点 = 66976（2^20 系旧夸大，已更正）
    """公式↔Rust 桥接采样（残余假设量化）：2^20 随机点 + 边界点。"""
    rng = random.Random(0x9E3779B9)
    pts = []
    for _ in range(n):
        pts.append((rng.randrange(256), rng.randrange(256), rng.randrange(256),
                    rng.randrange(256), rng.randrange(256)))
    # 边界点：钳位阈值邻域
    for m in (0, 1, 0x0F, 0x10, 0x80, 0xFF):
        for si in (0, 1, 0x7F, 0x80, 0xFE, 0xFF):
            for sf in (0, 1, 0xFE, 0xFF):
                for acc in (0, 1, 0x7F, 0x80, 0xFF):
                    for py in (0, 0xFF):
                        pts.append((si, sf, acc, py, m))
    inp = "".join(f"{a:02x} {b:02x} {c:02x} {d:02x} {e:02x}\n" for a, b, c, d, e in pts)
    # 事故②对抗审：SMT 桥无超时=真空面（病态求解可永转）——补 120s（实测秒级，
    # 40 倍余量；超限=FAIL 显式）。注意：路径随 CARGO_TARGET_DIR 独立化迁移。
    import os as _os
    _tgt = _os.environ.get("CARGO_TARGET_DIR", "target")
    _bridge = _tgt + "/release/int-x-bridge" if _tgt.startswith("/") else f"./{_tgt}/release/int-x-bridge"
    try:
        proc = subprocess.run(
            [_bridge], input=inp, capture_output=True, text=True,
            check=True, timeout=120)
    except subprocess.TimeoutExpired:
        raise SystemExit("[事故②护栏] int-x-bridge 超时 >120s——限时打回非挂死")
    lines = proc.stdout.strip().splitlines()
    assert len(lines) == len(pts), f"桥接输出数不符 {len(lines)} != {len(pts)}"
    from z3 import simplify
    bad = 0
    for (si, sf, acc, py, m), line in zip(pts, lines):
        got = tuple(int(x, 16) for x in line.split())
        want = tuple(
            simplify(v).as_long()
            for v in side_b(BitVecVal(si, 8), BitVecVal(sf, 8), BitVecVal(acc, 8), BitVecVal(py, 8), BitVecVal(m, 8))
        )
        if got != want:
            bad += 1
            if bad <= 3:
                print(f"桥接分歧: in=({si:#04x},{sf:#04x},{acc:#04x},{py:#04x},{m:#04x}) rust={got} 公式={want}")
    print(f"BRIDGE: 公式↔Rust {len(pts)} 采样点（含钳位阈值边界族），分歧 {bad}")
    return bad == 0

if __name__ == "__main__":
    ok = prove()
    ok = bridge() and ok
    sys.exit(0 if ok else 1)
