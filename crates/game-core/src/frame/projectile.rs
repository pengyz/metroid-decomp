//! 弹道与命中判定：Samus 弹/敌弹推进、门与雕像命中、子弹碰撞环。
//!
//! 由 `frame/mainloop.rs` 原样搬出（纯搬移）。asm 根：`UpdateSamusProjectiles` 族、
//! `SamusProjectileHitDoorOrStatue ; 07:E81E`、`CollisionDetection` 族。

use crate::enemy::behaviour::*;
use crate::frame::tick::*;
use crate::ram_map;

pub fn samus_projectile_hit_door_or_statue(
    mem: &mut crate::frame::FrameMemory,
    temp04_lo: u8,
    temp04_hi: u8,
) {
    // 弹径门（UpdatingSamusProjectile（$71（constants:600））——非弹更新径⇒即退）
    if mem.read_ram(0x0071) == 0 {
        return; // ClcExit
    }
    let mut x: i16 = 6;
    while x >= 0 {
        let xb = x as u8;
        let ptr_hi = mem.read_ram(0x005D + xb as u16);
        let ptr_lo = mem.read_ram(0x005C + xb as u16);
        if ((temp04_hi ^ ptr_hi) & 0x04) == 0 && ((temp04_lo ^ ptr_lo) & 0x1F) == 0 {
            let y = (xb.wrapping_mul(8)) | 0x80; // Amul8|80（槽偏移（$0300 基））
            if mem.read_ram(0x0300 + y as u16) != 0 {
                // 门存在——类型门（DoorType=$307+y（lsr/奇=蓝））
                let door_type = mem.read_ram(0x0307 + y as u16);
                let pi = mem.read_ram(0x004B) as u16; // PageIndex（弹槽偏移）
                let pstat = mem.read_ram(0x0300 + pi);
                let hit = if door_type & 1 != 0 {
                    true // 蓝/音乐蓝：任意弹伤
                } else {
                    // 红/导弹：仅 wa_Missile($0B) 或（wa_BulletExplode($04) 且
                    // animResetIndex==$91（ObjAnim_MissileExplode（$8603-$8572（表算））））
                    pstat == 0x0B
                        || (pstat == 0x04 && mem.read_ram(0x0305 + pi) == 0x91)
                };
                if hit {
                    if door_type & 1 == 0 {
                        // @hitByMissile：SFXTriInitFlags($0683)|=sfxTri_SamusBall(2)
                        // 【写径面校】SFXTriInitFlags($0683)=2KB RAM 镜像（非 wram）
                        let f = mem.read_ram(0x0683);
                        mem.write_ram(0x0683, f | 0x02);
                    }
                    // @blueDoor：isHit[$30A+y]←$04（bit2=被弹击（constants:919））
                    mem.write_ram(0x030A + y as u16, 0x04);
                    return; // bne ClcExit
                }
                return; // GotoSFX_Metal（弹开（SFX 面（随件（记账）））——不写 isHit
            }
        }
        x -= 2; // @next：dex/dex/bpl @loop
    }
    // 雕像兜底（环尽无门）：KraidStatueIsHit[$366+idx] inc（idx=Temp04_lo Adiv8 &1）
    let idx = ((temp04_lo >> 3) & 1) as u16;
    let v = mem.read_ram(0x0366 + idx);
    mem.write_ram(0x0366 + idx, v.wrapping_add(1));
}

/// UpdateSamusProjectiles+DoOneSamusProjectile（**asm 内容锚** `: 07:D4BF/D4CB`——r26）：
/// 3 槽（Objects 13-15=$D0/$E0/$F0）状态 dispatch：0=ExitSub（空径——移动对拍主径！）/
/// 1=Bullet/2=Wave/3=Ice/4=Explode/5-10=Bomb 族/11=Missile（打敌人流程件——随件）。
pub fn update_samus_projectiles(mem: &mut crate::frame::FrameMemory) {
    for slot in [0xD0u8, 0xE0, 0xF0] {
        mem.write_ram(0x004B, slot); // PageIndex（DoOneSamusProjectile（$D4CB））
        let status = mem.read_ram(0x0300 + slot as u16);
        match status {
            0 => {} // ExitSub
            1 | 11 => update_bullet(mem), // UpdateBullet（常规/导弹共壳（:4288））
            4 => update_bullet_explode(mem), // UpdateBulletExplode（$D5C8）
            2 | 3 => {
                // Wave/Ice（gear 门径（无波/冰齿轮不触发）——最小面共 UpdateBullet 壳
                crate::frame::DIVERGENCES.with(|d| d.borrow_mut().push(0xD522)); // 轨迹面（记账）
                update_bullet(mem);
            }
            _ => {
                // Bomb 族（5-10（埋弹触发面（不在本卡判据））——记账 no-op（防爆）
                crate::frame::DIVERGENCES.with(|d| d.borrow_mut().push(0xD4CB));
            }
        }
    }
}

/// 弹爆径（UpdateBullet_Explode（$D5E4（audit §8）））：anim←BulletHit(0x1D)
/// （导弹⇒MissileExplode(0x91)）+status←wa_BulletExplode(4)。
pub(crate) fn bullet_explode(mem: &mut crate::frame::FrameMemory) {
    let x = mem.read_ram(0x004B) as u16;
    let base = 0x0300u16 + x;
    let st = mem.read_ram(base);
    if st == 4 {
        return; // 已爆（asm beq @RTS）
    }
    let anim = if st == 0x0B { 0x91 } else { 0x1D };
    mem.write_ram(base + 5, anim);
    mem.write_ram(base + 6, anim);
    mem.write_ram(base + 4, 0);
    mem.write_ram(base, 4);
}

/// 弹移动（UpdateBullet_Move（$D624（audit §8）））：speedX/Y→位移（wrapping）。
pub(crate) fn bullet_move(mem: &mut crate::frame::FrameMemory) {
    let x = mem.read_ram(0x004B) as u16;
    let base = 0x0300u16 + x;
    let sx = mem.read_ram(base + 9);
    let sy = mem.read_ram(base + 8);
    mem.write_ram(base + 0x0E, mem.read_ram(base + 0x0E).wrapping_add(sx));
    mem.write_ram(base + 0x0D, mem.read_ram(base + 0x0D).wrapping_add(sy));
}

/// UpdateBullet（$D4EB（audit §8——Z 射真缺口面（2026-10-28 全链通读端口）））：
/// UpdatingSamusProjectile($71)←1/删除（onScreen 清）/精灵撞（isHit）/BG 撞
/// （空气（≥$A0）⇒移+免撞；实心⇒爆（$4E 门瓦⇒任务卡 2 径复用））/时寿 dec（长束免）/
/// DrawBullet（AnimDrawObject(A=1)）。
pub(crate) fn update_bullet(mem: &mut crate::frame::FrameMemory) {
    let x = mem.read_ram(0x004B) as u16;
    let base = 0x0300u16 + x;
    mem.write_ram(0x0071, 1);
    // DeleteIfOffScreen（$D5FC）：onScreen bit0 清⇒删
    if mem.read_ram(base + 0x0B) & 1 == 0 {
        mem.write_ram(base, 0);
        mem.write_ram(0x0071, 0);
        return;
    }
    // ExplodeIfHitSprite（$D5DA）：isHit,x≠0⇒清+爆
    let mut collided = false;
    if mem.read_ram(base + 0x0A) != 0 {
        mem.write_ram(base + 0x0A, 0);
        bullet_explode(mem);
        collided = true;
    }
    if !collided {
        // CollisionWithBG（$D609）：瓦判（GetObjRoomRAMPtr 同核（make_room_ram_ptr））
        let (lo, hi) = crate::frame::room::make_room_ram_ptr(
            mem.read_ram(base + 0x0D), mem.read_ram(base + 0x0E), mem.read_ram(base + 0x0C));
        let tile = mem.read_wram(((hi as u16) << 8) | lo as u16);
        if tile >= 0xA0 {
            bullet_move(mem); // 空气⇒移（carry=1（免撞））
        } else {
            if tile == 0x4E {
                // 门瓦⇒任务卡 2 弹击径（复用（isHit&4 写））+爆（carry=0）
                samus_projectile_hit_door_or_statue(mem, lo, hi); // 同模块（任务卡 2 件）
            } else if tile >= 0x80 {
                crate::frame::DIVERGENCES.with(|d| d.borrow_mut().push(0x8D2F)); // 瓦爆面（记账）
            }
            collided = true;
        }
    }
    // CheckBulletStat（$D4F8）
    if !collided {
        let d = mem.read_ram(base + 0x0F).wrapping_sub(1);
        mem.write_ram(base + 0x0F, d);
        if d == 0 {
            mem.write_ram(base, 0); // 时寿耗尽⇒清
        }
    } else if mem.read_ram(base) != 0 {
        bullet_explode(mem); // 撞⇒爆（status≠0 时）
    }
    // DrawBullet（$D517）：AnimDrawObject(A=1)（动画推进面 A=1）
    // 【goal-轮103 双调修】asm AnimDrawObject(DE47)=UpdateObjAnim+ObjDrawFrame 单含——
    // 旧形显式 update_obj_anim(1)+anim_draw_object 补调=双调 ✗⇒删显式（补调覆盖）。
    crate::render::oam::anim_draw_object(mem, mem.read_ram(0x004B), 1); // DrawBullet A=1
    mem.write_ram(0x0071, 0);
}

/// UpdateBulletExplode（$D5C8）：animFrame==_id_FrameNone($F7)（动画尽）⇒清；
/// DrawBullet。
pub(crate) fn update_bullet_explode(mem: &mut crate::frame::FrameMemory) {
    let x = mem.read_ram(0x004B) as u16;
    let base = 0x0300u16 + x;
    mem.write_ram(0x0071, 1);
    if mem.read_ram(base + 3) == 0xF7 {
        mem.write_ram(base, 0);
    }
    // 【goal-轮103 双调修】同上（爆径 AnimDrawObject 单含）。
    crate::render::oam::anim_draw_object(mem, mem.read_ram(0x004B), 1); // DrawBullet A=1
    mem.write_ram(0x0071, 0);
}

/// UpdateSkreeProjectiles（**asm 内容锚** `UpdateSkreeProjectile: ; 07:FBEC`（prg7:12157-
/// 12242 全谱——goal-轮13 件））：①dieDelay 倒数 ②速表（FC5D——(Y,X)：(0,−5)/(−5,−2)
/// /(−5,+2)/(0,+5)）③位移+出界⇒Kill ④**Samus 5 伤害**（:12214-12232——i 帧/螺旋转免伤）。
pub(crate) fn uw_skree_projectiles(mem: &mut crate::frame::FrameMemory) {
    // 【卡 E D2-2 端口】asm UpdateAllSkreeProjectiles 循环头（prg7_engine.asm:12146-
    // 12148）：`lda #$40 / sta PageIndex`——显示面共用 PowerUpDraw($340) 槽。
    mem.write_ram(0x004B, 0x40);
    const SPEED: [(i8, i8); 4] = [(0, -5), (-5, -2), (-5, 2), (0, 5)]; // FC5D 逐字
    for (si, yy) in [12u16, 8, 4, 0].iter().enumerate() {
        let base = 0x00A0u16 + yy;
        let d = mem.read_ram(base);
        if d == 0 {
            continue; // 空槽
        }
        mem.write_ram(base, d.wrapping_sub(1)); // dec dieDelay
        let (vy, vx) = SPEED[si];
        let mut py = mem.read_ram(base + 1);
        let mut px = mem.read_ram(base + 2);
        let (ny, oy) = py.overflowing_add(vy as u8);
        let (nx, ox) = px.overflowing_add(vx as u8);
        if oy || ox {
            mem.write_ram(base, 0); // Kill（出界——asm bcc KillSkreeProjectile）
            continue;
        }
        py = ny;
        px = nx;
        mem.write_ram(base + 1, py);
        mem.write_ram(base + 2, px);
        // 【goal-轮22 弹显示态件】UpdateSkreeProjectile（:12191-12205 逐字）：位姿写
        // PowerUpDraw（$340 槽——y/x/hi）——显示面状态（ObjDrawFrame SkreeProjectile
        // 帧（_id_ObjFrame_SkreeProjectile）随件）。
        mem.write_ram(0x0340 + 13, py); // PowerUpDraw.y（Object.y=$34D）
        mem.write_ram(0x0340 + 14, px); // PowerUpDraw.x（Object.x=$34E）
        mem.write_ram(0x0340 + 0x0C, mem.read_ram(base + 3) & 1); // PowerUpDraw.hi
        // （Object.hi=$34C——旧形误写 +7=data07；PowerUpDraw=instanceof Object
        // （constants_ram:817「just a regular object」——Object 字段位移权威））
        // 【goal-轮23 弹显示帧】（:12207-12212 逐字：animFrame←_id_ObjFrame_SkreeProjectile
        // +`jsr ObjDrawFrame`——asm txa/pha/pla=X 存还（ObjDrawFrame 内 ldx PageIndex
        // 覆 X）非 PageIndex 换槽（旧形误读）。
        // 【goal-轮24 弹帧 id 实证】ObjFramePtrTable（common_sprite_data:282-388）索引
        // 自证（ObjFrame2C=表位 0x2C ✓ 名实符）⇒SkreeProjectile=表位 **0x5A**
        // （:373−:283=90）——旧猜值 0x2C（=ObjFrame2C ✗）。
        mem.write_ram(0x0340 + 3, 0x5A); // PowerUpDraw.animFrame（_id_ObjFrame_SkreeProjectile）
        // 【卡 E D2-2】asm :12212 `jsr ObjDrawFrame`（DE4A **纯绘** PowerUpDraw($40) 槽
        // ——循环头 `lda #$40/sta PageIndex`（:12146-12148）；旧形 anim_draw_object
        // ((base&0xFF),0)（DE47 幻影走表+错绘 Door 槽 $3A0+yy）。
        crate::render::oam::obj_draw_frame(mem, 0x40);
        // Samus 5 伤害（:12214-12232）
        if mem.read_ram(crate::ram_map::SAMUS_INVINCIBLE_DELAY) == 0 {
            let sy = mem.read_ram(0x030D) as i16;
            let sx = mem.read_ram(0x030E) as i16;
            let sry = mem.read_ram(0x0301) as i16;
            let srx = mem.read_ram(0x0302) as i16;
            let dy = (sy - py as i16).unsigned_abs() as u16;
            let dx = (sx - px as i16).unsigned_abs() as u16;
            if dy < sry as u16 + 4 && dx < srx as u16 + 4 {
                if !crate::samus::physics::is_screw_attack_active(mem) {
                    mem.write_ram(0x010F, 0x80);
                    mem.write_ram(crate::ram_map::SAMUS_INVINCIBLE_DELAY, 0x32);
                    // 【goal-轮39 伤害源标记】mainloop:2193（敌撞伤害）。
                    mem.write_ram(0x006E, 0x50); // HealthChange←$50（5 伤害）
                    mem.write_ram(0x006F, 0x00);
                }
            }
        }
    }
}

/// CollisionDetection（**asm 内容锚** Lx269（F09D）:9970-10004 敌×弹环——**击杀链接
/// 线**（用户报「无法被子弹击杀」根因：本件原为 stub！））：每敌（status≠0∧≠Explode(3)
/// ∧≠Pickup(5)）×每弹（status∈{1,2,3,7,0A,0B}（asm 过滤 :9991-10000））→判盒
/// （F140：半径和（EnsExtra+Objects）+屏距）→F2CA（弹 status→weaponAction+弹 isHit 位
/// +敌 isHit 标志）→下帧 EnemyReactToSamusProjectile（F536——伤害/爆亡 ✓已端口）。
/// 注：SamusCollisionWithSolidEntities（D976）在 physics 链（不动）。
pub(crate) fn uw_collision_detection(mem: &mut crate::frame::FrameMemory) {
    // 【goal-轮20 碰撞环头件】CollisionDetection（F034-F038 逐字）：`lda #$FF/sta
    // SamusKnockbackIsBomb($73)/sta SamusHurt010F($010F)`——**每环重置 kb73=$FF**（无此
    // 件则 CheckHealthStatus 的击退向位径（bomb≠0 门）永不发（金腿 kb77=$FF 实证）。
    mem.write_ram(crate::ram_map::SAMUS_KNOCKBACK_IS_BOMB, 0xFF);
    mem.write_ram(0x010F, 0xFF);
    for s in 0..6u16 {
        let x = s * 16; // 字节偏移约定（Ens/EnsExtra 步距 16——asm 6 槽环 :10021 Xminus16）
        let st = mem.read_wram(0x6AF4 + x);
        if st == 0 || st == 3 || st == 5 {
            continue; // 空/爆中/Pickup（:9975-9986）
        }
        for py in [0x03D0u16, 0x03E0, 0x03F0] {
            let pstat = mem.read_ram(py);
            // asm 弹型过滤（:9991-10000）：<wa_BulletExplode(4)（1/2/3=束族）∨7∨0A∨0B
            if !matches!(pstat, 1 | 2 | 3 | 7 | 0x0A | 0x0B) {
                continue;
            }
            // 判盒（F140——半径和+屏距（拾取触碰同构：屏位纯差））
            let ey = mem.read_ram(0x0400 + x + 0) as i16;
            let ex = mem.read_ram(0x0400 + x + 1) as i16;
            let ery = mem.read_wram(0x6AF4 + x + 1) as i16;
            let erx = mem.read_wram(0x6AF4 + x + 2) as i16;
            let py_y = mem.read_ram(py + 0x0D) as i16;
            let py_x = mem.read_ram(py + 0x0E) as i16;
            let pry = mem.read_ram(py + 1) as i16;
            let prx = mem.read_ram(py + 2) as i16;
            let dy = (ey - py_y).unsigned_abs() as i16;
            let dx = (ex - py_x).unsigned_abs() as i16;
            if dy >= ery + pry || dx >= erx + prx {
                continue; // 未触碰（carry 置⇒F2CA bcs Lx293）
            }
            // ==== F2CA 命中面（:10391-10407）====
            mem.write_ram(0x0400 + x + 0x0E, pstat); // weaponAction←弹 status
            mem.write_ram(py + 0x0A, mem.read_ram(py + 0x0A) | 0x20); // 弹 isHit（SetSamusProjectileIsHit）
            // GetEnemyIsHitFlags（方向位：bit5=被武器/bit4=自顶/bit3=自左——简化形）→or isHit
            let mut flags = 0x20u8;
            if ey < py_y {
                flags |= 0x10;
            }
            if ex < py_x {
                flags |= 0x08;
            }
            let cur = mem.read_ram(0x0400 + x + 4);
            mem.write_ram(0x0400 + x + 4, cur | flags);
        }
        // ==== 敌×萨姆斯（:10009-10019——**碰撞伤害**（用户报「没有碰撞伤害」根因）====
        // 无 i 帧($70)∧活→判盒（Y=0=Samus）→F2ED：螺旋转有效⇒RTS（不伤）；否则
        // SamusHurt($010F)←$80+LF270（击退件）+LF306 伤害（AreaEnemyDamage→HealthChange）。
        if mem.read_ram(crate::ram_map::SAMUS_INVINCIBLE_DELAY) == 0 {
            // 【PENDING-001 根因闭合（2026-11-09）】IsSamusDead 门（asm 07:F0DA）：
            // `jsr IsSamusDead / beq NextEnemy`——IsSamusDead（07:CE84）：
            // status∈{sa_Dead(7),sa_Dead2(8),sa_Begin(255)}⇒Z=1⇒跳过敌×萨判定。
            // emu 实证（exec/read 回调采证）：killenemy 场景 f975-995 窗 Samus.status
            // （$0300）恒 FF（sa_Begin"can't be hurt"淡入完成态）⇒beq 608/608 全走分支、
            // F0DF CheckWithObjectYSlot 0 次执行⇒emu 永不触发 hurt=原版真行为；
            // rust 缺此门⇒f986 误触发（30→22）——本门即根因修复。
            let samus_status = mem.read_ram(0x0300);
            if samus_status == 7 || samus_status == 8 || samus_status == 0xFF {
                continue;
            }
            let sy = mem.read_ram(0x030D) as i16;
            let sx = mem.read_ram(0x030E) as i16;
            let sry = mem.read_ram(0x0301) as i16;
            let srx = mem.read_ram(0x0302) as i16;
            let ey = mem.read_ram(0x0400 + x + 0) as i16;
            let ex = mem.read_ram(0x0400 + x + 1) as i16;
            let ery = mem.read_wram(0x6AF4 + x + 1) as i16;
            let erx = mem.read_wram(0x6AF4 + x + 2) as i16;
            let dy = (sy - ey).unsigned_abs() as i16;
            let dx = (sx - ex).unsigned_abs() as i16;
            // 【goal-轮79 asm 合形修】F1FA hi（名表）距调——旧形无 hi 调=偏离（金方 hi 差
            // ⇒距+256⇒不撞）——hi 距调：hi 差⇒距+0x100（asm @positionHi_notEqual 语义）。
            let s_hi = mem.read_ram(0x030C) & 1; // Samus.hi（Objects.0.hi——F193 勘误）
            let (sry_y_raw, srx_x_raw) = (sy as i32, sx as i32);
            let e_hi = mem.read_wram(0x6AF4 + x + 7) & 1; // EnsExtra.0.hi（F152 ✓）
            let dy_adj = if s_hi != e_hi { dy + 0x100 } else { dy };
            let dx_adj = if s_hi != e_hi { dx + 0x100 } else { dx };
            // Temp10_DistHi（F1FA 构建，槽序「X=Samus、Y=敌」⇒ dy/dx 取「敌 减 Sam」——
            // emu f1112 `isHit=0x05`/`Samus.isHit=0x30` 反推）：0x04 | (dy<0)<<1 | (dx<0)。
            let dist_hi: u8 = {
                let vertical = mem.read_ram(crate::ram_map::SCROLL_DIR) & 0x02 == 0;
                let (yy, yx, yh) = (sry_y_raw, srx_x_raw, s_hi as i32);
                let (ey_i, ex_i, ehi_i) = (ey as i32, ex as i32, e_hi as i32);
                let dyf: i32 = if vertical {
                    (ehi_i * 240 + ey_i) - (yh * 240 + yy)
                } else {
                    ey_i - yy
                };
                let dxf: i32 = if vertical {
                    ex_i - yx
                } else {
                    (ehi_i * 256 + ex_i) - (yh * 256 + yx)
                };
                0x04u8 | (if dyf < 0 { 0x02 } else { 0 }) | (if dxf < 0 { 0x01 } else { 0 })
            };
            // 【goal-轮88 F1FA 判式边值直证】cmp 带位/borrow 链（金方不撞源）。
            // 【敌槽生成/位形卡·hi 源勘误（2026-11-05）】asm F17F/F193：双侧 hi=
            // `(hi ^ PPUCTRL_ZP)&1` 后等值比较（NT 同项相消⇒原始 hi 相等性等价）——
            // **Samus.hi=Objects.0.hi（$030C）**；旧形误读 wram[0x6AF4]（=EnsExtra 空槽
            // 0 的 status！）⇒恒 0⇒名表差漏判+256 距 ⇒morphball f1278 敌撞击退单腿
            // （emu f1167 x 回绕翻 hi=1、敌 hi=0 ⇒拒触 ✓；rust 假 hi=0 ⇒误触）。
            if dy_adj < sry + ery && dx_adj < srx + erx {
                // asm `SetEnemyTouchingSamusFlags ; 07:F2E8`（F282 第 1 件）：
                //   `jsr LF340`(= Temp10_DistHi eor #$03) → `bne Lx292` → `ora Ens.0.isHit,x / sta`
                // ⇒ **敌 isHit |= (DistHi ^ 3)**（bit0 自左侧触 / bit1 自上方触）。
                // 缺此件时金腿 f1112 有 isHit=0x05 而锈腿恒 0（case-002 首分歧）。
                let cur_e = mem.read_ram(0x0400 + x + 4);
                mem.write_ram(0x0400 + x + 4, cur_e | (dist_hi ^ 0x03));
                // 螺旋转（IsScrewAttackActive——carry clear=active）⇒不伤萨姆斯
                let screw = crate::samus::physics::is_screw_attack_active(mem);
                // 【敌槽生成/位形卡·F282 状态门（2026-11-05）】asm F282：螺旋转分岔后
                // `lda EnsExtra.0.status,x / cmp #enemyStatus_Frozen(4) / bcs Exit17`
                // ——**status≥4（Frozen/Pickup/Hurt）免伤**；旧形无此门（Lx269 环头只滤
                // 0/3/5）⇒Frozen/Hurt 敌误伤。环头+本门合成=asm 实效域 {Resting,Active}。
                if !screw && mem.read_wram(0x6AF4 + x) < 4 {
                    // 【goal-轮20 偏离撤除】F282 敌×萨径只写 isHit（SetSamusIsHitByEnemy）
                    // +LF306 伤害——**i 帧/击退速由 CheckHealthStatus（CDFA）统一置**
                    // （旧形此处直写 i 帧+单次推=偏离（金腿 f1652 i70=0 实证））。
                    mem.write_ram(0x010F, mem.read_wram(0x6AF4 + x + 14)); // Hurt010F←敌 type（F298）
                    mem.write_ram(0x006E, 0x80);
                    mem.write_ram(0x006F, 0x00);
                    // asm `SetSamusIsHitByEnemy ; 07:F338`：`lda Temp10_DistHi / asl asl asl`
                    // → `SetSamusIsHitFlags`（`ora Samus.isHit / sta`）⇒ **Samus.isHit |= DistHi<<3**
                    // （bit0→bit3 被推向右 / bit1→bit4 被推向下 / bit2→bit5 恒置）。
                    // 旧形 `0x30 | (ex>sx ? 0 : 8)` 在「敌我同列」时多置 bit3 ⇒ 击退方向反。
                    let cur_s = mem.read_ram(crate::ram_map::SAMUS_IS_HIT);
                    mem.write_ram(crate::ram_map::SAMUS_IS_HIT, cur_s | dist_hi.wrapping_shl(3));
                    // 【goal-轮40 isHit 写者标记】mainloop:2284（敌撞置 hit）。
                }
            }
        }
    }
    // ==== SubtractHealth（CE92 尾——BCD 扣血（:3203-3257））====
    subtract_health(mem);
}

/// SubtractHealth（**asm 内容锚** `SubtractHealth: ; 07:CE92`）：HealthChange≠0∧活⇒
/// （Varia⇒半伤（lsr+carry 补 $4F））→Base10Subtract（BCD）：Health($0106)/Health+1($0107)
/// 双字节扣→ClearHealthChange。
pub(crate) fn subtract_health(mem: &mut crate::frame::FrameMemory) {
    let hc0 = mem.read_ram(0x006E);
    let hc1 = mem.read_ram(0x006F);
    if hc0 == 0 && hc1 == 0 {
        return;
    }
    // 【共享门（lead 裁决卡 2026-11-09）】asm 07:CE98-CEA3：`jsr IsSamusDead / beq
    // GotoClearHealthChange`→`jmp ClearHealthChange(F323)`——status∈{sa_Dead(7),
    // sa_Dead2(8),sa_Begin(255)}⇒清 HealthChange 不扣血；门位在 Varia 半伤/BCD 扣减前。
    // 全部伤害径（:7049 熔岩/:10074 F13D 弹敌/:12232+12716 五伤/$8042←tourian）经本
    // 例程汇聚⇒一处门覆盖全谱；与 PENDING-001 敌碰径门（uw_collision_detection）幂等并存。
    let samus_status = mem.read_ram(0x0300);
    if samus_status == 7 || samus_status == 8 || samus_status == 0xFF {
        mem.write_ram(0x006E, 0);
        mem.write_ram(0x006F, 0);
        return;
    }
    // Varia（gr_VARIA=$20）⇒半伤。asm LCEA9-CEBD 忠实形：`lsr HealthChange（丢弃
    // 进位）/ lsr HealthChange+1（借出 bit0→carry）/ bcc 退出 / lda #$4F / adc
    // HealthChange（carry=1 ⇒ +0x4F+1）`——十位域补偿 +$50（百位借出半额）。
    // 【bcd 掩码审计卡 2026-11-10】旧形 (a+0x4F)>>1 为分歧（$0300→$0127≠asm $0150）。
    let (mut a, mut b) = (hc0, hc1);
    if mem.read_wram(crate::ram_map::SAMUS_GEAR) & 0x20 != 0 {
        let b_carry = b & 1 != 0;
        a >>= 1;
        b >>= 1;
        if b_carry {
            a = 0x4Fu8.wrapping_add(a).wrapping_add(1);
        }
    }
    // Base10Subtract（BCD 逐位减——$C3FB 族简化形：半字节借位）。第二调用 carry-in=
    // 首调用 carry-out（asm :3231-3235 第二次 jsr 前**无 sec**）⇒ borrow 作 +1 并入 b。
    let h0 = mem.read_ram(0x0106);
    let h1 = mem.read_ram(0x0107);
    let (r0, borrow) = bcd_sub(h0, a);
    let (r1, _) = bcd_sub(h1, if borrow { b.wrapping_add(1) } else { b });
    mem.write_ram(0x0106, r0);
    // 【bcd 掩码审计卡 2026-11-10】Health+1 全字节落写（asm :3233 `sta Health+1`
    // 无掩码）：高位 nibble 参与借位/回绕运算（$02+0x10 借位补、负结果 +$A0 调整
    // 合法改写 bit6-5），掩码剥离=分歧（asm $79→旧 rust $19）。证据见
    // docs/findings/bcd-mask-audit-subtracthealth.md。
    mem.write_ram(0x0107, r1);
    mem.write_ram(0x006E, 0);
    mem.write_ram(0x006F, 0);
}

/// BCD 半字节减（Base10Subtract 简化形）：返回 (结果, 借位)。
pub(crate) fn bcd_sub(v: u8, sub: u8) -> (u8, bool) {
    let mut borrow = false;
    let mut out = 0u8;
    for shift in [0u32, 4] {
        let mut d = ((v >> shift) & 0x0F) as i16 - ((sub >> shift) & 0x0F) as i16 - borrow as i16;
        if d < 0 {
            d += 10;
            borrow = true;
        } else {
            borrow = false;
        }
        out |= ((d as u8) & 0x0F) << shift;
    }
    (out, borrow)
}

