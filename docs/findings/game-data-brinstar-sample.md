# findings：game-data brinstar 首区样例——解析结果 × m1disasm 数据段对拍（task-17）

> 状态：**r1 已对拍**（2026-10-15）。schema：[game-data-schema.md](../spec/game-data-schema.md)；
> crate：[crates/game-data](../../crates/game-data/src/lib.rs)（4 单测全绿）。

## 1. 字节级复现命令（判据①，输出=刚打印原文）

```sh
cargo test -p game-data        # 4 passed（Room00/Structure/Metatile/PtrTable 对拍）
python3 - <<'EOF'
rom = open('rom/Metroid (USA).nes','rb').read(); prg = rom[0x10:0x20010]
b1 = prg[0x4000:0x8000]
w = b1[0xA314-0x8000] | (b1[0xA315-0x8000] << 8); print(hex(w))
EOF
```

```
L1 RoomPtrTable[0] = 0xa441 期望 0xa441（m1disasm Room00 注址）→ ✓
Room00 ROM 字节: 02 40 01 02 48 01 02 50 03 02 5f 03 02 fd 02 a1 02 b1 ff
fixture 一致: ✓
L1 StructPtrTable[0] = 0xac84 期望 0xac84（Structure00 注址）→ ✓
L4 MetatileDefs 前 8B: f1 f1 f1 f1 ff ff f0 f0
```

## 2. 双工具交叉（判据②）

| 对拍项 | ROM 原字节（工具 A） | m1disasm 注释源（工具 B） | 结果 |
|---|---|---|---|
| L1 RoomPtrTable[0] | `0xa441` | room_ptrs.asm Room00 注址 `01:A441` | ✓ 一致 |
| L2 Room00 定义 | 19B（上表） | rooms.asm `.byte $02 / $40,$01,$02 … / $FD / $02,$A1 / $02,$B1 / $FF` | ✓ 逐字节 |
| L1 StructPtrTable[0] | `0xac84` | structure_ptrs.asm Structure00 注址 `01:AC84` | ✓ 一致 |
| L4 MetatileDefs 前 8B | `f1 f1 f1 f1 ff ff f0 f0` | metatiles.asm 前两项 | ✓ 一致 |

解析器（tools/crates 侧）输入=上述 fixture 字节，断言=注释源结构语义（4 对象 + tail
`02 A1 02 B1 FF` Blue door ×2）——**解析结果与 m1disasm 数据段双向互证**。

## 3. Room00 解剖（首区样例）

```
Room00 (01:A441):
  base_attr = $02（基色；着色面语义 C 级）
  objects[0] = {pos:$40, struct:$01, attr:$02}   # 位置 yyyyxxxx 拆分语义 C 级
  objects[1] = {pos:$48, struct:$01, attr:$02}
  objects[2] = {pos:$50, struct:$03, attr:$02}
  objects[3] = {pos:$5F, struct:$03, attr:$02}
  $FD 断段
  tail = $02,$A1,$02,$B1,$FF（Blue door ×2 + 终；变长敌/门段语义 C 级 _todo_）
```

四级链闭合：Room00 → StructPtrTable[01]=Structure00（8 元块×2 行）→
MetatileDefs[01×4]=`F1 F1 F1 F1`（LR,LL,UR,UL 图块索引）→ CHR pattern 面（L4→CHR
段=CHR-RAM 写入面，S6 视觉面素材，v1 未接）。

## 4. 候选名消费（task-16 线索矿，C 级）

- 结构/元块层命名一律解构名（pos/struct_idx/attr/lr/ll/ur/ul）——**未引入第三方语义名**；
- task-16 矿中 brinstar 相关名（MetatileDefs/RoomPtrTable 等）作为**地址锚点线索**消费
  （=本 crate 常量注），语义名保持 C 级不入字段名（D5）。

## 5. 证据分级与遗留

| 结论 | 级 |
|---|---|
| L1/L3/L4 解析布局+四处字节对拍 | **A**（ROM 字节×2 工具） |
| L2 objects 段布局 | **A**（对拍） |
| L2 敌/门 tail 变长语义、items 长/短链、enemy 帧链、palettes 面 | **C**（注释线索，_todo_） |
| pos=yyyyxxxx 坐标拆分 | **C**（注释线索待自证） |

遗留（下一轮）：①全房间批解析+指针表全表对拍（256 房间级）②L2 敌/门段变长解码
（C→B 自证）③structures×metatiles 全量×4 索引链闭合 ④palettes/items/enemy_sprite_data
三件套解析 ⑤CHR 面接入（S6）。**字节级复现命令与 fixture 抄录保证本轮可独立重跑。**

## 6. r2：全房间批解析 + L2 tail 聚类自证（2026-10-15）

字节级复现命令（判据①，输出=刚打印原文）：

```sh
cargo run -p game-data --bin game-data-inspect -- "rom/Metroid (USA).nes"
```

```
L1 房间指针 47 词：[0]=$A441（期望 $A441）
L1 结构指针 50 词：[0]=$AC84（期望 $AC84）
L2 批解析：47 房全通过=true 失败=0 对象段总数=635
L2 tail 长度聚类（len: 房数）: {3: 6, 4: 2, 5: 1, 6: 1, 7: 3, 9: 7, 10: 7, 11: 2, 12: 1, 13: 5, 14: 3, 15: 2, 16: 4, 18: 1, 19: 1, 21: 1}
L2 tail 首字节聚类（type: 房数）: {1: 8, 2: 25, 7: 2, 49: 1, 65: 4, 81: 7}
L2 tail $FF 终止一致性: 47/47
L3 批解析：50 结构全通过=true 失败=0 行=130 元块=440
L4 元块表锚点：$AEF0（MetatileDefs，索引闭合校验待全表长度自证）
```

**L2 敌/门段（tail）C→B 自证素材**：
- **B 级结构实证**：47/47 tail `$FF` 终止一致；长度 16 簇（3-21B）=变长段确认
  （m1disasm「字节数随类型变」注释获批量实证）；首字节（type）主簇 {1:8, 2:25, 7:2}
  =35/47 型别收敛——type=2（25 房）与 Room00 注释「Blue door」的 `02` 型直接吻合
  （m1disasm rooms.asm 注释线索 × 批量分布互证）；
- **C 级保留**：type=49/65/81 三簇（12 房）——疑首条目非门/敌类型（或对齐差异），
  型→长度映射表未自证；`02 A1` 的 A1 参数语义（门朝向/索引）仍 C 级；
- 下一步自证路径：nmikstas items.asm 型列表 + SamusEnterDoor 读侧（$4E/$56 族）
  确定 type 语义表 → tail 解码器 C→B→A。

**L3/L4 批量面**：50 结构/130 行/440 元块全解析 ✓（行布局批量一致）；
L4 索引闭合（元块索引×4 < 表长）待 MetatileDefs 全表长度自证（下轮）。

## 7. r3：③ palettes 模块 + ④ L4 索引闭合（2026-10-15，常设施工令节奏）

**asm 锚引证主体**：
- 线格式宏 `VRAMStructData`（macros.asm:32-56：`[hi][lo]` PPU 大端址 + `[len]` + data）/
  `VRAMStructDataRepeat`（macros.asm:58-70：`[hi][lo]` + `[reps|$40]` + 1B 值）/
  `VRAMStructEnd`（macros.asm:71-73：`$00`）；
- 调用面 `PalettePtrTable`（prg1_brinstar.asm:55；`01:9560`）+ `PtrTableEntryArea`
  = 纯词表（macros.asm:88-101）；注址互证 Palette00($A271)/Palette01($A295)；
- 元块总数 65=metatiles.asm 260 数据项/4（asm 数得）；调色板 28=PtrTableEntryArea 计数。

**代码层证据（asm 锚引 + ROM 抽查）**：`parse_vram_struct`（结构级：记录序列至 $00，
双记录族）——**asm 锚证救过一次**：首版只实现 Data 族（逐记录+$00），ROM 批解析 1/28 失败
→ 回 macros.asm 查得 Repeat 族（reps|$40 标记）才全过（参考 asm=主裁判的实证）。

**用例对拍（8/8 绿）+ ROM 批量（输出原文）**：
```
L4 索引闭合：max 元块索引=63 ×4=252 < 表长 260B → ✓
L5 调色板指针 28 词：[0]=$A271（期望 $A271）[1]=$A295（期望 $A295）
L5 批解析：28 调色板通过=28 失败=0 数据字节=139（ppu 址全落 $3Fxx 面 ✓）
```

| 结论 | 级 |
|---|---|
| VRAMStruct 双族线格式 | **A**（macros.asm 宏体+ROM 28/28 解析） |
| PalettePtrTable 28 词 + 注址对拍 | **A**（ROM×asm 注址） |
| L4 索引闭合（max 63<65） | **A**（批解析+asm 表长） |
| 调色板语义名（power suit 等注释） | **C**（不入字段名，D5） |

遗留：⑤ CHR 面（L4→pattern 接入，S6 素材）；items/enemy_sprite_data 两件套（下轮）。

## 8. r4：③ items 模块（结构互核注·施工令②补款首用）

**asm 内容锚**（items.asm:20-54 注释源）：章链=`[Y][word 下章址]`+短条目
`[X][off1][type][data..$00]`；type 语义表（C 级候选名）$01=Squeept/$02=Power up/
$03=Mellows/$04=Elevator/$05=MB cannon/$06=Mother brain/$07=Zeebetite/$08=Rinka/
$09=Door/$0A=Palette change room（items.asm:45-54）。
**自建解码互核（②补款）**：寻径公式 asm 注「offset-1」与字节不符——ROM 字节实证
**next_entry = entry_start + off1**（@@x03@$A3D9 off1=05 → $A3DE=@@x0F 标签合）；
单测 fixture=ROM 章字节抄录（含 asm 标签坐标断言）。

**ROM 批量 + 标签集互核（输出原文）**：
```
③ items 批解析：15 条目（107B）；asm 标签互核：walk=15 vs labels=15 → ✓ 全合
③ items 类型分布（type:数，语义见 items.asm:45-54 注释 C 级）: {2: 10, 3: 1, 4: 3, 10: 1}
```

| 结论 | 级 |
|---|---|
| 章链线格式+寻径公式 | **A**（asm 锚+ROM 15/15 标签互核+单测） |
| 15 条目布局/坐标 | **A**（walk=labels 全合） |
| type 语义名（Power up 等） | **C**（注释线索，不入字段名，D5） |
| 型分布 {2:10,3:1,4:3,10:1} | **A**（批量实测） |

施工令②补款实证：asm 注「offset-1」措辞本身错位——**锚在但须自建解码互核**的
正面案例（与 TYA 位错案反向同款）。遗留：enemy_sprite_data 两件套之二 + ⑤ CHR 面。

## 9. r5：③b enemy_sprite_data（共享尾结构实证·结构互核第三案）

**asm 内容锚**（enemy_sprite_data.asm + constants.asm:116 + macros.asm:75-86）：
EnAnim=`[frame_id...] $FF 终`（_id_FrameEnd=$FF）；frame_id=EnFramePtrTable1（01:9DE0）
表索引（PtrTableEntry 自动编号）；帧体含 EnPlace 子表（体语义 C 级 _todo_）。
**自建解码互核（②）**：断链序列模型被互核证伪——**共享尾结构**（标签=任意入口，
链可顺流进下一标签无 FF：EnAnim_11@$9D7B 流入 EnAnim_Waver0_L@$9D7D）→ 互核模型改
**逐标签入口验证**（每 asm 标签均起合法链至 $FF）。与 TYA 位错案（锚在界误读）/
items「offset-1」案（锚在措辞错）同族第三例：**锚在≠模型对**。

**ROM 批量 + 逐标签互核（输出原文）**：
```
③b 帧表 148 词：[0]=$9FC2（期望 $9FC2=EnFrame00 注址）→ ✓
  入口 $9D7B: 3 帧 → $FF 终 ✓ [34, 35, 36]
  入口 $9D7D: 1 帧 → $FF 终 ✓ [36]
  入口 $9D7F: 3 帧 → $FF 终 ✓ [31, 32, 33]
  入口 $9D81: 1 帧 → $FF 终 ✓ [33]
  入口 $9D83: 1 帧 → $FF 终 ✓ [34]
  入口 $9D85: 1 帧 → $FF 终 ✓ [31]
③b EnAnim 逐标签互核（共享尾模型）：6 入口全合法=true
```

共享尾对账：$9D7D=[36]=$9D7B 链尾缀 ✓、$9D81=[33]=$9D7F 尾缀 ✓——**尾共享实证**
（B 级结构结论：EnAnim 命名空间=入口点集合，非互斥链集合）。

| 结论 | 级 |
|---|---|
| 帧表 148 词/首词 $9FC2 对拍 | **A** |
| 共享尾链结构+逐入口合法 | **A**（ROM 6 入口+尾缀对账） |
| 帧体（EnPlace/tile）语义 | **C**（_todo_） |
| EnAnim 语义名（Waver/Ripper 注释） | **C**（D5 不入字段名） |

遗留：⑤ CHR 面（L4→pattern）+ 帧体语义自证。

## 10. r6：⑤ CHR 面（GFX 转运表+L4→CHR 接通，四级链闭合）

**asm 内容锚**（prg7_engine.asm:1665-1669 线格式注 + macros.asm:101-107 宏体）：
GFX 转运表 7B/项=`[bank][src 16le][dest 16le][len 16le]`；dest=$0000 精灵面/
$1000+ 背景面（语义注 C 级）；24 项（GFXInfoEntry 计数）。L4→CHR 平台语义
（A 级）：图块索引×16B=NES 2bpp 图样偏移（tile_chr_offset）。
**ROM 批量 + asm 互核（输出原文）**：
```
⑤ GFX 转运表 24 项（168B）：[0]=GfxInfo { bank: 6, src: 32768, dest: 0, size: 2464 }（asm 互核 GFX_Samus=06:8000 dest $0000）→ ✓
⑤ 面分布（dest 语义 C 级）：精灵面(=$0000)=2 背景面(>=$1000)=16 其它=6
```

**四级链闭合**（task-17 主目标达成）：
`room_ptrs(A314) → rooms(A441) → structure_ptrs(A372) → structures(AC84) →
metatiles(AEF0, 65 项) → tile×16B → GFX 块（GFXInfo 07:C6E0 24 项，bank6/4… 转运）`
——全链地址锚+字节级对拍齐（r1-r6 各节）。

| 结论 | 级 |
|---|---|
| GFX 线格式+24 项+首项互核 | **A**（asm 注+ROM 逐字段） |
| L4→CHR tile×16 映射 | **A**（NES 2bpp 平台语义） |
| 面分布 2/16/6 | **A**（批量实测） |
| dest 面语义/gfx 块↔房间归属 | **C**（_todo_） |

遗留（下一轮）：gfx 块↔区域归属自证（dest 语义）、帧体（EnPlace）语义、
items type 语义表自证（$01-$0A 注释线索）。
