# 掉落物规格：生成链、运动与拾取结算

状态：取证中（#144）
范围：敌人死亡后的掉落物（导弹 / 小能量 / 大能量）——生成、每帧行为、拾取判定、结算数值、Tourian 例外。
不在本文件：Chozo 球/能力道具（`ScanForItems`/`SpawnPowerUp`/`UpdateAllPowerUps` 那条链，见 §7）、HUD 显示、音乐引擎（见 `docs/spec/audio-engine.md`）。

证据级别：A＝ROM 字节/asm 原文可直接复算；B＝由调用序或相邻代码推断；C＝假设，只进 §8。

---

## 0. 结论摘要

1. **不新建实体**：掉落物复用**被击杀敌人的槽**（`PageIndex` 不变）。写出的是 `EnsExtra.0.status=5(Pickup)`、`Ens.0.data0D=$60`、`EnsExtra.0.animFrame=PickupTypeTable[r&7]`，`EnsExtra.0.type` 被 `LoadTableAt977B & $20` 覆盖为 `0/$20`；坐标（y/x/hi）、`data05`、`radiusX/Y`、`specialAttribs` 全部沿用死亡时的值。
2. **没有运动**：不掉落、不漂浮、不被地形吸收。`UpdateEnemy_Pickup ; 07:F483` 的非接触尾固定跳 `UpdateEnemyCommon@noMoveNoAnim ; 07:F416`，该段只做「tough 调色板 → `DrawEnemy` → 清 `isHit/weaponAction`」，**不含**速度、重力或碰撞位移。唯一随时间变化的是调色板闪烁与 384 帧消失计时。
3. **不能被打掉**：敌×弹碰撞环对 `status==Pickup` 整段跳过（`07:F09D` 后 `cmp #Pickup / beq`）。拾取物生效的是 Samus 的**接触位**（`Ens.isHit` bit2），不是武器位。
4. **结算有 1 帧延迟（B）**：`UpdateWorld ; 07:CB29` 里 `UpdateAllEnemies` 在前、`CollisionDetection` 在后。第 N 帧碰撞段置接触位，第 N+1 帧的 Pickup AI 才结算。
5. **数值**：导弹 `+2`（metroid 掉落 `+$1E`，即 +30），按 `MaxMissiles` 钳位；能量 大 `+$0200`（BCD，+20）、小 `+$0050`（+5）、metroid `+$0300`（+30）。**没有冻结帧**，只发一条 SQ1 拾取音效。
6. **Tourian 例外**：Tourian 里 `type==2/4`（rinka 一类）**永不掉**；配额失败时在 Tourian 会把 `$93-$96` 重置成 `0/1` 并用**同一个** `RandomNumber1` 重试（原版 bug 面，C 级，见 §8）。

---

## 1. 生成链（`EnemyCompleteDeath ; 07:DCF5`）

### 1.1 入口与前提

`EnemyCompleteDeath` 由各区敌人的死亡动画走完后进入（`prg7_engine.asm` 内 `jsr EnemyCompleteDeath`，例如爆散帧结束）；进入时 `EnsExtra.0.status` 仍是 `Explode(3)`、`prevStatus` 带被击杀前状态。

### 1.2 逐跳

| # | asm | 行 | 作用 |
|---|---|---|---|
| 1 | `jsr ClearObjectCntrl` | `07:DCF5` | 清 `ObjectCntrl`，免得死亡动画残留调色板/镜像位 |
| 2 | `pla / pla` | `07:DCF8` | 丢弃 AI 调用帧，回 `UpdateAllEnemies` |
| 3 | `lda InArea / cmp #$13` | `07:DCFC-DD01` | Tourian 时先查 `type` |
| 4 | `lda EnsExtra.0.type,x / cmp #$04 / beq Lx139`<br>`cmp #$02 / beq Lx139` | `07:DD02-DD0C` | Tourian：type `$04/$02`（rinka 等）**直接 RemoveEnemy**，不掉 |
| 5 | `lda Ens.0.prevStatus,x / asl / bmi LDD75` | `07:DD0D-DD12` | `prevStatus` bit6＝小 Boss 位：切雕像/弹药支路，**不转掉落** |
| 6 | `jsr ReadTableAt968B / sta Temp00` | `07:DD13-DD17` | 记 `L968B[r16]` 到大能量资格位（§1.4） |
| 7 | `jsr LoadTableAt977B / and #$20 / sta EnsExtra.0.type,x` | `07:DD18-DD1F` | **覆盖 type** 为 `0` 或 `$20`（metroid 类） |
| 8 | `lda #enemyStatus_Pickup / sta EnsExtra.0.status,x` | `07:DD20-DD24` | 进 Pickup 态 |
| 9 | `lda #$60 / sta Ens.0.data0D,x` | `07:DD25-DD29` | 消失计时 `$60`，按 `FrameCount&3` 递减 ⇒ 384 帧 |
| 10 | `lda RandomNumber1 / cmp #$10 / bcc LDD5B` | `07:DD2A-DD2F` | `RandomNumber1 < $10` ⇒ 1/16 无掉落 |
| 11 | `and #$07 / tay / lda PickupTypeTable,y / sta animFrame` | `07:DD30-DD3C` | 按 `r&7` 选掉落型（表见 §1.3） |
| 12 | 导弹支路：`MissilePickupQtyMax vs Cur`、`MaxMissiles!=0`、`inc Cur` | `07:DD3D-DD4A` | 配额与容量门 |
| 13 | 能量支路：`EnergyPickupQtyMax vs Cur`、`inc Cur`、非大能量直接 `rts` | `07:DD4B-DD59` | 大能量还要过 §1.4 资格位 |
| 14 | `rts`（成功） | `07:DD4A`/`07:DD5A` | 掉落在本帧已成型，等下一帧 Pickup AI |

`RandomNumber1 = $2E`（`constants_ram.asm:522`）。

### 1.3 掉落型表

```text
PickupTypeTable ; 07:DE35（8 项，按 r&7 取）
00: 80  导弹      (_id_EnFrame_MissilePickup)
01: 81  小能量    (_id_EnFrame_SmallEnergyPickup)
02: 89  大能量    (_id_EnFrame_BigEnergyPickup)
03: 80
04: 81
05: 89
06: 81
07: 89
```

`animFrame` 既是 sprite 帧 id，也是结算分派键（§5）。

### 1.4 大能量资格（`L968B`）

`ReadTableAt968B` 读 `L968B[r16]`（`r16 = RandomNumber1>>4`？见 §8 C2），存 `Temp00`；能量支路末尾 `lsr Temp00 / bcs rts`——**carry=1 表示该敌不掉大能量**，于是大能量掉落到此被拒（`rts`，但 `animFrame` 已经是 `$89`；因未 `RemoveEnemy`，本帧就停在 Pickup 态、可见为静止大能量）。

> 注意：这与「1/16 无掉」不同——后者直接 `RemoveEnemy`；资格失败只挡「大能量」这一档，`EnergyPickupQtyCur` 已在此之前 `inc`（§1.2 第 13 步），所以计数会多记一个。

### 1.5 何时可见

`DrawEnemy ; 07:DD8B`：`animFrame == _id_FrameNone` ⇒ `ClearObjectCntrl` 不画；否则按 y/x/hi + `ObjectCntrl` 画。掉落物 `animFrame` 已在生成时写好，**只要本帧走到 `DrawEnemy` 就可见**；死亡当帧是否走到由死亡动画的收尾决定（C1）。

---

## 2. 每帧行为（`UpdateEnemy_Pickup ; 07:F483`）

### 2.1 判定顺序

```text
UpdateEnemy_Pickup:
    lda Ens.0.isHit,x
    and #$24
    beq @pickupWasNotTouched    ; 没碰到 → §2.2 计时/闪烁
    jsr RemoveEnemy             ; 碰到 → 先释放槽
    ldy EnsExtra.0.animFrame,x  ; 再从 EnsExtra 读掉落型
    ...结算（§5）
@pickupWasNotTouched:
    lda FrameCount / and #$03 / bne @endIf_D
        dec Ens.0.data0D,x / bne @endIf_D
            ; 到 0 ⇒ RemoveEnemy（超时消失）
@endIf_D:
    lda FrameCount / and #$02 / lsr
    ora #$80 | OAMDATA_PRIORITY
    sta ObjectCntrl             ; 闪烁：$80/$A1 交替
    jmp UpdateEnemyCommon@noMoveNoAnim
```

`isHit` 位定义（`constants_ram.asm:90-95`）：bit2＝touching Samus、bit5＝被 Samus 武器命中；`$24` 同时接受两者。对掉落物而言 bit5 不可能在本帧被置（§4.1 跳过），实际只有 bit2。

### 2.2 运动

**没有运动代码**。`@pickupWasNotTouched` 只做两件事：计时递减、闪烁，然后跳 `@noMoveNoAnim`；`@noMoveNoAnim`（`07:F416`）不含 `EnemyMove`/重力/地形判定。因此：

- 掉落物停在敌人死亡时的 (y,x,hi)，包括空中与墙内；
- 「被地形吸收」不存在；
- 上下漂浮也不存在，视觉上的“闪”只是调色板交替。

### 2.3 计时与闪烁

- 计时：`data0D` 每 4 帧减 1（`FrameCount&3==0`），`$60` ⇒ 最多 384 帧后消失。
- 闪烁：`ObjectCntrl = ((FrameCount&2)>>1) | $80 | $20`，两帧一换，在 `$80`（调色板 0）与 `$A1`（调色板 1 + 优先级）间交替。

---

## 3. 拾取判定（接触）

### 3.1 碰撞面

`CollisionDetection ; 07:F034`（`UpdateWorld` 内 `jsr CollisionDetection`，`prg7_engine.asm:2512`）：

1. 敌×弹环从 `x=$50` 起、`jsr Xminus16` 到槽 0（`07:F09D` 附近），**先判 `status==Pickup` 就跳过该敌**（`cmp #Pickup / beq`），所以子弹/导弹/波束都不能“打掉”掉落物；
2. Samus 接触单独走 `CollisionDetectionEnemy_CheckWithObjectYSlot`（半径和 `07:F1BF`、判盒 `07:F1FA`、命中反应 `ReactToCollisionWithSamus ; 07:F282`）；
3. `SetEnemyTouchingSamusFlags ; 07:F2E8` 把接触方向位写进 `Ens.isHit`；
4. `ReactToCollisionWithSamus` 对 `status >= Frozen(4)` **直接 rts**（拾取物/Frozen 不伤害 Samus）。

### 3.2 时序（B）

`UpdateWorld ; 07:CB29` 的顺序是 `UpdateAllEnemies`（内含 Pickup AI）→ … → `CollisionDetection`。故：

```text
帧 N   : 碰撞段置 isHit.bit2
帧 N+1 : UpdateEnemy_Pickup 读到 bit2 → 结算
```

实现时不能在同一帧“碰到立即加血”，否则比原版早 1 帧。

### 3.3 触发条件汇总

| 条件 | 来源 |
|---|---|
| Samus 与掉落物判盒重叠（半径和 + 16 位判盒） | `07:F1BF` / `07:F1FA`（后者见 `docs/spec/collision-vectors.md`） |
| `status==Pickup` 不参与敌×弹环 | `07:F09D` |
| 接触位进入 `isHit` bit2 | `07:F2E8` |
| 结算门 `isHit & $24` | `07:F48C-07:F493` |

---

## 4. 结算（`07:F48C-07:F4D1`）

### 4.1 导弹（`animFrame==$80`）

```text
RemoveEnemy                       ; 先释放槽
ldy #$02                          ; +2
lda EnsExtra.0.type,x / beq +     ; type!=0（metroid）⇒ +$1E
    ldy #$1E
clc / tya / adc MissileCount
bcc @noCap / lda MaxMissiles      ; 溢出 ⇒ 直接取 Max
@noCap:
cmp MaxMissiles / bcc @store
    lda MaxMissiles                ; 超过上限 ⇒ 钳到 Max
@store:
sta MissileCount
jmp SFX_MissilePickup
```

- 数值：普通 `+2`、metroid `+30`；
- 钳位：`sum > 255` 或 `sum >= MaxMissiles` ⇒ `MaxMissiles`；
- **不动 MaxMissiles**（那是导弹罐/小 Boss 奖励的事）。

### 4.2 能量（`animFrame!=$80`）

```text
ldy #$00 / ldx #$03               ; 默认 x=3,y=0 ⇒ $0300（+30，metroid）
lda type / bne @useDefault          ; type!=0 ⇒ 保持 $0300
    dex                             ; x=2 ⇒ $0200（+20，大能量）
    cmp #SmallEnergyPickup / bne @useDefault
        ldx #$00 / ldy #$50         ; x=0,y=$50 ⇒ $0050（+5，小能量）
@useDefault:
sty HealthChange / stx HealthChange+1
jsr AddHealth                     ; 07:CEF9
jmp SFX_EnergyPickup
```

- `HealthChange` 是 16 位 **BCD**：低字节 Y、高字节 X；
- 三档：小能量 `$0050`（+5）、大能量 `$0200`（+20）、metroid `$0300`（+30）；
- `AddHealth` 负责满血上限与音效外的 HUD 影响（见 `docs/spec/items-equipment.md`）。

### 4.3 音效（无冻结帧）

| 分支 | asm | 动作 |
|---|---|---|
| 导弹 | `SFX_MissilePickup ; 07:CBC0` | `lda #sfxSQ1_MissilePickup(%10000000)` → `SetSFXSQ1InitFlag ; 07:CBC2` ⇒ `$0681 \|= $80` |
| 能量 | `SFX_EnergyPickup ; 07:CBBC` | `lda #sfxSQ1_EnergyPickup(%01000000)` → 同上 ⇒ `$0681 \|= $40` |

`SetSFXSQ1InitFlag` 是 `ldx #SFXSQ1InitFlags-SFXNoiseInitFlags` 后跳 `SetSoundInitFlag`（`07:CBE8`），不阻塞、不减帧。

---

## 5. Tourian 例外（`LDD5B ; 07:DD5B`）

```text
LDD5B:
    ldx PageIndex
    lda InArea / cmp #$13 / beq Lx140
        jmp RemoveEnemy               ; 非 Tourian
Lx140:                                ; Tourian
    lda RandomNumber1                 ; 用同一个随机数
    ldy #$00 / sty EnergyPickupQtyCur / sty MissilePickupQtyCur
    iny / sty MissilePickupQtyMax / sty EnergyPickupQtyMax   ; max=1
    bne LDD30                         ; 回掉落型分派重试
```

要点：

1. 触发条件是**任意**掉落失败（1/16 无掉、导弹容量 0、配额满、大能量资格失败）；
2. 重试**不重新取随机数** ⇒ 掉落型不变；把配额清成 `0/1` 后一般能过配额门，但导弹容量 0 / 大能量资格失败仍会再次落 `LDD5B` ⇒ **潜在无限重试**（C3）；
3. 原版注释已自认此逻辑有 bug：跳小 Boss 的 NARPASSWORD 下假设不成立（`07:DD66-07:DD6E` 注释）。

---

## 6. 与 PowerUp（Chozo 球）链的区别

| 维度 | 敌人掉落（本文） | 特项/PowerUp（`crates/game-core/src/pickup/spawn.rs`） |
|---|---|---|
| 载体 | 复用敌人槽（`Ens`+`EnsExtra`） | `PowerUps` 2 槽×8B @`$0748` + `PowerUpDraw` 对象 @`$0340` |
| 生成 | `EnemyCompleteDeath 07:DCF5` | `ScanForItems 07:ED98`→`SpawnPowerUp 07:EDFE` |
| 数据源 | `PickupTypeTable 07:DE35` + `L968B`/`L977B` | `SpecItmsTbl`（`game_data::area::items`） |
| 判定 | `Ens.isHit & $24`（接触位） | `AreObjectsTouching 07:DC7F`（屏距径） |
| 结算 | 改 `MissileCount` / `HealthChange`+`AddHealth` | `SamusGear`、能量罐/弹药、`UniqueItemHistory` |
| 记录 | 无 | `CreateItemID 07:DC67`+`CheckForItem 07:EE4A` |

两者都叫“拾取”，但**没有共用例程**；规格与实现不要互相套用。

---

## 7. Rust 现状对照

### 7.1 已实现

`crates/game-core/src/enemy/framework.rs`：

- `enemy_complete_death`（`:563-626`）：1/16 无掉、`status←Pickup`、`data0D←$60`、`type` 覆盖为 `0/$20`、`animFrame←PickupTypeTable[r&7]`；
- `update_enemy_pickup`（`:461-540`）：`isHit & $24`、先 `RemoveEnemy` 再结算、导弹 `+2/+$1E` 与钳位、能量 `$0050/$0200/$0300`、`add_health`、`data0D` 每 4 帧递减、`ObjectCntrl` 闪烁。

### 7.2 缺（本条卡面要求标出）

| 缺项 | asm 依据 | 建议落点（#143 在办） |
|---|---|---|
| Tourian `type==2/4` 不掉的过滤 | `07:DD05-07:DD13` | `enemy_complete_death` 入口 |
| 导弹配额门（`MissilePickupQtyMax/Cur`、`MaxMissiles!=0`） | `07:DD41-07:DD4A` | 同上，`$93/$95` 读写 |
| 能量配额门（`EnergyPickupQtyMax/Cur`） | `07:DD4B-07:DD58` | 同上，`$94/$96` |
| 大能量资格（`L968B` 的 `lsr/bcs`） | `07:DD58` | 同上，`rom_table_968b` |
| Tourian 配额重置与同 RNG 重试 | `07:DD5B-07:DD72` | 同上 |
| 两条拾取音效 | `07:CBBC/CBC0` | `update_enemy_pickup` 结算出口；`or_sfx_sq1_init(0x80/0x40)` |

> 生成门的完整规格与最小变更见 `docs/spec/f1551-fix-plan.md`（#142，已验收）；本文只负责字段与行为口径，不重复伪代码。

---

## 8. 假设（C 级，只进假设区）

- **C1**：掉落物在“死亡当帧”是否已可见。`EnemyCompleteDeath` 本身不调用 `DrawEnemy`；若死亡动画所在 AI 在返回前没有再走一次 `UpdateEnemyCommon`，则首次可见推迟到下一帧 Pickup AI。验证：Mesen 在 `$07:DD8B`（`DrawEnemy`）入口记 `PageIndex+animFrame`，对同一只敌人在死亡帧与下一帧各采一次；或比对 f1550/f1551 的 OAM 槽字节。
- **C2**：`ReadTableAt968B` 的索引确切取法（`RandomNumber1>>4` 还是别的位段）未逐字节核；但 `lsr/bcs` 判的是返回值 bit0，只影响“哪种种子的敌人不掉大能量”，不影响数量级。验证：对 `RandomNumber1=$00..$FF` 逐值跑 `ReadTableAt968B` 打印 `L968B[r16]`，与 §1.4 的 `bcs` 结果对照。
- **C3**：Tourian 重试可能无限循环（同 RNG + 容量 0 / 资格失败）；原版是否真的卡死未实测。验证：在 Tourian 放一个 `MaxMissiles=0` 的存档、杀敌后观察引擎是否停摆（`FrameCount` 是否还走）。

---

## 9. 复跑命令

所有地址按 bank 映射取值：bank0–6 `16 + bank*0x4000 + (addr-0x8000)`，bank7 `16 + 7*0x4000 + (addr-0xC000)`。

```bash
python3 - <<'PY'
rom = open('rom/Metroid (USA).nes','rb').read()
def at(bank, addr):
    base = 16 + (bank*0x4000 if bank < 7 else 7*0x4000)
    start = 0x8000 if bank < 7 else 0xC000
    return rom[base + addr - start]
# PickupTypeTable 07:DE35
print('PickupTypeTable', [hex(at(7,0xDE35+i)) for i in range(8)])
# 结算三档 HealthChange 字面量：07:F49D-F4B4 的 ldx/ldy 立即数
print('LDX03', hex(at(7,0xF49D)), 'LDX00', hex(at(7,0xF4A8)), 'LDY50', hex(at(7,0xF4AA)))
# SFX 值
print('SFX_Energy', hex(at(7,0xCBBD)))  # lda #$40（操作数）
print('SFX_Missile', hex(at(7,0xCBC1))) # lda #$80
PY
```

期望：`PickupTypeTable ['0x80','0x81','0x89','0x80','0x81','0x89','0x81','0x89']`；`LDX03 0x03 / LDX00 0x00 / LDY50 0x50`；`SFX_Energy 0x40 / SFX_Missile 0x80`。

第二组：掉落门与结算分支的指令字节（证明顺序：先 RemoveEnemy，再读 animFrame/type）：

```bash
python3 - <<'PY'
rom = open('rom/Metroid (USA).nes','rb').read()
def dump(bank, addr, n, label):
    base = 16 + (bank*0x4000 if bank < 7 else 7*0x4000)
    start = 0x8000 if bank < 7 else 0xC000
    b = rom[base+addr-start: base+addr-start+n]
    print(label, b.hex(' '))
dump(7, 0xF483, 16, 'UpdateEnemy_Pickup')
dump(7, 0xF490, 12, 'RemoveEnemy_first')
dump(7, 0xDD30, 16, 'LDD30')
dump(7, 0xDD5B, 24, 'LDD5B')
PY
```

期望首段以 `bd 04 04 29 24 f0 4a` 开头（`lda Ens.0.isHit,x / and #$24 / beq`）；`LDD30` 段以 `29 07 a8 b9 35 de` 开头；`LDD5B` 段以 `a6 4b a5 74 c9 13 f0 03` 开头（`ldx PageIndex / lda InArea / cmp #$13 / beq`）。

---

## 10. 未覆盖 / 下游

- HUD 的导弹/血量显示更新时机（`DisplayHUD`）不在本文；
- 掉落物与 `ItemRoomMusicStatus`、`PowerUpMusic` 的关系（掉落不加音乐、不加延迟旗）只在此注明，不展开；
- 冻结帧/屏幕震动：**无**（本节 4.3 已核）；
- `L968B` 表的 128 项语义见 `docs/findings/enemy-kill-drop-path.md` 与 `docs/spec/f1551-fix-plan.md`。
