# 6725M any% 端到端：全片跑通 + 裁定腿首分歧 f2249

**日期**：2026-11-23
**判据**：`tools/l4_diff_run.py`（Mesen2 = 裁定腿）；rust 全片用 `rust_screenshot --schedule --state-out`
**范围**：`movies/eien86-6725M.inputlog`（28 690 帧，any% 07:52.69，TASVideos 6725M）

## 1. rust 全片跑通（A 级）

调度由 `l4_diff_run.schedule_text` 生成（inputlog 每行三列十六进制）：

```
frames=28690  joy={f: p1}  joy2={f: p2}（f3265 起 2P，242 条）  caps 每 500 帧
cargo run -q --release -p game-core --bin rust_screenshot -- \
  --schedule /tmp/anyp-full.sched --out /tmp/anyp_out --state-out /tmp/anyp_state.jsonl
```

结果：**real 7.5 s**、57 采集点、**缺口阻塞 0**、无 panic；`--state-out` 28 598 行（建造房载窗内的帧不产行）。

`(GameMode, MainRoutine)` 分布：

| 组合 | 帧数 | 含义 |
|---|---|---|
| (1,4) | 13 146 (46%) | Game Over / 演示 |
| (0,3) | 10 788 | 正常游戏 |
| (0,5) | 2 546 | 暂停 |
| (0,8) | 1 751 | SamusIntro |
| (0,9) | 343 | WaitTimer |

血量归零点 **f6135 / f10055 / f16650**；f25577 才回到 (0,3)。`InArea($0074)` **全程 0**（未离开 Brinstar）
⇒ **rust 走不到结局**：本轮全片只证「不崩、不阻塞」，不证「能通关」。

## 2. 裁定腿首分歧 = f2249（A 级）

Mesen2 跑同 inputlog 前 6600 帧（用例见 §5），逐字段首差：

| 字段 | 首差帧 | rust | mesen |
|---|---|---|---|
| `RoomNumber($005A)` | f485 | 243 | 242 |（`$005A≠FF` 载入窗，R7 不进分母）|
| `SamusScrX / SamusY / SamusX` | **f2249** | 8E / C1 / 72 | 8F / BE / 75 |
| `SamusStatus` | f2254 | 00 | 01 |
| `MapPosX` | f2268 | 04 | 05 |
| `HealthLo / HealthHi` | f2616 | 32 / 02 | 64 / 01 |
| `MainRoutine` | f4146 | 03 | 09 |
| `GameMode` | f4184 | 00 | 01 |

f2249 之后一切差异都是**级联**，不能当独立缺陷计数。f2249 本身就是历史挂账的 `known-residual`（`91ea6fbf`）。

## 3. f2249 分支身份：受伤击退（A 级）

逐帧探针（watch `0300/0307/0308/030A/030D/030E/0064/0070/0072`）：

| 帧 | mesen | rust |
|---|---|---|
| f2248 | `Samus.isHit($030A)=0x28` | `0x00` |
| f2249 | `Knockback($0072)=1`、`Invincible($0070)=49`、`SpeedYInt($0308)=$FD(-3)` | 无击退 |
| f2249+ | y C1→BE→BC→B9（上升）、x 73→75→76→78 | y 恒 C1、x 72→70→6F（向左） |

`OnElevator($0307)=0`、`InLava($0064)=0` ⇒ 排除电梯/岩浆。asm 链条与实测完全对上
（`CheckHealthStatus ; 07:CDFA`：`isHit & $20` ⇒ `InvincibleDelay←$32`、`KnockbackDir←isHit>>3 & 1`、
`speedY←$FD`、`AccelY←$38`）。像素面 `b2248`/`b2249` 两腿 **0 px**，`b2250` 起 7383 px
⇒ 分歧发生在**接触判定那一拍**，画面滞后一帧才变。

## 4. 接触判定为什么没在 rust 触发（未定谳，有否证）

`CollisionDetectionEnemy_ReactToCollisionWithSamus ; 07:F282` 的判据是
`dy_adj < sry+ery && dx_adj < srx+erx`（`SetSamusIsHitByEnemy ; 07:F338` 再把 `DistHi<<3` 塞进 `Samus.isHit`）。
`isHit=0x28` 反推 `DistHi=$05`（bit2 恒置 | bit0 ⇒ **敌在 Samus 左侧、y 不小于 Samus**）。

f2248 两腿 RAM 对读：

- **12 个敌槽只有 slot5 不同**（emu `y=C4 x=0E status=29` / rust `y=18 x=4C status=27`），
  但逐帧轨迹显示 slot5 在两侧都是**静止**且 `x=0x0E` 与 Samus `x=0x73` 相距 0x65 ⇒ **不是它**。
- 唯一「左且 y 相近」的是 **slot2**（`y=C4 x=62 status=29 type=04`）——**两腿逐字节相同**，
  含 `radiusY=0C / radiusX=08 / hi=01`；Samus `radiusY=0F / radiusX=04 / hi=01`。
- 按判据：dy=3 < 0F+0C=27 ✓，但 **dx=0x11=17 > 04+08=12 ✗** ⇒ 用**帧末位置**两侧都不该撞。

因此根因候选只剩两类（下一轮直接证伪其一）：

1. **判定时刻差**：asm 在敌槽更新循环内逐敌判（敌已移动、Samus 位置取本帧值），rust 在同一循环内
   的取点/次序不同 ⇒ 敌方中途位置与帧末位置不同（slot2 可能在帧中更靠近 Samus）。
2. **半径来源差**：`Temp05_YSlotRadiusX` 可能不是 `EnsExtra.radiusX`（而是某张按类型的常量表/或已含 Samus 半径之和）。

已排除：敌槽数量/类型/status 派发（除 slot5 外全同）、螺旋转（无）、i 帧（两腿 `$70` 都为 0）、
Samus 死亡门（status≠7/8/FF）。

## 5. 仪器（可复用）

- 全片 rust：`/tmp/anyp-full.sched` + `--state-out`（7.5 s），是每修的**回归仪器**；
  但 f2249 修好前只看「是否仍阻塞/是否崩」，不看分歧量。
- Mesen2 裁定腿前缀用例（临时，存 `/tmp/parked-cases/`）：
  `_tmp-anyp-6725m-judge.txt`（f6600，watch 11 字段 + 3 采集点）、
  `_tmp-anyp-6725m-f2249.txt`（f2272，2240–2271 每帧一采）、
  `_tmp-anyp-6725m-f2249b.txt`（分支身份 11 字段）、
  `_tmp-anyp-6725m-s5.txt`（slot5 轨迹）。
- `--state-out` 行 = tick 收口前的 seam 状态；`frames.jsonl` = Mesen2 `endFrame` 采样。
  两者比字段必须先过 `tools/phase_align_check.py`，否则±1 帧相位会被读成行为差。
- 教训：`analysis/traces/case-movie-anyp-p0.frames.jsonl` 是 **3666M** 的轨迹，**不能**与 6725M 混用。

## 6. 结论与下一步

1. 本轮 P0 的净结果：**rust 全片 28 690 帧零阻塞跑完（7.5 s）**，但走到 Game Over 3 次且不出区 ⇒ 「能通关」仍未证。
2. 与裁定腿的首分歧是 **f2249 受伤击退未触发**，属历史挂账帧，不是新缺陷。
3. 下一轮：按 §4 两个候选做判定时刻/半径来源的对照实验，定谳后修，再用全片 + 6600 帧裁定腿复核首分歧是否后移。
4. 其后才是 P1（Kraid/Tourian/Ridley 三区房扫补分母）——在 f2249 未修前，全片对照无法提供新的有效信号。
