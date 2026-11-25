# 敌人公共更新：`UpdateEnemyCommon` / `noMove` 五 bank 同址与状态机规格

> B3 前置静态规格（本卡只写这一个文件）。数据源：`rom/Metroid (USA).nes`（字节，A 级）+ `refs/m1disasm/SRC/*.asm`（结构/命名，线索）+ `reference/seed/metroid.asm`（机器地址锚点）。
> 本卡不跑 Mesen2：所有「ROM 会怎样」都是指令级推导；「实机是否会走到」标未取证。

## 0. 结论速览（先看这张表）

| 问题 | 结论 | 级别 |
|---|---|---|
| `UpdateEnemyCommon` 有几份实现？ | **1 份，只在 bank7 `07:F410`–`07:F43D`（0x2E=46 字节）**，3 个入口：`$F410` 全件、`$F416` noMoveNoAnim、`$F438` noMove | A（字节 + asm 标签） |
| bank01–05 有没有各自的实现？ | **没有**。每 bank 在 `$8000/$8003/$8006` 各有 9 字节跳板 `4C 10 F4 / 4C 38 F4 / 4C 16 F4`（→`$F410/$F438/$F416`），5 份**逐字节相同**（sha256 前 16：`c0af1d4dcd86fc34`） | A |
| 那「一份 Rust 实现覆盖 5 bank」成立吗？ | 成立，但依据是「5 份同字节跳板 + bank7 唯一实现」；**不是** 5 份实现。移植时只写一份，入口对应三个跳板语义 | A |
| `EnemyMove` 在哪？ | shared 段 `bank01–05:8058`（`$8058`–`$80AF`，0x58 字节），bank7 **没有**这份代码；bank7 `$F411` 的 `jsr $8058` 是**跨 bank 调用**，依赖当前区域 bank 常驻 `$8000` | A |
| bank01–05 的 shared 段是否完全相同？ | `$8000`–`$8D5F` **0 字节差**；从 `$8D60` 起才按区域不同（门表/敌人 AI 等） | A |
| 五个区域的「入口选择器」是否相同？ | `UpdateEnemyCommon_Decide` 5 份，各 **23 字节**（`a581…4c0680`；前 22 字节 sha256[:16]=`5907c96a4763a3de`，含末跳 23 字节=`914cf5a391dbf62e`），**逐字节相同**，只是地址不同（01:99A1 / 02:9839 / 03:97E2 / 04:9905 / 05:9830） | A |
| 移植现状 | `crates/game-core/src` 已有三入口 + `EnemyMove` + 4 个失败处理器 + `Decide`（`enemy/framework.rs:647/655/678/690`、`enemy/motion.rs:348`）；#51 把三入口内残余的裸地址读写收到 typed 面，并补 5 例字段级单测（见 §10） | A（代码行号） |

## 1. 五 bank 同址/同字节验证

### 1.1 PRG 偏移与跳板字节（A 级）

偏移公式（同 `porting-backlog.md` §0）：`file_off = 16 + bank*0x4000 + (addr - (addr>=0xC000 ? 0xC000 : 0x8000))`。
`$8000` 在 bank1–5 的 file_off 分别为 `0x4010 / 0x8010 / 0xC010 / 0x10010 / 0x14010`。

| bank | `$8000` 起 9 字节（`jmp $F410 / jmp $F438 / jmp $F416`） | sha256[:16] |
|---|---|---|
| 01 Brinstar | `4c 10 f4 4c 38 f4 4c 16 f4` | `c0af1d4dcd86fc34` |
| 02 Norfair | 同上 | `c0af1d4dcd86fc34` |
| 03 Tourian | 同上 | `c0af1d4dcd86fc34` |
| 04 Kraid | 同上 | `c0af1d4dcd86fc34` |
| 05 Ridley | 同上 | `c0af1d4dcd86fc34` |
| 06 | `$8000` 是图形数据（`00 07 1f 3f 2d 4b 0d 1d 00`），**没有这组跳板** | `ab95ab64858d8f5e` |

asm 对应：`areas_common.asm:21-28`（`CommonJump_UpdateEnemyCommon` / `_noMove` / `_noMoveNoAnim`），该文件被 bank1–6 包含；bank6 的 `$8000` 段已被图形数据占用，所以 bank6 不提供这组跳板（**B 级：本卡没有追谁会在 bank6 里调用它**）。

### 1.2 shared 段相同范围（A 级）

对 `$8000` 起 0x2000 字节逐 bank 与 bank1 比较：

| bank | 差字节数 | 第一个差异偏移（相对 `$8000`） |
|---|---|---|
| 02 | 4221 | `0x0D60`（`$8D60`） |
| 03 | 4183 | `0x0D60` |
| 04 | 3941 | `0x0D64` |
| 05 | 4156 | `0x0D60` |

⇒ bank1–5 的 shared 段前 **0x0D60 字节完全一致**，`UpdateEnemyCommon`/`EnemyMove`/失败处理器/`LoadTableAt977B`/`EnemyMovementPtrs` 全在这段之内，所以「一份实现」的前提在这些函数上成立。`$8D60` 之后是各区域自己的表（doors 等），不属本卡。

### 1.3 `UpdateEnemyCommon` 本体（A 级，`07:F410`–`07:F43D`；asm `prg7_engine.asm:10802`/`:10806`/`:10834`）

```
07:F410  20 94 E0   JSR $E094      ; UpdateEnemyAnim（A=新 anim 延时）
07:F413  20 58 80   JSR $8058      ; EnemyMove（在区域 bank！）
07:F416  A6 4B      LDX PageIndex
07:F418  BD 0F 04   LDA Ens.0.specialAttribs,X
07:F41B  10 08      BPL @endIf_A
07:F41D  A5 6B      LDA ObjectCntrl
07:F41F  30 04      BMI @endIf_A
07:F421  A9 A3      LDA #$80|$03|$20
07:F423  85 6B      STA ObjectCntrl
07:F425  BD F4 6A   LDA EnsExtra.0.status,X
07:F428  F0 03      BEQ @endIf_B
07:F42A  20 8B DD   JSR DrawEnemy    ; $DD8B
07:F42D  A6 4B      LDX PageIndex
07:F42F  A9 00      LDA #$00
07:F431  9D 04 04   STA Ens.0.isHit,X
07:F434  9D 0E 04   STA Ens.0.weaponAction,X
07:F437  60         RTS
07:F438  20 94 E0   JSR $E094      ; ← @noMove 入口（只在 $F438 起）
07:F43B  4C 16 F4   JMP $F416
```

三个入口的机器字节起点：`$F410`（全件）、`$F416`（noMoveNoAnim）、`$F438`（noMove）。注意 `$F438` 是**独立入口**（先 `jsr $E094` 再跳 `$F416`），不是 `$F416` 的别名。

### 1.4 入口选择器 `UpdateEnemyCommon_Decide`（A 级，5 份同字节）

```
01:99A1  A5 81      LDA EnemyStatusPreAI   ; $81
         C9 01      CMP #enemyStatus_Resting(1)
         F0 09      BEQ @resting
         C9 03      CMP #enemyStatus_Explode(3)
         F0 0A      BEQ @explode
         A5 00      LDA $00                ; 活动态 anim 节拍
         4C 00 80   JMP $8000              ; → $F410 全件
@resting A5 01      LDA $01                ; 休止态 anim 节拍
         4C 03 80   JMP $8003              ; → $F438（anim only）
@explode 4C 06 80   JMP $8006              ; → $F416（anim/move 都不做）
```

地址：01:99A1、02:9839、03:97E2、04:9905、05:9830；每份 **23 字节**（`@explode` 的 `4C 06 80` 三字节完整算入）。前 22 字节 sha256[:16] 同为 `5907c96a4763a3de`；含末跳 23 字节同为 `914cf5a391dbf62e`（两个长度都逐字节相同）。
`$00/$01` 是调用方预先装入的「anim 节拍」（典型值 `#$00` 与 `#$01`，见 `prg1_brinstar.asm:863` 附近）；它们是零页中间量，不是状态。

### 1.3 #51 落地核对表（五 bank 同址逐字节，A 级）

卡面要求「bank 号 + `$C000` 映射地址 + 长度 + sha」。本族是**一份实现 + 五份跳板**，所以表里两侧都列：
跳板在可切换窗口 `$8000`（各 bank 自己的 PRG 段），实现只在固定窗口 `$C000`（bank7）。

| bank | 跳板地址（可切换窗口） | 跳板长度 | 跳板 sha256[:16] | 实现落点（`$C000` 窗口） | 实现长度 | 实现 sha256[:16] | 与 #48 规格 |
|---|---|---|---|---|---|---|---|
| 01 Brinstar | `01:8000` | 9 B | `c0af1d4dcd86fc34` | `07:F410` | 0x2E=46 B | `2ad4f045ae6f9b1d` | 一致 |
| 02 Norfair | `02:8000` | 9 B | `c0af1d4dcd86fc34` | `07:F410` | 46 B | `2ad4f045ae6f9b1d` | 一致 |
| 03 Tourian | `03:8000` | 9 B | `c0af1d4dcd86fc34` | `07:F410` | 46 B | `2ad4f045ae6f9b1d` | 一致 |
| 04 Kraid | `04:8000` | 9 B | `c0af1d4dcd86fc34` | `07:F410` | 46 B | `2ad4f045ae6f9b1d` | 一致 |
| 05 Ridley | `05:8000` | 9 B | `c0af1d4dcd86fc34` | `07:F410` | 46 B | `2ad4f045ae6f9b1d` | 一致 |
| 06 | 无跳板（图形数据） | — | `ab95ab64858d8f5e` | `07:F410`（同实现，不被 bank6 引用） | 46 B | `2ad4f045ae6f9b1d` | 一致（bank6 不提供入口） |

复跑（输出即上表两列 sha）：

```bash
python3 - <<'EOF'
import hashlib
rom=open('rom/Metroid (USA).nes','rb').read()
off=lambda b,a:16+b*0x4000+((a-0xC000) if a>=0xC000 else (a-0x8000))
for b in range(1,6):
    print(f"bank{b} stub9", hashlib.sha256(rom[off(b,0x8000):off(b,0x8000)+9]).hexdigest()[:16])
print("impl46 bank7", hashlib.sha256(rom[off(7,0xF410):off(7,0xF410)+0x2E]).hexdigest()[:16])
EOF
```

「差异处理」：五份跳板 0 字节差、无分支 ⇒ 端口就是**同一份 Rust 实现**，不存在按 bank 分叉的写法。
若将来某 bank 出现差异，本节要求逐字节列出并在此表加列，不得套用「同址」结论。

## 2. 入口与尾件的语义（A 级，逐行）

**3 个主入口 + 2 个子入口**（子入口是同函数体内的标签，可从外部 `jmp` 直入）：

| 入口 | 地址 | 做 anim | 做 move | 尾件（调色板/DrawEnemy/清旗） | 何时被用 |
|---|---|---|---|---|---|
| `UpdateEnemyCommon` | `07:F410` | ✓（`jsr $E094`） | ✓（`jsr $8058`） | ✓ | `Decide` 默认分支（status≠1、≠3） |
| `@noMove` | `07:F438` | ✓ | ✗ | ✓ | `Decide` 的 Resting 分支 |
| `@noMoveNoAnim` | `07:F416` | ✗ | ✗ | ✓ | `Decide` 的 Explode 分支；也被 Frozen 例程 `jmp` 复用 |
| `@setHurtPalette`（子入口） | `07:F423` | — | — | 只做 `sta ObjectCntrl`（`85 6B`），A=调用方给的调色板 | `UpdateEnemy_Hurt` 在 `prg7_engine.asm:11005` 用 `jmp UpdateEnemyCommon@setHurtPalette` 直入，进来时 A=`$80\|$00\|PRIORITY`（调色板0） |
| `@clearIsHitFlags`（子入口） | `07:F42D` | — | — | 从清 `isHit/weaponAction` 起执行到 `rts` | 无外部调用者；给出它存在是为了逐行对齐时不被当成新入口 |

**寄存器/内存约定**
- 入口 X = 槽基址（`$0400 + 16k`），由调用者或 `PageIndex`（`$4B`）给出；`UpdateEnemyCommon` 内部每次 `jsr` 后都重新 `ldx PageIndex`，不假设 X 存活。
- 入口 A = **新的 anim 延时**（透传给 `UpdateEnemyAnim`）；`@noMoveNoAnim` 不读 A。
- 返回：`rts`，A=0（最后一个 `lda #$00`）、X=`PageIndex` 的值、Y 未定义（被 `DrawEnemy` 改过）。
- 读的字段：`Ens.specialAttribs`(+15) bit7、`ObjectCntrl`($6B) bit7、`EnsExtra.status`(+0, `$6AF4+16k`)。
- 写的字段：`ObjectCntrl`（tough 敌人用调色板 3，值 `$80|$03|$20`）、`Ens.isHit`(+4)=0、`Ens.weaponAction`(+14)=0；另有 `DrawEnemy` 内部的写（见 §4.2）。

**尾件三条分支（A 级）**
1. `specialAttribs.bit7=1`（tough）且 `ObjectCntrl.bit7=0` ⇒ `ObjectCntrl = $A3`（调色板3 + 优先级）。bit7 已置则不动（避免每帧重复写）。
2. `status≠0` 才 `jsr DrawEnemy`；`status=0`（空槽）跳过绘制 **但仍清 isHit/weaponAction**。
3. 无论是否绘制，最后都把 `isHit`、`weaponAction` 清 0 —— 这是「每帧重置命中/武器动作」的收口点。

⚠️ 两条路径不要混：`UpdateEnemy_Hurt`（`prg7_engine.asm:11005`）是 `jmp @setHurtPalette`，A=它自己算的调色板值（`$80|$00|PRIORITY`），**跳过上面第 1 条 tough 判断**；只有 `@noMoveNoAnim` 的正常入口才会按 `specialAttribs.bit7` 写 `$A3`。

## 3. animation 分支：`UpdateEnemyAnim`（A 级，`07:E094`）

本体 0x2D=45 字节（`07:E094`–`07:E0C0`）：
```
a6 4b    LDX PageIndex
bc f4 6a LDY EnsExtra.0.status,X
c0 05    CPY #enemyStatus_Pickup(5)
f0 1e    BEQ @RTS
bc f8 6a LDY EnsExtra.0.animDelay,X   ; animDelay=+4 → $6AF8
f0 05    BEQ @writeDelay
de f8 6a DEC EnsExtra.0.animDelay,X   ; 递减写回
d0 14    BNE @RTS                     ; 未到 0 ⇒ 退出
9d f8 6a STA EnsExtra.0.animDelay,X   ; A=新延时（入口 A 透传）
bc fa 6a LDY EnsExtra.0.animIndex,X   ; animIndex=+6 → $6AFA
b1 47    LDA (EnAnimTable),Y          ; $47 = AreaPointers_RAM.EnAnimTable
c9 ff    CMP #_id_FrameEnd($FF)
f0 09    BEQ @resetAnimIndex
9d f7 6a STA EnsExtra.0.animFrame,X   ; animFrame=+3 → $6AF7
c8       INY
98       TYA
9d fa 6a STA EnsExtra.0.animIndex,X
60       RTS
bc f9 6a @resetAnimIndex: LDY EnsExtra.0.resetAnimIndex,X  ; +5 → $6AF9
b0 ec    BCS @getAnimIndex            ; 分支恒真（C 由 CMP 遗留=1? 实为 BCS 恒真写法）
```

（asm 侧同源标注：`prg7_engine.asm:6661 UpdateEnemyAnim`；`BCS` 恒真属于原版「强制跳」写法，移植时按无条件跳处理即可。）

要点：
- **延时递减会写回**（`DEC abs,X` 是读改写，不是 `dey` 寄存器操作）——这一点与部分敌人的手持节拍实现不同，移植时别把 `animDelay` 改成局部变量。
- 新延时 = 入口 A（`Decide` 的 `$00`/`$01` 节拍；某些调用者直接 `lda #n`）。
- `animIndex` → 区域动画表（`$47` 指针）；`FrameEnd=$FF` ⇒ 回绕到 `resetAnimIndex` 再取。
- 只写 `animDelay/animFrame/animIndex`（+4/+3/+6），不动位置/速度。
- **`status==Pickup(5)` 时整件退出**（`BEQ @RTS`），所以被拾取的道具不再推进动画，但尾件仍会 `DrawEnemy`。

## 4. velocity 分支：`EnemyMove`（A 级，`bank01–05:8058`，shared）

### 4.1 门控与主轴（A 级）

本体 0x58=88 字节（`bank1:8058`–`$80AF`；bank2–5 同址同字节，sha256[:16]=`af39c7d9410f6546`）：
```
a6 4b                LDX PageIndex
bd 05 04             LDA Ens.0.data05,X
0a                   ASL                ; bit7 出界，bit6→N
30 4f                BMI @RTS           ; bit6=1 ⇒ 整件跳过
bd f4 6a             LDA EnsExtra.0.status,X
c9 02                CMP #enemyStatus_Active(2)
d0 48                BNE @RTS           ; 只有 Active 移动
20 44 82             JSR $8244          ; EnemyGetDeltaY → Temp00_Delta($00)
a5 00                LDA Temp00_Delta
10 0f                BPL @endIf_Up      ; 正/零 delta 走下行块
20 d4 c3             JSR TwosComplement ; $C3D4 ⇒ |delta|
85 66                STA EnemyMovePixelQty($66)
20 f5 83             @loop_Up: JSR EnemyMoveOnePixelUp   ; $83F5
20 b8 80             JSR EnemyIfMoveFailedUp            ; $80B8
c6 66                DEC EnemyMovePixelQty
d0 f6                BNE @loop_Up
f0 0c                BEQ @endIf_Down    ; qty==0 ⇒ 跳过下行块
85 66                STA EnemyMovePixelQty
20 4b 84             @loop_Down: JSR EnemyMoveOnePixelDown ; $844B
20 fb 80             JSR EnemyIfMoveFailedDown            ; $80FB
c6 66                DEC EnemyMovePixelQty
d0 f6                BNE @loop_Down
20 18 83             @endIf_Down: JSR EnemyGetDeltaX ; $8318 → Temp00_Delta 重算
a5 00                LDA Temp00_Delta
10 0f                BPL @endIf_Left
20 d4 c3             JSR TwosComplement
85 66                STA EnemyMovePixelQty
20 a7 84             @loop_Left: JSR EnemyMoveOnePixelLeft  ; $84A7
20 6e 81             JSR EnemyIfMoveFailedLeft             ; $816E
c6 66                DEC EnemyMovePixelQty
d0 f6                BNE @loop_Left
f0 0c                BEQ @RTS           ; 左段完成 ⇒ 尾
85 66                STA EnemyMovePixelQty
20 fe 84             @loop_Right: JSR EnemyMoveOnePixelRight ; $84FE
20 34 81             JSR EnemyIfMoveFailedRight              ; $8134
c6 66                DEC EnemyMovePixelQty
d0 f6                BNE @loop_Right
60                   @RTS: RTS
```

- 轴序固定 **Y 先、X 后**，且 X 的 delta 在 Y 移动**之后**才计算（`EnemyGetDeltaX` 会再次读 `L977B` 与移动串指针，可能被子像素/朝向变化影响）。
- 每像素一次「移动 + 失败处理器」；失败处理器把 `EnemyMovePixelQty` 置 1，`DEC` 后为 0 ⇒ 本轴立即出环（但**不影响另一个轴**：X 段仍会执行）。
- `data05.bit6` 置位的帧：整件跳过 ⇒ 这些帧**连动画也不推进**？不 —— 动画由 `UpdateEnemyAnim` 独立执行，bit6 只在 `EnemyMove` 生效；同帧 `UpdateEnemyAnim` 照常按 `animDelay` 推进。
- `EnemyMoveOnePixel*` 返回 carry=1 表示「本像素移动成功」，失败处理器只在 carry=0 时做判据。

### 4.2 `DrawEnemy` 的可见旗（A 级，`07:DD8B` → `LDEDE` 段）

`DrawEnemy` 在把帧写入 OAM 后，用 `IsObjectVisible` 的结果写 `Ens.data05` **bit1**（ROM `07:DDxx`：`lda data05 / and #$FD / ora Temp08 / sta data05`），再按同一 flag 决定是否真的写 OAM。⇒「可见旗」在 ROM 里是由 `DrawEnemy` 维护的，不是 `UpdateEnemyCommon` 的尾件。

移植现状：`crates/game-core/src/enemy/framework.rs:690` 把这段可见旗**提前到** `draw_enemy` 调用之前执行（同值写入，`(d5 & 0xFD) | 0x02`），语义应等价，但**本卡没有做差分验证**（无 trace），标 B 级「应当等价，待 L1 差分确认」。

## 5. 状态机与迁移表

### 5.1 `status`（`EnsExtra.status`，`$6AF4+16k`）→ 入口（A 级）

| status | 值 | 走哪个入口 | 依据 |
|---|---|---|---|
| `NoEnemy` | 0 | `UpdateEnemyCommon` 全件（但 §2 尾件跳过绘制） | `Decide` 默认 |
| `Resting` | 1 | `@noMove`（anim，无 move） | `Decide` `cmp #1` |
| `Active` | 2 | `UpdateEnemyCommon` 全件（`EnemyMove` 的 status 门放行） | `Decide` 默认 + `EnemyMove` `cmp #2` |
| `Explode` | 3 | `@noMoveNoAnim`（都不做，仅尾件） | `Decide` `cmp #3` |
| `Frozen` | 4 | **两条路**：①进入 `UpdateEnemy_Frozen` 先 `jsr EnemyReactToSamusProjectile`，若这一下把它打成 `Explode` ⇒ `beq UpdateEnemyCommon` **走全件入口（会 move！）**；②活着 ⇒ 尾件 `jmp @noMoveNoAnim`（anim/move 都不做） | asm `prg7_engine.asm:10836-10887`（`07:F43E`–`07:F480`） |
| `Pickup` | 5 | `UpdateEnemyAnim` 直接 `rts`；尾件仍走 | `07:E099` |
| `Hurt` | 6 | 不在本卡（`UpdateEnemy_Hurt` 后续处理） | `Decide` 只认1/3 |

### 5.2 `data05` 与 `data1F` 的状态语义（B 级：注释 + 代码测试点，见 `dispatch-tables.md` §2.2）

- `data05.bit6`：30FPS 交替；`EnemyMove` 置位帧跳过（见 §4.1）。
- `data05.bit7`：决定「朝向」读 bit2 还是 bit0（注释口径，本卡未取证到具体敌人用法，见 §8 未取证）。
- `data05.bit5`：置位 ⇒ 触发 resting（`EnemyTriggerResting`）。
- `data05.bit1`：可见旗，由 `DrawEnemy` 写。
- `data1F`：`$00` 反弹、`$40` 落地/落墙（触发 resting 且清对应加速）、`$80/$C0` 落天花板/左墙（速度减半）。

### 5.3 两张状态图（Mermaid 不保证渲染；等价迁移表在 §5.1/§5.2）

```
动画： (animDelay>0) --每帧dec--> (animDelay==0) --写新延时+A--> (读 animIndex)
       (读)==FrameEnd --> (重置到 resetAnimIndex) --> 读
移动： status!=Active 或 data05.bit6 --> 不动
       否则: deltaY 逐像素上/下 --> deltaX 逐像素左/右
       每像素失败 --> EnemyIfMoveFailed*（bounce / 半速 / resting / 翻朝向）
```

## 6. 依赖清单（B3 的下游）

| 依赖 | 地址 | 状态 |
|---|---|---|
| `UpdateEnemyAnim` | `07:E094` | 已入库：`enemy/framework.rs`（`update_enemy_anim`） |
| `DrawEnemy` | `07:DD8B` | 已入库：`render/enemy_sprite.rs` + `draw_enemy` 包装 |
| `EnemyMove` + 4 失败处理器 | `01–05:8058/$80B8/$80FB/$8134/$816E` | 已入库：`enemy/motion.rs:348` 起 |
| `EnemyMoveOnePixel*` | `01–05:83F5(Up)/844B(Down)/84A7(Left)/84FE(Right)` | 已入库：`enemy/motion.rs:552/580/619/676` |
| `EnemyGetDeltaX/Y` | `01–05:8318/$8244` | 已入库：`motion.rs`（含加速度分支） |
| `LoadTableAt977B` + `L977B` | `01–05:80B0/$977B` | 规格：#46 `dispatch-tables.md` §2；代码：`enemy/*` 内 `l977b_raw` |
| `TwosComplement` | `07:C3D4` | 已入库：`arith.rs:124 twos_complement` |
| `XorEnData05` | `01–05:856B` | 已入库：`enemy/motion.rs` |
| `EnemyTriggerResting(_AndClearEnAccelX/Y)` | `01–05:81B8/$81B1/$81C0` | 已入库：`motion.rs` 头部三个 helper |
| `EnemyMovementPtrs` / `EnemyMovementPtr` | `01–05:96DB` / `$81-$82` | 规格：#46 §3；代码：`motion.rs` |

⇒ B3 的「已入库」不是零起点：三入口与移动主体已存在；**B3 落地卡应做的是差分验证 + 缺口补齐**（见 §7）。

## 7. 与 backlog 对齐

`porting-backlog.md` §2 批 3（14–20 条）= 7 条、全部 shared/engine：

| 条目 | 本卡覆盖 | 现状 |
|---|---|---|
| 14 `UpdateEnemyCommon` | §1.3/§2 全语义 + 三入口 | 代码已在 `framework.rs:647/678/690`；本卡给不变量（无边界/尾件三条/寄存器 A 透传） |
| 15 `noMove` / `noMoveNoAnim` | §1.3/§2/§5.1 入口与状态映射 | 同上（`$F438` 是独立入口，不是别名） |
| 16 `EnemyMove` | §4.1 门控/轴序/失败处理器矩阵 | 代码已在 `motion.rs:348`；本卡给出「bit6 跳过整个移动」「每像素失败即停本轴」两条不变量 |
| 17 `EnemyIfMoveFailedUp/Down` | §4.1 矩阵上半 | 代码已存在；**B 级**：`data1F` 半速/resting 的具体数值行为需差分 |
| 18 `EnemyIfMoveFailedLeft/Right` | §4.1 矩阵下半 | 同上 |
| 19 `EnemyGetDeltaX/Y` | §6 只列地址与调用点 | 代码已存在；逐分支（加速度 vs 移动串）未在本卡展开 |
| 20 `UpdateEnemyAnim` | §3 全语义 | 代码已存在；本卡强调「延时递减要**写回 RAM**（`DEC abs,X`），不能改成局部变量」这条易错行为 |

**下一张卡（B3 落地）的入口定义**：
1. 目标 = 差分验证而非重写：在 `analysis/` 造「单敌单帧」用例（固定 `status/data05/data1F/type`），以 `rust_screenshot --trace` 对拍 `$0400-$04FF` 与 OAM，先验 §3 的延时语义与 §4 的失败处理器矩阵。
2. 先修 B 级两处：`DrawEnemy` 可见旗写点（§4.2 前置 vs ROM 内联）与 `data1F≥$80` 半速的舍入方向。
3. 出口判据：单敌用例逐帧 0 差异 + 现有 21 案 L4 无新增 diff；不新增判据/阈值。

## 8. 分级与未覆盖

**A 级（字节/行号可复现）**：五 bank 跳板字节与 sha、shared 段相同范围、`$F410`–`$F43D` 本体、`Decide` 5 份同字节（23B）、三主入口+两子入口地址与尾件三条分支、`UpdateEnemyAnim` 延时语义、`EnemyMove` 门控与轴序、`data05` 各 bit 与 `DrawEnemy` 可见旗的 ROM 写点。

**B 级（由注释/命名推定）**：bank6 为何没有跳板、`data1F` 各值在半速/resting 上的精确舍入、`DrawEnemy` 可见旗在 Rust 侧前置是否严格等价。

**未取证**：没有跑 Mesen2，所以「实机某帧是否真的进入 resting/半速」没有 trace 证据；`EnemyGetDeltaX/Y` 的加速度分支未逐行展开；`Hurt/Frozen/Pickup` 的完整迁移链只给了入口不变量（属于后续卡）；`data05.bit7` 被哪些敌人用来切换「bit2/bit0 朝向」未取证。

## 9. 复跑命令

```bash
# 1) 五 bank 跳板字节 + sha（期望 5 份 4c10f44c38f44c16f4）
python3 - <<'EOF'
rom=open('rom/Metroid (USA).nes','rb').read(); import hashlib
def off(b,a): return 16+b*0x4000+((a-0xC000) if a>=0xC000 else (a-0x8000))
for b in range(1,7):
    seg=rom[off(b,0x8000):off(b,0x8000)+9]
    print(b, hex(off(b,0x8000)), seg.hex(' '), hashlib.sha256(seg).hexdigest()[:16])
EOF

# 2) shared 段相同范围（期望前 0x0D60 字节 0 差）
python3 - <<'EOF'
rom=open('rom/Metroid (USA).nes','rb').read()
def off(b,a): return 16+b*0x4000+((a-0xC000) if a>=0xC000 else (a-0x8000))
a=rom[off(1,0x8000):off(1,0xA000)]
for b in range(2,6):
    c=rom[off(b,0x8000):off(b,0xA000)]
    d=[i for i,(x,y) in enumerate(zip(a,c)) if x!=y]
    print(b, len(d), hex(d[0]) if d else None)
EOF

# 3) UpdateEnemyCommon 本体（46 字节）
python3 -c "rom=open('rom/Metroid (USA).nes','rb').read(); o=16+7*0x4000+0x3410; print(rom[o:o+0x2E].hex(' '))"

# 4) 五个 Decide 块 sha（22B: 5907c96a4763a3de / 23B: 914cf5a391dbf62e）
python3 - <<'EOF'
rom=open('rom/Metroid (USA).nes','rb').read(); import hashlib
def off(b,a): return 16+b*0x4000+((a-0xC000) if a>=0xC000 else (a-0x8000))
for b,a in {1:0x99A1,2:0x9839,3:0x97E2,4:0x9905,5:0x9830}.items():
    for n in (0x16, 0x17):
        print(b, n, hashlib.sha256(rom[off(b,a):off(b,a)+n]).hexdigest()[:16])
EOF

# 5) 源码锚
grep -n "CommonJump_UpdateEnemyCommon\|^EnemyMove:\|^EnemyIfMoveFailed" refs/m1disasm/SRC/areas_common.asm
grep -n "^UpdateEnemyCommon:\|^@noMoveNoAnim:\|^@noMove:\|^UpdateEnemyAnim:\|^DrawEnemy:" refs/m1disasm/SRC/prg7_engine.asm
```

## 10. 落地（#51）：typed 化与三条路径单测

### 10.1 改动（仅 `crates/game-core/src/enemy/framework.rs`）

| ROM 写/读 | 旧形 | 现形 | 为什么 |
|---|---|---|---|
| `lda EnsExtra.status,X / beq @endIf_B` | `mem.read_wram(0x6AF4 + x)` | `mem.enemy(slot_of(x)).status_raw()` | 「+0=status」的语义收进 typed 面；`arch-coupling` 不再把它当裸字面量 |
| `sta Ens.isHit,X` | `mem.write_ram(0x0400+x+4, 0)` | `mem.enemy(slot_of(x)).set_is_hit(0)` | 同上 |
| `sta Ens.weaponAction,X` | `mem.write_ram(0x0400+x+14, 0)` | `mem.enemy(slot_of(x)).set_weapon_action(0)` | 同上 |
| `lda EnemyStatusPreAI`（`$81`） | `mem.read_ram(0x0081)` | `mem.enemy_status_pre_ai()` | 具名访问器已有（`frame/memory.rs:833`） |

子入口与 typed 面对应（#48 退回项，本卡按此落地）：

| asm 标签 | Rust 面 |
|---|---|
| `@noMoveNoAnim ; 07:F416` | `update_enemy_common_no_move_no_anim` |
| `@setHurtPalette ; 07:F423` | `mem.set_obj_cntrl(a)`（A 即调用方调色板；无独立函数——它只写 `$6B`） |
| `@clearIsHitFlags ; 07:F42D` | `Enemy::set_is_hit(0)` + `Enemy::set_weapon_action(0)`（尾件同一段） |
| `@noMove ; 07:F438` | `update_enemy_common_no_move_anim` |

`EnemyMove` 本体（`motion.rs`）**未改**：它内部仍按 ROM 地址写 speed/subpixel/data1F/delay，属独立的
逐指令搬迁面，本卡边界只覆盖 `update_enemy_common*` 家族（见 §8 未覆盖）。

### 10.2 单测（`framework.rs::tests_b3_common_paths`，5 例，字段级断言）

| 测试 | 断言到的字段 |
|---|---|
| `animation_path_writes_delay_frame_and_index` | `animDelay←A`、`animFrame←table[0]`、`animIndex 0→1` |
| `animation_path_decrements_delay_without_reloading_frame` | `animDelay 3→2` 写回、`animFrame/animIndex` 不变 |
| `velocity_path_moves_by_speed_and_skips_on_bit6` | 加速类 `speedX=2` ⇒ `pos_x +2`；`data05.bit6` 置位帧不动 |
| `no_move_no_anim_clears_flags_and_sets_tough_palette` | `isHit→0`、`weaponAction→0`、`animDelay/animFrame/x/y` 不变、tough ⇒ `ObjectCntrl=$A3` |
| `tough_palette_not_rewritten_when_bit7_already_set` | `ObjectCntrl.bit7` 已置 ⇒ 保持调用方调色板（`$8A`） |

测试脚手架也走 typed 面（`set_in_area` / `set_y` / `set_pos_x` / `set_enemy_type`）：新增裸字面量会让
`arch-coupling` 报「新增直接依赖」——这条门禁在测试代码里同样生效，实测第一版就因此红了两处。

### 10.3 验收数字（实跑，2026-11-25）

| 项 | 值 |
|---|---|
| `cargo test -p game-core` | lib **1047 passed / 0 failed**（本卡 +5）；各 target 全绿 |
| `python3 gates/cli.py check` | `PASS=24 FAIL=0 SKIP=3 TOLERATED=1`（`CHECK: PASS`） |
| `python3 tools/l4_diff_run.py run-all --skip-emu` | 21 案 172 格：**match 171 · diff 1 · blocked 0**（唯一 diff = input-conflict c674 84px，已知参考图代际伪差，#38 已定界） |

### 10.4 复跑

```bash
cargo test -q -p game-core --lib tests_b3_common_paths
cargo test -q -p game-core
python3 gates/cli.py check
python3 tools/l4_diff_run.py run-all --skip-emu
```
