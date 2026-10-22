# D 面数据表考证第一百一十九件：工单模板错位辨正+去重留档

> L4 就绪批微工单 · Samus 域（跳跃/变球）· 2026-10-22 · 四项核（对抗复核核（证伪立场（独立重导证据）））

工单文本给「ObjFrame_SamusRunPntUpFire1（01:89e6（common_sprite_data.asm（289（PtrTableEntry
ObjFramePtrTable, ObjFrame06）））」——**名/行引错位**（同 117 件模板症）。refs asm 实核：

| 工单项 | 实际归属 | refs 引证 | 已归档 |
|---|---|---|---|
| 名 ObjFrame_SamusRunPntUpFire1（01:89E6） | :354 PtrTableEntry / :1330 帧数据 | common_sprite_data.asm:354,1330 | 第一百一十五件 d-face-data-table-115-ObjFrame_SamusRunPntUpFire1.md |
| 行引 :289（PtrTableEntry ObjFrame06） | ObjFrame06（01:8802） | common_sprite_data.asm:289,831 | 第一百一十七件 d-face-data-table-117-ObjFrame06.md |

**判定**：两题均已立档（一符号一账，不重复立档）——本件为错位辨正+去重留档，非新表考证。

Evidence: common_sprite_data.asm:289（ObjFrame06 条目）+ :354（RunPntUpFire1 条目）+ :1330（帧数据）
——与 115/117 件 refs 引证一致（独立复核）。
Confidence: 95
