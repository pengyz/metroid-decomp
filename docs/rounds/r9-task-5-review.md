# r9 Task 5 复核（机制级最小重现 runner）

- 被复核对象：`64f053c1 feat(tools): 增加机制级最小重现 runner`（`tools/movie_ref/mechanism_scene.py` 374 行、
  `tools/tests/test_mechanism_scene.py` 68 行/7 测试、计划 `docs/superpowers/plans/2026-11-18-three-leg-oracle-alignment.md` Task 5 文案 +4/−1）
- 复核员立场：证伪（非实施方）；未修改 `tools/`、`data/`、`crates/`、`gates/`
- 约束遵守：只跑单次 ≤600 帧实跑；未跑 `cargo test`、未跑 `gates/cli.py check --full`、未跑 L4 矩阵
- 结论：**pass-with-findings**（工具本身诚实，但计划 Task 5 引用的「all-match」证据是常量窗口，无判别力）

## 1. 两腿真跑 / 非同一文件 / 非陈旧产物 —— PASS

```
$ ls -la --time-style=+%H:%M:%S analysis/traces/ | grep password-justin-600
-rw 17:36:09 mechanism-case-password-justin-600-87900857.frames.jsonl   2680991 B
-rw 17:36:21 mechanism-case-password-justin-600-87900857.run.json
$ python3 -c "...run.json..."
scenario: /tmp/mechanism-scene-9xqci2t6/mechanism-case-password-justin-600-87900857.txt
scenario_sha256: 0876563492a0c71d58abe4f3dca636ef382bdd984526aa88eebc847bc7a115c3
frames: 600   trace rows: 600   w keys 含 Joy1Latch / 0000-00ff / 0100-07ff
```

- 每次 run 的 trace 名带 uuid（`mechanism-<stem>-<frames>-<uuid>`），run.json 记录 `/tmp/mechanism-scene-*/…txt`
  临时场景路径与 sha256，mtime 为本次 ⇒ 不是陈旧产物。
- `tools/scenario_run.py:999-1000` 在跑之前 `done_path.unlink()`，无 trace 复用；
  `emu_cache_hit` 只出现在 `tools/l4_diff_run.py:287`（调用点 `:1121`），与 `scenario_run` 无关。
- 两腿是两份数据：mesen 600 行 / rust 590 行，缺帧集合不同（rust 缺 `[2,4,5,6,301,302,303,502,503,504]`），
  latch 取值不同（mesen 仅 f310/f350=16，rust f300=16/f600=1）⇒ 未读同一文件。
- 临时 case 不残留：`ls cases/ | grep -c mechanism` → `0`。

Mesen 腿可信度另证：

```
$ python3  # 旧 L4 trace(2139 行) vs 本次 600 帧 trace(585 行)，取 585 个共同帧
common rows 585；FC/MainRoutine/SamusStatus/RoomNumber 逐帧全等
first diff at 1 {'Joy1Latch': (None, 0)}   # 仅新增列
old status Counter({1:847,0:664,20:136,...,24:40})；new Counter({0:345,20:136,21:100,22:4})
```

## 2. all-match 是否空对空 —— **FAIL（空对空）**

`python3 tools/movie_ref/mechanism_scene.py --frames 600 --case cases/case-password-justin.txt --json-out /tmp/t5-pw600.json`

```
OFFSET: +0（anchor=latch(-1+gap)）
COMPARE: compared=1180 mismatch=0 incomparable=10 first_diff=None
RESULT: all-match      EXIT=0
```

但被比较的两个字段在整窗都是常量：

```
mesen status {0:600}  room {0:600}
rust  status {0:590}  room {0:590}
⇒ compared=1180 条比的是「0 == 0」
```

且 **offset 不是状态面解出来的**，是 latch 回退：状态面 0 次跳变 → `_choose_anchor` 落到 latch
（`anchor=latch(-1+gap)`）。`incomparable=10` 恰为 rust 缺的 10 帧。

实施方在计划里引用的另一条证据（临时合成 120 帧 case，`compared=212`）我**无法复跑**（case 已被删除），
但它的残留 trace 不是常量：`analysis/traces/mechanism-scene-synthetic-120-120-e36087ef.frames.jsonl`
status `{20:71, 0:49}` changes `[50]`、room `{255:73,0:45,9:2}` changes `[46,48]`、latch changes
`[11,13,41,43,61,63,81,83]`。`compared=212 = (120+106−14)`，其 offset 同样是 latch 回退
（状态面只有 1+2 次跳变，`align` 会判 insufficient）。

**判断**：`compare()` 的 `all_match` 只回答「没有失配」，不回答「这次比较有没有信息量」。
常量窗口下它照样打印 `RESULT: all-match` 并 exit 0 —— 这是工具级的过度自信输出，
也正是「机制级最小重现」最容易被误读成「该机制已复现」的地方。

补充：唯一有真实状态跳变的现有 case（morphball）不能产出 all-match，反而证明工具会 fail-loud：

```
$ python3 tools/movie_ref/mechanism_scene.py --frames 600 --case cases/case-morphball-probe.txt
ERROR: 状态面字段跳变不足，latch 也无法对齐（room: insufficient anchor changes
       (mesen=585 rust=586 changes=0/0)；latch: insufficient anchor changes）
EXIT=2
```

原因：mesen 该窗 status `0→20(f360)→21(f~497)→22(f~588)`，但 **rust 腿 status 全程 0**（真实分歧），
`align` 无匹配；room 0 跳变；latch 只有 f310/311、f350/351 两段 <3 ⇒ 三重失败，退出 2。这是正确行为。

## 3. `_choose_anchor` / `state_offset_from_latch` 方向 —— PASS（合成数据自证）+ 2 条约定类 finding

合成 240 帧（status/latch 各为 25% 变化的随机序列），令 `rust[f+k] == mesen[f]`、`latch` 相位 `k−1`（Task 1：latch 比 state 晚一行）：

```
rust 相位 +0: offset=+0 via status; aligned mismatch=0 compared=240
rust 相位 +1: offset=+1 via status; aligned mismatch=0 compared=240
rust 相位 +3: offset=+3 via status; aligned mismatch=0 compared=240
rust 相位 -2: offset=-2 via status; aligned mismatch=0 compared=238
常量状态 + latch 相位 +2 -> offset=+3 via latch(+2+gap)（期望 +3）
```

⇒ `main` 的 `rust_aligned = {g - offset}` 与 `_fit` 的 `seq_a[f]==seq_b[f+offset]` 自洽，
latch 回退 `latch_offset + gap` 的符号也正确（gap=+1 是「两个 doc 值之差」，对全局符号翻转不变）。

**finding 3a（约定易混）**：同一个仓库里有两套偏移约定——
`data/status/leg_offsets.json` 是 `leg[f] == mesen[f+k]`（biz +1），而 `align(a,b)` 返回
`a[f]==b[f+o]`（`align(mesen, biz) = −1`）。`state_offset_from_latch` 吃的是**后者**，
`_load_latch_gap` 读的是**前者**。实测误用后果：

```
state_offset_from_latch(doc["latch_offsets"]["rust_to_mesen"], doc) -> 2   # 真值 0
state_offset_from_latch(-1, doc)                                    -> 0   # 正确
```

docstring 已写「align(mesen, rust) 得到 latch_offset 后」，所以不算错；但函数名与自变量的符号
没有任何机制强制，建议改名 `latch_offset_align` 或加断言 `state == -doc["offsets"]["rust_to_mesen"]`。

**finding 3b（重复公式）**：`_choose_anchor` 内联了 `latch_offset + _load_latch_gap()`，
没有调用 `state_offset_from_latch`；两处将来分开改会分叉。

## 4. 600 上限与失败路径 fail-loud —— PASS

```
$ python3 tools/movie_ref/mechanism_scene.py --frames 601 --case cases/case-password-justin.txt
ERROR: frames=601 超过硬上限 600：机制级最小重现不允许退化成跑长片
EXIT=2    （0.088 s，未起任何仿真）
```

- `_run()` rc≠0 抛 RuntimeError；缺 trace / 缺 `--state-out` 抛 FileNotFoundError；`main` 捕获后 exit 2。
- 上面 morphball 600 的例子证明「对齐失败」不会静默全绿。
- 唯一缺口：`run_scene()` 成功但 `compared=0` 时会 `all_match=False` → exit 2，语义正确。

## 5. 7 条测试是否只测纯函数 —— pass-with-finding

```
$ python3 -m pytest tools/tests/test_mechanism_scene.py -q
7 passed in 0.09s
$ grep -nE "skip|xfail|pragma|environ|monkeypatch|DSH_" tools/tests/test_mechanism_scene.py
（无命中）
```

7 条覆盖：`compare` 的 first_diff / 只比指定字段 / 缺行不可比 / 缺字段不可比；`run_scene(...,601)` 抛
ValueError；parser help 含「600」「机制级」；`state_offset_from_latch(-1, doc)==0`。

**缺口**：没有一条覆盖真正有风险的路径——① `_choose_anchor` 的 latch 回退分支（本次实跑唯一走过的分支）；
② `_shorten_case`（`watch 0014` → `Joy1Latch`、capture/press 裁剪、hold 裁剪）；③ `run_scene` 的失败路径
（缺二进制 / 缺 trace / rc≠0）。这三处都是纯函数之外的胶水，恰是工具最容易坏的部分。

## 6. 「L4 case-morphball-probe 420 帧只有 2 次锚跳变」—— 实质成立、措辞不精确

```
$ python3  # analysis/traces/case-morphball-probe.frames.jsonl，取 f<=420
latch 列：全 None（该 trace 根本没有 latch 列）
status changes: [360]（1 次）；room changes: []
case 输入 f<=420: press 310 start、press 350 start（2 次按键）
```

⇒「2 次」指 2 次**按键**，不是 `$0014` 锚跳变；且该 trace 无 latch 列，这个数字不可能由它测出。
但「≤420 帧无法用 Task 1 的 align 对齐」的**实质成立**（状态面 1 次变化、latch 无数据）。

## 7. 计划文案只改错、未弱化判据 —— PASS（但证据链断）

`git show 64f053c1 -- ...three-leg-oracle-alignment.md` 仅两处：Step 2 Expected 由
`ImportError: cannot import name Ledger`（明显复制错）改为 `ModuleNotFoundError: No module named
'tools.movie_ref.mechanism_scene'`；Step 5 加注「不存在 case-morphball.txt/case-roll.txt、现有 case 均 >120 帧、
≤120 帧内没有 ≥3 次状态面跳变，实际用临时 120 帧 case 与 case-password-justin 600 帧」。
「与 L4 结论不得矛盾／矛盾如实记为缺陷、不得改期望值」原文未动 ⇒ 判据未弱化。

**evidence-chain 缺口**：计划现在引用一个已删除、无 sha、不可复跑的临时 case。要么把它落成
`cases/case-*.txt`（十几行），要么把其全文与 sha256 写进计划/复核档——否则「可复跑」的承诺是空的。

## 8. 残余修复清单（可直接执行）

1. `tools/movie_ref/mechanism_scene.py:67-107` `compare()`：增加 `informative` 字段 ——
   对被比较帧逐字段统计 distinct 值，任一字段 ≥2 即 `informative=True`；`all_match` 之外
   输出 `informative`。`main`（`:346-360`）在 `informative=False` 时打印
   `RESULT: uninformative（两侧常量一致，无判别力）` 并 exit 3（或要求显式 `--allow-constant`）。
2. `:238-266` `_choose_anchor`：改为 `return state_offset_from_latch(latch_offset, _load_offsets()), f"latch(...)"`，
   消除重复公式；`:226` 函数改名 `state_offset_from_latch(latch_offset_align: int, ...)` 并加断言
   `state == -int(doc["offsets"]["rust_to_mesen"])`（rust），防两套约定混用。
3. `tools/tests/test_mechanism_scene.py`：补三条 —— ①`_choose_anchor` 常量状态 + 合成 latch（断言 `+k`）；
   ②`_shorten_case`（`watch 0014` → `Joy1Latch`、frames 裁剪、capture 裁剪）；③`run_scene` 缺二进制/缺 trace 抛错。
4. 计划 Task 5 Step 5 注解：把「case-password-justin 600 帧」标注为**常量窗（status/room 全 0，无判别力）**，
   并落一个可复跑的小 case（`cases/case-mechanism-120.txt`，含其 sha256）；否则删除该 bullet 的「验证」语气。
5. `tools/movie_ref/mechanism_scene.py:331` 起的 `--fields` help 写「默认 status,room,latch」，
   实际 `DEFAULT_FIELDS=("status","room")` —— 改 help 或改注释，二者取一。
6. （可选）跑完不清理 `analysis/traces/mechanism-*`：本次复核后该目录有 354 个 mechanism 文件、8.5 GB。
   加 `--keep-trace`，默认删除本次 run 的 6 个副产品（保留 `--json-out`）。

## 9. 复核用命令清单

```
git show --stat 64f053c1
git show 64f053c1 -- docs/superpowers/plans/2026-11-18-three-leg-oracle-alignment.md
python3 tools/movie_ref/mechanism_scene.py --frames 600 --case cases/case-password-justin.txt --json-out /tmp/t5-pw600.json
python3 tools/movie_ref/mechanism_scene.py --frames 600 --case cases/case-morphball-probe.txt   # EXIT=2
python3 tools/movie_ref/mechanism_scene.py --frames 601 --case cases/case-password-justin.txt   # EXIT=2
python3 -m pytest tools/tests/test_mechanism_scene.py -q
python3 /tmp/t5_synth.py                  # 合成方向实验（§3）
python3 - <<'EOF' ... EOF                 # trace/栈、L4 trace 对比、password JSON 常量统计（§1/§2/§6）
```

---

# 第二轮终审：`ef5071e5` + `3cffe8e5`

总 verdict：**pass-with-findings**（防「空对空」的承重墙已立并在默认路径生效；4 条低风险残余见 §5）。

## 1. informative 真的挡住空对空 —— PASS（默认路径）

```
$ python3 tools/movie_ref/mechanism_scene.py --frames 600 --case cases/case-password-justin.txt --json-out /tmp/t5r2-pw600.json
OFFSET: +0（anchor=latch(-1+gap); mesen[f] == rust[f+0]）
COMPARE: compared=1180 mismatch=0 incomparable=10 first_diff=None
FIELDS: status, room
RESULT: uninformative（常量窗，无判别力）
EXIT=3        # stderr: 提示：常量窗没有对照信息量；确需接受请加 --allow-constant

$ … --allow-constant --json-out /tmp/t5r2-allow.json
RESULT: uninformative（常量窗，无判别力；--allow-constant 显式接受）
EXIT=0        # JSON: informative=False, distinct_values={'room':1,'status':1}, all_match=True
```

只有 latch 变、status/room 恒定（合成 120 帧，latch 每 10 帧翻）：

```
compare(mes, rust, ("status","room"))  => informative=False, distinct={'status':1,'room':1}, all_match=True
```

⇒ latch 未参与 informative 判定，符合「latch 是对齐锚、不是被判定字段」。真状态变化时：

```
compare(mes( status 每 10 帧翻), 同, ("status","room")) => informative=True, distinct={'status':2,'room':1}
```

**残余漏洞（低）**：显式 `--fields status,room,latch` 时 latch 会进 distinct ⇒ 状态全常量但 latch 有变化时
`informative=True`、`all_match=True`，空对空被重新放行：

```
compare(mes==rust, 状态常量, ("status","room","latch")) => informative=True, all_match=True, distinct={'status':1,'room':1,'latch':2}
```

## 2. `state_offset_from_latch` 语义 —— PASS（rust）/ 残余（biz 不支持）

```
doc: offsets {'biz_to_mesen': 1, 'rust_to_mesen': 0}  latch {'biz_to_mesen': 2, 'rust_to_mesen': 1}
rust: state_offset_from_latch(-1)   = 0      # 期望 0 ✓（align 口径 = -doc 值）
rust: state_offset_from_latch(+1)   => ValueError「口径不一致」✓（传错 doc 口径）
biz : state_offset_from_latch(-2)   => ValueError（断言写死 -offsets.rust_to_mesen）
```

`_choose_anchor` 已无内联 `+ _load_latch_gap()`，改为调用 `state_offset_from_latch`（inspect 源码确认）。
biz 腿不被该函数支持：工具本身只跑 mesen↔rust，无运行时影响；但函数签名看起来通吃两腿。

## 3. 新测试覆盖失败路径 —— PASS（抽查的两条都是真断言）

```
$ python3 -m pytest tools/tests/test_mechanism_scene.py -q
17 passed in 0.05s
```

- `test_run_command_failure_raises`（:140）：真起 `python -c exit(7)`，断言 `RuntimeError` 且文案含 `失败 rc=7`
  —— 是「必须抛且信息正确」，不是「不崩」。
- `test_run_scene_missing_rust_binary_raises`（:172）：造出 Mesen trace、mock `_run` 成功、把 `ROOT` 指到 tmp，
  断言 `FileNotFoundError` 且文案含「缺 Rust 腿二进制」—— 证明失败发生在 Mesen 腿之后、返回空 dict 之前。
- 另：`test_choose_anchor_uses_latch_fallback_and_measured_gap` 真断言 `offset == 0` 且 `anchor.startswith("latch(")`；
  `test_shorten_case_relabels_latch_and_clips` 真断言 `watch 0014 Joy1Latch`、`press 700`/`capture 800` 被裁、
  `watch_range` 被补。
- 无 skip/xfail/pragma（`monkeypatch` 命中均为 fixture，不是跳过）。

## 4. 计划文本 —— PASS（1 nit）

`git diff 64f053c1..HEAD -- …/three-leg-oracle-alignment.md`：Step 5 现写明
「`case-password-justin.txt` 600 帧是常量窗…**无判别力**；不作为 all-match 对照证据」「可复跑且 status ≥3 次跳变的
≤120 帧 case **未落地**（input-only 120 帧只有 0→20 一次；morphball 20→21 在 +146、21→22 在 +246）」「没有用 fixture
伪造状态面凑数」「本 task 只声称 informative 判据与轻量 all-match 在有判别力的合成窗口上成立」。
⇒ 不再把 all-match 当验证证据，未落地原因与阈值写清。

nit：Step 5 的 Run 行仍写 `--case cases/case-morphball.txt`（该文件不存在），只靠下面注记解释；
建议把 Run/Expected 换成实际执行过的命令或注明「示例，文件不存在」。

## 5. 清理 —— **部分 FAIL**（`_cleanup_trace` 漏 hex 快照）

```
运行前： ls analysis/traces | grep -c "^mechanism-"   -> 0（实施方已清 468→0 ✓）
跑 600 帧一次后 -> 44
$ ls analysis/traces/ | grep mechanism
mechanism-case-password-justin-600-34445311_f00030_game.ppu.hex
mechanism-case-password-justin-600-34445311_f00030_game.ram.hex
…（22 个采集点 × ppu/ram = 44 个，约 640 KB/run）
```

原因：`_cleanup_trace` 只 glob `{name}.*`；快照文件是 `{name}_f00030_game.ppu.hex`（下划线，无点）。
`--keep-trace` 生效（120 帧跑 + `--keep-trace` ⇒ 文件保留，含 `.frames.jsonl`）。
修法：`for pattern in (f"{name}.*", f"{name}_f*"):`。

（本次复核产生的 mechanism-* 已由我清理回 0，未动其它文件。）

## 6. 新问题 —— 无阻塞

- **exit 3 不会被误判**：全仓 grep，除 `tools/tests/` 外没有任何 runner/门禁消费 `mechanism_scene` 的退出码
  （未注册进 `gates/cli.py`）⇒ 3 只是 CLI 约定；但 **计划/README 未写退出码语义**（0 all-match / 1 DIFF /
  2 错误 / 3 常量窗），建议补一行，避免外部脚本把 3 当「其它错误」。
- **返回形状兼容**：`run_scene` 仍返回 `mesen`/`rust`/`offset`/`anchor_field`，只新增 `trace_name`；
  调用点只有 `main` 与测试，无破坏。
- 复核脚本与实跑均未改动 `tools/`、`data/`、`crates/`、`gates/`；`git status` 仅本档 `??`。

## 给维护者的警告（承重墙）

**`compare()` 的 `informative` 判据是「防 all-match 空对空」的承重墙**：
它必须是「被比较字段中至少一个出现 ≥2 个不同取值」，且 `latch` 永远不参与（latch 只是对齐锚）。
一旦有人为了「让短 case 也能打出 all-match」而放宽它（把 latch 计入、或改成 `compared>0`），
密码窗那类「0==0」的常量比较就会重新被当成机制复现的证据 —— 这正是 Task 5 第一轮被打回的根因。
配套回归：`test_main_uninformative_exit_3_and_allow_constant` 与 `test_compare_informative_*` 不要删。

**第二轮终审：pass-with-findings，Task 5 可完成（新 case 未落地，属如实缺口；4 条低风险残余见 §5/§6）**（2026-11-18，被复核 commit `ef5071e5`+`3cffe8e5`）
