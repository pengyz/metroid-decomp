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

## 7. f2249 定谳（2026-11-23 收口）

> 【2026-11-23 更正】§4 的两条候选（判定时刻差 / 半径来源差）**都不是根因**。
> 真因在 **rust 侧**：`crates/game-core/src/weapon/projectile.rs` 的 `uw_skree_projectiles`
> 尾部「Samus 5 伤害」是这条径的**简化替身**，它

1. 用硬编码半径 `+4`（`dy < sry+4 && dx < srx+4`）代替 `AreObjectsTouching` 的
   **Object 半径和**（`$0301/$0302` + `$0341/$0342`）；
2. 命中后**直写** `SamusInvincibleDelay←$32`、`$010F←$80`、`HealthChange←$50`，
   **跳过 `KnockSamusBackIfCollided ; 07:F311`** ⇒ `Samus.isHit` bit5 从不置位 ⇒
   下一帧 `CheckHealthStatus ; 07:CDFA` 的整条击退链（`speedY←$FD` / `AccelY←$38` /
   `KnockbackDir←(isHit&8)>>3`）不跑。

这同时解释了实测的三个现象：i 帧**早一帧**、Samus **不上升**、方向位**恒反**。

asm 真形（`refs/m1disasm/SRC/prg7_engine.asm:12410-12431`）：

```
    lda SamusInvincibleDelay
    bne @endIf_A                    ; i 帧内跳过
    ldy #Samus - Objects
    ldx #PowerUpDraw - Objects
    jsr AreObjectsTouching          ; 07:DC7F（Object 半径和 + CheckCollisionOfXSlotAndYSlot）
    bcs @endIf_A
    jsr IsScrewAttackActive         ; 07:CD9C（carry 清 = 有效 ⇒ 跳过）
    ldy #Samus - Objects
    bcc @endIf_A
        clc
        jsr KnockSamusBackIfCollided ; 07:F311
        lda #$50
        sta HealthChange
        jsr SubtractHealth
```

`KnockSamusBackIfCollided ; 07:F311`：`SamusHurt010F←$E0`（`07:F313`）；
`SetSamusIsHitByEnemy`（`Samus.isHit |= DistHi<<3`）；
`SamusKnockbackIsBomb←(Temp0F_DistX≠0 ? 1 : 0)`。

另一处 A 级细节：`CheckCollisionOfXSlotAndYSlot ; 07:F1FA` 的 hi 折算**分轴、按房间朝向**
——Y 轴只在竖滚房（`ScrollDir&2==0`）折 `+$100`，X 轴只在横滚房折 `+$100`
（asm `lda Temp03_ScrollDir / bne @else_sameHiY` 与 `lda Temp03_ScrollDir / beq @else_sameHiX`）；
不可滚的轴永不比 hi。旧形（以及本次修的第一版）两个轴一起折，会漏掉同屏跨页命中。

## 8. 修复效果（同轮实测）

| 判据 | 修前 | 修后 |
|---|---|---|
| 6725M 前 6600 帧（裁定腿 3 采点） | c3000=591px · c6000=14687px | **c1000/c3000/c6000 全 0px** |
| L4 矩阵（21 案/172 格） | match 166 · diff 6 · blocked 0 | 同（逐格不变） |
| Norfair 房扫 | ok 44 / mismatch 2 / blocked 0 | 未复扫（本修不动房内行为） |
| 测试 | 1078 | **1079**（+1 锚测 `skree_hit_sets_is_hit_bit5_and_does_not_touch_invincible_delay`） |

锚测的红证：把命中径改回「直写 inv、不置 isHit」⇒ `assertion left != right failed:
Skree 命中必须经 SetSamusIsHitByEnemy 置 bit5，实测 00`。

## 9. 下一轮入口

f2249 已不再是分歧点，6600 帧内的变现面**全部一致**。下一步是**把裁定腿向后推**
（f6600 → f12000 → 全片 28 690 帧），逐段把首分歧继续往后顶；
在那里暴露的才是 new：此项工作不需要再动 f2249 的假设。

## 10. 第二轮：复核 P1 修正后把裁定腿推到 f6300

### 10.1 复核抓到的问题与修正（commit `a4cfe0bc`）
独立复核（verdict=needs_revision）指出第一版 `d5a02caf` 有两处 P1：

- **P1-1 跨页折算错**：第一版写 `|Δlow| + $100`，使跨页距离恒 ≥256 ⇒ 跨页小距离命中被
  永久抹掉（相对旧形「忽略 hi」是回归）。asm 折的是**按页大小的有符号差**——竖滚 y 页
  240、横滚 x 页 256（`sec / lda Temp01_DiffHi / bne @RTS` 只在 |差| ≥ $100 时才拒）。
  可达例：竖滚房 Samus y=235/hi=0 与弹 y=5/hi=1 折后差 10。
- **P1-2 hi 未归一**：asm 对两槽 hi 都做 `eor PPUCTRL_ZP / and #$01`
  （`GetXSlotPosition_Common :10275-10279`、`GetYSlotPosition_Common :10287-10291`），
  `$FF` 是活影子（`ToggleNameTable ; 07:E9B7`）。漏掉它一旦 P1-1 放开就会让 `dist_hi` 的
  bit4/bit3（击退方向）翻错。

另修两条文档错：`SamusKnockbackIsBomb` 语义（0=同列垂直、1=斜向，asm `:10622` 附注文字与
代码相反）；`AreObjectsTouching` 的槽位（`ldy #Samus` / `ldx #PowerUpDraw` ⇒ Y 槽=Samus）。

### 10.2 测试脚手架的两条实测事实（写下来免得再踩）
- 槽 3（字节偏移 12）速度 `(0,+5)`（x 加 5、y 不变）、槽 0 是 `(0,-5)`（asm `txa/lsr/tay`
  得表索引 6 / 0）；`uw_skree_projectiles` **先 `ApplySpeedToPosition` 再把新位置写进
  `PowerUpDraw`** ⇒ 造测试必须按「移动后」摆并反推写入值。
- `PowerUpDraw` 的 radiusY/X（`$0341/$0342`）**会在绘制该帧时被
  `crates/game-core/src/render/oam.rs` 从帧数据 blob 字节 1/2 写入**（实测 Skree 帧给
  radiusY=4）⇒ 半径和不是「Samus 半径 + 0」，边界值取决于帧数据，锚测不宜钉死等号，
  只钉「明显超出不碰」。

### 10.3 修正后的效果
- 裁定腿 `/tmp/judge.txt`（3 采点）：c1000/c3000/c6000 **仍全 0px**（修正未回归）。
- 测试 1084 passed / 0 failed；L4 21 案/172 格 match 166 · diff 6 · blocked 0；
  `gates/cli.py check` PASS=23。

### 10.4 新前沿：f6300（本轮未定谳）

- 逐段二分：f6240–f6290 **全 0px**，**f6300 = 635px**（c6310=387px），与本次改动无关
  （第一版修复前后同值）。
- 差分 bbox `x24-197 / y33-207`，像素对全部是「emu 黑 / rust 有色」（270 橙、109 红…）
  ⇒ **rust 多画了精灵**，emu 该处为黑。
- **PPU 面逐区比对**（`ppu.hex`：CHR 8K + NT 4K + palette 32 + regs 3）：
  CHR **0 差**、NT **0 差**（四页 `$FF` 计数与互等性两腿一致）、palette **0 差**；
  唯一差异是 regs 末字节（emu 的 `ppu.ctrl` 取 `getState` 字段失败回落 0，属 dump 伪影）。

### 10.5 【2026-11-23 更正】「字段面全同」是假的——两次比较都读错了键

第 10.4 节最初写「标量字段/敌槽/OAM 逐帧全同 ⇒ 分歧只在 PPU 面」，**这是错的**，原因两层：

1. `rust_screenshot --state-out` 的每行是**整面内存 blob**（`{"f":N,"w":{"zp":…,"iram":…,
   "wram":…,"pw":…}}`），不是 `watch` 标签；而 emu 侧 `frames.jsonl` 的 `w` 是**标签字典**。
   按 rust 的键去 `emu.get(k)` 恒为 `None` ⇒ 全部 `continue` ⇒ 假「无差」。
2. 正确读法：`addr < 0x100 ⇒ zp[addr]`，`addr < 0x800 ⇒ iram[addr-0x100]`（`wram` 是
   `$6000+`）。改判后立刻出现差异。

**真实的字段面首差（限制 f≥6200 之后）**：

| 帧 | 字段 | rust | emu |
|---|---|---|---|
| f6256 | `SamusStatus($0300)` | `18` | `17` |
| f6296 | `SamusStatus` | **`FF`** | `18` |
| f6299 | `SamusStatus` | `00` | **`FF`** |
| f6303 | `SamusStatus` | `00` | `00`（重新合流） |

即：`$0300` 的 `18 → FF` 这段 dwell（4 帧）在 rust 侧**整体提前 3 帧**，f6303 起两腿重新同值。
期间 `HealthLo/Hi=00`、`isHit=00`、`Invincible=00`、`animIndex/animFrame` 亦随之错拍
（f6299–f6302 rust `animIndex=0A/animFrame=22`，emu 仍是 `05/07`）。

**处置**：本节结论以「`SamusStatus` 的 `18→FF` 提前 3 帧」为准；PPU 面全同是**真**的
（CHR/NT/palette 逐字节），f6300 的 635px 是该状态错拍的下游。
下一轮入口：查 `$0300` 写 `FF` 的写点（死亡/爆炸链）为何提前 3 帧——先看 f6256 那次
`17→18` 的 3 帧领先是否同源（同一种「提前 3 帧」形态在 40 帧内出现两次，指向一个**周期性**
的相位差而非一次性事件）。

### 10.6 首分歧其实在 f3268（不在 f6300）——前一轮的二分窗口漏了区间

前面「c6240–c6290 全 0px、c6300=635px」只覆盖了 6000–6600 这一段，**不足以**定位首分歧。
补跑 `f3900–4210`（每 10 帧一采）发现 32 个采点**全部失配**（~10 800 px），再向回二分：

| 区间 | 结果 |
|---|---|
| 3020–3260（每 20 帧） | **全 0 px** |
| **3268** | **288 px**（首个非零） |
| 3280 | 12 657 px（全屏） |
| 3440–3680 | 回落到 180–600 px 量级 |
| 3700–3900 | 再次 10 000+ px |
| **5980 / 6000 / 6020 / 6040** | **全 0 px**（重新合流） |

逐帧（3262–3282）定位：**f3268 = 288 px 是首个分歧**，bbox `x118-137 / y171-208`（Samus 精灵块），
随后每帧增长 3–130 px，f3279 起扩到全屏 `0-255/0-239`。

⇒ f6300 的 635 px 与 f2249 的 Skree 伤害径都**不是**这一段的成因；真正入口是 **f3268**。
f5980–6040 回到 0 px 是因为两腿都死亡重来（`MainRoutine=8` SamusIntro 段幂等），
**不能**被当作「已对齐」的证据。

### 10.7 上游候选：emu 在 f3276/3277 有 lag 帧，rust 没有

逐帧比对 `FrameCount($002D)`：`f3268` 之后两腿开始分叉，`f3275` 仍一致（177/177），
**`f3276`/`f3277` emu 冻结在 177（2 帧不 inc），rust 继续 178→179**，
随后 emu 恢复（f3278=178、f3281=181）。

- emu 同时段 `SamusScrX` 也停在 112（rust 继续递减到底后也到 112）⇒ **lag 帧期间整个主循环没推进**。
- 这与 `MainLoop` 尾部「`inc FrameCount` → `sta NMIStatus(0)` → 自旋等 NMI」的结构一致：
  若某帧工作量 > 1 帧，下一次 NMI 到来时上一帧还没收尾，`inc FrameCount` 就被跳过。
- rust 侧只有 `iter_bogged`/`iter_wait`/`iter_gates` 三个**拟合**门，且 `LOAD_BASE_CYCLES`
  只覆盖「载房」；**滚动/列流这一路的每帧工作量没有进入模型** ⇒ 这类 lag 帧结构上无法产生。

**注意（反例留档）**：用同一份 `frames.jsonl` 做「FC delta 不等」扫描得到 15 个帧
（3275/3276、3792/3793、4181、4183-4186、4345-4349、5849），但其中 f4181 之后两腿已处于
**不同生命**（emu `MainRoutine=4`、rust=3），那些行的字段对比**没有意义**，
不能作为「15 个 lag 帧」的结论。只有 f3275/3276 这一对处在「像素刚开始分叉」的干净窗口里。

### 10.8 下一步（入口已明确）

1. 取 f3276 前后 emu 的**工作量证据**：该帧在做什么（`ScrollDir=1` 向左滚、`SamusScrX` 近边界）；
   若是列流装载（`CheckUpdateNameTable` 32 字节列写入 × N），把这条工作量接进 cycle 模型。
2. 判据：模型要让 f3276/3277 产生「2 帧冻结」，且不动其他已对齐窗口
   （`LOAD_BASE_CYCLES` 的六点锚测是现成的回归网）。
3. 若工作量证据不足，则退一步只建模「滚动边界帧」，并在 findings 里标明这是**拟合**而非机制推导。

## 11. 第三轮：2P 仪器缺陷 + 空中开火弹道（2026-11-23）

### 11.1 §10.7 的 lag 假设撤回（仪器缺陷，非引擎差）

§10.7 依据「emu 有 FC 不前推的帧」推测 lag 模型。该推测**被推翻**：

- Mesen2 腿的 2P 按键被 OR 进端口 1。受控实验（`p1=0` + `p2=0x08`）实测 emu
  `Joy1Status=$14=0x08`、`Joy2Status=$15=0x00`；`p1=0x02 & p2=0x08` 实测 `$14=0x0A`、
  `$15=0x00`。三种写法（`setInput(btn2,1)` / `(btn2,2)` / `(btn2,1,0)`）与
  `settings.json` 的 `Nes.Port2.Type` 改动**全部无效** ⇒ 该构建的 Lua 端口参数不可用。
- 因此 6725M 影片 f3265 起的 242 帧 2P 输入污染了裁定腿：emu 收到 `0x8A`（P1|P2）、rust 收到 `0x82`。
- 处置（`c363480d`）：裁定腿**不再喂端口 2**（WARN 一次；用例声明 `require_joy2` 时 fail-loud）。
  ROM 对 `Joy2Status` 的唯一消费点是 PauseMode 的 2P `A+UP` 退出，本影片那 242 帧只按 `Up`，
  真机行为等同「2P 不按」⇒ 不影响保真。
- 修后 f3268 的伪分歧（288px）归零，`§10.7` 的「lag 帧」结论作废。
  详见 `docs/findings/mesen-2p-input-unsupported.md`。

### 11.2 2P 干净后的真首分歧：f1785→1790

二分（每 40 帧 → 每 5 帧）：`1000–1785` 全 0px，**1785=0 / 1790=64 / 1795=64 / 1800=102**。

### 11.3 定谳：`SamusJump_CheckFire@notAimingUp` 调错了入口

asm（`refs/m1disasm/SRC/prg7_engine.asm`）：

- `FireSamusProjectile ; 07:D1EE` = **纯分派器**：`lda Joy1Status / and #BUTTON_UP / beq
  FireSamusProjectileForwards / jmp FireSamusProjectileUpwards`。
- `SamusJump_CheckFire ; 07:D09C`（空中开火）：`lda Joy1Change / ora Joy1Retrig / asl / bpl rts`
  → `lda Samus.animResetIndex / cmp #ObjAnim_SamusJumpPntUp - ObjectAnimIndexTbl.b`；
  相等 ⇒ `jmp FireSamusProjectileUpwards`；**不等 ⇒ `@notAimingUp: jsr
  FireSamusProjectileForwards`（$D0AD）——直接进 Forwards，不经过 UP 判**。

rust 旧形把 Forwards 的实体写在分派器 `fire_samus_projectile` 内部，而 `samus_jump_check_fire`
的 else 分支调的是**分派器** ⇒ 空中按住 UP 射击时被分派器判成 Upwards。

逐帧证据（探针 `cases/case-tmp-anyp-fire.txt`，输入 `movies/eien86-6725M.inputlog`）：

| 帧 | 两腿共同字段 | emu 弹 | rust 弹 |
|---|---|---|---|
| 1783 | `$14=0x0A`、`status=01`、`animReset=0x0C`(SamusJump) | 槽 D status=0 | 同 |
| 1784 | `$14=0x49`(Right+Up+B)、`animReset=0x20`(SamusJumpFire) | `spd=(00,04)` y=AF x=11 | `spd=(FC,00)` y=A5 x=0A |

位置反推：emu 弹 = f1783 Samus(B5,09) −6 /+8（Forwards 的 arm cannon 偏移）；
rust 弹 = B5−0x10 / 09+1（Upwards 偏移）。`animReset=0x20` 只有 `@notAimingUp` 会写 ⇒
emu 走 Forwards 分支确定。

### 11.4 修复与验证

- `crates/game-core/src/samus/handler.rs`：`fire_samus_projectile` 还原为纯分派器；新增
  `fire_samus_projectile_forwards`（原 Forwards 实体）；`samus_jump_check_fire` 的 else 分支改调
  `fire_samus_projectile_forwards`。
- 锚测 `jump_fire_ignores_up_button`（`JOY1_STATUS=0x48` UP+B、`ANIM_RESET_INDEX=0x0C`
  ⇒ 弹 `speedX=+4`、`speedY=0`）；红证：else 分支改回分派器 ⇒ FAILED。
- 效果：探针 1780/1784/1785/1786/1790 **5/5 全 0px**（修前 1790=64px）；
  4000 帧裁定腿 `1785=0 · 1790=0`（修前 1790=64）。
- 回归：`cargo test --workspace` **1085 passed / 0 failed**（基线 1084→1085）；
  L4 矩阵 **21 案/172 格 match 166 · diff 6 · blocked 0**（逐格同基线）；`gates check` PASS=23。

### 11.5 残余（本轮未收口）

- 探针 1785–1800 逐帧：**1792/1796/1800 各 70px**（周期 4 帧），bbox 恒 `(135,188)-(150,196)`，
  差分像素**颜色多重集两侧完全一致**（精灵整体位移/换格，不是画错）。
  该窗口内 `$0300-$0316`（Samus 全字段）、`$0200` OAM 影子 0-29 项、`ppu.hex` 的
  chr/NT/palette **两腿全等** ⇒ 与已挂账的「显示集/OAM DMA 时刻」族同源，待单独定谳。
- 裁定腿其余分歧：`2000=120`、`3900=433`、`4000=8611`（在 1792 之后，属级联/另一机制）。

### 11.6 1792/1796/1800 的 70px：Samus 位件「翻转位」配到了镜像的另一半（新线索，未修）

进一步取证（同一探针产物）：

- 差分像素 bbox 恒 `(135,188)-(150,196)`，颜色多重集两侧完全一致 ⇒ 不是画错格，是**同一块腿的部位**
  在两腿取到了不同的翻转/优先级组合。
- `$0200` OAM 影子 0-29 项两腿逐字节相同（差异只在未使用的 30-63 槽）；`ppu.hex` 的
  chr / NT / palette 也全等 ⇒ 排除数据面。
- **权威面是渲染 OAM**：rust 用 `--trace oam` 打出的渲染列表与 `$0200` 影子本身就不同
  （影子非渲染输入），所以「影子相同」不是证据。按渲染面比：

  | 帧 | emu `$0200`（= 渲染输入） | rust 渲染 OAM |
  |---|---|---|
  | 1791 | 9:`09 x=87 a=20` · 10:`09 x=8F a=60` | 9:`09 x=8F a=20` · 10:`09 x=87 a=60` |
  | 1792 | 10:`09 x=87 a=20` · 11:`09 x=8F a=60` | 10:`09 x=8F a=20` · 11:`09 x=87 a=60` |

- 即：**左半（x=87）该无翻转（a=20）、右半（x=8F）该带 HFLIP（a=60），rust 把两者对调**。
  attr 的 `0x20` 是 `GetSpriteCntrlData ; 07:DCC3` 恒置的 PRIORITY 位，`0x40` 是该位件自身的
  HFLIP —— 所以「翻转位配错半边」只在**图案非对称的位件 + 与 BG 重叠**处才可见，
  正好解释「周期 4 帧（换格时腿形才非对称）、70px、其余帧 0px」。
- 下一步：读 `crates/game-core/src/render/oam.rs` 的位件走表段（`:307 flip_bit` 与 place 表遍历）
  与 asm `ObjDrawFrame ; 07:DE4A` 的位件序，定谳是「左右半边顺序反」还是「翻转位取自错误表项」。
