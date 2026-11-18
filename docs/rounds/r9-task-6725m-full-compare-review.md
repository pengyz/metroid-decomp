# rounds/r9 — task-6725M 全片对拍独立复核（verifier 证伪立场）

## verdict: pass-with-findings

六项送审主张**实质全部复现**（RAM 面六数逐一对上、像素面 136 点逐值对上、Rust 腿重放逐字节可复现）；
不成立的是 §10.3 的 3 处**数字笔误**（P1）与 7 处措辞/可复现性瑕疵（P2），不动摇主结论。
按 AGENTS.md §5「verdict=pass 才算步骤完成」，本步骤需先修 P1-1/P1-2/P1-3 三行后再宣告完成。

> 【2026-11-18 结案】P1-1/P1-2/P1-3 三行已由 lead 按本报告的实测数字改正（见 findings §10.3 的
> 「复核更正」批注）；P2 七条不改正文，理由逐条记在本文 §P2。故本步骤状态：**pass**（P1 已清，P2 记账）。

**复核对象**（HEAD=2b0431e8，工作树干净）：

| commit | 内容 | 本报告涉及 |
|---|---|---|
| `be08a3c6` | fix(tools): `bizhawk_ref.lua` BH_FRAMES 半开区间 | §T0 |
| `0d37804f` | feat(tools): `tools/movie_ref/` 双腿对拍工具链 | §T2–§T5 |
| `7895fff6` | docs(findings): `movie-bizhawk-6725m.md` §10 | §T1–§T6 |
| `2b0431e8` | docs 后续修订（§10.3/§10.4 数字口径） | 引用，不单独裁 |

**权威输入（按任务约定，未使用并发重写的 `analysis/traces/bizhawk-6725M-full.jsonl`）**：

| 文件 | 行数 | sha256 |
|---|---|---|
| `/tmp/bh-full-28408.jsonl` | 28408 | `e8a82fb15f6d76e9827a049928bfc0ef320e9b82230712b5b032cdf29b7407a9` |
| `/tmp/rust-states-full.jsonl` | 28308 | `0633b62602dfc62eaa27fe4fd964da0e3801d82b204d4ef695b5503c4e92fb73` |

**定级口径**：P0=结论/裁定被推翻或核心数字不可复现；P1=数字/事实错误（可机械反证，不动摇主结论）；
P2=措辞/可复现性/证据缺口。

---

## 0. 执行命令清单（全部在本机实跑）

```bash
# T1 轨迹完整性
wc -l /tmp/bh-full-28408.jsonl; head -1 /tmp/bh-full-28408.jsonl; tail -1 /tmp/bh-full-28408.jsonl
sha256sum /tmp/bh-full-28408.jsonl /tmp/rust-states-full.jsonl
python3 -c '<f 单调连续性/列集/空行检查>'
stat -c '%w %y' /tmp/bh-full-28408.jsonl

# T2/T3/T4 RAM 面对拍（六数 + 分岔前 + 首个实质分歧）
python3 tools/movie_ref/compare_bizhawk_rust.py --bizhawk /tmp/bh-full-28408.jsonl \
  --rust /tmp/rust-states-full.jsonl --inputlog movies/eien86-6725M.inputlog \
  --json-out /tmp/verify-ram-compare-full.json
#   ↑ 同脚本复跑 4 个窗口：默认 / --start 20 --end 1131 / --start 30 --end 1030 /
#     --start 20 --end 1000 / --start 1132 --end 28407
python3 -c '<独立重写 inputlog 解析，核 p1 与 inputlog 逐帧对应>'

# T5 像素面（真复跑判定 + 平移探针）
python3 tools/movie_ref/compare_pixels.py --oracle-dir shots --rust-dir /tmp/rustshots-full \
  --name-re 'eien86-6725M_' --json-out /tmp/verify-pixels.json
python3 tools/movie_ref/pixel_shift_probe.py --oracle-dir shots --rust-dir /tmp/rustshots-full \
  --only-diff --max-shift 3
python3 -c '<自算 bbox / 颜色集 / 两侧交换对统计>'

# Rust 腿独立重放（确定性核验）
./target/debug/rust_screenshot --schedule /tmp/m6725-full.sched --out /tmp/verify-shots \
  --state-out /tmp/verify-rust-states.jsonl
cmp /tmp/verify-rust-states.jsonl /tmp/rust-states-full.jsonl
cmp -s <(每个 PNG) <(对应旧 PNG)   # 136/136 逐字节相同

# §T0 工具修正
git show be08a3c6 -- tools/movie_ref/bizhawk_ref.lua tools/run_bizhawk_trace.sh
cmp /tmp/bh-fix-28408.jsonl /tmp/bh-full-28408.jsonl
head -28401 /tmp/bh-full-28408.jsonl | cmp - /tmp/bh-partial-28401.jsonl
cmp /tmp/bh-old-1200.jsonl /tmp/bh-fix-1200.jsonl
```

未跑（任务明令/成本）：`cargo test --workspace`、`gates/cli.py l4`、Mesen2 采集腿（会覆写 `shots/`）。

---

## 1. 逐条结论

| # | 主张 | 结论 |
|---|---|---|
| T0 | BH_FRAMES 半开区间修正 + 产物不变 | **PASS**（一处证据缺口 → P2-4） |
| T1 | 28408 行 f=0..28407 连续、末行 health_lo=72 | **PASS** |
| T2 | 28295 / 1809 / 26486 / 222 / 223 / 0 六数 | **PASS**（六数全中，另核 4 字段口径 2280、逐字段 5 数全中） |
| T3 | 分岔前 f20..1131 = 1098/73/1025；f30..1030 除已知点全同 | **PASS（实质）**；计数措辞 P2-1 |
| T4 | f1030 p1=Up\|Right；退球晚约 50 帧 | **PASS**；「p1 恒为 01」P2-2 |
| T5 | 像素面=Mesen2 腿、BizHawk 不可零容差、空白名单 | **PASS**（判定与探针真复跑；Mesen2 采集腿未复跑） |
| T6 | §10 是否存在超出数据的措辞 | **FAIL（轻度）**：3 处 P1 数字错 + 7 处 P2 |

---

## 2. T1 轨迹完整性 — PASS

```
$ wc -l /tmp/bh-full-28408.jsonl
28408 /tmp/bh-full-28408.jsonl
$ head -1 → {"f":0,...,"health_lo":255,"health_hi":255,"status":255,"room":255,"scrolldir":255,"p1":0,...}
$ tail -1 → {"f":28407,"health_lo":72,"health_hi":0,"status":1,"room":255,"scrolldir":0,"p1":8,...}
```

- 逐行检查：前 28408 行 `f` 严格 `f[i]==i`，0..28407 无缺行；键集恒为 10 列（无中途增列/漂移）；无空行；JSON 全部可解析。
- sha256 见上表；`f=0` 全 255 与「RESET 前未初始化」一致，`f=28407` 六字段与 §10.1 line 表**逐值相同**。
- 附证 §10.1 耗时：文件 birth `10:15:21` → mtime `10:23:14` = **7m53s**，与「7 min 53 s」独立吻合（不是自证，是文件系统读数）。
- 附证 §10.0 挂死叙事：`/tmp/bh-partial-28401.jsonl` 28401 行且是完整件的**严格前缀**；28408−28401=7，与「末 7 行丢缓冲」一致。

## 3. T2 RAM 面六数 — PASS（全中）

```
$ python3 tools/movie_ref/compare_bizhawk_rust.py --bizhawk /tmp/bh-full-28408.jsonl \
    --rust /tmp/rust-states-full.jsonl --inputlog movies/eien86-6725M.inputlog \
    --json-out /tmp/verify-ram-compare-full.json
window   f=20..28407  (28388 fps)
comparable 28295   all_equal 1809 (four-field 2280)   mismatch 26486
runs     mismatch 222   equal 198
latch    mismatch 223 / 28295   first f=1133
inputlog↔biz p1 mismatches 0
per-field health_lo 23055  health_hi 14248  status 21071  room 1546  scrolldir 15554
first mismatch f=27  room rust=FF biz=09  (仅 room)
```

- 六个送审数字 **28295 / 1809 / 26486 / 222 / 223 / 0 全部命中**；另送的 5 个逐字段数与「四字段 2280」「首失配 f27 仅 room」「首 latch f1133」也全部命中。
- equal_runs 前 6 段 = `30..481 / 484..672 / 676..939 / 941..943 / 951 / 954..1030`，与 §10.3 列的 6 段**逐个相同**。
- 「严格连续」分段口径成立：脚本默认 `--cluster-gap 0`，222 段确为无间隙合并；末 3 段 `[220]24027..24197`、`[221]24200..24455`、`[222]24458..28407` 全对。
- Rust 侧缺行不污染分母：窗内 93 帧缺行被计为不可比（28388−93=28295）✓。

## 4. T3 分岔前 — PASS（实质），计数措辞见 P2-1

```
$ ... --start 20 --end 1131
comparable 1098   all_equal 1025   mismatch 73
per-field: status 66  room 6  health_hi 2  scrolldir 1  health_lo 0
```
1098/73/1025 三数全中。73 的分段分解（17 段）经我逐段加总 = 11+51+11 = 73，且 11 个单帧点
`f1093/1097/1101/1105/1108/1112/1116/1120/1123/1127/1131` 与 §10.3 line 表**逐个对上**。

**「f30..1030 除 §6.2 那 11 个已知点外逐帧全同」**：
```
$ ... --start 30 --end 1030
span 1001   comparable 994 (7 缺行)   mismatch 8   latch 0/994
mismatch frames: f940 f944 f945 f948 f949 f950 f952 f953
```
- **实质成立**：994 个可比帧里只有这 8 帧失配，其余 986 帧五字段全同；`$0014` 锁存面 0/994 全同，「含 `$0014`」的括注正确。
- **计数不准**：送审点给的 11 点集合包含 `f27/f28/f29`，它们不在 30..1030 内 ⇒ 窗内应为 **8** 点。逻辑上「除 11 点外全同」不算假命题（排除集大于实有失配集），但写在 §10.8「已证实（A 级）」条目里是给读数加了不存在的失配项。

附：`--start 20 --end 1000` → 981 帧 / 可比 967 / 失配 11 / 四字段全同 957 / per-field `0,2,4,6,1` / latch 0
⇒ §10.4 对 §6.2「10 vs 11」的**更正成立**（确有 11 处失配，f27 属五字段而非四字段口径）。

## 5. T4 首个实质分歧 — PASS（结论），一处措辞 P2-2

**f1030 `p1`**：`0x09`。`tools/movie_ref/bizhawk_ref.lua:44-45` 位表 `A=0x80 B=0x40 Select=0x20 Start=0x10 Up=0x08 Down=0x04 Left=0x02 Right=0x01`
⇒ `0x09 = Up|Right`，与「按 Up+Right 退球」一致。我另写了一份 inputlog 解析（数据行第 n 行 = 0-based 帧 n−1，共 28408 数据行）
独立核对 f=20..28407：**p1 失配 0**，f1030=`0x09`、f1079=`0x81`。

**status 序列（两侧，f1028..1085 关键段）**：

| 帧 | Biz `status` | Rust `status` |
|---|---|---|
| 1028–1030 | `03` Roll | `03` Roll |
| 1031 | `04` PntUp | `03` Roll |
| 1032–1080 | `01` Run | `03` Roll |
| 1081 | `01` Run | `00` Stand |
| 1082–1085 | `01` Run | `01` Run |

（枚举依据 `refs/m1disasm/SRC/constants.asm:103-113`：0 Stand / 1 Run / 2 Jump / 3 Roll / 4 PntUp /
5 Door / 6 PntJump / 7 Dead / 8 Dead2 / 9 Elevator；255=sa_Begin，与 f0..f27 全 255 自洽。）

⇒ f1031 起 biz 已是 PntUp/Run，rust 仍 Roll 到 f1080、f1082 才收敛 = **50 帧延迟**，「退球晚约 50 帧」与数据一致。
另两个附属主张也成立：f1093..1131 biz status 恒 `01`，rust 该段 28×`01` + 7×`00` + 4×`04` = 11 失配；
biz `room` f1132..1137 = `14→F1→F1→F2→F4→FF`、rust 全程 `FF`（§10.5 的「分水岭」描述准确）。

**P2-2**：§10.5 line 492「f1031..1080 期间 `p1` **恒为** `01`（无 `Up`）」——f1079 `p1=0x81`（A|Right），
其余 49 帧确为 `01`。全段确实无 `Up(0x08)`，故「无 Up」这半句与论证成立，只是「恒为 01」是错的。

## 6. T5 像素面口径 — PASS（真复跑判定与探针；Mesen2 采集腿未复跑）

**口径属实**（代码级）：
- `tools/movie_ref/compare_pixels.py`：`wl = diff_visual.load_whitelist() if args.use_whitelist else []`、
  `gap_map = ... else []`，随后 `diff_visual.diff_frames(gname, oracle_bytes, rust_bytes, wl, gap_map)`。
  即默认**空白名单**，且判定实现与 L4 门禁同一份（`tools/diff_visual.py:143`，白名单外逐像素 RGBA 零容差）。
- BizHawk 腿不可零容差**有据**：`/tmp/bizshots` 136 张全 **256×224**（rust 256×240，尺寸即不可比）；
  调色板逐通道差 ±1（实测 biz/rust：`007C8E/007C8D`、`9390FE/9290FF`、`4240FE/4240FF`、`64B0FE/64B0FF`、
  `B53220/B53120`、`EB9F23/EA9E22`、`FEFEFE/FFFEFF`）⇒ 换腿声明不是借口，是硬件级事实。

**复跑结果（逐值全对）**：136 点配对、`0 px` = **83/136**、分歧 53 点、聚类 5 段
（f20=350；f670..f980 共 49 点；f1110/f1130/f1160 各 80）。逐点表与 §10.6 **每一行相同**：
f670=9076、f680=9180、f690..920=9178、f930=9275、f938..941=9270、f942=9264、f943=9078、f944=9130、
f945=9066、f946=8842、f947..948=8973、f949=8948、f950=8823、f951=8808、f952=9009、f953..980=8808；
f30..660 共 64 点 `0 px`；f990..1100 12 点 `0 px`；f1120/f1140..1150/f1170..1200 `0 px`。

**平移探针复跑**：dx=1 重叠 61200 ⇒ 残差 f690/800/900=**261**、f670=**312**、f680=**325**、f960/980=**157**（dx=0 为 9178）；
f690 dx=1 残差 bbox x=24..122 / y=34..145。与 §10.6 全对。f20 任何平移 best_diff 仍 350（真内容差，非相位）。

**我另行自算的两条**：f20 350px bbox x=96..158/y=96..126，oracle 恒 `(0,0,0)`、rust 恒 `(192,223,255)=C0DFFF`；
f1110/1130/1160 各 80px bbox x=135..147/y=193..208，两侧同四色集 {`000000`,`388700`,`B53120`,`EA9E22`}，
且 f1130 与 f1110 的交换对统计严格互逆（26/21/15/6/3 全镜像）⇒ §10.6「颜色互换、极性相反」成立。

**Rust 腿已独立重放**（不是复用别人的产物）：`rust_screenshot` 17.39 s 产出，状态文件 **28308 行逐字节相同**，
136 张 PNG 与 `/tmp/rustshots-full` **逐字节相同** ⇒ 像素面输入是可复现的。

**未复跑**：Mesen2 采集腿（会覆写仓库内 `shots/`，且成本≈30 s + 模拟器启动）——使用现成 `shots`
（mtime 10:25，136 张）复跑判定与探针已足以证伪/证实 §10.6 的数字面。**此点明确记为「未复跑，仅核对产物与口径」。**

## 7. T0 §10.0 工具修正 — PASS（一处证据缺口）

`git show be08a3c6`：`for frame = 0, TOTAL_FRAMES - 1 do`（原 `0, TOTAL_FRAMES`），
`run_bizhawk_trace.sh` 成功判据由「文件存在」改为「行数 == frames」——方向正确（闭区间会多跑一帧导致越界/挂死）。
产物面：`/tmp/bh-fix-28408.jsonl` 与 `/tmp/bh-full-28408.jsonl` **逐字节相同**（修正未改变数据）；
`1200` 帧小窗：`/tmp/bh-old-1200.jsonl` 与 `/tmp/bh-fix-1200.jsonl` 均 1200 行且逐字节相同。
§10.1「退出码 137」属运行时观测量，我未复跑（不必要）。

## 8. T6 过度声明 — P1 ×3、P2 ×7

### P1-1 §10.3 line 424 分岔后失配数错（可机械反证）

文档：「`f=1132..28407（分岔后）` | 27197 | **26408** | 784」。
实跑 `--start 1132 --end 28407`：可比 **27197** / 全同 **784** / 失配 **26413**。
反证：26408+784=27192 ≠ 27197（表格自身不自洽）；且全片失配 26486 − 分岔前 73 = **26413**。
**修法**：`26408 → 26413`。

### P1-2 §10.3 line 463-464 分段表两行错

| 文档行 | 实跑（权威窗口） |
|---|---|
| `f15020..16019 \| 215 \| 0 \| 215` | 996 / 215 / **781**（列错位：把 all_equal 填进了可比列） |
| `f28020..28400 \| 381 \| 0 \| 381` | 窗内应为 `f28020..28407` = **388** / 0 / **388** |

反证：381 = 28400−28020+1 恰是 `--end 28400` 的口径 ⇒ 该行混用了另一次 run 的窗口，
与表头「窗口 f=20..28407」冲突。**修法**：改为 `996/215/781` 与 `388/0/388`，窗口写 `28407`。

### P1-3 §10.3 line 448「最后一次全同出现在 f24457 之后即无」

实跑 equal_runs 共 198 段、末段 `[15525,15527]` ⇒ **最后一次五字段全同是 f15527**，
f15528..28407 共 12880 帧零全同。文档句子虽在字面上不假（f24457 之后确无），但把「分岔后完全无全同」的窗口
少算了约 9000 帧，且句子本身语义残缺（「出现在 f24457 之后即无」读不通）。
**修法**：改写为「最后一次五字段全同 = f15527；f15528..28407 共 12880 帧无任何全同帧」。

### P2 清单（措辞/可复现性/证据缺口）

| # | 位置 | 问题 | 修法 |
|---|---|---|---|
| P2-1 | §10.3 line 430 / §10.8 line 618 | 「除 §6.2 那 11 个已知点外逐帧全同」——窗内实为 **8** 点（f27/28/29 在窗外） | 写「除 f940/944/945/948/949/950/952/953 这 8 点外」 |
| P2-2 | §10.5 line 492 | 「`p1` 恒为 `01`」——f1079=`0x81` | 写「除 f1079（`0x81` A+Right）外均为 `01`，全段无 Up」 |
| P2-3 | §10.2 line 396 | 「缺行 100 帧…见 §10.3 的 30 段清单」——清单只覆盖窗内 93 帧 | 注明「另 7 帧在 f=2,4,5,6,15,16,17（窗外）」 |
| P2-4 | §10.0 line 370 | 「修正前 1201 行产物」——现存 `/tmp/bh-old-1200.jsonl` 为 1200 行，无 1201 行产物可核 | 删「1201 行」或补该产物/NOTES 说明 |
| P2-5 | §10.7 line 611 | 「md5 汇总 `cce608f6…`」——聚合口径未定义，我试 4 种口径（`md5sum` 带文件名 / 仅哈希排序 / 仅哈希 / `cat \| md5sum`）得 `efedceb7/487ba587/0d8a5739/bde4d906`，无一相符 | 写明口径（命令原文），或改成「逐文件 sha256 清单」 |
| P2-6 | §10.6 第 1 条 / §10.8 第 5 条 | 「sprite/OAM 完全一致」是从残差 bbox 推断，无 OAM dump 取证（工具链无该产物） | 降级为「残差集中在 x24..122/y34..145 的小对象区，**推测** sprite 未偏」，或补 OAM 采集 |
| P2-7 | §6.2 表 vs §10.3 | 同一 `status` 字节 §6.2 写十进制 `20`、§10 写十六进制 `14`，跨节对读易误判 | 统一 `0x`-前缀十六进制，§6.2 加更正注 |

### 另附「已核对但无误」的主张（避免只列问题）

- 「分岔后失配是级联、不可逐字段归因」——§10.3 自述「该段计数只说明分岔已发生」，未把级联当独立缺陷 ✓
- 「`$0014` 锁存失配 223 首 f1133 是**结果**」——与 health 写点时序一致，文档归因正确 ✓
- 「两腿已不在同一场景」——room 分布 biz 255 占 26962 / rust 28169，抽样 f3000/f5000/f12000/f18000/f20000 全部吻合 ✓
- 「像素面空白名单」——代码级确认，非声明性 ✓

## 9. 独立结论

**可复现（我亲自跑出的数字）**：
1. 轨迹 28408 行 / f=0..28407 连续 / 末行 `health_lo=72,health_hi=0,status=1,room=255,scrolldir=0,p1=8` / sha256 如上；
2. RAM 面 `28295 / 1809(2280) / 26486 / 222 / 223@f1133 / 0`、逐字段 `23055/14248/21071/1546/15554`、首失配 `f27 room FF/09`；
3. 分岔前 `1098 / 73 / 1025`、17 段分解、11 个单帧点；`f30..1030` 仅 8 帧失配（994 可比）、latch 0/994；
4. `f1030 p1=0x09`、status 序列 50 帧延迟、`f1093..1131` 11 失配、biz room `14→F1→F1→F2→F4→FF`；
5. 像素面 `83/136` 零点、136 点逐值表、dx=1 残差 261/312/325/157、f20 bbox 与颜色、f1110/1130 极性互换；
6. Rust 腿重放 17.39 s，状态文件与 136 PNG 逐字节相同（确定性成立）。

**不可复现 / 未复现（已在正文标注）**：
1. §10.7 的 `md5 汇总 cce608f6…`——口径未定义，4 种常见口径均不符（P2-5）；
2. §10.0 的「修正前 1201 行产物」——现存产物 1200 行（P2-4）；
3. Mesen2 采集腿本身（会覆写 `shots/`）——未复跑，但用现成产物复跑了全部判定逻辑与探针；
4. §10.1 的「退出码 137」、§10.6「既有 L4 门禁本来 0 px」（`data/status/baselines/baseline.json`
   无逐点视觉记录，且我未跑 `gates/cli.py l4`）——未复核，属引用既有门禁状态。

**裁定**：送审的数字面**不是编造的**——每一条主结论我都在独立进程里跑回来了，包括最容易被"抄上一次结果"糊弄的
Rust 腿重放（逐字节相同）与像素面逐点表（136/136 行全对）。问题集中在 §10.3 的三处笔误与表格式错位，
属「同一实验的多次 run 混用 + 手工转录」类错误，不构成数据伪造；但按项目纪律，**P1 三行必须在关闭本步前修掉**。

**未做**：`git commit`（按用户指令，复核档由用户自行提交）。
