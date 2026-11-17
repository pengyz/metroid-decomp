//! 敌人精灵绘制（`DrawEnemy ; 07:DD8B` / `AnimDrawEnemy ; 07:F97E` 的一半）。
//!
//! 【为什么在 `render/` 而不在 `enemy/`】判据是**写入目标**：本件写 OAM
//! （`render::oam::draw_sprite_stream`），读的虽然是敌人状态/相机/按区资产，
//! 但「谁被改」决定归属。旧位置 `crates/game-core/src/enemy/anim.rs` 的名字来自
//! 「动画」——那部分已下沉为 `Anim::step` + `Enemy::advance_anim`（实体），
//! 文件名反而描述了已经不在这里的东西（与 `world/scroll.rs` 实装 `UpdateWorld` 同类错位）。
//!
//! 【跨域依赖显式化】本件依赖相机（`ScrollX/ScrollY/ScrollDir`）。旧形在核心函数里
//! 直接 `read_ram` 相机字节；现在在**边界入口**取一次，装进 [`View`] 传进核心——
//! 好处是核心可以用假 `View` 单测（不必先摆相机字节），且「绘制依赖相机」在签名上可见。
//!
//! 【无状态】不持字段、不缓存按区表指针（换区后缓存会静默失效——见 per-bank-assets 教训）。

use crate::enemy::prep::enemy_check_if_visible_inner;
use crate::ram_map;

/// 绘制所需的外部世界输入（相机面）。
///
/// `horizontal` 来自 `ScrollDir & $02`：横滚房用房间内绝对 y，竖滚房用 y−`ScrollY`。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct View {
    pub scroll_x: u8,
    pub scroll_y: u8,
    pub horizontal: bool,
}

impl View {
    /// 从场景读一次相机状态（**唯一**读相机处；核心函数只吃本结构）。
    pub(crate) fn from_scene(mem: &crate::frame::Manager) -> Self {
        Self {
            scroll_x: mem.read_ram(ram_map::SCROLL_X),
            scroll_y: mem.read_ram(ram_map::SCROLL_Y),
            horizontal: mem.read_ram(ram_map::SCROLL_DIR) & 0x02 != 0,
        }
    }
}

/// `DrawEnemy ; 07:DD8B`（敌池入口，`x` = asm「`Ens` 基址 + 偏移」索引，16 步长）。
pub(crate) fn draw_enemy(mem: &mut crate::frame::Manager, x: u8) {
    let view = View::from_scene(mem);
    draw_at(mem, x, view);
}

/// 爆炸池绘制入口（`AnimDrawEnemy ; 07:F97E` 的 `jmp DrawEnemy` 一半）。
/// **槽号入参**，基址换算收在 `crate::enemy::explosion::ens_offset`（8 步长）。
pub(crate) fn draw_explosion(mem: &mut crate::frame::Manager, slot: u8) {
    let x = crate::enemy::explosion::ens_offset(slot);
    let view = View::from_scene(mem);
    draw_at(mem, x, view);
}

fn draw_at(mem: &mut crate::frame::Manager, x: u8, view: View) {
    use game_data::assets;
    // 【②回装（2026-10-29）】asm :6156-6168：帧记字 1/2→EnsExtra.radiusY/X（IsObject-
    // Visible 用半径（序=半径写先行））+[:6178-6186] bit1 逐帧闩（and #$FD/ora（txa<<1））。
    // 爆亡链件已补（F40D/DCF5/F483 全径（415 测））⇒连锁闭环（§10 回装判据达成）。
    // **按区**取敌帧/位数据面（`docs/findings/per-bank-assets.md` §1：五区基址/表长各异）。
    let area = crate::world::room_load::area_idx(mem.read_ram(ram_map::IN_AREA)) as u8;
    let frame0 = mem.ens_face(x).anim_frame();
    if frame0 != 0xFF {
        let fi0 = ((frame0 & 0x7F) as usize) + if frame0 & 0x80 != 0 { 128 } else { 0 };
        let (fo0, fl0) = assets::enemy_frame_off(area, fi0).unwrap_or((0, 0));
        // 【goal-轮94 fo0→radiusY 直证】帧面 radiusY（金方 1/9 vs 我 8）。
        if fl0 > 2 {
            let ry = assets::enemy_frames(area)[fo0 as usize + 1];
            let rx = assets::enemy_frames(area)[fo0 as usize + 2];
            let mut f = mem.ens_face(x);
            f.set_radius_y(ry);
            f.set_radius_x(rx);
        }
    }
    // 【goal-轮75 asm 合形修】draw 走 fresh 检（:6177 `jsr IsObjectVisible` 逐字——
    // 旧形走闩径（免 fresh）=偏离）——闩径 F37F 在 update 面（非 draw）。
    let visible = enemy_check_if_visible_inner(mem, x);
    // bit1 闩（`asm :6178-6186` `and #$FD / ora (txa<<1)`）。
    let d5 = mem.ens_face(x).data05();
    mem.ens_face(x).set_data05((d5 & 0xFD) | ((visible as u8) << 1));
    if !visible {
        mem.write_ram(ram_map::OBJECT_CNTRL, 0);
        return;
    }
    let frame = mem.ens_face(x).anim_frame();
    if frame == 0xFF {
        mem.write_ram(ram_map::OBJECT_CNTRL, 0);
        return;
    }
    let table2 = frame & 0x80 != 0;
    let fi = ((frame & 0x7F) as usize) + if table2 { 128 } else { 0 };
    let (fo, fl) = assets::enemy_frame_off(area, fi).unwrap_or((0, 0));
    let blob = |i: usize| -> u8 {
        if fl == 0 || i >= fl as usize {
            0
        } else {
            assets::enemy_frames(area)[fo as usize + i]
        }
    };
    let ctrl = blob(0);
    // 【X 形根修（2026-10-31 asm WriteSpriteRAM（DEE6）逐字）】$6B=**对象控制字节**
    // （SetMirrorCntrlBit/敌 tough 面写）**非帧 ctrl**——旧形 `sta $6B=帧ctrl` 覆写
    // +镜像映射交叉（bit6→0x80）⇒四精灵翻位成 X 形（金=星芒——贴图错根）。
    // 正解：attr=Temp05 ^（ObjectCntrl bit4<<2 & HFLIP($40)）（asm :6324-6328）；
    // 镜像面 Temp04=ctrl ^ 同源（GetSpriteCntrlData :5913-5920）。
    let obj_mirror = mem.read_ram(ram_map::OBJECT_CNTRL) & 0x10; // bit4=水平镜像控制
    let _ = obj_mirror;
    let place_idx = ((ctrl & 0x0F) as usize).min(127);
    let (po, pl) = assets::enemy_place_off(area, place_idx).unwrap_or((0, 0));
    let pblob = |i: usize| -> u8 {
        if pl == 0 || i >= pl as usize {
            0
        } else {
            assets::enemy_places(area)[po as usize + i]
        }
    };
    let (y, xpos) = {
        let f = mem.ens_face(x);
        (f.y(), f.x_pos())
    };
    let sy = (if view.horizontal {
        y
    } else {
        y.wrapping_sub(view.scroll_y)
    }) as i16;
    // 【勘误修（2026-10-29（问题 A 根因））】敌屏位 x=房位−**ScrollX($FD)**——旧形减
    // $26（SpareMem26（constants:488（$25=MMC1CTRL_ZP 同族误读（sprite.rs 勘误注
    // 早已证）））——走滚径写 $FD（scroll_right）而 0x26 停旧值⇒敌屏位不减镜头⇒
    // **随镜恒距**（用户截图症（爬虫悬空错位））。
    let sx = xpos.wrapping_sub(view.scroll_x) as i16;
    let obj_cntrl = mem.read_ram(ram_map::OBJECT_CNTRL);
    // GetSpriteCntrlData（$DCC3）尾：ObjectCntrl **MSB=1 ⇒ SpriteAttrsOverride（E038）
    // `Temp05=(ctrl&$C0)|(ObjectCntrl&$7F)`**——敌侧同样适用：`UpdateEnemyCommon@noMoveNoAnim`
    // （$F419）给 tough 敌写 `$80|$03|PRIORITY`、`UpdateEnemy_Frozen`（$F443）给冻结敌写
    // `$80|$01|PRIORITY`。旧形只取 bit4 做镜像、忽略 MSB ⇒ **tough/冻结敌的调色板覆盖
    // 完全没生效**（与本轮 f2084 门精灵同一缺陷类）。
    // ---- DrawSpriteObject（$DF19）帧流环——**唯一实现**见 `render::oam::draw_sprite_stream` ----
    // 历史：本环曾被两处各自复制，`0xFC` 偏移只跳过不叠加（case-002 220px）、`0xFD`
    // 覆盖形缺失（f2084 159px）两个像素缺陷都源于「同一 asm 环写两遍」。
    crate::render::oam::draw_sprite_stream(
        mem,
        crate::render::oam::SpriteStream {
            frame_byte: &|i| blob(i as usize),
            place_byte: &|i| pblob(i as usize),
            ctrl,
            obj_cntrl,
            screen_x: sx as u8,
            screen_y: sy as u8,
        },
    );
}

#[cfg(test)]
mod tests_view {
    use super::*;

    /// 边界取相机：一次性快照，之后核心不再读相机。
    #[test]
    fn view_snapshots_camera_bytes() {
        let mut m = crate::frame::Manager::default();
        m.write_ram(ram_map::SCROLL_X, 0x2A);
        m.write_ram(ram_map::SCROLL_Y, 0x07);
        m.write_ram(ram_map::SCROLL_DIR, 0x02);
        let v = View::from_scene(&m);
        assert_eq!(v.scroll_x, 0x2A);
        assert_eq!(v.scroll_y, 0x07);
        assert!(v.horizontal, "ScrollDir bit1 置位=横滚房");
        m.write_ram(ram_map::SCROLL_DIR, 0x00);
        assert!(!View::from_scene(&m).horizontal);
    }
}
