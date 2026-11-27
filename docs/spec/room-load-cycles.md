# 载房 `work_cycles` 分项 ROM 计数表（Brinstar 房 `$18` 三帧窗）

> 卡：#132（tsk_01M8D3KK8ZBS0PYFBDDF24）。写面仅本文件，`crates/` 零改动。
> 结论一句话：**rust 对房 `$18` 的 `work_cycles` 实测是 55308，而「3 帧窗」的门槛是
> 60886，差 5578**；分项里有一项是**已证实的折叠低估**——`LOAD_ROW` 上线 30，而
> `frame/cycles.rs` 按 asm 逐条算出的是 **72**（46 行 ⇒ 至多补 1932）；只补这一项仍
> 到不了 3 帧（≤57240 < 60886），剩下 ≥3646 只能来自**唯一未分解的 `LOAD_SHELL`**
> 或页交叉项。本文另给一条**独立于 work 的观测量**（结构流上界 57132 与实测 55308
> 差 1824），把差因写成可证伪的 C4，而不是猜。

## 0. 口径与结论摘要

| 项 | 值 | 来源 |
|---|---:|---|
| `LOAD_BASE_CYCLES` | 28454 | `crates/game-core/src/world/room_draw.rs:74`（`d9c98374` 收窄） |
| `FRAME_CYCLES` | 29780 | `room_draw.rs:75` |
| 判据 | `dropped = floor((28454 + work)/29780)`，下限 1 | `world/camera/scroll.rs:77` |
| 2 帧门槛 | work ≥ 31106 | `2×29780−28454` |
| **3 帧门槛** | **work ≥ 60886** | `3×29780−28454` |
| 4 帧门槛 | work ≥ 90666 | `4×29780−28454` |
| 房 `$18` 目标 | 十进制 24（f1572 门径载房的目标房） | `docs/findings/fc-drop-accounting.md` §8.3 |
| **房 `$18` rust 实测 work** | **55308** | `docs/findings/room-sweep-fixture-phase.md:58`（房 24，dropped=2）；`/tmp/r129/load.log:2567` `[load] f1574 rn=F0 phase=2 tail=0 work=55308 sx=2` |
| 同房结构流上界（本文脚本） | 57132 | §2；按结构流逐行逐元 tile 累加，**不是实测** |
| 上界与实测之差 | 1824 | 57132 − 55308（差因见 §2.1/C4） |
| **缺口（实测口径）** | **5578** | 60886 − 55308 |

证据分级：ROM 指令与地址 = A；`work` 是 rust 侧模型量（emu 没有这个数），其「等于原版
周期」的前提 = B（沿用 `docs/findings/load-window-multipoint-calibration.md` 的 A1 更正口径）；
本文的「结构流上界」= 脚本复现值，与 rust 实测的 1824 差因 = C（§8 C4）。

## 1. 分项 ROM 计数表

上线值 = `model_*`（逐条 asm）+ `legacy` 残差（`frame/cycles.rs` 的 `BALANCE_*`）。
`model_*` 的指令序列在 `frame/cycles.rs:94-260` 里逐条列出并带 asm 行号，下表只摘
「来源例程 / 每次触发 / 单次周期 / 谁在什么粒度调用」。

| 项（上线常量） | ROM 来源（bank:addr） | 触发粒度 | model | legacy 残差 | **上线值** |
|---|---|---|---:|---:|---:|
| `LOAD_INIT` 9262 | `InitTables 07:EFF8`（`prg7_engine.asm:9999`）：壳 + `FillRoomRAM 07:F01C`（`:10028`，3×256 B）+ 属性表 64 B | 每次 `SetupRoom` 1 次 | 9262 | 0 | **9262** |
| `LOAD_ROW` 30 | `DrawStructRow 07:EF13`（`:9760`）+ `AdvanceRow 07:EF78` | 每个结构行 1 次 | **72** | **−42** | **30** |
| `LOAD_MT_FAST` 214 | `DrawMetatile 07:EF2B`（`:9784`）且 `UpdateAttrib` 早退（`ObjectPalette==RoomPalette`） | 每个元 tile 1 次 | 214 | 0 | **214** |
| `LOAD_MT_FULL` 349 | 同上，`UpdateAttrib 07:EF9E`（`:9907`）走全径 | 每个元 tile 1 次 | 341 | +8 | **349** |
| `LOAD_OBJ` 180 | `DrawObject 07:EA60` 壳（不含 y 位环、不含被调 `DrawStruct`） | 每个对象 1 次 | 114 | +66 | **180** |
| `LOAD_Y_UNIT` 20 | `DrawObject` y 位环 `LEA72`（`lda WorkPtr/clc/adc #$40/sta/bcc/[inc +1]/dex/bne`） | 每个 y 单元 1 次 | 17 | +3 | **20** |
| `LOAD_ENEMY` 300 | `LoadEnemy 07:EB06`（`:8889`）→`GetEnemyData 07:EB0C`→`GetEnemyType`→`LEB4D`→`InitEnemyFacing…`/`ForceSpeed…` | 每条 type=1 敌记录 1 次 | 290 | +10 | **300** |
| `LOAD_SHELL` 1300 | `SetupRoom 07:EA2B`（`:8693`）的固定壳：`DeleteOffscreenRoomSprites 07:EC9B`（`:9234`）+ `ScanForItems 07:ED98`（`:9428`） | 每次 `SetupRoom` 1 次 | 未分解 | 未分解 | **1300** |

单次数字的构成（可逐条复核，均在 `frame/cycles.rs`）：

- `LOAD_INIT` = `FillRoomRAM` 8503（内环 255×11+10=2815，×3 页 + 外壳23 + 外环29 + rts6）
  + 属性填充 703（63×11+10）+ 壳 56（前段21 + `Xplus4`14 + 后段35） = **9262**。
- `LOAD_ROW` model 72 = 行头 50（`and #$0F/bne/lda #$10/sta/lda (StructPtr),y/jsr Adiv16/
  asl/adc WorkPtr/sta/lda #$00/adc WorkPtr+1/sta`，含 `Adiv16` 14）+ `AdvanceRow` 22。
- `LOAD_MT_FAST` 214 = 边界判12 + 取结构指针22 + 4 tile 环111 + 早退 `UpdateAttrib` 21
  + `AddYToPtr00` 24 + 行尾判16 + `PAGE_CROSS_METATILE` 6。
- `LOAD_MT_FULL` = `fast − 早退UA(21) + 全径UA(148)` = 341；全径 body 里 `SHIFT` 按
  **2 轮**记 20c（`(IMP+BR+IMP+IMP+BR)*2`），注释自认「期望 2 轮，非保证值」。
- `LOAD_OBJ` model 114 = `Adiv16` 14 + 前段42 + x 位15 + LEA8D 段57（callee 另计）。
- `LOAD_Y_UNIT` model 17：无跨字节进位的常见径；ROM 在进位时有 `inc WorkPtr+1`（+5c），
  residual +3 即这一项的期望补偿。
- `LOAD_ENEMY` model 290 = `jsr`6 + `GetEnemyData` 281 + 尾跳3；其中 `LEB4D` 自身 178、
  `ForceSpeed` 35（普通敌 bit7 清 ⇒ 跳过 `asl` 的 3c 跳径）、`GetEnemyType` 32（bit6 清）。

**调用粒度（`world/room_draw.rs`）**：

| 累加点 | 代码位置 | 粒度 |
|---|---|---|
| `LOAD_SHELL` | `room_load.rs:288` | 每次 `setup_room` |
| `LOAD_INIT` | `room_draw.rs:925`（`init_tables`） | 每次载房 1 次 |
| `LOAD_OBJ` | `room_draw.rs:146` | 每个对象记录（3 B） |
| `LOAD_Y_UNIT` | `room_draw.rs:150` | 每对象 × `pos>>4` |
| `LOAD_ROW` | `room_draw.rs:190` | 每个结构行 |
| `LOAD_MT_FAST/FULL` | `room_draw.rs:211-216` | 每个元 tile，按 `obj_palette==room_palette` 分支 |
| `LOAD_ENEMY` | `room_draw.rs:721` | 敌门环里每条 type=1 |

## 2. 房 `$18` 的结构与两种读数

`python3` 从 ROM 直解（不依赖游戏运行）：

- 区表（`crates/game-data/src/extract.rs` 的 `AREA_BR`）：房间指针 `bank01:A314` 47 项、
  结构指针 `bank01:A372` 50 项；房流 `bank01:*` 自界 `$FF`（`read_until_ff`）。
- 房 `$18` 流 64 B：`palette=01`，**16 个对象**（3 B/条），`$FD` 后 **3 条敌记录**（type=1），
  末 `$FF`。
- 16 个对象的结构合计：**46 行**、**140 元 tile**（70 走 fast、70 走 full——对象调色板与
  `RoomPalette=1` 相同/不同各半），y 单元合计 **100**。

按 rust 累加器逐项累加（结构流上界）：

```
work = LOAD_SHELL 1300
     + LOAD_INIT 9262
     + 16×LOAD_OBJ 180            = 2880
     + 100×LOAD_Y_UNIT 20         = 2000
     + 46×LOAD_ROW 30             = 1380
     + 70×LOAD_MT_FAST 214        = 14980
     + 70×LOAD_MT_FULL 349        = 24430
     + 3×LOAD_ENEMY 300           = 900
     = 57132        ← 结构流上界（不是实测）
```

rust 对该房的**实测** work 是 **55308**（两处独立工件）：

- `docs/findings/room-sweep-fixture-phase.md:58`：房 24（=十进制 24=`$18`）→ `work_cycles
  55308`、`dropped=2`；
- `/tmp/r129/load.log:2567`：`[load] f1574 rn=F0 phase=2 tail=0 work=55308 sx=2`（#129 门径
  载房窗内）。

两者相差 **1824**。按 `dropped = floor((28454+55308)/29780) = floor(83762/29780) = 2`
⇒ **rust 必然 2 帧**，与 #129 §8.8 的实测（emu 1572–1574 三帧、rust 1572–1573 两帧）一致。

**复算等价性校验**（同一脚本对既有实测点回归）：room20/19/9/8 得 31446/36620/43378/71946，
与 `docs/findings/load-window-multipoint-calibration.md` §2.2/§3 记录的四个 rust 实测 work
值**逐点相等**。这证明「计数脚本与 rust 累加器同口径」，但**不**证明房 `$18` 的结构流上界
等于它的实测值——房 `$18` 恰好差 1824，见 §2.1。

### 2.1 上界 57132 与实测 55308 的 1824 差：不是边界守卫

一个自然解释是 `room_draw.rs:205 metatile_out_of_bounds` 提前 return。**实测否掉了它**：
按守卫的判据（`hi<0x63` 放行；`0x63/0x67` 页内 `lo≥0xA0` 截断；`>0x67` 截断）对房 `$18`
逐元 tile 模拟，行指针最大值：

| RoomRAM 缓冲 | 最大行指针 | 守卫阈值 | 是否触发 |
|---|---|---|---|
| `$6000`（名表0） | `$6392` | `$63A0` | **否** |
| `$6400`（名表3） | `$6792` | `$67A0` | **否** |

⇒ 房 `$18` 在两种缓冲下都不会被 `metatile_out_of_bounds` 截断，1824 的差因**不在**这条
守卫。剩余可能见 §8 C4（门径载房对该房的流解释、或 `DrawObject` 在门径下少跑一个对象），
需运行时探针定谳；本文不把它当已证。

## 3. 3 帧窗需要多少 work

```
dropped ≥ 3  ⇔  28454 + work ≥ 3×29780  ⇔  work ≥ 60886
```

实测 55308 ⇒ **缺口 5578**（若用结构流上界 57132，则缺口 3754——两个数分别对应「rust 现在
真实差多少」与「按结构流最多差多少」，本文后续一律用**实测口径 5578**）。

## 4. 哪一项被折叠/低估（定谳与半定谳）

### 4.1 已证实：`LOAD_ROW` 上线 30，asm 逐条 72（−42/行）

`frame/cycles.rs:217-241` 的 `model_draw_struct_row()` 逐条列出 `DrawStructRow 07:EF13`
与 `AdvanceRow 07:EF78` 的指令，合计 **72**；而 `ship()` 后的上线常量是
`room_draw.rs:78 LOAD_ROW_CYCLES = 30`（`legacy::ROW`），残差表里明确写着 `ROW −42`。
46 行 × 42 = **至多 +1932**（若某对象在门径下被截，增量更小）。

以实测为基数：`55308 + 1932 = 57240`，仍 `< 60886` ⇒ **单独修 ROW 不会翻 3 帧**。

### 4.2 剩余 ≥3646：只能是 `LOAD_SHELL` 或页交叉（半定谳，A/B 分界写清）

补完 ROW（取上限）还差 `60886 − 57240 = 3646`。候选只有两个：

1. **`LOAD_SHELL` = 1300 是唯一未分解项**：它盖住 `DeleteOffscreenRoomSprites 07:EC9B`
   （2×`UpdateDoorData` + 6 敌槽 + 4 Mellow + 门/瓦爆/3 弹/电梯/`Mem0700` 环）与
   `ScanForItems 07:ED98`（按 `MapPosY/X` 走特项表 + `Spawn*` 分发）。1300 对这两个
   例程是**下界性质的手估**，没有逐条模型；只要真实壳 ≥ **4946**（1300+3646）即可补足。
2. **页交叉期望值**：`PAGE_CROSS_METATILE=6` 是固定补偿，注释自认「期望值不是保证值」；
   `UpdateAttrib` 全径的 `SHIFT` 也只按 2 轮记。房 `$18` 有 140 个元 tile（70 full），
   若平均每元 tile 多 26c 就够 3646——量级仍落在「页交叉 + SHIFT 轮数」范围内，但没有
   逐字节证据，故列 C 级（§8 C1/C2）。

**给织码的判据**：不要直接把所有 `legacy` 残差清零（那会把 MT_FULL/OBJ/Y/ENEMY 的正向
残差也一起改掉，方向相反）。按 §5 的两级预测走。

## 5. 可证伪预测（两级）

| 级 | 变更 | 预测 | 否证判据 |
|---|---|---|---|
| P0（现状） | 不改 | 房 `$18` 实测 work=55308，门径窗 **2 帧** | 若实测 work≠55308 或窗已是 3，本文 §2 的读数为假 |
| P1 | `LOAD_ROW` 30→72（仅此一项） | 实测 work ≤57240，门径窗**仍是 2 帧** | 若窗变 3 ⇒ ROW 不是唯一缺口，另有项被低估 |
| P2 | 再抬 `LOAD_SHELL` 1300→**≥4946** | work≥60886，**门径窗 3 帧**（f1572/1573/1574），且 f1301 `$0454` 仍为 2 B（#130 未修） | 若 work≥60886 而窗仍 2 ⇒ 窗长不由 work 决定，#129 的 work 假设被否证 |

补充一行防误读：**若 `metatile_out_of_bounds` 在不同房截掉不同数量的元 tile，ROW 修复的
实际增量会小于 1932**；所以 P1 的判据是 `work ≤ 57240`（上界），不是「必须等于 57240」。

P2 的窗口判据与 #129 §8.8 相同：emu `$2D` 在 1572/1573/1574 三次冻结，rust span 应同样
覆盖三帧；f1301 的 `$0454` 2 B 不受载房窗影响（#122/#130 已证两条链独立）。

## 6. 复跑命令

```bash
# ① 分项与房 $18 计数（纯 ROM，不跑游戏）
python3 - <<'PY'
from pathlib import Path
rom=Path('rom/Metroid (USA).nes').read_bytes(); prg=rom[16:16+0x20000]
base=lambda b:b*0x4000
def rd(b,a,n=1):
    o=base(b)+(a-0x8000); return prg[o:o+n]
def rd16(b,a):
    d=rd(b,a,2); return d[0]|(d[1]<<8)
def stream(b,a):
    o=base(b)+(a-0x8000); return prg[o:prg.index(0xFF,o)+1]
ROOM,STRUCT=0xA314,0xA372
def calc(i):
    s=stream(1,rd16(1,ROOM+2*i)); pal=s[0]; j=1; objs=[]
    while s[j]!=0xFF:
        if s[j]==0xFE: j+=1; continue
        if s[j]==0xFD: j+=1; break
        objs.append((s[j],s[j+1],s[j+2])); j+=3
    en=0
    while s[j]!=0xFF:
        ty=s[j]&0xF
        if ty==1: en+=1; j+=3
        elif ty in (2,4): j+=2
        elif ty==6: j+=1
        elif ty==7: j+=3
        else: j+=ty if ty in (0,3,5) else 1
    fast=full=rows=yu=0
    for pos,si,opal in objs:
        yu+=pos>>4
        st=stream(1,rd16(1,STRUCT+2*si)); k=0
        while k<len(st) and st[k]!=0xFF:
            h=st[k]; k+=1; rows+=1
            c=16 if (h&0xF)==0 else h&0xF
            if opal==pal: fast+=c
            else: full+=c
            k+=c
    w=1300+9262+len(objs)*180+yu*20+rows*30+fast*214+full*349+en*300
    return w,len(objs),rows,fast,full,yu,en
for i in (0x18,0x14,0x13,0x09,0x08):
    w,no,r,f,fl,yu,en=calc(i)
    print(f"room {i:#04x}: work={w} obj={no} rows={r} mt_fast={f} mt_full={fl} yu={yu} enemy={en}")
print("3 帧门槛 =", 3*29780-28454)
PY

# ② 门径窗实测（两侧同口径；rust 侧用 --trace load 看 work/phase）
python3 tools/tas_replay_diff.py replay \
  --inputlog movies/lordtom-metroid-100p.inputlog --frames 2200 \
  --range 0400-04FF --out /tmp/f1301-cmp --allow-rust-missing
```

期望输出（本次实跑）：`room 0x18: work=57132 obj=16 rows=46 mt_fast=70 mt_full=70 yu=100
enemy=3`、`room 0x14: work=31446`、`room 0x13: work=36620`、`room 0x09: work=43378`、
`room 0x08: work=71946`、`3 帧门槛 = 60886`。前三/后两点与校准文档逐点相等（说明脚本
与 rust 累加器同口径）；房 `$18` 的 57132 是**结构流上界**，实测 55308（差 1824，§2.1）。

## 7. 边界与未覆盖

- 本文只给「计数表 + 阈值 + 预测」，不改代码；`crates/` 零改动。
- `work_cycles` 是 rust 模型量，**没有 emu 直接读数**；房 `$18` 的实测值来自 rust 侧
  `--trace load`（两处工件见 §0），等价性靠四个既有实测工作点回归。
- 结构流上界与实测的 1824 差因**未定谳**（§2.1 已排除 `metatile_out_of_bounds`），列 C4。
- `LOAD_SHELL` 的 1300 仍未分解成 `DeleteOffscreenRoomSprites`/`ScanForItems` 两条的逐条
  周期——这正是 §4.2 的 ≥3646 缺口无法单点定谳的原因。要定谳需给这两个例程建
  `model_*`（写面在 `frame/cycles.rs`，属后续卡）。

## 8. 假设区（C 级）

| 编号 | 假设 | 推翻方法 |
|---|---|---|
| C1 | `PAGE_CROSS_METATILE=6` 低估了真实跨页次数（每元 tile 有 5 次 `(zp),y` 读，最坏 +5） | 在 `DrawMetatile` 入口/出口用 Mesen cycle 计数，或按指针实值逐次判跨页 |
| C2 | `UpdateAttrib` 全径 `SHIFT` 平均轮数 > 2（房 `$18` 的 70 个 full 元 tile） | 同上，按属性位位置统计 `@loop` 实际迭代数 |
| C3 | `LOAD_SHELL` 的真实周期 ≥ 4946（`DeleteOffscreen`+`ScanForItems` 的逐条和） | 给两条例程建 `model_*` 并与 ROM 指令逐条对账 |
| C4 | 房 `$18` 的实测比结构流上界少 1824，是门径载房对该房流的某种少算（例如某对象在门径下未被 `DrawObject` 处理、或结构流提前结束） | 在 `draw_room`/`draw_struct` 的每个对象/行入口打 `--trace load` 计数，对比本文 16 对象/46 行/140 元 tile；若计数一致则转向 C1/C2 |
