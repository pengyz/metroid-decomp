# refs/m1disasm 整体架构 survey（全局地图·架构指导文档）

> 2026-01 盘点（time-stamp 2026-11-04，Asia/Shanghai）· 任务：全局通读式架构图，
> 纠正「只从局部入手容易偏离」。**纯考证不改产品码**；本文是导航图，不是行为规格。
> 复核状态：**待对抗复核**（verifier 证伪立场独立重导证据后方可作为任务卡地图消费）。
>
> **证据分级**（AGENTS.md §3）：
> - **A 级** = 机械统计/汇编原文：文件清单、行数、`bank:addr` 注址、include 链、
>   section/orga 指令、RAM `=`/`.enum` 赋值。复现命令见 §0。
> - **B 级** = 头注 + 入口例程 + 调用语境推断（职责定性、归属判定）。
> - **C 级** = 假设，显式标注「【假设】」，只进假设区，不得据以下任务卡结论。
>
> m1disasm 第三方命名**不背书**（AGENTS D5）：下文例程名全部是 refs 符号，
> 语义结论引用前必须自证入台账。

## 0. 调研口径与机械复现

- 通读粒度：每文件头注 + 段标（`;----[ ... ]----` 横幅）+ 例例行清单 + 关键文件抽读；
  不逐行精读。行数/例程数为机械统计。
- 复现命令（A 级）：

~~~sh
# 文件清单 + 行数（本文 §2 数据源）
wc -l refs/m1disasm/SRC/*.asm refs/m1disasm/SRC/*/*.asm refs/m1disasm/SRC/songs/*/*.asm
# 例程标签数（顶层 Label:，含 {AREA} 后缀与缩进标签）
grep -cE "^[ \t]*[A-Za-z_][A-Za-z0-9_.]*(\{[A-Za-z0-9_]+\})?[ \t]*:" <file>
# bank 组装顺序 = SRC/linkfile（wlalink 对象序）+ build.py（wla-6502 逐 bank 汇编）
# 覆盖热图数据源 = docs/findings/*.md 内 `bankN:XXXX`/`bankN:$XXXX` 引证计数
~~~

- 版本锚点：`refs/` submodule 钉死 = 等价性验证所用版本（rom-disasm-equivalence.md）。
  本文全部结论只对当前 submodule checkout 负责。
- 构建：`build.py` 用 **WLA-DX（wla-6502 + wlalink）** 逐 bank 汇编 8 个 `prg*.asm`
  → `out/*.o` → `linkfile` 链接出 5 个目标 ROM（NES_NTSC/PAL/MZM/MZM_G/CNS），
  MD5 与 vanilla 比对。**本项目只消费 NES_NTSC**（D6）；`BUILDTARGET` 条件汇编
  （`.if BUILDTARGET == "NES_NTSC"` …）在多处按目标变体切数据（PAL 有代码级差异，
  如 `GotoSoundEngine` 桩）。

---

## 1. Bank 布局全景（8×16KB）

### 1.1 映射模型（A 级：memory_layout.asm + prg*.asm `.section` 指令）

~~~text
NES CPU 空间                     ROM 构成（PRG 128KB = 8×16KB，MMC1）
$0000-$07FF WRAM                 bank0  ─┐
$6000-$7FFF Cart RAM（存档）      bank1   │  任一时刻一个 bank 映射进 $8000-$BFFF
$8000-$BFFF ROMSwitchSlot ←┐     bank2   │  （"窗口 bank"，MMC1 PRG 寄存器切换）
$C000-$FFFF ROMFixedSlot ──┘     bank3   │
        固定 = bank7             bank4   │
                                 bank5   │
                                 bank6  ─┘  （bank6=graphics 例外，见 §1.3）
                                 bank7  ─── 固定映射 $C000-$FFFF
~~~

- `memory_layout.asm`（19 行）：WLA-DX `.memorymap`（HeaderSlot/RAMConsoleSlot $0000-$07FF/
  RAMCartSlot $6000/ROMSwitchSlot $8000/ROMFixedSlot $C000）+ `.rombankmap`（8 banks × $4000）。
  **这是全部地址归属的单一声明点**。
- 8 个 bank 源文件 = `prg0_title … prg7_engine`，每个是一个 WLA section：
  bank0-6 `bank N slot "ROMSwitchSlot" orga $8000 force`；bank7 `slot "ROMFixedSlot" orga $C000 force`。
- **组装方式（linkfile + build.py）**：`SRC/linkfile` 只声明 `[header] SRC/header.bin` +
  `[objects]` 8 个 `out/prg*.o` 顺序；`build.py` 对每个 bank 跑
  `wla-6502 -D BUILDTARGET=... -o out/<bank>.o SRC/<bank>.asm`，
  再 `wlalink -C -S SRC/linkfile out/M1_<target>.nes`，比对 MD5。
  **bank 组装顺序 = 源文件物理顺序 + section 强制 orga**；没有中心 linker script 地址表，
  地址契约由各 section 的 `orga` 常量固化。

### 1.2 八 bank 职责定位 + 体量（行数/例程数 = A 级统计；职责 = B 级）

| bank | 源文件 | 映射 | 职责（B 级） | 主文件行数 | 展开行数† | 顶层例程标签 |
|---|---|---|---|---|---|---|
| 0 | `prg0_title.asm` | $8000 窗口 | 标题/开场动画/密码/结局演出/世界地图 + end·intro 曲 + sound_engine 副本 | 5,326 | 12,600 | 311 |
| 1 | `prg1_brinstar.asm` | $8000 窗口 | Brinstar 区域 bank（区域数据+敌 AI+brinstar 曲） | 1,071 | 14,055 | 120 |
| 2 | `prg2_norfair.asm` | $8000 窗口 | Norfair 区域 bank | 874 | 13,688 | 123 |
| 3 | `prg3_tourian.asm` | $8000 窗口 | Tourian 区域 bank（Mother Brain/zebetite/rinka 特化 + escape/mthr 曲） | 2,397 | 14,287 | 188 |
| 4 | `prg4_kraid.asm` | $8000 窗口 | Kraid 藏身处区域 bank（ridley+kraid 曲） | 1,001 | 13,404 | 120 |
| 5 | `prg5_ridley.asm` | $8000 窗口 | Ridley 藏身处区域 bank（ridley+kraid 曲，与 bank4 同曲集） | 915 | 13,251 | 123 |
| 6 | `prg6_graphics.asm` | $8000 窗口 | **纯 CHR 图形数据 bank**（GFX_* 标签即图块段，无游戏逻辑） | 569 | 2,601 | 22 |
| 7 | `prg7_engine.asm` | $C000 固定 | **游戏引擎**（主循环/NMI/萨姆斯/敌系统/房间/门/碰撞/绘制/掉宝/图块爆破…） | 12,930 | 14,962 | 972 |

† 展开行数 = 递归展开 `.include` 后的汇编输入总行数（含共享 include 在每个 bank 的一份）；
全 SRC 逻辑总量 53,568 行 .asm + 约 60 个 .chr 二进制。
共享 include 造成的重复：`areas_common.asm`(1,750)+`common_sprite_data.asm`(1,635) 进 bank1-5；
`sound_engine.asm`(2,721) 进 bank0-5；`reset.asm`(76) 进全部 8 bank；
`hardware.asm`(139)+`constants.asm`(179)+`macros.asm`(289) 进全部 8 bank（纯定义无码）。

### 1.3 每 bank 内部段图（A 级：section orga + 注址标签；跨 bank 契约点为 B 级）

**区域 bank（bank1-5）统一模板**（`DOC/area banks.txt` 声称约 47% 逐 bank 相同——社区测量值，
本 survey 未独立复算，标 C 级引用；但 common/专属分界由 m1disasm 源结构直接证实）：

~~~text
$8000-$856B  areas_common.asm：CommonJump_* 跳板表($8000 起 25×jmp)、
             CrawlerMovementRoutinesTable($8048)、EnemyMove 族($8058-$856B 敌移动/加速度/逐像素移动)
$8572-$8B13  common_sprite_data.asm（经 areas_common.asm:1311 include）：
             ObjectAnimIndexTbl($8572)/ObjFramePtrTable($860B)/放置表($86FD)/帧表($87CB)
$8B13-$8D5F  areas_common.asm 门族：SamusEnterDoor($8B13)/UpdateAllDoors($8B79)/DrawDoor($8BB1)/DoorXTable($8D3A)
$8D60-$955F  区域 CHR 数据（2KB：$8D60 一段 + $9160 一段，如 brinstar=end_font+sprite_tiles）
$9560-$97D1  ★固定契约区（bank7 引擎按固定偏移读取）：
             PalettePtrTable($9560)、SpecItmsTblPtr($9598)、AreaPointers_ROM($959A 7 项
             room/struct/metatile/enemy-anim 指针)、Tourian 跳表($95A8，非 Tourian=RTS 桩)、
             AreaRoutine($95C3)、TwosComplement($95C6)、区域 init 数据($95CC-$95E4)、
             ChooseEnemyAIRoutine($95E5 16 项 AI 跳表)、敌属性表族($960B-$97AF：
             死亡动画/血量/休眠动画/活动动画/$968B 位旗/加速度/速度/移动指针)
$97D1-...    自由区（区域内可浮动）：EnemyMovementChoices/移动串($97D1+)→敌 AI 代码
             (enemies/*.asm)→敌动画数据(enemy_sprite_data.asm)→palettes→room_ptrs→
             structure_ptrs→items→rooms→structures→metatiles→songs→(残留图块)
$B200-       sound_engine.asm 副本（含按 BANK 条件汇编的 SongHeaders 指针）
$BFB0        reset.asm 副本（ROMSWITCH_RESET）
$BFFA        3×.word 向量（指向 bank7 的 NMI $C0D9 / 本 bank reset）——$BFFA 向量是
             **死字节**（CPU 只从 $FFFA 取向量），原样复刻 ROM 构建残留（B 级推断）
~~~

**bank0（title）段图**（A 级：段标+注址）：

~~~text
$8000  MainTitleRoutine（TitleRoutine 索引 JumpEngine 分发）
$8068- 开场动画族（ClearSpareMem/METROIDFadeIn/Sparkle/Crosshairs…）
$8AC7  IntroStarPaletteSwitch 族
$8B79  密码族（ProcessUniqueItems/CheckPassword/CalculatePassword/
       ValidatePassword/PasswordScramble/NARPASSWORDTbl/EnterPassword…）
$9560  PalettePtrTable（与区域 bank 同构的固定表！title bank 也带一份）
$9A07  NMIScreenWrite + 结局演出族（Ending routines 段标）
$A53E  WorldMap
$B000- 歌曲数据（end.asm/intro.asm）+ 残留图块
$B200  sound_engine 副本；$BFB0 reset 副本；$BFFA 向量
~~~

**bank6（graphics）**：`GFX_*` 22 段纯 `.incbin` CHR（$8000 Samus+items、$89A0 intro sprites、
$8BE0 title、$90E0 suitless、$9DA0 各区 bg、$B4C0 font…）+ reset/向量。**无 sound_engine 副本**。

**bank7（engine）段图**（A 级：段标横幅 + 注址；972 个例程标签）：

~~~text
$C000 RandomNumbers → $C01A Startup（MMC1 初始化/清 RAM）
$C0BC MainLoop → $C0D9 NMI（OAM DMA→palette/VRAM/scroll/pad→SoundEngine→UpdateAge）
$C114 GoMainRoutine（MainRoutinePtrTable $C141 10 项状态机）
$C158-C2C9 清 nametable/擦精灵/清 RAM/调色板/读手柄/计时器/JumpEngine($C27C)/算术
$C2CA-C3C6 PPU 写族（VRAMStructWrite $C30C/WriteVRAMString $C328/调色板串）
$C3D4-C4FB 数学/等 NMI/开关屏/PPU 镜像
$C4DE-C6E0 bank 切换（CheckBankSwitch/BankSwitch/MMCWritePrgBank/GoBankInit
           /BankInitTable $C51F + InitBank0-5 + InitXXXGFX 族 + GFXInfo $C6E0）
$C7AB-C800 LoadGFX/CopyGFXBlock（CHR 搬运）
$C801-C92A AreaInit/MoreInit/CopyAreaPointers/DestroyEnemies/SamusInit
$C92B GameEngine → UpdateAge/PrepareGameOver/PauseMode/GoPassword/SamusIntro
$CA23 SwitchBank/BankTable；$CA35-CB1C 存档族（未用）+ChooseEnding($CAF5)
$CB29 UpdateWorld（游戏世界主更新）
$CB73-CBFD 萨姆斯调色板 + SFX/音乐触发桩（SFX_* 一串 1 行入口）
$CC0D UpdateSamus → GoSamusHandler/ActionTable($CC87)/SamusStand/Run/Roll/Jump/PntUp
$D1EE-CF.. 开火族（FireSamusProjectile/导弹/波弹/冰弹）
$D3A8 SamusDoor / $D423 SamusElevator / $D41A SamusDead
$D4BF-D7A0 弹/导弹/波/冰/炸弹（UpdateSamusProjectiles/Bomb*）
$D7B3-D9D3 电梯族（ElevatorMove/Scroll/Fade/Stop）+ StartMusic($D92C)
$D9D4-DB16 石像/桥族（UpdateAllStatues/UpdateStatue_Raise/UpdateAllStatues_Bridge）
$DB17-DC8E 导弹切换/掉宝（UpdateAllPowerUps/MissileEnergyTank/MapScrollRoutine/AddItemToHistory）
$DC8F-E094 动画+精灵绘制（UpdateObjAnim/GetSpriteCntrlData/DrawEnemy/WriteSpriteRAM
           $DEE6/DrawSpriteObject $DF19/位移/可见性）
$E094 UpdateEnemyAnim；$E0C1-E1E0 DisplayBar/血条/HexToDec/BitScan
$E1F1-E6FE 门滚动/萨姆斯垂直水平移动/加速度/卷屏（ScrollUp/Down/Left/Right）
$E54A-E733 UpdateNameTable/属性表/GetRoomNum
$E770-EACA 碰撞点计算/IsWalkableTile/MakeRoomRAMPtr($E96A)/SetupRoom($EA2B)/
           DrawRoom($EAAA)/DrawObject/DrawMetatile($EF2B)
$EACA-EF.. EnemyLoop($EAD4)/LoadEnemy/GetEnemyType/LoadDoor($EB8C)/LoadElevator/
           LoadStatues/LoadPipeBugHole/ScanForItems($ED98)/Spawn* 族
$EF13-F033 DrawStructRow/DrawStruct/UpdateAttrib/InitTables/FillRoomRAM
$F034-F344 CollisionDetection 族（敌/门/弹/萨姆斯碰撞判定 + 反应）
$F345-FE00 敌系统主体（UpdateAllEnemies/UpdateEnemy/UpdateEnemyCommon $F410/
           冻结/捡拾/受伤/ExplodeEnemy/SpawnEnProjectile $F870/敌弹/Mellow/
           PipeBugHole/SkreeProjectile/ApplySpeedToPosition $FD8F）
$FDE3-FF.. Tourian 物件（UpdateTourianItems/CheckZebetite）+ TileBlast 族
           （UpdateAllTileBlasts/DrawTileBlast/TileBlastAnim $FF86）
$FFB0 ROMFIXED_RESET（=reset.asm）；$FFFA 向量（NMI=$C0D9 / RESET=$FFB0 / IRQ=$FFB0）
~~~

### 1.4 跨 bank 运行契约（B 级推断，A 级事实标注）

1. **窗口 bank 必须驻留 sound_engine**：NMI（bank7 固定）`jsr SoundEngine`（NTSC 目标，
   prg7_engine.asm:272）跳进 **$B3B4 = 窗口区**。sound_engine 副本存在于 bank0-5 的 $B200
   （A 级：include 结构），bank6/7 无副本 ⇒ **NMI 期间窗口不得是 bank6**（bank6 仅在
   CopyGFXBlock 装载期使用，须 NMI 关闭——推断，待 Mesen2 trace 佐证【假设】）。
   这是「每 bank 一份 sound_engine」重复的根因，也是 Rust 单份 sound 模块与 asm
   8 副本结构错位的根源（§6）。
2. **SongHeaders 按 BANK 条件汇编**（A 级：sound_engine.asm:2256+ `.if BANK == 3` 等）：
   同一 engine 副本里曲目指针表按 bank 不同——存在的曲指向本 bank 歌流，
   缺席曲指向 RAM 假地址（$0100/$0300/$0500/$0700，"point to the middle of RAM"）。
   歌流数据（songs/*.asm）是 per-bank 数据，曲集：bank0=end+intro；bank1=brinstar；
   bank2=norfair；bank3=escape+mthr_brn_room；bank4/5=ridley+kraid（同曲集）。
3. **固定契约区 $9560-$97D1**：bank7 引擎按固定偏移读区域 bank 数据（AreaPointers 复制到
   ZP $3B-$48，prg7 CopyAreaPointers $C8B0）。区域 bank 的这一段地址**不许浮动**，
   自由区才可浮动（DOC/area banks.txt + prgX 源结构互证）。
4. **bank 切换入口集中在 bank7**（$C4DE CheckBankSwitch/$C4EF BankSwitch/$C4FA MMCWritePrgBank/
   $C510 GoBankInit + BankInitTable $C51F + InitBank0-5）——每 bank 有自己的 init 入口表项。

---

## 2. 模块清单（SRC/ 全文件登记）

**统计口径**：行数=A 级（wc -l）；「码/数/混」=B 级（头注+段结构+标签密度）；
关键入口=前 3-5 个顶层标签（A 级引用，语义名属第三方命名 C 级）。
**命名纪律**：下表例程名=refs 符号（D5 不背书，自证后才入台账）。

### 2.1 根级文件（逻辑/定义/组装）

| 文件 | 行数 | 码/数/混 | 职责（B 级） | 关键入口/锚点 |
|---|---|---|---|---|
| `prg7_engine.asm` | 12,930 | 混（码为主+内嵌表） | 引擎 bank 全体；段图见 §1.3 | `RandomNumbers` 07:C000、`Startup` 07:C01A、`MainLoop` 07:C0BC、`NMI` 07:C0D9、`JumpEngine` 07:C27C |
| `prg0_title.asm` | 5,326 | 混 | title/intro/密码/结局/世界地图+曲+engine 尾 | `MainTitleRoutine` 00:8000、`ProcessUniqueItems` 00:8B79、`NMIScreenWrite` 00:9A07、`WorldMap` 00:A53E |
| `prg1_brinstar.asm` | 1,071 | 混 | bank1 骨架：固定契约区+移动串+组 include | `PalettePtrTable` 01:9560、`ChooseEnemyAIRoutine` 01:95E5、`EnemyMovementChoices` 01:97D1 |
| `prg2_norfair.asm` | 874 | 混 | bank2 骨架（同构） | 同上族（02: 地址） |
| `prg3_tourian.asm` | 2,397 | 混 | bank3 骨架 + **Tourian 特化码**（$95A8 跳表实体/metroid/rinka 语境） | `GotoSpawnMotherBrainRoutine` 03:95B4、`ChooseEnemyAIRoutine` 03:95E5 |
| `prg4_kraid.asm` | 1,001 | 混 | bank4 骨架（同构） | 同上族（04:） |
| `prg5_ridley.asm` | 915 | 混 | bank5 骨架（同构） | 同上族（05:） |
| `prg6_graphics.asm` | 569 | **纯数据** | CHR 图形 bank | `GFX_Samus` 06:8000、`GFX_Title` 06:8BE0、`GFX_Font` 06:B4C0 |
| `areas_common.asm` | 1,750 | 混 | 区域 bank 公共段：敌移动引擎+门族+跳板表 | `CommonJump_UpdateEnemyCommon` $8000、`EnemyMove` $8058、`SamusEnterDoor` $8B13、`DrawDoor` $8BB1 |
| `common_sprite_data.asm` | 1,635 | **纯数据** | 公共物件动画/帧/放置表（由 areas_common:1311 内嵌） | `ObjectAnimIndexTbl` $8572、`ObjFramePtrTable` $860B |
| `sound_engine.asm` | 2,721 | 混 | SFX+音乐引擎（$B200 起，6 bank 副本；SongHeaders 按 BANK 变体） | `SFXData` $B200、`SoundEngine` $B3B4、`InitializeMusic` $BF19、`SongHeaders` $BD31 |
| `reset.asm` | 76 | 码 | 公共 reset（sei/cld/等 2 次 vblank/复位 MMC1/jmp Startup）+构建残留字节 | `ROMSWITCH_RESET` $BFB0（bank7 复用为 `ROMFIXED_RESET` 07:FFB0） |
| `constants_ram.asm` | 1,336 | **纯数据** | RAM 布局+结构体定义（§5） | `.struct En`/`.struct Object`/`.enum $12` ZP/`.enum $0300` 物件槽 |
| `constants.asm` | 179 | 纯定义 | 枚举常量（物件类型/敌状态/动画 id 等）+charmap 引入 | `.enumid it_*` 族 |
| `hardware.asm` | 139 | 纯定义 | PPU/APU/MMC1 寄存器地址名 | `PPUCTRL`/`MMC1PRG` 等 |
| `macros.asm` | 289 | 纯定义 | 汇编宏：`SignMagSpeed`/`VRAMStructData`/`SongNote`/`PtrTableEntry`/`assertMsg` | （宏定义体） |
| `memory_layout.asm` | 19 | 纯定义 | WLA 内存图+ROM bank 图（§1.1） | `.memorymap`/`.rombankmap` |
| `linkfile` | 8 | 组装 | wlalink 链接清单（header + 8 objects） | — |
| `header.bin` | 16B | 数据 | iNES 头 | — |
| `charmap.tbl` | — | 数据 | 文本字符映射（.stringmaptable） | — |

### 2.2 enemies/（16 个敌 AI 代码文件——**共享源、per-bank 编译**）

| 文件 | 行数 | 例程标签 | 被哪些 bank include（A 级 include 链） | 例（第三方名，C 级） |
|---|---|---|---|---|
| `crawler.asm` | 195 | 11 | 1,2,4,5 | `CrawlerAIRoutine_{AREA}` |
| `kraid.asm` | 348 | 19 | 1,4 | `KraidAIRoutine_{AREA}`（brinstar 引用=崩溃 bug，DOC） |
| `metroid.asm` | 378 | 12 | 3 | `MetroidAIRoutine_{AREA}` |
| `ridley.asm` | 163 | 5 | 5 | `RidleyAIRoutine_{AREA}` |
| `rinka.asm` | 222 | 4 | 3 | （rinka 生成器） |
| `swooper.asm` | 136 | 7 | 2,5 | `SwooperAIRoutine_{AREA}` |
| `pipe_bug.asm` | 97 | 1 | 1,2,4,5 | `PipeBugAIRoutine_{AREA}` |
| `squeept.asm` | 103 | 4 | 2 | `SqueeptAIRoutine_{AREA}` |
| `dragon.asm` | 88 | 1 | 2 | `DragonAIRoutine_{AREA}` |
| `multiviola.asm` | 13 | 0† | 2,5 | （小） |
| `polyp.asm` | 64 | 2 | 2,5 | `PolypAIRoutine_{AREA}` |
| `skree.asm` | 71 | 5 | 1,4 | `SkreeAIRoutine_{AREA}` |
| `rio.asm` | 54 | 4 | 1 | `RioAIRoutine_{AREA}` |
| `ripper.asm` | 18 | 0† | 1,2,4 | （极小，无顶层标签） |
| `sidehopper.asm` | 32 | 5 | 1,4,5 | `SidehopperFloorAIRoutine_{AREA}` |
| `waver.asm` | 13 | 0† | 1 | （极小） |

† 小文件内标签多为缩进局部标签/宏展开，顶层标签计 0 或极少——体量以行数为准。
**标签后缀机制（A 级）**：`.redef AREA = {"BANK{BANK}"}`（prgX.asm:22）⇒ 同一源在 bankN
编译出 `*_BANKN` 符号，共享敌码按 bank 实例化。**这是否定「文件↔Rust 模块一对一」的关键事实**（§6）。

### 2.3 五区域子目录（每区同构 10 件套 + CHR 变体）

每区固定 10 件（A 级清单）：`enemy_sprite_data.asm`（敌动画/放置/帧指针表——数据）、
`items.asm`（特殊物品表——数据）、`metatiles.asm`（元块定义——数据）、`palettes.asm`（调色板——数据）、
`room_ptrs.asm`（房间指针表——数据）、`rooms.asm`（房间定义——数据）、
`structure_ptrs.asm`（结构指针表——数据）、`structures.asm`（结构体数据）、
`sprite_tiles.chr`（+`_cns` 变体）、`bg_chr*.chr`（+`_cns`/`_mzm` 变体）。

| 区 | rooms | structures | enemy_sprite_data | items | palettes | metatiles | room/struct ptrs | CHR 备注 |
|---|---|---|---|---|---|---|---|---|
| brinstar | 935 行/47 标签 | 307/50 | 1,159/222 | 162 | 123/28 | 91 | 68/72 | bg_chr_1；sprite_tiles |
| norfair | 995/46 | 305/49 | 1,351/202 | 252 | 123/28 | 88 | 67/70 | bg_chr_1+bg_chr_2 |
| tourian | 403/21 | 233/32 | 673/173 | 169 | 144/28 | 80 | 42/54 | bg_chr+**font_chr+zebetite_chr**（特化） |
| kraid | 846/38 | 249/39 | 1,084/220 | 132 | 116/28 | 43 | 58/61 | bg_chr_2+bg_chr_3 |
| ridley | 844/42 | 198/29 | 1,168/195 | 104 | 121/28 | 35 | 63/51 | bg_chr |

- **数据文件登记（重点）**：rooms/structures/room_ptrs/structure_ptrs/items/palettes/
  metatiles/enemy_sprite_data **全部是数据文件**（0 或极少可执行指令；`.byte`/`.word`/宏展开为主）。
  rooms 数据格式（rooms.asm 头注，B 级文档化+抽读证实）：首字节=属性表基色；
  随后 3 字节一组（`yyyyxxxx` 位置 + StrctPtrTbl 索引 + 属性字节）；尾部敌/门变长段，$FF 终结。
- **代码文件**：enemies/*.asm（AI 码）+ prgX 骨架内联段 + areas_common + prg7 + sound_engine + reset。
- **混合文件**：prg0（码+调色板数据+曲+地图数据）、prg1-5 骨架（契约区数据+移动串数据+include 序）、
  prg7（大码+内嵌小表如 ActionTable/DoorXTable/ExplodePlacementTbl 等）、sound_engine（码+表+曲头）。
- **CHR 资产**：common_chr/ 11 组（samus/items/title/font/intro_sprites/solid/exclamation/
  bg_CRE/bg_CRE_2/bg_boss_areas/samus_suitless，各带 `_cns`/`_mzm` 目标变体）；
  ending/ 2 组；五区域各 1-2 组。总计约 60 个 .chr 文件（1-2KB/个）。

### 2.4 songs/（24 个数据文件 = 12 曲 × NTSC/PAL）

`songs/ntsc/*.asm` 与 `songs/pal/*.asm` 各 12 首（brinstar/end/escape/fade_in/intro/
item_room/kraid/mthr_brn_room/norfair/power_up/ridley/tourian）。**纯数据**：
宏驱动音流（`SongRepeatSetup`/`SongNote "B3"`/`SongNoteLength`——宏在 macros.asm）。
曲流按 bank 聚合见 §1.4-2。样本体量：end 779 行、escape 360、brinstar 336、intro 316、norfair 180、
kraid 194、ridley 88、tourian 53、item_room/fade_in 47、power_up 43、mthr_brn_room 69。

### 2.5 DOC/ + scripts/（社区文档与工具，非汇编输入）

- `DOC/area banks.txt`（区域 bank 布局文档，本 survey §1.3 的对账源）、`enemies.txt`（敌型编号表）、
  `enemy systems.txt`、`sound engine.txt`。**注意**：DOC 是社区文档（其测量值引用标 C 级，
  结构性描述与源码互证处标 B 级）。
- `scripts/`：address_labels.py、force_labels_into_mesen.py、instruclower.py、
  print_rooms.py、romfile.py、transpose_song.py（辅助工具）。

---

## 3. 数据/代码分离分析

### 3.1 数据表组织边界（B 级定性 + A 级住址）

| 数据族 | 住址规律 | 物理位置 | 与代码关系 |
|---|---|---|---|
| 公共物件动画/帧/放置表 | 固定 $8572-$8B13（区域内不许浮动） | `common_sprite_data.asm`（独立文件，经 areas_common 内嵌） | **分离良好**：纯数据文件 |
| 门数据（DoorHitPointTable/DoorXTable） | $8BD1/$8D3A | 混在 `areas_common.asm` 尾部（门代码区内） | **混居**：码间插表 |
| 固定契约表（palette/area 指针/敌属性表族） | 固定 $9560-$97D1 | prgX 骨架**内联**（不独立文件） | **混居**：骨架文件=表+组 include 序 |
| 敌移动串/移动选择表 | $97D1 起（自由区起点） | prgX 骨架内联 | **混居**（紧邻契约表） |
| 敌动画/放置/帧指针 | 自由区（如 brinstar $9D6A+） | `<area>/enemy_sprite_data.asm` | **分离良好** |
| 房间/结构/元块/物品/调色板 | 自由区（room_ptrs 如 brinstar $A314；metatiles $AEF0） | `<area>/rooms|structures|metatiles|items|palettes|room_ptrs|structure_ptrs.asm` | **分离良好** |
| 曲流 | $B000 附近（自由尾） | `songs/{ntsc,pal}/*.asm` | **分离良好** |
| SFX/音乐表/曲头 | $B200-$BFxx 固定段 | `sound_engine.asm`（码表混居，SongHeaders 按 BANK 变体） | **混居** |
| CHR 图形 | $8D60-$955F（区域）/bank6 全体/散装 .incbin | `.chr` 二进制 + prg0/prg6 内联 incbin | **分离良好**（二进制资产） |
| RAM 布局/结构体 | — | `constants_ram.asm` | **分离良好** |
| prg7 内嵌小表（ActionTable/DoorXTable/ExplodePlacementTbl/PickupTypeTable 等） | bank7 各处 | prg7_engine.asm 码区内 | **混居**（表紧贴消费码） |

**分离程度评级（B 级综合）**：
- **区域内容（rooms/structures/sprites/palettes/items/metatiles/songs/CHR）= 分离良好**
  ——每类独立文件、独立子目录，数据面可以整体搬进 game-data 而不动码。
- **引擎内数据 = 混居**——bank7 的行为小表、门表、areas_common 的门表、sound_engine 的表
  与码同文件；但 m1disasm 已用段标+标签把表锚定（`XxxTable:` 独立标签），拆出难度低。
- **固定契约区是最大「伪混居」**：物理上写在 prgX 骨架里，逻辑上是纯数据契约
  （bank7 按固定偏移消费）——Rust 侧应视为**数据面**（game-data 区域表），不是逻辑。
- **结论**：数据/代码分离程度 **中上**——「按区数据」分离干净；「按引擎数据」混居但边界清晰；
  唯一结构性纠缠 = sound_engine 的 per-bank 曲头变体（`.if BANK`）与 {AREA} 实例化机制。

### 3.2 烘焙面（game-data 已提取的表）对照（A 级：crates/game-data 结构 + rom_tables.rs 注释）

| game-data 归宿 | 源表（asm） | 状态 |
|---|---|---|
| `rom_tables::TABLE_968B_BY_BANK`（8×128B 窗口参数化） | 敌位旗表 $968B（每区域 bank 一份） | 已烘焙（A9 更正后 8 bank 全量） |
| `rom_tables::ROM_VEC_BY_BANK` | 各 bank 末向量/表 | 已烘焙 |
| `tables::sprite_data::{object_anim_index_tbl, obj_frame_ptr_table,…}` | common_sprite_data $8572/$860B | 已烘焙 |
| `tables::enemy_type_flags_968b`、`door_hit_point_table` 等 | $968B/DoorHitPointTable $8BD1 | 已烘焙（asm-to-rust-completeness.md §1） |
| `area::{items, sprite_data, enemy_gfx, graphics}` | 五区 items/enemy_sprite_data/CHR | 已落（样本见 game-data-brinstar-sample.md） |
| `room_data::{mod, room_ram}` | rooms.asm/room_ptrs | 已落（L1 指针对拍 0xA441 先例） |
| `sound::{sound_data, music_tables, music_streams}` | sound_engine 表/songs | 已落 |
| `title::{palette_data, area_tables, gfx_data}` | prg0 调色板/表/CHR | 已落 |
| `constants::ram_layout` | constants_ram.asm | 已落 |

---

## 4. 入口例程导航表（「查 X 去哪」）

**消费纪律**：地址=A 级可直接定位；名字=refs 第三方符号（D5：自证后引用）。
文件行号以当前 submodule 为准。

### 4.1 状态机/帧骨架

| 查什么 | 去哪 | 文件:行 |
|---|---|---|
| 主循环/帧界 | `MainLoop` 07:C0BC | prg7_engine.asm:199 |
| NMI（每帧中断体） | `NMI` 07:C0D9 | prg7_engine.asm:233 |
| 状态机分发（MainRoutine 10 项） | `GoMainRoutine` 07:C114 / `MainRoutinePtrTable` 07:C141 | prg7_engine.asm:301/359 区 |
| 状态 id 常量 | `GameMode` $1D / `MainRoutine` $1E / `TitleRoutine` $1F | constants_ram.asm:.enum $12 |
| 通用跳表引擎 | `JumpEngine` 07:C27C | prg7_engine.asm:687 |
| 游戏世界主更新 | `GameEngine` 07:C92B → `UpdateWorld` 07:CB29 | prg7_engine.asm:2026/2494 |
| 暂停/密码/GameOver | `PauseMode` 07:C9B1 / `GoPassword` 07:C9C4 / `PrepareGameOver` 07:C9A6 | prg7_engine.asm:2138/2170/2126 |
| 标题/开场分发 | `MainTitleRoutine` 00:8000（TitleRoutine 索引） | prg0_title.asm:27 |
| 结局选择 | `ChooseEnding` 07:CAF5 | prg7_engine.asm:2439 |

### 4.2 萨姆斯处理径

| 查什么 | 去哪 | 文件:行 |
|---|---|---|
| 萨姆斯总入口 | `UpdateSamus` 07:CC0D → `GoSamusHandler` 07:CC1A | prg7_engine.asm:2705/2718 |
| 动作跳表 | `ActionTable` 07:CC87（run/roll/pntup 4 项） | prg7_engine.asm:2804 |
| 站/跑/跳/蹲/瞄准 | `SamusStand` 07:CC36 / `SamusRun` 07:CCC2 / `SamusJump` 07:D002 / `SamusRoll` 07:D0E1 / `SamusPntUp` 07:D198 | prg7:2735/2858/3467/3617/3736 |
| 开火/导弹/波/冰 | `FireSamusProjectile` 07:D1EE / `CheckHorizontalMissileLaunch` 07:D315 / `CheckHorizontalWaveBulletFire` 07:D359 / `CheckIceBulletFire` 07:D38E | prg7:3807/4003/4049/4077 |
| 移动/加速度/卷屏 | `SamusMoveVertically` 07:E2A9 / `SamusMoveHorizontally` 07:E31A / `VertAccelerate` 07:E37A / `HorzAccelerate` 07:E3E5 / `ScrollUp/Down/Left/Right` 07:E4F1/E519/E6A7/E6D2 | prg7:7060/7162/7263/7338/7519… |
| 血量/受伤/死亡 | `SubtractHealth` 07:CE92 / `AddHealth` 07:CEF9 / `IsSamusDead` 07:CE84 / `SamusDead` 07:D41A | prg7:3203/3282/3189/4157 |
| 门/电梯 | `SamusDoor` 07:D3A8 / `SamusElevator` 07:D423 / `ElevatorMove` 07:D83D | prg7:4092/4168/4890 |
| 萨姆斯↔实体碰撞 | `SamusCollisionWithSolidEntities` 07:D976 | prg7:5117 |

### 4.3 敌系统

| 查什么 | 去哪 | 文件:行 |
|---|---|---|
| 敌主循环 | `EnemyLoop` 07:EAD4 / `UpdateAllEnemies` 07:F345 / `UpdateEnemy` 07:F351 | prg7:8705/10488/10499 |
| 敌通用更新（跨 bank 跳板） | `UpdateEnemyCommon` 07:F410（经 `CommonJump_UpdateEnemyCommon` $8000） | prg7:10628 / areas_common.asm:21 |
| 敌 AI 分发（per-bank 16 项） | `ChooseEnemyAIRoutine` 01:95E5（各 bank 同址） | prgX 骨架（prg1:163 区） |
| 敌移动引擎 | `EnemyMove` $8058 族 + `EnemyGetDeltaY/X` $8244/$8318 + 逐像素 $83F5-$84FE | areas_common.asm:111+ |
| 敌动画 | `UpdateEnemyAnim` 07:E094 / `InitEnAnimIndex` 07:F68D | prg7:6612/11113 |
| 敌弹 | `SpawnEnProjectile` 07:F870 / `UpdateAllEnProjectiles` 07:F93B | prg7:11551/11700 |
| 敌属性表（血量/动画/位旗/速度） | $960B-$97AF 族（每 bank 同址） | prgX 骨架（prg1:168-240 区） |
| 单个敌 AI | `enemies/<name>.asm`（`{AREA}` 后缀符号） | enemies/*.asm |

### 4.4 门/房间/结构

| 查什么 | 去哪 | 文件:行 |
|---|---|---|
| 进门/门状态机 | `SamusEnterDoor` $8B13 / `UpdateAllDoors` $8B79 / `DrawDoor` $8BB1 | areas_common.asm:1317/1415/1450 |
| 门滚动 | `ScrollDoor` 07:E1F1 / `DoOneDoorScroll` 07:E22C | prg7:6891/6934 |
| 门生成 | `LoadDoor` 07:EB8C / `SpawnDoorRoutine` 07:EB92 | prg7:8867/8872 |
| 房间装载/绘制 | `SetupRoom` 07:EA2B / `DrawRoom` 07:EAAA / `DrawObject` 07:EA60 | prg7:8578/8666/8609 |
| nametable 更新 | `UpdateNameTable` 07:E590 / `WritePPUAttribTbl` 07:E5E2 | prg7:7652/7710 |
| 结构/元块绘制 | `DrawStruct` 07:EF8C / `DrawMetatile` 07:EF2B / `DrawStructRow` 07:EF13 | prg7:9715/9613/9590 |
| 房间数据/指针 | `<area>/rooms.asm` / `room_ptrs.asm`（`RoomPtrTable_{AREA}` 01:A314） | brinstar/room_ptrs.asm:20 |
| 物品扫描 | `ScanForItems` 07:ED98 / `AddItemToHistory` 07:DC54 | prg7:9255/5763 |

### 4.5 渲染/PPU

| 查什么 | 去哪 | 文件:行 |
|---|---|---|
| VRAM 结构化写 | `VRAMStructWrite` 07:C30C / `WriteVRAMString` 07:C328 / `CheckVRAMStructBufferWrite` 07:C2CA | prg7:856/886/790 |
| 精灵写入 | `WriteSpriteRAM` 07:DEE6 / `DrawSpriteObject` 07:DF19 / `EraseAllSprites` 07:C1A3 | prg7:6314/6341/443 |
| 调色板写 | `CheckPaletteWrite` 07:C1E0 / `PrepPPUPaletteString` 07:C37E / `SelectSamusPalette` 07:CB73 | prg7:507/963/2535 |
| 滚动寄存器/镜像 | `WriteScroll` 07:C29A / `SetPPUMirror` 07:C4B6 | prg7:717/1258 |
| CHR 装载 | `LoadGFX` 07:C7AB / `CopyGFXBlock` 07:C7D5 / `GFXInfo` 07:C6E0 | prg7:1748/1786/1669 |
| OAM/可见性 | `IsObjectVisible` 07:DFDF / `ObjDrawFrame` 07:DE4A | prg7:6495/6228 |

### 4.6 声

| 查什么 | 去哪 | 文件:行 |
|---|---|---|
| 引擎主入口（NMI 调用） | `SoundEngine` $B3B4 | sound_engine.asm:384 |
| SFX 数据/分发 | `SFXData` $B200 / `GetSoundRoutineData` $B289 / `GetSoundRoutine` $B4BD | sound_engine.asm:25/106/561 |
| 音乐引擎 | `InitializeMusic` $BF19 / `ContinueMusic` $BC4B / `UpdateMusicChannel` $BAC2 | sound_engine.asm:2666/2115/1786 |
| 曲头/歌流 | `SongHeaders` $BD31（按 BANK 变体）+ `songs/<region>/<song>.asm` | sound_engine.asm:2256 + songs/ |
| SFX 触发桩（bank7） | `SFX_*` 07:CB96-CBF3 / `SetSoundInitFlag` 07:CBE8 | prg7:2578-2699 |
| 曲目触发 | `StartMusic` 07:D92C / `PowerUpMusic` 07:CBF9 / `TourianMusic` 07:CC07 | prg7:5060/2681/2696 |

### 4.7 初始化/复位/bank

| 查什么 | 去哪 | 文件:行 |
|---|---|---|
| 复位 | `ROMSWITCH_RESET` $BFB0 / `ROMFIXED_RESET` 07:FFB0（= reset.asm） | reset.asm:26 |
| 开机初始化 | `Startup` 07:C01A | prg7_engine.asm:59 |
| 区域初始化 | `AreaInit` 07:C801 / `MoreInit` 07:C81D / `SamusInit` 07:C8D1 | prg7:1826/1843/1971 |
| bank 切换 | `CheckBankSwitch` 07:C4DE / `BankSwitch` 07:C4EF / `MMCWritePrgBank` 07:C4FA | prg7:1296/1314/1329 |
| bank init 表 | `GoBankInit` 07:C510 / `BankInitTable` 07:C51F / `InitBank0-5` 07:C531+ | prg7:1351/1363/1376+ |
| 标题初始化 | `InitializeAfterReset` 00:8071 | prg0_title.asm:104 |

### 4.8 随机数/工具

| 查什么 | 去哪 | 文件:行 |
|---|---|---|
| 随机数 | `RandomNumbers` 07:C000（RandomNumber1/2 = $2E/$2F） | prg7_engine.asm:33 |
| 除/乘 | `Adiv32/16/8` 07:C2BE / `Amul32/16/8` 07:C2C4 | prg7:759-780 |
| 十进制/位 | `Base10Add` 07:C3DA / `Base10Subtract` 07:C3FB / `HexToDec` 07:E198 / `BitScan` 07:E1E1 | prg7:1049/1072/6811/6872 |
| 二补数 | `TwosComplement` 07:C3D4 / `TwosComplement_` $95C6（区域 bank） | prg7:1038 / prgX 骨架 |
| 等 NMI/开关屏 | `WaitNMIPass` 07:C42C / `ScreenOff` 07:C439 / `ScreenOn` 07:C447 | prg7:1108/1123/1142 |
| 手柄 | `ReadJoyPads` 07:C215 / `ReadOnePad` 07:C21F | prg7:562/572 |

---

## 5. RAM 布局（constants_ram.asm + memory_layout.asm）

### 5.1 组织结构（A 级：`.struct`/`.enum`/`=` 赋值）

`constants_ram.asm`（1,336 行）四段式：
1. **Structs 段**（1-242）：`OAMSprite`(4B×$40)、`RinkaSpawner`、`SkreeProjectile`、`Mellow`、
   `TileBlast`、`PipeBugHole`、`PowerUp`、`Zebetite`、**`En`**(16B 敌槽)、`EnExtra`、
   `EnExplosion`、`EnExplosionExtra`、`Cannon`、`IntroSpr`、`AreaPointersStruct`(14B)、
   **`Object`**(16B 通用物件)。
2. **Zeropage 段**（244-800）：临时变量按「消费例程」分组重叠命名（Temp00-$11 多套语义复用，
   每组注释绑定例程名——**ZP 复用约定**是移植重灾区，见 §6）+ `.enum $12` 起固定变量
   （Joy/NMIStatus/GameMode/MainRoutine/CurrentBank/Timer/RandomNumber/RoomPtr/StructPtr/
   AreaPointers_RAM $3B-$48/ScrollDir/MapPos/SamusScrX/Y/DoorEntryStatus…）。
3. **Onepage $0106 段**：Health/EndTimer/MissileToggle 等。
4. **分页 RAM**：$0200 SpriteRAM（OAM 64×4B）→ $0300 Object RAM（**16 槽×16B**：
   slot0-1 Samus、2 Elevator、4 PowerUpDraw/Skree 弹、6-7 桥、8-B 门、D-F 萨姆斯弹；
   $0310 萨姆斯子像素/速度/加速度）→ $0320-$0370 电梯/标题/石像专用 →
   **$0400 Enemy RAM**（En 槽）→ $0500 → **$0600 声音引擎 RAM**（171 行）→ $0700 →
   **$6000 Cart RAM**（存档+更多敌 RAM+开场精灵）→ $7000/$77F0 尾段。
   （注：docs 引用「敌槽 16×16B $6AF4-$6BF3」= $6000 页里的敌槽实例，与 ram-region-6af4-contradiction 定谳一致。）

`memory_layout.asm` 只管**地址空间槽**（RAM/ROM 槽位），不含变量——两文件职责正交。

### 5.2 与代码的耦合面（B 级）

- **强耦合点**：`Object`/`En`/`EnExtra` 结构体布局是全部敌/物件码的访问基（`EnsExtra.0.status,x`
  语法=m1disasm 的结构化寻址宏）；ZP Temp 复用表把**例程名**写进变量名——
  RAM 语义与例程绑定，拆模块时 ZP 复用必须整组搬运（否则跨例程别名被拆散=行为漂移）。
- **状态机耦合**：GameMode/MainRoutine/TitleRoutine（$1D-$1F）是 §4.1 状态机的 RAM 锚。
- **NMI 同步耦合**：NMIStatus $1A / PPUDataPending $1B / PaletteDataPending $1C / SpriteLoadPending
  =帧契约（D7 未覆盖缺口「NMI 同步」显式登记项）。
- game-core 侧对应物：`ram_map.rs`（16KB）+ `memmap/*`（结构体域）——地址只在映射层（lib.rs 分层声明）。

---

## 6. 与 Rust 移植面的映射（crates/game-core ↔ asm）

### 6.1 域对齐表（B 级综合；bank:addr 引证=A 级）

| game-core 域 | 主要 asm 来源 | 对齐度 | 错位/偏离风险 |
|---|---|---|---|
| `frame/{mainloop,nmi,boot,clock,engine}` | prg7 $C000-$C155（Startup/MainLoop/NMI/GoMainRoutine）+ WaitTimer 族 | **对齐** | frame/engine 承担 asm「引擎骨干」，与 render 切分处=NMI 同步契约（跨模块交互缺口 D7） |
| `frame/{vram,sprite,gfx,render,room}` | prg7 $C2CA-C3C6（VRAM 族）+ $DD8B-E094（精灵）+ $C7AB（CHR）+ $EA2B（房间） | **对齐**（功能切分） | asm 单文件 prg7→Rust frame/render 双域；绘制与状态机共享 Object 槽语义 |
| `frame/{rng,statue,intro,title_bridge,sound_hook, memory}` | RandomNumbers $C000 / Statues $D9D4 / prg0 intro / 桥接 | 对齐 | title_bridge=**粘合层**（asm 里 title 与 engine 经 GameMode 直连，Rust 需显式桥） |
| `samus/{handler,physics,animation,control,integrators}` | prg7 $CC0D-D4A0（handler 族）+ $E2A9-E6FE（移动/加速度） | **对齐** | asm 一连续段→5 文件；**ZP Temp 复用组**跨这些例程共享（§5.2），拆分时别名被拆散=漂移风险 |
| `enemy/{system,ai,attrs,common,movement, projectile,slot,spawn_control,enemy_state…}` | prg7 $F345-FE00（系统）+ areas_common $8058-856B（移动）+ enemies/*.asm（AI）+ $95E5 表 | **部分对齐** | **错位①**：`enemies/*.asm` 是共享源 per-bank 实例化（`{AREA}`），Rust 现有 crawler/waver/mellow 等单份实现；kraid/ridley/metroid/rinka/skree/rio/ripper/sidehopper/swooper/squeept/dragon/polyp/pipe_bug **缺位**（gap.rs unported! 面）。**错位②**：`ChooseEnemyAIRoutine` $95E5 是 per-bank 16 项跳表=数据驱动，Rust enemy::ai 需按区域实例化 |
| `world/{door,room,camera}` | areas_common $8B13-8D5F（门）+ prg7 $E1F1-EACA（滚动/房间） | **对齐** | 门码物理上在「区域公共」层（areas_common），逻辑上属 world——按文件名找会错位到「区域域」 |
| `pickup/{items,powerup,spawn}` | prg7 $DB37-DC8E + `<area>/items.asm` | 对齐 | items.asm=数据面应归 game-data，不在 game-core |
| `weapon/{missile,tile_blast}` | prg7 $D4BF-D7A0（弹/炸弹）+ $FE1B-FFAA（TileBlast） | 对齐 | — |
| `sound/{music,sfx}` | sound_engine.asm $B200-BFxx（6 副本）+ prg7 SFX 桩 $CB96-CC0C + songs/ | **错位③** | asm=引擎 6 份副本+SongHeaders 每 bank 变体+曲集 per-bank；Rust=单份模块——**必须窗口 bank 参数化**（rom_tables 已按 8 bank 全量烘焙先例），不可默认「一份数据」 |
| `title/{gfx,interaction,sparkle,step_glue}` | prg0 $8000-8B78 | **错位④** | asm 文件 prg0 混装 title+intro+密码+结局+地图+曲+声尾；Rust title/ui/ending/sound 四域从同一 asm 文件不同区段切出——**文件↔模块不是一对一** |
| `ui/password` | prg0 $8B79-92xx（密码族） | 对齐 | 同上（从 prg0 切出） |
| `ending/sequence` | prg0 $9A07+（结局演出）+ prg7 ChooseEnding $CAF5 | 对齐 | 跨两 bank 两文件 |
| `input/joy` | prg7 ReadJoyPads $C215 | 对齐 | — |
| `memmap/* + ram_map.rs` | constants_ram.asm + memory_layout.asm | **对齐**（明确分层） | — |
| `gap.rs`（unported! 桩） | — | 缺口面 | fail-loud 钉 bank:addr（D7 缺口左移②） |
| game-data（数据面） | 区域 10 件套+CHR+songs+契约表+sound 表 | **对齐**（§3.2） | 唯一遗留=固定契约区/prg7 内嵌小表仍混在 asm 码文件（提取时勿漏） |

### 6.2 错位风险汇总（正是「局部入手容易偏离」的点）

1. **bank 副本 ≠ 多实现**：sound_engine/reset/{AREA} 敌码=同一逻辑多份实例化；
   按 asm 文件/标签一一移植会造出 6 份重复 Rust 或漏掉 per-bank 数据变体。
2. **文件归属 ≠ 逻辑归属**：门码在 areas_common（「区域」名下）实为 world 域；
   prg0 一个文件承载 4 个 Rust 域；prg7 一个文件承载 7+ 个域。
3. **固定契约区是数据不是逻辑**：$9560-$97D1 写在「代码文件」里，实际是 bank7 消费的数据契约。
4. **ZP 临时变量复用组**：变量名绑定例程名（Temp00_Diff/Temp00_FramePtr/… 同址多义），
   拆模块必须整组迁移或显式建别名层，否则静默漂移。
5. **{AREA}=BANK{n} 实例化 + SongHeaders `.if BANK`**：共享源的 per-bank 变体靠汇编期分支，
   Rust 侧对应物=按 window_bank 参数化的数据实例（rom_tables.rs 模式），不是条件编译逻辑。

---

## 7. 既有 findings 的地图化（覆盖热图）

**方法（A 级机械统计）**：扫 `docs/findings/*.md`（**265 份**）中 bank:addr 引证，
按 bank×1KB 聚合。**精确正则（复现唯一口径）**：`\b(?:bank)?0?([0-7])[:\s]+\$?([0-9A-Fa-f]{4})\b`
——覆盖 `bank7:C0BC`/`bank7 $C0BC`/`07:C0BC`/`07 $C0BC`/`7:C0BC` 全部书写形。
【2026-11-04 复核更正】初版仅写「bankN:XXXX 形」为欠精确口径描述（漏 `0?` 两位前缀形
`07:XXXX`——AGENTS 通行写法）——复核方以漏形正则复算得 5/4 等低值判 FAIL；以本精确正则
复算=本表数字逐格复现（bank1 $8800=473/55、bank7 $CC00=188/10、$DC00=21/6、$FC00=26/5
全对，F000 系 ±1=findings 滚动增长）。复现脚本（内联，可直接跑）：

~~~python
import re, glob
from collections import defaultdict
refs = defaultdict(lambda: [0, set()])
pat = re.compile(r'\b(?:bank)?0?([0-7])[:\s]+\$?([0-9A-Fa-f]{4})\b')
for f in glob.glob('docs/findings/*.md'):
    for m in pat.finditer(open(f, encoding='utf-8', errors='replace').read()):
        b, addr = int(m.group(1)), int(m.group(2), 16)
        k = (b, (addr >> 10) << 10)
        refs[k][0] += 1; refs[k][1].add(f)
for (b, kb), (c, fs) in sorted(refs.items()):
    print(f"bank{b} ${kb:04X}: {c}/{len(fs)}")
~~~

**口径限制**：裸 `$XXXX` 引证（bank 归属歧义）未计；引证密度≠深读深度（同址可被多文档
重复引用）；文件数=提及该区的文档数。热图数字=该区被 findings 引证次数 / 引证文档数。

### 7.1 热图（refs / files）

| 1KB 区 | bank0 | bank1 | bank2 | bank3 | bank4 | bank5 | bank6 | bank7 |
|---|---|---|---|---|---|---|---|---|
| $8000 | 9/5 | **786/27** | 63/21 | 12/4 | 6/2 | 18/6 | 13/6 | 9/5 |
| $8400 | 41/5 | **359/16** | — | — | — | — | — | 4/1 |
| $8800 | 5/4 | **473/55** | — | — | — | — | — | — |
| $8C00 | **84/3** | 12/3 | — | — | 3/1 | 4/3 | 6/3 | — |
| $9000 | 28/7 | 25/8 | 24/3 | 7/4 | 1/1 | 8/4 | 6/3 | 1/1 |
| $9400 | 8/3 | 1/1 | **60/3** | 18/3 | 13/4 | **77/6** | — | — |
| $9800 | 2/2 | — | — | 12/3 | 6/3 | **48/3** | — | — |
| $9C00 | 1/1 | 1/1 | — | — | — | — | — | — |
| $A000-$BFFF | 77-114/5（声） | 1-2 | — | 1 | — | — | 1 | 13/5 |
| $C000 | — | | | | | | | 66/14 |
| $C400 | | | | | | | | 47/7 |
| $C800 | | | | | | | | 80/9 |
| $CC00 | | | | | | | | **188/10** |
| $D000 | | | | | | | | **200/12** |
| $D400 | | | | | | | | 115/8 |
| $D800 | | | | | | | | 102/12 |
| $DC00 | | | | | | | | **21/6** |
| $E000 | | | | | | | | 93/7 |
| $E400 | | | | | | | | 60/11 |
| $E800 | | | | | | | | 105/12 |
| $EC00 | | | | | | | | 78/11 |
| $F000 | | | | | | | | **218/11** |
| $F400 | | | | | | | | **203/7** |
| $F800 | | | | | | | | 108/7 |
| $FC00 | | | | | | | | **26/5** |

（「—」=0 引证；bank0 $8C00 高值=密码/结局区；bank0 $B000-$BC00=sound_engine 引证；
bank1 $8000-$8800 高值=**公共区**（areas_common+common_sprite_data），引证常写作 bank1 地址。）

### 7.2 语义覆盖对照（findings 域审计 ↔ asm 区段）

| asm 区段 | 覆盖它的 findings（代表） | 深度 |
|---|---|---|
| 公共区 $8000-$8D5F（areas_common+sprite 数据+门） | s-face-full-implementation(906 引证)、454-case-evidence-*(898×2)、d-face-completeness-scan、d-face-data-table-analysis-*(60+ 份逐表)、door-system-audit、door-open-chain、anim-state-machine-model、objanim-offsets-reconciliation | **深** |
| bank7 帧骨架 $C000-$C500 | nmi-frame-evidence、execution-call-graph、engine-frame-rewrite-1/4、frame-alignment-*、game-rng-adjudication | **深** |
| bank7 萨姆斯 $CC00-$D400 | samus-jump-fire-chain、samus-movement-observations、samus-move3-scenario-package、morphball-probe-route、walk-fc-capture-seam、engine-mod-rewrite-block*(60+ 份) | **深** |
| bank7 门/电梯/石像 $D400-$DC00 | door-open-chain、elevator-branch-design、t45-1-statue-gate-verification、item-route-material | 中 |
| bank7 绘制/卷屏 $DC00-$E800 | scroll-column-streaming、scroll-content-mapping、display 链（enemy-display-chain-workorder） | **中偏浅**（$DC00 仅 21 引证=绘制动画段是相对空白） |
| bank7 房间/敌系统 $E800-$F800 | collision-family-enablement、enemy-*（crawler-trajectory、h3-enemy-slot-residual）、fc-phase-residual、decode-sweep-r3 | **深** |
| bank7 尾段 $F800-$FFFF（Mellow/PipeBug/Skree 弹/TileBlast） | tdd-mellow-empty-shell-cleanup-7、tdd-waver-empty-shell-cleanup-1 | **浅**（$FC00 仅 26 引证） |
| bank0 title/intro $8000-$8B00 | b-run-round2-title-clear、fade-blink-* | **浅**（intro 区仅 9-41 引证/文件 5-7） |
| bank0 密码/结局 $8B00-$A500 | ui-password-rewrite-block9、ending-sequence-rewrite-block12 | 中 |
| bank0 sound_engine $B200-$BFFF | sound-dispatch-blueprint、sound-domain-gap-workorder、audio-face-material、audio-fullload-sizing、b-run-round-w1-audio-clean | 中深 |
| bank1-5 区域专属 $8D60-$B1FF（CHR/契约表/敌 AI/rooms） | decode-sweep-r4-banks0-6、roomptrtable-crossref、game-data-brinstar-sample、boss-face-material、tourian-entry-path、banks06-batch-plan、bank06-pending-face | **浅-中**（每区 $A000-$B1FF rooms/structures 区近乎空白：0-2 引证） |
| bank6 graphics | bank06-pending-face、banks06-batch-plan | **空白**（纯数据，低风险） |
| 全 ROM 扫描 | decode-sweep-report(369)、decode-sweep-r2-jump-tables(191)、decode-sweep-r3/r4、reference-lead-atlas、rom-disasm-equivalence、s/d-face-completeness-scan | 横向全盖（切分裁定层） |

### 7.3 空白/薄弱区（结论，供任务卡排布）

1. **五区 rooms/structures/metatiles 数据段**（bank1-5 $A000-$B1FF）：引证≈0——
   解析器（game-data room_data）有先例但**行为消费侧**（房间装载行为）考据薄。
2. **bank7 $DC00-$E000**（UpdateObjAnim/GetSpriteCntrlData/DrawEnemy 绘制动画段）：21 引证——
   渲染链相对薄。
3. **bank7 $FC00-$FFFF**（TileBlast 尾/Mellow 尾）：26 引证。
4. **bank0 intro 区 $8000-$8B00**（开场动画状态机）：引证少。
5. **per-bank 敌 AI 码（enemies/*.asm）**：encode-sweep 走址未覆盖（reference-lead-atlas §5-2
   明言「m1disasm 无址标注文件（enemies/*.asm 等）未入走址」）——**全局地图上敌 AI 是
   最大无址标注盲区**。
6. **多区域对比面**：findings 几乎全是 bank1/7 视角，bank2/3/4/5 同构区的**差异部分**
   （各区独有表值）未逐区对拍。

---

## 8. 遗留假设与限制

1. 【假设】NMI 期间窗口 bank 恒为 sound_engine 驻留 bank（bank6 使用期 NMI 关闭）——
   B 级推断（include 结构+`jsr SoundEngine` $B3B4 语境），待 Mesen2 trace A 级佐证。
2. 【假设】bank0-6 `$BFFA` 向量块为构建残留死字节（CPU 只读 $FFFA）——B 级，
   reset.asm 的「Not used」注释旁证。
3. 「47% 逐 bank 相同」为 DOC/area banks.txt 社区测量（C 级引用，未独立复算）；
   common/专属分界 $8D60 由 m1disasm 源结构证实（B 级）。
4. 例程名一律第三方符号（D5）：本文未做任何 rename；语义消费前须自证入台账。
5. 热图口径限制见 §7.1；深读深度以 findings 明文为准，引证数仅作密度参考。
6. `{AREA}`/`.if BANK` 的全部变体点未逐条枚举（SongHeaders 已证；其他条件汇编点
   （BUILDTARGET 分支）与 NES_NTSC 无关部分未清点）。
7. 本文不覆盖 `reference/`（seed/src 轨道）与 nmikstas/MetroidMMC3（另见
   reference-lead-atlas.md）；只覆盖 `refs/m1disasm`。
8. DOC/enemy systems.txt、sound engine.txt 未逐行消化（§2.5 登记存在性）。

---

## 附：本文件消费方式建议

- **任务卡开卡前先查 §4 导航表定位例程、§1.3 段图定位契约区、§6.2 确认不在错位陷阱上**。
- 深读产出（findings）落图到 §7.2 表，保持热图可滚动更新（重跑 §0 命令即可刷新）。
- 对抗复核重点：§1.4-1（NMI/窗口 bank 契约）、§6.2（错位清单）、§7.3（空白清单）。
