#!/usr/bin/env python3
"""inputlog 与影片输入表零容差对拍（防"旧解码产物"复发）。

判据：`inputlog` 第 f 帧的 p1 必须逐位等于 bk2 `Input Log.txt` 第 f 行的
P1 字段（列序取 `LogKey:`；行首 `Power/Reset` 命令组不计入 P1）。

用法：
  python3 tools/movie_check.py movies/metroid-anyp-3666.bk2 movies/eien86-6725M.inputlog
退出码：0=全等；1=有偏差或行数不符；2=参数/文件错。
"""
from __future__ import annotations

import io
import sys
import zipfile

BIT_FOR = {
    "Up": 0x08, "Down": 0x04, "Left": 0x02, "Right": 0x01,
    "Start": 0x10, "Select": 0x20, "B": 0x40, "A": 0x80,
}


def open_bk2(path: str) -> zipfile.ZipFile:
    z = zipfile.ZipFile(path)
    nested = [n for n in z.namelist() if n.lower().endswith(".bk2")]
    if nested:
        z = zipfile.ZipFile(io.BytesIO(z.read(nested[0])))
    return z


def p1_truth(bk2: str) -> list[int]:
    z = open_bk2(bk2)
    txt = z.read("Input Log.txt").decode("utf-8", "replace").splitlines()
    ki = next(i for i, l in enumerate(txt) if l.startswith("LogKey:"))
    key = [t.strip().lstrip("#") for t in txt[ki].split(":", 1)[1].split("|") if t.strip()]
    p1 = [k.split(None, 1)[1] for k in key if k.startswith("P1 ")]
    if len(p1) != 8 or any(b not in BIT_FOR for b in p1):
        raise ValueError(f"LogKey P1 列异常: {p1}")
    out: list[int] = []
    for raw in txt[ki + 1:]:
        if not raw.startswith("|"):
            continue
        flat = "".join(raw.strip("|").split("|"))
        # 头部命令组（Power/Reset）在前，占 len(key)-16 列
        off = len(flat) - 16
        field = flat[off:off + 8]
        if len(field) != 8:
            raise ValueError(f"行宽异常: {raw!r}")
        out.append(sum(BIT_FOR[b] for b, ch in zip(p1, field) if ch not in ". "))
    return out


def main(argv: list[str]) -> int:
    if len(argv) != 3:
        print(__doc__)
        return 2
    try:
        truth = p1_truth(argv[1])
        got = [int(l.split()[0], 16) for l in open(argv[2]) if l.strip() and not l.startswith("#")]
    except Exception as exc:  # noqa: BLE001
        print(f"ERROR: {exc}")
        return 2
    if len(truth) != len(got):
        print(f"FAIL 行数：bk2={len(truth)} inputlog={len(got)}")
        return 1
    bad = [(i, got[i], truth[i]) for i in range(len(truth)) if got[i] != truth[i]]
    if bad:
        print(f"FAIL 不一致 {len(bad)}/{len(truth)} 帧；前 5：{[ (i, hex(a), hex(b)) for i,a,b in bad[:5] ]}")
        return 1
    print(f"OK {len(truth)} 帧 P1 全部一致")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
