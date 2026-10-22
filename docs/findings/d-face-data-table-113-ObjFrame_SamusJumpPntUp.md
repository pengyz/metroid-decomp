# D 面数据表考证第一百一十三件：ObjFrame_SamusJumpPntUp

> L4 就绪批微工单 · Samus 域（跳跃/变球）· 2026-10-22 · 纪律强化（refs 引用必写+更新留档可追溯）· 四项核（对抗复核核（证伪立场（独立重导证据）））

## 表名/表值/表用途/表功能

| 项 | 值 | refs 引证（bank:addr） |
|---|---|---|
| 表名 | ObjFrame_SamusJumpPntUp | common_sprite_data.asm:1197 |
| 表址 | 01:8961（bank1:$8961） | common_sprite_data.asm:1195-1197（`ObjFrame36/37/ObjFrame_SamusJumpPntUp` 三别名同址 $8961） |
| 表值 | 单帧精灵帧数据（OAMDATA_HFLIP+_id_ObjPlace6,$0F,$04 / $69 / $FD,OAMDATA_PRIORITY+$0 / $FE,$7A,$79 / $FE,$78,$77 / $FD,OAMDATA_PRIORITY+OAMDATA_HFLIP+$0 / $22,$07,$08 …） | common_sprite_data.asm:1198-1210 |
| 表用途 | Samus 跳跃+指上帧（`Samus jump and point up` 注释） | common_sprite_data.asm:1194 |
| 表功能 | ObjFramePtrTable 条目（PtrTableEntry ObjFramePtrTable, ObjFrame_SamusJumpPntUp）→ 动画组帧选择 | common_sprite_data.asm:339 |

## 四项核（对抗复核核）

- **独立重导证据**：PtrTableEntry 行 339 + 帧数据行 1194-1210 均独立 grep 取得（非转述）。
- **证伪立场**：别名 ObjFrame36/37 同址 $8961——同一帧数据被三个名字指代（合成名/序号名/语义名），非三份独立数据。语义名 `ObjFrame_SamusJumpPntUp` 采用依据 = `Samus jump and point up` 注释（m1disasm 源注释，A 级字节位）。
- **判定**：真捕获形（帧数据逐字节可读，非模板复制）——A 级实录。

## 判定

**真规则（固定径）**：ObjFramePtrTable 表目 → $8961 帧数据（OAMDATA 序列）——refs asm 引证齐（bank:addr 行号实录）。

Evidence: common_sprite_data.asm:339（PtrTableEntry）+ :1194-1210（帧数据+别名同址）——bank1:$8961。
Confidence: 95
