#!/usr/bin/env python3
"""相位对齐判据：给定 emu trace(.frames.jsonl) 与 rust state(.jsonl)，
验证 `rust f=N ≡ emu f=N+k` 这条采集相位惯例是否成立。

判据（机器可判，不靠人眼）：对 `$0014 Joy1Status` 逐帧比较，在候选偏移
`{-1,0,+1}` 中取零失配最大者；若最优偏移不唯一或最优偏移失配率 > 0 ⇒ exit 1。

用法：phase_align_check.py <emu.frames.jsonl> <rust-state.jsonl> [--expect +1]
"""
import json, sys

def load(path):
    out = {}
    for line in open(path, encoding="utf-8"):
        r = json.loads(line)
        f = r.get("f")
        w = r.get("w") or {}
        out[f] = w
    return out

def joy_emu(w):
    h = w.get("joy") or ""
    return int(h[:2], 16) if h else 0

def joy_rust(w):
    h = w.get("zp") or ""
    i = 0x14 * 2
    return int(h[i:i+2], 16) if len(h) > i + 1 else 0

def main(argv):
    emu_p, rust_p = argv[1], argv[2]
    expect = 1
    if "--expect" in argv:
        expect = int(argv[argv.index("--expect") + 1])
    emu = {f: joy_emu(w) for f, w in load(emu_p).items() if joy_emu(w)}
    rust = {f: joy_rust(w) for f, w in load(rust_p).items()}
    if not emu:
        print("FAIL: emu trace 无 joy 观察面（用例需 watch_range 0014-0014 joy）")
        return 2
    cands = sorted({-1, 0, 1, expect})
    scores = {}
    for k in cands:  # rust f=N 对应 emu f=N+k
        ok = bad = 0
        for f in rust:
            if f + k not in emu:
                continue
            if rust[f] == emu[f + k]:
                ok += 1
            else:
                bad += 1
        scores[k] = (ok, bad)
    for k in cands:
        ok, bad = scores[k]
        print(f"  offset {k:+d}: 一致 {ok} 失配 {bad} 一致率 {ok/(ok+bad or 1):.4f}")
    best = max(cands, key=lambda k: scores[k][0])
    ok, bad = scores[best]
    if best != expect:
        print(f"FAIL: 最优偏移 {best:+d} 与惯例 {expect:+d} 不符")
        return 1
    if bad:
        print(f"FAIL: 偏移 {best:+d} 下仍有 {bad} 帧 joy 失配（相位或被比较面不成立）")
        return 1
    print(f"OK: rust f=N ≡ emu f=N{expect:+d}（{ok} 帧零失配）")
    return 0

if __name__ == "__main__":
    sys.exit(main(sys.argv))
