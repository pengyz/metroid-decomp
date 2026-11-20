#!/usr/bin/env python3
"""apu_diff —— APU 寄存器面逐帧对拍（声音判据仪器 2026-11-20）。

判据哲学（用户裁定：完整声音系统的验证判据）：音频对拍不"听"——逐帧 APU 寄存器写
序列（addr,value）对拍。寄存器面等价 ⇒ 波形输出等价（强判据、可机械执行）。

输入（双腿产物，tools/scenario_capture.lua / rust_screenshot.rs 同缝同帧号同格式落盘）：
  · 单文件：<name>_f<NNNNN>_<tag>.apu.hex——逐行 "%04X %02X"（addr value）；
  · 或目录：递归集 *.apu.hex，帧号从文件名 _f(\\d+)_ 抽取；同一帧多文件取 tag 拼接
    排序后合并（同帧同 tag 惯例下不应发生，发生即显式告警）。
  · 单文件名不含帧号时：按单帧（f=0）处理并显式声明该假设（输出行）。

对齐假设（显式声明）：目录形态按帧号对齐；单侧缺帧=「缺帧」分歧（不是空桶——空桶
=存在空文件，双腿落空文件语义对称）。

输出纪律（参照 tools/l4_diff_run.py）：结论行打印；exit 0=全等 / 1=有分歧 / 2=参数或
输入格式错误。

用法：python3 tools/apu_diff.py <emu_apu.hex 或目录> <rust_apu.hex 或目录>
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

FRAME_RE = re.compile(r"_f(\d+)_")
LINE_RE = re.compile(r"^([0-9A-Fa-f]{4}) ([0-9A-Fa-f]{2})$")


# 已知非声域写：Mesen 的 `emu.callbackType.write` 会把 CPU 写**所有**地址都抛回来，
# `$4016` 是手柄 strobe 端口（`ReadJoyPads ; 07:C215` 每帧写两次），它不产生声音，
# 保留下来只会把「仪器噪声」冒充分歧。`$4017` 是真 APU 帧计数器，照实比较。
NOISE_ADDRS = {0x4016}


def parse_apu_text(text: str, src: str) -> list[tuple[int, int]]:
    """逐行解析 "%04X %02X"；格式漂移 fail-loud（ValueError——调用方归 exit 2）。"""
    writes: list[tuple[int, int]] = []
    dropped = 0
    for ln, raw in enumerate(text.splitlines(), 1):
        line = raw.strip()
        if not line:
            continue
        m = LINE_RE.match(line)
        if not m:
            raise ValueError(f"{src}:{ln}: 格式漂移（须 '%04X %02X'）: {raw!r}")
        addr = int(m.group(1), 16)
        if addr in NOISE_ADDRS:
            dropped += 1
            continue
        writes.append((addr, int(m.group(2), 16)))
    if dropped:
        print(f"[信息] {src}: 过滤已知非声域写 {dropped} 条（地址 {sorted(NOISE_ADDRS)}，见 NOISE_ADDRS 注）")
    return writes


def load_side(path: Path) -> dict[int, list[tuple[int, int]]]:
    """载入一侧：文件→单帧（无帧号时 f=0 并声明假设）；目录→*.apu.hex 按帧号收集。"""
    if path.is_file():
        m = FRAME_RE.search(path.name)
        frame = int(m.group(1)) if m else 0
        if not m:
            print(f"[假设] {path.name}: 文件名无帧号——按单帧 f=0 对齐处理")
        return {frame: parse_apu_text(path.read_text(encoding="ascii"), str(path))}
    if path.is_dir():
        files = sorted(path.rglob("*.apu.hex"))
        if not files:
            raise ValueError(f"{path}: 目录内无 *.apu.hex 产物")
        faces: dict[int, list[tuple[int, int]]] = {}
        for f in files:
            m = FRAME_RE.search(f.name)
            if not m:
                raise ValueError(f"{f}: 文件名无帧号（须 <name>_f<NNNNN>_<tag>.apu.hex）")
            frame = int(m.group(1))
            face = parse_apu_text(f.read_text(encoding="ascii"), str(f))
            if frame in faces:
                print(f"[告警] 帧 {frame} 多文件（{f.name}）——按文件名序拼接合并")
                faces[frame].extend(face)
            else:
                faces[frame] = face
        return faces
    raise ValueError(f"{path}: 既非文件也非目录")


def diff_frame(emu: list[tuple[int, int]], rust: list[tuple[int, int]]) -> str | None:
    """逐写比较；None=全等；否则返回首分歧摘要文本。"""
    if emu == rust:
        return None
    n = max(len(emu), len(rust))
    for i in range(n):
        a = emu[i] if i < len(emu) else None
        b = rust[i] if i < len(rust) else None
        if a != b:
            return (f"写数 emu={len(emu)} rust={len(rust)}；首分歧写#{i}: "
                    f"emu={fmt_w(a)} rust={fmt_w(b)}")
    return None  # 不可达（序列相等已在头部拦截）


def fmt_w(w: tuple[int, int] | None) -> str:
    return "（无）" if w is None else f"{w[0]:04X} {w[1]:02X}"


def frame_summary(emu: list[tuple[int, int]], rust: list[tuple[int, int]]) -> str:
    """当帧差异结构化摘要：地址差异计数 + 首若干条分歧明细。"""
    from collections import Counter
    ca, cb = Counter(a for a, _ in emu), Counter(a for a, _ in rust)
    only_emu = sorted((ca - cb).elements())
    only_rust = sorted((cb - ca).elements())
    lines = []
    if only_emu:
        lines.append("emu 独有地址写: " + ", ".join(f"{a:04X}x{n}" for a, n in Counter(only_emu).most_common(8)))
    if only_rust:
        lines.append("rust 独有地址写: " + ", ".join(f"{a:04X}x{n}" for a, n in Counter(only_rust).most_common(8)))
    # 地址写次数对照（判「每帧都写」还是「音符边界才写」的关键读数）
    both = sorted({a for a, _ in emu} | {a for a, _ in rust})
    rows = [f"  {a:04X}: emu={ca.get(a, 0)} rust={cb.get(a, 0)}" for a in both
            if ca.get(a, 0) != cb.get(a, 0) and 0x4000 <= a <= 0x4017]
    if rows:
        lines.append("  地址写次数对照:")
        lines.extend(rows[:10])
    k = max(len(emu), len(rust))
    shown = 0
    for i in range(k):
        a = emu[i] if i < len(emu) else None
        b = rust[i] if i < len(rust) else None
        if a != b:
            lines.append(f"  #{i}: emu={fmt_w(a)} rust={fmt_w(b)}")
            shown += 1
            if shown >= 6:
                lines.append("  …（截断）")
                break
    return "\n".join(lines)


def collect_events(writes: list[tuple[int, int]]) -> list[str]:
    """帧内事件抽取（纯寄存器面——新范式调试仪器 2026-11-20）。

    事件词表（机械定义，无引擎语义断言——asm 唯一真相源纪律：音乐/SFX 通道归属是
    sound_engine 语义，寄存器面不做 music/sfx 断言，只给 enable 位形）：
      · mute   —— $4015 写 0（全通道禁用——静音事件帧）；
      · enable —— $4015 写非零（通道启用/重启用——init 信号帧，附位形细节）；
      · lenload —— $4003/$4007/$400B/$400F 写（长度计数装载——发声（重）触发信号帧，
        通道序 pulse1/pulse2/triangle/noise 对应）。
    """
    ev: list[str] = []
    seen_len: list[int] = []
    for a, v in writes:
        if a == 0x4015:
            if v == 0:
                if "mute" not in ev:
                    ev.append("mute")
            elif "enable" not in ev:
                ev.append(f"enable={v:02X}")
        elif a in (0x4003, 0x4007, 0x400B, 0x400F) and a not in seen_len:
            seen_len.append(a)
    if seen_len:
        ev.append("lenload=" + ",".join(f"{a:04X}" for a in seen_len))
    return ev


def event_compare(emu_faces: dict, rust_faces: dict) -> None:
    """事件级对照视图（新范式下声音移植的主要调试面）：逐帧事件两侧对照列表。"""
    frames = sorted(set(emu_faces) | set(rust_faces))
    ev_emu = {f: collect_events(emu_faces[f]) for f in frames if f in emu_faces}
    ev_rust = {f: collect_events(rust_faces[f]) for f in frames if f in rust_faces}
    n_emu = sum(1 for v in ev_emu.values() if v)
    n_rust = sum(1 for v in ev_rust.values() if v)
    match = sum(1 for f in frames
                if f in ev_emu and f in ev_rust
                and ev_emu[f] == ev_rust[f] and ev_emu[f])
    print(f"事件级对照：emu 事件帧 {n_emu} · rust 事件帧 {n_rust} · 两侧同帧同事件 {match}")
    shown = 0
    for f in frames:
        e, r = ev_emu.get(f, []), ev_rust.get(f, [])
        if not e and not r:
            continue
        if e == r:
            continue  # 两侧一致的事件帧不逐条刷屏（汇总计数已含）
        if shown == 0:
            print("事件帧差异明细（帧: emu | rust；'—'=该侧无事件/无产物）:")
        print(f"  f{f}: {'; '.join(e) or '—'} | {'; '.join(r) or '—'}")
        shown += 1
        if shown >= 30:
            print("  …（截断 30 条）")
            break
    if n_emu == 0 and n_rust == 0:
        print("事件面：两侧全程无 APU 事件帧（可能引擎未触发发声）")


def main(argv: list[str]) -> int:
    if len(argv) != 3:
        print("用法：python3 tools/apu_diff.py <emu_apu 或目录> <rust_apu 或目录>")
        return 2
    emu_p, rust_p = Path(argv[1]), Path(argv[2])
    try:
        emu_faces = load_side(emu_p)
        rust_faces = load_side(rust_p)
    except (ValueError, OSError, UnicodeDecodeError) as exc:
        print(f"[错误] {exc}")
        return 2
    print(f"[输入] emu: {emu_p}（{len(emu_faces)} 帧）· rust: {rust_p}（{len(rust_faces)} 帧）")
    frames = sorted(set(emu_faces) | set(rust_faces))
    n_equal = 0
    diverged: list[int] = []
    missing: list[int] = []
    first_report: str | None = None
    first_frame: int | None = None
    for f in frames:
        if f not in emu_faces or f not in rust_faces:
            missing.append(f)
            continue
        verdict = diff_frame(emu_faces[f], rust_faces[f])
        if verdict is None:
            n_equal += 1
        else:
            diverged.append(f)
            if first_frame is None:
                first_frame = f
                first_report = verdict
    # 首分歧帧输出原文（仪器可工作的证据面——当帧写序列差异摘要）
    if first_frame is not None:
        print(f"首分歧帧: f{first_frame}（{first_report}）")
        print("当帧差异摘要:")
        print(frame_summary(emu_faces[first_frame], rust_faces[first_frame]))
    if missing:
        head = ", ".join(str(f) for f in missing[:10])
        more = f" …共 {len(missing)} 帧" if len(missing) > 10 else ""
        print(f"缺帧（单侧无产物——非空桶）: {head}{more}")
    event_compare(emu_faces, rust_faces)
    print(f"汇总：全等 {n_equal} · 分歧 {len(diverged)} · 缺帧 {len(missing)}（共 {len(frames)} 帧并集）")
    if not diverged and not missing:
        print("总评：✅ APU 寄存器面逐帧全等")
        return 0
    print("总评：❌ 未全等（分歧/缺帧见上——仪器照实报告，分歧本身即声链移植缺口定位面）")
    return 1


if __name__ == "__main__":
    sys.exit(main(sys.argv))
