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
/// 绝对变址**读**（`lda abs,x`/`ldy abs,x`）——跨页 +1 不计（下面的例程路径均不计页交叉）
pub const LDA_ABSX: u32 = 4;
/// 绝对变址**写**（`sta abs,x`）——写入固定 5c
pub const STA_ABSX: u32 = 5;
/// 绝对变址**读-改-写**（`asl abs,x`/`ror abs,x`/`inc abs,x`）——固定 7c
pub const RMW_ABSX: u32 = 7;
/// 绝对变址 Y **读**（`lda abs,y`）
pub const LDA_ABSY: u32 = 4;
/// `asl A`（累加器寻址，2c——与 `asl abs,x` 的 7c 差一个量级）
pub const ASL_A: u32 = 2;
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
    const POST: u32 = ZP + JSR + XP4 + ZP + ZP + LDA_ABSX + IMM; // 35（`lda ATDataTable,x` 读）
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
    const TILES: u32 = 3 * (ZP + INDY + INCDEC_ZP + LDA_ABSX + INDY_ST + IMP + BR_TAKEN)
        + (ZP + INDY + INCDEC_ZP + LDA_ABSX + INDY_ST + IMP + BR); // 111（`ldy TilePosTable,x` 读）
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
        + IMP + IMP + IMM + ZP + IMM + ZP + IMM + ZP; // 55
    const BITPAIR: u32 = IMM + ZP + BR + ZP + IMM + BR; // 14（x 初值 0 径）
    const TABLE: u32 = ZP + IMM + ZP + ZP + LDA_ABSX; // 15
    const CLEAR: u32 = IMM + INDY + INDY_ST; // 13
    const SHIFT: u32 = (IMP + BR + IMP + IMP + BR) * 2; // 20（期望 2 轮，非保证值）
    const SET: u32 = INDY + INDY_ST; // 11
    ADDR + BITPAIR + TABLE + CLEAR + SHIFT + SET
}

/// `DrawMetatile` 全径 = 早退径 − 早退 UpdateAttrib + 全径 UpdateAttrib。
pub const fn model_draw_metatile_full() -> u32 {
    const UA_FAST: u32 = JSR + (ZP + ZP + BR_TAKEN) + RTS; // 21（前缀 + beq 命中 + rts）
    // 全径的前缀仍在（`lda ObjectPalette / cmp RoomPalette` 只换 beq 走向：命中→rts，
    // 不命中→继续算属性字节）。r9 复核 A 面在本批前发现这里漏了前缀 8c。
    const UA_FULL: u32 = JSR + (ZP + ZP + BR) + model_update_attrib_full_body() + RTS; // 148
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
/// 口径（本批按 r9 复核 A 面重算过一遍，旧版把 `sta abs,x`(5)/`asl abs,x`(7)/`asl A`(2)
/// 全按 `lda abs,x`(4) 计，少算约 13c）：
/// - `LoadEnemy`：`jsr GetEnemyData` + `jmp EnemyLoop`（尾跳 3c）
/// - `GetEnemyData: ; 07:EB0C`：`lda (zp),y / and / tax / jsr IsSlotTaken / bne`（槽未占用，
///   不跳）+ `iny / lda (zp),y / jsr GetEnemyType / ldy #$02 / lda (zp),y / jsr LEB4D /
///   pha / pla / lda #$03 / rts`
/// - `IsSlotTaken: ; 07:EB7A`：`lda status,x`（4）+ `beq @RTS`（status==0 ⇒ 跳 3）+ `rts`
/// - `GetEnemyType: ; 07:EB28`：`pha / and #$C0 / sta specialAttribs,x`(5) + `asl A`(2) +
///   `bpl Lx228`（bit6 清 ⇒ 跳 3，跳过 mini-boss 支）+ `pla / and #$3F /
///   sta EnsExtra.0.type,x`(5) + `rts`
/// - `LEB4D: ; 07:EB4D`：18 条自身指令（含 5 个 `sta abs,x`=5、1 个 `ldy abs,x`=4、
///   1 个 `asl abs,x`=7）+ `jsr Amul16` + `jsr GetNameTableAtScrollDir` +
///   `jsr InitEnemyFacingDirectionAxisAndDelay` + `jmp InitEnemyForceSpeed…`
/// - `InitEnemyFacingDirectionAxisAndDelay: ; 07:FB7B`：`jsr LoadTableAt977B` +
///   `ror Ens.0.data05,x`(7) + `lda EnemyInitDelayTbl,y`(4) + `sta Ens.0.delay,x`(5) + `rts`
/// - `LoadTableAt977B: ; $80B0`（areas_common.asm:184）：`ldy type,x`(4) + `lda L977B,y`(4) +
///   `asl A`(2) + `rts`
/// - `InitEnemyForceSpeedTowardsSamusDelayAndHealth: ; 07:F85A`：`ldy type,x`(4) +
///   `lda ForceSpeedTbl,y`(4) + `sta data0D,x`(5) + `lda EnemyHealthTbl,y`(4) +
///   `ldy specialAttribs,x`(4) + `bpl @endIf_A`（normal 敌人 bit7 清 ⇒ 跳 3，跳过 `asl`）+
///   `sta health,x`(5) + `rts`
pub const fn model_load_enemy() -> u32 {
    const AMUL16: u32 = IMP * 4 + RTS; // 14（`asl×4` 累加器 + rts）
    const GET_NAME_TABLE: u32 = ZP + ZP + IMM + RTS; // 14
    const LOAD_TABLE_977B: u32 = LDA_ABSX + LDA_ABSY + ASL_A + RTS; // 16
    const FACING: u32 = JSR + LOAD_TABLE_977B + RMW_ABSX + LDA_ABSY + STA_ABSX + RTS; // 44
    const FORCE_SPEED: u32 = LDA_ABSX + LDA_ABSY + STA_ABSX + LDA_ABSY + LDA_ABSX
        + BR_TAKEN + STA_ABSX + RTS; // 35
    const LEB4D: u32 = IMP + IMM + IMM + STA_ABSX + IMP + JSR + IMM + STA_ABSX + IMM
        + STA_ABSX + IMM + STA_ABSX + JSR + STA_ABSX + LDA_ABSX + RMW_ABSX
        + JSR + JMP_ABS + AMUL16 + GET_NAME_TABLE + FACING + FORCE_SPEED; // 178
    const IS_SLOT: u32 = LDA_ABSX + BR_TAKEN + RTS; // 13
    const GET_TYPE: u32 = PHA + IMM + STA_ABSX + ASL_A + BR_TAKEN + PLA + IMM + STA_ABSX + RTS; // 32
    const GET_ENEMY_DATA: u32 = INDY + IMM + IMP + JSR + BR + IMP + INDY + JSR
        + IMM + INDY + JSR + PHA + PLA + IMM + RTS + IS_SLOT + GET_TYPE + LEB4D; // 281
    JSR + GET_ENEMY_DATA + JMP_ABS // 290（`jmp EnemyLoop` 的 3c 计入本项）
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

/// 残差：`RESIDUAL = legacy − model`（**派生量，不是自由参数**）。
///
/// ## 为什么改叫「残差」而不是「债」（r9 复核 D 面，2026-11-13）
///
/// 上一版叫 `BALANCE` 并把每项写成「旧值含固定补偿 / 旧值加权估计」之类的**归因结论**。
/// 复核指出这是方法论错误：`balance := legacy − model` 是**按构造恒等的残差**，
/// 任何模型误差都会被它原样吸收，于是「残差非 0」既可能是旧估值偏高、也可能是模型少算，
/// 而写成归因结论就等于**把模型缺陷洗成旧值的锅**。实证：复核发现 `MT_FULL` 残差 16c
/// 里有 8c 是我漏算了 `UpdateAttrib` 的全径前缀（本批已修，残差降到 8c）；
/// `ENEMY` 残差 26c 里有 ≥13c 是我把 `sta abs,x`(5)/`asl abs,x`(7)/`asl A`(2) 全按
/// `lda abs,x`(4) 计（本批已修，残差降到 10c）。
///
/// 所以现在的口径是：
/// - **模��� = asm 逐条算出的上界/常见径值**（可被证伪，见 `model_values_are_pinned`）
/// - **残差 = legacy − model**，只表示「还没解释掉多少」，**不表示成因**
/// - **每一项非 0 残差必须在 `UNEXPLAINED` 里登记一条待查理由**（测试强制数量对齐）
pub mod residual {
    pub const INIT: i32 = super::legacy::INIT as i32 - super::model_init_tables() as i32; // 0
    pub const MT_FAST: i32 = super::legacy::MT_FAST as i32 - super::model_draw_metatile_fast() as i32; // 0
    pub const MT_FULL: i32 = super::legacy::MT_FULL as i32 - super::model_draw_metatile_full() as i32; // 8
    pub const OBJ_SHELL: i32 = super::legacy::OBJ_SHELL as i32 - super::model_draw_object_shell() as i32; // 66
    pub const Y_UNIT: i32 = super::legacy::Y_UNIT as i32 - super::model_draw_object_y_unit() as i32; // 3
    pub const ROW: i32 = super::legacy::ROW as i32 - super::model_draw_struct_row() as i32; // -42
    pub const ENEMY: i32 = super::legacy::ENEMY as i32 - super::model_load_enemy() as i32; // 10
}

/// 未解释残差登记：**每一笔非 0 残差都要有一条待查理由**（`residuals_are_registered` 强校验）。
///
/// 纪律：这里只能写「待查什么」，**禁止写归因结论**。查清后要么修模型（残差随之变化），
/// 要么把 legacy 值改掉并给出 emu 实测证据——两种动作都会让对应条目消失。
pub const UNEXPLAINED: [&str; 5] = [
    "MT_FULL +8：`UpdateAttrib` 全径的 `bvc`/`beq` 走向与平移环轮数依赖具体数据，     模型按最常见径计；需用带人肉 6502 计数的样本钉住",
    "OBJ_SHELL +66：`DrawObject` 壳的旧值 180 明显含 `LEA8D` 段以外的补偿；     需找出旧值是按哪段 asm 数出来的",
    "Y_UNIT +3：`inc WorkPtr+1` 的进位径（每约 4 个 y 单元发生一次）未纳入模型，\
     需确认旧值是按平均计入还是按固定补偿",
    "ROW -42：旧值 30 只算了增量部分，`DrawStructRow` 整段行头解码未计入——     这是**旧估值偏低**，需确认 room.rs 每行只加一次（已核：155 行只加一次）",
    "ENEMY +10：模型 290 与旧 300 的 10c 差额未定位；需按 boss 径/槽占用径分别计数",
];

/// 模型 + 残差 = 上线值（零行为变化保证）。
const fn ship(model: u32, residual: i32) -> u32 {
    let v = model as i64 + residual as i64;
    assert!(v >= 0, "残差不能把成本压成负数");
    v as u32
}

/// 载房固定壳（`setup_room` 的 `ScanForItems`/`DeleteOffscreenRoomSprites` 等）。
///
/// **尚未逐条分解**：这两个例程体量大（`ScanForItems ; 07:ED98` 与
/// `DeleteOffscreenRoomSprites ; 07:EC9B` 各数百行），本批只把房内热循环
/// 转成逐条模型；壳留旧值，下一批再烧。
pub const LOAD_SHELL: u32 = legacy::SHELL;

/// `InitTables` 全成本（FillRoomRAM + 属性表 + 壳）。
pub const LOAD_INIT: u32 = ship(model_init_tables(), residual::INIT);
/// `DrawObject` 壳。
pub const LOAD_OBJ: u32 = ship(model_draw_object_shell(), residual::OBJ_SHELL);
/// `DrawObject` y 位环 每单元。
pub const LOAD_Y_UNIT: u32 = ship(model_draw_object_y_unit(), residual::Y_UNIT);
/// `DrawStructRow` 每行。
pub const LOAD_ROW: u32 = ship(model_draw_struct_row(), residual::ROW);
/// `DrawMetatile` 早退 / 全径。
pub const LOAD_MT_FAST: u32 = ship(model_draw_metatile_fast(), residual::MT_FAST);
pub const LOAD_MT_FULL: u32 = ship(model_draw_metatile_full(), residual::MT_FULL);
/// `LoadEnemy` 族 每敌人。
pub const LOAD_ENEMY: u32 = ship(model_load_enemy(), residual::ENEMY);

#[cfg(test)]
mod tests {
    use super::*;

    /// 模型值与 asm 逐条手算一致（钉住周期表；改周期表/改指令序列都会破这条）。
    #[test]
    fn model_values_are_pinned() {
        assert_eq!(model_fill_room_ram(), 8503, "FillRoomRAM（3 页 × 内环 11c/字节）");
        assert_eq!(model_attrib_fill(), 703, "属性表 64 字节填充");
        assert_eq!(model_init_tables_shell(), 56, "InitTables 壳");
        assert_eq!(model_init_tables(), 9262, "InitTables 合计（== 旧 LOAD_INIT_CYCLES）");
        assert_eq!(model_draw_metatile_fast(), 214, "DrawMetatile 早退径（含页交叉期望补偿）");
        assert_eq!(model_update_attrib_full_body(), 128, "UpdateAttrib 全径 body");
        assert_eq!(model_draw_metatile_full(), 341, "DrawMetatile 全径（含前缀 8c）");
        assert_eq!(model_draw_object_shell(), 114, "DrawObject 壳");
        assert_eq!(model_draw_object_y_unit(), 17, "y 位环单元");
        assert_eq!(model_draw_struct_row(), 72, "DrawStructRow 单行");
        assert_eq!(model_load_enemy(), 290, "LoadEnemy 族（absx 分档后）");
    }

    /// 上线值 == 2026-11-05 旧标定值（**零行为变化**：掉帧窗不变）。
    ///
    /// 本批改动的安全网：任何一处模型/残差误改都会让掉帧窗漂移、破坏 L4 零容差。
    #[test]
    fn shipped_values_match_legacy_calibration() {
        assert_eq!(LOAD_INIT, legacy::INIT);
        assert_eq!(LOAD_OBJ, legacy::OBJ_SHELL);
        assert_eq!(LOAD_Y_UNIT, legacy::Y_UNIT);
        assert_eq!(LOAD_ROW, legacy::ROW);
        assert_eq!(LOAD_MT_FAST, legacy::MT_FAST);
        assert_eq!(LOAD_MT_FULL, legacy::MT_FULL);
        assert_eq!(LOAD_ENEMY, legacy::ENEMY);
    }

    /// **每一笔非 0 残差都必须登记一条待查理由**（r9 复核 D 面整改的机械约束）。
    ///
    /// 这条不能写成 `residual == legacy - model`（那是恒等式，恒真无意义）。
    /// 它约束的是「残差存在时必须有人认领」——新增/变化的残差不登记就红。
    #[test]
    fn residuals_are_registered() {
        let nonzero = [
            residual::INIT,
            residual::MT_FAST,
            residual::MT_FULL,
            residual::OBJ_SHELL,
            residual::Y_UNIT,
            residual::ROW,
            residual::ENEMY,
        ]
        .iter()
        .filter(|r| **r != 0)
        .count();
        assert_eq!(
            nonzero,
            UNEXPLAINED.len(),
            "非 0 残差 {nonzero} 笔，但 UNEXPLAINED 登记了 {} 条——每笔残差都要有人认领",
            UNEXPLAINED.len()
        );
        // 逐条文本必须写明「待查」而不是归因结论（粗检：不得出现「因为」/「说明旧值」这类断言词）
        for r in UNEXPLAINED {
            assert!(
                r.contains("需") || r.contains("未") || r.contains("待"),
                "残差登记必须是待查项而不是归因结论：{r}"
            );
        }
    }

    /// 逐条模型 == 旧值的两族：残差精确为 0（旧估值本来就是逐条数的）。
    #[test]
    fn exact_families_have_zero_residual() {
        assert_eq!(residual::INIT, 0, "InitTables 9262 逐条 == 旧值");
        assert_eq!(residual::MT_FAST, 0, "DrawMetatile 早退 214 逐条 == 旧值");
    }

    /// `ROW` 残差为负 = **旧估值偏低**（模型 72 > 旧值 30）——方向必须可见。
    #[test]
    fn row_residual_is_negative() {
        assert!(residual::ROW < 0, "旧标定 30 < 逐条模型 72");
        assert!(model_draw_struct_row() > legacy::ROW);
    }
}
