# findings：E37A 头部考证半——sub_E7AD/sub_D976 族语义考证（task-19）

> 状态：**考证半已交付**（2026-10-16）；移植半待移植线接（task-4 原卡）。
> 判据：A/B/C 分级、双工具交叉（nmikstas asm 内容锚+自建解码互核=ROM 字节对拍）、
> 字节级复现命令。命名一律 **C 级**（nmikstas 名=第三方，D5 不背书；消费走台账自证）。

## 0. 字节级复现命令（判据①，输出=刚打印原文）

```sh
python3 -c "
rom = open('rom/Metroid (USA).nes','rb').read(); b7 = rom[0x10+0x1C000:0x10+0x20000]
print('E7AD:', b7[0xE7AD-0xC000:0xE7AD-0xC000+12].hex(' '))
print('D976:', b7[0xD976-0xC000:0xD976-0xC000+12].hex(' '))
print('E37A:', b7[0xE37A-0xC000:0xE37A-0xC000+16].hex(' '))"
```
```
E7AD: a6 4b a9 00 38 fd 01 03 85 02 20 be
D976: a9 00 8d 07 03 85 7d a8 a2 50 20 86
E37A: ad 14 03 d0 26 a9 18 8d 16 03 ad 0d 03 18 6d 01
```

## 1. 双工具交叉（判据②）：nmikstas 内容锚 × ROM 字节逐址互核

| 址 | nmikstas 指令流（Bank07.asm 锚） | ROM 字节 | 互核 |
|---|---|---|---|
| $E7AD | `ldx PageIndex($4B) / lda #$00 / sec / sbc ObjRadY,x($0301,x) / sta $02 / jsr LE8BE` | `a6 4b a9 00 38 fd 01 03 85 02 20 be…` | ✓ 逐字节 |
| $D976 | `lda #$00 / sta SamusOnElevator($0307) / sta OnFrozenEnemy($7D) / tay / ldx #$50 / jsr LF186` | `a9 00 8d 07 03 85 7d a8 a2 50 20 86…` | ✓ 逐字节 |
| $E37A | `lda SamusGravity($0314) / bne + / lda #$18 / sta SamusHorzSpdMax($0316) / lda ObjectY($030D) / clc / adc ObjRadY` | `ad 14 03 d0 26 a9 18 8d 16 03 ad 0d 03 18 6d 01…` | ✓ 逐字节 |

RAM 别名锚（Metroid_Defines.asm，语义注=C 级）：$4B=PageIndex、$55=IsSamus、
$58=SamusDoorData、$71=UpdtngPrjctl、$7D=OnFrozenEnemy、$0301=ObjRadY、$0302=ObjRadX、
$0307=SamusOnElevator、$030A=SamusHit、$0320=ElevatorStatus。

## 2. sub_E7AD（C 级名 CheckMoveDown——「向下阻挡检测」注）

**入口契约**：X=PageIndex（对象索引）；**出口**：CF=1 畅通 / CF=0 撞（Exit16）。
**语义概要（B 级推导自指令流+注释锚）**：
1. **取负**：A=0−ObjRadY,x（`sec/sbc $0301,x`）→ $02（对象向下探距——task-4 卡
   「$301,X 取负」实锤）；`jsr LE8BE`（取对象位置 ObjectHi/Y/X → $0B/$08/$09）；
2. ObjRadX,x→$03；`$09−$03 and #$07` 对齐判定（8 像素网格）；`jsr LE8CE`（网格
   归一：取负/and 07/移位——坐标→房间 RAM 偏移）→ $04；`jsr LE90F`（径向展开）；
3. **对象↔背景碰撞循环**（LE7E6）：`jsr MakeCartRAMPtr(E96A)`（房间 RAM 指针）→
   取瓦片值（`lda ($04),y`）→ **瓦片分类语义（注释锚，B 级）**：
   - `<$80`：实心瓦=撞（CF=0 → Exit16）；
   - `$80-$9F`：可炸瓦（jmp IsBlastTile）；
   - `≥$A0`：可通行（IsWalkableTile）；特例 `$4E`（LE81E 分支——投射物更新面，
     UpdtngPrjctl/$71）；
   - **Samus 特例**（IsSamus/$55≠0）：$A0=卷动门/$A1=横卷门 → SamusDoorData($58)
     自增标志（门数据写集）；
4. 多格扫描循环（dex/jsr LE98E/jmp LE7E6）至 x=0 → `sec`（无撞）/rts。
**写集**：$02/$03/$04/$06/$07（帧临时）、$58 SamusDoorData（Samus 门特例）。
**出口面**：CF 双态 + rts（另有 LE81E→ClcExit 分支）。

## 3. sub_D976（C 级名 SamusOnElevatorOrEnemy——「站在电梯/冻敌上？」注）

**入口契约**：无参；**出口**：rts（结果落 RAM 双旗）。
**语义概要（B 级）**：
1. **双清**：SamusOnElevator($0307)=0、OnFrozenEnemy($7D)=0（task-4 卡「$307/$7D 清零」
   实锤）；
2. **敌槽扫描循环**：X=$50 起（`jsr LF186` 初始化槽游标）→ `EnStatus,x==$04`（冻结
   敌状态？C 级）→ `jsr LF152/LF1BF/LF1FA`（F1xx 族几何/命中判定）→ CF 通过 →
   `jsr LD9BA`（命中旗写）→ `inc OnFrozenEnemy`；`jsr Xminus16`（X-=16 槽步进）→
   `bpl --` 循环；
3. **电梯判定**：ElevatorStatus($0320)≠0 → `ldy #$00/ldx #$20/jsr LDC82`（DC82 族
   扫描）→ `jsr LD9BA` → `inc SamusOnElevator`；
4. **sub_D9BA（辅助）**：$10/$11/$04 判定 → SamusHit($030A) `|=（$10|0x40）`（掩码
   $38 保位）——命中旗写集单元。
**写集**：$0307/$7D（双旗）、$030A SamusHit（经 D9BA）、循环体内 F1xx 族间接写。
**出口面**：单 rts。

## 4. E37A 头部规则（重力/下落发起，spec 续节同步）

`lda SamusGravity($0314) / bne（升/降中→跳过）/ lda #$18 / sta SamusHorzSpdMax($0316)`→
`ObjectY($030D)+ObjRadY and #$07`（8 像素相位判定）→ `jsr CheckMoveDown / bcc`→
`jsr SamusOnElevatorOrEnemy`→若 SamusOnElevator≠0 或 OnFrozenEnemy≠0 跳过重力→
否则 `lda #$1A / sta SamusGravity`（下落重力值）→ X=#$05（下落极速）→
VertCntrLinear($?)溢出积分→ObjVertSpeed 递增（重力积分链）。
**规则**：下落发起=非升非降 + Y 相位合拍 + 无向下阻挡 + 非电梯/冻敌面 → 重力 $1A。

## 5. C 级命名候选（消费线索矿，D5 不背书）

| 候选名（nmikstas） | 址 | 自证状态 |
|---|---|---|
| CheckMoveDown | $E7AD | C（语义注+指令流吻合，待台账自证） |
| SamusOnElevatorOrEnemy | $D976 | C |
| MakeCartRAMPtr | $E96A | C |
| Xminus16 | （敌槽步进助） | C |
| UpdateStatues | $D9C6 族 | C（E37A 邻域） |
| F1xx 族（F152/F186/F1BF/F1FA/F1F4） | — | C（几何/命中判定族，逐函数待考） |

## 6. 分级汇总

| 结论 | 级 |
|---|---|
| 三址指令流↔ROM 逐字节互核 | **A** |
| 取负/双清/敌槽扫描/电梯判定指令流结构 | **A**（字节）+语义 **B**（注释锚+调用图） |
| 瓦片分类（<$80 实心/$80-9F 可炸/≥$A0 可行/$4E 特例） | **B**（注释锚+分支结构） |
| EnStatus==$04=冻结态、F1xx 族逐函数语义 | **C**（待考） |
| 命名候选全部 | **C**（待台账自证） |

## 7. 遗留（移植半输入）

1. F1xx 族逐函数考证（F152/F186/F1BF/F1FA/F1F4——batch-1 阻塞面）；
2. LE8CE/LE90F/LE98E/MakeCartRAMPtr 子链几何语义细化（网格归一/径向展开/多格扫描）；
3. EnStatus==4 语义自证（冻结敌判定）；
4. SamusDoorData 位语义（$58 高 4 位存 1/2 的注释→门类型）。
