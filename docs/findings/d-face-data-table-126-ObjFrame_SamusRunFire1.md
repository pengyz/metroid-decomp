# D 面数据表考证第一百二十六件：ObjFrame_SamusRunFire1

> L4 就绪批微工单 · Samus 域（跳跃/变球）· 2026-10-22 · 纪律强化（refs 引用必写+更新留档可追溯）· 四项核（对抗复核核（证伪立场（独立重导证据）））

## 表名/表值/表用途/表功能

| 项 | 值 | refs 引证（bank:addr） |
|---|---|---|
| 表名 | ObjFrame_SamusRunFire1（单名无别名） | common_sprite_data.asm:890 |
| 表址 | 01:883E（bank1:$883E） | common_sprite_data.asm:890（`; $883E`） |
| 表值 | OAMDATA_HFLIP+($0<<4)+_id_ObjPlace0,$0F,$04 / $00,$01 / $FD,OAMDATA_PRIORITY+$0 / $4B,$4A,$49 / $FD,OAMDATA_PRIORITY+OAMDATA_HFLIP+$0 / $22,$23 / $FE / $32,$33,$34 / $FF（:891-905，$FF 终止） | common_sprite_data.asm:891-905 |
| 表用途 | Samus 跑动开火帧 1（`Samus run and fire.` 注释） | common_sprite_data.asm:889 |
| 表功能 | ObjFramePtrTable 条目（:296）；anim 消费者=ObjAnim_SamusRunFire（$8594）引 _id_ObjFrame_SamusRunFire1（:88） | common_sprite_data.asm:296,85-88 |

## 四项核（对抗复核核）

- **独立重导证据**：:296（PtrTableEntry）+ :889-905（注释+帧数据）+ :85-88（anim 消费径）独立 grep/awk 取得。
- **证伪立场**：与 RunFire0（$882C，123 件）帧头同形（$00/$01/$4B/$4A/$49）但 :899 起分歧（$22,$23 vs RunFire0 的 $20,$21），且 $FF 终止（:905）——**独立帧数据非模板复制**（同族近形两帧）。单名无别名（无 123 件同址多指问题）。
- **判定**：真规则（固定径）。

## 判定

**真规则（固定径）**：ObjFramePtrTable 表目（:296）→ $883E 帧数据（OAMDATA 序列，$FF 终止）；anim 消费径=ObjAnim_SamusRunFire（$8594）——refs asm 引证齐。

Evidence: common_sprite_data.asm:296（PtrTableEntry）+ :889-905（注释+帧数据 $883E+$FF 终止）+ :85-88（ObjAnim_SamusRunFire 消费径）。
Confidence: 95
