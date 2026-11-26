# TAS `--resume` 段结论与 rust 腿锚（#115）

> 状态：修复已落地（`tools/tas_replay_diff.py`），两个负例 fail-loud、正例不误报。
> 证据级别：以下每条读数均为本机实跑；命令与输出原样抄录。

## 1. 症状：改了 rust 腿，`--resume` 仍报旧结论（假绿）

`tas_replay_diff.py replay --segments N --resume` 的旧实现只按 `coverage.json` 里段条目的
`index` 复用：

```python
prior = {e["index"]: e for e in old.get("segments", [])
         if e.get("status") in ("pass", "diverge")}
```

rust 腿产物 `rust/full.state.jsonl` 也只要「存在且非空」就跳过重跑。两者都不与「这份结论是
在哪份 rust 腿上跑的」绑定 ⇒ 在同一 `--out` 目录上改了 rust 代码（或帧表被替换/截断/追加）后
再 `--resume`，旧的「pass / 无分歧」会原样留在报告里。

## 2. 复现（A 级）

基线（200 帧、段长 100、干净树）：

```
python3 tools/tas_replay_diff.py replay --inputlog movies/lordtom-metroid-100p.inputlog \
    --frames 200 --segments 100 --out /tmp/r115/base
→ seg 0 pass / seg 1 pass；段 2/2 已跑；无分歧   (rc=0)
```

把 rust 帧表第 6 行的一个 zp 字节改 1 位（模拟「rust 腿变了」，文件仍是合法 JSON）：

```
python3 - <<'EOF'
import json,pathlib
p=pathlib.Path('/tmp/r115/neg1/rust/full.state.jsonl')
lines=p.read_text().splitlines()
d=json.loads(lines[5]); zp=d['w']['zp']
d['w']['zp']=zp[:-2]+('FF' if zp[-2:]!='FF' else '00')
lines[5]=json.dumps(d,separators=(',',':'))
p.write_text('\n'.join(lines)+'\n')
EOF
```

**修复前**（`git show HEAD:tools/tas_replay_diff.py` 存到 `/tmp/r115/old_tool.py` 运行）：

```
python3 /tmp/r115/old_tool.py replay --inputlog "$PWD/movies/lordtom-metroid-100p.inputlog" \
    --frames 200 --segments 100 --out /tmp/r115/neg1-old --resume
→ seg 0 已完成（pass，--resume 跳过）
→ seg 1 已完成（pass，--resume 跳过）
→ 段 2/2 已跑；无分歧                                   (rc=0)
```

即：rust 帧表已被改动，报告仍宣称「无分歧」——正是卡面说的假绿。

## 3. 修法：段结论绑定 rust 腿锚

新增三件：

1. `_current_rust_anchor(out, frames)`：锚 = `git_head`（40 位）+ `rust_binary_sha256`
   （`target/debug/rust_screenshot` 的 sha 前 16 位）+ `rust_trace_sha256`（`rust/full.state.jsonl`
   整文件 sha256）+ `frames`，再取 `anchor_sha`（内容指纹，16 位）。
2. `coverage.json` 写入 `rust_anchor`（报告级），每条 pass/diverge 段写 `rust_anchor_sha`。
3. `--resume` 读旧报告后先跑 `_check_resume_anchor(old, current)`：
   - 缺报告级锚 ⇒ 报「coverage.json 缺 rust 腿锚（rust_anchor）…拒绝 --resume」；
   - 任一段缺段级锚 ⇒ 报「段 k 缺 rust 腿锚（rust_anchor_sha）」；
   - HEAD / 二进制 sha / 帧表 sha / 帧数任一不一致 ⇒ 报「段结论失效、需重跑：rust 腿锚变化
     （…）。旧段是在另一份 rust 腿上跑的，沿用会得出「改了代码仍报旧首分歧」的假绿」。
   全部通过才允许跳过已完成段。

`--report-only` 会保留原报告的 `rust_anchor` 一起重写，不会把锚丢掉。

## 4. 修复后读数（A 级，同一基线目录）

| 场景 | 命令 | 结果 |
|---|---|---|
| 正例：未改动 | `--resume`（同一 `/tmp/r115/base`） | rc=0；`seg 0 已完成（pass，--resume 跳过）`、`seg 1 已完成（pass，--resume 跳过）` |
| 负例一：rust 帧表被改 1 字节 | 同上，`--out /tmp/r115/neg1` | **rc=2**；`FAIL: 段结论失效、需重跑：rust 腿锚变化（rust 帧表 sha 385e4dd9… → aab6d8b8…）` |
| 负例二：手删报告级锚 | 同上，`--out /tmp/r115/neg2` | **rc=2**；`FAIL: coverage.json 缺 rust 腿锚（rust_anchor）：无法判断旧段结论是否仍有效，拒绝 --resume` |
| 负例三：手删段级锚 | 同上，`--out /tmp/r115/neg3` | **rc=2**；`FAIL: coverage.json 段 1 [100,200) 缺 rust 腿锚（rust_anchor_sha）：拒绝 --resume，需重跑该段` |

新基线报告里的锚实测：

```json
{"anchor_sha":"9a911411a50e32b9","frames":200,
 "git_head":"811bbbca9f297fed192df3283d0457ea44915d82",
 "rust_binary_sha256":"d3911e809787a42c",
 "rust_trace_sha256":"385e4dd91478acc21dbf73b238059c68fbf840e29271e4548c3638d32e58db15"}
```

单测：`python3 -m pytest tools/tests/test_tas_resume_anchor.py -q` → **6 passed**（正例、缺报告锚、
缺段锚、HEAD/二进制/帧表三变体）。

## 5. 影响面与未覆盖

- **旧 `coverage.json`（无 `rust_anchor`）一律拒绝 `--resume`**：这是有意的——无法证明旧结论
  与当前树同口径。现有长片报告（如 `analysis/tas-100/coverage.json`）需去掉 `--resume` 重跑一次，
  之后才带锚。这是行为变化，已在提交说明里写明。
- 锚只覆盖「rust 腿」这一侧；emu 腿仍由 Mesen 版本与 `analysis/` 缓存决定（`l4_diff_run` 侧已有
  `mesen_sha256` 校验，本卡不改）。
- `--report-only` 仅重写报告，不校验锚与当前树是否一致；它的语义是「按已有 entries 重排输出」，
  不是「复用结论判定」。若要用它出结论，仍是调用者责任——这一点写在 `--report-only` 的 help 里
  （本次未改 help，只在此文档说明）。

## 6. 复跑命令

```bash
python3 -m pytest tools/tests/test_tas_resume_anchor.py -q
python3 gates/cli.py check
python3 tools/l4_diff_run.py run-all
```
