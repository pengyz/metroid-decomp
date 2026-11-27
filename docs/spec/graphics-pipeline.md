> 状态：**取证中**（task #127，2026-11-27）。asm-first：`refs/m1disasm/SRC/prg7_engine.asm` 与各区
> `*/palettes.asm`、`prg6_graphics.asm` 为内容锚；本文是 Stage A 保真渲染的**上游契约**，
> 细化既有 [rendering-chain.md](rendering-chain.md)（该文给总架构与 S6 拆卡，本文给逐条写序与字节级证据）。
>
> 证据分级：**A**=字节/行号可直接复跑；**B**=调用图与数据流推断；**C**=假设（只进 §5）。

## 0. 结论摘要

1. **CHR-RAM 只在「大加载」时整块上传**：全 ROM 的 `LoadGFX` 调用点共 **52 处，全部落在 9 个
   初始化例程内**（标题/五区/结局/密码字库），**没有任何逐房间或运行时换图**。房间里看起来
   “换了图”的东西都走名表/metatile（§1.6），不是 CHR 上传。
2. 上传格式是 `GFXInfo` 表：**29 项 × 7 B** = `[bank][src lo/hi][dest lo/hi][len lo/hi]`；
   写入序 = 每 256 B 一个 `$2006` 地址对 + 连续 `$2007`，无 `$2005` 依赖，**依赖渲染关闭**
   （`ScreenNmiOff`/`ScreenOff`，§1.4）。
3. 调色板不是直接写 `$3F00`，而是**请求字节** `PaletteDataPending`（1-based 的调色板号），
   下一帧 NMI 里 `CheckPaletteWrite` 查 `PalettePtrTable` 后交给**与 VRAM 缓冲同一个解码器**
   `VRAMStructWrite` 落 PPU。每区 28 项（标题 19 项），但**每一项不是“固定 32+16”，而是若干
   VRAMStruct 拼起来的段序列**：`Palette00` 只写 BG `$3F00` 32 B（Brinstar/Norfair/Ridley/Kraid/
   Tourian 实测均为 1 段），精灵面由 `Palette01-04`（各 `$3F12` 2 B）、`Palette05`（`$3F14` 12 B；
   Kraid 为 `$3F11` 3 B，且 `Palette05-13` 九个标签同指 `04:A191`）、`Palette18-1B`（无装甲，`$3F10` 4 B×4）分别覆盖；
   每项以 `$00` 终止。
4. OAM 提交是 NMI 的**第一件事**：`OAMADDR=0` → `$4014←$02`（整页 256 B DMA）。生产者是主线程
   `UpdateWorld`：`SpritePagePos=0` → 各对象顺序画进 `$0200` → 剩余槽填 `$F4` 关闭。
5. **8 sprites/scanline 是硬件限制，ROM 没做排序/轮转**：一帧超过 8 个同扫描线精灵时由 PPU 丢弃，
   表现为闪烁/缺件；这是 D4 要保留的现象，**不是 Rust 侧的 bug**（登记方式见 §3.4，现挂
   `docs/bug-ledger.md` BUG-089 的「渲染/采样时序」条目，尚无独立条目）。

## 1. CHR-RAM 写入链

### 1.1 入口与调用面（A）

- 表：`GFXInfo: ; 07:C6E0`（`prg7_engine.asm:1669`）。
- 装载例程：`LoadGFX: ; 07:C7AB`（`:1748`）→ `CopyGFXBlock: ; 07:C7D5`（`:1786`）。
- **调用点归属（脚本计数，52 处/9 例程；下表「标签行」是例程标签所在行，第一处 `jsr` 在其下一行，例如 `LoadSamusGFX` 标签 `:1496`、首个调用 `:1498`）**：

| 例程 | 标签行 | 调用数 | 场景 |
|---|---|---|---|
| `InitTitleGFX` 07:C5D7 | `:1492` | 1 | 标题 |
| `LoadSamusGFX` 07:C5DC | `:1496` | 7 | 标题/结局的 Samus 与字体 |
| `InitBrinstarGFX` 07:C604 | `:1523` | 6 | 进 Brinstar |
| `InitNorfairGFX` 07:C622 | `:1542` | 7 | 进 Norfair |
| `InitTourianGFX` 07:C645 | `:1564` | 10 | 进 Tourian |
| `InitKraidGFX` 07:C677 | `:1595` | 8 | 进 Kraid |
| `InitRidleyGFX` 07:C69F | `:1620` | 7 | 进 Ridley |
| `InitGFX6` 07:C6C2 | `:1642` | 4 | 结局 |
| `InitPasswordFontGFX` 07:C6D6 | `:1655` | 2 | 密码画面 |

- 调用者只有三类：标题初始化（`07:C52B` 段，`prg7_engine.asm:1380-1393`）、换区（`InitBank1-5`，
  `:1398-1450`，`jsr Init*GFX` 后 `jmp NMIOn`）、结局（`:1490` `jmp InitGFX6`）、密码
  （`prg0_title.asm:2406/2806/2869`）。复跑：
  `python3 - <<'PY'` 遍历 `prg7_engine.asm`，按最近标签归属 `jsr/jmp LoadGFX`，应得 52/9。

### 1.2 `GFXInfo` 表（A，29 项 ×7 B）

字节实测（`python3` 从 ROM 读，bank7 线性映射 `16+7*0x4000+(addr-0xC000)`，表起 `$C6E0`）：

| # | bank | src | dest | len | 内容 |
|---|---|---|---|---|---|
| 0 | 6 | $8000 | $0000 | $09A0 | Samus/道具（精灵） |
| 1 | 4 | $8D60 | $0000 | $0520 | 结局 Samus |
| 2 | 1 | $8D60 | $1000 | $0400 | The End 字体（BG） |
| 3 | 6 | $9DA0 | $1000 | $0150 | Brinstar 房间 BG |
| 4 | 5 | $8D60 | $1200 | $0450 | 公共房间元素 CRE1 |
| 5 | 6 | $9EF0 | $1800 | $0800 | CRE2 |
| 6 | 1 | $9160 | $0C00 | $0400 | Brinstar 敌人（精灵） |
| 7 | 6 | $A6F0 | $1000 | $0260 | Norfair BG1 |
| 8 | 6 | $A950 | $1700 | $0070 | Norfair BG2 |
| 9 | 2 | $8D60 | $0C00 | $0400 | Norfair 敌人 |
| 10 | 6 | $A9C0 | $1000 | $02E0 | Boss 区 BG（Kr/Rd/Tr） |
| 11 | 6 | $ACA0 | $1200 | $0600 | Tourian BG |
| 12 | 6 | $B2A0 | $1900 | $0090 | Mother Brain 房（Zebetite） |
| 13 | 5 | $91B0 | $1D00 | $0300 | Tourian 字体/杂项 |
| 14 | 2 | $9160 | $0C00 | $0400 | Tourian 敌人 |
| 15 | 6 | $B330 | $1700 | $00C0 | Kraid 额外 BG |
| 16 | 4 | $9360 | $1E00 | $0200 | Kraid 额外 BG2 |
| 17 | 3 | $8D60 | $0C00 | $0400 | Kraid 敌人（小 Boss I） |
| 18 | 6 | $B3F0 | $1700 | $00C0 | Ridley 额外 BG |
| 19 | 3 | $9160 | $0C00 | $0400 | Ridley 敌人（小 Boss II） |
| 20 | 6 | $89A0 | $0C00 | $0100 | 结局/介绍精灵 |
| 21 | 6 | $8BE0 | $1400 | $0500 | 标题 |
| 22 | 6 | $9980 | $1FC0 | $0040 | 纯色砖（精灵区底） |
| 23 | 6 | $B4C0 | $1000 | $0400 | 完整字体 |
| 24 | 6 | $B4C0 | $0A00 | $00A0 | HUD 字体 |
| 25 | 6 | $9980 | $0FC0 | $0040 | 纯色砖（BG 区尾） |
| 26 | 6 | $B4C0 | $1D00 | $02A0 | Tourian 字体 |
| 27 | 6 | $90E0 | $0000 | $07B0 | 无装甲 Samus |
| 28 | 6 | $9890 | $1F40 | $0010 | 感叹号 |

- `dest` 决定落在 8 KB CHR-RAM 的哪一段：**精灵表 `$0000-$0FFF`**、**BG 表 `$1000-$1FFF`**
  （与 `PPUCTRL` 的 `PPUCTRL_OBJ_0000`/`PPUCTRL_BG_1000` 一致，`hardware.asm:23-30`）。
  注意 `$0FC0/$1FC0` 两处「纯色砖」是 64 B 覆盖：这两个位置是 16×16 实心砖的图案，供
  「门/爆砖/桥」等填充瓦用（`$1FC0` 用 4 个 tile，`$0FC0` 同）。
- 长项会跨页：`len` 是 16 位，`CopyGFXBlock` 每 256 B 重设一次 `$2006`。

### 1.3 `CopyGFXBlock` 的 `$2006/$2007` 写序（A）

```
1786 CopyGFXBlock: ; 07:C7D5
1788     lda Temp05_GFXSize        ; 长度低字节
1789     bne @loop                 ; 为 0 时先 dec 高字节（长度是 256 的整数倍的情况）
1790     dec Temp05_GFXSize+1
1791 @loop:
1793     lda Temp03_GFXDestination+1 / sta PPUADDR   ; 高字节
1795     lda Temp03_GFXDestination   / sta PPUADDR   ; 低字节
1798     ldy #$00
1799 @lowLoop:
1801     lda (Temp01_GFXSource),y / sta PPUDATA      ; 数据
1804     dec Temp05_GFXSize
1806     bne @endIf
1809     lda Temp05_GFXSize+1 / beq @RTS
1812     dec Temp05_GFXSize+1
1813 @endIf:
1815     iny / bne @lowLoop
1818     inc Temp01_GFXSource+1
1819     inc Temp03_GFXDestination+1
1820     jmp @loop
```

- **这是 4 条 CHR 写规则里最容易被误移植的一条**：低字节先减到 0、再看高字节是否 0 才退；
  因此「长度低字节=0」不是 0 字节，而是 256 的倍数（开头先 `dec` 高字节把计数对齐）。
- 没有 `PPUSTATUS` 读：进入前 `LoadGFX` 已设 `PPUCTRL_ZP &= ~PPUCTRL_INCR_DOWN` 并写 `PPUCTRL`，
  地址锁存在此之前由屏幕关闭路径（`WaitNMIPass_`/`PPUSTATUS` 读）复位。**移植侧等价做法**：
  每次 `$2006` 写之前清零地址锁存，增量=1（`render/ppu.rs::reset_addr_latch`）。
- 源 bank 由表项 byte0 决定，`BankSwitch` 进去、写完用 `CurrentBank` 切回（`LoadGFX:1770-1784`）。

### 1.4 渲染开关依赖（A）

- 换区：`InitBank1-5` 先 `jsr ScreenNmiOff`（`:1402/1444/1452/1475/1483`，实现 `:1165`）：
  关 `PPUMASK` 的 BG/OBJ 位 → 等 NMI 结束（`WriteAndWait`）→ 关 `PPUCTRL` 的 VBL NMI 位。
  **在 NMI 关闭 + 渲染关闭的窗口里做 CHR/名表/调色板上传**。
- 标题：`prg7_engine.asm:1382` 同样 `ScreenNmiOff` 后 `InitTitleGFX`。
- 结论：CHR 写入**不依赖 `$2005`**，只要求渲染关闭（PPU 在 forced blank 时允许随时写 VRAM）；
  关闭顺序是 `$2001` 先、`$2000` 后。

### 1.5 没有「运行时换图」（A，否证式）

- `grep -n "jsr LoadGFX\|jmp LoadGFX" prg7_engine.asm` 全部 52 处落在 §1.1 的 9 个例程内；
  `CopyGFXBlock` 的唯一调用者是 `LoadGFX`；`BankSwitch` 写 CHR bank 的 MMC1 位在本作是
  `MMC1CTRL_CHRBANK_8K`（CHR-**RAM**，`prg7_engine.asm:128`），切换不产生图块内容变化。
- 因此房间里「换背景/换敌人图」都靠 **metatile 与名表**（§1.6），不要给 Rust 侧加逐房间 CHR 上传。
- 唯一“半个例外”：无装甲 Samus（`GFX_SamusSuitless`）在标题/结局路径随 `JustInBailey` 决定是否
  额外上传（`LoadSamusGFX:1505-1510`），**游戏内不重传**——Samus 图块表里本来就同时含两套。

### 1.6 载房名表上传（A）

`AreaInit`（`:1826`）→ `MoreInit`（`:1843`）序列：

```
1871 jsr ScreenOff          ; 关 BG/OBJ（07:C439）
1872 jsr ClearNameTables    ; 清 4 张名表 + 属性（07:C158）
1873 jsr EraseAllSprites    ; $0200 整页 $F0
1874 jsr DestroyEnemies
1882-1885 MapPosX/Y ← AreaMapPosX/Y
1887-1888 PaletteToggle ← AreaPaletteToggle
1890 RoomNumber = $FF
1892 GetRoomNum / 1894-1897 SetupRoom 循环（预渲染到 RoomRAM $6000）
1899-1908 RoomRAMPtr → $00/$01, PPUCTRL 增量=1, 读 PPUSTATUS 复位锁存
1911-1923 PPUADDR=$2000,00；ldx #$04；4 页 ×256 B 逐字节 sta PPUDATA
1925-1927 stx DoorPaletteChangeDir / inx / stx PaletteDataPending(=1 → Palette00)
1930 jmp ScreenOn
```

- 这 **4 页 = `$2000-$2FFF`**：名表 0/1（2 KB）+ 属性表（512 B）+ 名表 2/3 的一部分？
  实际 4×256=1024 B 只覆盖 `$2000-$23FF`——即**名表 0 + 其属性表**；每屏 1 KB。
  （Rust `render/mod.rs::clear_name_table(s)` 与 `world/room_load.rs` 的投影面按同一 1 KB 粒度。）
- 滚屏进门时不再整块拷，改用 `CheckUpdateNameTable` 的增量写（`WriteVRAMString`/`WritePPUByte`，
  §2.1 的同一 VRAMStruct 缓冲，79 B/帧预算）。

## 2. 调色板

### 2.1 请求 → 指针 → 解码器（A）

```
507 CheckPaletteWrite: ; 07:C1E0
    （结局特例：GameMode!=0 且 TitleRoutine>=EndGame ⇒ EndGamePaletteWrite）
520     ldy PaletteDataPending
        bne @paletteDataIsPending     ; 0 = 无请求
        若在开场（TitleRoutine < StartContinueScreen15）⇒ StarPaletteSwitch（星闪）
@paletteDataIsPending: ; 07:C1FF
537     dey / tya / asl / tay           ; 调色板号-1，×2（2B 指针）
542     ldx PalettePtrTable,y / lda +1,y ; X=lo, A=hi
548     sta PaletteDataPending = 0      ; 先清请求再写（写失败也不会重试）
551 PreparePPUProcess: ; 07:C20E
        stx Temp00_VRAMStructPtr / sty +1
856     jmp VRAMStructWrite             ; 与 VRAM 缓冲共用解码器
```

- `PalettePtrTable` 每区一份、**28 项**（`Palette00..Palette1B`），标题 19 项：
  `prg0_title.asm:3201`（`00:9560`）、`prg1_brinstar.asm:55`（`01:9560`）、`prg2_norfair.asm:53`、
  `prg3_tourian.asm:53`、`prg4_kraid.asm:59`、`prg5_ridley.asm:61`。
- 每项是**一串 VRAMStruct**，格式 `[addrHi][addrLo][ctrl][data...]`，以 `$00` 终止；`ctrl` 的
  bit7=增量32、bit6=RLE、低 6 位=长度（0⇒256）。**没有“每项固定 32+16”这回事**，每项段数与
  长度按区不同（§2.2 的「字节」列就是“下一个标签−本标签”）：
  - `Palette00`：单段 `[$3F00][ctrl=$20]`+32 B（BG 4 组×4 色），六区实测都是 36 B（含终止符）。
  - `Palette01-04`：单段 `[$3F12][ctrl=$02]`+2 B（换 Samus 的 2 色，每种装备一项）。
  - `Palette05`：Brinstar/Norfair = `[$3F00][$10]`+16 B 再 `[$3F14][$0C]`+12 B（35 B）；Ridley = 只 `[$3F00][$10]`+16 B（20 B）；
    Kraid = 单段 `[$3F11][$03]`+3 B（7 B，`Palette05-13` 九个标签同址别名）；Tourian = 单段 `[$3F0A][$01]`+1 B（5 B）。
  - `Palette18-1B`：无装甲 Samus，`[$3F10][$04]`+4 B ×4。
  - 因此“渲染腿把 28 项落成 `$3F00-$3F1F`”要**按段解**，不能按固定偏移推算。

### 2.2 每区表地址与 `Palette00` 字节（A）

| 区 | `PalettePtrTable` | `Palette00` | 字节 | 32 B BG 部分（hexdump） |
|---|---|---|---|---|
| 标题 | `00:9560` | `00:9586` | 36 | `0f 28 18 08 0f 29 1b 1a 0f 0f 0f 0f 0f 0f 0f 0f 0f 16 1a 27 0f 37 3a 1b 0f 17 31 37 0f 32 22 12` |
| Brinstar | `01:9560` | `01:A271` | 36 | `0f 22 12 1c 0f 22 12 1c 0f 27 11 07 0f 22 12 1c 0f 16 19 27 0f 12 30 21 0f 27 2a 3c 0f 15 21 38` |
| Norfair | `02:9560` | `02:A178` | 36 | `0f 20 10 00 0f 28 16 04 0f 16 11 04 0f 31 13 15 0f 16 19 27 0f 12 30 21 0f 14 23 2c 0f 16 24 37` |
| Tourian | `03:9560` | `03:A718` | 36 | `0f 20 16 00 0f 20 11 00 0f 16 20 00 0f 20 10 00 0f 16 19 27 0f 12 30 21 0f 27 16 30 0f 16 2a 37` |
| Kraid | `04:9560` | `04:A155` | 36 | `0f 20 10 00 0f 28 19 1a 0f 28 16 04 0f 23 11 02 0f 16 19 27 0f 12 30 21 0f 27 1b 36 0f 17 22 31` |
| Ridley | `05:9560` | `05:A0EB` | 36 | `0f 20 10 00 0f 21 14 13 0f 28 1b 02 0f 15 16 04 0f 16 19 27 0f 12 30 21 0f 14 13 29 0f 13 15 27` |

- **关键条目的段结构（按标签边界实测；每段 `[hi][lo][ctrl]+len`，项尾 `$00`）**：

| 条目 | 标题 | Brinstar | Norfair | Tourian | Kraid | Ridley |
|---|---|---|---|---|---|---|
| `Palette00` | 36（$3F00×32） | 36（$3F00×32） | 36（$3F00×32） | 36（$3F00×32） | 36（$3F00×32） | 36（$3F00×32） |
| `Palette01` | 36（BG 32） | 6（$3F12×2） | 6（$3F12×2） | 6（$3F12×2） | 6（$3F12×2） | 6（$3F12×2） |
| `Palette05` | 36（BG 32） | **35**（$3F00×16 + $3F14×12） | **35**（同左） | **5**（$3F0A×1） | **7**（$3F11×3） | **20**（$3F00×16） |
| `Palette18` | 8（$3F00×4） | 8（$3F10×4） | 8（同） | 8（$3F10×4） | 8（同） | 8（同） |

  - 游戏内 `Palette18-1B` 四项都写 `$3F10` 4 B，只是第 4 色不同（`0f153417/19/28/29`）；标题的 18-1B 写 `$3F00`；
    这四项由 `SelectSamusPalette` 的无装甲分支选（§2.3）。
  - Tourian 的 `Palette05` 只有 1 色（`$3F0A`），5 B；Kraid 的 `Palette05` 是单段 `$3F11`+3 B（共 7 B）——
    `kraid/palettes.asm:53-67` 里 `Palette05` 到 `Palette13` 共 9 个标签**全部指向同一个 `04:A191`**，
    `04:A198` 起的 `$3F11` 段分别是 `Palette14/15/…` 的独立条目；Ridley 没有精灵段（20 B）。
  - 结论：**不要把 28 项都当成同尺寸**；渲染腿按“段”解，不要按固定偏移。

- 布局（`$3F00-$3F0F` 4 组 BG）：每 4 字节一调色板，`[0]=通用底色`（`$0F` 黑，全作唯一条目里都是 0F），
  `[1..3]`=该组 3 色；`$3F10-$3F1F` 是 4 组精灵调色板，`$3F10` 与 `$3F00` 硬件镜像（写哪个都改底色）。
- 精灵调色板由 `Palette01-04`（2 B 改 `$3F12` 的两色）与 `Palette18-1B`（无装甲）覆盖，见 §2.3。

### 2.3 触发点与覆盖规则（A/B）

| 触发 | 位置 | 写入值 |
|---|---|---|
| 进区 | `MoreInit:1845-1846` | `PaletteDataPending ← _id_Palette00+1`（房间默认） |
| 房间加载完 | `:1920-1924`（4 页拷完后） | `← 1`（再请求一次 Palette00 覆盖名表期间的变化） |
| Samus 换装/导弹切换 | `SelectSamusPalette:2535`，`prg7_engine.asm:2546-2559` | `01/02`（power/varia）`+2`（导弹选中）⇒ `03/04`；无装甲 `adc #_id_Palette18-_id_Palette01`（=23，即 `+0x17`）⇒ `18-1B` |
| 开场淡入 | `:2210-2220` | `SamusFadeInTimeTbl` 命中的帧 ⇒ `13/14/15/16` |
| 换区电梯/门 | `:5019-5030`（电梯 `ElevatorD8BF`）、`areas_common.asm:1626-1629`（门） | `PaletteToggle XOR (id00+1 ^ id05+1)` ⇒ 在 `Palette00 ⟷ Palette05` 间切；`InArea>=$12`（Kraid/Tourian/Ridley）再强制 `Palette00` |
| Mother Brain 受击 | `prg3_tourian.asm:1154-1164` | `MotherBrainFlashPaletteTable = [07+1, 06+1]` 按 `MotherBrainFlashDelay & 2` 交替 |
| 结局 | `prg7_engine.asm:511-518` | 走 `EndGamePaletteWrite`（`prg0_title.asm:4487`），不查 `PalettePtrTable` |
| 标题星闪 | `prg7_engine.asm:522-527` | `StarPaletteSwitch`（仅 `TitleRoutine < 15`，每 8 帧推一档） |

- 覆盖顺序：**写请求是“最后一次覆盖”语义**——`PaletteDataPending` 单字节，主线程多处写，NMI 取走即清；
  同一帧内后写者赢（典型：门切调色板 vs Samus 换装同帧）。这就是“闪烁”外观的来源之一。
- 淡入/淡出不是逐字节插值，是**整组调色板替换 + 隔帧绘制**：Samus 在开场按奇偶帧显示
  （`prg7_engine.asm:2222` `lda FrameCount/lsr`），淡入表在 `SamusFadeInTimeTbl`。

### 2.4 复跑命令（A）

```bash
python3 - <<'PY'
rom=open('rom/Metroid (USA).nes','rb').read()
def off(bank,addr): return 16+bank*0x4000+(addr-(0xC000 if bank==7 else 0x8000))
def segs(bank,addr,maxb=80):
    b=rom[off(bank,addr):off(bank,addr)+maxb]; out=[]; i=0
    while i<len(b):
        if b[i]==0: break                      # 单项结束符是 1 个 $00
        hi,lo,ctrl=b[i],b[i+1],b[i+2]
        ln=(ctrl&0x3f) or 256
        out.append((f'{hi:02X}{lo:02X}', ctrl, ln, b[i+3:i+3+ln].hex()))
        i+=3+ln
    return out
rows=[('title',0,0x9586,0x95AA,0x963A,0x97EA),
      ('brinstar',1,0xA271,0xA295,0xA2AD,0xA2F4),
      ('norfair',2,0xA178,0xA19C,0xA1B4,0xA1FB),
      ('tourian',3,0xA718,0xA73C,0xA754,0xA7B1),
      ('kraid',4,0xA155,0xA179,0xA191,0xA1B5),
      ('ridley',5,0xA0EB,0xA10F,0xA127,0xA15F)]
for name,bank,p00,p01,p05,p18 in rows:
    print('==',name)
    for tag,a in (('00',p00),('01',p01),('05',p05),('18',p18)):
        ss=segs(bank,a)
        total=sum(3+x[2] for x in ss)+1
        print(' Palette'+tag, f'len={total}', [(x[0],hex(x[1]),x[2]) for x in ss], ss[0][3][:24])
PY
# 期望：00 每区 36 B（3F00/0x20/32）；01 每区 6 B（3F12/0x02/2）；
#       05 Brinstar/Norfair=35（3F00/0x10/16 + 3F14/0x0C/12）、Ridley=20、Tourian=5、Kraid=36；
#       18 每区 8 B（3F10/0x04/4）
```

## 3. OAM 提交

### 3.1 NMI 严格序（A）

```
233 NMI: ; 07:C0D9
    php/pha/txa pha/tya pha
245     lda #$00 / 246 sta OAMADDR       ; OAM 地址 = 0
248     lda #>SpriteRAM / 249 sta OAMDMA ; $4014 ← $02，整页 256 B DMA
251     lda NMIStatus / 252 bne @endIf_A ; 上一帧没跑完 ⇒ 跳过本帧所有 PPU 写
    254-258 （GameMode!=0 时先 NMIScreenWrite 结局字幕）
260     jsr CheckPaletteWrite            ; 调色板
262     jsr CheckVRAMStructBufferWrite   ; 名表/瓦片增量缓冲
264     jsr WritePPUCtrl                 ; $2000/$2001 影子落地（07:C44D）
266     jsr WriteScroll                  ; $2005,$2005（先读 PPUSTATUS 复位）
268     jsr ReadJoyPads
271-275 jsr SoundEngine（NTSC 分支）/ 277 jsr UpdateAge
     ldy #$01 / sty NMIStatus         ; NMI = finished（MainLoop 等它变 1）
```

- `NMIStatus` 的语义（`constants_ram.asm:496`）：**1 = NMI 侧已完成（主线程可继续），0 = 正在 NMI/等待**。
  `MainLoop`（`:199`）每帧末尾 `sta NMIStatus=0` 后自旋等它变 1（`:206-214`）；`WaitNMIPass/_:1133/1141`
  是主线程里“主动让出一帧”的同一原语。
- **PPU 写全在 NMI 内、且被 `NMIStatus` 门控**：主线程超时会整帧跳过调色板/名表/滚动更新（掉帧），
  OAM DMA 不受门控（在门控之前）。
- `WriteScroll`（`:717`）先 `lda PPUSTATUS` 再写两次 `$2005`：**`$2005` 写序必须在 VRAM 写之后**，
  否则 `$2000`/`$2005` 的共享锁存会被打乱。

### 3.2 OAM 缓冲与逐帧清/填（A）

- 位置：`SpriteRAM instanceof OAMSprite $40 startfrom 0`（`constants_ram.asm:850`），即 **`$0200-$02FF`，
  64 槽 ×4 B = `[Y][tile][attr][X]`**。Rust 侧 typed 真源=`render/oam_shadow.rs`（A 级注释同源）。
- 帧内生产（`UpdateWorld: ; 07:CB29`，`:2494`）：

```
2497 SpritePagePos = 0
2499 jsr UpdateAllEnemies          ; 敌人（含 Samus 的碰撞? 不，见下）
2500 jsr UpdateSamusProjectiles
2501 jsr UpdateSamus
2502 jsr AreaRoutine
2503 jsr UpdateElevator
2504 jsr UpdateAllStatues
2505 jsr UpdateAllEnemyExplosions
2506 jsr UpdateAllMellows
2507 jsr UpdateAllEnProjectiles
2508 jsr UpdateAllSkreeProjectiles
2509 jsr SamusEnterDoor
2510 jsr UpdateAllDoors
2511 jsr UpdateAllTileBlasts
2512 jsr CollisionDetection
2513 jsr DisplayHUD
2514 jsr UpdateAllPipeBugHoles
2515 jsr CheckMissileToggle
2516 jsr UpdateAllPowerUps
2517 jsr UpdateTourianItems
2519-2527 剩余槽：ldx SpritePagePos; lda #$F4; loop: sta SpriteRAM.0.y,x; X+=4; bne loop
```

- **整页从不清零**：从 `SpritePagePos` 到页尾统一写 `$F4`（Y=$F4 在屏幕下方 + 空图案）。
  也就是说“清屏”是**尾部填充**，前段由各绘制者按顺序覆盖；页首没画到的槽位若 `SpritePagePos`
  没推进，仍会保留上一帧数据——这是原版行为，Rust 侧照抄（`world/update.rs:162` 的 `$F4` 哨兵同源）。
- `EraseAllSprites:443`（`07:C1A3`）只在大加载/切模式时把整页填 `$F0`，不是每帧。
- 绘制原语（`WriteSpriteRAM:6315`）字节序：
  1. `YDisplacement` + `ScreenY` - 1 ⇒ `SpriteRAM+0`（PPU 的 Y 比 CPU 少 1 行，代码里 `dec`）；
  2. `(FramePtr),y` ⇒ `+1` tile；
  3. `ObjectCntrl` 的 bit4（水平镜像）`asl asl` 后与 `Temp05_Cntrl` 异或 ⇒ `+2` attr（bit6 翻转覆盖）；
  4. `XDisplacement` + `ScreenX` ⇒ `+3`；
  5. `inx/x/x/x` 推进到下一槽。
- 元精灵控制字节（`GetNextFrameByte:6345`）：`<$FC`=tile；`$FC`=临时偏移 xy（`OffsetObjectPosition:6382`）；
  `$FD`=换 attr（`GetNewControlByte:6367`，可走 `SpriteAttrsOverride 07:E038` 用对象属性覆盖）；
  `$FE`=跳过一对 placement（`SkipPlacementData:6361`）；**`$FF`=结束**（落到 `ClearObjectCntrl:6356`）。
- 属性位：`OAMDATA_HFLIP`/`VFLIP`（`hardware.asm`），attr 低 2 位=精灵调色板组（0-3 ⇒ `$3F10+4*n`）。

### 3.3 每帧时序与字节上限（A）

- 一帧最多 64 槽（64 个 8×8 精灵）；`SpritePagePos` 是 u8、每次 +4 ⇒ 最多 **16 个 16×16 对象**（每个对象占 4 槽），或若干更小的部件。
- `SpriteRAM` 在 NMI 被 `$4014` 整页取走（DMA 期间 CPU 停 513 周期）；主线程下一帧继续改的是同一页影。
- 8×8 精灵 + `PPUCTRL_OBJH_8`（`hardware.asm:23`）。16×16 物件由 4 个 8×8 tile 拼成。

### 3.4 8-sprites/scanline 与掉帧的登记（B/A）

- 硬件限制：**每扫描线最多 8 个精灵**，超出部分 PPU 直接不画；ROM 侧**没有**优先级排序、轮转或
  分帧抖动逻辑（`WriteSpriteRAM` 只顺序追加，OAM DMA 原样提交）。
- 后果：同扫描线精灵多时出现闪烁/缺件；换帧后哪个被丢由 OAM 顺序与 PPU 内部决定。
- 登记方式：`docs/bug-ledger.md` 现只有 **BUG-089**（L4 f1000/f1150 残差=渲染/采样时序类）。
  **建议补一条 BUG-xxx「8 sprites/scanline 硬件裁剪：同线>8 时闪烁」**，并在 Rust 侧不改行为
  （视觉腿必须能复现同样的裁剪，而不是“把精灵都画出来”）。
- 掉帧：`NMIStatus` 门控导致「主线程超时 ⇒ 该帧调色板/名表/滚动更新被跳过」——这是可观测的
  视觉停顿，属 Stage A 保真项；`docs/spec/scroll-work-accounting*.md` 已量化该超时的触发条件。

## 4. 移植契约（Rust）

### 4.1 渲染腿需要的最小输入（接口语义，不规定实现）

| 输入 | 形态 | 来源 | 备注 |
|---|---|---|---|
| CHR-RAM | `[u8; 0x2000]` 8 KB | `game-data` GFXInfo 烘焙 | 只在加载时整体换；索引=tile<<4 |
| BG 调色板 | `[u8; 16]` | `$3F00-$3F0F` 影子 | 索引顺序=PPU 属性表低 2 位 |
| 精灵调色板 | `[u8; 16]` | `$3F10-$3F1F` | `$3F10`=底色镜像 |
| OAM | `[u8; 0x100]` | `$0200` 页 | 4 B/槽；Y 已含 -1 偏置 |
| 名表+属性 | `$2000-$2FFF` 2 KB 实际用 1 KB/屏 | `RoomRAM`→`VRAM` | 滚动增量由 VRAMStruct 缓冲 |
| 滚动/控制 | `ScrollX/Y`、`PPUCTRL_ZP`、`PPUMASK_ZP` | `$FC/$FD`、影子 | `$2005` 只在 NMI 末尾写 |

### 4.2 现有 Rust 面（可直接消费）

- `crates/game-core/src/render/ppu.rs`：`write_ppuaddr/write_reg_2006/reset_addr_latch/write_ppudata`（地址锁存+增量）。
- `crates/game-core/src/render/vram.rs`：`vram_struct_write`（逐字节语义，含 RLE/0⇒256 quirk）。
- `crates/game-core/src/render/oam.rs`（`draw_sprite_stream`）与 `oam_shadow.rs`（`$0200` typed 页）。
- `crates/game-core/src/render/mod.rs:585`：`load_gfx(rs,&[u8;0x2000])`（整页装配）；`render/gfx.rs:176` 是另一个同名函数 `load_gfx(mem,id)`（按表项装载），两者签名不同，引用时写清。
- `crates/game-core/src/render/mod.rs`：`render_frame(_bg)`（BG+精灵合成）、`clear_name_table(s)`。

### 4.3 约束（写进实现卡）

1. **CHR 不逐房间换**：`load_gfx` 只在区域/标题/结局加载点调用（§1.1 的 9 处）。
2. **`$2006` 双写前清锁存**；`$2005` 只在 NMI 末尾、VRAM 写之后。
3. **OAM Y 已 -1**，属性和翻转位按 `ObjectCntrl`/`Temp05_Cntrl` 的异或规则；`$F4/$F0` 语义不同
   （`$F4`=尾部填充，`$F0`=整页擦除）。
4. **调色板是请求字节**：`PaletteDataPending` 单字节、1-based、先清后写；实现侧用同样语义
   （不要在 Rust 里做“立即写调色板”）。
5. **8 sprites/scanline 不修**（D4）：视觉腿必须复现裁剪，不排序/不补画。
6. **帧门控**：`NMIStatus` 语义照抄（主线程超时⇒跳过 PPU 更新），否则 L4 帧序对不上。

## 5. 假设区（C 级，需后续卡推翻/证实）

- **C1**：`$0FC0/$1FC0` 的“纯色砖”覆盖面=门/爆砖/桥的填充瓦。依据：两段 dest 正好落在两表尾部 64 B，
  且 `DrawTileBlast` 的 routine=3 用 `$1FC0+` 的 4 个 tile。推翻方式：改这两段的字节，看同屏哪些瓦变。
- **C2**：`InitTourianGFX` 的 10 次 `LoadGFX` 中 `GFX_Zebetite($1900)` 与 `GFX_TourianFont($1D00)`
  只在 MB 房/结局用，其余房间白占 CHR；若 Rust 做“按需上传”会与 L4 图块差。推翻方式：进非 MB 房截图，
  看 `$1900-$19FF` 是否有内容（原版有）。
- **C3**：`NMIStatus` 门控丢的 PPU 更新里，调色板请求**不会**重试（`CheckPaletteWrite` 已先清 pending），
  所以超时帧的调色板变化永久丢失，直到下一次请求。推翻方式：构造超时帧后连跑 2 帧看调色板是否补上。
- **C4**：标题 19 项调色板表里 `Palette0C-12` 等与房间表同名不同内容（标题专用），游戏内不复用。
  推翻方式：交叉引用 `_id_Palette0C` 在 `prg1-5` 的使用点（预期 0）。

## 6. 复跑命令汇总

```bash
# ① GFXInfo 29 项（bank/src/dest/len）
python3 - <<'PY'
rom=open('rom/Metroid (USA).nes','rb').read()
def off(b,a): return 16+b*0x4000+(a-(0xC000 if b==7 else 0x8000))
base=off(7,0xC6E0)
for i in range(29):
    e=rom[base+7*i:base+7*i+7]
    print(i, e[0], hex(e[1]|e[2]<<8), hex(e[3]|e[4]<<8), hex(e[5]|e[6]<<8))
PY
# ② LoadGFX 调用点归属（应 52 处/9 例程）
python3 - <<'PY'
import re
cur=None; res={}
for i,l in enumerate(open('refs/m1disasm/SRC/prg7_engine.asm'),1):
    m=re.match(r'^([A-Za-z_]\w*):',l)
    if m: cur=m.group(1)
    if re.search(r'\b(jsr|jmp) LoadGFX\b',l): res.setdefault(cur,[]).append(i)
print(sum(map(len,res.values())), len(res), {k:len(v) for k,v in res.items()})
PY
# ③ 调色板字节（§2.2 表）
# 见 §2.4 脚本
# ④ NMI 序 / OAM DMA（只看行号）
sed -n '233,285p' refs/m1disasm/SRC/prg7_engine.asm
sed -n '2494,2530p' refs/m1disasm/SRC/prg7_engine.asm
```

## 7. 未覆盖 / 交给后续卡

1. **逐房间 metatile→tile 的具体映射**属 `docs/spec/room-structures.md` 与 game-data 四级链，本文只到
   “上传哪些页”为止；S6-7 的 L4 像素级验收仍未做（`docs/spec/l4-*.md`）。
2. **精灵属性覆盖 `SpriteAttrsOverride 07:E038`** 的完整分支（Boss 受击闪白等）未逐条展开；只给入口与
   调用点（`:6375`）。
3. **8 sprites/scanline 的独立 bug 条目**尚未写入 `docs/bug-ledger.md`（本文 §3.4 给了建议文本）。
4. **`StarPaletteSwitch`（标题星闪 8 帧周期）** 只给入口，未给每帧色号表。
5. **CHR 上传的 PPU 时序**：本文按“渲染关闭时可随时写”处理；forced blank 内单帧能否写完 8 KB
   未做周期测量（若 Rust 侧做逐周期模型需要另卡）。
