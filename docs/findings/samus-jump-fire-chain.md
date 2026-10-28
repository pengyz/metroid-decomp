# 跳跃+射击链考证（shell 可玩性缺口（X 跳爆 unported/Z 射无反应）——A 级全链）

> 2026-10-28 · 先通读 m1disasm 全链后实现（用户裁定（禁试错））·
> Evidence A 级（bank:addr）Confidence 95

## 0. 缺口定位（已证实）

- **X 跳爆**：samus_jump（handler.rs:735）尾 `unported!("bank07:D015")`——D015 链全缺。
- **Z 射无反应**：发射消费站**已在**（站姿 LCC5B B⇒FireSamusProjectile（handler.rs:698）/
  跑姿 samL10）+弹生成在（fire_samus_projectile（D1EE 族（goal-r10）））——**真缺口=
  弹生命期**（update_samus_projectiles 仅 DieDelay 倒计时→清（不移/不撞/不画/不爆））。

## 1. 派发与转迁（GoSamusHandler $CC1A，prg7_engine.asm:2718-2734——A 级）

status bit7（负）⇒SamusStand；else JumpEngine(status)：0=Stand(CC36)/1=Run(CCC2)/
2=Jump(D002)/3=Roll(D0E1)/4=PntUp(D198)/5=Door(D3A8)/6=PntJump(**⇒D002 同 Jump 壳**)/
7=Dead/8=Dead2/9=Elevator。**转迁表**（Stand 尾（:2790-2801））：status<sa_Door(5)⇒
JumpEngine(new status)：0=Exit/1=SetSamusRun/2=SetSamusJump(CFC3)/3=SetSamusRoll/
4=SetSamusPntUp。ActionTable($CC87)=[sa_Run,sa_Run,sa_Roll,sa_PntUp]（BitScan 0-3）。

## 2. 站姿 handler（SamusStand $CC36，:2735-2790——A 级）

任键（除 SELECT/START）⇒SetSamusStand_NoFootstep（**D4 保真 BUG 注**：射动画单帧
（asm 注「fire animation only plays for a single frame」——Stage A 保留））；
方向（DOWN/LEFT/RIGHT 或上帧 UP）⇒BitScan→Dir（<2）+ActionTable→status；
**LCC5B：FIRE（Joy1Change|Joy1Retrig bit6（asl/bpl））⇒FireSamusProjectile($D1EE)**；
LCC6E：JUMP（Joy1Change bit7）⇒status←sa_Jump(2)；尾：SetSamusData（A=4）。

## 3. SetSamusJump 入口件（$CFC3，:3430-3465——A 级）

animResetIndex←Y（Jump=0x0C/JumpPntUp=0x35）/animIndex←Y-1（JumpTransition/
JumpPntUpFire）/animDelay←4/JumpDsplcmnt←0/**speedY←$FC（-4 冲量）**/status==1 且
螺攻齿轮⇒ScrewAttack 面/Lx017：SFX_SamusJump/**AccelY←$18（高跳齿轮⇒$12）**（重力常数）。

## 4. SamusJump handler（$D002，:3467-3517——A 级（本卡主件））

停升门（disp≥$20 且 speedY≥0（下落…bit 测速向）且 JUMP 松）⇒StopVertMovement(D147)。
**Lx019（D015）**：①SamusJump_CheckHorzMovement(D055)②LCF2E（固实实体贴墙（r26 端口
（solid_entity_response）复用）③UP（Joy1Status bit2）⇒animResetIndex←JumpPntUp(0x35)
+status←sa_PntJump(6)。Lx020：④SamusJump_CheckFire(D09C)⑤岩浆（SamusInLava≠0 且
Joy1Change bit7）⇒SetSamusJump+SetSamusData_3FrameAnimDelay（岩浆重跳）。
Lx021：**落地检（AccelY==0）**：status==sa_PntJump⇒SetSamusPntUp(CF77)否则
SetSamusStand(CF55)。Lx023：A←$03 jmp SetSamusData(CD6D)。

## 5. SamusJump_CheckHorzMovement（$D055，:3519-3560——A 级）

X=1,Y=0；Joy1Status lsr（RIGHT=bit0）⇒Lx024；dex(X=0)/lsr（LEFT=bit1）无⇒Lx027(X=0)；
dex(X=-1)/iny(Y=1)。Lx024：Y==SamusDir⇒Lx027（同向）；**转向**：status==PntJump⇒
A=animResetIndex（≠Table04[y] 时保持/==时 Table04+1,y）；否则 Table06 同理——
**表族全匀**（Table06=[Jump×3]/Table04=[PntUp×3]（:3566-3571））⇒A 恒=animResetIndex
（重启动画）。Lx026：SetSamusAnim(A)+animDelay←8+SamusDir←Y。Lx027：
**Samus.speedX($0309)←X**（向位（1/0/-1））。SetSamusAnim(CF6B)=Reset+Index 双写←A/
delay←0；SetSamusNextAnim(CF6E)=Index←A/delay←0。

## 6. SamusJump_CheckFire（$D09C，:3573-3590——A 级）

Joy1Change|Joy1Retrig asl/bpl（**FIRE=bit6（B 键）**）⇒退；animResetIndex==JumpPntUp(0x35)
⇒**FireSamusProjectileUpwards($D275)**；否则 FireSamusProjectileForwards($D210)+
animIndex←JumpFire(SetSamusNextAnim)。

## 7. 发射族（D1EE 族，:3807-3975——A 级）

- FireSamusProjectile($D1EE)：UP（Joy1Status bit2）⇒Upwards/否则 Forwards（**分派器**）。
- SearchOpenSamusProjectileslot($D1F7)：Y=$D0/+16 环；空槽（status==0）⇒isHit←0
  （MissileToggle≠0⇒仅 $D0 槽（导弹限槽））；满⇒iny（零标清）。
- InitBullet($D2EB)：**status+=1**（0→1=wa_Bullet 生）/radiusY,X←2/anim←RegularBullet
  （InitObjectXSlotAnimIndex（D2FA 双写+Delay0））。
- Forwards($D210)：MetroidOnSamus≠0⇒退/搜槽/InitBullet/CheckHorizontalWaveBulletFire
  （波束齿轮位6⇒WaveBeam 态+相位面（gear 门））/CheckIceBulletFire（冰齿轮位7⇒
  IceBeam 态+HasBeamSFX|=1）/**DieDelay,y←$0C（12 帧时寿）**/speedX←BulletSpeedXTable
  [Dir]（$04/-$04）/speedY←0/onScreen←1/CheckHorizontalMissileLaunch（MissileToggle
  门（仅 $D0 槽）⇒status←wa_Missile(0x0B)+DieDelay←$FF+MissileCount 递减（耗尽退模
  +SelectSamusPalette））/PlaceBulletAtArmCannon($D306（StoreTemp(Samus)+ApplySpeed
  +LoadTemp——偏移 index=(status<<1|Dir)&3→[$0C,-$0C,$08,-$08]；Y=-$06））/HasBeamSFX
  位7（长束）/SFX_BulletFire（status==1 时）/animIndex←StandFire(SetSamusNextAnim)。
- Upwards($D275)：同构（speedY←$FC/speedX←0/纵臂炮位放（UpwardsOffset 表））。

## 8. 弹生命期（UpdateSamusProjectiles $D4BF，:4272-4545——A 级（Z 真缺口面））

DoOneSamusProjectile($D4CB)：JumpEngine(status)：0=Exit/1=UpdateBullet(常规)/
2=UpdateWaveBullet/3=UpdateIceBullet/4=UpdateBulletExplode/5-10=Bomb 族/11=UpdateBullet
（**导弹共壳**）。
- **UpdateBullet($D4EB)**：UpdatingSamusProjectile($71)←1/UpdateBullet_DeleteIfOffScreen
  （onScreen bit0 清⇒status←0）/ExplodeIfHitSprite（isHit,x≠0⇒清+爆）/
  CollisionWithBG($D609)：GetObjRoomRAMPtr→瓦：**≥$A0（气）⇒Move（speedX/Y→Temp05/04
  +StoreTemp+ApplySpeed+越界删/LoadTemp）+carry=1**；实心⇒母脑撞检+**$4E⇒
  SamusProjectileHitDoorOrStatue（任务卡 2 件（复用））**+CheckBlastTile（爆破瓦
  （$80-$9F）⇒clc+IsBlastTile（瓦爆面））；CheckBulletStat：carry=1（无撞）⇒
  长束免计时/否则 dec DieDelay（0⇒status←0）；carry=0（撞）⇒Explode（anim←
  BulletHit（导弹⇒MissileExplode（0x91））+status←wa_BulletExplode(4)）；
  DrawBullet：**A=1 jsr AnimDrawObject**；dec UpdatingSamusProjectile。
- UpdateBulletExplode($D5C8)：UpdatingSamusProjectile←1；animFrame==FrameNone
  （动画尽）⇒status←0；DrawBullet。

## 9. 移植计划（最小可玩面（lead 判据：起跳/松落/站/UP 指向跳/弹生成/飞出/撞墙爆））

1. samus_jump 尾 D015 链（CheckHorzMovement/LCF2E 复用/UP→PntJump/CheckFire/岩浆/
   落地判/SetSamusData）。
2. fire 族：fire_samus_projectile_upwards（D275）+D1EE 分派（UP⇒Upwards）+
   CheckFire（D09C）。
3. 弹生命期：update_bullet（Move/CollisionWithBG（$4E⇒task-2 复用/实心⇒爆）/Explode/
   DrawBullet（A=1 AnimDrawObject））+update_bullet_explode（status 4 壳）。
4. Wave/Ice（gear 门）：**最小面**（共 UpdateBullet 壳（记账））；Bomb 族（5-10）：
   记账 no-op（防爆（触发面（埋弹）不在判据））；爆破瓦写面（IsBlastTile）：记账
   （瓦写面留后续（撞爆主径已覆盖））。
5. 共享件复用（不重写）：solid_entity_response（LCF2E）/samus_projectile_hit_door_or_
   statue（任务卡 2）/update_obj_anim+anim_draw_object（绘制）/make_room_ram_ptr（GetObj-
   RoomRAMPtr 公式同核）/set_samus_stand（CF55）/set_samus_jump（CFC3）/set_samus_data。

Evidence: prg7_engine.asm:2718-2825（派发/站姿/ActionTable）/3430-3465（SetSamusJump）/
3467-3590（SamusJump/CheckHorz/CheckFire）/3807-3975（发射族）/4272-4545（弹生命期
六件）——A 级原文。
Confidence: 95
