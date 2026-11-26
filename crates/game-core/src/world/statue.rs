//! Statue 门控族（T45-1——task-37 对表核心；全仪式关键件）。
//!
//! **asm 内容锚**（`refs/m1disasm/SRC/prg7_engine.asm`，A 级逐行）：
//! · `UpdateAllStatues: ; 07:D9D4`（`:5194-5255`）：status 0 出；`==1` ⇒ `UpdateStatueBGTiles(0)`+
//!   `(1)`（降下图），两次都成功才 `inc status`；`==2` ⇒ 已抬的像补画抬起图（`(2)` Kraid / `(3)`
//!   Ridley），**只有真的调过 DrawTileBlast 且成功才 `inc status`**（`cpy #$02` 置 C=1 是为
//!   「一个都没抬」保留 status=2 的 quirk）；然后 `UpdateStatue`×2（x=Kraid/Ridley）+ Bridge。
//! · `UpdateStatue: ; 07:DA1A`：Raise → StartRaising → `animFrame ← StatueAnimFrameTable[x&1]`
//!   （Kraid=`ObjFrame_KraidStatue`=101 / Ridley=102）→ status 为 0 或 bit7 置则每帧画，否则
//!   **只在奇数帧画**（闪烁）→ `ObjDrawFrame`。
//! · `UpdateStatue_Raise: ; 07:DA3D`：RaiseState bit7 ⇒ 出；`Y&$0F==0` ⇒ 出；否则 `inc raise`
//!   （=2 raising）、`dec Y`；到 16 界 ⇒ `raise |= $80`、`status ← raise`、`inc raise`（变 $83）、
//!   再 `UpdateStatueBGTiles(x&1)`（asm 自注 useless）+ `(x&1+2)`（抬起图）。
//! · `UpdateStatue_StartRaising: ; 07:DA7C`：`Statue.y ← Y[x]`、`Statue.x ← [0x88,0x68][x&1]`；
//!   status 为 0 或 bit7 ⇒ 出；`raise != 1` ⇒ 出；`isHit == 0` ⇒ 出；命中 ⇒ `dec Y` +
//!   `SFXTriInitFlags |= sfxTri_StatueRaise`；**所有出口都清 isHit**。
//! · `UpdateStatueBGTiles: ; 07:DAB0`（y=0..3）：`TileBlasts.12` 的 ptr ← `LoTable[y]` /
//!   `(Statue.hi<<2)|HiTable[y]`、animFrame ← `$09`（2×3 纯空瓦）、`PageIndex ← 12`、
//!   `jsr DrawTileBlast`、恢复 `PageIndex`；C=0 成功。
//! · `UpdateAllStatues_Bridge: ; 07:DADA`：`$54` bit7 已生成 ⇒ 出；`DoorEntryStatus != 0` ⇒ 出；
//!   两像 status 的 bit7 相与为 0 ⇒ 出（两座都抬起才造桥）；置 `$54`；8×`TileBlasts`
//!   （routine=3 待重生 / delay=y*2（y=8..1）/ type=4 / ptr=(Statue.hi<<2)|$62 / lo=(y*2)+8）。
//! · `LoadStatues: ; 07:EC2F`（房间流 type=6）：`Statue.hi ← (PPUCTRL_ZP^ScrollDir)&1`；
//!   RidleyY ← `$30`（已抬）/`$40`；KraidY ← `$50`（已抬）/`$60`；`$54 ← 0`；`status ← 1`。
//! · 子弹命中（`prg7_engine.asm:8288-8305`，`SamusProjectileHitDoorOrStatue` 内门不命中后）：
//!   `(RoomRAMPtr>>3)&1` 选像 ⇒ `inc KraidStatueIsHit,x`（x=0 → `$0366`，x=1 → `$0367`）。
//!
//! 地址账（考证自证）：`KraidStatueStatus=$687B`（`constants_ram.asm:1234`，bit7=up）/
//! `RidleyStatueStatus=$687C`（`:1235`）/`StatuesBridgeIsSpawned=$0054`/`Statue 槽=$0360`
//! （Object 布局：hi=+$0C、y=+$0D、x=+$0E、data0F=+$0F ⇒ RaiseState `$0364/$0365`、
//! IsHit `$0366/$0367`、hi `$036C`、KraidY `$036F`、RidleyY `$0370`）/`TileBlasts=$0500`。

use crate::state::Manager;

/// 址常量（考证自证——见模块注）。
const KRAID_STATUS: u16 = 0x687B;
const RIDLEY_STATUS: u16 = 0x687C;
const BRIDGE_SPAWNED: u16 = 0x0054;
const DOOR_ENTRY_STATUS: u16 = 0x0056;
const STATUE_SLOT: u16 = 0x0360;
const STATUE_PAGE: u8 = 0x60; // Statue - Objects
const STATUE_HI: u16 = 0x036C;
const STATUE_Y: u16 = 0x036D;
const STATUE_X: u16 = 0x036E;
const KRAID_RAISE: u16 = 0x0364;
const RIDLEY_RAISE: u16 = 0x0365;
const KRAID_IS_HIT: u16 = 0x0366;
const RIDLEY_IS_HIT: u16 = 0x0367;
const KRAID_Y: u16 = 0x036F;
const RIDLEY_Y: u16 = 0x0370;
const TILE_BLASTS: u16 = 0x0500;
const SLOT: u16 = 16;
/// `TileBlasts.12`（石像专用块）。
const STATUE_TB: u8 = 12;

/// `StatueXTable ; 07:DA39` = Kraid `$88` / Ridley `$68`。
const STATUE_X_TABLE: [u8; 2] = [0x88, 0x68];
/// `StatueAnimFrameTable ; 07:DA3B` = `_id_ObjFrame_KraidStatue`(101) / `RidleyStatue`(102)。
const STATUE_ANIM: [u8; 2] = [101, 102];
/// `StatueTileBlastRoomRAMPtrLoTable ; 07:DAD2`（$6130/$60AC/$60F0/$606C）。
const TB_LO: [u8; 4] = [0x30, 0xAC, 0xF0, 0x6C];
/// `StatueTileBlastRoomRAMPtrHiTable ; 07:DAD6`（> 同名四址 ⇒ $61/$60/$60/$60）。
const TB_HI: [u8; 4] = [0x61, 0x60, 0x60, 0x60];
/// `sfxTri_StatueRaise = %00010000`（`constants.asm:166`）。
const SFX_TRI_STATUE_RAISE: u8 = 0x10;


/// `StatueXTable ; 07:DA39` = Kraid `$88` / Ridley `$68`。
/// `StatueAnimFrameTable ; 07:DA3B` = `_id_ObjFrame_KraidStatue`(101) / `RidleyStatue`(102)。
/// `StatueTileBlastRoomRAMPtrLoTable ; 07:DAD2`（$6130/$60AC/$60F0/$606C）。
/// `StatueTileBlastRoomRAMPtrHiTable ; 07:DAD6`（> 同名四址 ⇒ $61/$60/$60/$60）。
/// `sfxTri_StatueRaise = %00010000`（`constants.asm:166`）。


/// `UpdateAllStatues ; 07:D9D4`——瓦面两阶段 + 双像更新 + 造桥。
pub fn update_all_statues(mem: &mut Manager) {
    mem.set_page_index(STATUE_PAGE);
    if mem.statue_status() == 0 {
        return; // 房里没有石像
    }
    // status==1：画两块「未抬起」瓦面；两次都成功才推进到 2。
    if mem.statue_status() == 1 {
        update_statue_bg_tiles(mem, 0);
        if update_statue_bg_tiles(mem, 1) {
            let st = mem.statue_status().wrapping_add(1);
            mem.set_statue_status(st);
        }
    }
    // status==2：已抬起的补画「抬起」瓦面；只有真调过且成功才推进到 3。
    if mem.statue_status() == 2 {
        let mut called = false;
        let mut ok = false;
        if mem.statue_status_word(0) & 0x80 != 0 {
            ok = update_statue_bg_tiles(mem, 2);
            called = true;
        }
        if mem.statue_status_word(1) & 0x80 != 0 {
            ok = update_statue_bg_tiles(mem, 3);
            called = true;
        }
        // quirk：一个都没抬时 asm 的 CF 停在 `cpy #$02` 的 1 ⇒ status 保持 2。
        if called && ok {
            let st = mem.statue_status().wrapping_add(1);
            mem.set_statue_status(st);
        }
    }
    update_statue(mem, 0);
    update_statue(mem, 1);
    update_statues_bridge(mem);
}

/// `UpdateStatueBGTiles ; 07:DAB0`——把 `TileBlasts.12` 指向 y 指定的 2×3 空瓦区并重绘。
/// 返回 `DrawTileBlast` 成功（asm CF=0）。
fn update_statue_bg_tiles(mem: &mut Manager, y: u8) -> bool {
    let hi = mem.statue_hi() << 2 | TB_HI[(y & 3) as usize];
    let lo = TB_LO[(y & 3) as usize];
    mem.set_tb_ptr(STATUE_TB, ((hi as u16) << 8) | lo as u16);
    mem.set_tb_anim_frame(STATUE_TB, 0x09);
    let saved = mem.page_index();
    mem.set_page_index(STATUE_TB);
    let ok = crate::world::tile_blast::draw_tile_blast_slot(mem, STATUE_TB);
    mem.set_page_index(saved); // asm：`lda #Statue - Objects / sta PageIndex`
    ok
}

/// `UpdateStatue_Raise ; 07:DA3D`——1px/call 升到 16 界，完成时置双 bit7 并补画抬起图。
pub fn update_statue_raise(mem: &mut Manager, idx: u8) {
    let (raise, y_addr, status) = statue_addrs(idx);
    if mem.statue_raise_state(idx) & 0x80 != 0 {
        return; // 已抬起
    }
    mem.set_statue_raise_state(idx, 0x01); // not raised
    if mem.statue_raise_y(idx) & 0x0F == 0 {
        return; // 已对齐
    }
    let r = mem.statue_raise_state(idx).wrapping_add(1); // raising（1→2）
    mem.set_statue_raise_state(idx, r);
    let y = mem.statue_raise_y(idx).wrapping_sub(1);
    mem.set_statue_raise_y(idx, y);
    if y & 0x0F != 0 {
        return; // 未完
    }
    let done = mem.statue_raise_state(idx) | 0x80;
    mem.set_statue_raise_state(idx, done);
    mem.set_statue_status_word(idx, done);
    mem.set_statue_raise_state(idx, done.wrapping_add(1)); // asm：inc（$81→$82… 惯例保留）
    update_statue_bg_tiles(mem, idx & 1); // asm 自注 useless，但确实重绘
    update_statue_bg_tiles(mem, (idx & 1) + 2); // 抬起图
}

/// `UpdateStatue_StartRaising ; 07:DA7C`——命中后首发上移 1px（后续 15px 交给 Raise）。
pub fn update_statue_start_raising(mem: &mut Manager, idx: u8) {
    let (raise, y_addr, status) = statue_addrs(idx);
    let hit_addr = if idx == 0 { KRAID_IS_HIT } else { RIDLEY_IS_HIT };
    mem.set_statue_y(mem.statue_raise_y(idx));
    mem.set_statue_x(STATUE_X_TABLE[(idx & 1) as usize]);
    let st = mem.statue_status_word(idx);
    if st != 0 && st & 0x80 == 0 && mem.statue_raise_state(idx) == 0x01 && mem.statue_is_hit(idx) != 0 {
        mem.set_statue_raise_y(idx, mem.statue_raise_y(idx).wrapping_sub(1));
        let f = mem.tri_init_flags() | SFX_TRI_STATUE_RAISE;
        mem.set_tri_init_flags(f);
    }
    mem.set_statue_is_hit(idx, 0); // 所有出口都清
}

fn statue_addrs(idx: u8) -> (u16, u16, u16) {
    if idx == 0 {
        (KRAID_RAISE, KRAID_Y, KRAID_STATUS)
    } else {
        (RIDLEY_RAISE, RIDLEY_Y, RIDLEY_STATUS)
    }
}

/// `UpdateStatue ; 07:DA1A`——Raise → StartRaising → 动画帧 → 绘制（闪像只画奇数帧）。
fn update_statue(mem: &mut Manager, idx: u8) {
    update_statue_raise(mem, idx);
    update_statue_start_raising(mem, idx);
    mem.set_statue_anim_frame(STATUE_ANIM[(idx & 1) as usize]); // animFrame ← 表
    let st = mem.statue_status_word(idx);
    if st == 0 || st & 0x80 != 0 {
        crate::render::oam::obj_draw_frame(mem, STATUE_PAGE);
        return;
    }
    if mem.frame_count() & 1 == 0 {
        return; // 闪像：只在奇数帧画
    }
    crate::render::oam::obj_draw_frame(mem, STATUE_PAGE);
}

/// `LoadStatues ; 07:EC2F`——房间里出现石像时初始化（房间流 type=6）。
pub fn load_statues(mem: &mut Manager) {
    let nt = (mem.ppuctrl_zp() ^ mem.camera().dir()) & 1;
    mem.set_statue_hi(nt);
    mem.write_ram(RIDLEY_Y, if mem.statue_status_word(1) & 0x80 != 0 { 0x30 } else { 0x40 });
    mem.write_ram(KRAID_Y, if mem.statue_status_word(0) & 0x80 != 0 { 0x50 } else { 0x60 });
    mem.set_statues_bridge_spawned(0);
    mem.set_statue_status(1);
}

/// 子弹命中石像（as `SamusProjectileHitDoorOrStatue` 的门循环之后）：
/// 用 RoomRAM 指针 bit3 区分两座（Kraid 低 nibble `$0/$1`、Ridley `$C/$D`）。
pub fn statue_register_hit(mem: &mut Manager, ptr_lo: u8) {
    let idx = (ptr_lo >> 3) & 1;
    let addr = if idx == 0 { KRAID_IS_HIT } else { RIDLEY_IS_HIT };
    mem.set_statue_is_hit(idx, mem.statue_is_hit(idx).wrapping_add(1));
}

/// UpdateAllStatues_Bridge（**task-37 对表核心**——门判①②③+8×TileBlasts 生成）。
pub fn update_statues_bridge(mem: &mut Manager) {
    if mem.statues_bridge_spawned() & 0x80 != 0 {
        return; // ① 已生成
    }
    if mem.door_entry_status() != 0 {
        return; // ② 在门内
    }
    let both = mem.statue_status_word(0) & mem.statue_status_word(1);
    if both & 0x80 == 0 {
        return; // ③ 双 AND 门（双 Boss 击败=双像 bit7）
    }
    mem.set_statues_bridge_spawned(0x80); // 置位
    // 8×TileBlasts 生成（routine=3 待重生/delay=y*2（y=8..1）/type=4 generic shot）
    // asm（`UpdateAllStatues_Bridge`，prg7_engine.asm:5416-5432）除 routine/delay/type
    // 外还写 roomRAMPtr 两个字节：`roomRAMPtr ← y*2+8`、`roomRAMPtr+1 ← (Statue.hi<<2)|$62`。
    // 缺这两笔时槽里的指针是 0，等 delay 走完进「复原」态 `DrawTileBlast` 会拿 0 当
    // RoomRAM 地址写（`write_wram` 下溢 panic）——桥一生成就炸。
    let ptr_hi = (mem.statue_hi() << 2) | 0x62;
    for j in 0..8u16 {
        let y = 8 - j; // asm 的 y 从 8 递减到 1
        let ptr_lo = (y as u8).wrapping_mul(2).wrapping_add(8);
        mem.set_tb_ptr(j as u8, ((ptr_hi as u16) << 8) | ptr_lo as u16);
        mem.set_tb_routine(j as u8, 0x03);
        mem.set_tb_delay(j as u8, (y as u8).wrapping_mul(2));
        mem.set_tb_type(j as u8, 0x04);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【全仪式·桥门判】双 AND 门+8 瓦爆生成（task-37 核心面）。
    #[test]
    fn bridge_gate_and_tile_blasts() {
        let mut m = Manager::default();
        m.write_ram(STATUE_SLOT, 2);
        // 双像未全 up→门不开
        m.write_ram(KRAID_STATUS, 0x80);
        update_statues_bridge(&mut m);
        assert_eq!(m.read_ram(BRIDGE_SPAWNED), 0);
        // 双 up→桥生成
        m.write_ram(RIDLEY_STATUS, 0x80);
        update_statues_bridge(&mut m);
        assert_eq!(m.read_ram(BRIDGE_SPAWNED), 0x80);
        // 8 瓦爆：slot0 delay=16/type=4/routine=3
        assert_eq!(m.read_ram(TILE_BLASTS), 0x03);
        assert_eq!(m.read_ram(TILE_BLASTS + 7), 16);
        assert_eq!(m.read_ram(TILE_BLASTS + 10), 0x04);
        assert_eq!(m.read_ram(TILE_BLASTS + 7 * SLOT + 7), 2); // slot7 delay=2
        // roomRAMPtr 两字节（asm 5416-5432）：缺它时 delay 走完进复原态会拿 0 当
        // RoomRAM 地址写（write_wram 下溢 panic）。hi=(Statue.hi<<2)|$62，lo=y*2+8。
        assert_eq!(m.tb_ptr(0), 0x6218, "slot0: y=8 ⇒ lo=8*2+8=0x18，hi=$62");
        assert_eq!(m.tb_ptr(7), 0x620A, "slot7: y=1 ⇒ lo=0x0A");
        m.set_statue_hi(1);
        m.write_ram(BRIDGE_SPAWNED, 0);
        update_statues_bridge(&mut m);
        assert_eq!(m.tb_ptr(0), 0x6618, "hi=$62|(Statue.hi<<2)=$66");
    }

    /// 【全仪式·raise 状态机】1px/call 到 16 界→bit7=up 双址置。
    #[test]
    fn statue_raise_state_machine() {
        let mut m = Manager::default();
        m.write_ram(KRAID_Y, 0x1F); // 低半字节 F=15px 待升（asm 语义：Y&$0F=剩余偏移）
        for _ in 0..15 {
            update_statue_raise(&mut m, 0);
        }
        assert_eq!(m.read_ram(KRAID_RAISE) & 0x80, 0x80); // raised
        assert_eq!(m.read_ram(KRAID_STATUS) & 0x80, 0x80); // bit7=up
        assert_eq!(m.read_ram(KRAID_Y), 0x10); // 低半字节 0=对齐完成
    }

    /// 【全仪式·命中→首发】`UpdateStatue_StartRaising`：命中后首帧上移 1px + 触发音 + 清命中位。
    #[test]
    fn start_raising_first_pixel_and_sfx() {
        let mut m = Manager::default();
        m.write_ram(STATUE_SLOT, 3); // status 已推进（≠0 且 bit7 清）
        m.write_ram(KRAID_STATUS, 0x01); // 闪烁中（bit7 清）
        m.write_ram(KRAID_RAISE, 0x01); // not raised
        m.write_ram(KRAID_Y, 0x60);
        m.write_ram(KRAID_IS_HIT, 0x02); // 命中两次（asm 只看是否非 0）
        update_statue_start_raising(&mut m, 0);
        assert_eq!(m.read_ram(KRAID_Y), 0x5F, "首发上移 1px");
        assert_eq!(m.read_ram(STATUE_Y), 0x60, "Statue.y ← 递减前的 Y（asm 先抄位置再 dec）");
        assert_eq!(m.read_ram(STATUE_X), 0x88, "Kraid X=$88");
        assert_eq!(m.read_ram(crate::ram_map::SFX_TRI_INIT) & 0x10, 0x10, "sfxTri_StatueRaise");
        assert_eq!(m.read_ram(KRAID_IS_HIT), 0, "出口清命中位");
    }

    /// 【全仪式·未命中不清移动】没有命中位时不动 Y，但同样清命中位。
    #[test]
    fn start_raising_without_hit_keeps_y() {
        let mut m = Manager::default();
        m.write_ram(KRAID_STATUS, 0x01);
        m.write_ram(KRAID_RAISE, 0x01);
        m.write_ram(KRAID_Y, 0x60);
        update_statue_start_raising(&mut m, 0);
        assert_eq!(m.read_ram(KRAID_Y), 0x60);
        assert_eq!(m.read_ram(STATUE_X), 0x88);
    }

    /// 【全仪式·抬起完成】Raise 收尾把 RaiseState 置 $83、Status 置 bit7。
    #[test]
    fn raise_completion_writes_raise_0x83_and_status_bit7() {
        let mut m = Manager::default();
        m.write_ram(STATUE_SLOT, 3);
        m.write_ram(RIDLEY_Y, 0x11); // 低半字节 1 ⇒ 再降 1px 到位
        update_statue_raise(&mut m, 1);
        assert_eq!(m.read_ram(RIDLEY_Y), 0x10);
        assert_eq!(m.read_ram(RIDLEY_RAISE), 0x83, "bit7 置后再 inc（asm 的 $81→$82 惯例到 $83）");
        assert_eq!(m.read_ram(RIDLEY_STATUS), 0x82, "status ← raise（含 bit7）");
    }

    /// 【全仪式·status==2 的 carry quirk】两座都没抬起时 status 停在 2（不进 3）。
    #[test]
    fn status_2_stays_when_no_statue_raised() {
        let mut m = Manager::default();
        m.write_ram(STATUE_SLOT, 2);
        update_all_statues(&mut m);
        assert_eq!(m.read_ram(STATUE_SLOT), 2, "asm `cpy #$02` 的 C=1 ⇒ 不 inc");
    }

    /// 【全仪式·LoadStatues】位置按「已抬/未抬」选，桥旗清零、status=1。
    #[test]
    fn load_statues_initializes_positions() {
        let mut m = Manager::default();
        m.write_ram(BRIDGE_SPAWNED, 0x80);
        m.write_ram(RIDLEY_STATUS, 0x80); // Ridley 已抬 ⇒ $30
        m.write_ram(KRAID_STATUS, 0x00); // Kraid 未抬 ⇒ $60
        load_statues(&mut m);
        assert_eq!(m.read_ram(RIDLEY_Y), 0x30);
        assert_eq!(m.read_ram(KRAID_Y), 0x60);
        assert_eq!(m.read_ram(BRIDGE_SPAWNED), 0);
        assert_eq!(m.read_ram(STATUE_SLOT), 1);
    }

    /// 【全仪式·命中归属】指针 bit3 决定是 Kraid 还是 Ridley。
    #[test]
    fn statue_hit_routes_by_pointer_bit3() {
        let mut m = Manager::default();
        statue_register_hit(&mut m, 0x01); // 低位 $0/$1 ⇒ Kraid
        assert_eq!(m.read_ram(KRAID_IS_HIT), 1);
        assert_eq!(m.read_ram(RIDLEY_IS_HIT), 0);
        statue_register_hit(&mut m, 0x0C); // $C/$D ⇒ Ridley
        assert_eq!(m.read_ram(RIDLEY_IS_HIT), 1);
    }
}
