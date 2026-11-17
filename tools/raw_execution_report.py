#!/usr/bin/env python3
"""机械生成 `docs/findings/raw-execution-coverage.md`（P3 动态腿：字节面执行证据）。

数据来源（恰好两个，全部机器事实）：
1. `crates/game-core` 单测 `memmap::verify_shim::tests::dynamic_execution_evidence`
   在 `--nocapture` 下打印的单行 `RAWCOV {json}`——本脚本现跑 cargo 取回，不读缓存；
2. `data/status/nes_map.json`（账本，**只读**；本脚本不修改、不降门槛）。

用法：
    python3 tools/raw_execution_report.py            # 重写 findings 文档
    python3 tools/raw_execution_report.py --check    # 只比对，不一致退出 1（复核/CI 用）

纪律：
- 本工具不持有判定权：它只把"测到了什么"与账本做集合运算，不裁决对错；
- 台账（nes_map.json）的 Unmodeled/Undeclared 只减不增由 `gates/cli.py check` 守，
  本工具不写账本、不改 `data/status/`；
- 文档里每个数字都能由本脚本重算——散文只解释口径，不替代数字。
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
DOC = REPO / "docs/findings/raw-execution-coverage.md"
LEDGER = REPO / "data/status/nes_map.json"
TEST = "memmap::verify_shim::tests::dynamic_execution_evidence"
MARKER = "RAWCOV "

CLASS_ORDER = ["owner", "unmodeled", "undeclared", "view", "case", "mode", "ephemeral"]
CLASS_CN = {
    "owner": "Owner（typed 归属）",
    "unmodeled": "Unmodeled（已声明、字节面是真相）",
    "undeclared": "Undeclared（面部无任何声明）",
    "view": "View（只读别名）",
    "case": "Case（状态依赖）",
    "mode": "Mode（模式依赖）",
    "ephemeral": "Ephemeral（瞬时）",
}


def run_probe() -> dict:
    """跑单测取回单行 RAWCOV JSON（确定性；捕获 stdout）。"""
    proc = subprocess.run(
        ["cargo", "test", "-p", "game-core", "--lib", TEST, "--", "--nocapture"],
        cwd=REPO,
        capture_output=True,
        text=True,
    )
    if proc.returncode != 0:
        tail = "\n".join((proc.stdout + proc.stderr).splitlines()[-25:])
        raise SystemExit(f"探针测试失败（先修测试，再生成报告）:\n{tail}")
    for line in proc.stdout.splitlines():
        if line.startswith(MARKER):
            return json.loads(line[len(MARKER):])
    raise SystemExit("未捕获 RAWCOV 行——测试是否被过滤/未 --nocapture？")


def load_ledger() -> dict:
    return json.loads(LEDGER.read_text(encoding="utf-8"))


def addr_set(face: list[dict]) -> list[int]:
    out: list[int] = []
    for seg in face:
        out.extend(range(seg["start"], seg["start"] + seg["len"]))
    return out


def addr_to_class(ledger: dict) -> dict[int, str]:
    out: dict[int, str] = {}
    for seg in ledger["undeclared"]:
        for a in range(seg["start"], seg["start"] + seg["len"]):
            out[a] = "undeclared"
    for seg in ledger["unmodeled"]:
        for a in range(seg["start"], seg["start"] + seg["len"]):
            out[a] = "unmodeled"
    for seg in ledger["owner_spans"]:
        for a in range(seg["start"], seg["start"] + seg["len"]):
            out[a] = "owner"
    return out


def seg_touch(seg: dict, touched: dict[int, str]) -> dict:
    addrs = range(seg["start"], seg["start"] + seg["len"])
    read = sum(1 for a in addrs if "r" in touched.get(a, ""))
    write = sum(1 for a in addrs if "w" in touched.get(a, ""))
    return {
        "len": seg["len"],
        "read": read,
        "write": write,
        "any": sum(1 for a in addrs if a in touched),
        "none": sum(1 for a in addrs if a not in touched),
    }


def ranges(addrs: list[int]) -> list[tuple[int, int]]:
    out: list[tuple[int, int]] = []
    for a in sorted(addrs):
        if out and a == out[-1][1] + 1:
            out[-1] = (out[-1][0], a)
        else:
            out.append((a, a))
    return out


def fmt_ranges(rs: list[tuple[int, int]], limit: int = 8) -> str:
    if not rs:
        return "—"
    parts = [f"`${lo:04X}`" if lo == hi else f"`${lo:04X}-${hi:04X}`" for lo, hi in rs[:limit]]
    if len(rs) > limit:
        parts.append(f"…(+{len(rs) - limit})")
    return "、".join(parts)


def pct(n: int, d: int) -> str:
    return f"{n * 100 // d}%" if d else "—"


def build_report(probe: dict, ledger: dict) -> str:
    touched = {int(a, 16): b for a, b, _ in probe["touched"]}
    cls_of = {int(a, 16): c for a, _, c in probe["touched"]}
    face = addr_set(ledger["face"])
    face_total = len(face)
    face_set = set(face)

    # 面外（WRAM 未建模区）触达：由分场景区间并集减去 face 得出。
    out_of_face: set[int] = set()
    for sc in probe["scenarios"]:
        for key in ("wram_read", "wram_write", "ram_read", "ram_write"):
            for lo, hi in sc[key]:
                for a in range(lo, hi + 1):
                    if a not in face_set:
                        out_of_face.add(a)

    face_touched = [a for a in face if a in touched]
    never = [a for a in face if a not in touched]
    by_class: dict[str, dict[str, int]] = {}
    for c in CLASS_ORDER:
        addrs = [a for a in face if cls_of.get(a, addr_to_class(ledger).get(a)) == c]
        by_class[c] = {
            "total": len(addrs),
            "any": sum(1 for a in addrs if a in touched),
            "read": sum(1 for a in addrs if "r" in touched.get(a, "")),
            "write": sum(1 for a in addrs if "w" in touched.get(a, "")),
            "never": sum(1 for a in addrs if a not in touched),
        }
    # 未被探针分类的 face 地址（应恒为 0：face_classes 已覆盖全域）。
    unknown = face_total - sum(probe["face_classes"].values())

    # 账本核对
    u_touched_segs = []
    u_zero_segs = []
    for seg in ledger["undeclared"]:
        t = seg_touch(seg, touched)
        (u_touched_segs if t["any"] else u_zero_segs).append((seg, t))
    m_zero_segs = []
    m_touched_segs = []
    for seg in ledger["unmodeled"]:
        t = seg_touch(seg, touched)
        (m_touched_segs if t["any"] else m_zero_segs).append((seg, t))

    u_touched_bytes = sum(t["any"] for _, t in u_touched_segs)
    m_zero_bytes = sum(t["len"] for _, t in m_zero_segs)
    era = cls_of  # 别名：文档里"实测分类"=探针按 SPANS 算的类

    lines: list[str] = []
    add = lines.append
    add("# 动态执行证据：字节面读写实测（P3 动态腿）")
    add("")
    add("> 证据级别：**A**（测试内插桩实测 + 账本机械对账，2026-11-17）。")
    add("> 复现：`python3 tools/raw_execution_report.py`（内部现跑 `cargo test ... --nocapture`，"
        "不读缓存、不读环境变量）。")
    add("")
    add("## 0. 结论（全部可由本页表格/命令重算）")
    add("")
    add(f"- 账本比对面 **{face_total} B**；本次三场景实测触达 **{len(face_touched)} B**"
        f"（{pct(len(face_touched), face_total)}），未触达 **{len(never)} B**。")
    add(f"- 触达中：读 {sum(1 for a in face_touched if 'r' in touched[a])} B、"
        f"写 {sum(1 for a in face_touched if 'w' in touched[a])} B、"
        f"读写双触 {sum(1 for a in face_touched if touched[a] == 'rw')} B。")
    add(f"- 面外（WRAM `$6000-$7FFF` 未进账本 face 的部分）另测得 **{len(out_of_face)} B** "
        f"被读/写——账本**未建模**该区域（不是探针错误，见 §7）。")
    add(f"- 账本核对：Undeclared 段 **{len(u_touched_segs)}/{len(ledger['undeclared'])}** 段被实测使用"
        f"（{u_touched_bytes} B）；Unmodeled 段 **{len(m_zero_segs)}/{len(ledger['unmodeled'])}** 段"
        f"在窗口内零触达（{m_zero_bytes} B）。")
    add(f"- Owner 段实测触达 **{by_class['owner']['any']}/{by_class['owner']['total']} B**；"
        f"owner 冲突 **0 处**（测试内 `assert_owner_unique` 全量遍历 + 合成双 Owner 用例）。")
    add("")
    add("## 1. 方法")
    add("")
    add("### 1.1 插桩形态（只在测试内）")
    add("")
    add("- 探针：`crates/game-core/src/memmap/verify_shim.rs`，模块声明为 `#[cfg(test)] mod`，"
        "生产构建**不编译**；位图用 `thread_local!`，`arm()`/`disarm()` 显式开关，"
        "不读 `std::env`、不写文件。")
    add("- 挂点：`memmap/raw.rs` 的 `read_ram`/`write_ram`/`read_wram`/`write_wram` 四入口 + "
        "`samus/state.rs` 的 `Samus::get`/`Samus::set` **未命中 typed 时的字节面回退**。")
    add("- 口径：记录的是**字节面访问请求**（含 typed 域经兼容面读到的请求），不是物理存储写；"
        "地址归一与运行时一致（`$6000-$7FFF`→WRAM，其余 `& 0x7FF` 2KB 镜像）。")
    add("")
    add("### 1.2 录制窗口")
    add("")
    add("- 每个场景新建 `FrameEngine`；**tick1（boot）不计**——boot 的 `clear_pages` 经 `write_ram` "
        "无条件零填 `$0200-$07FF`/`$6000-$7EFF`，计入会把整个面部误判为「写过」。")
    add("- 窗口为 tick2..=末帧；场景表见 §3。")
    add("")
    add("### 1.3 已知盲区（未观测 ≠ 未使用）")
    add("")
    add("- `frame/memory.rs` 写 PPUDATA 时 `self.ram()[0x00FF]` 为**直读**，不经 `read_ram`——"
        "`$00FF` 的这一次读不可见（同址其他路径的读写仍可见）。")
    add("- `restore_mss` / typed 清零直接 `ram_mut()`/`wram_mut()`：测试装配径，非游戏循环。")
    add("- 三场景（标题/起步行走/蓝门）覆盖有限：未触达**不等于**死字节，只说明「本窗口没用到」。")
    add("")
    add("## 2. 面部总覆盖（按账本四态）")
    add("")
    add("| 类 | 面部字节 | 实测触达 | 读 | 写 | 未触达 | 触达率 |")
    add("|---|---:|---:|---:|---:|---:|---:|")
    for c in CLASS_ORDER:
        v = by_class[c]
        if v["total"] == 0:
            continue
        add(f"| {CLASS_CN[c]} | {v['total']} | {v['any']} | {v['read']} | {v['write']} | "
            f"{v['never']} | {pct(v['any'], v['total'])} |")
    add(f"| **合计** | **{face_total}** | **{len(face_touched)}** | "
        f"**{sum(1 for a in face_touched if 'r' in touched[a])}** | "
        f"**{sum(1 for a in face_touched if 'w' in touched[a])}** | **{len(never)}** | "
        f"**{pct(len(face_touched), face_total)}** |")
    add("")
    if unknown:
        add(f"> 注：{unknown} B 未能由探针归类（应恒为 0；非 0 说明 SPANS 与账本漂移）。")
        add("")
    add("## 3. 场景矩阵与分场景触达")
    add("")
    add("| 场景 | 帧窗 | 输入（帧号） | 面内触达 | 其中读 | 其中写 | WRAM 读区间数 | WRAM 写区间数 |")
    add("|---|---|---|---:|---:|---:|---:|---:|")
    for sc in probe["scenarios"]:
        lo, hi = sc["frames"]
        inside: set[int] = set()
        for key in ("ram_read", "ram_write", "wram_read", "wram_write"):
            for a, b in sc[key]:
                for x in range(a, b + 1):
                    if x in face_set:
                        inside.add(x)
        r = sum(1 for a in inside if any(lo2 <= a <= hi2 for lo2, hi2 in sc["ram_read"] + sc["wram_read"]))
        w = sum(1 for a in inside if any(lo2 <= a <= hi2 for lo2, hi2 in sc["ram_write"] + sc["wram_write"]))
        add(f"| `{sc['name']}` | {lo}–{hi} | 见 `crates/game-core/src/memmap/verify_shim.rs` | "
            f"{len(inside)} | {r} | {w} | {len(sc['wram_read'])} | {len(sc['wram_write'])} |")
    add("")
    add("输入回放与 `cases/case-001-game-start-samus-interaction.txt` / "
        "`cases/case-002-door-blue-interaction.txt` 同帧号；帧号即 `rust_screenshot` 的 `frames` 轴。")
    add("")
    add("## 4. Undeclared 段实测（账本 18 段）")
    add("")
    add("| # | 段 | 长 | 触达 | 读 | 写 | 未触达 | refs |")
    add("|---:|---|---:|---:|---:|---:|---:|---|")
    for i, seg in enumerate(ledger["undeclared"], 1):
        t = seg_touch(seg, touched)
        refs = "、".join(seg.get("refs", [])[:3]) + ("…" if len(seg.get("refs", [])) > 3 else "")
        add(f"| {i} | `${seg['start']:04X}` | {seg['len']} | {t['any']} | {t['read']} | "
            f"{t['write']} | {t['none']} | {refs} |")
    add("")
    zero_u = [(seg, t) for seg, t in u_zero_segs]
    if zero_u:
        add("零触达的 Undeclared 段（**本窗口**未使用，不代表死字节）：")
        add("")
        for seg, _ in zero_u:
            add(f"- `${seg['start']:04X}` × {seg['len']} B（refs：{'、'.join(seg.get('refs', []))}）")
        add("")
    add("## 5. Unmodeled 段实测（账本 27 段）")
    add("")
    add("| # | 段 | 长 | 触达 | 读 | 写 | 未触达 | refs |")
    add("|---:|---|---:|---:|---:|---:|---:|---|")
    for i, seg in enumerate(ledger["unmodeled"], 1):
        t = seg_touch(seg, touched)
        refs = "、".join(seg.get("refs", [])[:2]) + ("…" if len(seg.get("refs", [])) > 2 else "")
        add(f"| {i} | `${seg['start']:04X}` | {seg['len']} | {t['any']} | {t['read']} | "
            f"{t['write']} | {t['none']} | {refs} |")
    add("")
    if m_zero_segs:
        add("窗口内**零触达**的 Unmodeled 段（账本声称「字节面是真相」，实测未被执行）：")
        add("")
        for seg, _ in m_zero_segs:
            add(f"- `${seg['start']:04X}` × {seg['len']} B（refs：{'、'.join(seg.get('refs', []))}）")
        add("")
    add("## 6. 缺口清单（面部未触达字节）")
    add("")
    add(f"未触达 **{len(never)} B**，按连续区间（最多列 40 段；完整集合由脚本可重算）：")
    add("")
    never_ranges = ranges(never)
    for lo, hi in never_ranges[:40]:
        cls = sorted({addr_to_class(ledger).get(a, "?") for a in range(lo, hi + 1)})
        add(f"- `${lo:04X}-${hi:04X}`（{hi - lo + 1} B；账本类：{'/'.join(cls)}）")
    if len(never_ranges) > 40:
        add(f"- …共 {len(never_ranges)} 段（其余略）")
    add("")
    add("## 7. 与账本的差异（本页唯一的「冲突」口径）")
    add("")
    add(f"1. **Undeclared 被实测使用**：{len(u_touched_segs)}/{len(ledger['undeclared'])} 段、"
        f"{u_touched_bytes} B。账本口径 = 「面部无任何声明」（refs 是「谁碰过它」的线索，不是归属）——"
        "实测确认这些字节**不是死字节**，是**待归属的债**。")
    add(f"2. **Unmodeled 窗口内零触达**：{len(m_zero_segs)}/{len(ledger['unmodeled'])} 段、"
        f"{m_zero_bytes} B。账本称「已声明、字节面是真相」；实测在标题/起步/蓝门三场景内未执行——"
        "要么场景未覆盖（更可能），要么该字段当前版本走的是别的路径。**不据此改账本**。")
    add(f"3. **面外 WRAM**：{len(out_of_face)} B 被读写但不在 `face`（`$6000-$7FFF` 仅 "
        f"`$6AF4-$6BD3` 进账本）。这是账本的**已知面部边界**，不是探针越界；"
        "若将来要把 WRAM 全量纳入归属，分母需先扩。")
    add("4. **owner 冲突 0 处**：测试内对全部 SPANS 逐字节 `assert_owner_unique`；"
        "另有合成双 Owner 用例证明冲突会 panic 且文本含地址（`$0300`）。")
    add("")
    add("## 8. 限制与后续")
    add("")
    add("- 本报告是**执行证据**，不是完备性证明：三场景、约 3200 帧，覆盖的是「起步房间 + 蓝门」路径。")
    add("- 若要提高分母，按 `cases/` 增加场景即可（同一探针，无需改生产代码）；"
        "新增场景后重跑本脚本，数字会自动更新。")
    add("- 静态腿（`gates/cli.py check` 的 nes-map 门）仍守「每字节唯一 Owner / 账本只减不增」；"
        "动态腿不替代门禁，只把「没人用」从自陈变成实测。")
    add("")
    return "\n".join(lines)


def main() -> int:
    check = "--check" in sys.argv[1:]
    probe = run_probe()
    ledger = load_ledger()
    text = build_report(probe, ledger)
    if check:
        old = DOC.read_text(encoding="utf-8") if DOC.exists() else ""
        if old != text:
            print("raw-execution-coverage.md 与实测不一致——请重跑 python3 tools/raw_execution_report.py")
            return 1
        print("raw-execution-coverage.md 与实测一致")
        return 0
    DOC.parent.mkdir(parents=True, exist_ok=True)
    DOC.write_text(text, encoding="utf-8")
    print(f"已生成 {DOC.relative_to(REPO)}（{len(text)} 字节）")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
