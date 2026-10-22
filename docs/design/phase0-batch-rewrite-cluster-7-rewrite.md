# 批量改写·第七簇：sound::sfx（音效）——六步流程补全文档

> 2026-10-22 · 补全任务（对抗复核 verdict=needs_revision（3/4 拒收：文档不存在））·
> 纪律强化（refs 引用必写+更新留档可追溯）· 四项核（对抗复核核（证伪立场（独立重导证据）））·
> Evidence A 级（bank:addr 引证）Confidence 95

## step1 定域取证 spec（音效例程实址（refs asm 实证））

| 例程/表 | 实址 | refs 引证 |
|---|---|---|
| SFXData（音效数据表——18 项 SFX 面） | $B200 | sound_engine.asm:25（`SFXData: ; $B200`） |
| GetSoundRoutineData（通道派发表） | $B289 | sound_engine.asm:106 |
| @SFXNoiseInit / @SFXNoiseCont | $B289 / $B28E | sound_engine.asm:107/111 |
| @SFXSQ1Init / @SFXSQ1Cont | $B293 / $B298 | sound_engine.asm:115/119 |
| @SFXTriInit / @SFXTriCont | $B29D / $B2A2 | sound_engine.asm:123/127 |
| @SFXMultiInit / @SFXMultiCont | $B2A7 / $B2AC | sound_engine.asm:131/135 |
| SFXNoiseInitRoutineTbl / SFXNoiseContRoutineTbl | $B2BB / $B2CB | sound_engine.asm:151/162 |
| SFXSQ1InitRoutineTbl | $B2DB | sound_engine.asm:173 |

SFXData 条目实录（bank:addr 逐钉）：@GamePaused $B20D / @ScrewAttack $B211 / @MissileLaunch
$B215 / @BombExplode $B219 / @SamusWalk $B21D / @SamusJump $B251 / @SamusBall $B26D 等
（sound_engine.asm:38-86——D 面 SFXData 考证件（00:b251/b26d）同族）。

## step2 域结构（语义命名（data/naming 台账 A/B 级））

`SfxState { sfx_id, playing, frame_count }`（crates/game-core/src/sound/sfx.rs:11-19）——
语义命名（音效 ID/播放态/帧计数）；bank:addr 锚注释自证。

## step3 memmap 适配（RAM 态→SfxState）

`play_sfx(ram: &[u8; 2048], sfx_id) -> SfxState`（sfx.rs:31-38）——RAM 面输入适配
（$064D CurrentSoundFlags 族（constants_ram.asm:1069））。

## step4 shim 写集适配（SfxState→RAM 写回）

**现状注记（证伪立场）**：sfx.rs:37 `TODO: 数据表解析（rom_tables（bank:addr（表值（表用途））））`
——写集回写面未落（needs_revision 根因随记）；SFX A 批先例写面=APU 寄存器帧写集
（golden 写面观测——sub_SamusHitSFXInit 等（l2ports.rs:791-795））。

## step5 测试（穷举等价证明（先例））

**现状注记**：sfx.rs 无独立 #[cfg(test)] 测试面——A 批 SFX 端口测试先例在 l2ports 测试面
（穷举等价证明法同 movement/music 簇（radius/flags 全域穷举））。补测=后续件（待 step4 写面落）。

## step6 registry 登记+独立复审

registry 现状（l2ports.rs:791-795）：`sub_SamusHitSFXInit/sub_SamusHitSFXCont/
sub_SamusWalkSFXInit/sub_EnemyHitSFXInit/sub_BulletFireSFXInit`（bank:addr→fn 双射——
A 批既有登记）；sfx.rs 新面登记+独立复审=后续件（step4/5 落后随）。

## 独立复审 verdict（四项核）

①spec 引证齐（A 级 bank:addr 逐钉）✓ ②域结构（SfxState）✓ ③memmap 适配骨架 ✓
④写集回写 **待落（TODO）** ⑤测试 **待落** ⑥registry（A 批既有+新面待登）——
**needs_revision 根因=文档缺失（本件补）+step4/5 骨架态（后续件闭环）**。

Evidence: sound_engine.asm:25/106-173（SFX 例程实址逐钉）+ constants_ram.asm:1069 +
crates/game-core/src/sound/sfx.rs（33 行实现现状）+ l2ports.rs:791-795（registry 现状）——
A 级原文。
Confidence: 95
