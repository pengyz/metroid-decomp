# game-data 四级数据链 schema（区域五件套解析，task-17）

> 状态：v1（2026-10-15）。依据：asm 架构考古（m1disasm 数据段注释=格式文档源，
> 已证等价原则下格式注释升 A 级线索；**字段语义自证状态逐字段标注**）。
> 样例：brinstar 首区（bank1）；对拍素材见
> [game-data-brinstar-sample.md](../findings/game-data-brinstar-sample.md)。

## 0. 四级引用链（关卡=数据链）

```
room_ptrs → rooms → structure_ptrs → structures → metatiles → tile(CHR)
                ↘ palettes（着色面）
                ↘ items / enemy_sprite_data（实体面）
```

| 级 | 数据 | 格式（m1disasm 注释源，A 级线索） | 自证状态 |
|---|---|---|---|
| L1 | RoomPtrTable | N × word（房间定义指针） | 字节级已证（对拍） |
| L2 | RoomDef | 1B 属性（基色）+ 3B 结构段（yyyyxxxx 位置、StructPtrTbl 索引、属性）…$FD 断段 + 敌/门变长段…$FF 终 | 结构段已证；**敌/门段变长语义 C 级** |
| L3 | StructureDef | 每行=1B 行宽 + nB 元块索引（×4=MetatileDefs 索引）…$FF 终 | 已证 |
| L4 | MetatileDefs | 4B/元块（LR,LL,UR,UL 图块索引→CHR pattern） | 已证 |

## 1. 区域五件套（bank 布局，锚点=m1disasm `01:XXXX` 注，brinstar=bank1）

| 文件 | 锚点 | 内容 |
|---|---|---|
| room_ptrs.asm | 01:A314 RoomPtrTable | 房间指针表 |
| rooms.asm | 01:A441 Room00 起 | 房间定义（L2） |
| structure_ptrs.asm | 01:A372 StructPtrTable | 结构指针表 |
| structures.asm | 01:AC84 Structure00 起 | 结构定义（L3） |
| metatiles.asm | 01:AEF0 MetatileDefs | 元块定义（L4） |
| palettes.asm | — | 调色板（着色面，格式注释见源文件） |
| items.asm | — | 特殊物品（长/短条目：Y 坐标/word 链/X 坐标/偏移/类型/数据…$00 终） |
| enemy_sprite_data.asm | 01:9D72 起 | 敌人动画帧链（EnAnim/EnFrame/_id_FrameEnd 终） |

## 2. 解析层契约（crates/game-data）

- 输入=**ROM/prg 字节**（memmap 域：bank NN 窗口 $8000 / bank7 固定 $C000；
  字节级复现命令见 findings 对拍文）；
- 纯函数解析（`&[u8]`→结构），地址常量带 `Evidence` 字段（m1disasm `文件:行` +
  bank:addr）；
- 候选名消费 task-16 线索矿：本 crate 内命名一律 **C 级**（`_todo_` 前缀或解构字段名），
  语义自证后走 data/naming 台账（AGENTS D5）；
- 字段语义未证者标 `_todo_`（如敌/门段变长布局）。

## 3. 已证 vs 未证清单（v1）

**已证（字节级对拍）**：L1 指针表 ↔ m1disasm RoomXX 标签址逐词一致；L3 结构行布局；
L4 元块 4 字节序（LR,LL,UR,UL）。
**C 级待自证**：L2 敌/门段变长编码；items 长/短条目链（注释线索在案）；
enemy_sprite_data 帧链索引语义（_id_* 常量族）；palettes 面布局。
