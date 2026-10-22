# D 面数据表考证第一百二十一件：工单模板错位辨正+去重留档（复现）

> L4 就绪批微工单 · Samus 域（跳跃/变球）· 2026-10-22 · 四项核（对抗复核核（证伪立场（独立重导证据）））

工单文本给「ObjFrame_SamusJumpPntUp（01:8961（common_sprite_data.asm（289（PtrTableEntry
ObjFramePtrTable, ObjFrame06）））」——**名/行引错位**（同 117/119 件模板症复现）。refs asm 实核：

| 工单项 | 实际归属 | refs 引证 | 已归档 |
|---|---|---|---|
| 名 ObjFrame_SamusJumpPntUp（01:8961） | :339 PtrTableEntry / :1195-1210 帧数据（三别名同址） | common_sprite_data.asm:339,1195 | 第一百一十三件 d-face-data-table-113-ObjFrame_SamusJumpPntUp.md |
| 行引 :289（PtrTableEntry ObjFrame06） | ObjFrame06（01:8802，别名 ObjFrame_SamusFront） | common_sprite_data.asm:289,831 | 第一百一十七件 d-face-data-table-117-ObjFrame06.md |

**判定**：两题均已立档（一符号一账，不重复立档）——本件为错位辨正+去重留档，非新表考证。
模板症三连（117/119/121）——工单生成器名/行引拼接缺陷已实录（建议 lead 侧修模板源）。

Evidence: common_sprite_data.asm:289（ObjFrame06 条目）+ :339（JumpPntUp 条目）+ :1195（帧数据）
——与 113/117 件 refs 引证一致（独立复核）。
Confidence: 95
