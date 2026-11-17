//! 敌人绘制与动画实体化（`DrawEnemy` 07:F42E 邻域）。

use crate::enemy::anim_idx::AnimIdx;
use crate::enemy::framework::*;
use crate::enemy::motion::*;
use crate::enemy::prep::*;
use crate::frame::tick::*;
use crate::ram_map;

/// DrawEnemy（**asm 内容锚** `DrawEnemy: ; 07:DD8B`——r26 端口）：animFrame==FrameNone⇒
/// ClearObjectCntrl；帧指针（EnFramePtrTable1/2——animFrame bit7 carry 选表！）→
/// GetSpriteCntrlData→位指针（EnPlacePtrTable）→sprite 管线写（place_disp/四元组）。
pub(crate) fn draw_enemy(mem: &mut crate::frame::Manager, x: u8) {
    draw_at(mem, x);
}

/// 爆炸池绘制入口（`AnimDrawEnemy ; 07:F97E` 的 `jmp DrawEnemy` 一半）。
pub(crate) fn draw_explosion(mem: &mut crate::frame::Manager, slot: u8) {
    let x = crate::enemy::explosion::ens_offset(slot);
    draw_at(mem, x);
}

fn draw_at(mem: &mut crate::frame::Manager, x: u8) {
    // 【goal-轮73 draw 调用直证】双写源（n=803 清→置双写）。
    use game_data::assets;
    // 【②回装（2026-10-29）】asm :6156-6168：帧记字 1/2→EnsExtra.radiusY/X（IsObject-
    // Visible 用半径（序=半径写先行））+[:6178-6186] bit1 逐帧闩（and #$FD/ora（txa<<1））。
    // 爆亡链件已补（F40D/DCF5/F483 全径（415 测））⇒连锁闭环（§10 回装判据达成）。
    // **按区**取敌帧/位数据面（`docs/findings/per-bank-assets.md` §1：五区基址/表长各异）。
    let area = crate::world::room_load::area_idx(mem.read_ram(ram_map::IN_AREA)) as u8;
    let frame0 = mem.read_wram(0x6AF4 + x as u16 + 3);
    if frame0 != 0xFF {
        let fi0 = ((frame0 & 0x7F) as usize) + if frame0 & 0x80 != 0 { 128 } else { 0 };
        let (fo0, fl0) = assets::enemy_frame_off(area, fi0 as usize).unwrap_or((0, 0));
    // 【goal-轮94 fo0→radiusY 直证】帧面 radiusY（金方 1/9 vs 我 8）。
        if fl0 > 2 {
            mem.write_wram(0x6AF4 + x as u16 + 1, assets::enemy_frames(area)[fo0 as usize + 1]);
            mem.write_wram(0x6AF4 + x as u16 + 2, assets::enemy_frames(area)[fo0 as usize + 2]);
        }
    }
    // 【goal-轮75 asm 合形修】draw 走 fresh 检（:6177 `jsr IsObjectVisible` 逐字——
    // 旧形走闩径（免 fresh）=偏离）——闩径 F37F 在 update 面（非 draw）。
    let visible = enemy_check_if_visible_inner(mem, x);
    let e5 = 0x0400 + x as u16 + 5;
    let d5 = mem.read_ram(e5);
    mem.write_ram(e5, (d5 & 0xFD) | ((visible as u8) << 1)); // bit1 闩
    if !visible {
        mem.write_ram(0x006B, 0);
        return;
    }
    let frame = mem.read_wram(0x6AF4 + x as u16 + 3); // animFrame=+3
    if frame == 0xFF {
        mem.write_ram(0x006B, 0);
        return;
    }
    let table2 = frame & 0x80 != 0;
    let fi = ((frame & 0x7F) as usize) + if table2 { 128 } else { 0 };
    let (fo, fl) = assets::enemy_frame_off(area, fi as usize).unwrap_or((0, 0));
    let blob = |i: usize| -> u8 {
        if fl == 0 || i >= fl as usize { 0 } else { assets::enemy_frames(area)[fo as usize + i] }
    };
    let ctrl = blob(0);
    // 【X 形根修（2026-10-31 asm WriteSpriteRAM（DEE6）逐字）】$6B=**对象控制字节**
    // （SetMirrorCntrlBit/敌 tough 面写）**非帧 ctrl**——旧形 `sta $6B=帧ctrl` 覆写
    // +镜像映射交叉（bit6→0x80）⇒四精灵翻位成 X 形（金=星芒——贴图错根）。
    // 正解：attr=Temp05 ^（ObjectCntrl bit4<<2 & HFLIP($40)）（asm :6324-6328）；
    // 镜像面 Temp04=ctrl ^ 同源（GetSpriteCntrlData :5913-5920）。
    let obj_mirror = mem.read_ram(0x006B) & 0x10; // bit4=水平镜像控制
    let place_idx = ((ctrl & 0x0F) as usize).min(127);
    let (po, pl) = assets::enemy_place_off(area, place_idx as usize).unwrap_or((0, 0));
    let pblob = |i: usize| -> u8 {
        if pl == 0 || i >= pl as usize { 0 } else { assets::enemy_places(area)[po as usize + i] }
    };
    let y = mem.read_ram(0x0400 + x as u16 + 0); // y=+0
    let xpos = mem.read_ram(0x0400 + x as u16 + 1); // x=+1
    let scroll_y = mem.read_ram(crate::ram_map::SCROLL_Y);
    let horz = mem.read_ram(crate::ram_map::SCROLL_DIR) & 0x02 != 0;
    let sy = (if horz { y } else { y.wrapping_sub(scroll_y) }) as i16;
    // 【勘误修（2026-10-29（问题 A 根因））】敌屏位 x=房位−**ScrollX($FD)**——旧形减
    // $26（SpareMem26（constants:488（$25=MMC1CTRL_ZP 同族误读（sprite.rs 勘误注
    // 早已证）））——走滚径写 $FD（scroll_right）而 0x26 停旧值⇒敌屏位不减镜头⇒
    // **随镜恒距**（用户截图症（爬虫悬空错位））。
    let sx = xpos.wrapping_sub(mem.read_ram(crate::ram_map::SCROLL_X)) as i16;
    let obj_cntrl = mem.read_ram(0x006B);
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

/// `UpdateEnemyAnim ; 07:E094`（走表动画推进）。
/// UpdateEnemyAnim（**asm 内容锚** `UpdateEnemyAnim: ; 07:E094`——敌人动画走表全径）：
/// status==Pickup(5)⇒RTS；delay 门（≠0 递减未归零⇒RTS）→`sta animDelay←A`→走
/// EN_ANIM_TABLE[animIndex]（区表——Brinstar $9D6A）：FF⇒animIndex←reset 续走；
/// 否则 animFrame(+3)←表值、animIndex+1。调用形：CrawlerAI @exit `lda #3/jsr $800C`。
pub(crate) fn update_enemy_anim(mem: &mut crate::frame::Manager, x: u8, a: u8) {
    update_anim_at(mem, x, a);
}

/// 爆炸池的同一例程入口（`UpdateEnemyExplosion` 尾件 `jmp AnimDrawEnemy` 的
/// `jsr UpdateEnemyAnim` 一半）。**槽号入参**，基址换算收在 `explosion::ens_offset`。
///
/// 【为什么要有这两个入口】`UpdateEnemyAnim` 是 asm 里被敌池与爆炸池共用的例程：
/// 敌槽 `x` 16 步长、爆炸槽 `x` 8 步长，而 `$6AF4 + $C0` 恰等于爆炸池额外面基址
/// `$6BB4`。此前调用方直接把爆炸偏移喂给"敌函数"，靠朴素的加法巧合生效——
/// 一旦有人把内部改成 `slot_of(x)` 就整条静默错位（2026-11-17 试改即被池测抓出）。
/// 现在两个池各有具名入口，巧合被关在 `update_anim_at` 里且有 pin 测守着。
pub(crate) fn update_explosion_anim(mem: &mut crate::frame::Manager, slot: u8, a: u8) {
    // 与敌侧同源：表由调用方按区取，推进在实体（`Explosion::advance_anim`，无 Pickup 门）。
    let area = crate::world::room_load::area_idx(mem.read_ram(ram_map::IN_AREA)) as u8;
    let table = game_data::assets::enemy_anim_table(area);
    mem.explosion(slot).advance_anim(table, a);
}

fn update_anim_at(mem: &mut crate::frame::Manager, x: u8, a: u8) {
    // 【2026-11-17 三层归位】走表逻辑已下沉到 `Anim::step`（值对象，纯函数），
    // Pickup 门与写回在 `Enemy::advance_anim`（实体）。本函数只剩「按区取表 + 分派到句柄」
    // 这一件跨域适配——资产是外部依赖，实体不该自己去查。
    let area = crate::world::room_load::area_idx(mem.read_ram(ram_map::IN_AREA)) as u8;
    let table = game_data::assets::enemy_anim_table(area);
    mem.enemy(crate::enemy::state::slot_of(x)).advance_anim(table, a);
}


#[cfg(test)]
mod tests_dual_pool_base {
    /// **钉住双池共享的地址恒等式**：`update_enemy_anim`/`draw_enemy` 的 `x` 同时用于
    /// 敌槽（0..=0xAF，步长 16）与爆炸池（0xC0..=0xD8，步长 8）。这条恒等式一旦被
    /// 重构者"修正"成纯敌槽换算，爆炸池的动画会静默错位（见函数头注释）。
    #[test]
    fn explosion_pool_base_is_reachable_from_ens_offset() {
        for slot in 0..crate::enemy::explosion::SLOT_COUNT {
            let ex = crate::enemy::explosion::ens_offset(slot);
            assert!(ex >= 0xC0, "爆炸槽的 asm x 必须在 $C0 之上（got {ex:#04X}）");
            assert_eq!(
                crate::enemy::state::ENS_EXTRA_BASE + ex as u16,
                crate::enemy::explosion::EXTRA_BASE + slot as u16 * 8,
                "`$6AF4 + x` 必须落在爆炸池额外面（slot {slot}）"
            );
            // 换算成敌槽号会落到 12 及以上（`$C0/16=12`）——即**超出敌槽 0..=11 的合法范围**，
            // 这正是不能把共享例程的 `x` 当成敌槽偏移套 `slot_of()` 的原因。
            assert!(
                ex as usize / 16 >= crate::enemy::state::SLOT_COUNT as usize,
                "爆炸槽 {slot} 换算成敌槽号会落进合法敌槽区间（x={ex:#04X}）"
            );
        }
    }
}
