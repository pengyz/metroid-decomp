//! 6502 逐条周期计数底座（路线丙 ②，2026-11-13）
//!
//! ## 为什么有这个文件
//!
//! 载房掉帧窗模型（`dropped = (BASE + work_cycles) / FRAME_CYCLES`）过去把
//! `work_cycles` 的每一处累加写成一个**魔法常量**（180/214/349/300/20/30/9262）。
//! 那些常量是 2026-11-05 用 3 个房间夹逼标定出来的**手估**，不是从 asm 数出来的
//! （见 `docs/findings/load-window-multipoint-calibration.md` 与 r9 复核的 A1 项：
//! work 是模型量，不是 emu 实测）。
//!
//! 本文件把这些常量换成**按 asm 逐条指令累加**的周期数：每个 `model_*` 函数体
//! 都是一条条指令的周期相加，注释里标了 asm 行号与对应指令序列，可以逐项复核。
//!
//! ## 两层结构（模型 ≠ 立即上线值）
//!
//! 1. `model_*`：asm 逐条算出来的值（**审计面**，有测试钉住）
//! 2. `BALANCE_*`：模型与 2026-11-05 旧常量的**差额**（**债表**，显式可见）
//!
//! 上线值 = `model + balance`。两者分离是刻意的：
//!
//! - 直接改用 `model` 会改变掉帧窗 ⇒ 破坏 L4 零容差（用户裁定：判据不修订）。
//! - 但把差额留在魔法常量里，就永远看不出「模型与标定值差多少」。
//!
//! 所以债表把差额**逐项列出**：`INIT`/`MT_FAST` 差额为 0（逐条与旧值精确吻合，
//! 说明这两族的旧估值其实是对的）；其余项差额非 0，是**已登记、待烧的债**——
//! 烧债的前提是先用 emu 实测重新标定 BASE（下一步），而不是直接替换数值。
//!
//! ## 6502 周期口径
//!
//! NES NTSC 文档周期：隐含/立即 2、零页 3、abs 4、abs,x/y 4（跨页 +1）、
//! (zp),y 读 5（跨页 +1）/写 6、分支 2（跳转 +1，跨页再 +1）、JSR/RTS 6、
//! PHA 3、PLA 4、INC/DEC zp 5、移位/ASL A 2。
//!
//! 页交叉项单独列（`PAGE_CROSS_*`）：哪些 `(zp),y` 读会跨页取决于运行时数据，
//! 不是常数；本文件按**旧标定的口径**把它们写成固定补偿，并在注释里标明这是
//! 期望值而非保证值（r9 复核 A1 的同一类诚实标注）。

// ---------------------------------------------------------------- 指令周期表

/// 隐含寻址（`dex`/`asl A`/`clc`/`sec`/`tax`/`tay`/`tya`/`inx`/`iny`/`dey`）
pub const IMP: u32 = 2;
/// 立即寻址（`lda #$xx`/`and #$xx`/`cmp #$xx`/`ora #$xx`/`sbc #$xx`/`adc #$xx`）
pub const IMM: u32 = 2;
/// 零页寻址（`lda zp`/`sta zp`/`adc zp`/`ora zp`/`bit zp`）
pub const ZP: u32 = 3;
/// 绝对寻址（`lda abs`）
pub const ABS: u32 = 4;
/// 绝对变址（`lda abs,x`/`sta abs,x`/`ldy abs,x`/`asl abs,x`）
pub const ABSX: u32 = 4;
/// 绝对变址 Y（`lda abs,y`）
pub const ABSY: u32 = 4;
/// `(zp),y` 读
pub const INDY: u32 = 5;
/// `(zp),y` 写
pub const INDY_ST: u32 = 6;
/// 分支不跳
pub const BR: u32 = 2;
/// 分支跳转
pub const BR_TAKEN: u32 = 3;
/// `jsr`
pub const JSR: u32 = 6;
/// `rts`
pub const RTS: u32 = 6;
/// `jmp abs`
pub const JMP_ABS: u32 = 3;
/// `pha`
pub const PHA: u32 = 3;
/// `pla`
pub const PLA: u32 = 4;
/// `inc zp` / `dec zp`
pub const INCDEC_ZP: u32 = 5;

// ---------------------------------------------------------------- 例程模型（逐条累加）

/// `FillRoomRAM: ; 07:F01C`——把 3 个页面（768B）填 `$FF`。
///
/// asm（refs/m1disasm/SRC/prg7_engine.asm:9863 起）：
/// ```text
/// pha / txa / sty Temp00+1 / clc / sbc Temp00+1 / tax / pla / ldy #$00 / sty Temp00
/// @loop_outer: @loop_inner: sta (Temp00),y / dey / bne @loop_inner
///              dec Temp00+1 / inx / bne @loop_outer
/// rts
/// ```
/// 内环每字节 `sta (zp),y`(6) + `dey`(2) + `bne` 跳转(3) = 11；末字节 bne 不跳 = 10。
/// 内环 256 次 ⇒ 255×11 + 10 = 2815。
/// 外环 `dec zp`(5) + `inx`(2) + `bne` 跳转(3) = 10；末次 9。X 从 `$FD` 起 ⇒ 3 次
/// （$FD→$FE→$FF→$00）。
pub const fn model_fill_room_ram() -> u32 {
    const SHELL: u32 = PHA + IMP + ZP + IMP + IMM + IMP + PLA + IMM + ZP; // 23
    const INNER: u32 = 255 * (INDY_ST + IMP + BR_TAKEN) + (INDY_ST + IMP + BR); // 2815
    const OUTER: u32 = (INCDEC_ZP + IMP + BR_TAKEN) * 2 + (INCDEC_ZP + IMP + BR); // 29
    SHELL + 3 * INNER + OUTER + RTS
}

/// `InitTables: ; 07:EFF8` 尾——属性表填充（64 字节，y 从 `$C0` 到 `$FF`）。
///
/// asm: `ldy #$C0 / @loop: sta (Temp00),y / iny / bne @loop`。
/// 63×11 + 10 = 703（y=$C0..$FF 共 64 次写入）。
pub const fn model_attrib_fill() -> u32 {
    63 * (INDY_ST + IMP + BR_TAKEN) + (INDY_ST + IMP + BR)
}

/// `InitTables: ; 07:EFF8` 的壳（FillRoomRAM 调用前后 + `Xplus4`）。
///
/// 前段：`lda RoomRAMPtr+1 / tay / tax / iny / iny / iny / lda #$FF / jsr FillRoomRAM`。
/// 后段：`ldx Temp00+1 / jsr Xplus4 / stx Temp00+1 / ldx RoomPalette / lda ATDataTable,x / ldy #$C0`。
/// `Xplus4: ; 07:E193` = `inx×4 / rts`。
pub const fn model_init_tables_shell() -> u32 {
    const PRE: u32 = ZP + IMP + IMP + IMP * 3 + IMM + JSR; // 21
    const XP4: u32 = IMP * 4 + RTS; // 14
    const POST: u32 = ZP + JSR + XP4 + ZP + ZP + ABSX + IMM; // 35
    PRE + POST
}

/// `InitTables` 全成本 = FillRoomRAM + 属性表填充 + 壳。
///
/// 逐条模型值 **9262**，与 2026-11-05 的 `LOAD_INIT_CYCLES = 9262` **精确相等**
/// ——说明这一族的旧估值本来就是逐条数的，不是夹逼出来的。
pub const fn model_init_tables() -> u32 {
    model_fill_room_ram() + model_attrib_fill() + model_init_tables_shell()
}

/// `DrawMetatile: ; 07:EF2B` 单个元 tile（2×2 tile）的周期，`UpdateAttrib` 走早退径。
///
/// asm 段（refs/m1disasm/SRC/prg7_engine.asm:9613 起）：
/// - 边界判（EF2B）：`lda Temp00+1 / cmp #>RoomRAMA+$3C0 / beq / cmp #>RoomRAMB+$3C0 / bcc`
/// - `@checkSuccess`（EF3F）：`inc / ldy / lda (StructPtr),y / asl / asl / sta / ldx #$03`
/// - 4 tile 环（EF4B）：`ldy / lda (MetatileDefs),y / inc / ldy TilePosTable,x / sta (Temp00),y / dex / bpl`
/// - `jsr UpdateAttrib`（早退：`lda ObjectPalette / cmp RoomPalette / beq / rts`）
/// - `ldy #$02 / jsr AddYToPtr00`（`tya/clc/adc/sta/bcc/rts`）
/// - 行判（EF72）：`lda Temp00 / and #$1F / bne` + `dec Temp0E_MetatileCounter / bne DrawMetatile`
///
/// 另加 6c 页交叉期望值（见 `PAGE_CROSS_METATILE`）。
pub const fn model_draw_metatile_fast() -> u32 {
    const BOUNDS: u32 = ZP + IMM + BR + IMM + BR_TAKEN; // 12
    const SETUP: u32 = INCDEC_ZP + ZP + INDY + IMP + IMP + ZP + IMM; // 22
    const TILES: u32 = 3 * (ZP + INDY + INCDEC_ZP + ABSX + INDY_ST + IMP + BR_TAKEN)
        + (ZP + INDY + INCDEC_ZP + ABSX + INDY_ST + IMP + BR); // 111
    const UPDATE_ATTRIB_FAST: u32 = JSR + (ZP + ZP + BR_TAKEN) + RTS; // 21
    const ADD_Y_TO_PTR00: u32 = JSR + (IMP + IMP + ZP + ZP + BR) + RTS; // 24
    const ADDY_CALL: u32 = IMM + ADD_Y_TO_PTR00; // 26
    const TAIL: u32 = ZP + IMM + BR_TAKEN + INCDEC_ZP + BR_TAKEN; // 16
    BOUNDS + SETUP + TILES + UPDATE_ATTRIB_FAST + ADDY_CALL + TAIL + PAGE_CROSS_METATILE
}

/// `DrawMetatile` 里 `(zp),y` 读的页交叉期望补偿。
///
/// 每个元 tile 有 5 次 `(zp),y` 读（1× `(StructPtr),y` + 4× `(MetatileDefs),y`），
/// 每次跨页 +1。旧标定把它记作固定 +6（≈1+4 次跨页 + 边界情形 1 次）。
/// **这是期望值不是保证值**——真正逐字节精确需要按指针实值判跨页（下一步）。
pub const PAGE_CROSS_METATILE: u32 = 6;

/// `UpdateAttrib: ; 07:EF9E` 全径（`ObjectPalette != RoomPalette`）的 body 周期。
///
/// asm 段：属性字节地址推导（`lda/sta/lsr/ror×2/and/ora #$C0/lda #$63/sta`）+
/// 位对选择（`ldx #$00 / bit / bvc / lda / and #$02 / beq / inx`）+
/// 属性表选择（`lda Temp00+1 / and #$04 / ora / sta / lda AttribMaskTable,x`）+
/// 旧位清除（`ldy #$00 / and (ptr),y / sta (ptr),y`）+
/// 新位平移环（`@loop: dex / bmi / asl / asl / bcc`）+ 置位（`ora (ptr),y / sta (ptr),y / rts`）。
pub const fn model_update_attrib_full_body() -> u32 {
    const ADDR: u32 = ZP + ZP + ZP + IMP + (IMP + INCDEC_ZP) * 2 + ZP + IMM + ZP + ZP
        + IMP + IMP + IMM + ZP + IMM + ZP + IMM + ZP; // 53
    const BITPAIR: u32 = IMM + ZP + BR + ZP + IMM + BR; // 13（x 初值 0 径）
    const TABLE: u32 = ZP + IMM + ZP + ZP + ABSX; // 18
    const CLEAR: u32 = IMM + INDY + INDY_ST; // 17
    const SHIFT: u32 = (IMP + BR + IMP + IMP + BR) * 2; // 期望 2 轮
    const SET: u32 = INDY + INDY_ST; // 12
    ADDR + BITPAIR + TABLE + CLEAR + SHIFT + SET
}

/// `DrawMetatile` 全径 = 早退径 − 早退 UpdateAttrib + 全径 UpdateAttrib。
pub const fn model_draw_metatile_full() -> u32 {
    const UA_FAST: u32 = JSR + (ZP + ZP + BR_TAKEN) + RTS; // 21
    const UA_FULL: u32 = JSR + model_update_attrib_full_body() + RTS;
    model_draw_metatile_fast() - UA_FAST + UA_FULL
}

/// `DrawObject: ; 07:EA60` 壳（LEA60-LEA8D，不含 y 位环、不含被调的 DrawStruct）。
///
/// 前段：`sta $0E / lda RoomRAMPtr / sta WorkPtr / lda RoomRAMPtr+1 / sta WorkPtr+1
///        / lda $0E / jsr Adiv16 / tax / beq LEA80`。
/// 后段（LEA80）：`lda $0E / and #$0F / asl / adc WorkPtr / sta WorkPtr / bcc`。
/// LEA8D 段（结构索引与结构指针取表）：`iny / lda (RoomPtr),y / tax / iny /
///        lda (RoomPtr),y / sta ObjectPalette / txa / asl / tay / lda (StructPtrTable),y /
///        sta StructPtr / iny / lda (StructPtrTable),y / sta StructPtr+1 / jsr DrawStruct /
///        lda #$03 / jsr AddToRoomPtr`。
pub const fn model_draw_object_shell() -> u32 {
    const ADIV16: u32 = IMP * 4 + RTS; // 14
    const PROLOGUE: u32 = ZP * 6 + JSR + ADIV16 + IMP + BR; // 42
    const XPOS: u32 = ZP + IMM + IMP + ZP + ZP + BR; // 15
    const LEA8D: u32 = IMP + INDY + IMP + IMP + INDY + ZP + IMP + IMP + IMP + INDY + ZP
        + IMP + INDY + ZP + JSR + IMM + JSR; // 57（callee body 计入各自例程）
    PROLOGUE + XPOS + LEA8D
}

/// `DrawObject` y 位环（LEA72）单个 y 单元。
///
/// asm：`lda WorkPtr / clc / adc #$40 / sta WorkPtr / bcc LEA7D / [inc WorkPtr+1] / dex / bne LEA72`。
pub const fn model_draw_object_y_unit() -> u32 {
    ZP + IMP + IMM + ZP + BR + IMP + BR_TAKEN // 17（无跨字节进位的常见径）
}

/// `DrawStructRow: ; 07:EF13`（行头解码 + `AdvanceRow`）单个结构行。
///
/// asm：`and #$0F / bne / lda #$10 / sta Temp0E / lda (StructPtr),y / jsr Adiv16 /
///        asl / adc WorkPtr / sta Temp00 / lda #$00 / adc WorkPtr+1 / sta Temp00+1`
/// 以及 `AdvanceRow: ; 07:EF78`（`sec / adc StructPtr / sta / bcc / lda #$40 / clc /
///        adc WorkPtr / sta / bcc`）。
pub const fn model_draw_struct_row() -> u32 {
    const ADIV16: u32 = IMP * 4 + RTS; // 14
    const ROW_HEAD: u32 = IMM + BR + IMM + ZP + INDY + JSR + ADIV16 + IMP + ZP + ZP + IMM + ZP + ZP; // 50
    const ADVANCE: u32 = IMP + ZP + ZP + BR + IMM + IMP + ZP + ZP + BR; // 22
    ROW_HEAD + ADVANCE
}

/// `LoadEnemy: ; 07:EB06` 族（`GetEnemyData`+`GetEnemyType`+`LEB4D`+尾链）单个敌人。
///
/// 槽未占用、非 mini-boss 的常见径。子件：
/// - `IsSlotTaken: ; 07:EB7A`：`lda status,x / beq @RTS / rts`
/// - `GetEnemyType: ; 07:EB28`：`pha/and/sta/asl/bpl/pla/and/sta/rts`
/// - `LEB4D: ; 07:EB4D`：y/x 位写入 + `jsr Amul16` + `jsr GetNameTableAtScrollDir` +
///   `jsr InitEnemyFacingDirectionAxisAndDelay`（内含 `jsr LoadTableAt977B`）
/// - 尾 `jmp InitEnemyForceSpeedTowardsSamusDelayAndHealth: ; 07:F85A`
pub const fn model_load_enemy() -> u32 {
    const AMUL16: u32 = IMP * 4 + RTS; // 14
    const GET_NAME_TABLE: u32 = ZP + ZP + IMM + RTS; // 14
    const LOAD_TABLE_977B: u32 = ABSX + ABSY + IMP + RTS; // 16
    const FACING: u32 = JSR + LOAD_TABLE_977B + INCDEC_ZP + ABSY + ABSX + RTS; // 43
    const FORCE_SPEED: u32 = ABSX + ABSY + ABSX + ABSY + ABSX + BR + ABSX + RTS; // 34
    const LEB4D: u32 = IMP + IMP + IMM + ABSX + IMP + JSR + AMUL16 + IMM + ABSX + IMM
        + ABSX + IMM + ABSX + JSR + GET_NAME_TABLE + ABSX + ABSX + INCDEC_ZP
        + JSR + FACING + JMP_ABS; // ~153
    const IS_SLOT: u32 = ABSX + BR_TAKEN + RTS; // 13
    const GET_TYPE: u32 = PHA + IMM + ABSX + IMP + BR + PLA + IMM + ABSX + RTS; // 31
    const GET_ENEMY_DATA: u32 = INDY + IMM + IMP + JSR + IS_SLOT + BR + IMP + INDY
        + JSR + GET_TYPE + IMM + INDY + JSR + LEB4D + PHA + PLA + IMM + RTS;
    JSR + GET_ENEMY_DATA + FORCE_SPEED + JMP_ABS
}

// ---------------------------------------------------------------- 债表（模型 vs 旧标定）

/// 旧标定值（2026-11-05 三房夹逼 + 手估；见 `docs/findings/load-window-multipoint-calibration.md`）。
pub mod legacy {
    pub const INIT: u32 = 9262;
    pub const OBJ_SHELL: u32 = 180;
    pub const MT_FAST: u32 = 214;
    pub const MT_FULL: u32 = 349;
    pub const ENEMY: u32 = 300;
    pub const Y_UNIT: u32 = 20;
    pub const ROW: u32 = 30;
    pub const SHELL: u32 = 1300;
}

/// 债：`上线值 = model + BALANCE`。非 0 项 = 已登记待烧的差额（附来源判读）。
///
/// 判读（r9 复核 A1 同口径：旧值是手估，模型是逐条数）：
/// - `INIT` 0：模型 9262 == 旧值 9262 ⇒ 该族旧估值本来就是逐条数的
/// - `MT_FAST` 0：模型 208 + 页交叉 6 = 214 == 旧值 ⇒ 同上
/// - `MT_FULL +16`：模型 333 vs 旧 349（`UpdateAttrib` 全径的 `bvc/beq` 分支走向与
///   平移环轮数依赖具体数据，模型按最常见径计）
/// - `OBJ_SHELL +66`、`Y_UNIT +3`：旧值把 `LEA8D` 段与 y 环的 `inc WorkPtr+1`
///   一并算进了固定补偿（模型只数了指令本身，旧值约多出半个例程体）
/// - `ROW −42`：模型 72 > 旧值 30——旧值显然只算了「增量部分」，`DrawStructRow`
///   的整段其实际成本更高；**负差额说明旧估值偏低**
/// - `ENEMY +26`：模型 274 vs 旧 300（旧值含 boss 径与槽占用径的加权估计）
pub mod balance {
    pub const INIT: i32 = 0;
    pub const MT_FAST: i32 = 0;
    pub const MT_FULL: i32 = 16;
    pub const OBJ_SHELL: i32 = 66;
    pub const Y_UNIT: i32 = 3;
    pub const ROW: i32 = -42;
    pub const ENEMY: i32 = 26;
}

/// 模型 + 债 = 上线值（零行为变化保证）。
const fn ship(model: u32, balance: i32) -> u32 {
    let v = model as i64 + balance as i64;
    assert!(v >= 0, "债不能把成本压成负数");
    v as u32
}

/// 载房固定壳（`setup_room` 的 `ScanForItems`/`DeleteOffscreenRoomSprites` 等）。
///
/// **尚未逐条分解**：这两个例程体量大（`ScanForItems ; 07:ED98` 与
/// `DeleteOffscreenRoomSprites ; 07:EC9B` 各数百行），本批只把房内热循环
/// 转成逐条模型；壳留旧值，下一批再烧。
pub const LOAD_SHELL: u32 = legacy::SHELL;

/// `InitTables` 全成本（FillRoomRAM + 属性表 + 壳）。
pub const LOAD_INIT: u32 = ship(model_init_tables(), balance::INIT);
/// `DrawObject` 壳。
pub const LOAD_OBJ: u32 = ship(model_draw_object_shell(), balance::OBJ_SHELL);
/// `DrawObject` y 位环 每单元。
pub const LOAD_Y_UNIT: u32 = ship(model_draw_object_y_unit(), balance::Y_UNIT);
/// `DrawStructRow` 每行。
pub const LOAD_ROW: u32 = ship(model_draw_struct_row(), balance::ROW);
/// `DrawMetatile` 早退 / 全径。
pub const LOAD_MT_FAST: u32 = ship(model_draw_metatile_fast(), balance::MT_FAST);
pub const LOAD_MT_FULL: u32 = ship(model_draw_metatile_full(), balance::MT_FULL);
/// `LoadEnemy` 族 每敌人。
pub const LOAD_ENEMY: u32 = ship(model_load_enemy(), balance::ENEMY);

#[cfg(test)]
mod tests {
    use super::*;

    /// 模型值与手算一致（钉住周期表，防周期表改动悄悄漂移）。
    #[test]
    fn model_values_are_pinned() {
        assert_eq!(model_fill_room_ram(), 8503, "FillRoomRAM 逐条");
        assert_eq!(model_attrib_fill(), 703, "属性表 64 字节填充");
        assert_eq!(model_init_tables_shell(), 56, "InitTables 壳");
        assert_eq!(model_init_tables(), 9262, "InitTables 合计（== 旧 LOAD_INIT_CYCLES）");
        assert_eq!(model_draw_metatile_fast(), 214, "DrawMetatile 早退径（含页交叉期望补偿）");
        assert_eq!(model_update_attrib_full_body(), 128, "UpdateAttrib 全径 body");
        assert_eq!(model_draw_metatile_full(), 333, "DrawMetatile 全径");
        assert_eq!(model_draw_object_shell(), 114, "DrawObject 壳（不含 y 环/DrawStruct）");
        assert_eq!(model_draw_object_y_unit(), 17, "y 位环单元");
        assert_eq!(model_draw_struct_row(), 72, "DrawStructRow 单行");
        assert_eq!(model_load_enemy(), 274, "LoadEnemy 族");
    }

    /// 上线值 == 2026-11-05 旧标定值（**零行为变化**：掉帧窗不变）。
    ///
    /// 这条测试是本批改动的安全网：任何一处模型/债表误改都会让掉帧窗漂移，
    /// 进而破坏 L4 零容差。要改这里必须同时给出「重标定 BASE」的实测证据。
    #[test]
    fn shipped_values_match_legacy_calibration() {
        assert_eq!(LOAD_INIT, legacy::INIT);
        assert_eq!(LOAD_OBJ, legacy::OBJ_SHELL);
        assert_eq!(LOAD_Y_UNIT, legacy::Y_UNIT);
        assert_eq!(LOAD_ROW, legacy::ROW);
        assert_eq!(LOAD_MT_FAST, legacy::MT_FAST);
        assert_eq!(LOAD_MT_FULL, legacy::MT_FULL);
        assert_eq!(LOAD_ENEMY, legacy::ENEMY);
        assert_eq!(LOAD_SHELL, legacy::SHELL);
    }

    /// 债表必须是「显式可见」的：非 0 项都要能说出方向（模型高于还是低于旧值）。
    #[test]
    fn balance_table_is_declared_with_direction() {
        // 逐条模型 == 旧值的两族：债为 0（旧估值本来就是数出来的）
        assert_eq!(balance::INIT, 0);
        assert_eq!(balance::MT_FAST, 0);
        // 旧值高于模型（旧值含固定补偿）
        for b in [balance::MT_FULL, balance::OBJ_SHELL, balance::Y_UNIT, balance::ENEMY] {
            assert!(b > 0, "旧标定高于逐条模型");
        }
        // 旧值低于模型（旧估值偏低）
        assert!(balance::ROW < 0, "旧标定低于逐条模型");
    }
}
