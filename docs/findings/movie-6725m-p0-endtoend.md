# 6725M any% 端到端跑通（P0）：全片无缺口，裁定腿首分歧 = f2249（2026-11-23）

> 目的：回答「我们现在能不能从开机跑到结局」，并把 608 个 unported 里**真正挡路的那几个**
> 筛出来。判据不是像素 0 差（那需要全场对齐），而是「跑完没跑完、崩在哪、与裁定腿首分歧在哪」。
> 证据分级：A=实测产物/ROM 字节，B=静态推理，C=假设。

## 0. TL;DR

| 问题 | 结论 | 级 |
|---|---|---|
| Rust 腿能跑完 28690 帧输入吗？ | **能**。7.5 s 跑完，`缺口阻塞 0 采集点`，无 panic | A |
| 走得到结局吗？ | **不能**。Samus 在 f6135 血量归零，此后停在 Game Over/密码屏，`InArea` 全程为 0（没离开 Brinstar） | A |
| 与**裁定腿**（Mesen2）首个状态分歧在哪？ | **f2249**（=`known-residual 91ea6fbf` 的同一帧；不是新 bug） | A |
| 与覆盖腿（quickerNES）首个分歧？ | f1031 —— 已由 `movie-6725m-f1031-unroll.md` 定谳为**输入生效相位差**，非逻辑 bug | A |

## 1. 怎么跑的（可复现）

```bash
# 1) inputlog → 调度表（caps 每 500 帧一个，仅用于人工看画面）
python3 - <<'PY'
import sys; sys.path.insert(0,'tools')
from l4_diff_run import schedule_text
rows=[[int(x,16) for x in l.split()] for l in open('movies/eien86-6725M.inputlog')
      if l.strip() and not l.startswith('#')]
frames=len(rows)                       # 28690
joy={i+1:r[0] for i,r in enumerate(rows)}
joy2={i+1:r[1] for i,r in enumerate(rows) if r[1]}   # f3265 起有 2P
caps=[(f,f"f{f}") for f in range(500, frames+1, 500)]
open('/tmp/anyp-full.sched','w').write(schedule_text('anyp-full',frames,joy,caps,None,joy2=joy2 or None))
PY
cargo run -q --release -p game-core --bin rust_screenshot -- \
  --schedule /tmp/anyp-full.sched --out /tmp/anyp_out --state-out /tmp/anyp_state.jsonl
# → 完成：产出 57 采集点，缺口阻塞 0 采集点（real 7.5 s）

# 2) 裁定腿：Mesen2 跑同一录像前 6600 帧（temp 用例，不入库）
python3 tools/l4_diff_run.py run movies/cases/_tmp-anyp-6725m-judge.txt
```

## 2. Rust 全片状态面统计（`/tmp/anyp_state.jsonl`，28598 行）

`(GameMode, MainRoutine)` 分布（前八）：

| 组合 | 帧数 | 含义 |
|---|---|---|
| (1, 4) | 13146 | Game Over / 演示态（占比 46%） |
| (0, 3) | 10788 | 正常游戏 |
| (0, 5) | 2546 | 暂停 |
| (0, 8) | 1751 | SamusIntro |
| (0, 9) | 343 | WaitTimer |

血量事件（`$0106/$0107` 变化点）：f6135 与 f10055 与 f16650 三次归零，f25577 才重新进游戏
（`MoreInit → SamusInit → SamusIntro → GameEngine`），此后停在 (1,4)/(0,5) 直到片尾。
⇒ 全片 2/3 的帧数落在 Game Over 与暂停里，**后半段输入没有产生可用信号**。

## 3. 裁定腿对照（Mesen2，f30..6590）

各字段首个差异帧：

| 字段 | 首个差异 | rust | mesen |
|---|---|---|---|
| `RoomNumber($005A)` | f485 | 243 | 242 |
| `SamusScrX/SamusY/SamusX` | **f2249** | 8E/C1/72 | 8F/BE/75 |
| `SamusStatus($0300)` | f2254 | 00 | 01 |
| `MapPosX($0050)` | f2268 | 04 | 05 |
| `HealthLo/Hi` | f2616 | 32/02 | 64/01 |
| `MainRoutine($001E)` | f4146 | 03 | 09 |
| `GameMode($001D)` | f4184 | 00 | 01 |

`RoomNumber` 的 f485 属已登记的 `$005A≠FF` 载入窗（三腿契约 R7：不进分母、逐帧列账）。

## 4. f2249 逐字段（同一 f 标签，A 级）

| 字段（地址） | rust f2246..2252 | mesen f2246..2252 |
|---|---|---|
| status `$0300` | 00 01 00 00 00 00 00 | 00 01 00 00 00 00 00 |
| y `$030D` | C1 C1 C1 C1 C1 C1 C1 | C1 C1 C1 **BE BC B9 B7** |
| x `$030E` | 73 73 73 **72 70 6F 6D** | 73 73 73 **75 76 78 79** |
| subY `$0310` | 7D 7D 7D 7D 7D 7D 7D | 7D 7D 7D **B5 25 CD AD** |
| spSubY `$0312` | 00 00 00 00 00 00 00 | 00 00 00 **38 70 A8 E0** |
| spdX `$0309` | 00 00 00 **FE FE FE FE** | （未采） |
| scrX `$0051` | 8F 8F 8F 8E 8C 8B 89 | 8F 8F 8F 8F 8F 8F 8F |

读数：f2249 起两侧**走相反方向**——rust 向左（`spdX=0xFE`，x 72→6D），mesen 向右且**上升**
（y C1→B7，`spSubY` 每帧 +0x38）。同期输入（inputlog）f2246=Left、f2249=空、f2252=Right；
`Joy1Latch` 两侧恒差 1 帧（既有 latch 相位面，非本条）。

⇒ f2249 不是「帧轴错位一帧」能解释的（一帧错位下 y 不会有 3px/帧 的持续反向），
必须是**分支选择**不同。归属待定（§6）。

## 5. 与既有结论的关系

- f1031（quickerNES 侧）已定谳为**相位差**：把输入整体提前一帧后 40..1200 全同（1152/1152），
  说明该段是 quickerNES 行相位与 Rust 行相位不同相，不是逻辑 bug。见
  `docs/findings/movie-6725m-f1031-unroll.md` §5。
- f2249 此前登记为 `known-residual`（`91ea6fbf`）：当时的证据是「emu 该帧先上 3px 再右 2px，
  两侧 spX=08/F7 相位一致」。本轮用 **Mesen2（裁定腿）**复现同一帧，确认：
  分歧在 `y/x/subY/spSubY/spdX` 五个字段上同时出现，且此后不再收敛。

## 6. 下一步（按优先级）

1. **f2249 定谳**：先取 `SamusInLava($0064)`、`SamusOnElevator($0307)`、`Joy1Status/Change`
   与房间号，判断 mesen 的「上升」是**平台/电梯**、**受伤击退**还是**跳跃**；再回 asm 对分支。
2. 若 f2249 属跳跃/落地判据，优先查 `SamusJump`/`LCF2E`（`SetSamusRunAccel` 之后的那件）
   与 `CheckMoveVertical` 的 `radiusY` 换算——上一轮已在此族修过块对齐闸。
3. 在 f2249 修好前，**不要把全片对照当回归判据**：f2249 之后的一切分歧都是级联。
