# D 面数据表考证第一百一十五件：ObjFrame_SamusRunPntUpFire1

> L4 就绪批微工单 · Samus 域（跳跃/变球）· 2026-10-22 · 纪律强化（refs 引用必写+更新留档可追溯）· 四项核（对抗复核核（证伪立场（独立重导证据）））

## 表名/表值/表用途/表功能

| 项 | 值 | refs 引证（bank:addr） |
|---|---|---|
| 表名 | ObjFrame_SamusRunPntUpFire1 | common_sprite_data.asm:1330 |
| 表址 | 01:89E6（bank1:$89E6） | common_sprite_data.asm:1330（`; $89E6`） |
| 表值 | OAMDATA_HFLIP+($0<<4)+_id_ObjPlace6,$0F,$04 / $FE / $69 / $FD,OAMDATA_PRIORITY+$0 / $7A,$79 / $FE / $78,$77 / $FD,OAMDATA_PRIORITY+OAMDATA_HFLIP+$0 / $22,$23 / $FE / $32 … | common_sprite_data.asm:1331-1344+ |
| 表用途 | Samus 跑动+指上+开火帧 1（单帧精灵帧数据） | 表值 OAMDATA 序列（同 113 件法） |
| 表功能 | ObjFramePtrTable 条目（PtrTableEntry ObjFramePtrTable, ObjFrame_SamusRunPntUpFire1）→ 动画组帧选择 | common_sprite_data.asm:354 |

## 四项核（对抗复核核）——命名错位实录

- **独立重导证据**：PtrTableEntry 行 354 + 帧数据行 1330-1344+ 均独立 grep 取得。
- **证伪立场（关键发现）**：anim→frame 命名错位——`ObjAnim_SamusRunPntUpFire1`（$85AD，:137）引 `_id_ObjFrame_SamusRunPntUpFire0`（:143），`ObjAnim_SamusRunPntUpFire2`（$85AF，:141）引 `_id_ObjFrame_SamusRunPntUpFire1`（:143 后行）。**ObjFrame_SamusRunPntUpFire1 的实际消费者 = ObjAnim_SamusRunPntUpFire2**（非同名 anim）。命名差 1 档，非数据错。
- **判定**：真规则（固定径）——帧数据逐字节可读（非模板复制），anim 引用错位=命名债（不影响表读径）。

## 判定

**真规则（固定径）**：ObjFramePtrTable 表目 → $89E6 帧数据（OAMDATA 序列）——refs asm 引证齐。anim 消费者=ObjAnim_SamusRunPntUpFire2（$85AF）。

Evidence: common_sprite_data.asm:354（PtrTableEntry）+ :1330-1344+（帧数据）+ :137-143（anim 引用错位实录）——bank1:$89E6。
Confidence: 95
