# 横向滚屏地形列流考证（case-leftwalk-probe2 空列根因——A 级全链）

> 2026-10-28 · 先通读 m1disasm 全链后修改（用户裁定（禁试错））·
> Evidence A 级（bank:addr）Confidence 95

## 0. 症状（对拍证据）

case-leftwalk-probe2 左行长程：l380 diff=1191px 起→l580 stable ~9300px。
f01020_l580 图比：emu 左侧滚入地形列完整；rust **左侧新滚入列为空**（右侧旧列+敌人正常）。

## 1. 列流全链（asm 通读（A 级））

| 件 | 址 | 语义 |
|---|---|---|
| ScrollLeft | $E6A7（:7836） | ScrollDir 2/3 换向（3→2）；ScrollX==0 越界（dec MapPosX/GetRoomNum（门挡（ScrollBlock））/ToggleNameTable（PPUCTRL^=**3**））；dec ScrollX→**CheckUpdateNameTable** |
| CheckUpdateNameTable | $E54A（:7587） | SetupRoom 首件；**RoomNumber≠$FF⇒RTS**（装载任务未竟）；横（ScrollDir&2）⇒CheckUpdateNameTableHorizontal |
| CheckUpdateNameTableHorizontal | $E701（:7906） | ScrollX&7==阈（**右 0/左 7**）否则退；EndOfRoomHorizontal：**ScrollDir==TempScrollDir($4A)** 否则退（防重流）；Temp00=(ScrollX&0xF8)/8（列 idx）→UpdateNameTable |
| UpdateNameTable | $E590（:7652） | GetNameAddrs（PPU hi $20/$2C+RoomRAM hi $60/$64（GetNT(EB85)=(PPUCTRL^ScrollDir)&1））；控制位（横滚→**$1E\|$80**（PPU inc=32/长度 30））；VRAMStruct 缓冲写（addr+ctrl+30 瓦（RoomRAM[idx+row*32]→PPU[$20/$2C:idx+row*32]））→PPUDataPending←1（NMI 刷） |
| EndOfRoom | $EAF4（:8725） | RoomNumber←$F0；**TempScrollDir($4A)←ScrollDir**；and#2/bne→EndOfRoomHorizontal/Vertical（**首列/行流送 kick**） |
| GetRoomNum | $E720（:7934） | 横向：A=1左/2右；OnNameTable0→Y；**ScrollBlockOnNameTable3[y]&A⇒carry（门挡（停滚））**；否则图位（MapY*32+MapX→$7000+idx）→RoomNumber($5A) |
| ToggleNameTable | $E9B7（:8474） | PPUCTRL_ZP **^=3**（双位翻） |
| AttribTableWrite/RoomFinished | $EA13/$EA26（:8556-8574） | RoomNumber $F0-$F3 渐写（and $0F/inci）→$F4=RoomFinished→**$FF**（复位（流送恢复）） |

## 2. rust 侧现状（对读）

- check_update_name_table（physics.rs:1102（V2-lite 直写 PPU（金径=VRAMStruct+NMI
  （行级等价注）））：头/阈/防重/列写（idx=(ScrollX&0xF8)>>3/30 行（dst+idx+row*32））
  ——**与 asm 同形**。
- toggle_name_table=^3 ✓；scroll_left/scroll_right（E6A7/E6D2）✓；get_room_num ✓；
- attrib_table_write（$F0→$FF 渐写（RoomFinished）✓）。
- **根因（已证实）**：`end_of_room`（room.rs:457-459）**只写 RoomNumber←$F0，
  漏写 TempScrollDir($4A)←ScrollDir**（asm $EAF4 内件）——$4A 全树仅测试写站
  （physics.rs:1342）——gameplay 中 $4A=0 ⇒check_update_name_table 防重判
  `dir!=$4A`（2≠0）**永真跳过 ⇒列流永锁 ⇒左列空**（症状全释）。
- 次缺口：EndOfRoom 的首列/行流送 kick（EndOfRoomHorizontal/Vertical）未随件
  （修后≤8px 延迟列（可接受）或随修（本卡随修））。

## 3. 修复面（TDD）

1. end_of_room：补 `TempScrollDir($4A)←ScrollDir`（根因修）+首列 kick（EndOfRoom-
   Horizontal 复用（check_update_name_table 列写段抽出共用））。
2. 测：$4A 写面/长程左行列流（render.nametable 列实文）/防重判（dir≠$4A 跳）/
   门链 case-002 五点守（回归面）。
3. 判据：case-leftwalk-probe2 l580-l1400 diff 显著收敛（地形列补齐）+L4 ≥20/35
   无回退+门链五点保持+shell 左行随滚随载。

Evidence: prg7_engine.asm:7836-7913（ScrollLeft/Right）/7587-7652（CheckUpdate/
UpdateNameTable）/7906-7934（Horizontal/GetRoomNum）/8474（Toggle）/8556-8574（attr
任务机）/8725（EndOfRoom）+physics.rs:1102-1210+room.rs:457——A 级原文。
Confidence: 95
