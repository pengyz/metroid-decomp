# adsr/nes-metroid 第四源反汇编交叉对读（密码系统 + 音频例程域）

> 2026-11-09 · porter 执行卡（lead 裁决，与 AccuracyCoin 卡并行，写面不相交）。
> 纪律：第三方命名只作线索不背书（AGENTS D5）；本轮**只产 B 级候选清单 + 证据指针**，
> **不落 rename、不写 data/naming/**（D5：rename 须逐条独立 A/B 证据入台账）。
> 复核状态：**待对抗复核**（verifier 证伪立场独立重导证据后方可消费结论）。

## 0. 源锚点（A 级：clone 实录）

- 仓库：https://github.com/adsr/nes-metroid → clone 到 `analysis/adsr-nes-metroid/repo/`
  （analysis/ 为可再生工作区，不入库——AGENTS §2）。
- **commit sha：`97b8a6153badff5e5cb7aff41855d94d1e72d3a9`**（2026-05-07，
  commit message 自述 `mirror of http://www.metroid-database.com/m1/sourcecode.php`）。
- 格式：6809 风格 `Lxxxx:` 标签 + `.alias` 符号 + 密集注释（8 个分页 .asm）。
- 复现：`git clone https://github.com/adsr/nes-metroid analysis/adsr-nes-metroid/repo`
  后 `git -C analysis/adsr-nes-metroid/repo rev-parse HEAD`。

## 1. 等价性核验实录（A 级：逐字节 hexdump 对 ROM）

**换算声明**（gotcha-cpu-file-offset-conversion.md 纪律：公式显式）：

```
file = 0x10 + bank*0x4000 + (cpu − 0x8000)
```

- 密码域目标在 bank0（title 页）：`$8F2D → file 0x0F3D`；
- 音频域目标在 bank1（sound engine 副本）：`$BBD0 → file 0x7BE0`。

### 1.1 密码例程段（bank0，2026-11-09 实测）

| CPU 地址 | file 偏移 | ROM 字节（hexdump） | adsr listing 指令 | 一致 |
|---|---|---|---|---|
| $8F2D | 0x0F3D | `B9 88 69 4A 4A 60` | `LDA PasswordByte00,Y; LSR; LSR; RTS`（SixUpperBits） | ✓ |
| $8F33 | 0x0F43 | `B9 88 69 29 03 20 C5 C2 85 00 B9 89 69 20 BF C2 05 00 60` | TwoLowerAndFourUpper（内联 JSR $C2C5/$C2BF） | ✓ |
| $8F46 | 0x0F56 | `B9 88 69 29 0F 0A 0A 85 00 B9 89 69 2A 2A 2A 29 03 05 00 60` | FourLowerAndTwoUpper | ✓ |
| $8F5A | 0x0F6A | `B9 88 69 29 3F 60` | SixLowerBits | ✓ |

### 1.2 音频例程段（bank0/bank1 双副本同字节——旁证 m1disasm §6 副本结构）

| CPU 地址 | file 偏移 | ROM 字节 | adsr listing | 一致 |
|---|---|---|---|---|
| $BBD0 | bank1→0x7BE0；bank0→0x3BE0（两副本同） | `0A 0A C9 3C 90 02 A9 3C 8D 2A 06 4C 37 BB AD 88` | `ASL;ASL; CMP #$3C; BCC; LDA #$3C; STA TriLinearCount; JMP $BB37`（AddTriangleLength 体） | ✓ |

复现命令（A 级，一行）：
`python3 -c "d=open('rom/Metroid (USA).nes','rb').read(); print(d[0x10+0xF2D:0x10+0xF2D+20].hex(' '), d[0x10+0x4000+0x3BD0:0x10+0x4000+0x3BD0+16].hex(' '))"`

**结论：adsr listing 与我们 ROM 在两域抽样段逐字节等价 ⇒ 后续对读合法。**

## 2. 谱系警示（重要前提，影响"四源"独立性）

adsr commit 自述为 **metroid-database m1 sourcecode 的 mirror**；实测比对发现
`refs/nmikstas-metroid-disassembly/Source_Files/Bank00.asm` 与
`refs/MetroidMMC3/dev.Original/code/Title.asm` 使用**同一套 `Lxxxx:` 标签 + 同名符号 +
同文注释**（如 `L8E6E: JSR SixUpperBits ;($8F2D)` 三方逐字符一致）。
⇒ 名义四源实为**两条独立谱系**：

1. **metroid-database 谱系**：adsr = nmikstas = MetroidMMC3/dev.Original（同源镜像，相互印证价值低）；
2. **m1disasm 谱系**：独立命名（如 `$BBFA` 在 database 谱系叫 `InitMusicIndexTbl`、
   m1disasm 叫 `SongHeaderOffsetTable`——同址异名是谱系独立的正面证据）。

**"四源一致"降格执行为「database 谱系命名 ↔ m1disasm 命名 同址一致 + ROM 字节自证」**
——这仍是有价值的双谱系交叉，但不能按 4 份独立证据计权。

## 3. 两域对读表（候选清单，全部 B 级候选；本轮禁 rename）

对读方法：同地址符号名/注释比对 + §1 字节等价 + 调用语境（JSR/JMP 目标地址）。

### 3.1 密码系统域（bank0 $8E17–$8F5A + RAM $6988）

| 地址 | adsr/database 符号 | m1disasm 对应 | 一致性 | 语义候选 | 证据级/指针 |
|---|---|---|---|---|---|
| $8F2D | `SixUpperBits` | `SixUpperBits`（prg0_title.asm:2126） | 同名同址 | 取字节高 6 位（LSR×2）产密码字符 | B：字节自证 §1.1 + 双谱系同名 |
| $8F33 | `TwoLowerAndFourUpper` | `TwoLowerAndFourUpper`（prg0_title.asm:2134） | 同名同址 | 低2位<<4 ‖ 高4位拼字节 | B：同上 |
| $8F46 | `FourLowerAndTwoUpper` | `FourLowerAndTwoUpper`（prg0_title.asm:2147） | 同名同址 | 低4位<<2 ‖ 高2位拼字节 | B：同上 |
| $8F5A | `SixLowerBits` | `SixLowerBits`（prg0_title.asm:2165） | 同名同址 | `AND #$3F` 取低 6 位 | B：字节自证 `29 3F 60` |
| $8F60 | `ConsolidatePassword` | `ConsolidatePassword`（prg0_title.asm:2174） | 同名同址 | 24 密码字符 → 18 密码字节 | B：调用语境 `$8C5E JSR` 双谱系 |
| $8E17 | `PasswordChecksumAndScramble` | `PasswordChecksumAndScramble`（prg0_title.asm:2035） | 同名同址 | 校验和+乱序 | B：双谱系同名+调用语境 |
| $8E21 | `PasswordChecksum` | `PasswordChecksum`（prg0_title.asm:2044） | 同名同址 | `$6988-$6998` 求和 | B：同上 |
| $C2BF | `Adiv16` | `Adiv16`（prg7_engine.asm:762，07:C2BF） | 同名同址 | A÷16 | B：bank7 固定槽，跨 bank 调用（bank0 内 `JSR $C2BF` 字节自证 §1.1） |
| $C2C5 | `Amul16` | `Amul16`（prg7_engine.asm:776，07:C2C5） | 同名同址 | A×16 | B：同上 |
| RAM $6988 | `.alias PasswordByte00` | `PasswordByte`（constants_ram.asm:1238，$6988-$6999） | 同名同址（+11 校验和 $6999） | 密码字节区 18B | B：constants_ram 注释 + 指令操作数 `B9 88 69` 字节自证 |

### 3.2 音频域（sound engine 副本 bank0/1/3/4/5，$BAC2–$BF19 段）

| 地址 | adsr/database 符号 | m1disasm 对应 | 一致性 | 语义候选 | 证据级/指针 |
|---|---|---|---|---|---|
| $BBFA | `InitMusicIndexTbl` | `SongHeaderOffsetTable`（sound_engine.asm:2040 区） | **同址异名** | 曲目→SongHeader($BD31 基) 偏移索引表 | B：字节自证（.byte 值两谱系一致）+ 初始化调用语境 |
| $BD31 | `InitMusicTbl`（注释 base=$BD31） | `SongHeaders`/`SongMthrBrnRoomHeader`（sound_engine.asm:2256/2258） | 同址异名 | 歌曲初始化头表（5 字节参数+4 指针） | B：头注释格式说明双谱系一致（adsr MetroidBrinstarPage.asm:5451 注释原文同 m1disasm sound_engine.asm:2040 区） |
| $BADC | `LoadNextChannelIndexData` | `LoadNextMusicChannelInstr`（sound_engine.asm:1808） | 同址异名 | 通道指令流取下一条 | B：`JMP $BADC` 调用语境内联注释两谱系 |
| $BBA8 | `LoadNewMusicFrameCount` | `SetMusicInstrDelayToLength`（sound_engine.asm:1975） | 同址异名 | 帧计数重载（音符时长→帧延迟） | B：同上（**帧计数语义：adsr 名更贴近音序循环实测用途，m1disasm 名贴指令流语义——语义候选分歧点，升 A 需 trace**） |
| $BBB7 | `LoadTriangleCntrl0` | `UpdateMusicTriLinearCount`（sound_engine.asm:1988） | 同址异名 | 三角波 linear counter 装载 | B：同上 |
| $BBCD | （database 谱系无独立名，内联）`@setToNoteLength`（m1disasm sound_engine.asm:2006）；adsr 同段为 AddTriangleLength 体 $BBCD- | m1disasm `@setToNoteLength` ; $BBCD | 部分对齐 | 三角波长度钳位 #$3C | B：字节自证 §1.2 `C9 3C 90 02 A9 3C` |
| $BBDE | `LoadNoiseChannelMusic` | `MusicChannelInstr_SongNoteNoise`（sound_engine.asm:2023） | 同址异名 | 噪声通道鼓点装载 | B：`JMP $BBDE` 注释两谱系 |
| $BC4B | `ContinueMusic` | `ContinueMusic`（sound_engine.asm:2115） | 同名同址 | 音乐续播入口 | B：双谱系同名 |
| $BF19 | `InitializeMusic` | `InitializeMusic`（sound_engine.asm:2666） | 同名同址 | 音乐初始化入口 | B：双谱系同名 |
| $BEF7 | `NoteLengths0Tbl`（注释 Base is $BEF7） | `NoteLengthsTbl@…`（按 BUILDTARGET 分实例） | 同址异名（多实例） | 音符时长表 | B：adsr MetroidBrinstarPage.asm:5342 注释 + m1disasm SongHeader 引用 |

### 3.3 结构性观察（对读副产品）

- **sound engine 副本错位面**（呼应 m1disasm-architecture.md §6）：bank6 graphics 页在 adsr
  listing 中同地址是**裸 .byte 数据**（MetroidGraphicsPage.asm:1034 `LBBD0: .byte $0A…`），
  即 bank6 无 sound engine 副本——两谱系对该错位结构描述一致。
- adsr `.alias` RAM 符号集中在 MetroidDefines.asm（如 `MusicInitIndex $065E`），
  可作 D 面 RAM 命名第四线索（本轮未逐条对读，遗留假设区）。

## 4. 升 A 路径（每候选）

1. **字节级已 A**：§1 表内 ROM 字节比对（含操作数目标地址 `$6988/$C2BF/$C2C5/$BB37`）。
2. **语义级升 A**：每候选需 Mesen2 trace 自证调用时序/数据流（如 $BBA8 帧计数语义分歧：
   trace 音序循环内 `DEC SQ1MusicFrameCount`→`JMP $BBA8` 实测路径）；结论入 data/naming/
   台账时**一条一证据字段**（D5/AGENTS §3.4），本轮不落 rename。
3. 谱系警示（§2）必须随候选一起入台账 evidence 字段：计权按双谱系非四独立源。

## 5. 写面与门禁

- 本卡写面：`analysis/adsr-nes-metroid/`（clone 工作区，不入库）+ 本文档。未触碰
  tools/oracle、oracle-accuracycoin-baseline.md、morphball-trajectory-root.md、data/naming/。
- 门禁：`python3 gates/cli.py check`（纯 docs 改动；L4 全矩阵不需要）→ commit+push。
