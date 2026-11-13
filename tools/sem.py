#!/usr/bin/env python3
"""核心状态语义对拍（`.sem`）——两侧逐帧状态列对比，学 super-metroid-decomp 的两栈分离。

**两套栈不合并**（`~/workspace/super-metroid-decomp/docs/plan/two-verification-stacks.md`）：
  A 语义轨迹（本工具，`.sem` 每帧一行 N 列，毫秒级 diff，不需要 PPU）
  B 帧截图（本项目 L4，稀疏关键帧像素 0 差，440s/矩阵）
分辨率/失败语义/依赖三者都不同，硬合成一条链会让「哪些帧必须截」污染语义判据。

## 判据与纪律

1. **只写真正追踪的列**：某一侧的行里取不到某列 ⇒ 该列记 `<na>` 并计入 `na_cols`，
   **不得写 0 冒充**（AGENTS.md §3）。
2. **只比双方都有值的列**（交集）——单侧缺失是「未接线」不是「不一致」，
   由 `not_covered` 显式打印，绝不静默跳过。
3. `.sem` **内容不入库**：入库的是 ①场景 DSL（`cases/*.txt`，输入真相）
   ②`<name>.sem.meta.json` 指纹（`frames/cols/rom_sha256/sha256/runtime/note`）。
   理由：快照会立刻腐烂，且诱导「改数字让它变绿」；合法行为迁移时必须写 `note`。
4. **金面前置 = ≥3 连跑 sha256 全同**（确定性未证不得当金面）。

## 列契约（16 列，全部落在 ZP `$0000-$00FF` 与 IRAM `$0100-$07FF`）

| 列 | 地址 | 语义 | 规格锚 |
|---|---|---|---|
| `fc` | `$2D` | FrameCount | `constants_ram.asm:496` |
| `gm` | `$1D` | GameMode | `:482` |
| `mr` | `$1E` | MainRoutine | `:483` |
| `tr` | `$1F` | TitleRoutine | `:484` |
| `gp` | `$31` | GamePaused | `:504` |
| `nr` | `$20` | NextRoutine | `:480` |
| `t3` | `$2C` | Timer3（每 10 帧递减） | `:495` |
| `cb` | `$23` | CurrentBank | `:490` |
| `bsp` | `$24` | BankSwitchPending | `:491` |
| `ia` | `$74` | InArea | |
| `rm` | `$5A` | RoomNumber | `:567` |
| `st` | `$0300` | Samus.status | `:218` |
| `sy` | `$030D` | Samus.y | `:237` |
| `sx` | `$030E` | Samus.x | `:238` |
| `hp` | `$0106/$0107` | Health（BCD 两字节，拼 `HHLL`） | `:806-807` |
| `nm` | `$1A` | NMIStatus | `:475` |

规格与转移合法性判据见 `docs/spec/core-state-machine.md` §11。

## 用法

    python3 tools/sem.py watch-lines              # 打印让 oracle 带齐列的 3 行指令
    python3 tools/sem.py oracle-case <name>       # 生成 analysis/sem/<name>.sem.txt（跑 Mesen 用）
    python3 tools/sem.py case <name> [--meta]     # 一键：Rust 腿 + 建两侧 .sem + diff
    python3 tools/sem.py build <in.jsonl> [--out <o.sem>]
    python3 tools/sem.py diff <a.sem> <b.sem> [--only c1,c2]

退出码：0=全同 · 1=有分歧 · 2=格式/输入错（fail-loud，不静默通过）。
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
WORK = ROOT / "analysis" / "sem"

# 列顺序即输出顺序。值为绝对 RAM 地址（`pair` = 两字节拼接，BCD 低位在前）。
COLUMNS: list[tuple[str, int | tuple[int, int]]] = [
    ("fc", 0x002D),
    ("gm", 0x001D),
    ("mr", 0x001E),
    ("tr", 0x001F),
    ("gp", 0x0031),
    ("nr", 0x0020),
    ("t3", 0x002C),
    ("cb", 0x0023),
    ("bsp", 0x0024),
    ("ia", 0x0074),
    ("rm", 0x005A),
    ("st", 0x0300),
    ("sy", 0x030D),
    ("sx", 0x030E),
    ("hp", (0x0106, 0x0107)),
    ("nm", 0x001A),
]
COL_NAMES = [c for c, _ in COLUMNS]

# 区间面标签 → 基址。Mesen `watch_range` 与 Rust `--state-out` 的 `w` 键同名同格式
# （大写 hex 串，`tools/scenario_capture.lua:1111` readRange / `crates/game-core/src/
#  memmap/frame_face.rs:31 WATCH_FACES`）——两侧差异由本表吸收，不改 Rust。
RANGE_BASES: dict[str, int] = {
    "zp": 0x0000,
    "iram": 0x0100,
    "wram": 0x6000,
    "pw": 0x6988,
    "samus": 0x0300,   # 窄面（sem watch-lines 用，省带宽）
    "health": 0x0106,  # 窄面
}

# 单值 watch 标签别名（历史用例里写过的名字全收进来，显式列出——不做魔法推断）
LABEL_ALIASES: dict[str, list[str]] = {
    "fc": ["FC", "FrameCount"],
    "gm": ["GameMode"],
    "mr": ["MainRoutine"],
    "tr": ["TitleRoutine"],
    "gp": ["GamePaused"],
    "nr": ["NextRoutine"],
    "t3": ["Timer3"],
    "cb": ["CurrentBank"],
    "bsp": ["BankSwitchPending", "BSP"],
    "ia": ["InArea"],
    "rm": ["RoomNum", "RoomNumber"],
    "st": ["SamusStatus", "status"],
    "sy": ["SamusY"],
    "sx": ["SamusX"],
    "hp": ["Health"],
    "nm": ["NMIStatus"],
}

NA = "<na>"

# **相位敏感列**：$1A NMIStatus 是 NMI 与主循环之间的握手瞬时位——NMI 末尾置 1、
# 主循环 `inc FC` 后清 0、随后自旋等它变 1。两侧采样点落在握手窗口的不同侧，
# 因此它恒定不同（实测 oracle 2593/2600 帧=00、rust 2582/2583 帧=01），
# 不代表任何行为差异。默认不参与比较，但**显式打印**在 `phase_sensitive=` 里，
# 需要时用 `--only nm` 单独看。
PHASE_SENSITIVE = frozenset({"nm"})

# oracle 侧带齐 16 列只需 3 条区间（zp 覆盖 12 列、samus 3 列、health 1 列）；
# 区间必须窄——每帧 274B，2600 帧 ≈ 1.4MB hex，够快。
SEM_WATCH_LINES = """watch_range 0000-00FF zp
watch_range 0300-030F samus
watch_range 0106-0107 health"""


def _hex_bytes(blob: str) -> bytes | None:
    if not isinstance(blob, str) or not blob or len(blob) % 2:
        return None
    try:
        return bytes.fromhex(blob)
    except ValueError:
        return None


def project_row(w: dict) -> dict[str, str]:
    """一条 `{"f":N,"w":{label:value}}` → 16 列（取不到的列 = `<na>`）。"""
    memo: dict[int, int] = {}
    for label, base in RANGE_BASES.items():
        raw = _hex_bytes(w.get(label))
        if raw:
            for i, b in enumerate(raw):
                memo[base + i] = b

    out: dict[str, str] = {}
    for col, spec in COLUMNS:
        val = NA
        if isinstance(spec, tuple):
            lo, hi = spec
            if lo in memo and hi in memo:
                val = f"{memo[lo]:02X}{memo[hi]:02X}"
        elif spec in memo:
            val = f"{memo[spec]:02X}"
        if val == NA:  # 回退单值 watch 标签
            for alias in LABEL_ALIASES.get(col, []):
                if alias in w:
                    h = None
                    try:
                        h = int(str(w[alias]), 16)
                    except (TypeError, ValueError):
                        h = None
                    if h is not None:
                        val = f"{h:04X}" if col == "hp" else f"{h & 0xFF:02X}"
                        break
        out[col] = val
    return out


def build_sem(jsonl: Path, out: Path) -> tuple[int, dict[str, int]]:
    """读 jsonl → 写 .sem。返回 (帧数, {列: 缺失帧数})。"""
    na = {c: 0 for c in COL_NAMES}
    rows: list[str] = []
    with jsonl.open(encoding="utf-8") as f:
        for lineno, line in enumerate(f, 1):
            line = line.strip()
            if not line:
                continue
            try:
                rec = json.loads(line)
            except json.JSONDecodeError as e:
                raise SystemExit(f"[sem] FAIL 解析 {jsonl}:{lineno}: {e}")
            if not isinstance(rec.get("w"), dict):
                raise SystemExit(f"[sem] FAIL {jsonl}:{lineno} 缺 w 对象")
            proj = project_row(rec["w"])
            for c in COL_NAMES:
                if proj[c] == NA:
                    na[c] += 1
            rows.append(str(rec.get("f")) + "\t" + "\t".join(proj[c] for c in COL_NAMES))
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text("# cols=" + ",".join(COL_NAMES) + "\n" + "\n".join(rows) + ("\n" if rows else ""),
                   encoding="utf-8")
    return len(rows), na


def read_sem(path: Path) -> tuple[list[str], list[list[str]]]:
    cols: list[str] | None = None
    rows: list[list[str]] = []
    with path.open(encoding="utf-8") as f:
        for lineno, line in enumerate(f, 1):
            s = line.rstrip("\n")
            if not s.strip():
                continue
            if s.startswith("#"):
                body = s[1:].strip()
                if body.startswith("cols="):
                    cols = [c.strip() for c in body[5:].split(",") if c.strip()]
                continue
            if cols is None:
                raise SystemExit(f"[sem] FAIL {path}:{lineno} 缺 `# cols=` 头")
            parts = s.split("\t")
            if len(parts) != len(cols) + 1:
                raise SystemExit(
                    f"[sem] FAIL {path}:{lineno} 字段数 {len(parts)} ≠ 列数 {len(cols)}+frame")
            rows.append(parts)
    if cols is None:
        raise SystemExit(f"[sem] FAIL {path} 无 `# cols=` 头")
    return cols, rows


def diff_sem(a: Path, b: Path, only: list[str] | None = None):
    """按列报首个分歧帧；只比双方都有值的列（交集）。"""
    ca, ra = read_sem(a)
    cb, rb = read_sem(b)
    shared = [c for c in ca if c in cb]
    phase_sensitive = [c for c in shared if c in PHASE_SENSITIVE]
    if only:
        shared = [c for c in shared if c in only]
    else:
        shared = [c for c in shared if c not in PHASE_SENSITIVE]
    if not shared:
        raise SystemExit("[sem] FAIL 两侧无共同列（检查 `# cols=` 头）")

    idx_a = {c: ca.index(c) + 1 for c in shared}
    idx_b = {c: cb.index(c) + 1 for c in shared}
    covered = [c for c in shared
               if any(r[idx_a[c]] != NA for r in ra) and any(r[idx_b[c]] != NA for r in rb)]
    not_covered = [c for c in shared if c not in covered]
    if not covered:
        raise SystemExit("[sem] FAIL 没有任何列在两侧都有值（两侧都没带对应区间面？）")

    lines: list[str] = []
    # 【对齐口径】按**帧号**对齐，不按行序——两侧采样面不同：Rust `--state-out` 只在
    # spin 出口落行（掉帧窗/长例程忙跨帧无行），oracle 每帧一行。按行序对齐会把
    # 「未采样」误报成「不一致」（实测 f1/f2/f3 vs f1/f3 即此形）。
    fa_map = {int(r[0], 10): r for r in ra}
    fb_map = {int(r[0], 10): r for r in rb}
    common = sorted(set(fa_map) & set(fb_map))
    only_a = sorted(set(fa_map) - set(fb_map))
    only_b = sorted(set(fb_map) - set(fa_map))
    if only_a:
        lines.append(f"FRAMES_ONLY_ORACLE {len(only_a)}: {','.join(map(str, only_a[:20]))}"
                     f"{' …' if len(only_a) > 20 else ''}")
    if only_b:
        lines.append(f"FRAMES_ONLY_RUST {len(only_b)}: {','.join(map(str, only_b[:20]))}"
                     f"{' …' if len(only_b) > 20 else ''}")
    if not common:
        raise SystemExit("[sem] FAIL 两侧无共同帧（检查 frames 与采样相位）")

    matched = 0
    diverge = 0
    for fr in common:
        row_a, row_b = fa_map[fr], fb_map[fr]
        bad = []
        for c in covered:
            va, vb = row_a[idx_a[c]], row_b[idx_b[c]]
            if va == NA or vb == NA:
                continue
            if int(va, 16) != int(vb, 16):
                bad.append(f"{c}: oracle={va} rust={vb}")
        if bad:
            diverge += 1
            lines.append(f"DIVERGE f{fr} " + " | ".join(bad))
            if diverge >= 20:
                lines.append("...（已报 20 处首分歧，后续省略）")
                break
        else:
            matched += 1
    lines.append(f"MATCHED {matched} {len(common)}  cols={','.join(covered)}"
                 f"  not_covered={','.join(not_covered) or '-'}"
                 f"  frames_only=oracle+{len(only_a)}/rust+{len(only_b)}"
                 f"  phase_sensitive={','.join(phase_sensitive) or '-'}"
                 f"{'（已默认排除，--only 可单独看）' if phase_sensitive and not only else ''}")
    return (0 if diverge == 0 else 1), lines


# ---------------- 用例派生 ----------------

def _derive_sched(case_name: str) -> Path:
    """派生 Rust 腿 sched（剥 capture＝不跑渲染腿，快）——展开走唯一实现，不二次展开。

    `tools/scenario_run.py:548` 明令：Rust 与 Lua 同消费 `expand_schedule` 的产物；
    序列化只走 `l4_diff_run.schedule_text`。本函数不改输入，只去掉渲染采集点：
    语义对拍不需要 PPU（两栈分离的第一条理由）。
    """
    sys.path.insert(0, str(ROOT / "tools"))
    import l4_diff_run  # noqa: E402
    import scenario_run  # noqa: E402

    case = ROOT / "cases" / f"{case_name}.txt"
    if not case.exists():
        raise SystemExit(f"[sem] FAIL 用例不存在：{case}")
    plan = scenario_run.parse_scenario(case)
    joy = scenario_run.expand_schedule(plan)
    snap = (plan.get("bundle_load") or {}).get("path")
    text = l4_diff_run.schedule_text(f"{case_name}.sem", int(plan["frames"]), joy, [], snap)
    WORK.mkdir(parents=True, exist_ok=True)
    sched = WORK / f"{case_name}.sem.sched"
    sched.write_text(text, encoding="utf-8", newline="\n")
    return sched


def cmd_oracle_case(case_name: str) -> int:
    """生成 oracle 侧用例：原名 <name>.sem、剥 capture、追加 3 条区间 watch。"""
    case = ROOT / "cases" / f"{case_name}.txt"
    if not case.exists():
        raise SystemExit(f"[sem] FAIL 用例不存在：{case}")
    kept: list[str] = []
    for line in case.read_text(encoding="utf-8").splitlines():
        s = line.strip()
        if not s or s.startswith("#"):
            continue
        # 剥 name（换名避免覆盖用例自身轨迹）与全部采集指令（capture / capture_every /
        # capture_seq …——语义对拍不需要渲染腿，采集点只服务像素栈）
        head = s.split()[0]
        if head == "name" or head.startswith("capture"):
            continue
        kept.append(line)
    WORK.mkdir(parents=True, exist_ok=True)
    out = WORK / f"{case_name}.sem.txt"
    out.write_text(
        f"# 由 tools/sem.py oracle-case 生成（勿手改；源=cases/{case_name}.txt）\n"
        f"# 差异：name→{case_name}.sem（避免覆盖用例自身的 analysis/traces/<name>.frames.jsonl）、\n"
        f"#       剥 capture（语义对拍不需要渲染腿）、追加 3 条区间 watch（16 列来源）。\n"
        f"name {case_name}.sem\n" + "\n".join(kept) + "\n" + SEM_WATCH_LINES + "\n",
        encoding="utf-8")
    print(f"[sem] oracle 用例 → {out}")
    print(f"[sem] 跑一次（约 30-60s）：python3 tools/scenario_run.py run {out}")
    return 0


def cmd_case(case_name: str, meta: bool = False) -> int:
    WORK.mkdir(parents=True, exist_ok=True)
    sched = _derive_sched(case_name)
    rust_jsonl = WORK / f"{case_name}.rust.jsonl"
    cmd = ["cargo", "run", "-q", "-p", "game-core", "--bin", "rust_screenshot", "--",
           "--schedule", str(sched), "--out", str(WORK / f"{case_name}.rust.out"),
           "--state-out", str(rust_jsonl)]
    print(f"[sem] Rust 腿：{' '.join(cmd)}")
    r = subprocess.run(cmd, cwd=ROOT)
    if r.returncode != 0:
        print(f"[sem] FAIL Rust 腿退出码 {r.returncode}", file=sys.stderr)
        return 2

    oracle = ROOT / "analysis" / "traces" / f"{case_name}.sem.frames.jsonl"
    if not oracle.exists():
        print(f"[sem] FAIL 缺 oracle 轨迹 {oracle}\n"
              f"      先跑：python3 tools/sem.py oracle-case {case_name}\n"
              f"             python3 tools/scenario_run.py run analysis/sem/{case_name}.sem.txt",
              file=sys.stderr)
        return 2

    a_sem = WORK / f"{case_name}.oracle.sem"
    b_sem = WORK / f"{case_name}.rust.sem"
    fa, na_a = build_sem(oracle, a_sem)
    fb, na_b = build_sem(rust_jsonl, b_sem)
    print(f"[sem] oracle {fa} 帧 → {a_sem}")
    print(f"[sem] rust   {fb} 帧 → {b_sem}")
    for side, na in (("oracle", na_a), ("rust", na_b)):
        miss = {c: n for c, n in na.items() if n}
        if miss:
            print(f"[sem]   {side} 缺列（{sum(miss.values())} 格）：{miss}")
    code, lines = diff_sem(a_sem, b_sem)
    for ln in lines:
        print(ln)
    if meta:
        import hashlib
        md = {
            "case": case_name,
            "frames": fb,
            "cols": COL_NAMES,
            "oracle_sha256": hashlib.sha256(a_sem.read_bytes()).hexdigest(),
            "rust_sha256": hashlib.sha256(b_sem.read_bytes()).hexdigest(),
            "runtime": "rust_screenshot --state-out（无渲染腿）",
            "note": "",
        }
        mp = WORK / f"{case_name}.sem.meta.json"
        mp.write_text(json.dumps(md, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        print(f"[sem] 指纹 → {mp}（note 必填：行为合法迁移时写清为什么变）")
    return code


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        print(__doc__)
        return 2
    cmd = argv[1]
    if cmd == "watch-lines":
        print(SEM_WATCH_LINES)
        return 0
    if cmd == "oracle-case":
        if len(argv) < 3:
            print("用法：sem.py oracle-case <case-name>", file=sys.stderr)
            return 2
        return cmd_oracle_case(argv[2])
    if cmd == "build":
        if len(argv) < 3:
            print("用法：sem.py build <in.jsonl> [--out <out.sem>]", file=sys.stderr)
            return 2
        src = Path(argv[2])
        out = Path(argv[argv.index("--out") + 1]) if "--out" in argv else src.with_suffix(".sem")
        frames, na = build_sem(src, out)
        print(f"[sem] {src} → {out}（{frames} 帧；缺列 {sum(na.values())} 格）")
        for c, n in na.items():
            if n:
                print(f"[sem]   缺列 {c}: {n} 帧")
        return 0
    if cmd == "diff":
        if len(argv) < 4:
            print("用法：sem.py diff <a.sem> <b.sem> [--only c1,c2]", file=sys.stderr)
            return 2
        only = None
        if "--only" in argv:
            only = [x.strip() for x in argv[argv.index("--only") + 1].split(",") if x.strip()]
        code, lines = diff_sem(Path(argv[2]), Path(argv[3]), only)
        for ln in lines:
            print(ln)
        return code
    if cmd == "case":
        if len(argv) < 3:
            print("用法：sem.py case <case-name> [--meta]", file=sys.stderr)
            return 2
        return cmd_case(argv[2], meta="--meta" in argv)
    print(f"[sem] FAIL 未知子命令 {cmd}", file=sys.stderr)
    return 2


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
