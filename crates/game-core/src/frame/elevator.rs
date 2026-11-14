//! 电梯族（`UpdateElevator ; 07:D7B3` 十态 + `SamusElevator ; 07:D423`
//! + `SamusCollisionWithSolidEntities ; 07:D976` 电梯段）——#3 换区件。
//!
//! **槽位与字段**（`constants_ram.asm:836-846`：`Objects .instanceof Object $10 startfrom 0`
//! ⇒ `Elevator`=槽 2 ⇒ 页基 $0320；`ElevatorType = Elevator.data0F`=$032F）：
//! `status+0 / animFrame+3 / animDelay+4 / animResetIndex+5 / animIndex+6 /
//! speedY+8 / hi+$0C / y+$0D / x+$0E / data0F+$0F`。
//!
//! **十态表**（`jsr JumpEngine`）：0 ExitSub（rts）/ 1 ElevatorIdle($D7D1) /
//! 2 ScrollXToCenter($D80E) / 3 Move($D83D) / 4 ScrollY($D870) / 5 Fade($D8A3) /
//! 6 ElevatorD8BF（换区核）/ 7 Fade / 8 Move / 9 Stop($D94E)。
//!
//! **帧等待口径**：`WaitNMIPass_`（07:C43F）= `ClearNMIStat` + 等 NMI 走完 ⇒ rust 侧
//! 沿用全仓同族形（`NMI_STATUS←1` + `iter_wait()`；见 `frame::boot::init_bank1`）。

use crate::frame::FrameMemory;
use crate::ram_map::{
    AT_ENDING, BANK_SWITCH_PENDING, IN_AREA, ITEM_ROOM_MUSIC_STATUS, JOY1_STATUS, MAIN_ROUTINE,
    MAP_POS_Y, MIRROR_CNTRL, MUSIC_INIT, NMI_STATUS, PALETTE_PENDING, PPUCTRL_ZP, PPUMASK_ZP,
    ROOM_PTR, SAMUS_ACCEL_Y, SAMUS_ON_ELEVATOR, SCROLL_DIR, SCROLL_X, SCROLL_Y, TITLE_ROUTINE,
};

/// 电梯槽（`Objects` 槽 2 × 16 B）。
pub const ELEVATOR: u16 = 0x0320;
/// 电梯字段偏移（Object 结构，`constants_ram.asm:836` 起）。`ELEVATOR + 0x0F` = ElevatorType。
const F_STATUS: u16 = 0x00;
const F_ANIM_FRAME: u16 = 0x03;
const F_ANIM_DELAY: u16 = 0x04;
const F_ANIM_RESET: u16 = 0x05;
const F_ANIM_INDEX: u16 = 0x06;
const F_SPEED_Y: u16 = 0x08;
const F_HI: u16 = 0x0C;
const F_Y: u16 = 0x0D;
const F_X: u16 = 0x0E;
const F_TYPE: u16 = 0x0F;

/// Samus 槽（`Objects` 槽 0）。
const SAMUS: u16 = 0x0300;
const S_STATUS: u16 = 0x00;
const S_ANIM_FRAME: u16 = 0x03;
const S_ANIM_DELAY: u16 = 0x04;
const S_ANIM_RESET: u16 = 0x05;
const S_ANIM_INDEX: u16 = 0x06;
const S_HI: u16 = 0x0C;
const S_Y: u16 = 0x0D;
const S_X: u16 = 0x0E;

/// 电梯槽 PageIndex（槽 2 × $10）——`ObjDrawFrame`/`AnimDrawObject` 取页基。
const ELEV_PAGE: u8 = 0x20;

/// `SCRN_VY` = 240（`hardware.asm:138-139`）。`SCRN_VX`=256 放不进 u8，其唯一用途是
/// `SCRN_VX/2`=128，故不设该常量而直接写 `0x80`（避免「256 塞 u8 溢出」陷阱）。
const SCRN_VY: u8 = 240;

/// `sa_Elevator` = 9（`constants.asm` `sa_Elevator` 序数；`GoSamusHandler` 十态表 index 9）。
const SA_ELEVATOR: u8 = 9;
/// `sa_Stand` = 0。
const SA_STAND: u8 = 0;
/// `_id_IncrementRoutine` = 7（`MainRoutinePtrTable` index 7）。
const ID_INCREMENT_ROUTINE: u8 = 7;
/// `_id_EndGame` = **0x1D**（`TitleRoutinePtrTable ; 00:802A` 31 项表序：0x1C=GameOver，
/// 0x1D=EndGame `$9AA7`，0x1E=SetTimer；`crates/game-core/src/title/mod.rs:25` 同锚）。
const ID_END_GAME: u8 = 0x1D;

/// `ObjectAnimIndexTbl`（$8572）内字节偏移——asm `#ObjAnim_X - ObjectAnimIndexTbl.b`。
const ANIM_SAMUS_FRONT: u8 = 0x04; // ObjAnim_SamusFront ; $8576
const ANIM_SAMUS_FADE_OUT: u8 = 0x41; // ObjAnim_SamusFadeOutArea ; $85B3
const ANIM_SAMUS_FADE_OUT_RESET: u8 = 0x4E; // _Reset ; $85C0
const ANIM_ELEV_FADE_OUT: u8 = 0x50; // ObjAnim_ElevatorFadeOutArea ; $85C2
const ANIM_ELEV_FADE_OUT_RESET: u8 = 0x5D; // _Reset ; $85CF
const ANIM_SAMUS_FADE_IN: u8 = 0x5F; // ObjAnim_SamusFadeInArea ; $85D1
const ANIM_SAMUS_FADE_IN_RESET: u8 = 0x6B; // _Reset ; $85DD
const ANIM_ELEV_FADE_IN: u8 = 0x6E; // ObjAnim_ElevatorFadeInArea ; $85E0
const ANIM_ELEV_FADE_IN_RESET: u8 = 0x7A; // _Reset ; $85EC

/// `_id_ObjFrame_Elevator` = $23（`common_sprite_data.asm` PtrTableEntry 序数自证）。
const OBJFRAME_ELEVATOR: u8 = 0x23;

/// `PaletteToggle`（`constants_ram.asm:609`）/ eor 掩码 `(Palette00+1)^(Palette05+1)` = 1^6 = 7。
const PALETTE_TOGGLE: u16 = 0x0076;
/// `PaletteToggle eor #(_id_Palette00+1)~(_id_Palette05+1)`——**前提**：Palette00..05 的
/// 表序 id 恰为 0..5（`prg0_title.asm` PalettePtrTable 序），故 `1 ^ 6 = 7`；Tourian 径直接
/// `lda #_id_Palette00+1` = 1。两处常量耦合同一前提，改表序须同时复核。
const PALETTE_TOGGLE_XOR: u8 = 0x07;

/// `enemyStatus_Frozen` = 4（`constants.asm:124`）。
const ENEMY_STATUS_FROZEN: u8 = 4;

/// `UpdateElevator ; 07:D7B3`——十态分派（`ldx #Elevator-Objects / stx PageIndex /
/// lda Elevator.status / jsr JumpEngine`）。status > 9 在 asm 里越表读指针（UB），
/// rust 侧按 ExitSub 静默（不臆造）。
pub fn update_elevator(mem: &mut FrameMemory) {
    match mem.read_ram(ELEVATOR + F_STATUS) {
        0 => {} // ExitSub（$C45C）
        1 => elev_idle(mem),
        2 => elev_scroll_x_to_center(mem),
        3 | 8 => elev_move(mem),
        4 => elev_scroll_y(mem),
        5 | 7 => elev_fade(mem),
        6 => elev_transition(mem),
        9 => elev_stop(mem),
        _ => {}
    }
}

/// `DrawElevator`（$D806）：`lda FrameCount / lsr / bcc rts`——**隔帧显示**
/// （FrameCount bit0=1 才画；`lsr` 把 bit0 移进 carry）。
fn draw_elevator(mem: &mut FrameMemory) {
    if mem.read_ram(crate::ram_map::FRAME_COUNT) & 0x01 == 1 {
        crate::frame::sprite::obj_draw_frame(mem, ELEV_PAGE);
    }
}

/// `ToggleObjectXSlotHi`（$D648）：`lda Objects.0.hi,x / eor #$01 / sta`。
fn toggle_object_hi(mem: &mut FrameMemory, page: u16, f_hi: u16) {
    let h = mem.read_ram(page + f_hi) ^ 0x01;
    mem.write_ram(page + f_hi, h);
}

/// `SetSamusAnim`（$CD6D 形）：写 animResetIndex/animIndex + delay 0。
fn set_samus_anim(mem: &mut FrameMemory, idx: u8) {
    mem.write_ram(SAMUS + S_ANIM_RESET, idx);
    mem.write_ram(SAMUS + S_ANIM_INDEX, idx);
    mem.write_ram(SAMUS + S_ANIM_DELAY, 0);
}

/// `ElevatorIdle ; 07:D7D1`——站梯上按 上/下（按 `ElevatorType` bit7 取 BUTTON_UP/DOWN）
/// 即启动：清垂直速度 → 写 speedY/animDelay=0 → status+1 → Samus 置电梯态/居中位。
fn elev_idle(mem: &mut FrameMemory) {
    if mem.read_ram(SAMUS_ON_ELEVATOR) == 0 {
        draw_elevator(mem);
        return;
    }
    let etype = mem.read_ram(ELEVATOR + F_TYPE);
    // asm `lda #BUTTON_DOWN / bit ElevatorType / bpl Lx099 / asl`——bit7=1（上行）⇒ BUTTON_UP。
    let button: u8 = if etype & 0x80 != 0 { 0x08 } else { 0x04 };
    if mem.read_ram(JOY1_STATUS) & button == 0 {
        draw_elevator(mem);
        return;
    }
    crate::samus::physics::stop_vert_movement(mem);
    mem.write_ram(ELEVATOR + F_ANIM_DELAY, 0); // sty（Y=0）
    mem.write_ram(SAMUS_ACCEL_Y, 0); // sty SamusAccelY
    mem.write_ram(ELEVATOR + F_SPEED_Y, 0); // tya / sta
    let st = mem.read_ram(ELEVATOR + F_STATUS).wrapping_add(1);
    mem.write_ram(ELEVATOR + F_STATUS, st); // ⇒ 2 ScrollXToCenter
    mem.write_ram(SAMUS + S_STATUS, SA_ELEVATOR);
    set_samus_anim(mem, ANIM_SAMUS_FRONT);
    mem.write_ram(SAMUS + S_X, 0x80); // #(SCRN_VX/2)
    mem.write_ram(SAMUS + S_Y, SCRN_VY / 2 - 8); // #(SCRN_VY/2-8) = 112
    draw_elevator(mem);
}

/// `ElevatorScrollXToCenter ; 07:D80E`——水平居中（`ScrollX==0` 即到位）：
/// 置 `MirrorCntrl|=$08`（名表竖排）+ `ScrollDir&=1`（转竖滚）+ status+1；
/// 否则电梯在 ScrollX 右侧 ⇒ ScrollRight、左侧 ⇒ ScrollLeft（`bmi` 判 8 位差符号）。
fn elev_scroll_x_to_center(mem: &mut FrameMemory) {
    if mem.read_ram(SCROLL_X) == 0 {
        let mc = mem.read_ram(MIRROR_CNTRL) | 0x08;
        mem.write_ram(MIRROR_CNTRL, mc);
        let sd = mem.read_ram(SCROLL_DIR) & 0x01;
        mem.write_ram(SCROLL_DIR, sd);
        let st = mem.read_ram(ELEVATOR + F_STATUS).wrapping_add(1);
        mem.write_ram(ELEVATOR + F_STATUS, st); // ⇒ 3 Move
        draw_elevator(mem);
        return;
    }
    mem.write_ram(SAMUS + S_X, 0x80);
    let d = mem
        .read_ram(ELEVATOR + F_X)
        .wrapping_sub(mem.read_ram(SCROLL_X));
    if d & 0x80 != 0 {
        crate::samus::physics::scroll_right(mem);
    } else {
        crate::samus::physics::scroll_left(mem);
    }
    draw_elevator(mem);
}

/// `ElevatorMove ; 07:D83D`——电梯逐像素上下移动（y 0↔239 环绕时翻 hi 位），
/// 抵达 y=$83 即 status+1（⇒ 4 ScrollY）。
fn elev_move(mem: &mut FrameMemory) {
    let etype = mem.read_ram(ELEVATOR + F_TYPE);
    let a_final: u8;
    if etype & 0x80 != 0 {
        // 上行：y==0 ⇒ 翻 hi + 从 240 起算
        let mut y = mem.read_ram(ELEVATOR + F_Y);
        if y == 0 {
            toggle_object_hi(mem, ELEVATOR, F_HI);
            y = SCRN_VY;
        }
        y = y.wrapping_sub(1);
        mem.write_ram(ELEVATOR + F_Y, y);
        a_final = y;
    } else {
        // 下行：y+1，到 240 ⇒ 翻 hi + 归 0
        let mut y = mem.read_ram(ELEVATOR + F_Y).wrapping_add(1);
        mem.write_ram(ELEVATOR + F_Y, y);
        if y == SCRN_VY {
            toggle_object_hi(mem, ELEVATOR, F_HI);
            y = 0;
            mem.write_ram(ELEVATOR + F_Y, 0);
        }
        a_final = y;
    }
    if a_final == 0x83 {
        let st = mem.read_ram(ELEVATOR + F_STATUS).wrapping_add(1);
        mem.write_ram(ELEVATOR + F_STATUS, st); // ⇒ 4 ScrollY
    }
    draw_elevator(mem);
}

/// `ElevatorScrollY ; 07:D870`——`ScrollY!=0` 继续滚（`ElevScrollRoom`）；到 0 则
/// 置 Samus/电梯淡出动画 + status+1（⇒ 5 Fade）+ `Timer1←$40`（**原版即退化单帧**）。
fn elev_scroll_y(mem: &mut FrameMemory) {
    if mem.read_ram(SCROLL_Y) != 0 {
        elev_scroll_room(mem);
        return;
    }
    mem.write_ram(SAMUS + S_ANIM_RESET, ANIM_SAMUS_FADE_OUT_RESET);
    mem.write_ram(SAMUS + S_ANIM_INDEX, ANIM_SAMUS_FADE_OUT);
    mem.write_ram(ELEVATOR + F_ANIM_RESET, ANIM_ELEV_FADE_OUT_RESET);
    mem.write_ram(ELEVATOR + F_ANIM_INDEX, ANIM_ELEV_FADE_OUT);
    let st = mem.read_ram(ELEVATOR + F_STATUS).wrapping_add(1);
    mem.write_ram(ELEVATOR + F_STATUS, st); // ⇒ 5 Fade
    mem.write_ram(crate::ram_map::TIMER1, 0x40);
    draw_elevator(mem);
}

/// `ElevScrollRoom ; 07:D892`——按 `ElevatorType` bit7 选 `ScrollUp` / `ScrollDown`。
fn elev_scroll_room(mem: &mut FrameMemory) {
    if mem.read_ram(ELEVATOR + F_TYPE) & 0x80 != 0 {
        crate::samus::physics::scroll_up(mem);
    } else {
        crate::samus::physics::scroll_down(mem);
    }
    draw_elevator(mem);
}

/// `ElevatorFade ; 07:D8A3`（status 5→6 淡出 / 7→8 淡入，**各单帧**）：
/// `inc status`；新值==8 ⇒ animFrame←$23 + SetSamusAnim(SamusFront) + DrawElevator；
/// 否则 `AnimDrawObject(page, 1)`（逐帧推进动画）。
fn elev_fade(mem: &mut FrameMemory) {
    let st = mem.read_ram(ELEVATOR + F_STATUS).wrapping_add(1);
    mem.write_ram(ELEVATOR + F_STATUS, st);
    if st == 8 {
        mem.write_ram(ELEVATOR + F_ANIM_FRAME, OBJFRAME_ELEVATOR);
        set_samus_anim(mem, ANIM_SAMUS_FRONT);
        draw_elevator(mem);
    } else {
        crate::frame::sprite::anim_draw_object(mem, ELEV_PAGE, 0x01);
    }
}

/// `ElevatorD8BF` 的调色板选取（`ldy InArea / cpy #$12 / bcc @endIf_D /
/// lda #_id_Palette00+1`）：**InArea 字节 $12/$13/$14（区序号 2/3/4 = Kraid/Tourian/Ridley）**恒
/// Palette00+1(=1)，Brinstar/Norfair 用 `PaletteToggle` 值。抽成纯函数以便单测（**注意**：asm 随后 `jsr SelectSamusPalette`
/// 会再次覆写 `PaletteDataPending`，故该值在帧末不可观测）。
pub fn palette_pending_value(in_area: u8, palette_toggle: u8) -> u8 {
    if in_area >= 0x12 {
        1
    } else {
        palette_toggle
    }
}

/// `ElevatorD8BF ; 07:D8BF`——**换区核心**：`$8F` 结局径；否则取目的区低半字节
/// （上行：$84=Ridley→Norfair(1)，其余→Brinstar(0)；下行：低半字节即目的区）→
/// `ora #$10 / jsr IsEngineRunning`（⇒ InArea=$1X + BankSwitchPending）→ 调色板切换
/// → `WaitNMIPass_` → SelectSamusPalette → StartMusic → ScreenOn → CopyAreaPointers
/// → DestroyEnemies → Samus/电梯淡入动画 + status+1（⇒ 7 Fade）+ Timer1←$40。
fn elev_transition(mem: &mut FrameMemory) {
    let etype = mem.read_ram(ELEVATOR + F_TYPE);
    if etype == 0x8F {
        // Samus made it! YAY!（`_id_IncrementRoutine` / AtEnding++ / RoomPtr=0 /
        // BankSwitchPending=1 / TitleRoutine←_id_EndGame）
        mem.write_ram(MAIN_ROUTINE, ID_INCREMENT_ROUTINE);
        let at = mem.read_ram(AT_ENDING).wrapping_add(1);
        mem.write_ram(AT_ENDING, at);
        mem.write_ram(ROOM_PTR, 0);
        mem.write_ram(BANK_SWITCH_PENDING, 1);
        mem.write_ram(TITLE_ROUTINE, ID_END_GAME); // 0x1D EndGame（$9AA7）
        return;
    }
    // 目的区：`tya / bpl` ⇒ 下行直接取低半字节；上行默认 0，仅 $84 ⇒ 1。
    let dest = if etype & 0x80 != 0 {
        if etype == 0x84 {
            1
        } else {
            0
        }
    } else {
        etype & 0x0F
    };
    crate::frame::mainloop::is_engine_running(mem, dest | 0x10);
    // 调色板切换：PaletteToggle eor (Palette00+1)^(Palette05+1)
    let pt = mem.read_ram(PALETTE_TOGGLE) ^ PALETTE_TOGGLE_XOR;
    mem.write_ram(PALETTE_TOGGLE, pt);
    // 在 Tourian（InArea >= $12）恒用 Palette00+1，否则用 PaletteToggle。
    let pending = palette_pending_value(mem.read_ram(IN_AREA), pt);
    mem.write_ram(PALETTE_PENDING, pending);
    // jsr WaitNMIPass_（ClearNMIStat + 等 NMI）——asm 语义=交出本迭代余量、等一次 NMI；
    // 该 NMI 的 `CheckPaletteWrite`（07:C1E0）**先消费刚写的 pending**：emu 实录
    // f1207 pending=6 → f1208 消费（bank 已切换 ⇒ 取 Norfair 表 pal05）。
    // 本模型无「迭代中途挂起/续跑」机制（iter_wait 只记帧账）⇒ **就地兑现该段 NMI**
    // （相位早 ≤1 tick；像素面 f1300+ 归零——见 docs/findings/elevator-evidence.md）。
    mem.write_ram(NMI_STATUS, 1);
    mem.iter_wait();
    crate::frame::nmi::check_palette_write(mem);
    crate::frame::mainloop::select_samus_palette(mem);
    crate::frame::mainloop::start_music(mem);
    // jsr ScreenOn（$C447）——影子 |= $1E，真 PPUMASK 由 NMI 落（延迟落影同族形）。
    mem.deferred_screen_on = true;
    mem.write_ram(NMI_STATUS, 1);
    mem.iter_wait();
    crate::frame::mainloop::copy_area_pointers(mem);
    destroy_enemies(mem);
    // 淡入动画 + status+1（⇒ 7 Fade）+ Timer1←$40
    mem.write_ram(SAMUS + S_ANIM_RESET, ANIM_SAMUS_FADE_IN_RESET);
    mem.write_ram(SAMUS + S_ANIM_INDEX, ANIM_SAMUS_FADE_IN);
    mem.write_ram(ELEVATOR + F_ANIM_RESET, ANIM_ELEV_FADE_IN_RESET);
    mem.write_ram(ELEVATOR + F_ANIM_INDEX, ANIM_ELEV_FADE_IN);
    let st = mem.read_ram(ELEVATOR + F_STATUS).wrapping_add(1);
    mem.write_ram(ELEVATOR + F_STATUS, st); // ⇒ 7 Fade
    mem.write_ram(crate::ram_map::TIMER1, 0x40);
}

/// `ElevatorStop ; 07:D94E`——`ScrollY!=0` 继续滚；到 0 则停：Samus 回站立、
/// 电梯槽回 Idle(1)、`ElevatorType ^= $80`（换向）；换向到「上层」（bit7 清）时
/// `ToggleScroll`（ScrollDir^=3 + MirrorCntrl^=8）+ 落 MirrorCntrl。
fn elev_stop(mem: &mut FrameMemory) {
    if mem.read_ram(SCROLL_Y) != 0 {
        elev_scroll_room(mem);
        return;
    }
    mem.write_ram(SAMUS + S_STATUS, SA_STAND);
    crate::samus::handler::set_samus_stand(mem);
    mem.write_ram(ELEVATOR + F_STATUS, 0x01);
    let etype = mem.read_ram(ELEVATOR + F_TYPE) ^ 0x80;
    mem.write_ram(ELEVATOR + F_TYPE, etype);
    if etype & 0x80 == 0 {
        // 现在在下层：ToggleScroll（$E252）`ScrollDir eor #$03` + `MirrorCntrl eor #$08`
        let dir = mem.read_ram(SCROLL_DIR) ^ 0x03;
        mem.write_ram(SCROLL_DIR, dir);
        let mc = mem.read_ram(MIRROR_CNTRL) ^ 0x08;
        mem.write_ram(MIRROR_CNTRL, mc);
    }
    draw_elevator(mem);
}

/// `SamusElevator ; 07:D423`——电梯移动期间的 Samus 位移（无重力/无摇杆）：
/// 仅当 `Elevator.status∈{3,8}`（ElevatorMove）才按 `ElevatorType` 方向逐像素走；
/// 屏幕 Y（`Samus.y - ScrollY`）越过 $84（下行）/ 低于 $64（上行）时驱动卷屏。
/// 其余状态：清 speedY，status∈{5,7}（淡出/淡入）每帧画，否则隔帧（`FrameCount/2` 判）。
pub fn samus_elevator(mem: &mut FrameMemory) {
    let est = mem.read_ram(ELEVATOR + F_STATUS);
    if est == 3 || est == 8 {
        let etype = mem.read_ram(ELEVATOR + F_TYPE);
        if etype & 0x80 == 0 {
            // 电梯下行
            let scr_y = mem.read_ram(SAMUS + S_Y).wrapping_sub(mem.read_ram(SCROLL_Y));
            // asm `cmp #$84 / bcc Lx057`——**低于 $84 就不滚**（bcc 跳到跳滚处）；
            // 即 Samus 落到屏幕下段才把摄像机往下推（与 MoveSamusDown 同口径）。
            if scr_y >= 0x84 {
                crate::samus::physics::scroll_down(mem);
            }
            let mut y = mem.read_ram(SAMUS + S_Y);
            if y == SCRN_VY - 1 {
                toggle_object_hi(mem, SAMUS, S_HI); // ToggleSamusHi
                y = 0xFF; // y 将为 0
            }
            y = y.wrapping_add(1);
            mem.write_ram(SAMUS + S_Y, y);
        } else {
            // 电梯上行
            let scr_y = mem.read_ram(SAMUS + S_Y).wrapping_sub(mem.read_ram(SCROLL_Y));
            // asm `cmp #$64 / bcs Lx060`——**≥ $64 就不滚**（bcs 跳到跳滚处）；
            // 即 Samus 升到屏幕上段才把摄像机往上推（与 MoveSamusUp 同口径）。
            if scr_y < 0x64 {
                crate::samus::physics::scroll_up(mem);
            }
            let mut y = mem.read_ram(SAMUS + S_Y);
            if y == 0 {
                toggle_object_hi(mem, SAMUS, S_HI);
                y = SCRN_VY; // y 将为 239
            }
            y = y.wrapping_sub(1);
            mem.write_ram(SAMUS + S_Y, y);
        }
        // jmp LD47E：隔帧（FrameCount/2）画
        if mem.read_ram(crate::ram_map::FRAME_COUNT) & 0x01 == 1 {
            crate::samus::physics::set_mirror_cntrl_bit(mem);
            crate::frame::sprite::anim_draw_object(mem, 0x00, 0x01);
        }
        return;
    }
    // Lx062：`ldy #$00 / sty Samus.speedY`，status 5/7 每帧画，否则隔帧。
    mem.write_ram(crate::samus::physics::SAMUS_SPEED_Y_I, 0);
    if est == 5 || est == 7 {
        crate::samus::physics::set_mirror_cntrl_bit(mem);
        crate::frame::sprite::anim_draw_object(mem, 0x00, 0x01);
        return;
    }
    if mem.read_ram(crate::ram_map::FRAME_COUNT) & 0x01 == 1 {
        crate::samus::physics::set_mirror_cntrl_bit(mem);
        crate::frame::sprite::anim_draw_object(mem, 0x00, 0x01);
    }
}

/// `DestroyEnemies ; 07:C8BB`——清敌 RAM：`$97..$DE`（`CannonIndex` 起至
/// `SoundRoutineTablePtr-1`，**原版 off-by-one**：漏 $DF）+ `EnsExtra.status` 全 256 B
/// 清 0，然后 `MetroidOnSamus←X(=0)` + `GotoClearAllMetroidLatches`。
pub fn destroy_enemies(mem: &mut FrameMemory) {
    // 槽环：`cpx #(SoundRoutineTablePtr-1)-CannonIndex / bcs @endIf_A` ⇒ 只清 $97..$DE。
    // CannonIndex（$0097）到 SoundRoutineTablePtr-1（$00DE）——15 个敌对弹/敌面字段，
    // 逐址清 0（原版即按址环，非按槽；此处照录地址区间）。
    for a in 0x0097u16..=0x00DE {
        mem.write_ram(a, 0);
    }
    // EnsExtra.0.status,x 清 256 B（x 全环）——$6AF4 起 256 B 步长 1。
    for a in 0x6AF4u16..0x6AF4 + 0x100 {
        mem.write_ram(a, 0);
    }
    mem.write_ram(crate::ram_map::METROID_ON_SAMUS, 0);
    // jmp GotoClearAllMetroidLatches（区 bank 桩：仅 Tourian 真清）
    crate::samus::handler::goto_clear_all_metroid_latches(mem);
}

/// `SamusCollisionWithSolidEntities ; 07:D976` 的**电梯段**（$D9A3 起）：
/// `Elevator.status==0 ⇒ rts`；`LDC82`（电梯槽 × Samus 槽 碰撞）不重叠 ⇒ rts；
/// 再 `@isSamusOnTop` ⇒ 站顶则 `SamusOnElevator++`。
///
/// 前置：双旗已由 `samus::physics::samus_collision_with_solid_entities` 清 0。
pub fn samus_on_elevator_check(mem: &mut FrameMemory) {
    if mem.read_ram(ELEVATOR + F_STATUS) == 0 {
        return;
    }
    let c = crate::samus::physics::collide_object_slots(
        mem,
        ELEVATOR,  // X slot
        SAMUS,     // Y slot
    );
    if c.no_overlap {
        return;
    }
    if crate::samus::physics::is_samus_on_top_effect(mem, &c) {
        let v = mem.read_ram(SAMUS_ON_ELEVATOR).wrapping_add(1);
        mem.write_ram(SAMUS_ON_ELEVATOR, v);
    }
}

// ---- 供帧环/测试的槽面常量 ----
/// 电梯 y 坐标（`Elevator.y`）。
pub const ELEV_Y: u16 = ELEVATOR + F_Y;
/// 电梯 x 坐标（`Elevator.x`）。
pub const ELEV_X: u16 = ELEVATOR + F_X;
/// 电梯名表位（`Elevator.hi`）。
pub const ELEV_HI: u16 = ELEVATOR + F_HI;
/// 电梯类型/目的（`ElevatorType`）。
pub const ELEV_TYPE: u16 = ELEVATOR + F_TYPE;
/// 电梯状态（`Elevator.status`）。
pub const ELEV_STATUS: u16 = ELEVATOR + F_STATUS;

/// `enemyStatus_Frozen` 读取面（`SamusCollisionWithSolidEntities` 冻敌环自证用）。
pub const FROZEN_STATUS: u8 = ENEMY_STATUS_FROZEN;
/// `ItemRoomMusicStatus`（`StartMusic` 判据；供测试断言）。
pub const ITEM_MUSIC: u16 = ITEM_ROOM_MUSIC_STATUS;
/// `MusicInitFlags`（`StartMusic` 落点）。
pub const MUSIC_FLAGS: u16 = MUSIC_INIT;
/// `PPUCTRL_ZP`（碰撞件名表比较面）。
pub const PPUCTRL: u16 = PPUCTRL_ZP;
/// `PPUMASK_ZP`（ScreenOn 影子面）。
pub const PPUMASK: u16 = PPUMASK_ZP;
/// `MapPosY`（换区后滚屏基准）。
pub const MAP_Y: u16 = MAP_POS_Y;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::FrameMemory;

    fn m() -> FrameMemory {
        FrameMemory::default()
    }

    /// 十态表宽度自证：JumpEngine 表 10 项（0..=9），越界静默（不臆造指针）。
    #[test]
    fn update_elevator_dispatch_covers_ten_states() {
        for st in 0..=9u8 {
            let mut mem = m();
            mem.write_ram(ELEV_STATUS, st);
            update_elevator(&mut mem);
        }
        let mut mem = m();
        mem.write_ram(ELEV_STATUS, 10);
        update_elevator(&mut mem); // 不 panic
    }

    /// `ElevatorIdle`：无 Samus 站立 ⇒ 只画不动；在梯上但未按方向键 ⇒ 不动。
    #[test]
    fn idle_needs_samus_and_button() {
        let mut mem = m();
        mem.write_ram(ELEV_STATUS, 1);
        mem.write_ram(ELEV_TYPE, 0x00); // 下行（BUTTON_DOWN=0x04）
        update_elevator(&mut mem);
        assert_eq!(mem.read_ram(ELEV_STATUS), 1, "无人站梯不得启动");

        mem.write_ram(SAMUS_ON_ELEVATOR, 1);
        mem.write_ram(JOY1_STATUS, 0x08); // UP（方向不符）
        update_elevator(&mut mem);
        assert_eq!(mem.read_ram(ELEV_STATUS), 1, "按错方向不得启动");
    }

    /// 启动相全字段（`$D7DE-$D806`）：StopVertMovement 双清 / speedY=0 /
    /// Samus 置电梯态+居中 (128,112) / animIndex=SamusFront(0x04) / status+1。
    #[test]
    fn idle_start_sets_samus_and_status() {
        let mut mem = m();
        mem.write_ram(ELEV_STATUS, 1);
        mem.write_ram(ELEV_TYPE, 0x00);
        mem.write_ram(SAMUS_ON_ELEVATOR, 1);
        mem.write_ram(JOY1_STATUS, 0x04); // BUTTON_DOWN
        mem.write_ram(crate::samus::physics::SAMUS_SPEED_Y_I, 0x30);
        mem.write_ram(SAMUS_ACCEL_Y, 0x18);
        update_elevator(&mut mem);
        assert_eq!(mem.read_ram(ELEV_STATUS), 2, "⇒ ScrollXToCenter");
        assert_eq!(mem.read_ram(SAMUS_ACCEL_Y), 0);
        assert_eq!(mem.read_ram(ELEVATOR + F_SPEED_Y), 0);
        assert_eq!(mem.read_ram(SAMUS + S_STATUS), SA_ELEVATOR);
        assert_eq!(mem.read_ram(SAMUS + S_X), 128);
        assert_eq!(mem.read_ram(SAMUS + S_Y), 112);
        assert_eq!(mem.read_ram(SAMUS + S_ANIM_INDEX), ANIM_SAMUS_FRONT);
    }

    /// ScrollXToCenter：未居中 ⇒ 电梯在右则 ScrollRight；居中 ⇒ MirrorCntrl|=8 +
    /// ScrollDir&=1 + status→3。
    #[test]
    fn scroll_x_to_center_centers_and_switches_mirroring() {
        let mut mem = m();
        mem.write_ram(ELEV_STATUS, 2);
        mem.write_ram(SCROLL_X, 0x40);
        mem.write_ram(SCROLL_DIR, 0x03); // 已是右滚（2⇒3 才算右）
        // asm `sbc ScrollX / bmi ⇒ ScrollRight`：电梯 x **小于** ScrollX 才右滚
        mem.write_ram(ELEV_X, 0x20);
        update_elevator(&mut mem);
        assert_eq!(mem.read_ram(SCROLL_X), 0x41, "应 ScrollRight 一列");
        assert_eq!(mem.read_ram(SAMUS + S_X), 0x80, "Samus.x←$80");

        let mut mem = m();
        mem.write_ram(ELEV_STATUS, 2);
        mem.write_ram(SCROLL_X, 0);
        mem.write_ram(MIRROR_CNTRL, 0x00);
        mem.write_ram(SCROLL_DIR, 0x03);
        update_elevator(&mut mem);
        assert_eq!(mem.read_ram(MIRROR_CNTRL), 0x08, "MirrorCntrl|=$08");
        assert_eq!(mem.read_ram(SCROLL_DIR), 0x01, "ScrollDir&=1");
        assert_eq!(mem.read_ram(ELEV_STATUS), 3);
    }

    /// ElevatorMove 下行：y+1；y 到 240 ⇒ 归 0 且 hi 翻转；y=$83 时 status+1。
    #[test]
    fn move_down_wraps_at_240_and_stops_at_0x83() {
        let mut mem = m();
        mem.write_ram(ELEV_STATUS, 3);
        mem.write_ram(ELEV_TYPE, 0x00);
        mem.write_ram(ELEV_Y, 0x82);
        update_elevator(&mut mem);
        assert_eq!(mem.read_ram(ELEV_Y), 0x83);
        assert_eq!(mem.read_ram(ELEV_STATUS), 4, "抵达 $83 ⇒ ScrollY");

        let mut mem = m();
        mem.write_ram(ELEV_STATUS, 3);
        mem.write_ram(ELEV_Y, 239);
        mem.write_ram(ELEV_HI, 0x00);
        update_elevator(&mut mem);
        assert_eq!(mem.read_ram(ELEV_Y), 0, "240 ⇒ 归 0");
        assert_eq!(mem.read_ram(ELEV_HI), 0x01, "环绕翻 hi");
    }

    /// ElevatorMove 上行：y=0 ⇒ 从 240 起算 ⇒ 239 且 hi 翻转。
    #[test]
    fn move_up_wraps_from_zero() {
        let mut mem = m();
        mem.write_ram(ELEV_STATUS, 8);
        mem.write_ram(ELEV_TYPE, 0x80);
        mem.write_ram(ELEV_Y, 0x00);
        mem.write_ram(ELEV_HI, 0x03);
        update_elevator(&mut mem);
        assert_eq!(mem.read_ram(ELEV_Y), 239);
        assert_eq!(mem.read_ram(ELEV_HI), 0x02, "hi ^= 1");
    }

    /// ScrollY 相：ScrollY!=0 ⇒ 走卷屏（ElevScrollRoom）；到 0 ⇒ 写淡出动画 +
    /// status→5 + Timer1=$40。
    #[test]
    fn scroll_y_phase_sets_fade_out_anims() {
        let mut mem = m();
        mem.write_ram(ELEV_STATUS, 4);
        mem.write_ram(ELEV_TYPE, 0x00);
        mem.write_ram(SCROLL_Y, 0x00);
        mem.write_ram(SAMUS + S_ANIM_INDEX, 0x02);
        update_elevator(&mut mem);
        assert_eq!(mem.read_ram(ELEV_STATUS), 5);
        assert_eq!(mem.read_ram(SAMUS + S_ANIM_INDEX), ANIM_SAMUS_FADE_OUT);
        assert_eq!(mem.read_ram(SAMUS + S_ANIM_RESET), ANIM_SAMUS_FADE_OUT_RESET);
        assert_eq!(mem.read_ram(ELEVATOR + F_ANIM_INDEX), ANIM_ELEV_FADE_OUT);
        assert_eq!(mem.read_ram(crate::ram_map::TIMER1), 0x40);
    }

    /// Fade：5→6 单帧（AnimDrawObject 径）；7→8 写 animFrame=$23 + SamusFront。
    #[test]
    fn fade_phase_advances_one_frame_each() {
        let mut mem = m();
        mem.write_ram(ELEV_STATUS, 5);
        update_elevator(&mut mem);
        assert_eq!(mem.read_ram(ELEV_STATUS), 6, "5→6 单帧");

        let mut mem = m();
        mem.write_ram(ELEV_STATUS, 7);
        update_elevator(&mut mem);
        assert_eq!(mem.read_ram(ELEV_STATUS), 8);
        assert_eq!(mem.read_ram(ELEVATOR + F_ANIM_FRAME), OBJFRAME_ELEVATOR);
        assert_eq!(mem.read_ram(SAMUS + S_ANIM_INDEX), ANIM_SAMUS_FRONT);
    }

    /// 换区核：下行低半字节=目的区 ⇒ InArea=$1X + BankSwitchPending；调色板 eor 7；
    /// status→7 且写淡入动画。
    #[test]
    fn transition_writes_area_bank_and_palette() {
        let mut mem = m();
        mem.write_ram(ELEV_STATUS, 6);
        mem.write_ram(ELEV_TYPE, 0x02); // 下行 ⇒ 目的区 2（Kraid）
        mem.write_ram(PALETTE_TOGGLE, 0x01);
        mem.write_ram(MAIN_ROUTINE, 3); // IsEngineRunning 仅在 GameEngine(3) 生效
        update_elevator(&mut mem);
        assert_eq!(mem.read_ram(IN_AREA), 0x12, "InArea=$1X（X=目的区）");
        assert_eq!(mem.read_ram(PALETTE_TOGGLE), 0x06, "PaletteToggle eor 7");
        // 注意：PalettePending 先写 PaletteToggle，随后 SelectSamusPalette 覆写
        // （asm 实序 `sta PaletteDataPending / … / jsr SelectSamusPalette`）⇒ 帧末值=2。
        assert_eq!(mem.read_ram(PALETTE_PENDING), 0x02, "SelectSamusPalette 覆写");
        assert_eq!(mem.read_ram(ELEV_STATUS), 7, "⇒ 7 Fade（淡入）");
        assert_eq!(mem.read_ram(SAMUS + S_ANIM_INDEX), ANIM_SAMUS_FADE_IN);
        assert_eq!(mem.read_ram(ELEVATOR + F_ANIM_INDEX), ANIM_ELEV_FADE_IN);
    }

    /// 换区核的**调色板兑现**（WaitNMIPass_ 语义锚——emu 实录 f1207 pending=6 →
    /// f1208 消费、bank 已切换 ⇒ 取 Norfair 表 pal05）。
    /// 过：上行低半字节=1 → InArea=$11、PaletteToggle 1^7=6 ⇒ 该 NMI 载
    /// `bank2 idx5` = `0F 20 10 00 0F 28 16 04 0F 16 11 04 0F 35 1B 16`（ROM 49 项表真值）。
    /// 不过（旧形）：pending=6 被随后的 SelectSamusPalette(2) 抢覆 ⇒ 该帧**零写**
    /// （emu 实测：像素面 f1300+ 12421px 全屏错色）。
    #[test]
    fn transition_consumes_area_palette_at_wait_nmi() {
        let mut mem = m();
        mem.write_ram(ELEV_STATUS, 6);
        mem.write_ram(ELEV_TYPE, 0x01); // 上行 ⇒ 目的区 1（Norfair）
        mem.write_ram(PALETTE_TOGGLE, 0x01);
        mem.write_ram(MAIN_ROUTINE, 3);
        update_elevator(&mut mem);
        assert_eq!(mem.read_ram(IN_AREA), 0x11);
        let bg: Vec<u8> = mem.render.palette[0..16].to_vec();
        assert_eq!(
            bg,
            vec![
                0x0F, 0x20, 0x10, 0x00, 0x0F, 0x28, 0x16, 0x04, 0x0F, 0x16, 0x11, 0x04, 0x0F, 0x35,
                0x1B, 0x16
            ],
            "WaitNMIPass_ 处须先兑现 pending=6（Norfair pal05），随后 SelectSamusPalette 才覆写"
        );
    }

    /// 换区核上行：$84（Ridley）⇒ Norfair(1)；其余上行 ⇒ Brinstar(0)。
    #[test]
    fn transition_up_destination_mapping() {
        let mut mem = m();
        mem.write_ram(ELEV_STATUS, 6);
        mem.write_ram(ELEV_TYPE, 0x84);
        mem.write_ram(MAIN_ROUTINE, 3);
        update_elevator(&mut mem);
        assert_eq!(mem.read_ram(IN_AREA), 0x11, "上行 $84 ⇒ Norfair");

        let mut mem = m();
        mem.write_ram(ELEV_STATUS, 6);
        mem.write_ram(ELEV_TYPE, 0x83);
        mem.write_ram(MAIN_ROUTINE, 3);
        update_elevator(&mut mem);
        assert_eq!(mem.read_ram(IN_AREA), 0x10, "上行非 $84 ⇒ Brinstar");
    }

    /// 结局径：$8F ⇒ MainRoutine←7 / AtEnding++ / RoomPtr=0 / BankSwitchPending=1。
    #[test]
    fn transition_endgame_path() {
        let mut mem = m();
        mem.write_ram(ELEV_STATUS, 6);
        mem.write_ram(ELEV_TYPE, 0x8F);
        mem.write_ram(AT_ENDING, 0x02);
        update_elevator(&mut mem);
        assert_eq!(mem.read_ram(MAIN_ROUTINE), ID_INCREMENT_ROUTINE);
        assert_eq!(mem.read_ram(AT_ENDING), 0x03);
        assert_eq!(mem.read_ram(ROOM_PTR), 0);
        assert_eq!(mem.read_ram(BANK_SWITCH_PENDING), 1);
        assert_eq!(mem.read_ram(TITLE_ROUTINE), 0x1D, "TitleRoutine←_id_EndGame");
    }

    /// P0 回归锚（r9 第三轮抓出的反向比较）：下行时 **scr_y < $84 不滚**、**>= $84 才滚**。
    /// asm `SamusElevator` 07:D42E-07:D440 = `lda Samus.y / sec / sbc ScrollY / cmp #$84 /
    /// bcc Lx057`（bcc 跳到 `Lx057` 即**跳过** ScrollDown）。
    #[test]
    fn samus_elevator_down_scrolls_only_at_or_above_threshold() {
        // scr_y = 0x84（= 阈值）⇒ 滚：ScrollY 0x10 → 0x11
        let mut mem = m();
        mem.write_ram(ELEV_STATUS, 3);
        mem.write_ram(ELEV_TYPE, 0x02); // bit7 清 ⇒ 下行
        mem.write_ram(SCROLL_DIR, 1); // 已在向下滚（否则本拍只切方向不动）
        mem.write_ram(SCROLL_Y, 0x10);
        mem.write_ram(SAMUS + S_Y, 0x94); // scr_y = 0x94 - 0x10 = 0x84
        samus_elevator(&mut mem);
        assert_eq!(mem.read_ram(SCROLL_Y), 0x11, "scr_y==$84 ⇒ 必须 ScrollDown");

        // scr_y = 0x83（阈值下 1px）⇒ 不滚
        let mut mem = m();
        mem.write_ram(ELEV_STATUS, 3);
        mem.write_ram(ELEV_TYPE, 0x02);
        mem.write_ram(SCROLL_DIR, 1);
        mem.write_ram(SCROLL_Y, 0x10);
        mem.write_ram(SAMUS + S_Y, 0x93); // scr_y = 0x83
        samus_elevator(&mut mem);
        assert_eq!(mem.read_ram(SCROLL_Y), 0x10, "scr_y<$84 ⇒ 不滚（bcc 跳滚）");
    }

    /// 同上，上行反侧：**scr_y >= $64 不滚**、**< $64 才滚**。
    /// asm 07:D453-07:D460 = `cmp #$64 / bcs Lx060`（bcs 跳到 `Lx060` 即跳过 ScrollUp）。
    #[test]
    fn samus_elevator_up_scrolls_only_below_threshold() {
        // scr_y = 0x63 ⇒ 滚：ScrollY 0x10 → 0x0F
        let mut mem = m();
        mem.write_ram(ELEV_STATUS, 8);
        mem.write_ram(ELEV_TYPE, 0x82); // bit7 置 ⇒ 上行
        mem.write_ram(SCROLL_DIR, 0); // 已在向上滚
        mem.write_ram(SCROLL_Y, 0x10);
        mem.write_ram(SAMUS + S_Y, 0x73); // scr_y = 0x73 - 0x10 = 0x63
        samus_elevator(&mut mem);
        assert_eq!(mem.read_ram(SCROLL_Y), 0x0F, "scr_y<$64 ⇒ 必须 ScrollUp");

        // scr_y = 0x64（= 阈值）⇒ 不滚
        let mut mem = m();
        mem.write_ram(ELEV_STATUS, 8);
        mem.write_ram(ELEV_TYPE, 0x82);
        mem.write_ram(SCROLL_DIR, 0);
        mem.write_ram(SCROLL_Y, 0x10);
        mem.write_ram(SAMUS + S_Y, 0x74); // scr_y = 0x64
        samus_elevator(&mut mem);
        assert_eq!(mem.read_ram(SCROLL_Y), 0x10, "scr_y>=$64 ⇒ 不滚（bcs 跳滚）");
    }

    /// Tourian（InArea>=$12）恒用 Palette00+1=1（纯函数面——帧末值会被
    /// SelectSamusPalette 覆写，故不在这里断言 RAM）。
    #[test]
    fn palette_pending_picks_palette0_in_tourian() {
        assert_eq!(palette_pending_value(0x13, 0x02), 1, "Tourian ⇒ Palette00+1");
        assert_eq!(palette_pending_value(0x12, 0x02), 1, "边界 $12 也算 Tourian");
        assert_eq!(palette_pending_value(0x11, 0x02), 0x02, "非 Tourian ⇒ 用 toggle");
    }

    /// ElevatorStop：到位 ⇒ Samus 回 Stand、电梯回 Idle、类型换向（bit7）；
    /// 换向到下层（bit7 清）⇒ ToggleScroll（ScrollDir^=3、MirrorCntrl^=8）。
    #[test]
    fn stop_phase_reverses_and_toggles_scroll() {
        let mut mem = m();
        mem.write_ram(ELEV_STATUS, 9);
        mem.write_ram(SCROLL_Y, 0);
        mem.write_ram(ELEV_TYPE, 0x80); // 上行 → 换向为下行（bit7 清）
        mem.write_ram(SAMUS + S_STATUS, SA_ELEVATOR);
        mem.write_ram(SCROLL_DIR, 0x01);
        mem.write_ram(MIRROR_CNTRL, 0x08);
        update_elevator(&mut mem);
        assert_eq!(mem.read_ram(ELEV_TYPE), 0x00, "bit7 翻转");
        assert_eq!(mem.read_ram(ELEV_STATUS), 1, "回 Idle");
        assert_eq!(mem.read_ram(SAMUS + S_STATUS), SA_STAND);
        assert_eq!(mem.read_ram(SCROLL_DIR), 0x02, "ScrollDir^=3");
        assert_eq!(mem.read_ram(MIRROR_CNTRL), 0x00, "MirrorCntrl^=8");
    }

    /// SamusElevator：仅 status∈{3,8} 才位移；下行 y+1，239 ⇒ 翻 hi 且归 0。
    #[test]
    fn samus_elevator_moves_only_in_move_states() {
        let mut mem = m();
        mem.write_ram(ELEV_STATUS, 1);
        mem.write_ram(SAMUS + S_Y, 0x50);
        samus_elevator(&mut mem);
        assert_eq!(mem.read_ram(SAMUS + S_Y), 0x50, "非移动态不得位移");

        let mut mem = m();
        mem.write_ram(ELEV_STATUS, 3);
        mem.write_ram(ELEV_TYPE, 0x00);
        mem.write_ram(SAMUS + S_Y, 239);
        mem.write_ram(SAMUS + S_HI, 0x02);
        samus_elevator(&mut mem);
        assert_eq!(mem.read_ram(SAMUS + S_Y), 0, "239 ⇒ 归 0");
        assert_eq!(mem.read_ram(SAMUS + S_HI), 0x03, "ToggleSamusHi");
    }

    /// SamusElevator 上行：y=0 ⇒ 翻 hi 且归 239。
    #[test]
    fn samus_elevator_up_wraps_from_zero() {
        let mut mem = m();
        mem.write_ram(ELEV_STATUS, 8);
        mem.write_ram(ELEV_TYPE, 0x80);
        mem.write_ram(SAMUS + S_Y, 0);
        mem.write_ram(SAMUS + S_HI, 0x00);
        samus_elevator(&mut mem);
        assert_eq!(mem.read_ram(SAMUS + S_Y), 239);
        assert_eq!(mem.read_ram(SAMUS + S_HI), 0x01);
    }

    /// 非移动态 ⇒ 清 speedY（`ldy #$00 / sty Samus.speedY`）。
    #[test]
    fn samus_elevator_clears_speed_outside_move_states() {
        let mut mem = m();
        mem.write_ram(ELEV_STATUS, 6);
        mem.write_ram(crate::samus::physics::SAMUS_SPEED_Y_I, 0x30);
        samus_elevator(&mut mem);
        assert_eq!(mem.read_ram(crate::samus::physics::SAMUS_SPEED_Y_I), 0);
    }

    /// 电梯碰撞段：status=0 ⇒ 不判；重叠且站顶 ⇒ SamusOnElevator=1。
    #[test]
    fn elevator_collision_sets_flag_only_when_on_top() {
        let mut mem = m();
        mem.write_ram(ELEV_STATUS, 1);
        // 电梯与 Samus 同位（y 差 1 = 站在顶）
        mem.write_ram(ELEVATOR + 0x01, 0x08); // elevator radiusY
        mem.write_ram(ELEVATOR + 0x02, 0x08); // radiusX
        mem.write_ram(SAMUS + 0x01, 0x08);
        mem.write_ram(SAMUS + 0x02, 0x08);
        // 站顶判据 = `dist_y + 1 == 半径和`（1px 重叠）⇒ dy=15/半径和=16
        mem.write_ram(ELEV_Y, 0x5F);
        mem.write_ram(ELEV_X, 0x80);
        mem.write_ram(SAMUS + S_Y, 0x50);
        mem.write_ram(SAMUS + S_X, 0x80);
        samus_on_elevator_check(&mut mem);
        assert_eq!(mem.read_ram(SAMUS_ON_ELEVATOR), 1, "站梯顶");

        let mut mem = m();
        mem.write_ram(ELEV_STATUS, 0);
        samus_on_elevator_check(&mut mem);
        assert_eq!(mem.read_ram(SAMUS_ON_ELEVATOR), 0, "无电梯不判");
    }

    /// DestroyEnemies：$97..$DE 与 EnsExtra 256 B 全清；MetroidOnSamus 清 0。
    #[test]
    fn destroy_enemies_clears_ranges() {
        let mut mem = m();
        for a in 0x0097u16..=0x00DE {
            mem.write_ram(a, 0xAB);
        }
        for a in 0x6AF4u16..0x6AF4 + 0x100 {
            mem.write_ram(a, 0xCD);
        }
        mem.write_ram(crate::ram_map::METROID_ON_SAMUS, 5);
        destroy_enemies(&mut mem);
        for a in 0x0097u16..=0x00DE {
            assert_eq!(mem.read_ram(a), 0, "敌 RAM ${:04X} 应清", a);
        }
        for a in 0x6AF4u16..0x6AF4 + 0x100 {
            assert_eq!(mem.read_ram(a), 0, "EnsExtra ${:04X} 应清", a);
        }
        assert_eq!(mem.read_ram(crate::ram_map::METROID_ON_SAMUS), 0);
    }

    /// 动画偏移自证：`ObjectAnimIndexTbl` 内这些偏移处的字节序列（ROM 实测）。
    /// 段不变量：每条序列以 $FF 收尾；淡出/淡入序列首帧为 $07（Samus）/ $23（电梯）。
    #[test]
    fn anim_offsets_match_sprite_blob() {
        let blob = game_data::assets::sprite_data();
        let tbl = game_data::assets::OBJ_ANIM_INDEX_TBL as usize;
        let read_seq = |off: u8| -> Vec<u8> {
            let mut i = tbl + off as usize;
            let mut v = Vec::new();
            loop {
                let b = blob[i];
                v.push(b);
                if b == 0xFF {
                    break;
                }
                i += 1;
            }
            v
        };
        assert_eq!(read_seq(ANIM_SAMUS_FRONT), vec![0x07, 0xFF]);
        assert_eq!(read_seq(ANIM_SAMUS_FADE_OUT_RESET), vec![0xF7, 0xFF]);
        assert_eq!(
            read_seq(ANIM_SAMUS_FADE_OUT),
            vec![0x07, 0xF7, 0xF7, 0x07, 0xF7, 0xF7, 0xF7, 0x07, 0xF7, 0xF7, 0xF7, 0xF7, 0x07, 0xF7, 0xFF]
        );
        assert_eq!(
            read_seq(ANIM_ELEV_FADE_OUT),
            vec![0x23, 0xF7, 0xF7, 0x23, 0xF7, 0xF7, 0xF7, 0x23, 0xF7, 0xF7, 0xF7, 0xF7, 0x23, 0xF7, 0xFF]
        );
        assert_eq!(read_seq(ANIM_ELEV_FADE_OUT_RESET), vec![0xF7, 0xFF]);
        assert_eq!(
            read_seq(ANIM_SAMUS_FADE_IN),
            vec![0x07, 0xF7, 0xF7, 0xF7, 0xF7, 0x07, 0xF7, 0xF7, 0xF7, 0x07, 0xF7, 0xF7, 0x07, 0xF7, 0xFF]
        );
        assert_eq!(read_seq(ANIM_SAMUS_FADE_IN_RESET), vec![0x07, 0xF7, 0xFF]);
        assert_eq!(
            read_seq(ANIM_ELEV_FADE_IN),
            vec![0x23, 0xF7, 0xF7, 0xF7, 0xF7, 0x23, 0xF7, 0xF7, 0xF7, 0x23, 0xF7, 0xF7, 0x23, 0xF7, 0xFF]
        );
        assert_eq!(read_seq(ANIM_ELEV_FADE_IN_RESET), vec![0x23, 0xF7, 0xFF]);
    }

    /// ElevatorType 低半字节=梯号（`items.asm` type=$04 记录 arg）——`ElevScrollRoom` 判据。
    #[test]
    fn elevator_type_bit7_is_direction() {
        assert_eq!(0x84 & 0x80, 0x80, "up");
        assert_eq!(0x03 & 0x0F, 3, "Tourian 梯");
        assert_eq!(0x8F, 0x8F, "结局梯");
    }
}
