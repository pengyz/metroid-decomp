# findings：RoomPtrTable 对表支撑（game-data 四级链仓——porter 间接读对表源）

> 状态：**对表件**（2026-10-18）。用途=porter 实现 RoomPtrTable 间接读（GetRoomNum/
> SetupRoom 链）时的四级链字节格式+API 对表。基体=task-17 game-data 四级链考证
> +crates/game-data 解析器（已测 14+ 用例）。

## 1. 四级链（task-17 定谳——bank1 窗地址锚）

```
room_ptrs      ($A314) ─→ rooms        ($A441)  [Y][word][X][off1][type][data…$00]
                          └→ structure_ptrs ($A372) ─→ structures ($AC84)
                               └→ metatiles   ($AEF0, 65 项) ─→ tile×16B
                                    └→ GFXInfo ($C6E0, 24 项×7B) [bank][src][dest][len]
```

## 2. 字节格式×game-data API 对表

| 层 | 格式 | API（crates/game-data） |
|---|---|---|
| room_ptrs | 16 位指针表 | `parse_ptr_table(data, n)` |
| rooms | [Y][ptr][X][off1][type][data…$00]（长条目 7B 头） | `parse_room(data)→(RoomDef, len)` |
| structure_ptrs | 16 位指针表 | `parse_ptr_table` |
| structures | 变长瓦行（行内平铺瓦号） | `parse_structure(data)→(Vec<Vec<u8>>, len)` |
| metatiles | 4 瓦×2 属性（16B/项） | `parse_metatile(data)→(MetatileDef, len)` |
| GFXInfo | 7B/项 [bank][src lo/hi][dest lo/hi][len lo/hi] | `parse_gfx_info(data, n)` |
| VRAM 结构 | [hi][lo][ctrl][data…]（hi=0 终止） | `parse_vram_struct(data)`（rendering-chain §2.1 同源） |
| 特殊道具 | 见 task-36 ItemData 对表 | `parse_spec_items(data, base)` |

## 3. RoomPtrTable 间接读语义（porter 对表点）

- GetRoomNum（$E720 族）：MapPos→room_ptrs 索引→rooms 条目扫描（Y/X 比对）→
  RoomNumber($5A)——**间接读=指针表+条目遍历双层**；
- SetupRoom（$EA2B）：RoomNumber→rooms 条目→structure_ptrs 逐层解→metatile 铺排
  →WRAM $6000 房间 RAM（rendering-chain §2.3 四页拷贝源）；
- 条目遍历语义：Y 比对 `cmp MapPosY / beq/bcs`（Ascending 半序扫描——r3 已考面）；
- **端序/跨页注意**：ptr 16 位 LE；条目 data 段 $00 终止（长条目 off1≠$FF 续链）。

## 4. 窗口 bank 歧义注（task-14 面随件）

- 案面仓 yield 注同源：$8xxx 跳板桩簇首的**物理 bank 未定**（窗口映射态决定
  prg_byte 读址——重定向歧义已回退实录）；
- RoomPtrTable 同面：**$A314 等地址=窗口态地址**——bank1 映射时有效（物理 bank 1）
  ——间接读实现需带 window_bank 参数（L2In.window_bank 契约面同源）；
- task-14 消歧后：casegen 产率改进（跳板桩重定向）可重试+本对表地址锚定物理化。
