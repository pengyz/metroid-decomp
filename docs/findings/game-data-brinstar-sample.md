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
