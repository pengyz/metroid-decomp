# D 面数据表考证第一百一十七件：ObjFrame06（工单模板错位辨正）

> L4 就绪批微工单 · Samus 域（跳跃/变球）· 2026-10-22 · 纪律强化（refs 引用必写+更新留档可追溯）· 四项核（对抗复核核（证伪立场（独立重导证据）））

## 工单模板错位（证伪更正文化·原地记录）

工单文本给「ObjFrame_SamusJumpPntUp（01:8961（common_sprite_data.asm（289（PtrTableEntry
ObjFramePtrTable, ObjFrame06）））」——**名与行引错位**。refs asm 实核：

- :289 = `PtrTableEntry ObjFramePtrTable, ObjFrame06`（表址 01:8802，:831）
- :339 = `PtrTableEntry ObjFramePtrTable, ObjFrame_SamusJumpPntUp`（表址 01:8961，已归
  第一百一十三件 docs/findings/d-face-data-table-113-ObjFrame_SamusJumpPntUp.md）

本件按工单**行引**（:289）立题 = ObjFrame06；JumpPntUp 不重复立档。

## 表名/表值/表用途/表功能

| 项 | 值 | refs 引证（bank:addr） |
|---|---|---|
| 表名 | ObjFrame06（别名 ObjFrame_SamusFront 同址） | common_sprite_data.asm:831-832 |
| 表址 | 01:8802（bank1:$8802） | common_sprite_data.asm:831（`; $8802`） |
| 表值 | ($0<<4)+_id_ObjPlace0,$0F,$04 / $09 / $FD,OAMDATA_PRIORITY+OAMDATA_HFLIP+$0 / $09 / $FD,OAMDATA_PRIORITY+$0 / $FE / $19,$1A / $FD,OAMDATA_PRIORITY+$0 / $29,$2A / $FE / $39 … | common_sprite_data.asm:833-844+ |
| 表用途 | Samus 正面朝向帧（`Samus facing forward` 注释） | common_sprite_data.asm:830 |
| 表功能 | ObjFramePtrTable 条目（:289）→ 动画组帧选择；ObjFrame06 与 ObjFrame_SamusFront 别名同址（同 113 件三别名法） | common_sprite_data.asm:289 |

## 四项核（对抗复核核）

- **独立重导证据**：:285-295（PtrTableEntry 序列）+ :828-844+（帧数据）独立 sed 取得。
- **证伪立场**：①工单模板名/行错位已辨正（上节）；②别名同址=同一帧数据两名指代（ObjFrame06 序号名 vs ObjFrame_SamusFront 语义名），语义名采用依据=`Samus facing forward` 源注释（:830）。
- **判定**：真规则（固定径）——帧数据逐字节可读（非模板复制）。

## 判定

**真规则（固定径）**：ObjFramePtrTable 表目（:289）→ $8802 帧数据（OAMDATA 序列）——refs asm 引证齐。工单名/行错位已辨正（JumpPntUp 归 113 件）。

Evidence: common_sprite_data.asm:289（PtrTableEntry ObjFramePtrTable, ObjFrame06）+ :830-844+
（帧数据+别名同址 $8802+"Samus facing forward" 注释）——bank1:$8802。
Confidence: 95
