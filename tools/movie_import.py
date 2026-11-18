#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/movie_import.py —— 模拟器录像（TAS movie）→ 逐帧输入表（L4 对拍输入面）。

为什么存在：`cases/*.txt` 的输入是我们自己编的（15 案覆盖首区为主）。社区 TAS 录像
本质是**别人手写的逐帧输入流**，把它的 input log 当输入喂给 emu 与 rust 双腿，
就能用现成的像素对拍去裁「两侧是否一致」——输入覆盖面从十余个手写场景扩到整局。

定位纪律（必须写死，否则会得出假结论）：
- 录像**不是**「官方升级版脚本」，它的**同步绑定具体模拟器核心/版本**（同一条 fm2
  换 FCEUX 版本都会十分钟后失步）。我们**不需要**「录像在 Mesen2 里跑出作者要的结果」，
  只需要**同一份逐帧输入**。判据始终是 emu↔rust 一致，不是复现 TAS 成绩。
- 因此本工具**只做格式解析与合法性过滤**，不做「同步性修正」，不猜输入语义。

支持的容器/格式：
- `.bk2`（BizHawk，ZIP）：`Header.txt` + `Input Log.txt`（每行 `|` 开头，分组位）；
  兼容 bizhawk 把真正 bk2 再套一层 zip 的形态（外层 zip 里只有一个 .bk2）。
- `.fm2`（FCEUX，纯文本）：头部 key-value + 每行 `commands|port0|port1|port2`
  （port 字段为 RLDUTSBA 八字符位；`commands` bit0=软复位 bit1=硬复位）。

按钮位映射（三处同构，任一处漂移即「两侧跑不同输入」）：
  RLDUTSBA 字符序 == bit0..bit7 == tools/scenario_run.py::BUTTON_BITS
  == crates/game-platform PadState（A=bit7 … Right=bit0）。

产物（默认 stdin 路径派生，**不入库**——TASVideos 许可：不得转发未改内容/须署名）：
  <out>.inputlog   文本：`# key value` 元信息 + 每帧一行 `p1 p2 cmd`（hex）
  <out>.provenance.json  来源/校验和/帧数/首个 2P 输入帧（对拍报告引用它）

用法：
    python3 tools/movie_import.py inspect movies/x.bk2
    python3 tools/movie_import.py convert movies/x.bk2 -o movies/x.inputlog
    python3 tools/movie_import.py convert movies/x.bk2 -o movies/x.inputlog --frames 3400
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ROM = ROOT / "rom" / "Metroid (USA).nes"

# 与 tools/scenario_run.py::BUTTON_BITS 同序（RLDUTSBA == bit0..bit7）
BTN_ORDER_TEXT = "RLDUTSBA"

# 字符 → PadState 位（与 crates/game-platform PadState 同一约定：A=bit7 … Right=bit0）。
# 为什么需要它：**列序不等于位序**。fm2 的列序恰好是 RLDUTSBA（== bit0..bit7，
# 所以"第 i 列 = bit i"在 fm2 上成立），但 bk2 的列序由 `Input Log.txt` 的
# `LogKey:` 行声明，实测 6725M 为 `#Reset|Power|#P1 Up|P1 Down|P1 Left|P1 Right|
# P1 Start|P1 Select|P1 B|P1 A|…`（= UDLRTSBA）——按 fm2 位序硬解会把四个方向
# 全搬错一格（Up↔Right、Down↔Left），其余四键因 Start/Select/B/A 恰好落在同一
# 位而"看起来正确"，是典型的静默错误。
BTN_BIT = {
    "Right": 0,
    "Left": 1,
    "Down": 2,
    "Up": 3,
    "Start": 4,
    "Select": 5,
    "B": 6,
    "A": 7,
}

ORDER_ALIAS = {
    "R": "Right",
    "L": "Left",
    "D": "Down",
    "U": "Up",
    "T": "Start",
    "S": "Select",
}

CMD_SOFT_RESET = 0x01
CMD_HARD_RESET = 0x02


class MovieError(Exception):
    """录像解析/合法性错误（fail-loud：宁可拒绝，不静默降级成空输入）。"""


def sha256_file(p: Path) -> str:
    h = hashlib.sha256()
    with p.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def _read_zip_members(path: Path) -> tuple[str, str]:
    """从 ZIP 里取 (Header.txt, Input Log.txt)；支持套娃（外层 zip 内含一个 .bk2）。"""
    with zipfile.ZipFile(path) as z:
        names = z.namelist()
        header = next((n for n in names if n.endswith("Header.txt")), None)
        log = next((n for n in names if n.endswith("Input Log.txt")), None)
        if header and log:
            return z.read(header).decode("utf-8-sig", "replace"), z.read(log).decode(
                "utf-8-sig", "replace"
            )
        nested = [n for n in names if n.lower().endswith(".bk2")]
        if len(nested) == 1:
            tmp = ROOT / "analysis" / "_tmp_nested.bk2"
            tmp.parent.mkdir(parents=True, exist_ok=True)
            tmp.write_bytes(z.read(nested[0]))
            try:
                return _read_zip_members(tmp)
            finally:
                tmp.unlink(missing_ok=True)
    raise MovieError(f"{path}: ZIP 内找不到 Header.txt/Input Log.txt（成员: {names[:6]}）")


def _bk2_tokens(path: Path, log_text: str) -> list[str]:
    """取 `Input Log.txt` 的 `LogKey:` 列名表（fail-loud：没有就拒绝，不猜位序）。"""
    for raw in log_text.splitlines():
        s = raw.strip()
        if s.lower().startswith("logkey:"):
            body = s.split(":", 1)[1]
            toks = [t.strip() for t in body.split("|") if t.strip()]
            if toks:
                return toks
    raise MovieError(f"{path}: `Input Log.txt` 缺 LogKey 行（列序未知 ⇒ 拒绝解析）")


def _bk2_columns(path: Path, log_text: str) -> list[tuple[str, int, str]]:
    """LogKey → 列描述表 `[("cmd",0,name) | ("pad",port,btn), ...]`。

    注意 **LogKey 的 `|` 是列分隔符**，而数据行的 `|` 是分组符（组内每字符一列），
    两者不是一回事：数据行 `|..|........|........|` 展开为 18 列，与 LogKey 的 18 项
    一一对应。按分组数比列数会直接误判成"格式异常"。
    """
    desc: list[tuple[str, int, str]] = []
    seen_pad = False
    for tok in _bk2_tokens(path, log_text):
        name = tok.lstrip("#").strip()
        if name in ("Reset", "Power"):
            if seen_pad:
                raise MovieError(f"{path}: 命令列出现在按键列之后（LogKey={tok!r}）")
            desc.append(("cmd", 0, name))
            continue
        m = re.match(r"P(\d)\s+(.+)$", name)
        if not m:
            raise MovieError(f"{path}: 无法识别的 LogKey 列名 {tok!r}")
        port, btn = int(m.group(1)), m.group(2).strip()
        if port not in (1, 2):
            raise MovieError(f"{path}: 只支持 P1/P2，LogKey 出现 P{port}")
        if btn not in BTN_BIT:
            raise MovieError(f"{path}: 未知按键 {btn!r}（LogKey={tok!r}）")
        seen_pad = True
        desc.append(("pad", port, btn))
    if not any(k == "cmd" for k, _, _ in desc):
        raise MovieError(f"{path}: LogKey 缺命令列")
    for port in (1, 2):
        if not any(k == "pad" and pt == port for k, pt, _ in desc):
            raise MovieError(f"{path}: LogKey 缺 P{port} 按键列")
    return desc


def _bk2_apply(desc: list[tuple[str, int, str]], resp: str) -> tuple[int, int, int]:
    """按列描述把一行的扁平字符解成 (p1, p2, cmd)。"""
    cmd = p1 = p2 = 0
    for ch, (kind, port, name) in zip(resp, desc, strict=True):
        if ch in ". ":
            continue
        if kind == "cmd":
            cmd |= CMD_HARD_RESET if name == "Power" else CMD_SOFT_RESET
        elif port == 1:
            p1 |= 1 << BTN_BIT[name]
        else:
            p2 |= 1 << BTN_BIT[name]
    return p1, p2, cmd


def parse_bk2(path: Path) -> dict:
    header_text, log_text = _read_zip_members(path)
    header: dict[str, str] = {}
    for line in header_text.splitlines():
        line = line.strip()
        if not line:
            continue
        k, _, v = line.partition(" ")
        header[k] = v.strip()

    frames: list[tuple[int, int, int]] = []
    desc = _bk2_columns(path, log_text)
    for raw in log_text.splitlines():
        line = raw.strip()
        if not line.startswith("|"):
            continue
        resp = "".join(line.strip("|").split("|"))
        if len(resp) != len(desc):
            raise MovieError(
                f"{path}: 输入行列数 {len(resp)} 与 LogKey {len(desc)} 不符: {line!r}"
            )
        frames.append(_bk2_apply(desc, resp))
    return {
        "format": "bk2",
        "header": header,
        "platform": header.get("Platform", ""),
        "core": header.get("Core", ""),
        "rom_sha1": header.get("SHA1", ""),
        "game": header.get("GameName", ""),
        "board": header.get("BoardName", ""),
        "frames": frames,
    }


def bits_from_text(s: str, order: str = BTN_ORDER_TEXT) -> int:
    """按列序 `order` 把八字符文本解成位域（非 '.'/' ' 即按下）。

    `order` 的第 i 个字符 = 第 i 列对应的按键（`T`=Start、`S`=Select）。
    """
    out = 0
    if len(s) != len(order):
        raise MovieError(f"按键文本宽度 {len(s)} 与列序 {order!r} 不符: {s!r}")
    for ch, name in zip(s, order, strict=True):
        if ch in ". ":
            continue
        key = ORDER_ALIAS.get(name, name)
        if key not in BTN_BIT:
            raise MovieError(f"未知按键列 {name!r}（列序 {order!r}）")
        out |= 1 << BTN_BIT[key]
    return out


def parse_fm2(path: Path) -> dict:
    text = path.read_text(encoding="utf-8-sig", errors="replace")
    header: dict[str, str] = {}
    frames: list[tuple[int, int, int]] = []
    in_log = False
    for raw in text.splitlines():
        line = raw.rstrip("\r\n")
        if not in_log and line.startswith("|"):
            in_log = True
        if in_log:
            if not line.startswith("|"):
                continue
            parts = line.strip("|").split("|")
            if len(parts) < 2:
                raise MovieError(f"{path}: fm2 输入行异常: {line!r}")
            cmd = int(parts[0] or 0)
            p0 = bits_from_text(parts[1]) if len(parts) > 1 else 0
            p1 = bits_from_text(parts[2]) if len(parts) > 2 else 0
            frames.append((p0, p1, cmd & 0x03))
        else:
            k, _, v = line.partition(" ")
            if k:
                header[k] = v
    return {
        "format": "fm2",
        "header": header,
        "platform": "NES",
        "core": f"FCEUX {header.get('emuVersion', '?')}",
        "rom_sha1": "",
        "rom_checksum": header.get("romChecksum", ""),
        "game": header.get("romFilename", ""),
        "board": "",
        "frames": frames,
    }


def load_movie(path: Path) -> dict:
    if not path.is_file():
        raise MovieError(f"录像不存在: {path}")
    suffix = path.suffix.lower()
    if suffix == ".bk2" or zipfile.is_zipfile(path):
        mv = parse_bk2(path)
    elif suffix == ".fm2":
        mv = parse_fm2(path)
    else:
        raise MovieError(f"不支持的录像格式: {suffix}（支持 .bk2 / .fm2）")
    if mv["platform"] and mv["platform"].upper() != "NES":
        raise MovieError(f"{path}: Platform={mv['platform']!r}，本项目只支持 NES")
    if not mv["frames"]:
        raise MovieError(f"{path}: 输入日志为空")
    return mv


def screen_summary(mv: dict, path: Path) -> dict:
    """合法性过滤与统计（不修改输入，只报告）。"""
    frames = mv["frames"]
    hdr = mv["header"]
    first_p1 = next((i for i, (p1, _, _) in enumerate(frames) if p1), None)
    first_p2 = next((i for i, (_, p2, _) in enumerate(frames) if p2), None)
    resets = [i for i, (_, _, c) in enumerate(frames) if c]
    return {
        "source": str(path),
        "sha256": sha256_file(path),
        "format": mv["format"],
        "core": mv["core"],
        "board": mv["board"],
        "rom_sha1": mv["rom_sha1"],
        "frames": len(frames),
        "frames_with_input": sum(1 for p1, p2, _ in frames if p1 or p2),
        "first_p1_frame": first_p1,
        "first_p2_frame": first_p2,
        "reset_frames": resets[:8],
        "reset_count": len(resets),
        "pal": hdr.get("palFlag", "0") not in ("0", ""),
        "from_savestate": bool(hdr.get("savestate")) or any(
            n.lower().startswith("savestate") for n in ([])
        ),
        "comment": hdr.get("comment", "") or hdr.get("Comments", ""),
    }


def write_inputlog(mv: dict, out: Path, meta: dict, limit: int | None) -> None:
    frames = mv["frames"][:limit] if limit else mv["frames"]
    out.parent.mkdir(parents=True, exist_ok=True)
    with out.open("w", encoding="utf-8") as f:
        f.write("# inputlog v1（tools/movie_import.py 产物；**不入库**）\n")
        for k in ("source", "sha256", "format", "core", "board", "rom_sha1", "frames"):
            f.write(f"# {k} {meta.get(k, '')}\n")
        f.write("# 每帧一行: <p1 hex> <p2 hex> <cmd hex>（RLDUTSBA==bit0..bit7；cmd bit0=软复位 bit1=硬复位）\n")
        for p1, p2, cmd in frames:
            f.write(f"{p1:02x} {p2:02x} {cmd:02x}\n")


def read_inputlog(path: Path) -> dict[int, tuple[int, int, int]]:
    """读回 .inputlog → {frame(1-based): (p1, p2, cmd)}。语义与写入端对称（单源）。"""
    table: dict[int, tuple[int, int, int]] = {}
    n = 0
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        parts = line.split()
        if len(parts) != 3:
            raise MovieError(f"{path}: inputlog 行异常: {raw!r}")
        n += 1
        p1, p2, cmd = (int(x, 16) for x in parts)
        if p1 or p2 or cmd:
            table[n] = (p1, p2, cmd)
    return table


def cmd_inspect(args: argparse.Namespace) -> int:
    mv = load_movie(Path(args.movie))
    meta = screen_summary(mv, Path(args.movie))
    json.dump(meta, sys.stdout, ensure_ascii=False, indent=2)
    sys.stdout.write("\n")
    return 0


def cmd_convert(args: argparse.Namespace) -> int:
    src = Path(args.movie)
    mv = load_movie(src)
    meta = screen_summary(mv, src)
    if meta["pal"]:
        raise MovieError(f"{src}: palFlag=1（PAL 时序），本项目只支持 NTSC")
    if args.frames:
        if args.frames > meta["frames"]:
            raise MovieError(f"--frames {args.frames} 超过录像长度 {meta['frames']}")
    out = Path(args.out)
    write_inputlog(mv, out, meta, args.frames)
    prov = out.with_suffix(".provenance.json")
    prov.write_text(json.dumps(meta, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[movie_import] {src.name}: {meta['frames']} 帧 → {out}")
    print(f"  首个 1P 输入帧 {meta['first_p1_frame']} / 首个 2P 输入帧 {meta['first_p2_frame']}"
          f" / 复位命令 {meta['reset_count']} 次")
    print(f"  provenance: {prov}")
    return 0


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description="模拟器录像 → 逐帧输入表（L4 输入面）")
    sub = ap.add_subparsers(dest="cmd", required=True)

    p = sub.add_parser("inspect", help="只解析并打印元信息/统计")
    p.add_argument("movie")
    p.set_defaults(func=cmd_inspect)

    p = sub.add_parser("convert", help="转换为 .inputlog（+ provenance）")
    p.add_argument("movie")
    p.add_argument("-o", "--out", required=True)
    p.add_argument("--frames", type=int, default=None, help="只取前 N 帧（做小规模 pilot）")
    p.set_defaults(func=cmd_convert)

    args = ap.parse_args(argv)
    try:
        return int(args.func(args))
    except MovieError as e:
        print(f"[movie_import] FAIL: {e}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
