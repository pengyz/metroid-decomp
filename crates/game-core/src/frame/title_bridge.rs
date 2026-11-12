//! 会师接线桥（bank00:8000 穿越）：GoMainRoutine 标题路 ↔ title::dispatch_step 统一入口。
//!
//! **接线形态**（port2 契约三答对表）：桥侧=解码→调用→应用（phase→臂匹配归 title 域）：
//! ①TitleFace 构造（star_src=WRAM $6E00 面/joy=$12/ppuctrl=$FF/timer3/frame_count）
//! ②dispatch_step 统一分发 ③TitleStep 应用（apply_title_step 唯一源）+
//! unported 表址爆（`bank00:<表址>` per-entry 形）+ jumped 转接面（当前=爆——转接件随接）。

use crate::frame::FrameMemory;
use crate::memmap::title_state;
use crate::ram_map::{
    FRAME_COUNT, JOY1_CHANGE, PPUCTRL_ZP, TITLE_ROUTINE,
};

/// TitleRoutinePtrTable 烘焙（00:802A——31 项；JumpEngineRoutinePtr=$0C/$0D 面）。
const TITLE_ROUTINE_PTR_TABLE: [u16; 31] = [
0x8071, 0x80D0, 0x80F9, 0x812C, 0x8142, 0x8109, 0x814D, 0x8163, 0x8182, 0x81D1, 0x806E, 0x806E, 0x822E, 0x8243, 0x8263, 0x8283, 0x8068, 0x82A3, 0x82ED, 0x82F3, 0x82F3, 0x90BA, 0x90D7, 0x911A, 0x9147, 0x9359, 0x9394, 0x90BA, 0x939E, 0x9AA7, 0xC4AA,
];

/// MainTitleRoutine 分发一步（asm 内容锚 `MainTitleRoutine: ; 00:8000`）。
pub fn main_title_routine(mem: &mut FrameMemory) {
    // 【r23 spin 等待段】>0=本 tick 工作零动（跨相续段正交机构——臂的等待段语义）。
    // 【r9 FC 冻结（porter+逐帧会计修订）】spin 消费 tick=asm 例程内部 WaitNMIPass_
    // 等待帧（MainLoop 卡在例程体内⇒迭代未完成⇒FC 不自增）⇒一律置旗（通则——
    // 0x00 臂 f2 冻/转换臂尾旋 f313-318 冻均此理，emu 逐帧实录）；族相位旗
    // （\$00/\$01/\$15/\$1B=arm 体不返回——0x00 系 Step 2 入族：工作拍 FC 冻结，
    // 完成由 iter_span_fc 在 span 尾收口）仍先于 spin 检查置位。
    mem.title_no_return = matches!(
        mem.read_ram(TITLE_ROUTINE), 0x00 | 0x01 | 0x15 | 0x1B);
    if mem.title_spin > 0 {
        mem.title_spin -= 1;
        mem.title_no_return = true;
        return;
    }
    // 【f1190 相位·延拍消费】CheckPassword 链上一拍已置 pending（emu UnscramblePassword
    // 双层滚字节循环忙跨 2 tick——prg0_title.asm:2078）：本拍才执行 Consolidate/Validate/
    // Unscramble→InitializeGame，使 JIB($69B3)/LoadSamusGFX 与 emu 同拍（capture f1190 JIB=0）。
    if mem.check_password_pending {
        mem.check_password_pending = false;
        run_check_password_chain(mem);
        // 变体 B（A/B 定谳 2026-11-12）：不置 title_no_return——emu FC 冻结跨 4 dump 帧，
        // 但 piece④ 前未建模也 match（f1430=0）；置 true 反引入 f1430 169px 回归（NT B 未写）。
        return;
    }
    let mut phase = mem.read_ram(TITLE_ROUTINE);
    let mut star_src = [0u8; 0xA0];
    for (i, b) in star_src.iter_mut().enumerate() {
        *b = mem.read_wram(0x6E00 + i as u16);
    }
    let mut face = crate::title::TitleFace {
        phase,
        cursor: mem.title_cursor,
        a: mem.regs.a,
        x: mem.regs.x,
        y: mem.regs.y,
        timer3: mem.read_ram(title_state::TIMER3),
        frame_count: mem.read_ram(FRAME_COUNT),
        joy_change: mem.read_ram(JOY1_CHANGE),
        ppuctrl: mem.read_ram(PPUCTRL_ZP),
        star_src,
        wram: mem.wram.to_vec(),
        ram: mem.ram.to_vec(),
    };
    // **JumpEngine TempX/TempY 保存**（asm C27C：`sty TempY($6A)/stx TempX($69)`——
    // B 案 $69/$6A=FF/FF 机理：调用点寄存器保存（帧 3 面））
    mem.write_ram(0x0069, mem.regs.x);
    mem.write_ram(0x006A, mem.regs.y);
    // JumpEngine 指针写（asm C27C：`pla/sta JumpEngineRoutinePtr/pla/sta $0D`——
    // 分发目标址入 $0C/$0D（B 案 $0C/$0D=$8071 对形））
    if (phase as usize) < 31 {
        let target = TITLE_ROUTINE_PTR_TABLE[phase as usize];
        mem.write_ram(0x000C, target as u8);
        mem.write_ram(0x000D, (target >> 8) as u8);
    }
    // **MainTitleRoutine 头**（$8000-$8027 分派头——port2 契约：头在 JumpEngine 前跑）：
    // `cmp #$15/bcs`+`lda Joy1Change/and #$10/beq`→hit⇒TitleRoutine←$1B（**0x15/0x1B
    // 共体双入口**——$90BA）+四 spare 清（$D1/$BB/$B7/$B8）+$FF&=$FC（名表 0）。
    let cursor = mem.read_ram(crate::ram_map::TITLE_ROUTINE); // 当前相位（头检面）
    let (head_hit, head_events, head_new) = crate::title::title_tick_head(
        cursor,
        mem.read_ram(crate::memmap::title_state::JOY1_CHANGE),
        mem.read_ram(crate::ram_map::PPUCTRL_ZP),
    );
    if head_hit {
        for (addr, val) in head_events {
            mem.write_ram(addr, val);
        }
        mem.write_ram(crate::ram_map::TITLE_ROUTINE, head_new); // 0x1B 落位
        // 【r9 FC 冻结】命中帧=转换例程体同帧执行（不返回主循环）⇒冻
        // （【复核 R3 更正】emu 行距实录：menu 冻结窗=f311-f313 三行 FC 持平，命中帧
        // 所处行号的采样归因依采样沿惯例而定——以 fcprobe 逐帧 0 分歧为判据）
        mem.title_no_return = true;
        // 【Step 4b dive】asm 头检命中=菜单迭代内 **jmp 直入**转换例程体（同迭代继续
        // ——不完成不 inc）：同拍分派臂 cursor0（face 以新相位重建）；迭代收尾 inc 由
        // 臂末窗 0xFF 哨兵承担（emu f314 FC=50 单 inc 首现行实录——map-v2 §2）。
        mem.title_cursor = 0;
        phase = head_new;
        face = crate::title::TitleFace { phase: head_new, cursor: 0, ..face };
    }
    let step = crate::title::dispatch_step(&face);
    // 【Step 2】0x00 臂重活跨帧（InitializeAfterReset+LoadStarSprites 数万周期——emu f2
    // bogged 单窗，map-v2 §2 N₀=1 实锤）：门队列声明——span tick 无缝行/无 prelude，
    // FC 延至 span 尾收口（iter_span_fc——emu FC=1 首现 f3 实录）。
    if phase == 0x00 {
        mem.iter_bogged(1);
        mem.iter_span_fc = true;
    }
    // 【Step 4a】0x01 段拍后的溢出窗（n₁=1——map-v2 §2 实锤）：工作超帧 NMIStatus=1
    // 残留⇒溢出窗 NMI 内块跳（asm prg7:251 同形）——门队列插在段拍与 0xFF 哨兵间。
    if phase == 0x01 && step.next_cursor == 0xFF {
        mem.iter_bogged(1);
    }
    mem.title_cursor = step.next_cursor; // 跨帧游标存回（>0=spin 续点/0=完成复位）
    mem.title_cursor_phase = phase; // 续段归属相位（同相位续段判定——r14 复核 F4）
    // shell 组合点执行（**全臂**——两层边界：title 域声明/本侧执行；
    // S6-10 渲染腿接线：PrepareVram/ClearNameTables PPU 面随批兑现）。
    for sc in &step.shell {
        exec_shell_call(mem, sc);
    }
    if let Some(addr) = step.unported {
        if step.shell.contains(&crate::title::ShellCall::NmiYield) {
            // **占位臂**（表址供首分歧记账+NmiYield 帧界让出——B 跑分类面，非引爆）
            super::DIVERGENCES.with(|d| d.borrow_mut().push(addr));
            return;
        }
        let loc = format!("bank00:{addr:04X}");
        crate::unported!(&loc); // 真未武装（18/1E——深路专件/随件）：per-entry 爆
    }
    title_state::apply_title_step(&mut mem.ram, &mut mem.wram, &step.events);
    if let Some(a) = step.a_out {
        mem.regs.a = a;
    }
    if let Some(x) = step.x_out {
        mem.regs.x = x;
    }
    if let Some(y) = step.y_out {
        mem.regs.y = y;
    }
    if let Some(addr) = step.jumped {
        if addr == 0x932B {
            // InitializeStats（**asm 内容锚** `InitializeStats: ; 00:932B`——游戏启动
            // 统计面）：13 写清零（SamusStat00/EnergyTankCount/SamusGear/MissileCount/
            // MaxMissiles/KraidStatueStatus/RidleyStatueStatus/SamusAge×4/AtEnding/
            // JustInBailey）+`BankSwitchPending($28)←2`（Brinstar 页切换）。
            for a in [
                crate::ram_map::SAMUS_STAT00, crate::ram_map::ENERGY_TANK_COUNT,
                crate::ram_map::SAMUS_GEAR, crate::ram_map::MISSILE_COUNT,
                crate::ram_map::MAX_MISSILES, 0x687B, 0x687C,
                crate::ram_map::SAMUS_AGE, crate::ram_map::SAMUS_AGE + 1,
                crate::ram_map::SAMUS_AGE + 2, crate::ram_map::SAMUS_AGE + 3,
                crate::ram_map::AT_ENDING, crate::ram_map::JUST_IN_BAILEY,
            ] {
                mem.write_ram(a, 0);
            }
            mem.write_ram(crate::ram_map::BANK_SWITCH_PENDING, 2);
        } else if addr == 0x8C5E {
            // CheckPassword（**asm 内容锚** `CheckPassword: ; 00:8C5E`——prg0_title.asm:
            // 1718-1733 逐行对读：jsr ConsolidatePassword($8F60)→jsr ValidatePassword(
            // $8DDE)→bcs L8C69（无效：SFXMultiInitFlags|=sfxMulti_IncorrectPassword=
            // %01（constants.asm:164）+Timer3←$0C（120 帧）+TitleRoutine←_id_EnterPassword
            // （0x18）/valid⇒jmp InitializeGame($92D4)）。
            // 【f1190 相位（A 级实测）】emu UnscramblePassword（00:8E4E @loop_A/@loop_B
            // 双层滚字节循环，prg0_title.asm:2078）纯 CPU 忙跨 2 tick ⇒ JIB($69B3)/
            // InitializeGame 落后 START 消费 1 拍（emu capture f1190 JIB=0 装 SAMUS）；
            // rust 同拍一气呵成 ⇒ 早 1 帧（JIB=1 装 SUITLESS，2063px）。首拍只置
            // pending，链延到下一 tick main_title_routine 头执行（见 check_password_pending）。
            mem.check_password_pending = true;
        } else {
            let loc = format!("bank00:{addr:04X}-jump");
            crate::unported!(&loc); // 其余跳转出口（转接面——随接）
        }
    }
}

/// CheckPassword 全链（asm `CheckPassword: ; 00:8C5E` prg0_title.asm:1718-1733）：
/// Consolidate/Validate/Unscramble → valid⇒InitializeGame / 无效⇒错码+回 0x18。
/// 独立 fn 以便延拍执行（见 main_title_routine 头 check_password_pending 消费点）。
fn run_check_password_chain(mem: &mut FrameMemory) {
    let mut chars = [0u8; 24];
    for (i, c) in chars.iter_mut().enumerate() {
        *c = mem.read_wram(crate::ram_map::PASSWORD_CHAR + i as u16);
    }
    let nar = mem.read_wram(crate::ram_map::NAR_PASSWORD);
    let (pb, valid, nar_hit) = crate::ui::password::check_password(&chars, nar != 0);
    // ConsolidatePassword/UnscramblePassword 原位写 PasswordByte（$6988 面——
    // DisplayPassword/存档族消费者面）。
    for (i, b) in pb.iter().enumerate() {
        mem.write_ram(crate::ram_map::PASSWORD_BYTE + i as u16, *b);
    }
    if nar_hit {
        mem.write_ram(crate::ram_map::NAR_PASSWORD, 1); // NARPASSWORD←1（:2031-2036）
    }
    if valid {
        initialize_game(mem, &pb, nar != 0 || nar_hit); // NARPASSWORD 旗=入口读值|本回合命中（asm 内存语义）
    } else {
        let f = mem.read_ram(crate::ram_map::SFX_MULTI_INIT) | 0x01;
        mem.write_ram(crate::ram_map::SFX_MULTI_INIT, f);
        mem.write_ram(title_state::TIMER3, 0x0C);
        mem.write_ram(crate::ram_map::TITLE_ROUTINE, 0x18);
    }
}

/// shell 组合点执行体（全臂——S6-4 精灵面 + S6-10 PPU 面）。
fn exec_shell_call(mem: &mut FrameMemory, sc: &crate::title::ShellCall) {
    use crate::title::ShellCall::*;
    match sc {
        EraseAllSprites => crate::frame::boot::c1a3_unit(mem), // $F0 填充（C1A3）
        ClearNameTables => crate::frame::boot::c158_unit(mem), // RAM 面+PPU 面（C158——S6-10）
        RemoveIntroSprites => {
            // **F4 离屏惯例**（C1BC：「sprite values with #$F4——bottom right+
            // blank graphic」——S6-4 精灵面迭代件；port2 fcda70b 变体就绪）。
            // 【范围修正（r14 复核 F2——asm 内容锚）】`RemoveIntroSprites: ; 07:C1BC`：
            // `ldy #$5F/lda #$F4/@loop: sta ($00),y/dey/bpl`——**只填 $0200-$025F（精灵 0-23）**；
            // 星槽 $0260-$02FF（LoadStarSprites 载面）不得抹——旧 0x100 全页填会每帧
            // 抹掉 40 星条目（f212 起星像素消失实证）。尾 `jmp DecSpriteYCoord`（$988A）在
            // title 窗被 SpriteLoadPending=0 门掉（无操作），运动面随件核。
            for i in 0..0x60u16 {
                mem.write_ram(0x0200 + i, 0xF4);
            }
        }
        PrepareVram(addr) => prepare_vram(mem, *addr),
        // DisplayInputCharacters（00:940B，prg0_title.asm:2933-2949）：密码屏字符网格。
        DisplayInputCharacters => display_input_characters(mem),
        // spin 等待段（r23：计数面——main_title_routine 入口消费）
        SpinWait(n) => mem.title_spin = *n,
        // ScreenOff/ScreenOn/NMIOn/VBOffAndHorzWrite/WaitNMIPass：$FE/$FF/$1A 镜像面
        // =臂事件承载（two-layer——事件即 RAM 面权威）；InitPasswordFontGfx/
        // DisplayInputCharacters 深件随批（未烘焙面随件爆）。
        _ => {}
    }
}

/// DisplayInputCharacters（00:940B）实现体——asm 逐行（prg0_title.asm:2933-2949）：
/// `lda PPUSTATUS`（锁存复位）→ `$00/$01←0`（$01=行计数）→ 环 5 行：
/// `x=行*2 / lda PasswordRowsTbl,x → PPUADDR hi+lo`（$943F=[$21E4,$2224,$2264,$22A4,$22E4]）
/// → 13 字符 `lda PasswordRow0,y → PPUDATA` + `lda #$FF → PPUDATA`（空白交错，`iny` **跨行连续**
/// 不重置）→ `inc $01 / cmp #$05 / bne`。PasswordRow0（$99A2，`.stringmap "0123456789ABC…"`）
/// ROM 逐字节=[0..63,$FF,…]，5×13=65 项恰用 0..64（第 65 项=$FF）。
fn display_input_characters(mem: &mut FrameMemory) {
    mem.render.reset_addr_latch();
    // $00/$01 初值 0（asm `tya/sta $00/sta $01`——$01=行计数器）
    mem.write_ram(0x0000, 0);
    mem.write_ram(0x0001, 0);
    const ROWS: [u16; 5] = [0x21E4, 0x2224, 0x2264, 0x22A4, 0x22E4];
    let mut y = 0usize;
    for (row, &addr) in ROWS.iter().enumerate() {
        mem.write_io(0x2006, (addr >> 8) as u8);
        mem.write_io(0x2006, (addr & 0xFF) as u8);
        for _ in 0..13 {
            // PasswordRow0[y]：y<64 ⇒ tile y；y==64 ⇒ $FF（ROM $99A2 第 65 项）
            let tile = if y < 64 { y as u8 } else { 0xFF };
            mem.write_io(0x2007, tile);
            mem.write_io(0x2007, 0xFF); // 空白 tile 交错（`lda #$FF/sta PPUDATA`）
            y += 1;
        }
        // `inc $01`（行计数——终值 5 与 asm 环出口一致）
        mem.write_ram(0x0001, (row + 1) as u8);
    }
}

/// PreparePPUProcess 组合点（asm 引证主体 `PreparePPUProcess: ; 07:C20E`——ptr→VRAMStructWrite）。
/// blob 面=game-data 烘焙（tools/bake_vram_structs.py）；未烘焙址 fail-loud（随件补）。
fn prepare_vram(mem: &mut FrameMemory, addr: u16) {
    match addr {
        0x82F4 => super::vram::vram_struct_write(
            mem, &game_data::title::vram_structs::INTRO_BG_VRAM_STRUCT),
        // L9984（prg0_title.asm:3534——StartContinueScreen :2333 装载件；r25 补）
        0x9984 => super::vram::vram_struct_write(
            mem, &game_data::title::vram_structs::START_CONTINUE_VRAM_STRUCT),
        // L99E3（prg0_title.asm:3560——LoadPasswordScreen :2399 `ldx #<L99E3.b / ldy #>L99E3.b
        // jsr PreparePPUProcess_`）："PASS WORD PLEASE" 行 $2088 + 属性表四段
        // （$23C0×$10 $00 / $23D0×$08 $55 / $23D8×$20 $FF / $23DA×$04 $F0）。卡 C 阻塞件。
        0x99E3 => super::vram::vram_struct_write(
            mem, &game_data::title::vram_structs::PASSWORD_PLEASE_VRAM_STRUCT),
        // @VRAMStruct（prg0_title.asm:2874——GameOver :939E `ldx #<@VRAMStruct.b /
        // jsr PreparePPUProcess_`）："GAME OVER" 行 $218C。
        0x93B9 => super::vram::vram_struct_write(
            mem, &game_data::title::vram_structs::GAME_OVER_VRAM_STRUCT),
        other => crate::unported!(&format!("bank00:{other:04X}-PrepareVram")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 统一入口穿越（phase 0 臂**单工作段+bogged span**（Step 2 机构改形）：工作段=
    /// 全仪式+相位推进+FC 同拍；等待=bogged 门队列（下一 tick 工作零动+NMI 内块跳过）。
    #[test]
    fn bridge_crosses_via_unified_entry() {
        let mut m = FrameMemory::default();
        main_title_routine(&mut m); // 工作段（仪式+推进）
        assert_eq!(m.read_ram(TITLE_ROUTINE), 1); // 相位推进同拍
        assert_eq!(m.iter_gates, vec![0], "bogged 门队列挂起（N₀=1）");
        assert!(m.iter_span_fc, "FC 延至 span 尾收口旗");
        assert!(m.title_no_return, "0x00 工作拍 FC 冻结（Step 2 入族）");
    }

    /// DisplayInputCharacters（00:940B）锚测：5 行网格 + y 跨行连续 + $FF 交错 + $01 终值 5。
    /// A 级：PasswordRowsTbl $943F=[$21E4,$2224,$2264,$22A4,$22E4]（prg0_title.asm:3544）+
    /// PasswordRow0 $99A2=[0..63,$FF,…]（ROM 逐字节）+ L4 f530 NT0 diff 0x1e4-0x2fa 实测。
    #[test]
    fn display_input_characters_writes_five_rows() {
        let mut m = FrameMemory::default();
        display_input_characters(&mut m);
        // 行 0（$21E4 → off 0x1E4）：tile 0..12 + $FF 交错（偶数位=字符/奇数位=$FF）
        for i in 0..13usize {
            assert_eq!(m.render.nametable[0x1E4 + i * 2], i as u8, "行0 字符 {i}");
            assert_eq!(m.render.nametable[0x1E4 + i * 2 + 1], 0xFF, "行0 空白 {i}");
        }
        // 行 4（$22E4 → off 0x2E4）：y=52..64（第 65 项=$FF）
        for i in 0..13usize {
            let y = 52 + i as u8;
            let expect = if y < 64 { y } else { 0xFF };
            assert_eq!(m.render.nametable[0x2E4 + i * 2], expect, "行4 字符 {i}");
        }
        // 行计数器 $01 终值 5（asm `inc $01/cmp #$05` 环出口）
        assert_eq!(m.read_ram(0x0001), 5);
        // 中间行地址（行 1/2/3 起点）
        assert_eq!(m.render.nametable[0x224], 13);
        assert_eq!(m.render.nametable[0x264], 26);
        assert_eq!(m.render.nametable[0x2A4], 39);
    }

    /// OBJECTS_CLEAR 复合标签展开（「标签展开在 shim」契约点——$0300-$031F 32 事件）。
    #[test]
    fn objects_clear_label_expands_in_shim() {
        let events = crate::memmap::title_state::objects_clear_events();
        assert_eq!(events.len(), 32);
        assert_eq!(events[0], (0x0300, 0));
        assert_eq!(events[31], (0x031F, 0));
    }

    /// RemoveIntroSprites 范围修正（r14 复核 F2——C1BC `ldy #$5F` 只填 $0200-$025F）：
    /// 星槽 $0260-$02FF（LoadStarSprites 载面）不得抹。
    #[test]
    fn remove_intro_sprites_preserves_star_slots() {
        let mut m = FrameMemory::default();
        m.write_ram(0x0260, 0x73); // 星槽首字节（IntroStarsData[0]）
        m.write_ram(0x02FF, 0x85); // 星槽末字节
        m.write_ram(0x0210, 0x00);
        exec_shell_call(&mut m, &crate::title::ShellCall::RemoveIntroSprites);
        assert_eq!(m.read_ram(0x0200), 0xF4, "精灵 0 填 F4");
        assert_eq!(m.read_ram(0x025F), 0xF4, "精灵 23 尾位填 F4");
        assert_eq!(m.read_ram(0x0260), 0x73, "星槽 24 保留（范围修正）");
        assert_eq!(m.read_ram(0x02FF), 0x85, "星槽 63 保留");
    }

    /// 00 臂 IntroStarOffset 复位钉死（asm `prg0_title.asm:124 sty IntroStarOffset`——
    /// r14 残差件：$C3 恒差根）。
    #[test]
    fn init_after_reset_resets_star_offset() {
        let mut m = FrameMemory::default();
        m.write_ram(0x00C3, 0x05); // 预置非零（复位必须落地）
        main_title_routine(&mut m); // 00 臂段0（工作）
        assert_eq!(m.read_ram(0x00C3), 0, "IntroStarOffset 复位（:124 sty）");
    }

    // ---- 卡 B G1/G3：密码提交→校验→InitializeGame 出生态（TDD 锚例）----

    /// JUSTIN BAILEY tile 序列（卡 A emu 腿实测锚——帧号表 0x18@f505/提交 START f1188）。
    const JB_CHARS: [u8; 24] = [
        0x13, 0x1E, 0x1C, 0x1D, 0x12, 0x17, 0x0B, 0x0A, 0x12, 0x15, 0x0E, 0x22,
        0x3F, 0x3F, 0x3F, 0x3F, 0x3F, 0x3F, 0x3F, 0x3F, 0x3F, 0x3F, 0x3F, 0x3F,
    ];

    /// JB 密码 0x1A 提交→0x8C5E→InitializeGame suitless Norfair 出生态。
    /// 断言值=卡 A emu 实测锚（$6878=7F/$6879=FF/$69B3=01/$74=11）+asm 推导面
    /// （BankTable[1]=3（07:CA30）/RestartYPosTbl[1]=$8C/X=$78/MainRoutine←MoreInit）。
    #[test]
    fn justin_bailey_password_jump_spawns_suitless_norfair() {
        let mut m = FrameMemory::default();
        for (i, &c) in JB_CHARS.iter().enumerate() {
            m.write_wram(crate::ram_map::PASSWORD_CHAR + i as u16, c);
        }
        // 预置脏面（验证覆盖真发生）。注：SAMUS_STAT00($6876) 密码路**不清**
        // （asm InitializeGame 无此写；真机域=开机 RAM 清零残留 0）——故不预置脏值。
        m.write_wram(crate::ram_map::SAMUS_GEAR, 0xAA);
        m.write_ram(crate::ram_map::BANK_SWITCH_PENDING, 0xEE);
        m.write_ram(crate::ram_map::TITLE_ROUTINE, 0x1A); // WaitForSTART
        m.write_ram(crate::ram_map::JOY1_CHANGE, 0x10); // START
        main_title_routine(&mut m);
        // 【延拍语义锚（emu UnscramblePassword 忙跨——prg0_title.asm:2078）】首拍只置
        // pending（START 消费拍），链（Consolidate/Validate/Unscramble→InitializeGame）
        // 在下一拍执行——emu 实测 JIB($69B3)/出生态落后 START 消费 1 tick。
        assert!(m.check_password_pending, "首拍=START 消费拍，链置 pending");
        main_title_routine(&mut m); // 第二拍：执行链
        assert!(!m.check_password_pending, "第二拍消费后清位");
        // PB 回写面（$6988——Consolidate+Unscramble 原位铁证）
        let want_pb: [u8; 18] = [
            0xA6, 0xF3, 0x8E, 0xA4, 0xB9, 0x65, 0x24, 0xA9, 0xD1, 0x7F,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        ];
        for (i, &b) in want_pb.iter().enumerate() {
            assert_eq!(m.read_wram(crate::ram_map::PASSWORD_BYTE + i as u16), b, "PB[{i}]");
        }
        // 出生态锚（卡 A emu 实测）
        assert_eq!(m.read_wram(crate::ram_map::SAMUS_GEAR), 0x7F, "suitless 位=1");
        assert_eq!(m.read_wram(crate::ram_map::MISSILE_COUNT), 0xFF);
        assert_eq!(m.read_wram(crate::ram_map::JUST_IN_BAILEY), 1);
        assert_eq!(m.read_ram(crate::ram_map::IN_AREA), 0x11, "Norfair 低半=1");
        assert_eq!(m.read_ram(crate::ram_map::BANK_SWITCH_PENDING), 3, "BankTable[1]");
        assert_eq!(m.read_ram(crate::ram_map::SAMUS_Y), 0x8C);
        assert_eq!(m.read_ram(crate::ram_map::SAMUS_X), 0x78);
        assert_eq!(m.read_ram(crate::ram_map::MAIN_ROUTINE), 1, "MoreInit");
        assert_eq!(m.read_ram(crate::ram_map::SAMUS_ANIM_FRAME), 0x5A);
        assert_eq!(m.read_ram(crate::ram_map::SAMUS_HI), 0);
        assert_eq!(m.read_wram(crate::ram_map::SAMUS_STAT00), 0, "缺省 0（密码路不清零 quirk）");
        // pw[15]=$FF：雕像四 bits 全置——Kraid=$80|$01、Ridley=$80|$02（0 亦写语义）。
        assert_eq!(m.read_wram(crate::ram_map::KRAID_STATUE_STATUS), 0x81);
        assert_eq!(m.read_wram(crate::ram_map::RIDLEY_STATUE_STATUS), 0x82);
        assert_eq!(m.read_wram(crate::ram_map::MAX_MISSILES), 0xCD, "MaxMissiles=205（ROM 探针 $8D3D 实跑）");
    }

    /// 非法密码提交→invalid 路：SFX 旗+Timer3=$0C+回 EnterPassword，不出生态。
    #[test]
    fn bad_password_jump_returns_to_enter() {
        let mut m = FrameMemory::default();
        let bad = [0x01u8; 24];
        for (i, &c) in bad.iter().enumerate() {
            m.write_wram(crate::ram_map::PASSWORD_CHAR + i as u16, c);
        }
        m.write_ram(crate::ram_map::TITLE_ROUTINE, 0x1A);
        m.write_ram(crate::ram_map::JOY1_CHANGE, 0x10);
        main_title_routine(&mut m); // 首拍=START 消费拍（链置 pending，未执行）
        assert!(m.check_password_pending, "首拍只置 pending");
        main_title_routine(&mut m); // 第二拍执行链（invalid 路）
        assert_eq!(m.read_ram(crate::ram_map::SFX_MULTI_INIT) & 0x01, 1, "IncorrectPassword SFX");
        assert_eq!(m.read_ram(title_state::TIMER3), 0x0C, "120 帧");
        assert_eq!(m.read_ram(crate::ram_map::TITLE_ROUTINE), 0x18, "回 EnterPassword");
        assert_eq!(m.read_ram(crate::ram_map::MAIN_ROUTINE), 0, "未出生态");
        assert_eq!(m.read_ram(crate::ram_map::BANK_SWITCH_PENDING), 0);
    }


    /// 未武装臂表址爆（per-entry 形——**域外相位=契约违例面 fail-loud 样本**
    /// （0x18/0x1E 均已臂化；title 全清））。
    #[test]
    fn bridge_unported_arm_detonates_with_table_addr() {
        let mut m = FrameMemory::default();
        m.write_ram(TITLE_ROUTINE, 0xFF); // 域外相位（契约违例 fail-loud 样本）
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| main_title_routine(&mut m)));
        let err = r.unwrap_err();
        let msg = err.downcast_ref::<String>().cloned()
            .or_else(|| err.downcast_ref::<&'static str>().map(|s| (*s).to_string()))
            .unwrap_or_default();
        assert!(msg.contains("unported @ bank00:"), "表址爆失准: {msg}");
    }
}

/// InitializeGame（**asm 内容锚** `InitializeGame: ; 00:92D4`——prg0_title.asm:2699-2758
/// 逐行移植；pw=解校验后的 PasswordByte 18B，nar_active=NARPASSWORD 旗读值）。
fn initialize_game(mem: &mut FrameMemory, pw: &[u8; 18], nar_active: bool) {
    // ClearRAM_33_DF（$C1D4）：zp $33-$DF 清零。
    for a in 0x33..=0xDF {
        mem.write_ram(a, 0);
    }
    // ClearSamusStats（$C578）：$0100-$010F 清零。
    for a in 0x0100..=0x010F {
        mem.write_ram(a, 0);
    }
    // LoadPasswordData（$8D12）：NARPASSWORD≠0⇒跳过全部（quirk 保真）。
    if !nar_active {
        load_password_data(mem, pw);
    }
    // 清对象数据四件：SpritePagePos($5B)/PageIndex($4B)/ObjectCntrl($6B)/Samus.hi($030C)。
    mem.write_ram(crate::ram_map::SPRITE_PAGE_POS, 0);
    mem.write_ram(crate::ram_map::PAGE_INDEX, 0);
    mem.write_ram(crate::ram_map::OBJECT_CNTRL, 0);
    mem.write_ram(crate::ram_map::SAMUS_HI, 0);
    // SilenceMusic（$CB8E）：SFXNoiseInitFlags($0680)|=sfxNoise_SilenceMusic=%01。
    let n = mem.read_ram(crate::ram_map::SFX_NOISE_INIT) | 0x01;
    mem.write_ram(crate::ram_map::SFX_NOISE_INIT, n);
    // Samus.animFrame($0303)←_id_ObjFrame_SkreeProjectile=0x5A（ObjFramePtrTable index 90）。
    mem.write_ram(crate::ram_map::SAMUS_ANIM_FRAME, 0x5A);
    // 出生位选：InArea&$0F≠0⇒x=1（非 Brinstar）else x=0。
    let in_area_low = mem.read_ram(crate::ram_map::IN_AREA) & 0x0F;
    let x: usize = if in_area_low != 0 { 1 } else { 0 };
    // RestartYPosTbl($9325)={64,8C,5C}/RestartXPosTbl($9328)={78,78,5C}——Samus.y($030D)/x($030E)。
    mem.write_ram(crate::ram_map::SAMUS_Y, [0x64, 0x8C, 0x5C][x]);
    mem.write_ram(crate::ram_map::SAMUS_X, [0x78, 0x78, 0x5C][x]);
    // inc SamusStat0B（16 位链——`inc $6881/bne skip/inc $6882`）。
    let lo = mem.read_wram(crate::ram_map::SAMUS_STAT0B).wrapping_add(1);
    mem.write_ram(crate::ram_map::SAMUS_STAT0B, lo);
    if lo == 0 {
        let hi = mem.read_wram(crate::ram_map::SAMUS_STAT0B + 1).wrapping_add(1);
        mem.write_ram(crate::ram_map::SAMUS_STAT0B + 1, hi);
    }
    // MainRoutine($1E)←_id_MoreInit=1。
    mem.write_ram(crate::ram_map::MAIN_ROUTINE, 0x01);
    // ScreenNmiOff（07:C45D）：PPUMASK_ZP($FE)&=~$18+PPUCTRL_ZP($FF)&=~$80（$2000 端口
    // 写走镜像读出面——本层 RAM 镜像事件）。
    let pm = mem.read_ram(crate::ram_map::PPUMASK_ZP) & !0x18;
    mem.write_ram(crate::ram_map::PPUMASK_ZP, pm);
    // 【转场暗窗相位（2026-11-12）】`WriteAndWait`（07:C43D）写影子后 `WaitNMIPass_`
    // （07:C43F）**等一个 NMI**——真 PPUMASK 由该 NMI 内的 `WritePPUCtrl`（07:C44D）
    // 落，因此本次等待所在帧的渲染色已是「关屏」。本层 tick 原子执行整个链，等价的
    // 近似=此刻即写真寄存器（暗窗起点因此与该帧对齐：emu 实测 f1191 首黑）。
    mem.write_io(0x2001, pm);
    let pc = mem.read_ram(crate::ram_map::PPUCTRL_ZP) & !0x80;
    mem.write_ram(crate::ram_map::PPUCTRL_ZP, pc);
    // LoadSamusGFX（$C5DC，prg7_engine.asm:1496-1520）：Samus→(JIB?Suitless)→
    // IntroSprites→Font_Complete→Font_Hud→Solid_0FC0→Solid_1FC0 七件（ScreenNmiOff 后/
    // NMIOn 前——ROM→CHR-RAM 写在 NMI 关闭窗口，同 asm 序）。
    super::gfx::load_samus_gfx(mem);
    // NMIOn（07:C487）：PPUCTRL_ZP|=$80。
    let pc = mem.read_ram(crate::ram_map::PPUCTRL_ZP) | 0x80;
    mem.write_ram(crate::ram_map::PPUCTRL_ZP, pc);
    // BankTable（07:CA30）={$02,$03,$05,$04,$06, $48,$48,$20,$96,$CA,...}→BankSwitchPending($24)。
    // 表外索引（InArea&$0F>4=checksum 合法伪造密码域）读表后代码字节——quirk 照烤
    // （ROM 逐字节 07:CA30-CA3F：02 03 05 04 06 48 48 20 96 CA AD 84 68 10 0D 29）。
    const BANK_TABLE: [u8; 16] = [
        0x02, 0x03, 0x05, 0x04, 0x06, 0x48, 0x48, 0x20, 0x96, 0xCA, 0xAD, 0x84, 0x68, 0x10, 0x0D, 0x29,
    ];
    mem.write_ram(crate::ram_map::BANK_SWITCH_PENDING, BANK_TABLE[in_area_low as usize]);
}

/// LoadPasswordData（**asm 内容锚** `LoadPasswordData: ; 00:8D12`——prg0_title.asm:
/// 1839-1863；NAR 跳过由调用侧承担）。
fn load_password_data(mem: &mut FrameMemory, pw: &[u8; 18]) {
    load_unique_items(mem, pw);
    load_tanks_and_missiles(mem, pw);
    // JustInBailey($69B3)←PB[8]&$80 ? 1 : 0；InArea($74)←PB[8]&$3F。
    let pb8 = pw[8];
    mem.write_ram(crate::ram_map::JUST_IN_BAILEY, if pb8 & 0x80 != 0 { 1 } else { 0 });
    mem.write_ram(crate::ram_map::IN_AREA, pb8 & 0x3F);
    // SamusAge($687D..6880)←PB[11..14]（`ldy #3` 逆序拷贝——PB[11+B]→$687D+B）。
    for (i, b) in pw[11..15].iter().enumerate() {
        mem.write_ram(crate::ram_map::SAMUS_AGE + i as u16, *b);
    }
}

/// LoadUniqueItems（**asm 内容锚** `LoadUniqueItems: ; 00:8BD4`——prg0_title.asm:
/// 1614-1716）：59 项逐位扫描（**位序=LSB 先**——`ror` 使 bit0→carry；A 级 ROM 探针
/// （2026-11-10 $8BD4 实跑）：JB PB=A6 F3… 命中序列与 LSB-first 映射逐位吻合、
/// MSB-first 假设被证伪）；
/// 有位⇒UniqueItemHistory[$6887+n..n+1]←ItemData[item] 低/高字节，NumberOfUniqueItems
/// [$6886] 2 递增。ItemData（00:9029-$909F，@end-$start)/2=59——59 项烘焙表（见下）。
fn load_unique_items(mem: &mut FrameMemory, pw: &[u8; 18]) {
    mem.write_ram(crate::ram_map::NUMBER_OF_UNIQUE_ITEMS, 0);
    let mut n = 0u16;
    for item in 0..59usize {
        if pw[item >> 3] & (1 << (item & 7)) != 0 {
            let v = ITEM_DATA[item];
            mem.write_ram(crate::ram_map::UNIQUE_ITEM_HISTORY + n, (v & 0xFF) as u8);
            mem.write_ram(crate::ram_map::UNIQUE_ITEM_HISTORY + n + 1, (v >> 8) as u8);
            n += 2;
            mem.write_ram(crate::ram_map::NUMBER_OF_UNIQUE_ITEMS, n as u8);
        }
    }
}

/// 6502 adc 全语义（A+B+Cin→(A,Cout)）。
fn adc(a: u8, b: u8, c_in: bool) -> (u8, bool) {
    let s = a as u16 + b as u16 + c_in as u16;
    (s as u8, s > 0xFF)
}

/// LoadTanksAndMissiles（**asm 内容锚** `LoadTanksAndMissiles: ; 00:8D3D`——
/// prg0_title.asm:1867-1992 逐行对读）。
fn load_tanks_and_missiles(mem: &mut FrameMemory, pw: &[u8; 18]) {
    mem.write_ram(crate::ram_map::SAMUS_GEAR, pw[9]);
    mem.write_ram(crate::ram_map::MISSILE_COUNT, pw[10]);
    // 雕像态（Temp00/Temp02 组装后 sta——0 亦写）：
    let mut kraid = 0u8;
    if pw[15] & 0x80 != 0 { kraid |= 0x80; } // 雕像 up
    if pw[15] & 0x40 != 0 { kraid |= 0x01; } // Kraid 败
    mem.write_ram(crate::ram_map::KRAID_STATUE_STATUS, kraid);
    let mut ridley = 0u8;
    if pw[15] & 0x20 != 0 { ridley |= 0x80; }
    if pw[15] & 0x10 != 0 { ridley |= 0x02; }
    mem.write_ram(crate::ram_map::RIDLEY_STATUE_STATUS, ridley);
    // 坦克计数：遍历 UniqueItemHistory 高字节（y=0..$83 每 2B；`and #$FC` cmp $20/
    // $24——**全 $84 字节域扫描**（零填充域不命中））。ETank cap 6（`cmp #6/bcc`）。
    let mut history = [0u8; 0x84];
    for (i, h) in history.iter_mut().enumerate() {
        *h = mem.read_wram(crate::ram_map::UNIQUE_ITEM_HISTORY + i as u16);
    }
    let mut etank = 0u8;
    let mut mtank = 0u8;
    let mut y = 0usize;
    while y < 0x84 {
        let hi = history[y + 1] & 0xFC;
        if hi == 0x20 {
            etank = etank.wrapping_add(1);
        } else if hi == 0x24 {
            mtank = mtank.wrapping_add(1);
        }
        y += 2;
    }
    // `cmp #$06/bcc/lda #6`——C=ETank>=6（**quirk：C 残留进 MaxMissiles adc 链**）。
    let mut a: u8 = if etank >= 6 { 6 } else { etank };
    let mut c = etank >= 6;
    mem.write_ram(crate::ram_map::ENERGY_TANK_COUNT, a);
    // MaxMissiles：A=0；mtank≠0⇒clc+adc#5×mtank；Kraid≠0⇒adc#$4B（bcs⇒cap）；
    // Ridley≠0⇒adc#$4B（bcc⇒过/bcs⇒cap $FF）。**carry-in quirk 照搬**（mtank=0 时
    // C=ETank>=6 残留——0x4B+1=0x4C 域，asm `ldy Temp02/beq` 不清 C）。
    a = 0;
    if mtank != 0 {
        c = false; // clc
        for _ in 0..mtank {
            let (v, ov) = adc(a, 5, c);
            a = v;
            c = ov;
        }
    }
    if kraid != 0 {
        let (v, ov) = adc(a, 0x4B, c);
        a = v;
        c = ov;
        if c {
            mem.write_ram(crate::ram_map::MAX_MISSILES, 0xFF);
            return;
        }
    }
    if ridley != 0 {
        let (v, ov) = adc(a, 0x4B, c);
        a = v;
        c = ov;
        if c {
            mem.write_ram(crate::ram_map::MAX_MISSILES, 0xFF);
            return;
        }
    }
    mem.write_ram(crate::ram_map::MAX_MISSILES, a);
}

/// ItemData 59 项（**asm 数据锚** `ItemData: ; 00:9029`-$909F @end——prg0_title.asm:
/// 2237-2416 逐行抄录：.word ui_X+(x<<5)+y；ui_ enumid=constants.asm:67-87）。
pub const ITEM_DATA: [u16; 59] = [
    0x104E, 0x264B, 0x28E5, 0x2882, 0x2327, 0x2B25, 0x0325, 0x2A69, 0x2703, 0x2363,
    0x29E2, 0x15E2, 0x212E, 0x264E, 0x262F, 0x2B4C, 0x276A, 0x278A, 0x278B, 0x276B,
    0x274B, 0x268F, 0x266F, 0x2B71, 0x0771, 0x29F0, 0x0DF0, 0x2676, 0x2696, 0x2A55,
    0x2353, 0x2794, 0x28F5, 0x2535, 0x2495, 0x28F6, 0x2156, 0x28F8, 0x287B, 0x24BB,
    0x2559, 0x291D, 0x211D, 0x2658, 0x2A39, 0x2239, 0x269E, 0x2A1D, 0x21FD, 0x271B,
    0x2867, 0x2927, 0x292B, 0x3C00, 0x4000, 0x4400, 0x4800, 0x4C00, 0x3800,
];
