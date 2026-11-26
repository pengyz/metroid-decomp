# `$6AFB-$6BD3` 13 段 140 B 语义规格（B2-b 取证）

> 状态：取证完成，待实现卡（本文不改代码）。
> 范围：WRAM `$6AFB-$6BD3` 共 **140 B**（13 段：12 段 × 9 B + 32 B），只读分析 `rom/Metroid (USA).nes`
> 与 `reference/seed/metroid.asm`；证据级别按 `AGENTS.md` §3 定义。
> 口径注：卡面写「B2-b」，`docs/design/typed-struct-plan.md` 里覆盖本范围的是 **B3**（`:159`「新增
> `enemy/extra_face.rs`（`$6AFB-$6BD3` 13 段共 140 B）」）。两者范围等价，下文按范围论证，不按批次名。

## 0. 结论摘要

1. 这 140 B **不是一个独立数组**，而是三个对象「额外面」数组在 WRAM 里首尾相接后的**尾段**：
   `EnsExtra`（敌 6 槽 × 16 B）→ `EnProjectilesExtra`（弹 6 槽 × 16 B）→ `EnExplosionsExtra`（爆炸 4 槽 × 8 B）。
   `$6AF4 + $C0 = $6BB4`，三段无空洞（§2）。
2. 12 段 9 B 是 `EnExtra` 结构（`constants_ram.asm:125-160`）每槽的**第 8–16 字节**：`hi/subPixelY/
   subPixelX/accelY/accelX/spare1C/jumpDsplcmnt/type/data1F`；前 7 B 已是 `Owner`（`enemy/state.rs:942-970`）。
3. 32 B 是 4 个 `EnExplosionExtra`（8 字段，`constants_ram.asm:161-170`）槽，从 `$6BB4` 起 8 B 步长。
4. **寻址方式**：代码几乎从不用 `EnExplosionsExtra` 符号，而是统一写 `EnsExtra.0.<字段>,x`，`x` 取
   「对象槽相对 `Ens`（`$0400`）的字节偏移」——敌 `$00-$5F`、弹 `$60-$AF`、爆炸 `$C0-$F8`（§2.2）。
   这解释了为什么 12 段尾字节的静态 xref 全部挂在 `EnsExtra.0.*` 上、而 32 B 窗口在 seed 里 0 命中。
5. 无命名缺口：9 个尾字段在 asm 里全有名字，32 B 的 8 个字段也全有名字；**不需要** `_todo_<addr>`。
   唯一「有名但无语义」的是 `spare1C`（asm 注 `Unused`，0 处 xref，10 009 份 trace 恒 0）。
6. 5 区归属：**全局共享一份、无区号绑定**。12 段在 5 个区的 trace 里都出现过非零（§4 矩阵）；
   per-area 复制假设已被 #85 与本文矩阵双重否证。
7. 落点建议：新增两个 typed face（12×9 B + 4×8 B），四态 `Owner 1449→1589`、`Unmodeled 385→245`、
   `Undeclared 438` 不变、`View 23` 不变、`sum 2272` 不变（§9）。

## 1. 判定口径

- **A 级**：ROM 字节 / asm 行 / 可复跑命令直接给出。本文的表结构、字段顺序、地址、步长、读取次数均属 A。
- **B 级**：代码语境 + 调用图推断（如「`x` 的取值域 = 槽偏移」由三处循环初始值推出）。
- **C 级**：假设，只能进 §11；不据此命名、不改代码。
- 「xref 计数」一律指 **`refs/m1disasm/SRC/**.asm` 具名行数**（`areas_common.asm` 被 bank1–5 各
  `include` 一次 ⇒ 1 行 = 5 个物理副本，物理份数 = 行数 × 5）；另有「seed 可解码地址条数」一列，
  指 `reference/seed/metroid.asm` 文本里能数到的 `$6AFx,X`/`$6B0x,X` 指令条数（受 IDA 解码覆盖限制）。

## 2. 结构事实：三数组连续 + 统一索引

### 2.1 地址与步长（A）

`refs/m1disasm/out/M1_NES_NTSC.sym`：

```
00000400 Ens
00000460 EnProjectiles
000004c0 EnExplosions
00006af4 EnsExtra
00006b54 EnProjectilesExtra
00006bb4 EnExplosionsExtra
```

`constants_ram.asm:1294-1296`：

```
EnsExtra               instanceof EnExtra        6 startfrom 0
EnProjectilesExtra     instanceof EnExtra        6 startfrom 0
EnExplosionsExtra      instanceof EnExplosionExtra 4 startfrom 0
```

`EnExtra` = 16 B（`:125-160`），`EnExplosionExtra` = 8 B（`:161-170`）：

```
$6AF4 + 6*16 = $6B54   (EnsExtra 尾 → EnProjectilesExtra 首)
$6B54 + 6*16 = $6BB4   (EnProjectilesExtra 尾 → EnExplosionsExtra 首)
$6BB4 + 4*8  = $6BD4   (窗口末 $6BD3 的下一字节)
```

⇒ 140 B 的三段拼法是 **12 个 9 B 尾 + 32 B 爆炸额外面**，没有对齐空洞。

### 2.2 统一索引（A/B）

`x` 是对象槽相对 `Ens` 的字节偏移，三个数组共用同一套 `EnsExtra.0.<字段>,x` 指令：

- 敌槽：`x = 0,16,…,80`（6 槽），`$6AF4+x`（B：由 `UpdateAllEnemies` 循环与 6 槽声明推出）。
- 弹槽：`x = $60,…, $B0`，`$6AF4+x = $6B54+…`（A：`EnsExtra.0.status,x` 与 `$6B54` 段等长重合）。
- 爆炸槽：`x = $C0,$C8,$D0,$D8`，`$6AF4+x = $6BB4+…`。

三处直接证据：

1. 爆炸生成循环（`prg7_engine.asm:11229-11239`）以 `ldx #EnExplosions.0 - Ens`（= `$C0`）起步、
   步长 `_sizeof_EnExplosions.0`（`EnExplosion` 8 B，`constants_ram.asm:147-154`），读
   `lda EnsExtra.0.status,x`（`:11231`）—— 对槽 0 就是 `$6BB4`。
2. 动画初始化 `InitEnAnimIndex`（`:11290-11297`）写 `sta EnsExtra.0.resetAnimIndex,x`、
   `sta EnsExtra.0.animIndex,x`；爆炸生成时 `jsr InitEnAnimIndex`（`:11246`）带的正是爆炸槽 `x`，
   于是槽 0 的 `resetAnimIndex/animIndex` 落在 `$6BB9/$6BBA`。
3. 动画推进 `UpdateEnemyAnim`（`:E094`，含 `dec/lda/sta EnsExtra.0.animDelay,x`、`sta EnsExtra.0.animFrame,x`
   于 `:6674/:6680/:6692`）被 `AnimDrawEnemy` 复用给爆炸（`:12168 jmp AnimDrawEnemy`），
   槽 0 的 `animFrame/animDelay` 落在 `$6BB7/$6BB8`。
   10 009 份 trace 里爆炸窗口的非零字节正是 offsets 3–6（= `animFrame/animDelay/resetAnimIndex/animIndex`，
   见 §6.4），与该寻址一一对应。
4. **ROM 字节直证**（`reference/seed/metroid.asm` `bank07:F630-F682`，该段 IDA 未解码，下面是原始字节手工对读）：

```
bank07:F634  A2 C0        LDX #$C0              ; x = EnExplosions.0 - Ens
bank07:F636  BD F4 6A     LDA $6AF4,X           ; EnsExtra.0.status,x
bank07:F63D  69 08        ADC #$08              ; 步长 8（= _sizeof_EnExplosion，非 16）
bank07:F640  C9 E0        CMP #$E0              ; 0xC0+4*8，4 个爆炸槽
bank07:F651  FE F4 6A     INC $6AF4,X           ; EnsExtra.0.status,x ← 爆炸槽 0 即 $6BB4
bank07:F670  9D FB 6A     STA $6AFB,X           ; EnsExtra.0.hi,x     ← 爆炸槽 0 即 $6BBB
```

`m1disasm` 把 `:11267` 写成 `EnExplosionsExtra.0.hi - EnExplosions + Ens,x`，汇编结果就是上表
`$6AFB`；两者是同一字节序列（`9D FB 6A`），不是两个写点。

**因此**：说「`$6BB4-$6BD3` 是 `EnExplosionsExtra` 4×8」只在**布局**上成立；说「代码按
`EnExplosionsExtra.0.*` 访问」不成立。asm 里唯一出现该符号的行是
`prg7_engine.asm:11267 sta EnExplosionsExtra.0.hi - EnExplosions + Ens,x`，其有效操作数
`$6BBB-$04C0+$0400 = $6AFB`，即「`EnsExtra` 基址 + `hi` 偏移」，仍是同一套扁平寻址。

## 3. 13 段逐段表

「段」按 nes_map 现有 13 个 `Unmodeled` span 排列（`python3 tools/nes_map_check.py --list` 实读）。

| # | 起-止 | 长度 | RAM 命名（asm） | 所属结构/槽 | 区 | 写点 bank:addr（代表） | xref（行/写/读/RMW） | seed 地址条数 | 级别 |
|---|---|---|---|---|---|---|---|---|---|
| 1 | `$6AFB-$6B03` | 9 | `EnsExtra.0.hi…data1F` | `EnsExtra` 槽0 尾 | 全局 | `01:855F hi`、`07:FA57/FB3C hi`、`01:82AB jumpDsplcmnt`、`07:DD1D/EB49/FB70 type`、`01:82CD data1F` | 见 §3.1 | 见 §7.1 | A |
| 2 | `$6B0B-$6B13` | 9 | `EnsExtra.1.hi…data1F` | `EnsExtra` 槽1 尾 | 全局 | 同型（`x=0x10` 起步，写点指令同上） | 同 §3.1 | 同 §7.1 | A |
| 3 | `$6B1B-$6B23` | 9 | `EnsExtra.2.hi…data1F` | `EnsExtra` 槽2 尾 | 全局 | 同型（`x=0x20`） | 同 §3.1 | 同 §7.1 | A |
| 4 | `$6B2B-$6B33` | 9 | `EnsExtra.3.hi…data1F` | `EnsExtra` 槽3 尾 | 全局 | 同型（`x=0x30`） | 同 §3.1 | 同 §7.1 | A |
| 5 | `$6B3B-$6B43` | 9 | `EnsExtra.4.hi…data1F` | `EnsExtra` 槽4 尾 | 全局 | 同型（`x=0x40`） | 同 §3.1 | 同 §7.1 | A |
| 6 | `$6B4B-$6B53` | 9 | `EnsExtra.5.hi…data1F` | `EnsExtra` 槽5 尾 | 全局 | 同型（`x=0x50`） | 同 §3.1 | 同 §7.1 | A |
| 7 | `$6B5B-$6B63` | 9 | `EnProjectilesExtra.0.hi…data1F` | `EnProjectilesExtra` 槽0 尾 | 全局 | 同型（`x=0x60`） | 同 §3.1 | 同 §7.1 | A |
| 8 | `$6B6B-$6B73` | 9 | `EnProjectilesExtra.1.hi…data1F` | `EnProjectilesExtra` 槽1 尾 | 全局 | 同型（`x=0x70`） | 同 §3.1 | 同 §7.1 | A |
| 9 | `$6B7B-$6B83` | 9 | `EnProjectilesExtra.2.hi…data1F` | `EnProjectilesExtra` 槽2 尾 | 全局 | 同型（`x=0x80`） | 同 §3.1 | 同 §7.1 | A |
| 10 | `$6B8B-$6B93` | 9 | `EnProjectilesExtra.3.hi…data1F` | `EnProjectilesExtra` 槽3 尾 | 全局 | 同型（`x=0x90`） | 同 §3.1 | 同 §7.1 | A |
| 11 | `$6B9B-$6BA3` | 9 | `EnProjectilesExtra.4.hi…data1F` | `EnProjectilesExtra` 槽4 尾 | 全局 | 同型（`x=0xA0`） | 同 §3.1 | 同 §7.1 | A |
| 12 | `$6BAB-$6BB3` | 9 | `EnProjectilesExtra.5.hi…data1F` | `EnProjectilesExtra` 槽5 尾 | 全局 | 同型（`x=0xB0`） | 同 §3.1 | 同 §7.1 | A |
| 13 | `$6BB4-$6BD3` | 32 | `EnExplosionsExtra.0…3.status…hi` | `EnExplosionsExtra` 槽0–3 全体 | 全局 | `07:F651 INC $6AF4,X status`、`07:F670 STA $6AFB,X hi`、`07:F68D→$6BB9/$6BBA`、`07:E094→$6BB7/$6BB8` | 见 §7.1 末行（字面 0，经 `x≥0xC0` 间接） | seed 字面 0（§7.2） | A |

「写点」列给的是**槽 0 的代表地址**；其余 11 段是同一指令换 `x`（脚本 §10.2 按字段统计，不按槽重复计数）。
`bank:addr` 取自 `reference/seed/metroid.asm` 的 `bankNN:XXXX` 行；bank01–05 同地址各一份（`areas_common`/区文件同源）。

12×9 + 32 = **140**。段 1–12 的起点通式 `$6AFB + 16k`（k=0…11），对应「第 `k` 个 16 B 槽的 +7」。

### 3.1 段 1–12 的字段级 xref（同一组字段，每段各一份，故不重复列 12 行）

| 字段 | 偏移 | asm 行 | 写 | 读 | RMW | 说明 |
|---|---|---|---|---|---|---|
| `hi` | +7 | 29 | 10 | 16 | 1 | 命名表号（`$00` 表0 / `$01` 表3）；`prg7_engine.asm:8951 sta EnsExtra.0.hi,x ;Store name table.` |
| `subPixelY` | +8 | 5 | 3 | 1 | 1 | 亚像素 Y；`areas_common.asm:899/902`、`enemies/rinka.asm:20/144/145` |
| `subPixelX` | +9 | 5 | 3 | 1 | 1 | 亚像素 X；`areas_common.asm:973/976`、`enemies/rinka.asm:21/154/155` |
| `accelY` | +10 | 18 | 12 | 4 | 0 | 加速度 Y；`areas_common.asm:449/526/528/848`、`metroid.asm:98/184/329`、`pipe_bug.asm:28/40`、`swooper.asm:85/112`、`rio.asm:13/40`、`squeept.asm:26`、`prg7_engine.asm:11640`（7 个文件） |
| `accelX` | +11 | 9 | 5 | 3 | 1 | 加速度 X；`areas_common.asm:462/479/481/927/932`、`metroid.asm:92/187/328` |
| `spare1C` | +12 | **0** | 0 | 0 | 0 | asm 注 `Unused`（`constants_ram.asm:139`），全游戏 0 处引用 |
| `jumpDsplcmnt` | +13 | 13 | 3 | 3 | 5 | 跳跃位移计数（`inc/dec`）；`areas_common.asm:686/1056/1134/1206/1279`、`skree.asm:24/29` |
| `type` | +14 | 38 | 7 | 26 | 0 | 敌种索引，作 `L968B` 等参数表下标；`prg7_engine.asm:11338/11350` |
| `data1F` | +15 | 11 | 2 | 6 | 2 | 位域：bit6/7 位移符号、bit2–3 静置敌属性（`prg1_brinstar.asm:418` 注释）；`areas_common.asm:216/282/344/404/704/725/752` |

计数复跑命令见 §10.2。注意 `spare1C` 行是**否定式断言**，已逐条 grep：0 命中，不是"没找"。

## 4. 逐区活跃矩阵（运行期，10 009 份 trace）

样本：`analysis/traces/*.ram.hex`（2272 B = `$0000-$07FF` 镜像 + `$6AF4-$6BD3`）；区号取自文件名
`_a([0-4])_r`；`other` = 非 sweep 用例（movie/door 等）。每格 = 「该段任一字节非零的 trace 数 / 该区 trace 总数」。
**快照边界**：矩阵取自 2026-11-26 取证时的 **10 009 份** trace；仓库 trace 数仍在增长（交付时 `ls | wc -l`
已是 11 116），复跑 §10.3 会打印当时的 `total`，比值随样本漂移属正常，结论（13 段全局共享、无区绑定）不随样本数变化。

| 段 | a0 (3505) | a1 (3833) | a2 (347) | a3 (288) | a4 (282) | other (1754) |
|---|---|---|---|---|---|---|
| EnsExtra 槽0 | 1126 | 1039 | 95 | 40 | 72 | 647 |
| EnsExtra 槽1 | 1251 | 808 | 78 | 32 | 84 | 699 |
| EnsExtra 槽2 | 1028 | 1005 | 88 | 32 | 80 | 866 |
| EnsExtra 槽3 | 1370 | 1288 | 76 | 24 | 72 | 413 |
| EnsExtra 槽4 | 3505 | 1547 | 88 | 16 | 60 | 1085 |
| EnsExtra 槽5 | 3505 | 1383 | 52 | 16 | 52 | 1171 |
| EnProjectilesExtra 槽0 | 0 | 119 | 0 | 0 | 0 | 14 |
| EnProjectilesExtra 槽1 | 0 | 76 | 0 | 0 | 0 | 8 |
| EnProjectilesExtra 槽2 | 0 | 72 | 0 | 0 | 0 | 2 |
| EnProjectilesExtra 槽3 | 0 | 66 | 0 | 0 | 0 | 0 |
| EnProjectilesExtra 槽4 | 0 | 25 | 0 | 0 | 0 | 2 |
| EnProjectilesExtra 槽5 | 0 | 0 | 0 | 0 | 0 | 0 |
| EnExplosionsExtra | 0 | 0 | 0 | 0 | 0 | 345（非零偏移 = 3,4,5,6） |

读法（都写进 §11 的边界）：

- 12 段在 5 个区**都有**非零 ⇒ 全局共享，不存在「每区一份」的分段。槽 4/5 在 a0 恒非零，
  说明 ens 槽号与敌种无关（池化复用），不能据「哪槽活跃」推区。
- 弹额外面只有槽 0 的 `status`（offset 0）会非零 —— 弹的 9 B 尾在样本里全 0。
  **不代表尾字段永不写**（弹道代码可能读到 0 就退出），只说明样本未覆盖到。
- 爆炸只在 `other`（movie 用例）里活跃 345 份，5 个 sweep 区 0 份：sweep 场景太短、没炸到敌人；
  **不是区绑定证据**。

## 5. 语义分组

| 组 | 段 | 语义 | 归属 |
|---|---|---|---|
| A | 段 1–6 | 敌槽额外面尾 9 B（`EnExtra` +7…+15） | `EnsExtra` 6×16 B 的后半 |
| B | 段 7–12 | 弹槽额外面尾 9 B（同结构） | `EnProjectilesExtra` 6×16 B 的后半 |
| C | 段 13 | 爆炸额外面 4×8 B | `EnExplosionsExtra`，字段只到 `hi` |

无「`$6Axx` 区数组」这一类：本窗口内 13 段全部落在上面三个数组里，没有跨到 `$6Axx` 的其他数组
（`$6AF4` 之前的 `$6AFB` 已被 12 段起点覆盖；`$6AF4-$6AFA` 是槽 0 头 7 B，属 `enemy/state.rs` Owner）。

## 6. 逐字节布局（4 段可复核样例）

### 6.1 `EnsExtra` 槽 0 全 16 B（`$6AF4-$6B03`）

| 偏移 | 字段 | 宽 | 当前状态 | 语义 |
|---|---|---|---|---|
| +0 | `status` | 1 | Owner | 槽状态（`$00` 空 / `$04` 冰冻 …） |
| +1 | `radiusY` | 1 | Owner | 碰撞半径 Y |
| +2 | `radiusX` | 1 | Owner | 碰撞半径 X |
| +3 | `animFrame` | 1 | Owner | 当前动画帧 |
| +4 | `animDelay` | 1 | Owner | 帧间延时 |
| +5 | `resetAnimIndex` | 1 | Owner | 动画序列起点 |
| +6 | `animIndex` | 1 | Owner | 当前动画下标 |
| +7 | `hi` | 1 | **Unmodeled** | 命名表号 |
| +8 | `subPixelY` | 1 | **Unmodeled** | 亚像素 Y |
| +9 | `subPixelX` | 1 | **Unmodeled** | 亚像素 X |
| +10 | `accelY` | 1 | **Unmodeled** | 加速度 Y |
| +11 | `accelX` | 1 | **Unmodeled** | 加速度 X |
| +12 | `spare1C` | 1 | **Unmodeled** | 保留（asm 注 Unused） |
| +13 | `jumpDsplcmnt` | 1 | **Unmodeled** | 位移计数 |
| +14 | `type` | 1 | **Unmodeled** | 敌种索引 |
| +15 | `data1F` | 1 | **Unmodeled** | 位域 |

实样本 `_tmp_sweep_666066-1795105504_a0_r16_f00960_f60.ram.hex`：

```
$6AF4: 01 00 00 00 00 00 00 01 00 00 00 00 00 00 02 00
$6B04: 01 00 00 00 00 00 00 01 00 00 00 00 00 00 02 00
$6B14: 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
```

读法：槽0/槽1 `status=1`、`hi=1`（表 3）、`type=2`；尾段其余字节 0。槽2 空。

### 6.2 尾 9 B 的类型建议（供实现卡）

全部单字节，**没有多字节量** ⇒ 不需要 `View`（`View only 0` 维持）：

| 字段 | Rust 建议类型 | 取值面证据 |
|---|---|---|
| `hi` | `u8`（0/1） | `prg7_engine.asm:8951` 注释 Store name table；`:11373` 与 `PPUCTRL_ZP` 比较 |
| `subPixelY/X` | `u8` 累加器 | `areas_common.asm:972-981`：`lda EnsExtra.0.subPixelX,x / clc / adc Ens.0.speedSubPixelX,x / sta`，进位 `adc Ens.0.speedX,x` 进像素位移 |
| `accelY/X` | `u8`（参与 `adc`，语义为有符号增量，按字节保真） | `metroid.asm:92/187/328`、`areas_common.asm:927` `adc` |
| `spare1C` | `u8` + 注释 `reserved` | 0 xref + 10009 trace 恒 0 + asm `Unused` |
| `jumpDsplcmnt` | `u8` | `inc/dec` 5 处（`areas_common.asm:1056/1134/1206/1279`） |
| `type` | `u8` | 作参数表下标（`prg7_engine.asm:11338`） |
| `data1F` | `u8` 位域（bit6/7、bit2–3） | `areas_common.asm:216/282/344/404/752` 读、`:725` 写 |

### 6.3 `EnProjectilesExtra` 槽 0 全 16 B（`$6B54-$6B63`）

结构同 `EnExtra`（+0…+15 同上表）。实样本
`_tmp_sweep_2025711-1795331882_a1_r29_f01719_prevend.ram.hex`：

```
$6B54: 02 04 04 42 01 4D 4E 01 00 00 00 00 00 00 00 00
$6B64: 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
```

读法：`status=2`、`radiusY=radiusX=4`、`animFrame=$42`、`animDelay=1`、`resetAnimIndex=$4D`、
`animIndex=$4E`、`hi=1`；**尾 9 B 全 0**（与 §4 矩阵一致）。

### 6.4 `EnExplosionsExtra` 槽 0 8 B（`$6BB4-$6BBB`）

`constants_ram.asm:161-170` 字段顺序：

| 偏移 | 字段 | 语义 |
|---|---|---|
| +0 | `status` | 槽状态（`$00` 空） |
| +1 | `radiusY` | 半径 Y |
| +2 | `radiusX` | 半径 X |
| +3 | `animFrame` | 动画帧 |
| +4 | `animDelay` | 帧间延时 |
| +5 | `resetAnimIndex` | 动画起点 |
| +6 | `animIndex` | 当前下标 |
| +7 | `hi` | 命名表号 |

实样本 `case-movie-100p-sweep5_f04500_s4500.ram.hex`：

```
$6BB4: 00 00 00 F7 03 43 47 00
$6BBC: 00 00 00 00 00 00 00 00
$6BC4: 00 00 00 00 00 00 00 00
$6BCC: 00 00 00 00 00 00 00 00
```

读法：槽0 `animFrame=$F7`、`animDelay=3`、`resetAnimIndex=$43`、`animIndex=$47`，`status/radiusY/
radiusX/hi=0`；槽1–3 全空。`animFrame=$F7` 是 #65 记过的「炸弹击退奇数帧整帧不画」同族值，
说明爆炸额外面确实走共享动画器的字节面，不做范围裁剪。

## 7. xref 全表与 seed 解码缺口

### 7.1 字段级合计（同 §3.1，此处给 seed 侧）

| 字段 | asm 行 | seed 地址条数 | seed 命中示例 |
|---|---|---|---|
| `hi` | 29 | 22 | bank01–05 各 `855A LDA/855F STA/8563 LDA`；bank07 `DDBA/DD02` 区 `E79C/ECAE/F15C/F390/FA57/FB3C` |
| `subPixelY` | 5 | 10 | bank01–05 各 `8383 LDA/838A STA` |
| `subPixelX` | 5 | 10 | `83E3/83EA` ×5 |
| `accelY` | 18 | 20 | `81B4 STA/8206 LDA/820C STA/8341 LDA` ×5 |
| `accelX` | 9 | 25 | `81C3 STA/81D1 LDA/81D7 STA/83A5 ADC/83AF LDY` ×5 |
| `spare1C` | 0 | 0 | — |
| `jumpDsplcmnt` | 13 | 25 | `82AB STA/8446 INC/84A2 DEC/84F9 INC/8555 DEC` ×5 |
| `type` | 38 | 18 | 1–5 区 `80B0 LDY/82FB LDY`；bank07 `DD02 LDA/DD1D STA/EB49 STA/F74B LDY/F85A LDY/FB1B CMP/FB67 LDY/FB70 STA` |
| `data1F` | 11 | 35 | 1–5 区各 `80CC/810F/8148/8182/82B3 LDA`、`82CD STA`、`82E4 LDA` |

### 7.2 缺口（必须写进实现卡的验证范围）

- `reference/seed/metroid.asm` 在 `bank07:F660-F682` 一带是**裸字节**（IDA 未解码），
  因此 `prg7_engine.asm:11281 sta EnsExtra.0.data1F,x`、`:12278/12279 inc/dec` 在 seed 文本里 0 命中。
  ⇒ seed 条数只作下限，asm 行数才作「引用存在」的依据。
- 32 B 窗口在 seed 文本里 **0 条** `$6BB4-$6BD3` 字面操作数（这是 §2.2 扁平寻址的直接后果，
  不是「没人用」）。反证：运行期 345 份 trace 非零（§4）。
- `$6BB4` 段的写入点全部经 `EnsExtra.0.<字段>,x`（x≥`$C0`）或 `EnExplosionsExtra.0.* - EnExplosions + Ens,x`
  间接到达；实现卡不得按「字面地址」搜写点，必须按 `x` 值域验证。

## 8. 与现有面的重叠与 split 点

| 现有面 | 覆盖 | 与本窗口关系 | split 点 |
|---|---|---|---|
| `object_slot_face` | `$0400-$0500` | **不重叠**（不同 WRAM 页） | 无 |
| `sound_ram_face` | `$0600-$068D`（`FACE_END=0x068E`） | **不重叠** | 无 |
| `enemy/state.rs` `EnsExtra` 头 7 B ×12 | `$6AF4+16k … +6` | 12 处紧邻，需在每槽 **+7** 处切 | 12 个切点：`$6AFB, $6B0B, $6B1B, $6B2B, $6B3B, $6B4B, $6B5B, $6B6B, $6B7B, $6B8B, $6B9B, $6BAB` |
| `enemy/explosion.rs` `Explosion`（`$6BB4-$6BD3`，32 B 全 Unmodeled） | 整段 | 同一段，所有权翻转，不是重叠 | 边界 `$6BB4` / 末字节 `$6BD3` |

约束：新 face 的 span 必须**严格从 +7 起、长 9**，不得包含前 7 B（会与现有 Owner 重叠触发
coverage 冲突）；爆炸段要么复用 `explosion.rs` 的既有 face 翻所有权，要么新建 face 但**不能两处同时声明**
同一字节（`tools/nes_map_check.py` 按字节区间查重叠）。

## 9. 落点建议（memmap）与四态算术

### 9.1 建议

1. 新增 `crates/game-core/src/enemy/extra_face.rs`（与 `typed-struct-plan.md:159` 一致）：
   - `EnemyExtraTail`（9 个 `u8` 字段，§6.2），12 个 span `$6AFB+16k` × 9；
   - `ExplosionExtra`（8 个 `u8` 字段，§6.4），4 个 span `$6BB4+8k` × 8。
2. `enemy/explosion.rs:305-312` 的 `impl NesMapped for Explosion` 改为把 32 B 委托给 `ExplosionExtra`
   （或直接改为引用新 face），**不要**同时保留 32 B `Unmodeled`。
3. 现有 `enemy/state.rs:942-970` 的 7 B Owner 与 `:309 extra()` 字节访问器不动；实现卡只把尾 9 B
   从「字节切片」升级为 typed getter/setter，调用方按字段名读写（`no_raw_addr.py` 禁裸地址）。
4. `spare1C` 保留字段 + `reserved` 注释，不删（删了就少 1 B，四态对不上，且破坏 byte 保真）。

### 9.2 四态变化（机械算术）

当前（`python3 tools/nes_map_check.py --list` 实读）：`Owner 1449 / View 23(only 0) / Unmodeled 385 /
Undeclared 438`，`sum 2272`。

本卡范围 = `Unmodeled` 里的 140 B（12×9 + 32）：

```
Owner      1449 + 140 = 1589
Unmodeled   385 - 140 =  245
Undeclared  438 +   0 =  438
View         23 +   0 =   23   (only 0 不变)
sum        1589 + 245 + 438 = 2272   ✓
恒等式     ΔOwner + ΔView = -(ΔUnmodeled + ΔUndeclared) ⇒ +140 + 0 = -(-140 + 0)  ✓
```

（`View` 是正交标记，已含在 `Owner` 里的多字节量才有；本范围全单字节 ⇒ 不新增 View。）

### 9.3 验收判据（给实现卡）

- `nes_map_check.py` 四态 = 上表；`check` PASS24/0/3/1。
- `cargo test -p game-core` 不回退（**取证时**实测 lib 1082 / 各 target 合计 1109 / workspace 1193；
  本卡交付日 `ratchet:rust_test_count` 基线已到 1217，验收以当时的基线为准，不引用本行数字做门槛）。
- 新增测试至少覆盖：`EnemyExtraTail` 读写与既有 `EnsExtra` 头 7 B 不串；`ExplosionExtra` 的
  `animFrame=$F7` 原样保留（不做 0..N 裁剪）；`spare1C` 恒 0 的保真读。
- `l4_diff_run.py run-all` 不回退（当前 171/172，唯一 diff = `case-mechanism-input-conflict` f674 84 px）。

## 10. 复跑命令

### 10.1 段清单与四态

```bash
cd /home/peng/workspace/metroid-decomp
python3 tools/nes_map_check.py --list | grep -E '\$6A(F|B)|6BB4'      # 13 段现状
python3 tools/nes_map_check.py --list | tail -5                        # 四态合计
```

### 10.2 字段级 xref（asm 行/写/读/RMW）

```bash
python3 - <<'EOF'
import re,glob,collections,pathlib
fields=['hi','subPixelY','subPixelX','accelY','accelX','spare1C','jumpDsplcmnt','type','data1F']
W={'sta','stx','sty'};RMW={'inc','dec','adc','sbc','asl','lsr','rol','ror','and','ora','eor'};R={'lda','ldx','ldy','cmp','cpx','cpy','bit'}
for f in fields:
    rows=[]
    for p in glob.glob('refs/m1disasm/SRC/**/*.asm',recursive=True):
        for ln,l in enumerate(open(p,encoding='utf-8',errors='replace'),1):
            if f'EnsExtra.0.{f}' in l: rows.append((pathlib.Path(p).name,ln,l.split(';')[0].strip()))
    ops=collections.Counter()
    for _,_,c in rows:
        m=re.match(r'([a-z]{3})\b',c); ops[m.group(1) if m else '?']+=1
    w=sum(v for k,v in ops.items() if k in W); r=sum(v for k,v in ops.items() if k in R)
    rm=sum(v for k,v in ops.items() if k in RMW)
    print(f'{f:14s} lines={len(rows)} write={w} read={r} rmw={rm} files={sorted(set(x[0] for x in rows))}')
EOF
```

### 10.3 逐区活跃矩阵

```bash
python3 - <<'EOF'
import glob,pathlib,re,collections
seg=[(0x000,'Ens0'),(0x010,'Ens1'),(0x020,'Ens2'),(0x030,'Ens3'),(0x040,'Ens4'),(0x050,'Ens5'),
     (0x060,'Proj0'),(0x070,'Proj1'),(0x080,'Proj2'),(0x090,'Proj3'),(0x0A0,'Proj4'),(0x0B0,'Proj5'),
     (0x0C0,'Expl')]
area_re=re.compile(r'_a([0-4])_r'); agg=collections.defaultdict(lambda: collections.Counter()); tot=collections.Counter()
for p in glob.glob('analysis/traces/*.ram.hex'):
    b=bytes.fromhex(pathlib.Path(p).read_text().strip())
    if len(b)<2272: continue
    ex=b[2048:2272]; m=area_re.search(pathlib.Path(p).name); a=m.group(1) if m else 'other'
    tot[a]+=1
    for off,name in seg:
        n=16 if off<0xC0 else 8
        if any(ex[off:off+n]): agg[name][a]+=1
print('total',dict(tot))
for _,n in seg: print(n, {a:f'{agg[n][a]}/{tot[a]}' for a in ['0','1','2','3','4','other']})
EOF
```

### 10.4 三段 hexdump（不需 Mesen）

```bash
python3 - <<'EOF'
import pathlib
def hexs(fn,start,n,label):
    ex=bytes.fromhex(pathlib.Path(fn).read_text().strip())[2048:2272]
    off=start-0x6AF4
    print(label, f'${start:04X}:', ' '.join(f'{v:02X}' for v in ex[off:off+n]))
hexs('analysis/traces/_tmp_sweep_666066-1795105504_a0_r16_f00960_f60.ram.hex',0x6AF4,16,'EnsExtra.0')
hexs('analysis/traces/_tmp_sweep_2025711-1795331882_a1_r29_f01719_prevend.ram.hex',0x6B54,16,'EnProjectilesExtra.0')
hexs('analysis/traces/case-movie-100p-sweep5_f04500_s4500.ram.hex',0x6BB4,32,'EnExplosionsExtra')
EOF
```

## 11. 假设区（C 级，不进代码）与未覆盖

1. **C1**：弹额外面 9 B 尾「在样本里恒 0」不等于永不写；§4 的 6 行只说明 10 009 份 trace 未采到。
   实现卡可加断言（写 0 读 0 保真），但不得据此删字段。
2. **C2**：`spare1C` 的「Unused」是原版注释结论 + 0 xref + 恒 0 三重一致，仍按 reserved 保留；
   若将来 trace 出现非零，按 `BUG-xxx` 登记而非直接使用。
3. **C3**：`data1F` 的 bit2–3 语义来自 `prg1_brinstar.asm:418` 注释（「bit2-3: bit6-7 of data1F for
   resting enemies」），本文只记录位段存在，不声称完整位图。
4. **未覆盖**：5 区 sweep 场景不产生爆炸（§4 末行），故 32 B 的槽 1–3 在 trace 里全空；
   「4 槽是否都会被同时使用」需实现卡用更长的 movie 场景验证。
5. **未覆盖**：bank07 IDA 未解码区（§7.2）里的写点只能靠 asm 源码行计数，无法做地址级 xref。

## 12. 证据指针

| 结论 | 证据 |
|---|---|
| 三数组布局与步长 | `refs/m1disasm/out/M1_NES_NTSC.sym` 六行；`constants_ram.asm:125-170`、`:1294-1296` |
| 扁平寻址（x=槽偏移） | `prg7_engine.asm:11229-11239`（爆炸循环）、`:11290-11297`（`InitEnAnimIndex`）、`:6660-6710`（`UpdateEnemyAnim`）、`:11267`（唯一 `EnExplosionsExtra` 行） |
| 尾字段顺序 | `constants_ram.asm:125-160`（`EnExtra` 16 字段）、`:139`（`spare1C ; Unused`） |
| 字段引用计数 | §10.2 脚本实跑输出（本文 §3.1/§7.1 数字） |
| 逐区共享 | §10.3 脚本 + 10 009 份 `analysis/traces/*.ram.hex` |
| 逐字节样本 | §10.4 三条 hexdump（文件名已给出，可直接复跑） |
| 现四态 | `python3 tools/nes_map_check.py --list`：Owner 1449 / View 23 / Unmodeled 385 / Undeclared 438 / sum 2272 |
| 既有 typed 头 7 B | `crates/game-core/src/enemy/state.rs:942-970`、`:274 ENS_EXTRA_BASE=0x6AF4` |
| 爆炸 32 B 现状 | `crates/game-core/src/enemy/explosion.rs:21 EXTRA_BASE=0x6BB4`、`:23 EXTRA_STRIDE=8`、`:305-312` |
| 计划批次对应 | `docs/design/typed-struct-plan.md:159`（B3 范围与预期 Owner 增量） |
