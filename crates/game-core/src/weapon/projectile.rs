//! 弹道与命中判定：Samus 弹/敌弹推进、门与雕像命中、子弹碰撞环。
//!
//! 由 `frame/mainloop.rs` 原样搬出（纯搬移）。asm 根：`UpdateSamusProjectiles` 族、
//! `SamusProjectileHitDoorOrStatue ; 07:E81E`、`CollisionDetection` 族。

use crate::enemy::{framework::*, anim_tick::*, skree::*, crawler::*};
use crate::frame::tick::*;
use crate::ram_map;

pub fn samus_projectile_hit_door_or_statue(
    mem: &mut crate::frame::Manager,
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
                let pi = mem.page_index() as u16; // PageIndex（弹槽偏移）
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
                        let f = mem.tri_init_flags();
                        mem.set_tri_init_flags(f | 0x02);
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
pub fn update_samus_projectiles(mem: &mut crate::frame::Manager) {
    for slot in [0xD0u8, 0xE0, 0xF0] {
        mem.set_page_index(slot); // PageIndex（DoOneSamusProjectile（$D4CB））
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
            // Bomb 族（`DoOneSamusProjectile ; 07:D4CB` 表项 5/8=BombInit、
            // 6/9=BombCountdown、7/10=BombExplode——两套半程各一份，asm 原样）
            5 | 8 => crate::weapon::bomb::bomb_init(mem),
            6 | 9 => crate::weapon::bomb::bomb_countdown(mem),
            7 | 10 => crate::weapon::bomb::bomb_explode(mem),
            _ => {
                crate::frame::DIVERGENCES.with(|d| d.borrow_mut().push(0xD4CB));
            }
        }
    }
}

/// 弹爆径（UpdateBullet_Explode（$D5E4（audit §8）））：anim←BulletHit(0x1D)
/// （导弹⇒MissileExplode(0x91)）+status←wa_BulletExplode(4)。
pub(crate) fn bullet_explode(mem: &mut crate::frame::Manager) {
    let x = mem.page_index() as u16;
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

/// 弹移动（`UpdateBullet_Move ; 07:D624`）：`StoreObjectPositionToTemp` →
/// `ApplySpeedToPosition ; 07:FD8F` → `LoadObjectPositionFromTemp ; 07:D638`。
///
/// 【2026-11-20 复核 P1 修】旧形只做 y/x 的 wrapping 加，**既不管越界失败**，也
/// **不维护 `Objects.0.hi`**。asm 真形：
/// - `ApplySpeedToPosition` 的越界分支（y≥$F0 / 和进借位 / x 进位）在有**不允许的
///   滚动方向**（y 越界要竖滚、x 越界要横滚）时 `clc` 返回失败；
///   `UpdateBullet_Move` 的 `bcc Lx078` ⇒ **弹 status←0（删）**。
/// - 越界且方向允许时 `inc Temp0B_PositionHi`（$FDAC/$FDD0）。
/// - `LoadObjectPositionFromTemp` 写回 y/x 后 `lda Temp0B_PositionHi / and #$01 /
///   sta Objects.0.hi,x` ⇒ **hi 必须落写**（旧形从不写，hi 恒为生成时的值）。
pub(crate) fn bullet_move(mem: &mut crate::frame::Manager) -> bool {
    let slot = mem.page_index() / 16;
    debug_assert_eq!(mem.page_index() % 16, 0, "槽号由 x 字节偏移反推，必须 16 的倍数");
    let (mut py, mut px, mut phi, sy, sx) = {
        let o = mem.object(slot);
        (o.y(), o.x(), o.hi(), o.speed_y(), o.speed_x())
    };
    // Temp02_ScrollDir = ScrollDir & $02（bit1 = 横滚）
    let hscroll = mem.read_ram(crate::ram_map::SCROLL_DIR) & 0x02 != 0;
    let mut failed = false;
    // —— y 轴（$FD95-$FDBF）——
    if sy >= 0 {
        if sy != 0 {
            let (ny, carry) = py.overflowing_add(sy as u8);
            if carry || ny >= 0xF0 {
                // $FDA4：+$10（两条进入路径的 carry 都使 ADC #$0F 实际加 $10）
                if hscroll {
                    failed = true;
                } else {
                    phi = phi.wrapping_add(1);
                    py = ny.wrapping_add(0x10);
                }
            } else {
                py = ny;
            }
        }
    } else {
        let (ny, carry) = py.overflowing_add(sy as u8);
        if carry {
            py = ny;
        } else if hscroll {
            // $FDB5：<$00 回绕 ⇒ −$10；只有竖滚允许
            failed = true;
        } else {
            phi = phi.wrapping_add(1);
            py = ny.wrapping_sub(0x10);
        }
    }
    // —— x 轴（$FDC6-$FDDD）——
    if !failed {
        if sx >= 0 {
            if sx != 0 {
                let (nx, carry) = px.overflowing_add(sx as u8);
                if carry {
                    // $FDCB：x≥$100 ⇒ 只允许横滚
                    if hscroll {
                        phi = phi.wrapping_add(1);
                        px = nx;
                    } else {
                        failed = true;
                    }
                } else {
                    px = nx;
                }
            }
        } else {
            let (nx, carry) = px.overflowing_add(sx as u8);
            if carry {
                px = nx;
            } else if hscroll {
                phi = phi.wrapping_add(1);
                px = nx;
            } else {
                failed = true;
            }
        }
    }
    if failed {
        mem.object(slot).set_status(0); // Lx078：SetObjectXSlotStatus ← #$00（越界删弹）
        return false;
    }
    // LoadObjectPositionFromTemp（$D638）：写回 y/x 且 hi←Temp0B & $01
    let mut o = mem.object(slot);
    o.set_y(py);
    o.set_x(px);
    o.set_hi(phi & 0x01);
    true
}

/// UpdateBullet（$D4EB（audit §8——Z 射真缺口面（2026-10-28 全链通读端口）））：
/// UpdatingSamusProjectile($71)←1/删除（onScreen 清）/精灵撞（isHit）/BG 撞
/// （空气（≥$A0）⇒移+免撞；实心⇒爆（$4E 门瓦⇒任务卡 2 径复用））/时寿 dec（长束免）/
/// DrawBullet（AnimDrawObject(A=1)）。
/// `wa_Missile`（`constants.asm:118-127`）。
pub(crate) const WA_MISSILE: u8 = 0x0B;

pub(crate) fn update_bullet(mem: &mut crate::frame::Manager) {
    let x = mem.page_index() as u16;
    let base = 0x0300u16 + x;
    let slot = mem.page_index() / 16;
    let mut deleted = false;
    mem.write_ram(0x0071, 1);
    // DeleteIfOffScreen（$D5FC）：onScreen bit0 清⇒删
    if mem.read_ram(base + 0x0B) & 1 == 0 {
        mem.write_ram(base, 0);
        mem.write_ram(0x0071, 0);
        return;
    }
    // ExplodeIfHitSprite（$D5DA）：isHit,x≠0 ⇒ 清 + 爆（status←4）。
    // **注意**：asm 这里**不设 carry**——ExplodeIfHitSprite 之后仍无条件执行
    // `jsr UpdateBullet_CollisionWithBG`（$D609），撞敌的弹若前方是空气**照样前移**。
    // 旧形把它当「已碰撞」⇒ 跳过 CollisionWithBG ⇒ 弹停在撞点（比原版少移一步）。
    // 实证：录像 f2135——emu `$03D0` status=4 且 x=214（先爆后移），rust 停在 x=210。
    if mem.read_ram(base + 0x0A) != 0 {
        mem.write_ram(base + 0x0A, 0);
        bullet_explode(mem);
    }
    // CollisionWithBG（$D609）恒执行；`collided` 只表示**瓦判为固实**（asm carry 清）。
    let mut collided = false;
    {
        // CollisionWithBG（$D609）：瓦判（GetObjRoomRAMPtr 同核 = Room::addr）
        // 【2026-11-20 复核 P1 修】`GetObjRoomRAMPtr ; 07:D79F` → `MakeRoomRAMPtr ;
        // 07:E96A`（`lda Temp0B_PositionHi / asl / asl / and #$04`）⇒ 对象 hi 是
        // **名表号 0/1**，必须换成页选位。旧形直传 hi ⇒ `& $04` 恒 0 ⇒ 弹的 BG
        // 碰撞永远读 A 页（B 页的固实瓦、门瓦 $4E 全部读不到）。
        let (oy, ox, ohi) = {
            let o = mem.object(slot);
            (o.y(), o.x(), o.hi())
        };
        let ptr = crate::enemy::motion::make_room_ram_ptr(mem, oy, ox, ohi);
        let (lo, hi) = (ptr as u8, (ptr >> 8) as u8);
        let tile = mem.read_wram(ptr);
        // 【判据分叉登记（复核 F3）】瓦判的单源是 `world::room_ram::tile_blocks_non_projectile`
        // （`tile < $A0` ⇒ 阻挡）。子弹径不共用它：这里还要分"门瓦 `$4E` ⇒ 开槽"与
        // "可炸瓦 `$80-$9F` ⇒ 记 DIVERGENCES 并爆"两条副作用，而可走性结论与单源一致
        // （只有 `>=$A0` 才 move）。
        if tile >= 0xA0 {
            // 空气⇒移；`ApplySpeedToPosition` 失败（越界且滚动方向不允许）⇒ carry 清 ⇒
            // 调用方 `CheckBulletStat` 走 @collided 且 status==0 ⇒ `beq Lx069` **跳过**
            // `dec dieDelay` 与 `AnimDrawObject`（复核 F1）。
            if !bullet_move(mem) {
                deleted = true;
            }
        } else if crate::frame::in_area_raw(mem) & 0x0F == 3 {
            // Tourian：`jsr GotoUpdateBullet_CollisionWithMotherBrain` 在固实瓦判之后、
            // `cmp #$4E` 之前（`07:D609-07:D616`），而 `03:A0C6` 的出口是
            // `pla / pla / clc / rts`（双返回）⇒ 跳过 $4E 与可炸瓦后续处理、CF=0 即碰撞。
            crate::enemy::zebetite::tourian_bullet_hit(mem, slot as u8, tile, lo, hi);
            collided = true;
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
    if deleted {
        // status==0 ⇒ `beq Lx069`：连 DrawBullet 一起跳过
    } else if !collided {
        let d = mem.read_ram(base + 0x0F).wrapping_sub(1);
        mem.write_ram(base + 0x0F, d);
        if d == 0 {
            mem.write_ram(base, 0); // 时寿耗尽⇒清
        }
    } else if mem.read_ram(base) != 0 {
        bullet_explode(mem); // 撞⇒爆（status≠0 时）
    }
    // DrawBullet（$D517）：`@collided` 且 status==0 时 asm `beq Lx069` 跳过绘制。
    if !deleted && !(collided && mem.read_ram(base) == 0) {
        // 【goal-轮103 双调修】asm AnimDrawObject(DE47)=UpdateObjAnim+ObjDrawFrame 单含——
        // 旧形显式 update_obj_anim(1)+anim_draw_object 补调=双调 ✗⇒删显式（补调覆盖）。
        crate::render::oam::anim_draw_object(mem, mem.page_index(), 1); // DrawBullet A=1
    }
    mem.write_ram(0x0071, 0);
}

/// UpdateBulletExplode（$D5C8）：animFrame==_id_FrameNone($F7)（动画尽）⇒清；
/// DrawBullet。
pub(crate) fn update_bullet_explode(mem: &mut crate::frame::Manager) {
    let x = mem.page_index() as u16;
    let base = 0x0300u16 + x;
    mem.write_ram(0x0071, 1);
    if mem.read_ram(base + 3) == 0xF7 {
        mem.write_ram(base, 0);
    }
    // 【goal-轮103 双调修】同上（爆径 AnimDrawObject 单含）。
    crate::render::oam::anim_draw_object(mem, mem.page_index(), 1); // DrawBullet A=1
    mem.write_ram(0x0071, 0);
}

/// UpdateSkreeProjectiles（**asm 内容锚** `UpdateSkreeProjectile: ; 07:FBEC`（prg7:12157-
/// 12242 全谱——goal-轮13 件））：①dieDelay 倒数 ②速表（FC5D——(Y,X)：(0,−5)/(−5,−2)
/// /(−5,+2)/(0,+5)）③位移+出界⇒Kill ④**Samus 5 伤害**（:12214-12232——i 帧/螺旋转免伤）。
pub(crate) fn uw_skree_projectiles(mem: &mut crate::frame::Manager) {
    // 【卡 E D2-2 端口】asm UpdateAllSkreeProjectiles 循环头（prg7_engine.asm:12146-
    // 12148）：`lda #$40 / sta PageIndex`——显示面共用 PowerUpDraw($340) 槽。
    mem.set_page_index(0x40);
    const SPEED: [(i8, i8); 4] = [(0, -5), (-5, -2), (-5, 2), (0, 5)]; // FC5D 逐字
    for yy in [12u16, 8, 4, 0] {
        // 【2026-11-16 修：case-002 f1250】速度表按**槽号**索引（asm 更新循环 `ldy` 从 0 递增，
        // 槽 i ↔ SPEED[i]）。旧形用 `enumerate` 序号（循环序 12,8,4,0 ⇒ si=0 对槽 3）⇒四枚弹
        // 方向整体错位（f1250 rust x=0x9C vs emu 0x38）⇒无一命中 Samus ⇒ 缺伤害/击退。
        let si = (yy / 4) as usize;
        let base = 0x00A0u16 + yy;
        let d = mem.read_ram(base);
        if d == 0 {
            continue; // 空槽
        }
        mem.write_ram(base, d.wrapping_sub(1)); // dec dieDelay
        let (vy, vx) = SPEED[si];
        // 【2026-11-16 修：case-002 f1250 真因】`ApplySpeedToPosition ; 07:FD8F` 逐字：
        // 越界不是「u8 加法进位」而是**分轴、按滚动方向**判：
        //  · y 正：`adc pos / bcs` 或 `cmp #$F0 / bcs` ⇒ 包到 240 以上；水平房（ScrollDir&2≠0）
        //    ⇒ @exit_failure；垂直房 ⇒ +$10 回绕并 inc hi。
        //  · y 负：`adc pos / bcs` 无借位=正常；借位 ⇒ 水平房 failure / 垂直房回绕 inc hi。
        //  · x 正/负：与 y **对称相反**——垂直房 failure，水平房回绕 inc hi（永不杀）。
        // 旧形 `overflowing_add` 一判到底 ⇒ 凡负速（`pos + 0xFB`）恒 u8 进位 ⇒ 四枚弹里
        // 三枚在出生帧即被误杀（dieDelay 0 / 位姿停在出生点），仅向 +5 的那枚存活。
        let vy_v = mem.read_ram(crate::ram_map::SCROLL_DIR) & 0x02 != 0; // true=水平房
        let mut py = mem.read_ram(base + 1);
        let mut px = mem.read_ram(base + 2);
        let mut phi = mem.read_ram(base + 3);
        let mut failed = false;
        if vy > 0 {
            let s = py as u16 + vy as u16;
            if s > 0xFF || s >= 0xF0 {
                if vy_v {
                    failed = true;
                } else {
                    py = (s as u8).wrapping_add(0x10);
                    phi = phi.wrapping_add(1);
                }
            } else {
                py = s as u8;
            }
        } else if vy < 0 {
            let m = (-(vy as i16)) as u8;
            if py < m {
                if vy_v {
                    failed = true;
                } else {
                    py = py.wrapping_sub(m).wrapping_sub(0x10);
                    phi = phi.wrapping_add(1);
                }
            } else {
                py -= m;
            }
        }
        if !failed {
            if vx > 0 {
                let s = px as u16 + vx as u16;
                if s > 0xFF {
                    if vy_v {
                        px = s as u8;
                        phi = phi.wrapping_add(1);
                    } else {
                        failed = true;
                    }
                } else {
                    px = s as u8;
                }
            } else if vx < 0 {
                let m = (-(vx as i16)) as u8;
                if px < m {
                    if vy_v {
                        px = px.wrapping_sub(m);
                        phi = phi.wrapping_add(1);
                    } else {
                        failed = true;
                    }
                } else {
                    px -= m;
                }
            }
        }
        if failed {
            mem.write_ram(base, 0); // KillSkreeProjectile
            continue;
        }
        mem.write_ram(base + 1, py);
        mem.write_ram(base + 2, px);
        mem.write_ram(base + 3, phi & 1);
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
        // Samus 被 Skree 弹击中（asm `UpdateSkreeProjectile ; 07:FBEC` 尾段 :12410-12431）：
        // `lda SamusInvincibleDelay / bne @endIf_A` ⇒ i 帧内跳过；
        // `AreObjectsTouching(Samus, PowerUpDraw)`（X 槽=Samus、Y 槽=$340，**Object 半径和**）；
        // `IsScrewAttackActive / bcc @endIf_A` ⇒ 螺旋转有效跳过；
        // 否则 `jsr KnockSamusBackIfCollided`（07:F311）→ `HealthChange←$50` → `SubtractHealth`。
        //
        // 【2026-11-23 修·f2249 根因】旧形是本径的**简化替身**：硬编码半径 `+4`（非 Object
        // 半径和）、并**直写** `SamusInvincibleDelay←$32` 绕过 `KnockSamusBackIfCollided` ⇒
        // `Samus.isHit` bit5 从未置、下一帧 `CheckHealthStatus` 的击退链（`speedY←$FD`/
        // `AccelY←$38`/`KnockbackDir`）整条不跑。实测后果：6725M any% f2249 受伤击退缺失
        // （mesen `isHit=0x28`→inv 49/speedY FD/向右上飞；rust 无 knockback、继续左行）。
        if mem.samus_invincible_delay() == 0 {
            // PowerUpDraw（$340）= Object 槽 4（`$0300 + 4*16`）；asm 调用是
            // `ldy #Samus-Objects / ldx #PowerUpDraw-Objects` ⇒ **Y 槽=Samus、X 槽=$340**。
            const POWERUP_DRAW_SLOT: u8 = 4;
            let (sy, sx, s_ry, s_rx, s_hi) = {
                let s = mem.samus();
                (s.y() as i16, s.x() as i16, s.radius_y() as i16, s.radius_x() as i16, s.hi() & 0x01)
            };
            let (py2, px2, p_ry, p_rx, p_hi) = {
                let o = mem.object(POWERUP_DRAW_SLOT);
                (
                    o.y() as i16,
                    o.x() as i16,
                    o.radius_y() as i16,
                    o.radius_x() as i16,
                    o.hi() & 0x01,
                )
            };
            // CheckCollisionOfXSlotAndYSlot（07:F1FA）三件：
            // ①**两槽 hi 都先 `eor PPUCTRL_ZP / and #$01`**（`GetXSlotPosition_Common`
            //   `:10275-10279`、`GetYSlotPosition_Common` `:10287-10291`）——这是「相对当前
            //   基名表」的页号；`$FF PPUCTRL_ZP` 在 rust 侧是活影子（`ToggleNameTable ; 07:E9B7`）。
            // ②hi 折算**分轴、按房间朝向**：`Temp03 = $02 & ScrollDir`；`bne @else_sameHiY`
            //   ⇒ 横滚房（dir bit1 置）Y 轴不比 hi；`beq @else_sameHiX` ⇒ 竖滚房 X 轴不比 hi。
            // ③折算是**按页大小的有符号差**（竖滚 y 页 240、横滚 x 页 256），取绝对值后与
            //   半径和比；|差| ≥ $100 由 `sec / lda Temp01_DiffHi / bne @RTS` 天然拒掉。
            //   旧形（第一版修）写成 `|Δlow| + $100` ⇒ 一跨页就永远 ≥256 ⇒ 跨页小距离命中全漏。
            let c = mem.ppuctrl_zp() & 0x01;
            let s_hi = { let s = mem.samus(); (s.hi() ^ c) & 0x01 };
            let p_hi = { let o = mem.object(POWERUP_DRAW_SLOT); (o.hi() ^ c) & 0x01 };
            let vertical = mem.camera().dir() & 0x02 == 0;
            let dyf: i32 = if vertical {
                (p_hi as i32 * 240 + py2 as i32) - (s_hi as i32 * 240 + sy as i32)
            } else {
                py2 as i32 - sy as i32
            };
            let dxf: i32 = if vertical {
                px2 as i32 - sx as i32
            } else {
                (p_hi as i32 * 256 + px2 as i32) - (s_hi as i32 * 256 + sx as i32)
            };
            let ry_sum = (s_ry + p_ry) as u32;
            let rx_sum = (s_rx + p_rx) as u32;
            let touching =
                dyf.unsigned_abs() < ry_sum && dxf.unsigned_abs() < rx_sum;
            // 螺旋转有效（IsScrewAttackActive carry 清）⇒ 跳过伤害。
            if touching && !crate::samus::physics::is_screw_attack_active(mem) {
                // KnockSamusBackIfCollided（07:F311）：SamusHurt010F←$E0；
                // SetSamusIsHitByEnemy（isHit |= DistHi<<3）；KnockbackIsBomb←(DistX≠0)。
                mem.samus().set_hurt(0xE0);
                // `Temp10_DistHi` 演化：初值 `#$02` → Y 负 `inc` → `asl` → X 负 `inc`
                // ⇒ bit5 恒置、bit4=Y 差为负、bit3=X 差为负（`SetSamusIsHitByEnemy ; 07:F338`
                // 再 `asl×3` 落 `Samus.isHit`）。
                let dist_hi: u8 =
                    0x04u8 | (if dyf < 0 { 0x02 } else { 0 }) | (if dxf < 0 { 0x01 } else { 0 });
                let cur = mem.samus().is_hit();
                mem.samus().set_is_hit(cur | dist_hi.wrapping_shl(3));
                // `lda Temp0F_DistX / beq`——低字节差为 0 ⇒ 同列 ⇒ 垂直击退（0）。
                let same_x = sx.wrapping_sub(px2) == 0;
                mem.samus().set_knockback_is_bomb(if same_x { 0x00 } else { 0x01 });
                // HealthChange←$50（5 伤害）→ SubtractHealth。
                mem.set_health_change(0x50);
                subtract_health(mem);
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
/// `GetEnemyIsHitFlags ; 07:F332`（`GetEnemyDistHi ; 07:F340` = `Temp10_DistHi eor #$03`
/// → `Amul8 ; 07:C2xx`）：`(distHi ^ 3) << 3`。bit5（被武器命中）只在 `distHi==7` 时出现。
pub(crate) fn enemy_is_hit_flags(dist_hi: u8) -> u8 {
    (dist_hi ^ 0x03) << 3
}

/// ROM `CheckCollisionOfXSlotAndYSlot ; 07:F1FA` 的单轴距离（16 位语义）。
///
/// 调用约定（由调用方按 `ScrollDir & 2` 决定哪个轴是主轴）：
/// * `use_hi`：`true` ⇒ 该轴比较 16 位（hi 不等时 `sbc` 的借位进 `dhi`）；`false` ⇒ 只比低字节；
/// * `compensate_240`：纵向房 Y 轴的 240 行补偿（`lo -= 0x10`，借位则 `dhi -= 1`）。
///
/// 返回 `Some(dist)` 表示 `dhi==0`（ROM：`lda Temp01_DiffHi / bne @RTS` 才继续）；
/// `None` 表示两槽跨页（≥256 px），一律判「不碰」。
///
/// 证据：`docs/spec/f1301-fix-plan.md` §4（#130 规格）、`prg7_engine.asm` 的 F1FA/F1B6。
pub(crate) fn rom_axis_delta(a: u8, ha: u8, b: u8, hb: u8, use_hi: bool, compensate_240: bool) -> Option<u8> {
    let (mut lo, borrow0) = a.overflowing_sub(b);
    let mut dhi: i32 = if use_hi && ha != hb {
        ha as i32 - hb as i32 - borrow0 as i32
    } else {
        -(borrow0 as i32)
    };
    if dhi < 0 {
        let (nlo, borrow1) = 0u8.overflowing_sub(lo);
        lo = nlo;
        dhi = -dhi - borrow1 as i32;
    }
    if compensate_240 && use_hi && ha != hb {
        let (nlo, borrow2) = lo.overflowing_sub(0x10);
        lo = nlo;
        if borrow2 {
            dhi -= 1;
        }
    }
    if dhi == 0 { Some(lo) } else { None }
}

pub(crate) fn uw_collision_detection(mem: &mut crate::frame::Manager) {
    // 【goal-轮20 碰撞环头件】CollisionDetection（F034-F038 逐字）：`lda #$FF/sta
    // SamusKnockbackIsBomb($73)/sta SamusHurt010F($010F)`——**每环重置 kb73=$FF**（无此
    // 件则 CheckHealthStatus 的击退向位径（bomb≠0 门）永不发（金腿 kb77=$FF 实证）。
    mem.write_ram(crate::ram_map::SAMUS_KNOCKBACK_IS_BOMB, 0xFF);
    mem.write_ram(0x010F, 0xFF);
    for s in 0..6u16 {
        let x = s * 16; // 字节偏移约定（Ens/EnsExtra 步距 16——asm 6 槽环 :10021 Xminus16）
        let st = mem.read_wram(0x6AF4 + x);
        // asm :10140-10147 只跳过 0（空）与 3（Explode）；**Pickup(5) 不跳出环**——
        // 它只跳过「弹×敌」子环（:10150-10155 `cmp #enemyStatus_Pickup / beq
        // @endIf_EnemyProjectile`），后面的「敌×Samus」子环照跑（:10176-10182），
        // 拾取物的触碰位（isHit bit2）就由那里写入。旧形连 5 一起 continue ⇒ 道具永不
        // 置触碰位 ⇒ UpdateEnemy_Pickup 的 `isHit & $24` 永不成立 ⇒ 永不拾取
        // （any% f1464/f1465 能量包丢失的根因）。
        if st == 0 || st == 3 {
            continue; // 空/爆中（:10140-10147）
        }
        for py in [0x03D0u16, 0x03E0, 0x03F0] {
            if st == 5 {
                break; // Pickup：跳过弹×敌子环（asm :10150 `beq @endIf_EnemyProjectile`）
            }
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
            // 【#137 判盒 16 位重建】ROM CheckCollisionOfXSlotAndYSlot(07:F1FA)：
            // 只有**主轴**比 hi（`ScrollDir & 2`：0=纵向房 Y 主轴、1=横向房 X 主轴），
            // 另一轴仍只比低字节；跨页（dhi!=0）一律不碰。
            // 旧形两轴都只比低字节 ⇒ f1301 把「敌在 nametable 0、弹在 nametable 1」的
            // 262 px 当成 6 px，误入 F2CA 写 $0454/$045E（首分歧，见 docs/findings/tas-round6.md）。
            // typed 句柄取位（B0 允许面）：相机朝向 / PPUCTRL 影子 / 敌与弹的 nametable 页号。
            let vertical_room = mem.camera().dir() & 0x02 == 0;
            let base = mem.ppuctrl_zp() & 0x01;
            let e_hi = mem.enemy((x / 16) as u8).hi() ^ base;
            let p_hi = mem.object(((py - 0x0300) / 16) as u8).hi() ^ base;
            let (dy, dx) = if vertical_room {
                (
                    rom_axis_delta(ey as u8, e_hi, py_y as u8, p_hi, true, true),
                    rom_axis_delta(ex as u8, e_hi, py_x as u8, p_hi, false, false),
                )
            } else {
                (
                    rom_axis_delta(ey as u8, e_hi, py_y as u8, p_hi, false, false),
                    rom_axis_delta(ex as u8, e_hi, py_x as u8, p_hi, true, false),
                )
            };
            let (Some(dy), Some(dx)) = (dy, dx) else {
                continue; // 跨页（dhi!=0）⇒ 不碰
            };
            if dy as i16 >= ery + pry || dx as i16 >= erx + prx {
                continue; // 未触碰（carry 置⇒F2CA bcs Lx293）
            }
            // ==== F2CA 命中面（:10391-10407）====
            mem.write_ram(0x0400 + x + 0x0E, pstat); // weaponAction←弹 status
            mem.write_ram(py + 0x0A, mem.read_ram(py + 0x0A) | 0x20); // 弹 isHit（SetSamusProjectileIsHit）
            // [r9 修] `GetEnemyIsHitFlags ; 07:F332` = `GetEnemyDistHi ; 07:F340`
            // （`Temp10_DistHi eor #$03`）+ `Amul8` ⇒ **只有 `(distHi^3)<<3` 一项**，
            // bit5(命中) 是 distHi==7 的产物，不是恒置位。旧形「恒 0x20 + 位置比较加
            // bit4/bit3」是自造规则：emu f1293 为 `0x20`（distHi=7），旧形给 `0x38`。
            // distHi 构建（F1FA，槽序 Y=敌/X=弹）：`0x04 | (Δy<0)<<1 | (Δx<0)`。
            let dist_hi: u8 = 0x04
                | (((ey - py_y) < 0) as u8) << 1
                | ((ex - py_x) < 0) as u8;
            let flags: u8 = enemy_is_hit_flags(dist_hi);
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
            let samus_status = mem.samus().status_raw();
            if samus_status == 7 || samus_status == 8 || samus_status == 0xFF {
                continue;
            }
            let sy = mem.samus().y() as i16;
            let sx = mem.samus().x() as i16;
            let sry = mem.samus().radius_y() as i16;
            let srx = mem.samus().radius_x() as i16;
            let ey = mem.read_ram(0x0400 + x + 0) as i16;
            let ex = mem.read_ram(0x0400 + x + 1) as i16;
            let ery = mem.read_wram(0x6AF4 + x + 1) as i16;
            let erx = mem.read_wram(0x6AF4 + x + 2) as i16;
            let dy = (sy - ey).unsigned_abs() as i16;
            let dx = (sx - ex).unsigned_abs() as i16;
            // 【goal-轮79 asm 合形修】F1FA hi（名表）距调——旧形无 hi 调=偏离（金方 hi 差
            // ⇒距+256⇒不撞）——hi 距调：hi 差⇒距+0x100（asm @positionHi_notEqual 语义）。
            let s_hi = mem.samus().hi() & 1; // Samus.hi（Objects.0.hi——F193 勘误）
            let (sry_y_raw, srx_x_raw) = (sy as i32, sx as i32);
            let e_hi = mem.read_wram(0x6AF4 + x + 7) & 1; // EnsExtra.0.hi（F152 ✓）
            let dy_adj = if s_hi != e_hi { dy + 0x100 } else { dy };
            let dx_adj = if s_hi != e_hi { dx + 0x100 } else { dx };
            // Temp10_DistHi（F1FA 构建，槽序「X=Samus、Y=敌」⇒ dy/dx 取「敌 减 Sam」——
            // emu f1112 `isHit=0x05`/`Samus.isHit=0x30` 反推）：0x04 | (dy<0)<<1 | (dx<0)。
            let dist_hi: u8 = {
                let vertical = mem.camera().dir() & 0x02 == 0;
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
    // ==== 弹体×萨姆斯（@exitLoop_Enemy 07:F0ED-F112 + ReactToCollision 07:F2ED）====
    // 敌槽环走完后接弹体池（EnProjectiles=$60 起 6 槽）：跳过 status==0/Pickup(5)、
    // i 帧（$70≠0）、萨姆斯死亡（07:F0DA IsSamusDead 门，全环一次）。
    // 命中 ⇒ SetEnIsHit（isHit|=DistHi）+ IsScrewAttackActive 分岔：螺旋转有效⇒仅此
    // （弹体下帧 Resting 径拾取化）；无效⇒SamusHurt($010F)←$80 + Samus.isHit|=
    // (DistHi^3)<<3（GetEnemyIsHitFlags F332+F340+Amul8）+ HealthChange←AreaEnemyDamage
    // （区头 $95CE word，随件 LF306）。
    if mem.samus_invincible_delay() == 0 {
        // GetObjectXSlotPosition(Samus)（07:F172）：X 槽=Objects.0（=Samus typed 视图）。
        let s = mem.samus();
        let samus_status = s.status_raw();
        let (sy, sx) = (s.y() as i16, s.x() as i16);
        let (sry, srx) = (s.radius_y() as i16, s.radius_x() as i16);
        let s_hi = s.hi() & 1;
        let samus_is_hit = s.is_hit();
        drop(s);
        if !(samus_status == 7 || samus_status == 8 || samus_status == 0xFF) {
            for i in 0..crate::enemy::en_projectile::SLOT_COUNT {
                let slot = crate::enemy::en_projectile::base_slot(i);
                let st = mem.enemy(slot).status_raw();
                if st == 0 || st == 5 {
                    continue; // :10198-10200
                }
                let p = mem.enemy(slot);
                let (py, px_, p_hi) = (p.y(), p.pos_x(), p.hi() & 1);
                let (pry, prx) = (p.radius_y() as i16, p.radius_x() as i16);
                // CheckCollisionOfXSlotAndYSlot（07:F1FA）：hi 差 ⇒ 距 +$100（F21B/F25B
                // @positionHi_notEqual；同前述敌×萨 hi 调勘误同构）。
                let dy_raw = (sy - py as i16).unsigned_abs() as i16;
                let dx_raw = (sx - px_ as i16).unsigned_abs() as i16;
                let dy = if s_hi != p_hi { dy_raw + 0x100 } else { dy_raw };
                let dx = if s_hi != p_hi { dx_raw + 0x100 } else { dx_raw };
                if dy >= sry + pry || dx >= srx + prx {
                    continue; // carry 置=无重叠 ⇒ F2ED bcs @RTS
                }
                // Temp10_DistHi（F1FA 构建）：0x04 | dy 符号<<1 | dx 符号（「弹在萨姆斯
                // 下/右」置位——槽序 X=Samus、Y=弹）。
                let vertical = mem.camera().dir() & 0x02 == 0;
                let dyf: i32 = if vertical { (p_hi as i32 * 240 + py as i32) - (s_hi as i32 * 240 + sy as i32) } else { py as i32 - sy as i32 };
                let dxf: i32 = if vertical { px_ as i32 - sx as i32 } else { (p_hi as i32 * 256 + px_ as i32) - (s_hi as i32 * 256 + sx as i32) };
                let dist_hi: u8 =
                    0x04 | (if dyf < 0 { 0x02 } else { 0 }) | (if dxf < 0 { 0x01 } else { 0 });
                // SetEnIsHit（07:F2DF）：弹 isHit |= DistHi。
                let mut p = mem.enemy(slot);
                p.set_is_hit(p.is_hit() | dist_hi);
                // 螺旋转有效（carry 清）⇒ jmp SetHealthChangeToAreaEnemyDamage@RTS——
                // 不伤萨姆斯（isHit 已置，弹体自身下帧拾取化）。
                if crate::samus::physics::is_screw_attack_active(mem) {
                    continue;
                }
                mem.write_ram(crate::ram_map::SAMUS_HURT_010F, 0x80); // SamusHurt010F（07:F313）
                // GetEnemyIsHitFlags（F332：DistHi eor 3）+ Amul8 → Samus.isHit。
                let flags = samus_is_hit | (dist_hi ^ 0x03).wrapping_shl(3);
                mem.samus().set_is_hit(flags);
                // SetHealthChangeToAreaEnemyDamage（LF306）：HealthChange←AreaEnemyDamage
                // （区头 $95CE word——ROM 面直读，RAM $6C/$6D 是门挡位不相干）。
                let area = crate::frame::cur_area(mem);
                let dmg = game_data::assets::area_header(area).enemy_damage;
                mem.set_health_change(dmg);
            }
        }
    }
    // ==== SubtractHealth（CE92 尾——BCD 扣血（:3203-3257））====
    subtract_health(mem);
}

/// SubtractHealth（**asm 内容锚** `SubtractHealth: ; 07:CE92`）：HealthChange≠0∧活⇒
/// （Varia⇒半伤（lsr+carry 补 $4F））→Base10Subtract（BCD）：Health($0106)/Health+1($0107)
/// 双字节扣→ClearHealthChange。
pub(crate) fn subtract_health(mem: &mut crate::frame::Manager) {
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
    let samus_status = mem.samus().status_raw();
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
    let hp = mem.samus().health();
    let (h0, h1) = (hp as u8, (hp >> 8) as u8);
    let (r0, borrow) = bcd_sub(h0, a);
    let (r1, borrow2) = bcd_sub(h1, if borrow { b.wrapping_add(1) } else { b });
    mem.samus().set_health(u16::from_le_bytes([r0, r1]));
    // 【bcd 掩码审计卡 2026-11-10】Health+1 全字节落写（asm :3233 `sta Health+1`
    // 无掩码）：高位 nibble 参与借位/回绕运算（$02+0x10 借位补、负结果 +$A0 调整
    // 合法改写 bit6-5），掩码剥离=分歧（asm $79→旧 rust $19）。证据见
    // docs/findings/bcd-mask-audit-subtracthealth.md。
    // 【asm `SubtractHealth ; 07:CECC` 死亡分支（`prg7_engine.asm:3259-3275`）】旧形缺整支：
    //   `lda Health / and #$F0 / ora Health+1 / beq @dead`（BCD 十位域与高位**同时**为 0
    //   ⇒ 判死；`and #$F0` 让 Health<10 也算死 = 原版 quirk）
    //   `@dead: Health←0 / Health+1←0 / status←sa_Dead / jsr SFX_SamusDie / jmp SetSamusExplode`
    //   非死径才 `bcs AddHealth@GotoClearHealthChange`（清 HealthChange）；**死径不清**。
    // 实测（100% 录像 f3889）：两侧 Health 都已为 0，emu 转 Dead(7) 并起爆炸，rust 仍在 Run
    // ⇒ 此后 Samus 精灵/物理全岔（f3890 起 34px 起跳，级联到 f4000 的 461px）。
    // asm 的判死**两条并集**：`beq @endIf_D`（Health&$F0|Health+1 == 0）与
    // `bcs AddHealth@GotoClearHealthChange`（**进位=1 才不是死**）——后者是
    // `Base10Subtract ; 07:C3FB` 的返回进位：高半字节 `sbc` 借位时 `clc` 清零 ⇒
    // **扣穿（借位）也算死，即使回绕结果非 0**。
    // 实测（100% 录像 f3889）：Health=0x0060、HealthChange=0x0080 ⇒ BCD 回绕成 0x9980，
    // 但借位 ⇒ emu 判死、Health 清零、HealthChange 保留 0x80；旧 rust 只看「结果非 0」⇒ 不死。
    let zero_health = (r0 & 0xF0) == 0 && r1 == 0;
    let dead = zero_health || borrow2;
    if dead {
        mem.samus().set_health(0);
        mem.samus()
            .set_status(crate::samus::state::SamusStatus::Dead);
        // SFX_SamusDie（`:CBE2` = `lda #sfxTri_SamusDie($80)` → SetSFXTriInitFlag）
        let f = mem.tri_init_flags() | 0x80;
        mem.set_tri_init_flags(f);
        crate::samus::handler::set_samus_explode(mem);
        return;
    }
    mem.write_ram(0x006E, 0);
    mem.write_ram(0x006F, 0);
}

/// BCD 半字节减（Base10Subtract 简化形）：返回 (结果, 借位)。
/// `Base10Add ; 07:C3DA`：BCD 逐位相加（低半字节→高半字节），返回 `(结果, 进位)`。
pub(crate) fn bcd_add(v: u8, add: u8) -> (u8, bool) {
    let mut carry = false;
    let mut out = 0u8;
    for shift in [0u32, 4] {
        let mut d = ((v >> shift) & 0x0F) as i16 + ((add >> shift) & 0x0F) as i16 + carry as i16;
        if d > 9 {
            d -= 10;
            carry = true;
        } else {
            carry = false;
        }
        out |= (d as u8) << shift;
    }
    (out, carry)
}

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


#[cfg(test)]
mod tests_rom_axis_delta {
    use super::rom_axis_delta;

    /// #130 规格 §4 的四组向量（同 hi / 跨 hi 都覆盖）。
    #[test]
    fn four_vectors_from_spec() {
        // f1301 场景：敌 120(hi0) vs 弹 126(hi1) ⇒ 262 px，跨页必不碰。
        assert_eq!(rom_axis_delta(120, 0, 126, 1, true, false), None);
        // 同 hi、两侧：借位进 dhi 后取补成 10。
        assert_eq!(rom_axis_delta(120, 0, 130, 0, true, false), Some(10));
        assert_eq!(rom_axis_delta(130, 0, 120, 0, true, false), Some(10));
        assert_eq!(rom_axis_delta(0, 0, 0, 0, true, false), Some(0));
    }

    /// 纵向房 Y 轴的 240 行补偿：跨页时按 240 屏高折算（`lo -= 16`）。
    #[test]
    fn vertical_240_row_compensation() {
        // 敌 y=5(hi0) vs 弹 y=245(hi1)：真实 240-5+... ⇒ 同页折回不碰？按 ROM：dhi!=0 ⇒ None。
        assert_eq!(rom_axis_delta(5, 0, 245, 1, true, true), None);
        // 同 hi 时补偿不生效（只有 hi 不等才做）。
        assert_eq!(rom_axis_delta(5, 0, 15, 0, true, true), Some(10));
    }

    /// 改前判据（只比低字节，`unsigned_abs`）在这组向量上会给出「碰」，即负例在改前代码上红。
    /// 保留为对照实现，防止有人把判盒又退化成低字节比较。
    fn legacy_low_byte_delta(a: u8, b: u8) -> u8 {
        a.abs_diff(b)
    }

    #[test]
    fn legacy_low_byte_judge_box_is_the_pre_fix_bug() {
        // 改前代码对 f1301 的判定值：|120-126| = 6（半径和 10 ⇒ 误判为碰）。
        assert_eq!(legacy_low_byte_delta(120, 126), 6);
        assert!(legacy_low_byte_delta(120, 126) < 10);
        // 改后同一对（跨页）返回 None ⇒ 不碰。两者结论相反，正是 #137 修的差。
        assert_eq!(rom_axis_delta(120, 0, 126, 1, true, false), None);
    }
}

#[cfg(test)]
mod tests_bullet_explode_order {
    use crate::frame::Manager;

    /// 弹出膛槽（$0300+PageIndex）：status/speeds/isHit/onScreen/hi/y/x/timer。
    fn bullet(m: &mut Manager, y: u8, x: u8, sx: u8) {
        m.write_ram(0x004B, 0); // PageIndex → base=$0300
        m.write_ram(0x0300, 1); // status
        m.write_ram(0x0308, 0); // speedY
        m.write_ram(0x0309, sx); // speedX
        m.write_ram(0x030A, 1); // isHit（撞敌）
        m.write_ram(0x030B, 1); // onScreen
        m.write_ram(0x030C, 0); // hi
        m.write_ram(0x030D, y);
        m.write_ram(0x030E, x);
        m.write_ram(0x030F, 0x20); // 时寿
    }

    fn room_byte(m: &mut Manager, y: u8, x: u8, val: u8) {
        let idx = (((y & 0xF8) as u16) << 2) | ((x >> 3) as u16);
        m.write_wram(0x6000 | (idx & 0x3FF), val);
    }

    /// asm `$D4EB`：ExplodeIfHitSprite **不设 carry** ⇒ CollisionWithBG 仍执行；
    /// 前方空气（≥$A0）⇒ 照样前移一步。旧形把「撞敌」当「已碰撞」⇒ 停在撞点。
    /// 实证：TASVideos 100% 录像 f2135（emu status=4 且 x 210→214）。
    #[test]
    fn hit_sprite_then_move_when_air_ahead() {
        let mut m = Manager::default();
        bullet(&mut m, 0x90, 0xD0, 4);
        room_byte(&mut m, 0x90, 0xD0, 0xA0); // 空气
        super::update_bullet(&mut m);
        assert_eq!(m.read_ram(0x0300), 4, "撞敌⇒status←4（爆）");
        assert_eq!(m.read_ram(0x030A), 0, "isHit 清");
        assert_eq!(m.read_ram(0x030E), 0xD4, "空气⇒仍前移一步（asm carry 由瓦判给）");
    }

    /// 前方固实（<$80）⇒ 不移，且走 CheckBulletStat 的 collided 径（status≠0 ⇒ 爆）。
    #[test]
    fn hit_sprite_then_stop_when_solid_ahead() {
        let mut m = Manager::default();
        bullet(&mut m, 0x90, 0xD0, 4);
        room_byte(&mut m, 0x90, 0xD0, 0x01); // 固实
        super::update_bullet(&mut m);
        assert_eq!(m.read_ram(0x0300), 4, "固实⇒爆");
        assert_eq!(m.read_ram(0x030E), 0xD0, "固实⇒不移");
    }
}

#[cfg(test)]
mod tests_bullet_hi_page {
    //! 【复核 P1 锚（2026-11-20）】`GetObjRoomRAMPtr ; 07:D79F` → `MakeRoomRAMPtr ; 07:E96A`
    //! 把对象 `hi`（名表号 0/1）按 `asl / asl / and #$04` 换成页选位；`UpdateBullet_Move`
    //! 的 `LoadObjectPositionFromTemp` 还会把 `Temp0B_PositionHi & $01` 落写回
    //! `Objects.0.hi`（越界且滚动方向允许时 `inc Temp0B_PositionHi`）。
    use super::*;

    /// hi=0 读 A 页、hi=1 读 B 页：两页同一 (y,x) 放「门瓦 vs 空气」，看哪一页被读到。
    #[test]
    fn bg_collision_reads_page_selected_by_object_hi() {
        for (hi, a_page_tile, b_page_tile) in [(0u8, 0x4Eu8, 0xA0u8), (1u8, 0xA0u8, 0x4Eu8)] {
            let mut m = crate::frame::Manager::default();
            m.set_page_index(0x30); // 弹槽 = PowerUpDraw 面（$0330）
            {
                let mut o = m.object(3);
                o.set_status(1); // 存活
                o.set_is_hit(0); // 不触发撞敌爆
                o.set_on_screen(1); // 未出界
                o.set_hi(hi);
                o.set_y(0x50);
                o.set_x(0x10);
                o.set_speed_y(0);
                o.set_speed_x(0);
            }
            let a = crate::world::room_ram::Room::addr(0x50, 0x10, 0x00);
            let b = crate::world::room_ram::Room::addr(0x50, 0x10, 0x04);
            assert_ne!(a, b);
            m.write_wram(a, a_page_tile);
            m.write_wram(b, b_page_tile);
            update_bullet(&mut m);
            // 读到 $4E（门瓦）⇒ 爆：status←4；读到空气则保持 1
            assert_eq!(m.object(3).status(), 4,
                       "hi={} 应读到页内门瓦并爆（A={a:#06X} B={b:#06X}）", hi);
        }
    }

    /// 横滚越界（x 和进位）⇒ `Temp0B_PositionHi ++1` 且 hi 落写 bit0。
    #[test]
    fn x_wrap_increments_and_stores_object_hi() {
        let mut m = crate::frame::Manager::default();
        m.set_page_index(0x30);
        {
            let mut o = m.object(3);
            o.set_status(1);
            o.set_is_hit(0);
            o.set_on_screen(1);
            o.set_hi(0x00);
            o.set_y(0x50);
            o.set_x(0xF8);
            o.set_speed_y(0);
            o.set_speed_x(0x10); // +16 ⇒ 0xF8+0x10 进位
        }
        m.write_ram(crate::ram_map::SCROLL_DIR, 0x03); // 横滚
        bullet_move(&mut m);
        assert_eq!(m.object(3).hi(), 0x01, "hi 应 ++1 并落写");
        assert_eq!(m.object(3).x(), 0x08, "x 应回绕到 0x08（0xF8+0x10 取低字节）");
        assert_eq!(m.object(3).status(), 1, "横滚越界不删弹");
    }

    /// 竖滚时 x 越界 ⇒ 失败 ⇒ 弹删除（asm `bcc Lx078` ⇒ status←0）。
    #[test]
    fn x_wrap_fails_and_deletes_bullet_when_vertical_scroll() {
        let mut m = crate::frame::Manager::default();
        m.set_page_index(0x30);
        {
            let mut o = m.object(3);
            o.set_status(1);
            o.set_hi(0x00);
            o.set_y(0x50);
            o.set_x(0xF8);
            o.set_speed_y(0);
            o.set_speed_x(0x10);
        }
        m.write_ram(crate::ram_map::SCROLL_DIR, 0x01); // 竖滚
        assert!(!bullet_move(&mut m), "竖滚下 x 越界 ⇒ 返回失败");
        assert_eq!(m.object(3).status(), 0, "竖滚下 x 越界 ⇒ 弹删");
        assert_eq!((m.object(3).y(), m.object(3).x(), m.object(3).hi()), (0x50, 0xF8, 0x00),
                   "失败帧不得写回坐标/hi（asm 在 @exit_failure 前就 jmp，不落写）");
    }
}

#[cfg(test)]
mod tests_enemy_is_hit_flags {
    use super::enemy_is_hit_flags;

    /// `GetEnemyIsHitFlags ; 07:F332`：`(distHi^3)<<3`（`Amul8`）。
    /// 真值校验：emu 6725M f1293 槽5 `isHit=$20` ⇒ distHi=7（敌在弹的左上方）。
    #[test]
    fn flags_are_dist_hi_xor_three_times_eight() {
        assert_eq!(enemy_is_hit_flags(7), 0x20, "distHi=7（上方+左侧）⇒ 只有 bit5");
        assert_eq!(enemy_is_hit_flags(4), 0x38, "distHi=4（下方+右侧）⇒ $38");
        assert_eq!(enemy_is_hit_flags(5), 0x30, "distHi=5（下方+左侧）⇒ $30");
        assert_eq!(enemy_is_hit_flags(6), 0x28, "distHi=6（上方+右侧）⇒ $28");
    }
}

#[cfg(test)]
mod tests_pickup_touch_flag {
    use super::*;
    use crate::enemy::state::EnemyStatus;
    use crate::frame::Manager;

    /// 【any% f1464 定谳】asm `CollisionDetection ; 07:F034` 的敌人环头只跳过
    /// `status==0`（`:10144 beq`）与 `status==Explode`（`:10147 beq`）；
    /// **Pickup(5) 不跳出环**，它只跳过「弹×敌」子环（`:10150-10155`
    /// `cmp #enemyStatus_Pickup / beq @endIf_EnemyProjectile`），「敌×Samus」子环
    /// （`:10176-10182`）照跑并调 `SetEnemyTouchingSamusFlags ; 07:F2E8` ⇒ 写
    /// `isHit |= (distHi^3)`（bit2=触碰）。旧形环头把 5 一起 continue ⇒ 道具永不
    /// 置触碰位 ⇒ `UpdateEnemy_Pickup ; 07:F483` 的 `isHit & #$24` 永不成立 ⇒
    /// 道具永不拾取（f1465 能量包丢失的根因）。
    #[test]
    fn pickup_gets_touch_flag_from_samus_branch() {
        let mut m = Manager::default();
        m.write_ram(crate::ram_map::SCROLL_DIR, 0x03); // 横滚（dir&2!=0）
        m.write_ram(crate::ram_map::SAMUS_INVINCIBLE_DELAY, 0);
        m.samus().set_status(crate::samus::state::SamusStatus::Run);
        m.samus().set_y(185);
        m.samus().set_x(76);
        m.samus().set_radius_y(15);
        m.samus().set_radius_x(4);
        // 拾取物：status=Pickup，位置在 Samus 上方 17px（radii 和 19 ⇒ 触碰）
        m.enemy(5).set_status(EnemyStatus::Pickup);
        m.enemy(5).set_y(168);
        m.enemy(5).set_pos_x(76);
        m.enemy(5).set_radius_y(4);
        m.enemy(5).set_radius_x(4);
        uw_collision_detection(&mut m);
        let hit = m.enemy(5).is_hit();
        assert_ne!(hit & 0x04, 0, "拾取物必须由敌×Samus 子环写上触碰位（bit2），实测 {:02X}", hit);
        assert_eq!(hit & 0x80, 0, "弹×敌子环必须跳过（bit7 是武器命中面），实测 {:02X}", hit);
    }
}

#[cfg(test)]
mod tests_skree_projectile_damage {
    use super::*;
    use crate::frame::Manager;

    /// 造一支 Skree 弹并**按移动后位置**摆几何。
    ///
    /// 槽 3（字节偏移 12 ⇒ asm `txa / lsr / tay` 得 speed 表索引 6 = `(0,+5)`：**x 加 5、
    /// y 不变**）——`uw_skree_projectiles` 先 `ApplySpeedToPosition` 再把新位置写进
    /// `PowerUpDraw`，碰撞判的是**移动后**的位置，故这里按「移动后」摆并反推写入值。
    ///
    /// 注：`PowerUpDraw`（`$340`）的 radiusY/radiusX **在 asm 里从未被写过**（写点只有
    /// y/x/hi/animFrame ⇒ `:12387-12403`），RAM 清场后恒 0 ⇒ 运行态半径和 = Samus 半径。
    /// 下面按运行态置 0；跨页用例再显式放大 Samus 半径。
    fn setup(dir: u8, sy: u8, sx: u8, post_y: u8, post_x: u8) -> Manager {
        let mut m = Manager::default();
        m.set_scroll_dir(dir);
        m.samus().set_status(crate::samus::state::SamusStatus::Run);
        m.samus().set_y(sy);
        m.samus().set_x(sx);
        m.samus().set_radius_y(15);
        m.samus().set_radius_x(4);
        m.set_samus_invincible_delay(0);
        m.object(4).set_radius_y(0);
        m.object(4).set_radius_x(0);
        m.write_ram(0x00AC, 0x10); // 槽 3 dieDelay（$00A0 + 12）
        m.write_ram(0x00AD, post_y); // y（x 向速度，y 不变）
        m.write_ram(0x00AE, post_x.wrapping_sub(5)); // x（移动后 +5）
        m
    }

    /// Skree 弹×Samus（asm `UpdateSkreeProjectile ; 07:FBEC` 尾段 :12405-12433）：
    /// `AreObjectsTouching` → `IsScrewAttackActive` → `KnockSamusBackIfCollided ; 07:F311`
    /// → `HealthChange←$50` + `SubtractHealth`。
    ///
    /// 【2026-11-23 修·6725M any% f2249 根因】旧形是简化替身：硬编码半径 `+4`、并**直写**
    /// `SamusInvincibleDelay←$32` 绕过 `KnockSamusBackIfCollided` ⇒ `Samus.isHit` bit5 从不置
    /// ⇒ 下一帧 `CheckHealthStatus ; 07:CDFA` 的击退链（`speedY←$FD`/`AccelY←$38`/
    /// `KnockbackDir`）整条不跑。
    #[test]
    fn skree_hit_sets_is_hit_bit5_and_does_not_touch_invincible_delay() {
        let mut m = setup(0x03, 185, 76, 185, 76);
        uw_skree_projectiles(&mut m);
        let hit = m.samus().is_hit();
        assert_ne!(hit & 0x20, 0, "命中必须经 SetSamusIsHitByEnemy 置 bit5，实测 {:02X}", hit);
        assert_eq!(
            m.samus_invincible_delay(),
            0,
            "i 帧必须留给下一帧 CheckHealthStatus 置（旧形直写 $32 = f2249 根因）"
        );
        assert_eq!(m.samus().hurt(), 0xE0, "采样 hurt 位 07:F313");
        assert_eq!(m.read_ram(0x006E), 0x50, "HealthChange←$50");
    }

    /// `KnockSamusBackIfCollided`：`Temp0F_DistX==0 ⇒ $73=0`（同列=垂直击退），`≠0 ⇒ 1`。
    #[test]
    fn knockback_is_bomb_is_zero_only_when_both_on_same_column() {
        let mut m = setup(0x03, 185, 76, 185, 76);
        uw_skree_projectiles(&mut m);
        assert_ne!(m.samus().is_hit() & 0x20, 0, "同列应命中");
        assert_eq!(m.samus().knockback_is_bomb(), 0, "同列 ⇒ 0（垂直击退）");

        let mut m2 = setup(0x03, 185, 76, 185, 78); // 差 2px，仍在 radiusX(4) 内
        uw_skree_projectiles(&mut m2);
        assert_ne!(m2.samus().is_hit() & 0x20, 0, "差 2px 仍应命中");
        assert_eq!(m2.samus().knockback_is_bomb(), 1, "不同列 ⇒ 1（斜向）");
    }

    /// 跨页小距离必须命中：`CheckCollisionOfXSlotAndYSlot ; 07:F1FA` 折的是**按页大小的
    /// 有符号差**（竖滚 y 页 240），不是「|Δlow| + $100」。旧形（本修第一版）把跨页一刀切判
    /// 不碰，是相对原实现的回归。
    #[test]
    fn cross_name_table_small_distance_still_hits() {
        // 竖滚房：Samus y=235/hi=0、弹移动后 y=5/hi=1 ⇒ 差 = 5+240-235 = 10 < radiusY 15。
        let mut m = setup(0x01, 235, 76, 5, 76);
        m.samus().set_hi(0);
        m.write_ram(0x00AF, 0x01); // 槽 3 hi ⇒ PowerUpDraw.hi
        uw_skree_projectiles(&mut m);
        assert_ne!(
            m.samus().is_hit() & 0x20,
            0,
            "竖滚房跨页 10px 必须命中（fold 后取绝对值，不是 +$100）"
        );
        // 横滚房：Samus x=250/hi=0、弹移动后 x=2/hi=1 ⇒ 差 = 2+256-250 = 8；radiusX 放大到 10。
        // 用槽 0（`SPEED[0]=(0,-5)`：x 减 5、不溢出 ⇒ 不会被 `ApplySpeedToPosition` 杀掉）。
        let mut m2 = setup(0x03, 100, 250, 100, 7);
        m2.write_ram(0x00AC, 0x00); // 关掉槽 3
        m2.write_ram(0x00A0, 0x10); // 槽 0 dieDelay
        m2.write_ram(0x00A1, 100); // y
        m2.write_ram(0x00A2, 7); // x（移动后 = 2）
        m2.write_ram(0x00A3, 0x01); // hi
        m2.samus().set_radius_x(10);
        m2.samus().set_hi(0);
        uw_skree_projectiles(&mut m2);
        assert_ne!(m2.samus().is_hit() & 0x20, 0, "横滚房跨页 8px 必须命中");
    }

    /// 半径和是「`bcs` 即不碰」⇒ 明显超出时必须不命中。
    ///
    /// 注意：`PowerUpDraw` 的 radiusY/X（`$0341/$0342`）在绘制该帧时会被
    /// `crates/game-core/src/render/oam.rs` 从帧数据 blob 字节 1/2 写入（实测 Skree 帧给
    /// radiusY=4）⇒ **半径和不是「Samus 半径 + 0」**，边界值取决于帧数据，不宜钉死；
    /// 这里只钉「远离时不碰」。
    #[test]
    fn clearly_outside_radius_does_not_hit() {
        let mut m = setup(0x03, 185, 76, 215, 76); // dy=30，远超半径和
        uw_skree_projectiles(&mut m);
        assert_eq!(m.samus().is_hit() & 0x20, 0, "dy=30 ⇒ 不碰");
    }

    /// 螺旋转有效 ⇒ 跳过伤害（asm `jsr IsScrewAttackActive / bcc @endIf_A`）。
    #[test]
    fn screw_attack_active_skips_damage() {
        let mut m = setup(0x03, 185, 76, 185, 76);
        m.write_ram(crate::ram_map::SAMUS_GEAR, 0x08); // gr_SCREWATTACK
        m.samus().set_anim_reset_index(0x0E); // Salto ⇒ 判定 active
        uw_skree_projectiles(&mut m);
        assert_eq!(m.samus().is_hit() & 0x20, 0, "螺旋转有效时必须跳过伤害");
    }

    /// i 帧内 ⇒ 跳过（asm `lda SamusInvincibleDelay / bne @endIf_A`）。
    #[test]
    fn invincible_delay_blocks_damage() {
        let mut m = setup(0x03, 185, 76, 185, 76);
        m.set_samus_invincible_delay(0x20);
        uw_skree_projectiles(&mut m);
        assert_eq!(m.samus().is_hit() & 0x20, 0, "i 帧内必须跳过伤害");
    }
}
