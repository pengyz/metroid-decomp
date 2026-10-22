# D 面数据表考证第一百二十三件：ObjFrame09（新表考证·模板源已修后首件）

> L4 就绪批微工单 · Samus 域（跳跃/变球）· 2026-10-22 · 纪律强化（refs 引用必写+更新留档可追溯）· 四项核（对抗复核核（证伪立场（独立重导证据）））

## 表名/表值/表用途/表功能

| 项 | 值 | refs 引证（bank:addr） |
|---|---|---|
| 表名 | ObjFrame09（四别名同址：ObjFrame09/ObjFrame0A/ObjFrame0B/ObjFrame_SamusRunFire0） | common_sprite_data.asm:870-873 |
| 表址 | 01:882C（bank1:$882C） | common_sprite_data.asm:870（`; $882C`） |
| 表值 | OAMDATA_HFLIP+($0<<4)+_id_ObjPlace0,$0F,$04 / $00,$01 / $FD,OAMDATA_PRIORITY+$0 / $4B,$4A,$49 / $FD,OAMDATA_PRIORITY+OAMDATA_HFLIP+$0 / $20,$21 / $FE,$FE / $31 … | common_sprite_data.asm:874-886+ |
| 表用途 | Samus 跑动开火帧（`Samus run and fire.` 注释） | common_sprite_data.asm:869 |
| 表功能 | ObjFramePtrTable 四条目（:292 ObjFrame09 / :293 ObjFrame0A / :294 ObjFrame0B / :295 ObjFrame_SamusRunFire0）同指 $882C；anim 消费者=ObjAnim_SamusRunFire（$8594）引 _id_ObjFrame_SamusRunFire0（:87） | common_sprite_data.asm:292-295,84-88 |

## 四项核（对抗复核核）

- **独立重导证据**：:292-295（PtrTableEntry×4）+ :869-886+（注释+帧数据）+ :84-88（anim 消费径）独立 sed/grep 取得。
- **证伪立场**：四别名同址=**同一帧数据四名指代**（4 行 PtrTableEntry 同指 $882C，非四份独立帧）；语义名采用依据=`Samus run and fire.` 源注释（:869）。anim 实际引 _id_ObjFrame_SamusRunFire0（:87），其余三名（09/0A/0B）无 anim 直引（仅表目占位）。
- **判定**：真规则（固定径）——帧数据逐字节可读（非模板复制）。

## 判定

**真规则（固定径）**：ObjFramePtrTable 四表目（:292-295）→ $882C 帧数据（OAMDATA 序列）；anim 消费径=ObjAnim_SamusRunFire（$8594，:84-88）——refs asm 引证齐。

Evidence: common_sprite_data.asm:292-295（PtrTableEntry×4）+ :869-886+（"Samus run and fire." 注释+帧数据 $882C）+ :84-88（ObjAnim_SamusRunFire 消费径）。
Confidence: 95
