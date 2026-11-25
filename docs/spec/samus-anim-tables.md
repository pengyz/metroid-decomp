# Samus 动画表与状态机规格（B6 前置）

范围：玩家侧（object slot 0）的动画数据表与 `$0300` 状态机；只取证，不改代码。
本文件所有数字都能由 §5 命令重算；标 **[A]** 的是字节/反汇编直证，**[B]** 是代码语境推断，**[C]** 是假设（只进 §7）。

## 0. 结论与卡面口径更正

1. **动画数据是三层**：动画索引表（`01:8572`，每动画一段字节序列，`$FF` 结尾）→ 帧指针表（`01:860B`，106 项 ×2 B）→ 帧数据（同 bank 内，`[属性/放置字节][radiusY][radiusX]` + tile 字节，`$FC/$FD/$FE` 是控制字节）。**[A]**
2. **没有「受伤状态」**：被击中不改 `$0300`。受伤 = `$030A isHit` 的 bit5（`$20`）被置位 ⇒ `CheckHealthStatus` 设 50 帧无敌、击退速度（`speedY=$FD`、`accelY=$38`），状态机仍在原 handler 里跑。**[A]**（`07:CDFA`）。所以卡面要的「受伤动画」不是独立动画表：当前动画继续跑，被**炸弹**炸时另走闪烁分支把 `animFrame←$F7`（整帧不画）；被敌人撞则不闪烁（见 §2.3）。
3. **卡面的字段偏移错两个**：`$0303` 是 `animFrame`（当前帧号），不是 `animIndex`；`animIndex` 在 **`$0306`**。`$0305`=`animResetIndex`，`$0304`=`animDelay`。卡面写的「朝向 `$030A/$030B/$030C`」也不对：那是 `isHit/onScreen/hi`；朝向是 **`SamusDir`（`$004D`）** + `ObjectCntrl` 的镜像位。**[A]**（`constants_ram.asm:216-236`）。
4. **状态机 = 10 个 handler 的跳转表**（`07:CC22`），状态值 0–9；`status=$FF`（`sa_Begin`）也走 0 号 `SamusStand`（`bmi` 分支）。**[A]**
5. **每帧动画推进只有一处**：`UpdateObjAnim`（`07:DC8F`）。它先倒数 `animDelay`，归零后取 `ObjectAnimIndexTbl[animIndex]` 写 `animFrame` 并把 `animIndex+1`；遇到 `$FF` 就回绕到 `animResetIndex`。**[A]**
6. 帧时长不是一个统一常数：站立/跑 = 3 帧一换（`lda #$03` @ `07:CD6B`），跳跃 = 4（@ `07:CFC5`），翻滚 = 8（@ `07:D088` 附近），其余（如指向上、射击帧）多为 0（立即生效）。**[A]**（逐条见 §3.3）。

## 1. 动画索引表（AnimTable）

- 位置：**`01:8572`**，长度 **153 B（`0x99`）**（= `0x860B` − `0x8572`，即紧邻帧指针表之前），bank1–5 各有一份**逐字节相同**的副本（`common_sprite_data.asm` 被五个区域 bank include）。**[A]**
- 记录布局：一段连续字节，每字节 = **帧号**（`_id_ObjFrame_*`，即 `ObjFramePtrTable` 的下标）；`$FF` = 动画结束；`$F7` = 该帧不可见（`_id_FrameNone`）。**[A]**
- 引用方式：`SetSamusAnim`（`07:CF6B`）写入的是相对 `ObjectAnimIndexTbl` 的**偏移**（asm 写作 `#ObjAnim_X - ObjectAnimIndexTbl.b`）。**[A]**
- 五个必需状态对应的条目（`01:8572` 起，逐字节解出）：

| 状态 | 动画 label | 偏移 | 地址 | 帧序列（`$FF` 前） |
|---|---|---|---|---|
| 站立 idle | `ObjAnim_SamusStand` | `0x07` | `01:8579` | `08`（单帧） |
| 跑 | `ObjAnim_SamusRun` | `0x00` | `01:8572` | `03 04 05` |
| 跳（中性） | `ObjAnim_SamusJump` | `0x0C` | `01:857E` | `10` |
| 跳（空翻） | `ObjAnim_SamusSalto` | `0x0E` | `01:8580` | `17 18 19 1A` |
| 球（滚） | `ObjAnim_SamusRoll` | `0x16` | `01:8588` | `1E 1D 1C 1B`（**倒序**：先 Roll0 的后一帧） |
| 受伤 | —（复用当前动画） | — | — | 见 §2.3 闪烁，无独立条目 |

补充条目（同一张表）：`Front`=`0x04`→`07`；`Unroll`=`0x06`→`17 08`（与 Stand 共享尾字节）；`StandFire`=`0x09`→`22`；`JumpTransition`=`0x0B`→`04 10`（先跑帧再跳帧）；`RunJump`=`0x13`→`03 17`；`PntUp`=`0x27`、`RunPntUp`=`0x37`、`JumpPntUp`=`0x35`（与 Rust 常量互证，见 §4）。**[A]**

## 2. 帧指针表与帧数据

### 2.1 帧指针表

- 位置：**`01:860B`**，长度 `0xD4` = 212 B = **106 项 ×2 B**（小端指针，指向同一 bank 的帧数据）。bank1–5 各一份。**[A]**
- 用法：`animFrame` 存的是**未翻倍的序号**；`ObjDrawFrame`（`07:DE4A`）里 `asl` 成 `*2` 再取指针。**[A]**
- 例：`id=8`（`_id_ObjFrame_SamusStand`）→ `01:860B+16` 的 `18 88` → `01:8818`。**[A]**
- 表在 `game-data` 的烘焙偏移：`OBJ_ANIM_INDEX_TBL=0x0099` 之前是索引表本体，`OBJ_FRAME_PTR_TABLE=0x0099`、`OBJ_PLACE_PTR_TABLE=0x1C9`（`crates/game-data/src/assets.rs:904-910,1604`），与 ROM 的 `0x8572/0x860B/0x86DF` 差值逐项相等。**[A]**

### 2.2 帧数据布局

每帧数据（`[B]`，依据 `DrawSpriteObject` `07:DF19`、`WriteSpriteRAM` `07:DEE6`、`GetNewControlByte` `07:DF3B`、`OffsetObjectPosition` `07:DF51` 逐指令）：

```
偏移 0: 属性/放置基字节（高半字节 = 初始 y 偏移/翻转位组合，低半字节 = 放置表起点 id）
偏移 1: radiusY（object 半径；绘制前 ReduceYRadius 再 -$10）
偏移 2: radiusX
偏移 3..: 交错序列
  0x00-0xFB: tile 号（走 WriteSpriteRAM，消耗一组放置表 y/x）
  0xFC: 改变对象绘制原点：后跟 y 偏移、x 偏移两个字节
  0xFD: 设置新的 sprite 控制字节（后跟 1 B 属性），通常用于翻转/优先级切换
  0xFE: 跳过下一个放置项的 y/x（该位置留空，用于挖空）
  0xFF: 结束（落到 ClearObjectCntrl）
```

两条真实帧：

```
SamusStand (01:8818, 18 B): 40 0F 04  FD 20  0E 0D  FE  1E 1D 2E 2D  FE  FD 60 3B 3C FE
SamusRun0  (01:87CB, 18 B): 40 0F 04  00 01  FD 20  FE  41 40  FD 60  20 21  FE FE  31 FF
```

（`0x40` = 属性基、`0F`/`04` = radiusY/radiusX；两帧都先 `FD 20` 换一次控制字节，再排 tile。）**[A]**

### 2.3 「受伤」在画面上怎么表现

- `isHit & $20` ⇒ `07:CE0F` 起：`SamusInvincibleDelay←$32`（50 帧）、`speedY←$FD`、`accelY←$38`、`SamusKnockbackDir` 取 `isHit` bit3。**[A]**
- 每次进 handler 后 `CheckHealthStatus`（`07:CDFA`）跑一遍：`InvincibleDelay` 非 0 就减 1，并按 `SamusKnockbackDir` 用 `Adiv16` 算击退水平速度（上限 3 px/帧）。**闪烁只在被炸弹炸时**：`lda SamusKnockbackIsBomb77; bpl` 跳过，否则 `lda FrameCount; and #$01; bne` ⇒ 奇数帧 `animDelay←0`、`animFrame←$F7`（整帧不画）。对应汇编 `prg7_engine.asm:3152-3163`（`07:CE53` 区）。**[A]**
- 所以「受伤动画」不是表项：不写 `ObjectAnimIndexTbl`、不改 `animResetIndex`，只叠加帧号 `$F7` 与击退速度。**[A]**

## 3. 状态机与字段写点

### 3.1 `$0300` 状态值与 handler

`GoSamusHandler`（`07:CC1A`）：`lda Samus.status; bmi SamusStand`（`$FF`=sa_Begin 也落 0 号），否则 `JumpEngine 07:C27C` 查表 `07:CC22`（10 项小端地址）。**[A]**

| status | 名 | handler | 说明 |
|---|---|---|---|
| 0 | `sa_Stand` | `07:CC36` | 站立；按下方向/开火在这里转 Run/StandFire |
| 1 | `sa_Run` | `07:CCC2` | 跑；也是空翻跳（somersault）的 handler |
| 2 | `sa_Jump` | `07:D002` | 中性跳 |
| 3 | `sa_Roll` | `07:D0E1` | 球形态滚动 |
| 4 | `sa_PntUp` | `07:D198` | 指向上站立 |
| 5 | `sa_Door` | `07:D3A8` | 门内过场（屏幕滚动） |
| 6 | `sa_PntJump` | `07:D002` | 与 2 同实现（指向上跳共用 `SamusJump`） |
| 7 | `sa_Dead` | `07:D41A` | 死亡第一段 |
| 8 | `sa_Dead2` | `07:D41F` | 死亡爆炸段（`dec animDelay` 倒数） |
| 9 | `sa_Elevator` | `07:D423` | 电梯内 |

### 3.2 关键字段（`Object` 结构，slot 0 基址 `$0300`）

| 字段 | 地址 | 写者（bank:addr） | 语义 |
|---|---|---|---|
| `status` | `$0300` | 7 处直接 `sta Samus.status`（见 §3.4 表） | 状态值 |
| `radiusY/radiusX` | `$0301/$0302` | `07:DEBC` 区（帧绘制时按帧数据写） | 碰撞半径（像素） |
| `animFrame` | `$0303` | `UpdateObjAnim 07:DC8F`；闪烁分支 `prg7_engine.asm:3163` | 当前帧号（未翻倍） |
| `animDelay` | `$0304` | `07:CD6B`(3)/`07:CFC5`(4)/`07:D088` 区(8)/`07:D41F`(dec) | 换帧倒计时 |
| `animResetIndex` | `$0305` | `SetSamusAnim 07:CF6B` | 动画回绕点 |
| `animIndex` | `$0306` | `SetSamusAnim 07:CF6E`、`UpdateObjAnim 07:DCAB` | 表内游标 |
| `speedY/speedX` | `$0308/$0309` | 各 handler | 速度（MSB=负） |
| `isHit` | `$030A` | 碰撞检测写 | 受伤/接触标志（bit5=被击中） |
| `onScreen` | `$030B` | `07:DEDE` 区 | 可见性 |
| `hi` | `$030C` | `ToggleSamusHi` | 名表 0/3 |
| `y/x` | `$030D/$030E` | 各 handler | 房间坐标 |
| `SamusDir` | `$004D` | `07:CFA0` 等 | 朝向（0=右 1=左） |
| `SamusAccelX/Y` | `$0315/$0314` | 加速度写点 | 子像素加速度 |
| `SamusInvincibleDelay` | `$0070` | `07:CE0F`=50 | 无敌帧数 |

### 3.3 切状态时写的三个动画字段（同一时刻、同一函数）

`SetSamusAnim`（`07:CF6B`）：

```
sta Samus.animResetIndex   ; $0305 ← 动画基（表内偏移）
sta Samus.animIndex        ; $0306 ← 同一值（下一帧就取它）
sta Samus.animDelay = #$00 ; $0304 ← 0：立即生效
```

`SetSamusData_3FrameAnimDelay`（`07:CD6B`）先 `lda #$03` 再落到 `SetSamusData`（`07:CD6D`）→ `jsr UpdateObjAnim`，即「以 3 为换帧周期」并立即推进一帧。跳跃路径不走这里，而是 `07:CFC5` 直接 `lda #$04 / sta Samus.animDelay`；翻滚在 `07:D088` 前后 `lda #$08`。**[A]**

### 3.4 每帧时序（一次 `UpdateSamus` 内）

```
UpdateSamus 07:CC0D
  PageIndex←0, IsSamus←1
  GoSamusHandler（按 status 分派到 §3.1 的 handler）
     ├─ handler 内可能 SetSamusAnim / SetSamusData* → 写 $0305/$0306/$0304
     ├─ 移动/物理：写 speedX/speedY/accel*/RadiusX（翻滚时半径变 8）
     └─ CheckHealthStatus：受伤/无敌/击退/闪烁（写 $030A、$0070、$0303=$F7）
  IsSamus←0
```

动画推进本身在 **`UpdateObjAnim 07:DC8F`**（由 `SetSamusData` 或 `AnimDrawObject 07:DE47` 调用）：

```
animDelay≠0 → dec；仍≠0 就 return（本帧不换帧）
animDelay=0 → animDelay←A（调用方给的初值）
            → y←animIndex；frame←ObjectAnimIndexTbl[y]
               frame=$FF → y←animResetIndex，重取
               否则 animFrame←frame；animIndex←y+1
```

所以「帧时长」的语义是**调用方每次给 `UpdateObjAnim` 的 A**，不是表里的字段。**[A]**

### 3.5 状态迁移（只列与动画表相关的主干）

- 站立 → 跑：`SamusStand` 里方向键按下 → `SetSamusRunAccel` + `BitScan` → `SetSamusAnim(Run)`（`07:CFA0` 附近）。**[A]**
- 跑 → 跳/空翻：`SamusRun` 里跳跃键 → `SetSamusJump`（`07:CFBE`：`animDelay←4`、`speedY←$FC`），有 ScrewAttack 时用 `Salto`，否则 `Jump`。**[A]**
- 站/跑 → 球：下键+方向 → `sa_Roll`，半径改 8（`07:D0E1` 区）。**[A]**
- 任意 → 死：`IsSamusDead` 判 `status∈{7,8}`；进入 `sa_Dead` 后 `ObjDrawFrame` 里 `ObjectCounter==$19` 时 `status←8`、`animDelay←$28`（40 帧爆炸）。**[A]**（`07:DEBC` 前段）
- 受伤：**不改 status**（§2.3）。**[A]**

## 4. 与 Rust 现有实现的逐项对照

| 项 | ROM 事实 | Rust 落点 | 判定 |
|---|---|---|---|
| 动画索引表 | `01:8572`，153 B（`0x99`） | `crates/game-core/src/samus/physics.rs:1028-1046` `OBJECT_ANIM_INDEX_TBL`（153 项，逐项同） | 一致 |
| 帧指针表 | `01:860B`，`0xD4` | `crates/game-data/src/assets.rs:1604` `OBJ_FRAME_PTR_TABLE=0x0099`；`oam.rs:196-205` 取用 | 一致 |
| 帧数据 | 同 bank，`FC/FD/FE/FF` 控制字节 | `assets.rs:908-910` 断言与 ROM 偏移差一致；`oam.rs:194-260` | 一致（tile 序列由烘焙 blob 供） |
| 动画推进 | `UpdateObjAnim 07:DC8F` | `samus/physics.rs:1052-1078` `update_obj_anim`（含 `$FF` 回绕、16 次护栏） | 一致 |
| 动画常量 | Run `00`/Front `04`/Stand `07`/Jump `0C`/Salto `0E`/Roll `16`/PntUp `27`/JumpPntUp `35`/RunPntUp `37` | `samus/handler.rs:125-135` | 一致 |
| 状态枚举 | 0–9（§3.1） | `samus/state.rs`（`SamusStatus`） | 一致 |
| `SetSamusAnim` | `07:CF6B` 写 `$0305/$0306/$0304=0` | `handler.rs:163-176` 等 | 一致 |
| 受伤 | 不改 status；`$0070←$32`、`speedY=$FD`、`accelY=$38`、闪烁 `$F7` | `samus/health_change.rs`（knockback）；`oam.rs` 画前读 `$0303` | 待核对：闪烁写点未见显式 `$F7`（`grep -rn '0xF7' samus/` 只命中表常量），需要 L4 逐帧比对确认 |
| 帧时长 | 站立/跑 3、跳 4、滚 8 | `handler.rs:266` `set_anim_delay(3)`；跳跃在 `handler.rs` 内；翻滚阈值待核 | 待核对：跳跃/翻滚的 `4`/`8` 是否在各路径都写到 |
| 电梯/死亡等 | `07:D423`/`07:D41A` | `elevator.rs`、`death.rs:125-140` | 一致 |

## 5. 复现命令（全部实跑过，输出即下文）

### 5.1 动画索引表与各动画的帧序列

```bash
python3 - <<'PY'
import pathlib
r=pathlib.Path('rom/Metroid (USA).nes').read_bytes(); prg=r[16:16+128*1024]
def rd(b,a,n):
    base=0xC000 if b==7 else 0x8000
    return prg[b*0x4000+(a-base):][:n]
t=rd(1,0x8572,0x99); print('len',len(t)); print(t.hex(' '))
for name,off in (('Stand',0x07),('Run',0x00),('Jump',0x0C),('Salto',0x0E),('Roll',0x16)):
    seq=[]; i=off
    while t[i]!=0xFF and len(seq)<8: seq.append(hex(t[i])); i+=1
    print(name, 'off', hex(off), 'addr 01:%04X'%(0x8572+off), seq)
PY
```

输出：`len 153`；`Stand off 0x7 addr 01:8579 ['0x8']`；`Run off 0x0 … ['0x3','0x4','0x5']`；`Jump … ['0x10']`；`Salto … ['0x17','0x18','0x19','0x1a']`；`Roll … ['0x1e','0x1d','0x1c','0x1b']`。

### 5.2 状态分派表（`07:CC22`，10 项）

```bash
python3 - <<'PY'
import pathlib
r=pathlib.Path('rom/Metroid (USA).nes').read_bytes(); prg=r[16:16+128*1024]
b=7;a=0xCC22
print(prg[b*0x4000+(a-0xC000):][:20].hex(' '))
PY
```

输出：`36 cc c2 cc 02 d0 e1 d0 98 d1 a8 d3 02 d0 1a d4 1f d4 23 d4`
→ `CC36 CCC2 D002 D0E1 D198 D3A8 D002 D41A D41F D423`，与 §3.1 表逐项一致（状态 2 与 6 同址）。

### 5.3 帧指针与帧数据

```bash
python3 - <<'PY'
import pathlib
r=pathlib.Path('rom/Metroid (USA).nes').read_bytes(); prg=r[16:16+128*1024]
def rd(b,a,n):
    base=0xC000 if b==7 else 0x8000
    return prg[b*0x4000+(a-base):][:n]
for fid in (8,3):
    p=rd(1,0x860B+fid*2,2); addr=p[0]|(p[1]<<8)
    print('id',fid,'ptr','01:%04X'%addr, rd(1,addr,18).hex(' '))
PY
```

输出：`id 8 ptr 01:8818 40 0f 04 fd 20 0e 0d fe 1e 1d 2e 2d fe fd 60 3b`；`id 3 ptr 01:87CB 40 0f 04 00 01 fd 20 fe 41 40 fd 60 20 21 fe fe`。

### 5.4 受伤链的写点（无反汇编器也可定位）

```bash
grep -n "sta SamusInvincibleDelay" refs/m1disasm/SRC/prg7_engine.asm
grep -n "CheckHealthStatus:" refs/m1disasm/SRC/prg7_engine.asm
```

输出：`3090`、`7139`（另 `1986` 是 `stx`，不是 `sta`）；`3082:CheckHealthStatus: ; 07:CDFA`。

## 6. 已确认的负面结论

1. **没有独立的「受伤动画表项」**，也没有 `sa_Hurt` 状态（§0.2、§2.3）。任何按状态号索引动画的假设在这里都会失败。
2. **帧时长不在表里**：`ObjectAnimIndexTbl` 只存帧号，换帧周期来自调用方传给 `UpdateObjAnim` 的 A（§3.3）。
3. **`$0303` 是帧号不是索引**；`animIndex` 在 `$0306`。卡面偏移写错两个。

## 7. 假设区（C 级，未取证，不得当结论）

- **H1**：`$F7`（`_id_FrameNone`）在受伤闪烁里只用于「整帧不画」，但同一常量在敌人动画里也当占位；未逐帧 trace 确认 Samus 的闪烁是否**每奇偶帧**都写，还是只在被炸时才写（`07:CE53` 的 `SamusKnockbackIsBomb77` 分支只覆盖炸弹）。
- **H2**：`SetSamusData_3FrameAnimDelay` 的名字暗示 3 帧，但 `UpdateObjAnim` 的倒数先减后判——实际可见周期可能是 3 次调用 = 3 帧还是 4 帧，需要 L4 逐帧采样确认。
- **H3**：翻滚 `animDelay←8` 只在某一分支（`07:D088` 邻近）成立；球在斜坡/半砖上的帧率是否不同未取证。

## 8. 证据指针

| 结论 | 指针 |
|---|---|
| 动画索引表地址/长度 | `01:8572`，字节 dump §5.1；`refs/m1disasm/out/M1_NES_NTSC.sym:1200` |
| 帧指针表 | `01:860B`，`0xD4`；`assets.rs:908-910` 断言 |
| 帧数据控制字节 | `07:DF19`（tile）、`07:DF3B`（FD）、`07:DF32`（FE）、`07:DF51`（FC）、`07:DF2D`（FF/清控制） |
| 动画推进 | `07:DC8F`；Rust `physics.rs:1052` |
| SetSamusAnim | `07:CF6B`（写 `$0305/$0306/$0304`） |
| 3 帧周期 | `07:CD6B`（`lda #$03`） |
| 跳跃 4 帧 | `07:CFC5`（`lda #$04`） |
| 受伤/无敌 | `07:CDFA`；`$0070`；`07:CE53-3158` 闪烁 |
| 状态分派 | `07:CC1A` + 表 `07:CC22` |
| 死亡两段 | `07:D41A`/`07:D41F`；`07:DEBC` 前段 `status←8, animDelay←$28` |
| Rust 对照 | `samus/physics.rs:1028-1078`、`samus/handler.rs:125-176`、`render/oam.rs:194-260`、`game-data/src/assets.rs:904-910,1604` |

## 9. 未覆盖

- Samus 的放置表（`ObjPlacePtrTable 01:86DF`）逐项语义与帧数据的 `FE` 跳过项组合未逐帧核对。
- `SamusPntUp/RunPntUp` 各动画的完整帧序列（本文件只给偏移，未逐项列帧）。
- 与 L4 的实际像素对齐（`f2151` 等已收口的帧未反推动画相位）。
