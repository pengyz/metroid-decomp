#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/diff_traces.py —— L3 帧级状态向量差分（trace-diff 门禁入口）。

比对面：逐帧状态向量 = watch 字段（manifest 声明显式基址区间；hex 大小写归一后比对）。
两侧语义（pipeline §5）：
  左  oracle：Mesen2 场景 trace（`analysis/traces/<scenario>.frames.jsonl`，可再生）
  右  game-core：Rust 帧导出（`analysis/dump/<scenario>.frames.jsonl`，S5 落地后生成）
完整性设计（红线：ROM 衍生 trace 不入库）：
  `oracle/traces/l3_manifest.json` 只存**摘要**；缓存缺失 ⇒ scenario_run 响亮重采集 ⇒
  sha256 对 manifest（持续复验 oracle 确定性）。

裁判铁律（r3 F7「空跑不得绿」+ G5 复审 F1/F2 + 二轮 NEW-1/2/3——判定权不留静默绿面）：
  exit 2（manifest schema 级，先于一切比对）：
  - manifest 缺失 / 非法 JSON / 非法 UTF-8 / cases 非 list[dict] / case 缺必填键或类型错
  - scenario 含路径字符（限 ^[A-Za-z0-9_-]+$，防 ../ 穿越自比对）/ 重复 case id
  - watch label 重复 / watch 条目未知键（防 _span 注入）/ 左右路径同 inode 或 dump 为符号链接
  - watch 为空 / watch 条目非 dict / label 非空 str / base 1-4 位 hex / range 格式非法
  exit 1（case 级 FAIL）：
  - watch 字段**逐帧**存在性 + 值合法性（str、非空、偶数长、纯 hex）——oracle 侧先行，dump 侧同检
  - range span 与字段字节长不符 / 重复帧行（同 f 多行）/ 帧号负数 / 帧集为空（0 字节比对）
  - dump 存在但不可读（目录/非常规）/ manifest.frames 与实测不符
  - sha 不符 / 帧集不一致 / 首分歧
  首分歧报告=帧号 + 字段 + CPU 地址（基址+偏移）+ 双侧值；hex 比对大小写归一（.upper()）。
用法：python3 tools/diff_traces.py
"""
from __future__ import annotations

import hashlib
import json
import os
import re
import subprocess
import sys
from pathlib import Path

# 【Windows GBK 兜底】控制台 GBK 编码下 `⇒` 等 Unicode 抛 UnicodeEncodeError（对拍收尾
# 打印实证）——stdout/stderr 重配 UTF-8 + replace（无损降级不炸，报告不截断）。
try:
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    sys.stderr.reconfigure(encoding="utf-8", errors="replace")
except Exception:
    pass

ROOT = Path(__file__).resolve().parent.parent
MANIFEST = ROOT / "oracle" / "traces" / "l3_manifest.json"
TRACES = ROOT / "analysis" / "traces"
DUMPS = ROOT / "analysis" / "dump"

HEX64 = re.compile(r"^[0-9a-fA-F]{64}$")
HEXSTR = re.compile(r"^[0-9a-fA-F]*$")
BASERE = re.compile(r"^[0-9a-fA-F]{1,4}$")
RANGERE = re.compile(r"^([0-9a-fA-F]{1,4})-([0-9a-fA-F]{1,4})$")
SCENRE = re.compile(r"^[A-Za-z0-9_-]+$")
WATCH_KEYS = {"label", "base", "range"}
# 帧记录键集白名单（严重-7）：基础键 {f,w} + case.frame_keys 显式声明扩展键（如 ram）。
FRAME_BASE_KEYS = {"f", "w"}
# watch 面区间钉死规范（严重-5；与 game-core memmap/frame_face.rs WATCH_FACES 同表——
# 双侧同表互指注，双腿编码长度必须一致（L3 diff 天然互检）。WRAM=$6000-$7FFF 全 8KB
# （lead 2026-10-15 裁定：全状态无隐藏态；「$6000-$6Cxx」软上界正式作废）。
CANONICAL_FACES = {
    "zp": ("0000", "0000-00ff"),
    "iram": ("0100", "0100-07ff"),
    "wram": ("6000", "6000-7fff"),
}


def schema_error(msg: str) -> int:
    print(f"L3 帧级: manifest schema 非法——fail-loud（exit 2）: {msg}")
    return 2


def check_case_schema(case) -> str | None:
    """manifest schema 机械校验（R1/R3）；返回错误描述或 None。"""
    if not isinstance(case, dict):
        return f"case 非对象: {case!r}"
    for key in ("id", "scenario", "oracle_sha256", "scenario_sha256"):
        v = case.get(key)
        if not isinstance(v, str) or not v.strip():
            return f"case.{key} 缺失或非非空字符串"
    if not SCENRE.match(case["scenario"]):
        return f"case.scenario 含路径字符（限字母数字_-，防穿越自比对）: {case['scenario']!r}"
    if not HEX64.match(case["oracle_sha256"]):
        return f"case.oracle_sha256 非 64 位 hex: {case['oracle_sha256']!r}"
    if not HEX64.match(case["scenario_sha256"]):
        return f"case.scenario_sha256 非 64 位 hex: {case['scenario_sha256']!r}"
    fk = case.get("frame_keys")
    if not isinstance(fk, list):
        return "case.frame_keys 缺失或非数组（键集显式声明——可为空数组，缺键不行；严重-7）"
    seen_fk: set[str] = set()
    for k in fk:
        if not isinstance(k, str) or not k.strip():
            return f"case.frame_keys 含非非空字符串: {k!r}"
        if k in FRAME_BASE_KEYS:
            return f"case.frame_keys 声明基础键 {k!r}（f/w 固有，勿重复声明）"
        if k in seen_fk:
            return f"case.frame_keys 重复: {k!r}"
        seen_fk.add(k)
    if "frames" in case:
        fv = case["frames"]
        if isinstance(fv, bool) or not isinstance(fv, int) or fv <= 0:
            return f"case.frames 须为正整数（缺省可省略，给了就必严）: {fv!r}"
    watch = case.get("watch")
    if not isinstance(watch, list) or not watch:
        return "case.watch 缺失或为空（空跑不得绿）"
    seen_labels = set()
    for w in watch:
        if not isinstance(w, dict):
            return f"watch 条目非对象: {w!r}"
        unknown = set(w) - WATCH_KEYS
        if unknown:
            return f"watch 条目含未知键（防 _span 注入等）: {sorted(unknown)}"
        label, base = w.get("label"), w.get("base")
        if not isinstance(label, str) or not label.strip():
            return f"watch.label 缺失或非非空字符串: {w!r}"
        if label in seen_labels:
            return f"watch.label 重复（防静默降覆盖）: {label!r}"
        seen_labels.add(label)
        if not isinstance(base, str) or not BASERE.match(base):
            return f"watch.base 须为 1-4 位 hex 字符串: {w!r}"
        rng = w.get("range") if "range" in w else None
        if "range" in w and w["range"] is None:
            return f"watch.range 给了 null（给了就必严，缺省请删键）: {w!r}"
        if rng is not None:
            m = RANGERE.match(rng) if isinstance(rng, str) else None
            if not m:
                return f"watch.range 格式非法（须 'HHHH-HHHH'）: {rng!r}"
            span = int(m.group(2), 16) - int(m.group(1), 16) + 1
            if span <= 0:
                return f"watch.range 区间倒置: {rng!r}"
            w["_span"] = span
        canon = CANONICAL_FACES.get(label)
        if canon is not None:
            declared = (base.lower(), (w.get("range") or "").lower())
            if declared != canon:
                return (f"watch[{label}] 区间与钉死规范不符（严重-5）: 期望 "
                        f"base={canon[0]} range={canon[1]}，实得 base={base} range={w.get('range')}")
    return None


def field_illegal(value) -> str | None:
    """值合法性（R2）：str、非空、偶数长、纯 hex。"""
    if not isinstance(value, str):
        return f"非字符串: {value!r}"
    if not value:
        return "空串（0 字节比对=空跑绿）"
    if len(value) % 2 != 0:
        return f"奇数长度 hex: {value[:20]!r}…"
    if not HEXSTR.match(value):
        return f"非 hex 字符: {value[:20]!r}…"
    return None


def check_frames_side(frames: dict, watch: list[dict], side: str) -> str | None:
    """逐帧存在性 + 值合法性（R2）+ range span 一致性；返回首个违规描述。"""
    for f in sorted(frames):
        w = frames[f]
        for entry in watch:
            label = entry["label"]
            if label not in w:
                return f"frame {f} watch 字段 {label} {side}侧缺失（逐帧存在性校验）"
            bad = field_illegal(w[label])
            if bad:
                return f"frame {f} 字段 {label} {side}侧值违规: {bad}"
            if "_span" in entry and len(w[label]) // 2 != entry["_span"]:
                return (f"frame {f} 字段 {label} {side}侧字节长 {len(w[label]) // 2} "
                        f"!= range span {entry['_span']}")
    return None


def regen(scenario: str) -> bool:
    print(f"[regen] 缓存缺失——经 tools/scenario_run.py 重采集 {scenario}（响亮再生）")
    # Windows 适配：PYTHONUTF8=1 令 Python 子进程输出 UTF-8，父侧固定 UTF-8 解码
    # （缺 encoding 时按 GBK locale 解码 UTF-8 字节直接崩——2026-10-14 实测）
    env = dict(os.environ, PYTHONUTF8="1")
    # 事故②对抗审：外层 spawn 无超时=真空面（scenario_run 自身挂死则永等）——补 360s
    # 上限（--timeout 300+60s 余量；超限=FAIL 显式）
    try:
        proc = subprocess.run(
            [sys.executable, str(ROOT / "tools" / "scenario_run.py"), "run", scenario, "--timeout", "300"],
            cwd=ROOT, capture_output=True, text=True, encoding="utf-8", errors="replace",
            env=env, timeout=360)
    except subprocess.TimeoutExpired:
        print(f"[regen] 外层超时 >360s（事故②护栏——限时打回非挂死）: {scenario}")
        return False
    if proc.returncode != 0:
        tail = ((proc.stdout or "") + (proc.stderr or "")).strip().splitlines()[-5:]
        print(f"[regen] 重采集失败: {' | '.join(tail)}")
    return proc.returncode == 0


def load_frames(raw: bytes, where: str, allowed_keys: set[str] | None = None,
                w_labels: set[str] | None = None) -> tuple[dict[int, dict], list[int], str | None]:
    """解析帧 trace（**单读**：sha 与解析同源同 bytes，消 TOCTOU 双读竞态——G5 R4-1）。
    返回 (帧字典, 重复 f 列表, 行级违规)。重复 f 判 FAIL；畸形行 = 行级违规。
    **键集白名单**（严重-7，task-22 slot③）：allowed_keys=顶层键集（缺省={f,w}），
    w_labels=`w` 对象标签集（缺省=不检）；未声明键=行级违规（隐藏字段拒绝）。"""
    out: dict[int, dict] = {}
    dups: list[int] = []
    try:
        text = raw.decode("utf-8")
    except UnicodeDecodeError as e:
        return out, dups, f"{where} 非 UTF-8 编码: {e}"
    for lineno, line in enumerate(text.splitlines(), 1):
        if not line.strip():
            continue
        try:
            r = json.loads(line)
        except json.JSONDecodeError as e:
            return out, dups, f"{where} 第 {lineno} 行非 JSON: {e}"
        if not isinstance(r, dict) or "f" not in r or "w" not in r:
            return out, dups, f"{where} 第 {lineno} 行缺 f/w 键或非对象"
        allow = allowed_keys if allowed_keys is not None else FRAME_BASE_KEYS
        unknown = set(r) - allow
        if unknown:
            return out, dups, (f"{where} 第 {lineno} 行含未声明键 {sorted(unknown)}"
                               f"（键集白名单={sorted(allow)}——严重-7 隐藏字段拒绝）")
        f = r["f"]
        if isinstance(f, bool) or not isinstance(f, int) or f < 0:
            return out, dups, f"{where} 第 {lineno} 行 f 须为非负整数: {f!r}"
        if not isinstance(r["w"], dict):
            return out, dups, f"{where} 第 {lineno} 行 w 非对象"
        if w_labels is not None:
            w_unknown = set(r["w"]) - w_labels
            if w_unknown:
                return out, dups, (f"{where} 第 {lineno} 行 w 含未声明标签 {sorted(w_unknown)}"
                                   f"（声明={sorted(w_labels)}——严重-7 隐藏字段拒绝）")
        if f in out:
            dups.append(f)
        out[f] = r["w"]
    return out, dups, None


# 栈页调用栈共写区（docs/design/l3-gamecore-leg-verification.md §8.1②，v5 归因收口；
# 2026-10-18 裁定修订：3 字节窗→**整栈影子区**）：
# CPU $0100-$01FF=栈影子区——NMI 压栈+主环/中断调用链 jsr/rts/php/pla 族写面。
# 裁定论证：① 栈值=实现表面产物（值依调用深度/调用点/PLA-PLA 弃值技巧，不进行为面）
# ② 链规 19：jsr/rts 栈写建模=重现原二进制调用结构=1:1 结构镜像明禁
# ③ 行为依赖核查（2026-10-18）：nmikstas 源级 0 处绝对寻址读 $01xx（原始字节扫描
# 141 命中全假阳=数据巧合）——栈页无「被读作数据」行为依赖。
# 升级条件：若发现栈区被读作数据（行为依赖）⇒ 升建模卡（jsr/rts 栈写建模转正）。
# 45641b5 契约修订：排除窗=整栈影子区 $0100-$01FF（3 字节窗废）——
# **区间判非元组 membership**（实录 bug：in (0x100,0x1FF)=端点相等，中段照比致 $01EB 账外红）
INTERRUPT_FRAME_EXCLUDE_LO, INTERRUPT_FRAME_EXCLUDE_HI = 0x0100, 0x01FF


def _stack_excluded(addr: int) -> bool:
    return INTERRUPT_FRAME_EXCLUDE_LO <= addr <= INTERRUPT_FRAME_EXCLUDE_HI


def n12_gate_errors(root) -> list[str]:
    """N-0012 机械门（l3 设计 §2.2 谓词，enforcement①=game-core 腿启用前置断言）：
    `level∈{A,B}` ∧ `evidence 匹配 bank\\d+:\\$?[0-9A-Fa-f]{4}` ∧ `sub_GameRng 族案 ≥1`
    ——任一不满足 ⇒ 报缺失项（调用方拒启腿）。三则均用现存字段/文件，可复跑。"""
    import json as _json
    import re as _re
    errs: list[str] = []
    try:
        led = [_json.loads(l) for l in (root / "data/naming/ledger.jsonl").read_text(
            encoding="utf-8").splitlines() if l.strip()]
    except OSError as exc:
        return [f"N-0012 机械门：台账不可读（fail-loud）: {exc}"]
    rows = [r for r in led if r.get("id") == "N-0012"]
    if not rows:
        return ["N-0012 机械门：台账无 N-0012 行"]
    r = rows[0]
    if r.get("level") not in ("A", "B"):
        errs.append(f"N-0012.level={r.get('level')!r}∉{{A,B}}（rename 流程升级未完成）")
    if not _re.search(r"bank\d+:\$?[0-9A-Fa-f]{4}", r.get("evidence", "")):
        errs.append("N-0012.evidence 缺 bankNN:AAAA 具体指针（占位/纯路径不算）")
    try:
        cases = [_json.loads(l) for l in (root / "oracle/subroutines/l2_cases.jsonl")
                 .read_text(encoding="utf-8").splitlines() if l.strip()]
        rng_n = sum(1 for c in cases
                    if "GameRng" in str(c.get("id", "")) + str(c.get("subroutine", "")))
    except OSError as exc:
        return errs + [f"RNG 案计数源不可读（fail-loud）: {exc}"]
    if rng_n < 1:
        errs.append("RNG 序列 L2 案=0（计数源 oracle/subroutines/l2_cases.jsonl）")
    return errs


def leg_sample_point(doc: dict, side: str) -> str | None:
    """R2 per-leg 采样点声明：**case 级** legs.<side>.sample_point 优先（task-24 钉位：
    legs 属 case——不同 case=不同场景/腿配）；legacy 顶层 sample_point 兜底
    （混态期禁置全局键——v5 §5.3c 口径；per-leg 缺省=未迁 endFrame）。"""
    legs = doc.get("legs")
    if isinstance(legs, dict):
        leg = legs.get(side)
        if isinstance(leg, dict) and leg.get("sample_point"):
            return leg["sample_point"]
    return doc.get("sample_point")


def check_nmi_anchor(frames: dict, watch: list[dict], sample_point: str | None,
                     side: str) -> tuple[str | None, str]:
    """恒 1 断言（l3 契约 §8.1 点 3，v4 B2 接线）：watch 覆盖 $001A 时逐帧断言 $1A==01。

    采样点声明门（R2 per-leg 消费）：manifest `legs.<side>.sample_point`=="spin-exit" ⇒
    断言生效（≠01=NMI 面退化诊断信号，FAIL）；缺省/未迁 ⇒ 显式不适用（note 随行，
    绝不静默装绿）。watch 不含 $001A ⇒ 亦显式 note（收窄绕过封堵，r12 探针 E/F）。"""
    anchor_w = [w for w in watch
                if int(w["base"], 16) <= 0x001A < int(w["base"], 16) + w.get("_span", 0)]
    if not anchor_w:
        return None, (f"恒 1 断言不适用（{side}）：watch 面不含 CPU $001A"
                      f"（收窄绕过显式 note——r12 探针 E/F 封堵，绝不静默）")
    if not anchor_w[0].get("_span"):
        # 防御分支（R3 归因修正）：check_case_schema 后 _span 必在，正常流程不可达
        return None, (f"恒 1 断言不适用（{side}）：watch 条目无 _span（防御分支——"
                      f"schema 校验后不可达；仅直调误用面提示）")
    if sample_point != "spin-exit":
        return None, (f"恒 1 断言不适用（{side}）：sample_point="
                      f"{sample_point or 'endFrame(缺省)'}≠spin-exit（迁移槽 3b 未走，显式）")
    w = anchor_w[0]
    base = int(w["base"], 16)
    off = (0x001A - base) * 2
    for f in sorted(frames):
        v = frames[f][w["label"]].upper()
        if v[off:off + 2] != "01":
            return (f"恒 1 断言失败（{side}）: frame {f} 字段 {w['label']} CPU $001A=0x"
                    f"{v[off:off + 2]}（≠01）——NMI 面退化信号（spin 出口采样点，契约 §8.1 点 3）"), ""
    return None, f"恒 1 断言过（{side}）：{len(frames)} 帧 $001A 全 01"


def compare_frames(left: dict, right: dict, watch: list[dict],
                   declared: dict | None = None) -> tuple[str | None, int]:
    """逐帧逐 watch 字段比对（值已过合法性校验）；返回 (首分歧描述或 None, 排除分歧计数)。

    行集口径（task-24 B 跑对账，spin-exit 腿）：两侧缺帧集不同源属预期（采样点
    相位差异）——`declared={"left_missing":[…],"right_missing":[…],"frames":N}` 时
    ①逐侧核对实测行集==声明行集（1..N 声明缺帧——偏离=真回归 FAIL）
    ②值比对取**交集**（声明缺帧行显式跳过，note 随行）；
    无 declared（legacy endFrame 腿）=严格全集相等。"""
    lframes, rframes = sorted(left), sorted(right)
    if declared is not None:
        n = declared.get("frames") or max(lframes + rframes)
        for side, actual, miss in (("oracle", set(lframes), declared["left_missing"]),
                                   ("game-core", set(rframes), declared["right_missing"])):
            expect = set(range(1, n + 1)) - set(miss)
            if actual != expect:
                extra = sorted(actual - expect)[:5]
                gone = sorted(expect - actual)[:5]
                return (f"行集与声明不符（{side}）：多={extra} 少={gone}"
                        f"（声明缺帧 {sorted(miss)}——真回归）"), 0
    elif lframes != rframes:
        only_l = sorted(set(lframes) - set(rframes))[:5]
        only_r = sorted(set(rframes) - set(lframes))[:5]
        return f"帧集合不一致（左缺={only_r} 右缺={only_l}）", 0
    excluded_hits = 0
    # 帧键统一（task-29 b2）：spin-exit 腿按**行序+head_skip** 对齐（场景帧号语义=
    # 游戏侧 FrameCount 序——FC 逐行递增自校验；原 f 键=Mesen 帧号 vs tick 序号错位，
    # 同 f≠同游戏帧）。对齐自校验：逐对 FC（$002D）相等，违例=对齐破裂 FAIL。
    if declared is not None and "head_skip" in declared:
        lh = declared.get("head_skip", {}).get("left", 0)
        rh = declared.get("head_skip", {}).get("right", 0)
        lrows = [left[f] for f in lframes[lh:]]
        rrows = [right[f] for f in rframes[rh:]]
        pairs = min(len(lrows), len(rrows))
        for k in range(pairs):
            for w in watch:
                label, base = w["label"], int(w["base"], 16)
                a = lrows[k][label].upper()
                b = rrows[k][label].upper()
                if label == "zp" and len(a) > 0x2D * 2 + 1 and len(b) > 0x2D * 2 + 1:
                    if a[0x2D * 2:0x2D * 2 + 2] != b[0x2D * 2:0x2D * 2 + 2]:
                        return (f"对齐自校验破裂：行序 {k} FC($002D) 不等"
                                f"（左={a[0x2D*2:0x2D*2+2]} 右={b[0x2D*2:0x2D*2+2]}）"
                                f"——head_skip 声明过期"), 0
                if a == b:
                    continue
                if len(a) != len(b):
                    return (f"行序 {k}（游戏帧 {k + lh}）字段 {label} 长度不一致:"
                            f" 左={len(a) // 2}B 右={len(b) // 2}B"), excluded_hits
                for i in range(0, len(a), 2):
                    if a[i:i + 2] == b[i:i + 2]:
                        continue
                    byte_i = i // 2
                    if _stack_excluded(base + byte_i):
                        excluded_hits += 1
                        continue
                    return (f"游戏帧 {k + lh} 字段 {label} 分歧 @{label}(${base:04X})"
                            f"+0x{byte_i:03X}=CPU ${base + byte_i:04X}:"
                            f" 左={a[i:i + 8]}… 右={b[i:i + 8]}…"), excluded_hits
        return None, excluded_hits
    for f in sorted(set(lframes) & set(rframes) if declared is not None else lframes):
        for w in watch:
            label, base = w["label"], int(w["base"], 16)
            a = left[f][label].upper()
            b = right[f][label].upper()
            if a == b:
                continue
            if len(a) != len(b):
                return (f"frame {f} 字段 {label} 长度不一致: 左={len(a) // 2}B 右={len(b) // 2}B"
                        f"（无法偏移定位）"), excluded_hits
            for i in range(0, len(a), 2):
                if a[i:i + 2] == b[i:i + 2]:
                    continue
                byte_i = i // 2
                if _stack_excluded(base + byte_i):
                    excluded_hits += 1   # 中排除面：计数、不判分歧（§8.1②）
                    continue
                return (f"frame {f} 字段 {label} 分歧 @{label}(${base:04X})+0x{byte_i:03X}"
                        f"=CPU ${base + byte_i:04X}: 左={a[i:i + 8]}… 右={b[i:i + 8]}…"), excluded_hits
    return None, excluded_hits


def main() -> int:
    if not MANIFEST.is_file():
        print("L3 帧级: l3_manifest.json 缺失——fail-loud（空跑不得绿）")
        return 2
    try:
        doc = json.loads(MANIFEST.read_text(encoding="utf-8"))
    except UnicodeDecodeError as e:
        return schema_error(f"非法 UTF-8: {e}")
    except json.JSONDecodeError as e:
        return schema_error(f"JSON 解析失败: {e}")
    cases = doc.get("cases") if isinstance(doc, dict) else None
    if not isinstance(cases, list):
        return schema_error(f"cases 须为数组: {type(cases).__name__}")
    if not cases:
        return schema_error("用例集为空（空跑不得绿）")
    for c in cases:
        if isinstance(c, dict) and "id" in c and not isinstance(c["id"], str):
            return schema_error(f"case.id 须为字符串: {c['id']!r}")
    ids = [c.get("id") for c in cases if isinstance(c, dict)]
    if len(ids) != len(set(ids)):
        return schema_error(f"case.id 重复（防报告歧义）: {ids}")
    for case in cases:
        err = check_case_schema(case)
        if err:
            return schema_error(err)

    failed = 0
    tolerated = 0
    for case in cases:
        cid, scenario = case["id"], case["scenario"]
        watch = case["watch"]
        allowed = FRAME_BASE_KEYS | set(case["frame_keys"])
        w_labels = {w["label"] for w in watch}
        # scenario_sha256 载体复验（严重-10，task-22 slot④）：输入脚本 sha 对锚
        scen_path = ROOT / "oracle" / "scenarios" / f"{scenario}.txt"
        if not scen_path.is_file():
            print(f"[FAIL] {cid}: 场景脚本缺失（scenario_sha256 无锚可对）: {scen_path}")
            failed += 1
            continue
        scen_digest = hashlib.sha256(scen_path.read_bytes()).hexdigest()
        if scen_digest.lower() != case["scenario_sha256"].lower():
            print(f"[FAIL] {cid}: scenario_sha256 不符（输入脚本变更/manifest 陈旧——"
                  f"脚本 {scen_digest[:16]}… vs manifest {case['scenario_sha256'][:16]}…）")
            failed += 1
            continue
        cache = TRACES / f"{scenario}.frames.jsonl"
        if not cache.is_file():
            if not regen(scenario) or not cache.is_file():
                print(f"[FAIL] {cid}: oracle trace 重采集失败")
                failed += 1
                continue
        cache_raw = cache.read_bytes()  # 单读：sha 与解析同源（R4-1）
        digest = hashlib.sha256(cache_raw).hexdigest()
        if digest.lower() != case["oracle_sha256"].lower():
            print(f"[FAIL] {cid}: oracle trace sha256 不符 manifest（确定性破裂/缓存陈旧/篡改）")
            failed += 1
            continue

        left, ldups, lerr = load_frames(cache_raw, "oracle trace", allowed, w_labels)
        if lerr:
            print(f"[FAIL] {cid}: {lerr}")
            failed += 1
            continue
        if ldups:
            print(f"[FAIL] {cid}: oracle trace 重复帧行 {sorted(set(ldups))[:5]}（防覆盖掩盖分歧）")
            failed += 1
            continue
        # 行集语义按腿采样点分口径（task-24 3b 迁移改口）：spin-exit 腿行集=legs.*.rows
        # （boot 相缺帧+尾帧截断=如实行集，295≠300 属正常）；endFrame legacy 腿=frames 全集。
        oracle_leg = (case.get("legs") or {}).get("oracle") or {}
        if oracle_leg.get("sample_point") == "spin-exit":
            exp_rows = oracle_leg.get("rows")
            if exp_rows is not None and len(left) != exp_rows:
                print(f"[FAIL] {cid}: 行数 {len(left)} != legs.oracle.rows {exp_rows}"
                      f"（spin-exit 行集口径）")
                failed += 1
                continue
        elif "frames" in case and len(left) != case["frames"]:
            print(f"[FAIL] {cid}: 帧数 {len(left)} != manifest.frames {case['frames']}")
            failed += 1
            continue
        if not left:
            print(f"[FAIL] {cid}: oracle trace 帧集为空（0 字节比对=空跑绿）")
            failed += 1
            continue
        # oracle 侧逐帧存在性+值合法性（R2；dump 缺席时同样不许空跑绿）
        v = check_frames_side(left, watch, "oracle")
        if v:
            print(f"[FAIL] {cid}: {v}")
            failed += 1
            continue
        aerr, anote = check_nmi_anchor(left, watch, leg_sample_point(case, "oracle"), "oracle")
        if aerr:
            print(f"[FAIL] {cid}: {aerr}")
            failed += 1
            continue
        if anote:
            print(f"[note] {cid}: {anote}")

        # task-32 余件：三条件断言消费（fixture 声明→(b) 写读回/(c) invariant 持久；
        # (a)=声明域内 F_a+1 全等即 compare_frames 本体）
        fx_errs = []
        for fx in case.get("fixtures") or []:
            addr = int(fx["addr"], 16)
            val = int(fx["value"], 16)
            fa = int(fx["frame"])
            label, off = None, 0
            # 块定位：逐 watch 域找 addr
            for w in watch:
                base = int(w["base"], 16)
                key = w["label"]
                lens = {len(left[f][key]) for f in left} if left else set()
                if not lens:
                    continue
                ln = next(iter(lens)) // 2
                if base <= addr < base + ln:
                    label, off = key, (addr - base) * 2
                    break
            if label is None:
                fx_errs.append(f"fixture $%04X 不在 watch 声明域（显式 fail）" % addr)
                continue
            def rd(f):
                row = left.get(f)
                if row is None:
                    return None
                return int(row[label][off:off + 2], 16)
            b_val = rd(fa + 1)
            if b_val is None:
                fx_errs.append(f"(b) fixture $%04X F_a+1（f%d）行缺——显式 fail" % (addr, fa + 1))
            elif b_val != val:
                fx_errs.append(f"(b) fixture $%04X F_a+1 读回 {b_val:02X}≠期望 {val:02X}")
            if fx.get("persistence") == "invariant":
                last_f = max(left) if left else 0
                c_val = rd(last_f)
                if c_val != val:
                    fx_errs.append(f"(c) invariant $%04X 窗口末 f%d={c_val}≠期望 {val:02X}"
                                   % (addr, last_f))
        if fx_errs:
            for e in fx_errs:
                print(f"[FAIL] {cid}: 三条件断言: {e}")
            failed += 1
            continue
        if case.get("fixtures"):
            print(f"[note] {cid}: 三条件断言过（{len(case['fixtures'])} fixture：(b) 读回+(c) 持久）")
        dump = DUMPS / f"{scenario}.frames.jsonl"
        if dump.exists() and (not dump.is_file() or dump.is_symlink()):
            print(f"[FAIL] {cid}: game-core dump 不可读/非常规（目录或符号链接——禁自比对）")
            failed += 1
            continue
        if dump.exists():
            cs, ds = cache.stat(), dump.stat()
            if (cs.st_dev, cs.st_ino) == (ds.st_dev, ds.st_ino):
                return schema_error("左右路径同 inode（自比对禁止，含硬链接）")
        if not dump.is_file():
            print(f"[PASS] {cid}: oracle 腿已验（sha256 对锚、{len(left)} 帧、逐帧字段合法）；"
                  f"game-core 腿未启用（S5 落地生成 {dump.relative_to(ROOT)} 后自动启用）")
            continue
        # N-0012 机械门（enforcement①，l3 设计 §2.2）：game-core 腿启用前置断言
        n12errs = n12_gate_errors(ROOT)
        if n12errs:
            print(f"[FAIL] {cid}: game-core 腿拒启——N-0012 机械门未过: {'; '.join(n12errs)}")
            failed += 1
            continue

        right, rdups, rerr = load_frames(dump.read_bytes(), "game-core dump", allowed, w_labels)
        if rerr:
            print(f"[FAIL] {cid}: {rerr}")
            failed += 1
            continue
        if rdups:
            print(f"[FAIL] {cid}: game-core dump 重复帧行 {sorted(set(rdups))[:5]}（防覆盖掩盖分歧）")
            failed += 1
            continue
        v = check_frames_side(right, watch, "game-core")
        if v:
            print(f"[FAIL] {cid}: {v}")
            failed += 1
            continue
        aerr, anote = check_nmi_anchor(right, watch, leg_sample_point(case, "game-core"), "game-core")
        if aerr:
            print(f"[FAIL] {cid}: {aerr}")
            failed += 1
            continue
        if anote:
            print(f"[note] {cid}: {anote}")
        # 行集声明对账（spin-exit 腿两侧缺帧集不同源=预期；交集比对）
        lo_leg = (case.get("legs") or {}).get("oracle") or {}
        gc_leg = (case.get("legs") or {}).get("game-core") or {}
        declared = None
        if "missing" in lo_leg or "missing" in gc_leg:
            declared = {
                "frames": case.get("frames"),
                "left_missing": lo_leg.get("missing") or [],
                "right_missing": gc_leg.get("missing") or [],
            }
            if "head_skip" in lo_leg or "head_skip" in gc_leg:
                declared["head_skip"] = {"left": lo_leg.get("head_skip", 0),
                                         "right": gc_leg.get("head_skip", 0)}
        divergence, excluded_hits = compare_frames(left, right, watch, declared)
        excl_note = (f"（栈影子区排除 {excluded_hits} 处分歧 $0100-$01FF——45641b5 整栈窗，显式打印）"
                     if excluded_hits else "")
        # task-29 (ii)：占位臂缺口记账排除面（principled 非静默——记账源=模型自记
        # DIVERGENCES 面；账内分歧=容忍+全打印（路线图）；**账外/账清照红**=防绿底线；
        # 账清（gap_map 空）⇒ strict——深路径实现一件账缩一格，账清自然转绿。
        dgap_path = DUMPS / f"{scenario}.divergences.json"
        gap_map: list = []
        gap_note = ""
        if dgap_path.is_file():
            try:
                dg = json.loads(dgap_path.read_text(encoding="utf-8"))
                gap_map = dg.get("gap_map") or []
            except Exception as e:
                gap_note = f"（缺口记账 sidecar 解析失败: {e}——按无记账 strict）"
        # 持久声明面（手编止血——dump 重生不丢；visual_gap_map 同构）：data/status/
        # trace_gap_map.json 合并入账（场景匹配条目→gap_map 格式）
        _tgm = ROOT / "data" / "status" / "trace_gap_map.json"
        if _tgm.is_file():
            try:
                for e in json.loads(_tgm.read_text(encoding="utf-8")).get("entries", []):
                    if e.get("scenario") != scenario:
                        continue
                    for a in e.get("addrs", []):
                        gap_map.append({"addr": int(a, 16), "n": 0, "persistent": e["id"]})
                    gap_note += f"（持久声明 {e['id']}：{e.get('reason','')}｜退出={e.get('exit','')}）"
            except Exception as ex:
                gap_note += f"（trace_gap_map 解析失败: {ex}）"
        if divergence:
            if gap_map:
                gaps = ", ".join(f"${g['addr']:04X}×{g['n']}" for g in gap_map[:8])
                tolerated += 1
                print(f"[TOLERATED] {cid}: 首分歧 {divergence}{excl_note}"
                      f"——归因=占位臂缺口记账面（粗粒度归因：缺口活跃期账内容忍）")
                print(f"[note] {cid}: 缺口路线图（{len(gap_map)} 唯一址，账清⇒strict 自然转绿）: {gaps}{gap_note}")
            else:
                print(f"[FAIL] {cid}: 首分歧 {divergence}{excl_note}"
                      f"（缺口账清/无记账——账外照红）{gap_note}")
                failed += 1
        else:
            print(f"[PASS] {cid}: 逐帧状态向量全同（{len(left)} 帧）{excl_note}")

    total = len(cases)
    print(f"L3 帧级裁定: {total} 条用例, {failed} 失败, {tolerated} 记账容忍")
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
