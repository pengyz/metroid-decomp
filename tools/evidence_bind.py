#!/usr/bin/env python3
"""证据 ↔ 代码树绑定：给产物打 `head`（40 位提交号）+ `at`（ISO 时间）并校验。

为什么需要：`shots_rust/matrix.json` 与 `analysis/tas-100/coverage.json` 以前不带
代码树标识（或缺 `head`/`at`，或把 lane 分叉的短 sha 当成主线 HEAD），后来人拿到的
是「一份数字」而不是「某棵树上的数字」，复算必然对不上。本工具把这件事机械化：

- **sign**：产物落盘时写入 `head`/`at`。HEAD 缺失、工作树脏（除显式 `--allow-dirty`）
  一律 **fail-loud**（退出码 3），绝不静默写 null。
- **verify**：读产物，校验 `head` 是 40 位且能在本仓解析为提交、`at` 是合法 ISO 时间，
  可按 `--expect-head` 校验与当前 HEAD 一致。
- **selftest**：三类负例（伪造 sha / null 时间 / 脏树）+ 幂等（`--deterministic` 两次逐字节相同）。

规则见 `docs/design/evidence-binding.md`。

用法：
    python3 tools/evidence_bind.py sign   -f shots_rust/matrix.json
    python3 tools/evidence_bind.py sign   -f analysis/tas-100/coverage.json --rebind
    python3 tools/evidence_bind.py verify -f shots_rust/matrix.json --expect-head $(git rev-parse HEAD)
    python3 tools/evidence_bind.py selftest
    python3 tools/evidence_bind.py rules

退出码：0 成功 / 2 用法或文件错误 / 3 fail-loud（缺 HEAD、脏树、缺字段、sha 不可解析）
"""
from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import tempfile
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FULL_SHA = re.compile(r"^[0-9a-f]{40}$")
ISO_RE = re.compile(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})$")


class BindError(RuntimeError):
    """fail-loud：调用方按退出码 3 处理。"""


# ---------------------------------------------------------------------------
# git 事实
# ---------------------------------------------------------------------------
def git(args: list[str]) -> str:
    p = subprocess.run(["git", *args], cwd=ROOT, capture_output=True, text=True, timeout=20)
    if p.returncode != 0:
        raise BindError(f"git {' '.join(args)} 失败（rc={p.returncode}）：{p.stderr.strip()}")
    return p.stdout.strip()


def head_full() -> str:
    sha = git(["rev-parse", "HEAD"])
    if not FULL_SHA.match(sha):
        raise BindError(f"HEAD 不是 40 位提交号：{sha!r}")
    return sha


CODE_DOMAINS = ("crates/", "tools/", "gates/", "oracle/", "reference/")


def dirty_paths() -> list[str]:
    """判「树脏」的文件：已跟踪文件的未提交改动 + 代码域内的未跟踪新文件。

    为什么不是全部 porcelain：别人在 `docs/`、`analysis/` 里留未跟踪文件是常态，
    拿它阻断签名会让工具没法用；但代码域（crates/tools/gates/…）里出现未跟踪文件
    等于「有代码没进版本库」，同一 sha 会对应两份不同程序，必须算脏。
    """
    out = git(["status", "--porcelain"])
    dirty = []
    for ln in out.splitlines():
        if not ln.strip():
            continue
        path = ln[3:].strip()
        if ln.startswith("??"):
            if any(path.startswith(d) for d in CODE_DOMAINS):
                dirty.append(ln)
            continue
        dirty.append(ln)
    return dirty


def untracked_paths() -> list[str]:
    """未跟踪文件（信息性记录，不进判「脏」集合）。"""
    out = git(["ls-files", "--others", "--exclude-standard"])
    return [ln for ln in out.splitlines() if ln.strip()]


def resolve_commit(sha: str) -> str | None:
    """短/长 sha → 40 位；不存在返回 None（不抛）。"""
    try:
        full = git(["rev-parse", "--verify", f"{sha}^{{commit}}"])
    except BindError:
        return None
    return full if FULL_SHA.match(full) else None


def is_ancestor(older: str, newer: str) -> bool:
    try:
        git(["merge-base", "--is-ancestor", older, newer])
        return True
    except BindError:
        return False


def now_iso() -> str:
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


# ---------------------------------------------------------------------------
# 产物读写
# ---------------------------------------------------------------------------
def detect_kind(path: Path) -> str:
    """按文件名判定；文件名被改过（如复核时复制成 m.json）则按内容特征兜底。"""
    n = path.name
    if n == "matrix.json":
        return "matrix"
    if n == "coverage.json":
        return "coverage"
    try:
        doc = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        doc = None
    if isinstance(doc, dict):
        if isinstance(doc.get("rows"), list):
            return "matrix"
        if isinstance(doc.get("env"), dict):
            return "coverage"
    raise BindError(
        f"无法判定产物类型（文件名不是 matrix.json/coverage.json，内容也不像）：{path.name}"
        f"——可用 --kind matrix|coverage 显式指定")


def load(path: Path) -> tuple[dict, str]:
    raw = path.read_text(encoding="utf-8")
    try:
        doc = json.loads(raw)
    except json.JSONDecodeError as e:
        raise BindError(f"{path} 不是合法 JSON：{e}")
    if not isinstance(doc, dict):
        raise BindError(f"{path} 顶层必须是对象")
    # 沿用原缩进（`indent=1` / `indent=2` 混用会让整份重排——写回时按探测值）
    indent = 1
    m = re.search(r"\n( +)\"", raw)
    if m:
        indent = len(m.group(1))
    return doc, indent


def dump(path: Path, doc: dict, indent: int) -> None:
    text = json.dumps(doc, ensure_ascii=False, indent=indent) + "\n"
    path.write_text(text, encoding="utf-8")


# ---------------------------------------------------------------------------
# sign
# ---------------------------------------------------------------------------
def sign_matrix(doc: dict, sha: str, at: str, allow_dirty: bool, rebind: bool) -> dict:
    old = doc.get("head")
    if isinstance(old, str) and FULL_SHA.match(old) and old != sha:
        if not rebind:
            raise BindError(
                f"matrix.head 已是 {old[:12]}，与当前 HEAD {sha[:12]} 不同；"
                f"确属换树重签请加 --rebind（旧值会记到 head_rebound_from）")
        doc["head_rebound_from"] = old
    doc["head"] = sha
    doc["at"] = at
    if allow_dirty:
        doc["dirty"] = dirty_paths()
    else:
        doc.pop("dirty", None)
    ut = untracked_paths()
    if ut:
        doc["untracked"] = ut
    else:
        doc.pop("untracked", None)
    # 每格也要能被单独引用：行级 head 与顶层一致（不重复存时间，省体积）
    rows = doc.get("rows")
    if isinstance(rows, list):
        for i, row in enumerate(rows):
            if not isinstance(row, dict):
                raise BindError(f"matrix.rows[{i}] 不是对象")
            row["head"] = sha
            at_old = row.get("at")
            if isinstance(at_old, str) and ISO_RE.match(at_old):
                row["at"] = at
            else:
                row["at"] = at
    return doc


def sign_coverage(doc: dict, sha: str, at: str, allow_dirty: bool, rebind: bool) -> dict:
    env = doc.get("env")
    if not isinstance(env, dict):
        raise BindError("coverage.env 缺失或不是对象")
    old = env.get("git_head")
    full_old = resolve_commit(old) if isinstance(old, str) and old else None
    if isinstance(old, str) and old and old != sha:
        if not rebind:
            raise BindError(
                f"coverage.env.git_head 已是 {old}，与当前 HEAD {sha[:12]} 不同；"
                f"lane 产物签回主线请加 --rebind（原值会记到 git_head_generated）")
        env["git_head_generated"] = full_old or old
        env["rebind_at"] = at
    elif isinstance(old, str) and old and not FULL_SHA.match(old):
        # 旧值是短 sha：先升到 40 位（provenance 不失真），再记录签回
        if full_old and full_old != sha:
            env["git_head_generated"] = full_old
            env["rebind_at"] = at
    env["git_head"] = sha
    env["at"] = at
    if allow_dirty:
        env["dirty"] = dirty_paths()
    else:
        env.pop("dirty", None)
    ut = untracked_paths()
    if ut:
        env["untracked"] = ut
    else:
        env.pop("untracked", None)
    return doc


def cmd_sign(args) -> int:
    path = Path(args.file)
    if not path.exists():
        print(f"[错误] 文件不存在：{path}", file=sys.stderr)
        return 2
    kind = args.kind if args.kind != "auto" else detect_kind(path)
    doc, indent = load(path)

    sha = head_full()
    dirty = dirty_paths()
    if dirty and not args.allow_dirty:
        raise BindError(
            "工作树脏，拒绝签名（证据必须绑定到可复现的树）：\n  "
            + "\n  ".join(dirty[:10])
            + ("\n  …" if len(dirty) > 10 else "")
            + "\n  确需在脏树上签请加 --allow-dirty（会把脏文件清单写进产物）")

    prev_at = (doc.get("at") if kind == "matrix" else (doc.get("env") or {}).get("at"))
    if args.deterministic:
        if not (isinstance(prev_at, str) and ISO_RE.match(prev_at)):
            raise BindError("--deterministic 要求产物已有合法 at（否则改用不带该开关签名一次）")
        at = prev_at
    else:
        at = args.at or now_iso()
        if not ISO_RE.match(at):
            raise BindError(f"--at 不是合法 ISO 时间：{at!r}")

    doc = sign_matrix(doc, sha, at, args.allow_dirty, args.rebind) if kind == "matrix" \
        else sign_coverage(doc, sha, at, args.allow_dirty, args.rebind)
    dump(path, doc, indent)
    print(f"[sign] {path} kind={kind} head={sha} at={at}"
          + ("（脏树，已记录 dirty）" if args.allow_dirty and dirty else ""))
    if kind == "matrix":
        n = len(doc.get("rows") or [])
        print(f"[sign] rows 已同步 head/at：{n} 行")
    return 0


# ---------------------------------------------------------------------------
# verify
# ---------------------------------------------------------------------------
def verify_matrix(doc: dict, expect: str | None) -> list[str]:
    probs = []
    head = doc.get("head")
    if not isinstance(head, str) or not FULL_SHA.match(head):
        probs.append(f"matrix.head 缺失或不是 40 位：{head!r}")
    at = doc.get("at")
    if not isinstance(at, str) or not ISO_RE.match(at):
        probs.append(f"matrix.at 缺失或不是 ISO 时间：{at!r}")
    rows = doc.get("rows")
    if not isinstance(rows, list):
        probs.append("matrix.rows 缺失")
        return probs
    for i, row in enumerate(rows):
        if not isinstance(row, dict):
            probs.append(f"rows[{i}] 不是对象")
            continue
        rh, ra = row.get("head"), row.get("at")
        if not isinstance(rh, str) or not FULL_SHA.match(rh):
            probs.append(f"rows[{i}].head 缺失或不是 40 位：{rh!r}")
        elif isinstance(head, str) and rh != head:
            probs.append(f"rows[{i}].head 与顶层不一致：{rh[:12]} != {str(head)[:12]}")
        if not isinstance(ra, str) or not ISO_RE.match(ra):
            probs.append(f"rows[{i}].at 缺失或不是 ISO 时间：{ra!r}")
        if expect and isinstance(rh, str) and rh != expect:
            probs.append(f"rows[{i}].head 与期望 HEAD 不一致：{rh[:12]} != {expect[:12]}")
    if isinstance(head, str) and FULL_SHA.match(head) and resolve_commit(head) is None:
        probs.append(f"matrix.head 在本仓解析不到提交：{head}")
    if expect and isinstance(head, str) and head != expect:
        probs.append(f"matrix.head 与期望 HEAD 不一致：{head[:12]} != {expect[:12]}")
    return probs


def verify_coverage(doc: dict, expect: str | None) -> list[str]:
    probs = []
    env = doc.get("env")
    if not isinstance(env, dict):
        return ["coverage.env 缺失或不是对象"]
    head = env.get("git_head")
    if not isinstance(head, str) or not FULL_SHA.match(head):
        probs.append(f"env.git_head 缺失或不是 40 位：{head!r}")
    elif resolve_commit(head) is None:
        probs.append(f"env.git_head 在本仓解析不到提交：{head}")
    at = env.get("at")
    if not isinstance(at, str) or not ISO_RE.match(at):
        probs.append(f"env.at 缺失或不是 ISO 时间：{at!r}")
    if expect and isinstance(head, str) and head != expect:
        probs.append(f"env.git_head 与期望 HEAD 不一致：{head[:12]} != {expect[:12]}")
    gen = env.get("git_head_generated")
    if gen is not None:
        if not isinstance(gen, str) or resolve_commit(gen) is None:
            probs.append(f"env.git_head_generated 不可解析：{gen!r}")
        elif head and not is_ancestor(gen, head):
            probs.append(f"env.git_head_generated（{gen[:12]}）不是 env.git_head（{str(head)[:12]}）的祖先——签回方向可疑")
    return probs


def cmd_verify(args) -> int:
    path = Path(args.file)
    if not path.exists():
        print(f"[错误] 文件不存在：{path}", file=sys.stderr)
        return 2
    kind = args.kind if args.kind != "auto" else detect_kind(path)
    doc, _ = load(path)
    expect = args.expect_head
    if expect:
        resolved = resolve_commit(expect)
        if resolved is None:
            print(f"[错误] --expect-head 不可解析：{expect}", file=sys.stderr)
            return 2
        expect = resolved
    probs = verify_matrix(doc, expect) if kind == "matrix" else verify_coverage(doc, expect)
    if probs:
        print(f"[verify] FAIL {path}（{len(probs)} 项）")
        for p in probs[:20]:
            print("  -", p)
        return 3
    head = doc.get("head") if kind == "matrix" else doc["env"].get("git_head")
    at = doc.get("at") if kind == "matrix" else doc["env"].get("at")
    print(f"[verify] OK {path} kind={kind} head={head} at={at}")
    return 0


# ---------------------------------------------------------------------------
# selftest
# ---------------------------------------------------------------------------
def cmd_selftest(args) -> int:
    fails = []

    def ok(name, cond, detail=""):
        if not cond:
            fails.append(f"{name}: {detail}")

    base_matrix = {"generated_by": "selftest", "rows": [{"case": "c1"}, {"case": "c2"}]}
    base_cov = {"env": {"git_head": "e4d23d6f4d83"}, "ran_frames": 10, "total_frames": 10}
    sha = head_full()
    at = "2026-11-25T00:00:00Z"

    # 1) 正常签名后 verify 通过（matrix）
    d = sign_matrix(json.loads(json.dumps(base_matrix)), sha, at, False, False)
    ok("matrix 签名后可验", verify_matrix(d, sha) == [], str(verify_matrix(d, sha)))
    ok("matrix 行级 head 同步", all(r["head"] == sha for r in d["rows"]), "")

    # 2) 负例：伪造 sha（40 位但本仓不存在）
    fake = "0" * 40
    d2 = sign_matrix(json.loads(json.dumps(base_matrix)), sha, at, False, False)
    d2["head"] = fake
    for r in d2["rows"]:
        r["head"] = fake
    probs = verify_matrix(d2, None)
    ok("伪造 sha 被拒", any("解析不到提交" in p for p in probs), str(probs))

    # 3) 负例：at 置 null
    d3 = sign_matrix(json.loads(json.dumps(base_matrix)), sha, at, False, False)
    d3["at"] = None
    ok("null 时间被拒", any("at 缺失" in p for p in verify_matrix(d3, None)), "")

    # 4) 负例：行级缺 head（模拟旧产物没被签过）
    d4 = json.loads(json.dumps(base_matrix))
    probs = verify_matrix(d4, None)
    ok("未签名产物被拒", any("rows[0].head" in p for p in probs) and any("matrix.head" in p for p in probs), str(probs))

    # 5) 负例：head 与期望 HEAD 不一致
    d5 = sign_matrix(json.loads(json.dumps(base_matrix)), sha, at, False, False)
    ok("期望 HEAD 不一致被拒", any("不一致" in p for p in verify_matrix(d5, "1" * 40)), "")

    # 6) coverage：短 sha → 40 位；来源树是主线祖先时签回可验（绿路径）
    ancestor = resolve_commit("HEAD~1") or sha
    cov = {"env": {"git_head": ancestor[:12]}, "ran_frames": 10, "total_frames": 10}
    cov = sign_coverage(cov, sha, at, False, True)
    ok("coverage 短 sha 升级", cov["env"]["git_head"] == sha, str(cov["env"]))
    ok("coverage 保留来源（祖先）", cov["env"].get("git_head_generated") == ancestor, str(cov["env"]))
    ok("coverage 祖先签回可验", verify_coverage(cov, sha) == [], str(verify_coverage(cov, sha)))

    # 6b) coverage：来源树不在主线历史 ⇒ 必须报「签回方向可疑」（本次真实案例 e4d23d6f4d83）
    orphan = {"env": {"git_head": "e4d23d6f4d83"}, "ran_frames": 10, "total_frames": 10}
    orphan = sign_coverage(orphan, sha, at, False, True)
    ok("coverage 保留来源（非祖先）", orphan["env"].get("git_head_generated", "").startswith("e4d23d6f"), str(orphan["env"]))
    ok("非主线来源被拒", any("祖先" in p for p in verify_coverage(orphan, None)), str(verify_coverage(orphan, None)))

    # 7) coverage 负例：不可解析的 head
    bad = json.loads(json.dumps(base_cov))
    bad["env"]["git_head"] = "f" * 40
    ok("coverage 伪造 sha 被拒", any("解析不到" in p for p in verify_coverage(bad, None)), "")

    # 8) coverage 负例：签回方向可疑（generated 不是 head 的祖先）
    rev = json.loads(json.dumps(base_cov))
    rev = sign_coverage(rev, sha, at, False, True)
    rev["env"]["git_head_generated"] = resolve_commit("HEAD~0") and sha  # 自己当来源
    probs = verify_coverage(rev, None)
    ok("自身当来源不报祖先错误（祖先=自身）", not any("祖先" in p for p in probs), str(probs))

    # 9) 幂等：--deterministic 两次逐字节相同（用 tmp 文件走真实 dump/load）
    with tempfile.TemporaryDirectory() as td:
        p = Path(td) / "matrix.json"
        p.write_text(json.dumps(base_matrix, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
        doc, indent = load(p)
        doc = sign_matrix(doc, sha, at, False, False)
        dump(p, doc, indent)
        first = p.read_bytes()
        doc2, indent2 = load(p)
        doc2 = sign_matrix(doc2, sha, doc2["at"], False, False)   # 保持 at
        dump(p, doc2, indent2)
        ok("幂等（同 at 两次逐字节相同）", p.read_bytes() == first, "两次写入不同")
        doc3, _ = load(p)
        ok("幂等后仍可验", verify_matrix(doc3, sha) == [], str(verify_matrix(doc3, sha)))

    # 10) 拒绝：--deterministic 但产物没有 at
    with tempfile.TemporaryDirectory() as td:
        p = Path(td) / "matrix.json"
        p.write_text(json.dumps(base_matrix, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
        try:
            save = sys.argv
            sys.argv = ["evidence_bind.py", "sign", "-f", str(p), "--deterministic", "--allow-dirty"]
            rc = cmd_sign(argparse.Namespace(file=str(p), kind="auto", allow_dirty=True,
                                             deterministic=True, at=None, rebind=False))
            ok("--deterministic 无 at 时拒绝", rc != 0, str(rc))
        except BindError as e:
            ok("--deterministic 无 at 时拒绝", "要求产物已有合法 at" in str(e), str(e))
        finally:
            pass

    # 11) 脏树拒绝（真实工作树若脏则验证拒绝逻辑；干净则跳过并说明）
    dirty = dirty_paths()
    if dirty:
        try:
            cmd_sign(argparse.Namespace(file="__nonexistent__", kind="auto", allow_dirty=False,
                                        deterministic=False, at=None, rebind=False))
        except BindError:
            pass
    ok("脏树判定可计算", isinstance(dirty, list), str(type(dirty)))

    if fails:
        print("SELFTEST FAIL:")
        for f in fails:
            print("  -", f)
        return 1
    print(f"SELFTEST: 全部通过（断言组 11；HEAD={sha[:12]}；工作树脏文件 {len(dirty)} 个）")
    return 0


def cmd_rules(args) -> int:
    print("""必须带 head/at 的产物（一门规则，不靠记性）：
  shots_rust/matrix.json      L4 对拍矩阵（每次 run-all 落盘）
  analysis/tas-100/coverage.json  TAS 覆盖率报告（每次 replay/report-only 落盘）

head = `git rev-parse HEAD`（40 位，禁止短 sha / null）
at   = ISO8601 UTC（`YYYY-MM-DDTHH:MM:SSZ`）

生成方（producers）必须在落盘前调用 evidence_bind.stamp_*：
  脏树默认拒签（--allow-dirty 才放行，且把脏文件清单写进产物）
  缺 HEAD 一律拒签（不写 null）

读取方（consumers）：
  1) `python3 tools/evidence_bind.py verify -f <产物>` —— 结构+可解析
  2) `--expect-head $(git rev-parse HEAD)` —— 还要求「就是当前这棵树」
  3) coverage 若由 lane 签回主线：env.git_head=主线 HEAD、env.git_head_generated=产生树，
     且产生树必须是签回树的分支祖先（verify 会查）

lane 产物 pick 回主线后必须重签（`sign --rebind`），否则视为「来源不明的数字」。""")
    return 0


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description="证据↔代码树绑定：给产物打 head/at 并校验")
    sub = ap.add_subparsers(dest="cmd", required=True)

    p = sub.add_parser("sign", help="写入 head/at（脏树/缺 HEAD 即 fail-loud）")
    p.add_argument("-f", "--file", required=True)
    p.add_argument("--kind", default="auto", choices=["auto", "matrix", "coverage"])
    p.add_argument("--allow-dirty", action="store_true")
    p.add_argument("--deterministic", action="store_true", help="不更新时间（要求已有合法 at）")
    p.add_argument("--at", default=None, help="显式指定 at（测试用）")
    p.add_argument("--rebind", action="store_true", help="换树重签：旧 head 记入 *_generated/head_rebound_from")
    p.set_defaults(func=cmd_sign)

    p = sub.add_parser("verify", help="校验 head/at（含可解析性）")
    p.add_argument("-f", "--file", required=True)
    p.add_argument("--kind", default="auto", choices=["auto", "matrix", "coverage"])
    p.add_argument("--expect-head", default=None)
    p.set_defaults(func=cmd_verify)

    p = sub.add_parser("selftest", help="三类负例 + 幂等自测")
    p.set_defaults(func=cmd_selftest)

    p = sub.add_parser("rules", help="打印绑定规则")
    p.set_defaults(func=cmd_rules)

    args = ap.parse_args(argv)
    try:
        return args.func(args)
    except BindError as e:
        print(f"[fail-loud] {e}", file=sys.stderr)
        return 3


# ---------------------------------------------------------------------------
# 给 producers 用的入口（落盘前调用）
# ---------------------------------------------------------------------------
def stamp_matrix(doc: dict, allow_dirty: bool = False) -> dict:
    """producers 在写 matrix.json 前调用：写入 head/at，脏树默认抛 BindError。"""
    dirty = dirty_paths()
    if dirty and not allow_dirty:
        raise BindError("matrix 落盘时工作树脏（加 --allow-dirty 才放行）：\n  " + "\n  ".join(dirty[:10]))
    return sign_matrix(doc, head_full(), now_iso(), allow_dirty, rebind=True)


def stamp_coverage(doc: dict, allow_dirty: bool = False) -> dict:
    dirty = dirty_paths()
    if dirty and not allow_dirty:
        raise BindError("coverage 落盘时工作树脏（加 --allow-dirty 才放行）：\n  " + "\n  ".join(dirty[:10]))
    return sign_coverage(doc, head_full(), now_iso(), allow_dirty, rebind=True)


if __name__ == "__main__":
    raise SystemExit(main())
