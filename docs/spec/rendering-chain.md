# spec：渲染管线链（task-42 考证产物——S6 排程基准）

> 状态：**考证定谳**（2026-10-17）。asm-first 铁律：m1disasm prg7_engine/prg6_graphics
> 为主文档（内容锚=bank:addr 指令流）；game-data GFXInfo/VRAM 结构面互证 ✓
> （parse_vram_struct/parse_gfx_info 同源核）。像读源码精读面=PrepareVram 家族/
> WritePPUByte/CheckVRAMStruct/CHR-RAM 上传序/$2007 写序全链。

## 1. 渲染链总架构（NMI 冲刷序=A 级指令流锚）

```
【游戏线程侧·生产者】
  静态加载（标题/区域/房间）──┐
  运行时更新（瓦片爆破/门/道具）─┤→ VRAMStructBuffer($07A1-$07EF, 79B 上限)
  调色板变更（PaletteDataPending）─┘   + PPUDataPending 标志
【NMI 侧·冲刷（07:C0D9 严格序）】
  1. OAMADDR=0 + OAMDMA（页2 SpriteRAM→OAM——精灵面先走）
  2. NMIStatus==0（帧及时完成）时：
     a. NMIScreenWrite（$9A07，非 Play 模式尾消息）
     b. CheckPaletteWrite($C1E0)   ←调色板（PalettePtrTable→同构结构）
     c. CheckVRAMStructBufferWrite($C2CA) ←VRAM 缓冲冲刷
     d. WritePPUCtrl($C44D)   ←$2000/$2001
     e. WriteScroll($C29A)   ←滚动寄存器
     f. ReadJoyPads($C215)
  3. SoundEngine / UpdateAge
  4. NMIStatus=1（尾写——spin 出口缝 $1A 面同源）
```

## 2. 核心组件契约（A 级）

### 2.1 VRAMStruct 格式（与 game-data parse_vram_struct 互证 ✓）
```
chunk = [hi][lo][ctrl][data...]   // hi=0=终止符（落 WriteScroll）
ctrl: bit7=PPU 增量（0=+1, 1=+32）| bit6=RLE | bits0-5=长度(1-63)
data: RLE=1 字节×len；非 RLE=len 字节顺序
```
- 解码器 VRAMStructWrite($C30C)：逐 chunk PPUADDR=hi,lo→SetPPUInc(增量)→
  PPUDATA 循环（RLE/直写）→AddYToPtr00 步进→hi==0 时**落 WriteScroll 尾跳**；
- 生产者 WritePPUByte($C36B)：尾插字节入缓冲（index X）；**缓冲满=$4F 自截断**
  （EndVRAMStruct: 补 0 终止+PLA PLA 栈弹出中止=溢出安全）；
- WriteVRAMString($C328)：瓦片块写手（宽×高→逐行 [hi][lo][len][tile…]，
  行间 +$20（32 增量）=名表换行语义）。

### 2.2 CHR-RAM 上传（07:C7AB LoadGFX + CopyGFXBlock）
```
GFXInfo 表（7B/项：[bank][src lo/hi][dest lo/hi][len lo/hi]）
  → Y=头索引 → 偏移=7Y+6 → 7 字节拷 $00-$06（Bank/Source/Dest/Size）
  → BankSwitch(源 bank) → PPU 增量=1 → CopyGFXBlock:
     PPUADDR=dest → ROM→PPUDATA 逐字节（16 位长度计数）
     每 256B 页步进（inc src/dest 高字节）→ BankSwitch 回
```
- dest 面=图案表 $0000-$1FFF（CHR-RAM 8KB：BG $0000/$1000 分区+精灵）；
- GFXInfo 表=g**ame-data parse_gfx_info 同源**（task-17 已考面互证 ✓）；
- prg6_graphics=CHR 数据仓（incbin .chr 库——samus/区域 BG/字体/敌人全表）。

### 2.3 房间名表上传（07:C86F AreaInit 面）
```
SetupRoom 循环（预渲染房间→WRAM $6000 Room RAM）
  → $00=RoomRAMPtr → PPUADDR=$2000（名表 0）→ 4 页×256B 直拷 PPUDATA
  = 名表+属性表整块（$2000-$2FFF 1KB）
```
- 房间 RAM 构成=game-data 四级链（room→structure→metatile→tile）✓ 互证；
- 名表清屏=07:C195（填充环：名表+关联属性表同清）。

### 2.4 调色板链（07:C1E0 CheckPaletteWrite）
```
PaletteDataPending(1-8 索引) → PalettePtrTable×2 → PreparePPUProcess
  → VRAMStructWrite（静态结构指针——与缓冲冲刷共用解码器！）
```
- 结局序列旁路 EndGamePaletteWrite；intro 星闪=StarPaletteSwitch 周期切换。

### 2.5 寄存器面（WritePPUCtrl $C44D / WriteScroll $C29A）
- $2000/$2001 由 PPUCTRL_ZP/影子字节驱动；增量位=SetPPUInc 实时改写；
- 滚动=ScrollX/ScrollY/ScrollDir→$2005 双写（写序在 VRAM 冲刷后=关键时序）。

## 3. $2007 写序定谳（加载序总规则）
1. **图案表（CHR）先于名表**（LoadGFX 在 AreaInit/标题加载前段）；
2. 名表整块（4 页直拷）→ 属性表（含在 4 页内）；
3. 调色板（独立结构）；
4. 运行时增量（缓冲 79B/帧预算——超发=自截断保护）；
5. 每次 PPUADDR 双写前必读 PPUSTATUS（flip-flop 复位）——解码器首行实证。

## 4. S6 立卡拆分（定尺寸=估时）

| 卡 | 内容 | 尺寸 | 估时 | 依赖 |
|---|---|---|---|---|
| S6-1 | VRAMStruct 缓冲生产者（WritePPUByte/EndVRAMStruct/WriteVRAMString） | S | 0.5d | — |
| S6-2 | VRAMStructWrite 解码器+SetPPUInc（nmi.rs 已有雏形=移植补全） | S | 0.5d | S6-1 |
| S6-3 | CheckPaletteWrite+PalettePtrTable（调色板链） | S | 0.5d | S6-2 |
| S6-4 | OAM DMA 面（SpriteRAM 页→精灵状态建模） | M | 1d | — |
| S6-5 | WritePPUCtrl+WriteScroll（寄存器影子+写序） | S | 0.5d | S6-2 |
| S6-6 | LoadGFX/CopyGFXBlock（CHR-RAM 上传——GFXInfo 消费） | M | 1d | game-data GFXInfo |
| S6-7 | 房间名表上传（SetupRoom→4 页拷贝） | M | 1d | game-data 四级链 |
| S6-8 | 名表清屏（C195 填充环） | S | 0.25d | — |
| S6-9 | L4 视觉第二腿（渲染输出→PNG→diff_visual 集成） | M | 1d | S6-1..8 |
| S6-10 | 视觉回归全案（金面帧扩面+白名单机制复核） | S | 0.5d | S6-9 |
| — | **合计** | — | **~7d** | — |

- 现有基础：game-core nmi.rs vram_struct_write 雏形+game-data GFXInfo/VRAM 结构
  解析全好=L4 第二腿就绪度 30%；
- 风险项：S6-4 精灵面（8 sprites/scanline quirk——D4 bug 台账面）+S6-9 调色板
  映射（视觉白名单已有机制）。
