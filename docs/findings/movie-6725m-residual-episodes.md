# 6725M 残留分歧判定归档：两段真差、一段进复现、一段进加载窗（2026-11-18）

> 任务来源：`docs/superpowers/plans/2026-11-18-three-leg-oracle-alignment.md` 的 Task 6。
> 输入：Task 1 `data/status/leg_offsets.json`（帧偏移与分母）、Task 2
> `data/status/leg_episodes.json`（43 段三分类）、Task 5 `tools/movie_ref/mechanism_scene.py`
> （机制级最小重现，尚未产出 informative 短场景）、Task 3 `docs/spec/oracle-verification-chain.md`
> （R6/R7 判据）。本文只做「判定归档」，不重新推导数字。
> 证据分级沿用 AGENTS §3：A=硬证据（ROM/轨迹/逐帧数值），B=可复现静态推理，C=假设。

## 0. TL;DR

| 问题 | 结论 | 级 |
|---|---|---|
| 帧偏移是多少 | 状态面 `biz_to_mesen=+1`、`rust_to_mesen=0`；锁存面 `+2/+1`（恒比状态面偏一行） | A |
| f20..1199 有多少段失配 | **43 段**：phase 6 / transient 31 / value 6 / undecidable 0 | A |
| 其中几段是干净的行为差 | **1 段**：`biz f1031..1081 status`（= mesen 轴 `f1031..1081`，biz 行 `1030..1080`） | A |
| `f943..952` 那一簇是什么 | `kind=value` 但 `during_load=true` ⇒ 按 R7 不进归因分母，成因未定 | A（分类）/C（成因） |
| 进不进机制级复现 | `f1031..1081` 进（唯一候选）；`f943..952` 不进 | B |
| 现在能升台账吗 | **不能**：机制级短场景未落地，`coverage_ledger` 三条仍是 `route-only` | A |

## 1. 判定表

| 段（biz 行号） | mesen 轴 | 字段 | kind | during_load | 进复现 | 理由 |
|---|---|---|---|---|---|---|
| `1030..1080` | `1031..1081` | `status($0300)` | **value** | false | **是** | 51 帧持续、无任何公共位移 k∈[-2,2] 能对齐、对面 ±5 帧内不存在该值序列；落在 `$005A=0xFF` 干净窗内 |
| `944..953` | `945..954` | `room($005A)` + `status` | value | **true** | 否 | 该段 `$005A≠0xFF`，属房间加载瞬态；按 R7 不进归因分母（仍逐帧列账） |
| `24..29` | `25..30` | room/scrolldir/status/health_hi | phase | true | 否 | 上电入场段，`f<40` 跨腿相位不可用（§11.2 更正） |
| `672` / `675..678` / `676..678` | 同行 +1 | room | transient / phase | true | 否 | 单帧尖峰或位移可对齐；加载窗内 |
| `940` | `941` | scrolldir | transient | false | 否 | 单帧尖峰，对面窗口仅一帧异值 |
| `1131..1136` | `1132..1137` | status+room | phase | true | 否 | 位移可对齐 + 加载窗 |
| `1176..1177` / `1191..1192` | 同行 +1 | status | transient | false | 否 | 单帧尖峰（rust 侧），无持续值差 |

其余 30 余段的逐条记录在 `data/status/leg_episodes.json` 的 `episodes[]`，本表只列代表项。

## 2. 分母怎么算（R7 要求三数）

| 项 | 值 | 来源 |
|---|---|---|
| 总帧 | 1200 | 窗口 f20..1199 |
| 可比帧 | 1180 | Task 2 `denominators.total` |
| 排除：加载窗 | 33 | `$005A != $FF`（含 14 个缺行） |
| 进分母 | **1147**（rust 腿 1155） | Task 2 `denominators.compared` |
| 进分母后的干净差 | 1 段 / 51 帧 | 上一节 |

纪律：排除帧必须**逐帧列账**（`leg_episodes.json` 每条带 `during_load` 标记），不是「不计入就丢掉」。

## 3. `f1031..1081` 的判定链（唯一进复现的段）

1. **不是逐例程判据错**（A）：`docs/findings/movie-6725m-f1031-unroll.md` 已证 `SamusRoll ; 07:D0E1`
   与 `ObjectCheckMoveVertical` 两侧判据、地址换算、瓦分类一致，Rust 在**自己的 x 上**回滚动是对的。
2. **现在也不是「输入 +1 帧」**（A）：该假说源自旧轨迹文件的行内不一致；相位改写后
   `post[f] == pre[f-1]` 28 388/28 388 全等，状态面 `rust_to_mesen=0`，两腿同帧号可比。
3. **三方仲裁后的归属**（A）：`f1030..1080` 上 Mesen2 与 Rust 均为 `status=03`（Roll），
   quickerNES 为 `04`（退球）⇒ 该差异是 **quickerNES ↔ Mesen2 的核心差**，不是移植错。
4. 因此进机制级复现的目的**不是修移植**，而是把「同输入下退球时机」做成可判定的短场景，
   以便后续若出现同类分歧时能用 Mesen2 腿独立裁定（Task 5 的 `informative` 门槛就是为此设的）。

## 4. `f943..952` 为什么不进复现

- 该段 `$005A` 非 `0xFF`：`constants_ram.asm:589` 定义 `RoomNumber db ;$5A ;Room number currently being loaded. #$FF=no room requested.`
  （`crates/game-core/src/ram_map.rs:216` = `ROOM_NUMBER`）。两侧在加载链上的中间值集合本就不同
  （Mesen2 采到 `F1..F4` 的不同分布，quickerNES 亦然，Rust 第三种）——**中间值的帧级比较无意义**。
- 端点与「链是否出现」仍可比（§11.3 更正后保留的结论），但那属于加载链机制，不属玩法行为。
- 若要升格判定，需要「加载链速率」这一独立机制场景（当前未建）。

## 5. 为什么台账还不能升 `verified`

`data/status/coverage_ledger.json` 的三条（`door-transition` / `morphball-unroll` /
`room-load-chain`）都是 `route-only`。要升 `verified` 需要**机制级最小重现 + Rust 同场景 0px**
（Task 4 定义的阈值），而 Task 5 目前只能证明：①两腿真跑 ②失败路径 fail-loud
③常量窗会被判 `uninformative` 并 exit 3。**可复跑、≤120 帧、窗口内 status 有 ≥3 次跳变的
场景尚未落地**（120 帧内 morphball 的 `20→21` 在 +146、`21→22` 在 +246，天然超窗），
故如实停在此处，不用 fixture 伪造状态面凑数。

## 6. 复现命令

```bash
python3 tools/movie_ref/align_legs.py --json-out /tmp/off.json          # 状态面 +1/0、锁存面 +2/+1
python3 tools/movie_ref/classify_episodes.py --json-out /tmp/eps.json   # 43 段三分类与分母
python3 tools/movie_ref/mechanism_scene.py --case cases/case-password-justin.txt --frames 600
# ⇒ RESULT: uninformative（常量窗，无判别力）, exit 3 —— 常量窗不得当证据
```

## 7. 未证与未做

- `case-password-justin` 600 帧窗口内 `status/room` 全 0 ⇒ 该窗**无判别力**，不得作为对照证据。
- 机制级短场景未落地（见 §5），`morphball-unroll` 的实测差异仍只有录像级证据。
- `f>1200` 的三腿互证未做；`f22449` 起的 2P 段只做过单点抽查。
- 加载链速率（`F1..F4` 推进节拍）未立机制场景，`f943..952` 的成因停在 C 级。
