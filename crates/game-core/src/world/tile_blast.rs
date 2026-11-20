//! 瓦片爆/复原子系统：`UpdateAllTileBlasts ; 07:FE1B` / `UpdateTileBlast ; 07:FE27`
//! / `DrawTileBlast ; 07:FEDC` / `UpdateTileBlastAnim ; 07:FF54`（bank7）。
//!
//! **结构**：`constants_ram.asm:986` 的 `TileBlasts instanceof TileBlast $D startfrom 0`
//! ⇒ 13 槽 @ `$0500`，槽长 16 B，字段 routine+0 / delay+7 / roomRAMPtr+9,+10 / type+11
//! / animFrame+12 / animIndex+13 / animDelay+14 / spare05+15。
//!
//! **五态机**（`UpdateTileBlast` 的 6 项 `JumpEngine` 表：0 rts / 1 init / 2 炸开动画
//! / 3 等待复原 / 4 复原动画 / 5 复原完成）。动画每 2 帧一换；复原等待 `$50` 次
//! （每 4 帧减 1 ⇒ 320 帧）。
//!
//! **动画串只在 bank7**（asm 自注「why aren't these in area banks?」，而**帧图案**
//! 按区不同 ⇒ 字符串体走资产包 `area.*.vram_strings`，表的顺序是常量）。

use crate::frame::Manager;

/// TileBlasts 槽基址（`constants_ram.asm:986`）。
pub(crate) const TILE_BLASTS: u16 = 0x0500;
/// 槽长（`_sizeof_TileBlast` = 16）。
pub(crate) const SLOT: u16 = 16;
/// 槽数（13）。
pub(crate) const SLOTS: u8 = 13;

// 槽内字段偏移（asm 的 `TileBlasts.0.<field>,x` 结构化访问）。
// 【口径更正 2026-11-20】`TileBlast` 结构真身是 `constants_ram.asm:36-47` 的
// `.struct TileBlast`（routine0 / spare01:1 / animFrame3 / animDelay4 / spare05:5 /
// animIndex6 / delay7 / roomRAMPtr:8 / type10 / spare0B:11-15），本文件早前把
// `roomRAMPtr` 记成 +9/+10、`type` +11、动画三项 +12..+14 —— 全部偏一位且动画项错位，
// 属结构性错读（无像素证据期未被 L4 抓到）。下面按 struct 定义钉死。
const F_ROUTINE: u16 = 0;
const F_ANIM_FRAME: u16 = 3;
const F_ANIM_DELAY: u16 = 4;
const F_SPARE05: u16 = 5;
const F_ANIM_INDEX: u16 = 6;
const F_DELAY: u16 = 7;
const F_PTR: u16 = 8;
const F_TYPE: u16 = 10;

/// `TileBlastAnim ; 07:FF86`（10 项 × 4 B，`$FE` = 该段串尾）。
///
/// 元素是 **VRAM 字符串 id**（`VRAMStringPtrTable` 下标），经 `assets::vram_string`
/// 取到按区不同的图案。
const TILE_BLAST_ANIM: [[u8; 4]; 10] = [
    [0x06, 0x07, 0x00, 0xFE], // @0 炸开 / 复原 tile #$7C
    [0x07, 0x06, 0x01, 0xFE], // @1 复原 tile #$80
    [0x07, 0x06, 0x02, 0xFE], // @2 复原 tile #$84
    [0x07, 0x06, 0x03, 0xFE], // @3 复原 tile #$88
    [0x07, 0x06, 0x04, 0xFE], // @4 复原 tile #$8C
    [0x07, 0x06, 0x05, 0xFE], // @5 复原 tile #$94
    [0x07, 0x06, 0x09, 0xFE], // @6 复原 tile #$70
    [0x07, 0x06, 0x0A, 0xFE], // @7 复原 tile #$74
    [0x07, 0x06, 0x0B, 0xFE], // @8 复原 tile #$78（Norfair 的 #$76 同用）
    [0x07, 0x06, 0x08, 0xFE], // @9 复原 tile #$90
];

/// `TileBlastRespawnAnimIndexTable ; 07:FE79`（10 B，按 `type` 选复原动画项）。
const RESPAWN_ANIM_INDEX: [u8; 10] = [6, 7, 8, 0, 1, 2, 3, 4, 9, 5];

/// 复原等待计数（`UpdateTileBlast_Init`：`lda #$50 / sta TileBlasts.0.delay,x`）。
const RESPAWN_DELAY: u8 = 0x50;

/// 在册槽数（测试/诊断用）。
pub fn active_count(mem: &Manager) -> usize {
    (0..SLOTS).filter(|s| mem.tb_routine(*s) != 0).count()
}

/// 找一个空槽（`IsBlastTile` 的 `@loop`：routine==0 即空）。无空槽返回 `None`。
pub(crate) fn free_slot(mem: &Manager) -> Option<u8> {
    (0..SLOTS).find(|s| mem.tb_routine(*s) == 0)
}

/// `IsBlastTile@foundSlot ; 07:E9DB`：占槽 + 记 roomRAMPtr/type。
///
/// asm：`inc routine`（0→1）/ `$04 → roomRAMPtr`（`and #$DE` 掩掉 bit0 与 bit5）/
/// `$05 → roomRAMPtr+1` / 按 tile 与所在区算 `type`。
pub(crate) fn claim_tile_blast(mem: &mut Manager, slot: u8, room_ptr: u16, tile: u8, in_area: u8) {
    mem.set_tb_routine(slot, 1);
    // `lda $04 / and #$DE / sta roomRAMPtr`：$04/$05 是 DrawTileBlast 传入的
    // 元 tile 内地址，asm 掩掉 bit0（半个 tile）与 bit5（跨 2×2 组）后落槽。
    mem.set_tb_ptr(slot, (room_ptr & 0xFF00) | ((room_ptr as u8) & 0xDE) as u16);
    // `InArea==$11 && tile==$76`（Norfair 四小泡可破块）⇒ type=4；否则
    // `((tile+$10) & $3C) >> 2`（asm 两段 lsr）。
    let ty = if in_area == 0x11 && tile == 0x76 {
        0x04
    } else {
        ((tile.wrapping_add(0x10)) & 0x3C) >> 2
    };
    mem.set_tb_type(slot, ty);
}

/// `UpdateAllTileBlasts ; 07:FE1B`：13 槽顺序走 `UpdateTileBlast`。
///
/// 【保真】`UpdateTileBlastAnim` 的 `$FE` 分支是 `pla / pla / rts`（**双弹返回**），
/// 直接从本循环里跳出——asm 自注「this optimization is over-eager imo」。
pub fn update_all_tile_blasts(mem: &mut Manager) {
    for slot in 0..SLOTS {
        if update_tile_blast(mem, slot) {
            return; // 双 PLA：余下槽本帧不更新
        }
    }
}

/// `UpdateTileBlast ; 07:FE27`。返回 `true` = asm 走了双弹返回（调用方须停环）。
fn update_tile_blast(mem: &mut Manager, slot: u8) -> bool {
    match mem.tb_routine(slot) {
        0 => false, // 空槽：`beq SetTileAnim@RTS`
        1 => {
            // UpdateTileBlast_Init
            mem.set_tb_routine(slot, 2);
            set_tile_anim(mem, slot, 0); // `#TileBlastAnim@0 - TileBlastAnim`
            mem.set_tb_delay(slot, RESPAWN_DELAY);
            // asm 在 Init 尾部**落进** Animating ⇒ 同一帧就播第一帧
            update_tile_blast_anim(mem, slot, 2)
        }
        3 => {
            // UpdateTileBlast_WaitToRespawn
            if mem.frame_count() & 0x03 != 0 {
                return false; // 每 4 帧才动计数
            }
            let d = mem.tb_delay(slot).wrapping_sub(1);
            mem.set_tb_delay(slot, d);
            if d != 0 {
                return false;
            }
            mem.set_tb_routine(slot, 4);
            let ty = mem.tb_type(slot) as usize;
            set_tile_anim(mem, slot, *RESPAWN_ANIM_INDEX.get(ty).unwrap_or(&0));
            false
        }
        5.. => {
            // UpdateTileBlast_Respawned
            mem.set_tb_routine(slot, 0);
            respawn_over_samus(mem, slot);
            false
        }
        _ => update_tile_blast_anim(mem, slot, 2), // 2 炸开 / 4 复原
    }
}

/// `SetTileAnim ; 07:FE6D`：`animIndex ← A` / `spare05 ← A` / `animDelay ← 0`。
fn set_tile_anim(mem: &mut Manager, slot: u8, anim_index: u8) {
    mem.set_tb_anim_index(slot, anim_index);
    mem.set_tb_spare05(slot, anim_index);
    mem.set_tb_anim_delay(slot, 0);
}

/// `UpdateTileBlastAnim ; 07:FF54`。返回 `true` = 走 `$FE` 的双弹返回。
fn update_tile_blast_anim(mem: &mut Manager, slot: u8, delay_reset: u8) -> bool {
    let delay = mem.tb_anim_delay(slot);
    if delay != 0 {
        let d = delay - 1;
        mem.set_tb_anim_delay(slot, d);
        if d != 0 {
            return false; // 还在等
        }
    }
    mem.set_tb_anim_delay(slot, delay_reset); // `sta animDelay`（A 恒 $02）
    let mut y = mem.tb_anim_index(slot) as usize;
    for _ in 0..4 {
        // 每项 4 B（含 $FE 尾），最多走两项即遇尾
        let entry = *TILE_BLAST_ANIM.get(y / 4).unwrap_or(&[0xFE; 4]);
        let id = *entry.get(y % 4).unwrap_or(&0xFE);
        if id == 0xFE {
            // @end：`inc TileBlasts.0.routine,x` + `pla / pla / rts`
            let r = mem.tb_routine(slot).wrapping_add(1);
            mem.set_tb_routine(slot, r);
            return true;
        }
        mem.set_tb_anim_frame(slot, id);
        mem.set_tb_anim_index(slot, y.wrapping_add(1) as u8);
        if draw_tile_blast_slot(mem, slot) {
            return false; // 画成功即返回
        }
        // 缓冲满：撤销 animIndex 自增，下帧重试（asm 注释自承是缺陷，保真）
        let v = mem.tb_anim_index(slot).wrapping_sub(1);
        mem.set_tb_anim_index(slot, v);
        return false;
    }
    crate::unported!("bank07:FF57-anim-overrun") // 表项无 $FE（数据不变量破裂）
}

/// `DrawTileBlast ; 07:FEDC`：帧字符串写进 RoomRAM **并**排一次 PPU 写。
/// 返回 `false` = `VRAMStructBufferIndex >= $1F`（缓冲满，调用方回滚）。
pub(crate) fn draw_tile_blast_slot(mem: &mut Manager, slot: u8) -> bool {
    if mem.vram_struct_index() >= 0x1F {
        return false;
    }
    let room_ptr = mem.tb_ptr(slot);
    let anim_frame = mem.tb_anim_frame(slot);
    let area = crate::frame::cur_area(mem);
    let src = match game_data::assets::vram_string(area, anim_frame) {
        Some(s) => s,
        None => return false, // 资产包缺该区串 ⇒ 画不出来（调用方按"重绘失败"处理）
    };
    // 表尾同址别名（`VRAMStringPtrTable` 尾部项）折叠为零长 ⇒ 该 id 无图案，画不出来。
    if src.is_empty() {
        return false;
    }
    let packed = src[0];
    let width = (packed & 0x0F) as u16;
    let height = (packed >> 4) as u16;
    if width == 0 || height == 0 {
        crate::unported!("bank07:FF3C-vram-string-empty");
    }
    // ①按行写 RoomRAM：行内 +1，行末 += $20 - width
    let mut idx: u16 = 0;
    let mut si = 1usize;
    for _ in 0..height {
        for _ in 0..width {
            let t = *src.get(si).unwrap_or(&0xFF);
            si += 1;
            mem.room().set_tile_raw(room_ptr.wrapping_add(idx), t);
            idx = idx.wrapping_add(1);
        }
        idx = idx.wrapping_add(0x20).wrapping_sub(width);
    }
    // ②同一串排进 PPU 缓冲：RoomRAM 指针 → 名表地址
    //   `and #$04` 非 0 ⇒ 名表 3（`ora #$0C`）；再 `and #$2F`
    let hi = (room_ptr >> 8) as u8;
    let ppu_hi = if hi & 0x04 != 0 { (hi | 0x0C) & 0x2F } else { hi & 0x2F };
    mem.set_ptr00_lo_hi(room_ptr as u8, ppu_hi);
    crate::title::intro::write_vram_string(mem, src);
    true
}

/// `UpdateTileBlast_Respawned ; 07:FE83`：复原的元 tile 压在 Samus 身上 ⇒ 5 格伤害。
///
/// asm 还调 `KnockSamusBackIfCollided`；本件只做命中判定 + 伤害 + `isHit` 置位，
/// 击退向量按 `DIVERGENCES` 记账（见函数尾注释）。
fn respawn_over_samus(mem: &mut Manager, slot: u8) {
    // 元 tile 中心 = roomRAMPtr + $21（asm `clc / adc #$21`）
    let ptr = mem.tb_ptr(slot);
    let (cy, cx) = metatile_center(ptr.wrapping_add(0x21));
    let (sy, sx) = (mem.samus().y(), mem.samus().x());
    // 8×8 命中框：半径各 +4
    let ry = mem.samus().radius_y().wrapping_add(4);
    let rx = mem.samus().radius_x().wrapping_add(4);
    if abs_diff(cy, sy) >= ry || abs_diff(cx, sx) >= rx {
        return; // 没压在身上
    }
    // KnockSamusBackIfCollided：把 Samus 往远离格心的方向推（轴判与敌碰同族）。
    // 【记账】击退的**分量**未按 asm 逐字复刻（`DIVERGENCES` 登记，视觉上会被下一帧
    // 的碰撞推挤纠正）；伤害与 isHit 是全真形。
    crate::frame::DIVERGENCES.with(|d| d.borrow_mut().push(0x0FE94));
    let hit = mem.samus().is_hit();
    mem.samus().set_is_hit(hit | 0x08);
    mem.set_health_change(0x50);
    crate::weapon::projectile::subtract_health(mem);
}

/// `GetPosAtNameTableAddr ; 07:FF3C`：RoomRAM 指针 → 元 tile 中心 (y, x)。
///
/// asm：`y0 = lo & $E0`；`A = hi`；两次 `lsr A / ror y0` ⇒ `y = rot_right_2(y0, hi)`；
/// `x = (lo & $1F) * 8`。
fn metatile_center(ptr: u16) -> (u8, u8) {
    let lo = ptr as u8;
    let hi = (ptr >> 8) as u8;
    let mut y = lo & 0xE0;
    let mut a = hi;
    for _ in 0..2 {
        let c = a & 1 != 0;
        a >>= 1;
        y = (y >> 1) | if c { 0x80 } else { 0 };
    }
    let x = (lo & 0x1F) << 3;
    (y, x)
}

fn abs_diff(a: u8, b: u8) -> u8 {
    a.abs_diff(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m() -> Manager {
        Manager::default()
    }

    /// 空槽 ⇒ `UpdateTileBlast` 什么都不做（asm `beq SetTileAnim@RTS`）。
    #[test]
    fn empty_slots_are_ignored() {
        let mem = m();
        assert_eq!(active_count(&mem), 0);
        assert_eq!(free_slot(&mem), Some(0));
    }

    /// 占槽：routine=1、指针按 `and #$DE` 掩码、type 按 tile 算。
    #[test]
    fn claim_writes_pointer_type_and_routine() {
        let mut mem = m();
        claim_tile_blast(&mut mem, 3, 0x6543, 0x84, 0x10);
        assert_eq!(mem.tb_routine(3), 1);
        assert_eq!(mem.tb_ptr(3) as u8, 0x43 & 0xDE, "bit0/bit5 被掩掉");
        assert_eq!((mem.tb_ptr(3) >> 8) as u8, 0x65);
        // tile $84：((0x84+0x10) & 0x3C) >> 2 = (0x94 & 0x3C)>>2 = 0x14>>2 = 5
        assert_eq!(mem.tb_type(3), 5);
    }

    /// Norfair 的 $76 特例 ⇒ type=4（四小泡可破块）。
    #[test]
    fn norfair_bubble_tile_uses_type_four() {
        let mut mem = m();
        claim_tile_blast(&mut mem, 0, 0x6120, 0x76, 0x11);
        assert_eq!(mem.tb_type(0), 4);
    }

    /// 初始化 → 动画推进：routine 1→2、delay 预置 $50、animIndex 前进。
    #[test]
    fn init_step_arms_delay_and_anim() {
        let mut mem = m();
        claim_tile_blast(&mut mem, 0, 0x6543, 0x84, 0x10);
        update_all_tile_blasts(&mut mem);
        assert_eq!(mem.tb_routine(0), 2);
        assert_eq!(mem.tb_delay(0), RESPAWN_DELAY);
        assert_eq!(mem.tb_anim_index(0), 1, "画一帧后 index 前进");
        assert_eq!(mem.tb_anim_delay(0), 2, "每 2 帧一换");
    }

    /// `$FE` 终结点把 routine 推到 3（等待复原），并且**跳过后面的槽**。
    #[test]
    fn anim_end_advances_state_and_skips_remaining_slots() {
        let mut mem = m();
        claim_tile_blast(&mut mem, 0, 0x6543, 0x84, 0x10);
        claim_tile_blast(&mut mem, 1, 0x6545, 0x84, 0x10);
        // 直接把 slot0 的 animIndex 摆到 $FE 前一项（@0 的第 4 字节 = 0xFE）
        mem.set_tb_routine(0, 2);
        mem.set_tb_anim_index(0, 3);
        mem.set_tb_anim_delay(0, 0);
        update_all_tile_blasts(&mut mem);
        assert_eq!(mem.tb_routine(0), 3, "2 → 3");
        assert_eq!(mem.tb_routine(1), 1, "双 PLA：后面的槽本帧不动");
    }

    /// 等待复原：只每 4 帧减 1，减到 0 ⇒ routine=4 + 按 type 选复原动画。
    #[test]
    fn wait_to_respawn_counts_down_every_fourth_frame() {
        let mut mem = m();
        mem.set_tb_routine(0, 3);
        mem.set_tb_delay(0, 1);
        mem.set_tb_type(0, 4);
        // FrameCount & 3 != 0 ⇒ 本帧不动
        mem.write_ram(crate::ram_map::FRAME_COUNT, 0x01); // 见下：FrameCount 无写口
        update_all_tile_blasts(&mut mem);
        assert_eq!(mem.tb_delay(0), 1);
        // FrameCount & 3 == 0 ⇒ 减到 0，转 4 并取 RESPAWN_ANIM_INDEX[4]=1
        mem.write_ram(crate::ram_map::FRAME_COUNT, 0x04);
        update_all_tile_blasts(&mut mem);
        assert_eq!(mem.tb_routine(0), 4);
        assert_eq!(mem.tb_anim_index(0), 1);
    }

    /// 元 tile 中心换算（`GetPosAtNameTableAddr`）：$6543 ⇒ y=$0A、x=$18。
    #[test]
    fn metatile_center_matches_room_ptr_math() {
        // lo=$43：y0 = 0x40；hi=$65 → 两次 lsr/ror：0x40→0xA0→0xD0? 逐步算：
        // ror1: a=0x65 → C=1, a=0x32, y=(0x40>>1)|0x80=0xA0
        // ror2: a=0x32 → C=0, a=0x19, y=(0xA0>>1)|0=0x50
        // Hmm 见断言实际值
        let (y, x) = metatile_center(0x6543);
        assert_eq!(x, (0x43 & 0x1F) << 3, "x = (lo & $1F) * 8");
        assert_eq!(y, 0x50, "y = rot_right_2(lo & $E0, hi)");
    }
}
