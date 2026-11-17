//! frame 域测试集（原 mainloop.rs 的测试模块，纯搬移）。
//!
//! 子模块照旧 `use super::*;`——本模块用 `pub(crate) use` 把 frame 域公开面再导出一次，
//! 使其对子模块可见（普通 `use` 不能穿透到子模块的 glob）。

pub(crate) use crate::frame::tick::*;
use crate::frame::Manager;
use crate::ram_map::*;

#[cfg(test)]
mod tests_pause_chain {
    use super::*;
    use crate::frame::memory::Manager;
    use crate::ram_map;

    /// `GoMainRoutine` START 开关：3↔5 双向 + `GamePaused ^= 1` + `PauseMusic`（$CB92=%10）
    /// ——A 级锚 `prg7_engine.asm:311-338`。
    #[test]
    fn start_toggles_between_engine_and_pause() {
        let mut m = Manager::default();
        m.write_ram(ram_map::MAIN_ROUTINE, 3);
        m.write_ram(ram_map::JOY1_CHANGE, 0x10); // START
        assert!(start_pause_toggle(&mut m), "3 + START 应切换");
        assert_eq!(m.read_ram(ram_map::MAIN_ROUTINE), 5, "GameEngine → PauseMode");
        assert_eq!(m.read_ram(ram_map::GAME_PAUSED), 1, "GamePaused ^= 1（0→1）");
        assert_eq!(m.read_ram(ram_map::SFX_NOISE_INIT) & 0x02, 0x02, "PauseMusic 置 noise init bit1");
    }

    #[test]
    fn start_unpauses_back_to_engine() {
        let mut m = Manager::default();
        m.write_ram(ram_map::MAIN_ROUTINE, 5);
        m.write_ram(ram_map::GAME_PAUSED, 1);
        m.write_ram(ram_map::JOY1_CHANGE, 0x10);
        assert!(start_pause_toggle(&mut m));
        assert_eq!(m.read_ram(ram_map::MAIN_ROUTINE), 3, "PauseMode → GameEngine");
        assert_eq!(m.read_ram(ram_map::GAME_PAUSED), 0, "GamePaused ^= 1（1→0）");
    }

    /// 对照（防空对照）：非 3/5 态必须忽略 START（asm `cmp #5 / bne @endIf_B`）。
    #[test]
    fn start_ignored_outside_engine_and_pause() {
        for mr in [0u8, 1, 2, 4, 8, 9] {
            let mut m = Manager::default();
            m.write_ram(ram_map::MAIN_ROUTINE, mr);
            m.write_ram(ram_map::JOY1_CHANGE, 0x10);
            assert!(!start_pause_toggle(&mut m), "mr={mr} 不应被 START 改变");
            assert_eq!(m.read_ram(ram_map::MAIN_ROUTINE), mr);
            assert_eq!(m.read_ram(ram_map::SFX_NOISE_INIT), 0, "不得起 PauseMusic");
        }
        // 无 START 也不切
        let mut m = Manager::default();
        m.write_ram(ram_map::MAIN_ROUTINE, 3);
        m.write_ram(ram_map::JOY1_CHANGE, 0x00);
        assert!(!start_pause_toggle(&mut m));
        assert_eq!(m.read_ram(ram_map::MAIN_ROUTINE), 3);
    }

    /// PauseMode 成功退出：**2P 的 A+UP 同按** + `EndTimer+1==0` ⇒ `GamePaused←0` + `inc MainRoutine`
    /// （5→6 = GoPassword，**mr=6 的唯一产生点**）。A 级锚 `prg7_engine.asm:2139-2166`。
    #[test]
    fn pause_mode_exits_to_go_password() {
        let mut m = Manager::default();
        m.write_ram(ram_map::MAIN_ROUTINE, 5);
        m.write_ram(ram_map::GAME_PAUSED, 1);
        m.write_ram(ram_map::JOY2_STATUS, 0x88); // BUTTON_A|BUTTON_UP
        pause_mode(&mut m);
        assert_eq!(m.read_ram(ram_map::MAIN_ROUTINE), 6, "5 → 6（inc MainRoutine = GoPassword）");
        assert_eq!(m.read_ram(ram_map::GAME_PAUSED), 0, "sta GamePaused（A=0）");
    }

    /// 对照三态：只按 A / 只按 UP / 逃脱计时器活动 —— 都不得退出。
    #[test]
    fn pause_mode_blocked_when_gate_not_met() {
        for joy2 in [0x80u8, 0x08, 0x00] {
            let mut m = Manager::default();
            m.write_ram(ram_map::MAIN_ROUTINE, 5);
            m.write_ram(ram_map::JOY2_STATUS, joy2);
            pause_mode(&mut m);
            assert_eq!(m.read_ram(ram_map::MAIN_ROUTINE), 5, "joy2={joy2:02X} 不应退出");
        }
        // 逃脱场景（EndTimer+1≠0）禁止退出
        let mut m = Manager::default();
        m.write_ram(ram_map::MAIN_ROUTINE, 5);
        m.write_ram(ram_map::JOY2_STATUS, 0x88);
        m.write_ram(ram_map::END_TIMER + 1, 1);
        pause_mode(&mut m);
        assert_eq!(m.read_ram(ram_map::MAIN_ROUTINE), 5, "逃脱场景中不能退出暂停");
    }

    /// GoPassword：`TitleRoutine←0x19` + `$24←1` + `SilenceMusic` + ScreenOff（三件 + 关屏）。
    #[test]
    fn go_password_switches_to_title_bank_for_display_password() {
        let mut m = Manager::default();
        m.write_ram(ram_map::MAIN_ROUTINE, 6);
        m.write_ram(0x00FE, 0x1E);
        go_password(&mut m);
        assert_eq!(m.read_ram(ram_map::TITLE_ROUTINE), 0x19, "TitleRoutine←_id_DisplayPassword");
        assert_eq!(m.read_ram(ram_map::BANK_SWITCH_PENDING), 0x01, "BankSwitchPending←1");
        assert_eq!(m.read_ram(ram_map::SFX_NOISE_INIT) & 0x01, 0x01, "SilenceMusic 置 bit0");
        assert_eq!(m.read_ram(0x00FE), 0x06, "ScreenOff（0x1E & ~0x18）");
        assert_eq!(m.render.ppumask, 0x06, "真 PPUMASK 同拍落地");
    }

    /// 端到端：3 + START → 5（暂停）→ 2P A+UP → 6 → 帧界换页 → GameMode=1 且 TitleRoutine=0x19。
    #[test]
    fn pause_to_password_is_reachable_end_to_end() {
        let mut m = Manager::default();
        m.write_ram(ram_map::MAIN_ROUTINE, 3);
        m.write_ram(ram_map::JOY1_CHANGE, 0x10);
        // 第一拍：START ⇒ 暂停（同帧分派 PauseMode；Joy2 未按 ⇒ 停在 5）
        game_main_routine(&mut m);
        assert_eq!(m.read_ram(ram_map::MAIN_ROUTINE), 5, "第一拍后应处暂停");
        // 第二拍：2P A+UP ⇒ GoPassword ⇒ 关屏 + 切页
        m.write_ram(ram_map::JOY1_CHANGE, 0);
        m.write_ram(ram_map::JOY2_STATUS, 0x88);
        game_main_routine(&mut m);
        assert_eq!(m.read_ram(ram_map::MAIN_ROUTINE), 6, "第二拍应进 GoPassword（PauseMode 的 inc 已生效）");
        // 第三拍：mr=6 才被分派（asm `PauseMode` 的 inc 在 JumpEngine 之后 ⇒ 新值下一帧才跑）
        game_main_routine(&mut m);
        assert_eq!(m.read_ram(ram_map::TITLE_ROUTINE), 0x19, "DisplayPassword 相位");
        // 帧界：消费 $24=1 ⇒ InitBank0 ⇒ GameMode←1（标题轴接管）
        crate::frame::boot::check_bank_switch(&mut m);
        assert_eq!(m.read_ram(ram_map::GAME_MODE), 1, "InitBank0 置 GameMode←1");
    }

    /// IncrementRoutine（$C155）= `inc MainRoutine / rts`。
    #[test]
    fn increment_routine_advances_one() {
        let mut m = Manager::default();
        m.write_ram(ram_map::MAIN_ROUTINE, 7);
        increment_routine(&mut m);
        assert_eq!(m.read_ram(ram_map::MAIN_ROUTINE), 8);
    }
}

#[cfg(test)]
mod tests_prepare_game_over {
    use super::*;
    use crate::frame::memory::Manager;
    use crate::ram_map;

    /// 死亡链端到端（state 机层，A 级 asm 锚）：
    /// `GameEngine` 置 `NextRoutine←4`（prg7:2074-2075 `ldx #$04/jmp SetTimer`）→
    /// `WaitTimer(mr=9)` 等 `Timer3` 尽 → `MainRoutine←4`（prg7:1224-1231）→
    /// `PrepareGameOver(4)` 三写（prg7:2124-2130）→ 帧界 `CheckBankSwitch` 消费 `$24=1`
    /// → `InitBank0`（`GameMode←1`）。
    /// 修前此链在 `mr=4` 处 `unported!("bank07:C9A6")` panic（审计 §6 件 #1）。
    #[test]
    fn death_chain_reaches_game_over_and_switches_to_title_bank() {
        let mut m = Manager::default();
        // 死亡前置：GameEngine 已 SetTimer(#$04, x=4) 且 Timer3 已尽（WaitTimer 待跑）
        m.write_ram(ram_map::MAIN_ROUTINE, 9); // WaitTimer
        m.write_ram(ram_map::TIMER3, 0);
        m.write_ram(0x0020, 4); // NextRoutine←PrepareGameOver
        m.write_ram(0x00FE, 0x1E); // 屏亮态（ScreenOff 前）

        // 第一拍：WaitTimer 尽 ⇒ MainRoutine←NextRoutine(4)，且不得起乐（NextRoutine∈{4,6}）
        game_main_routine(&mut m);
        assert_eq!(m.read_ram(ram_map::MAIN_ROUTINE), 4, "WaitTimer 尽应置 PrepareGameOver");

        // 第二拍：PrepareGameOver 三写
        game_main_routine(&mut m);
        assert_eq!(m.read_ram(ram_map::TITLE_ROUTINE), 0x1C, "TitleRoutine←_id_GameOver");
        assert_eq!(m.read_ram(ram_map::BANK_SWITCH_PENDING), 0x01, "BankSwitchPending←1（切标题页）");
        assert_eq!(m.read_ram(0x00FE) & 0x18, 0, "ScreenOff：PPUMASK_ZP 关 BG/OBJ（and #~$18）");
        assert_eq!(m.render.ppumask & 0x18, 0, "WriteAndWait：真 PPUMASK 同拍落地");

        // 帧界：CheckBankSwitch 消费 $24=1 ⇒ InitBank0 ⇒ GameMode←1（标题轴接管）
        crate::frame::boot::check_bank_switch(&mut m);
        assert_eq!(m.read_ram(ram_map::BANK_SWITCH_PENDING), 0, "CheckBankSwitch 先清 $24");
        assert_eq!(m.read_ram(ram_map::GAME_MODE), 1, "InitBank0 置 GameMode←1");
    }

    /// 对照（防空对照）：只清 BG/OBJ 两位（`and #~$18`），**其余位保持**，
    /// 且绝不把 `$1E` 写回（那是 ScreenOn 的形）——防「用点屏冒充关屏」的假绿。
    #[test]
    fn prepare_game_over_clears_only_bg_obj_bits() {
        let mut m = Manager::default();
        m.write_ram(ram_map::MAIN_ROUTINE, 4);
        m.write_ram(0x00FE, 0x1E);
        prepare_game_over(&mut m);
        assert_eq!(m.read_ram(0x00FE), 0x06, "0x1E & ~0x18 = 0x06（低位保留，asm 只清 $18）");
        assert_ne!(m.read_ram(0x00FE), 0x1E, "不得点屏");
        assert_eq!(m.render.ppumask, 0x06, "真 PPUMASK 同形落地");
    }
}










pub use crate::enemy::slot_cycle::update_all_enemies;











// ==== r26 UpdateWorld dispatch 壳（复核 C blocker——缺件静默分歧根修！）====
// 模式：空径端口（槽环全空⇒忠实跳过）+ 活性 fail-loud（弹件径同式）。













/// 【BCD 掩码审计卡（2026-11-10，verifier cda2f2cb r9 复核遗留 B 级→模拟升级 A 级）】
/// asm 真源：SubtractHealth 07:CE92（refs/m1disasm/SRC/prg7_engine.asm:3203-3257）
/// →Base10Subtract 07:C3FB（:1072-1092）→ExtractNibbles 07:C41D（:1094-1104）。
/// Python 6502 级逐指令对拍（analysis/bcd_mask_audit/diff_model.py，3.36M 组合）：
/// ① Health+1 掩码（旧 `r1 & 0x9F | (h1 & 0x60)`）为**分歧实现**——asm 无对应物，
///   asm 的 Health+1 高位 nibble 参与 BCD 借位/回绕运算（借位时 $02+0x10、
///   负结果 +$A0 调整会合法改写 bit6-5），掩码剥离该结果=静默漂移（asm $79→rust $19）。
/// ② Varia 半伤公式分歧：asm LCEA9-CEBD=`a>>=1; b>>=1; b_carry⇒a=$4F+a+1`
///   （半字节域补偿 +50 到十位）；旧 rust=`(a+0x4F)>>1`——Tourian $0300 伤害 Varia
///   实测 asm=$0150 旧 rust=$0127。
/// 对拍明细见 docs/findings/bcd-mask-audit-subtracthealth.md。
#[cfg(test)]
mod bcd_mask_audit_tdd {
    use super::*;
    use crate::frame::Manager;

    fn setup(m: &mut Manager, h0: u8, h1: u8, a: u8, b: u8, varia: bool) {
        m.write_ram(0x0300, 0); // sa_Stand=0（过 IsSamusDead 门）
        m.write_ram(0x0106, h0);
        m.write_ram(0x0107, h1);
        m.write_ram(0x006E, a);
        m.write_ram(0x006F, b);
        let gear = if varia { 0x20 } else { 0x00 };
        m.write_wram(crate::ram_map::SAMUS_GEAR, gear);
    }

    /// 【RED→GREEN】掩码分歧锚 1（Health+1 BCD 回绕）：Health=(00,00)，
    /// HealthChange=(00,21)（210）。asm Base10Subtract($00, $21) 借位链：
    /// $02=0x20+0x10=0x30、0x00−0x30 借位→adc #$A0 调整→0x79；
    /// 旧掩码剥 bit6-5（0x79&0x9F=0x19）⇒ rust=0x19≠asm=0x79。
    #[test]
    fn health_plus1_bcd_wrap_no_mask() {
        // 【2026-11-17 更正】本测原以 `subtract_health` 断言 Health+1=0x79；死亡分支落地后
        // 该借位（`bcs` 未取）走死径把血量清零 ⇒ 0x79 不再出现在 RAM 面。
        // 掩码真值改为在 **BCD 层**钉住（同一 asm `Base10Subtract ; 07:C3FB` 输出）：
        assert_eq!(super::bcd_sub(0x00, 0x21), (0x79, true), "asm 借位回绕真值 $79（掩码不得剥离 bit6-5）");
        // 顶层行为：借位 ⇒ 判死 + 血量清零（asm `@endIf_D` 起）。
        let mut m = Manager::default();
        setup(&mut m, 0x00, 0x00, 0x00, 0x21, false);
        subtract_health(&mut m);
        assert_eq!(m.samus().health(), 0x0000);
        assert_eq!(m.samus().status_raw(), 7);
    }

    /// 【RED→GREEN】掩码分歧锚 2（Brinstar 可达：罐数 2 + 伤害 $0080）：
    /// Health=(50,20)，HealthChange=(80,00)。asm r1=$19（借位进罐 nibble 域合法）；
    /// 旧掩码 |h1&0x60 拼回 0x20 ⇒ rust=$39≠asm=$19。
    #[test]
    fn health_plus1_tank_borrow_faithful() {
        let mut m = Manager::default();
        setup(&mut m, 0x50, 0x20, 0x80, 0x00, false);
        subtract_health(&mut m);
        assert_eq!(m.read_ram(0x0106), 0x70);
        assert_eq!(m.read_ram(0x0107), 0x19, "asm 真值 $19（旧掩码回拼 $39）");
    }

    /// 【RED→GREEN】Varia 半伤公式（Tourian 可达：伤害 $0300 × Varia）：
    /// asm LCEA9-CEBD：b bit0=1（lsr 借出）⇒ a=$4F+($00>>1)+1=$50 ⇒ $0150；
    /// 旧 rust=(a+0x4F)>>1=$27 ⇒ $0127。锚 (50,98)=asm 模型逐字节真值。
    #[test]
    fn varia_half_damage_formula() {
        // 【2026-11-17 更正】同 `health_plus1_bcd_wrap_no_mask`：$0300 半伤=$0150 扣在
        // 0 血上必然借位 ⇒ 现在走死径清零。半伤公式真值在 BCD 层钉住：
        let (r0, borrow) = super::bcd_sub(0x00, 0x50);
        let (r1, _) = super::bcd_sub(0x00, if borrow { 0x01 + 1 } else { 0x01 });
        assert_eq!((r0, r1), (0x50, 0x98), "Varia $0300→$0150：a=$4F+(a>>1)+1");
        let mut m = Manager::default();
        setup(&mut m, 0x00, 0x00, 0x00, 0x03, true);
        subtract_health(&mut m);
        assert_eq!(m.samus().status_raw(), 7, "扣穿即死");
    }

    /// 对照：无借位常规扣血不回归（30−8=22）。
    #[test]
    fn plain_subtract_no_regression() {
        let mut m = Manager::default();
        setup(&mut m, 0x00, 0x03, 0x08, 0x00, false);
        subtract_health(&mut m);
        assert_eq!(m.read_ram(0x0106), 0x92, "30−8=22 BCD（0x00−8 借位百位 3→2，个位 0→2 的回绕=0x92）");
        assert_eq!(m.read_ram(0x0107), 0x02, "300−8=292：百位 3−借位1=2");
    }
}

/// DisplayBar（**asm 内容锚** `DisplayBar: ; 07:E0C1`——HUD 精灵动态面，全径端口）：
/// ① 拷 DataDisplayTbl(E1B9, 40B=10 精灵)→OAM[$5B..]（ROM 字节核实 40 字节）
/// ② 健康 2 位：($0107&0xF)|$A0、($0106>>4)|$A0（SPRWriteDigit E173 `ora #$A0`）
/// ③ $010B≠$FF⇒逃生计时 3 位+TI/ME($58/$59 attrib++）；否则 MaxMissiles($687A)≠0⇒
///    HexToDec(E198) 3 位；=0⇒LE10A 擦图标 tile（$0200+X+$0D/$11←$FF，X=原页+8，
///    恰为表槽 5/6 图标 tileID）；LE14A 能量罐（full=$0107>>4，tile $6F/空 $6E，
///    AddOneTank E17B：y=$17/attr=$01/x=$40 递减 $0A）。





/// BankTable 烘焙（07:CA30——16 项（A&0xF 索引→BankSwitchPending 值））。







/// AreaPointers_ROM 五区烘焙（**asm `CopyAreaPointers: ; 07:C8B0`** `lda AreaPointers_ROM,x`——
/// ROM 侧=$959A 起 14B，**取自当前区 bank**（bank 由 InArea 经 BankTable 07:CA30 换）——
/// 7 指针：RoomPtrTable/StructPtrTable/MetatileDefs/EnFramePtrTable1/2/EnPlacePtrTable/EnAnimTable）。
/// 五区 ROM 独立核验（file off = bank*0x4000+0x1560-0x4000+0x4000 区换算，python 逐字节）：
/// bank1(Brinstar)=`14 a3 72 a3 f0 ae e0 9d e0 9e 0e 9f 6a 9d`（=旧单区常量，原位不变）
/// bank2(Norfair)=`1b a2 77 a2 ec ae 64 9c 64 9d 78 9d da 9b` /
/// bank3(Tourian)=`d1 a7 fb a7 49 ae 2c a4 2c a5 40 a5 06 a4` /
/// bank4(Kraid)=`d5 a1 1f a2 32 ac f7 9c f7 9d 25 9e 86 9c` /
/// bank5(Ridley)=`7f a1 d3 a1 23 ab f0 9b f0 9c 04 9d 85 9b`。
/// 区序（InArea&$0F 0..4）→bank（BankTable）→上列行：0→1/1→2/2→4/3→3/4→5。





// （旧 game_engine/update_world 桩已并入上方 r26 端口形——原块删除）



#[cfg(test)]
mod tests {
    use crate::ram_map::{TIMER2, TIMER3};
    use super::*;

    /// RandomNumbers 合成值自证（+$19/+$5F——注×5 互证面）。
    #[test]
    fn random_numbers_accumulates() {
        let mut m = Manager::default();
        m.write_ram(RANDOM1, 0x11); // boot 初值（C0AC 自证）
        m.write_ram(RANDOM2, 0xFF);
        random_numbers(&mut m);
        assert_eq!(m.read_ram(RANDOM1), 0x2A);
        assert_eq!(m.read_ram(RANDOM2), 0x5E);
        assert_eq!(m.regs.a, 0x2A);
    }

    /// 迭代序门（r14 复核 F3/F4）：续段 tick 冻结 RandomNumbers/CheckBankSwitch/UpdateTimer
    /// （等待段自旋于 GoMainRoutine 内部——循环体四件每迭代一次）。
    #[test]
    fn continuation_tick_freezes_iteration_items() {
        let mut m = crate::frame::Manager::default();
        m.write_ram(GAME_MODE, 1); // 标题路（continuation 语义面——GameMode=1 免游戏派发）
        m.write_ram(crate::ram_map::TITLE_ROUTINE, 1);
        m.title_cursor = 1;
        m.title_cursor_phase = 1; // 同相位续段（真续段）
        m.write_ram(crate::ram_map::RANDOM1, 0x10);
        m.write_ram(crate::ram_map::RANDOM2, 0x20);
        m.write_ram(TIMER_DELAY, 5);
        m.write_ram(FRAME_COUNT, 7);
        main_loop_iter(&mut m);
        assert_eq!(m.read_ram(crate::ram_map::RANDOM1), 0x10, "RandomNumbers 冻结");
        assert_eq!(m.read_ram(crate::ram_map::RANDOM2), 0x20, "RandomNumbers 冻结");
        assert_eq!(m.read_ram(TIMER_DELAY), 5, "UpdateTimer 冻结");
    }

    /// F4 孤儿游标=新相位首个真迭代（相位推进后残留 cursor——禁误门）：循环体跑。
    #[test]
    fn orphan_cursor_after_phase_advance_is_fresh_iteration() {
        let mut m = crate::frame::Manager::default();
        m.write_ram(GAME_MODE, 1); // 标题路（continuation 语义面）
        m.write_ram(crate::ram_map::TITLE_ROUTINE, 2);
        m.title_cursor = 2;
        m.title_cursor_phase = 1; // 归属相位≠当前（孤儿）
        m.write_ram(TIMER_DELAY, 5);
        main_loop_iter(&mut m);
        assert_eq!(m.read_ram(TIMER_DELAY), 4, "UpdateTimer 跑（真迭代——禁误门）");
    }

    /// FC 冻结门（r9 porter 修——findings/fc-phase-residual.md）：内旋族相位
    /// （$01 上电初始化臂/$1B StartContinue 臂——asm 例程体+WaitNMIPass_ 自旋尾不返回
    /// 主循环）tick 不自增 FC；相位推进并入臂内最后工作段（emu 冻帧内推进实录：
    /// 上电 f4-5 冻/f6 推进、menu f311-312 冻/f313 起自增），臂毕恢复自增。
    /// mutation 面：旧形（每 tick 无条件加）下两窗各多 3 ⇒ fcprobe 逐帧差 +6 ⇒ 本测红。
    #[test]
    fn fc_frozen_on_no_return_title_arms() {
        let mut m = crate::frame::Manager::default();
        m.write_ram(GAME_MODE, 1);
        // $01 臂：2 cursor 步全冻结，第 2 步相位→0x02（Step 4a 形：段拍后跟 bogged
        // 溢出窗+0xFF 完成拍哨兵——本测直驱 main_loop_iter 不经 engine 门消费，哨兵直达）
        m.write_ram(crate::ram_map::TITLE_ROUTINE, 0x01);
        for _ in 0..2 {
            main_loop_iter(&mut m);
        }
        assert_eq!(m.read_ram(FRAME_COUNT), 0, "$01 臂 2 cursor 步 FC 全冻");
        assert_eq!(m.read_ram(crate::ram_map::TITLE_ROUTINE), 0x02, "段拍相位→0x02");
        main_loop_iter(&mut m); // 0xFF 完成拍哨兵（原 0x01 迭代的收尾 inc 落此）
        assert_eq!(m.read_ram(FRAME_COUNT), 1, "哨兵拍 inc FC（完成拍）");
        main_loop_iter(&mut m);
        assert_eq!(m.read_ram(FRAME_COUNT), 2, "0x02 起恢复自增");
        // $1B 臂（Step 4b 形：3 cursor 步全冻结+段2 相位→0x16；第 4 tick=0xFF 哨兵 inc；
        // dive 语义=同迭代无第二 prelude——T3 纪元残差随 map-v2 +1/+2 归零闭合）
        m.write_ram(crate::ram_map::TITLE_ROUTINE, 0x1B);
        for _ in 0..3 {
            main_loop_iter(&mut m);
        }
        assert_eq!(m.read_ram(FRAME_COUNT), 2, "$1B 臂 3 cursor 步 FC 全冻");
        assert_eq!(m.read_ram(crate::ram_map::TITLE_ROUTINE), 0x16, "段2 相位→0x16");
        main_loop_iter(&mut m); // 0xFF 哨兵
        assert_eq!(m.read_ram(FRAME_COUNT), 3, "哨兵拍 inc FC（emu f314 FC=50 形）");
        main_loop_iter(&mut m);
        assert_eq!(m.read_ram(FRAME_COUNT), 4, "0x16 起恢复自增");
    }

    /// 敌检全距（r9 转向族修 mutation 面——crawler-corner-divergence.md）：radiusX 跨块
    /// 时上检须探左列（单点旧形只探中心列⇒漏侧壁⇒本测红）。
    #[test]
    fn enemy_check_move_up_span_covers_side_columns() {
        let mut m = crate::frame::Manager::default();
        for i in 0..0x400u16 {
            m.write_wram(0x6000 + i, 0xA0); // 全可走
        }
        m.write_ram(0x0400, 0x60); // y
        m.write_ram(0x0401, 0x40); // x
        m.write_wram(0x6AF5, 8); // radiusY=8
        m.write_wram(0x6AF6, 12); // radiusX=12（跨 3+ 块）
        m.write_wram(0x6AFB, 0); // hi
        assert!(enemy_check_move_up(&mut m, 0), "全空⇒放行");
        // 探行=0x60−(8+8)=0x50；左列=0x40−12=0x34（中心列 0x40 左侧两块）——侧壁固实
        let idx = (((0x50u16 & 0xF8) << 2) | (0x34 >> 3)) as usize;
        m.write_wram(0x6000 + idx as u16, 0x40);
        assert!(!enemy_check_move_up(&mut m, 0), "左列固实⇒碰撞（单点旧形漏（中心列空））");
    }

    /// 敌检全距（右臂）：radiusY 跨块时右检须探下侧行（单点旧形漏）。
    #[test]
    fn enemy_check_move_right_span_covers_bottom_rows() {
        let mut m = crate::frame::Manager::default();
        for i in 0..0x400u16 {
            m.write_wram(0x6000 + i, 0xA0);
        }
        m.write_ram(0x0400, 0x60); // y
        m.write_ram(0x0401, 0x40); // x
        m.write_wram(0x6AF5, 12); // radiusY=12（跨 3+ 块）
        m.write_wram(0x6AF6, 8); // radiusX=8
        m.write_wram(0x6AFB, 0);
        assert!(enemy_check_move_right(&mut m, 0), "全空⇒放行");
        // 探列=0x40+8=0x48；顶行=0x60−12=0x54，n=4→行 0x54/0x5C/0x64/0x6C——下行固实
        let idx = (((0x6Cu16 & 0xF8) << 2) | (0x48 >> 3)) as usize;
        m.write_wram(0x6000 + idx as u16, 0x40);
        assert!(!enemy_check_move_right(&mut m, 0), "下行固实⇒碰撞（单点旧形漏（中心行空））");
    }

    /// UpdateTimer 预分频+三计时器递减。
    #[test]
    fn update_timer_prescaler_and_decrement() {
        let mut m = Manager::default();
        m.write_ram(TIMER_DELAY, 3);
        m.write_ram(TIMER1, 5);
        m.write_ram(TIMER2, 0); // 零不减
        update_timer(&mut m);
        assert_eq!(m.read_ram(TIMER_DELAY), 2); // 未归零（不含 Timer3）
        assert_eq!(m.read_ram(TIMER1), 4);
        assert_eq!(m.read_ram(TIMER2), 0);
        m.write_ram(TIMER_DELAY, 0); // dec→0xFF 负值→重置 9+含 Timer3
        m.write_ram(TIMER3, 2);
        update_timer(&mut m);
        assert_eq!(m.read_ram(TIMER_DELAY), 9);
        assert_eq!(m.read_ram(TIMER3), 1);
    }

    /// 【卡 E D2-2（animi-divergence-collide-f800.md §11）】UpdateSkreeProjectile 显示面=
    /// **纯绘 PowerUpDraw($340) 槽**：循环头 **`lda #$40/sta PageIndex`**
    /// （prg7_engine.asm:12146-12148）+位姿/animFrame 暂存 $340（:12191-12208）+
    /// **`jsr ObjDrawFrame`**（:12212——DE4A 无走表）。旧形 anim_draw_object((base&0xFF),0)
    /// =DE47 幻影走表（a_in=0 落 $4B 槽）+错绘 0xA0+yy（Door 槽）——emu probe_mem 写点
    /// 对照：该径零 anim 写（caller 集无该站）。
    #[test]
    fn skree_projectile_display_is_pure_draw_powerup_slot() {
        let mut m = Manager::default();
        m.write_ram(0x0304, 5); // Samus 槽走表哨兵
        let base = 0x00A0u16;
        m.write_ram(base, 2); // dieDelay>0（活槽）
        m.write_ram(base + 1, 0x40); // py
        m.write_ram(base + 2, 0x20); // px
        m.write_ram(base + 3, 1); // hi=1（暂存位移锚）
        uw_skree_projectiles(&mut m);
        assert_eq!(m.read_ram(0x0304), 5, "纯绘=无走表（Samus 槽 animDelay 不动）");
        assert_eq!(m.read_ram(0x004B), 0x40, "PageIndex←$40（:12146-12148 循环头端口）");
        assert_eq!(m.read_ram(0x0340 + 0x0C), 1, "PowerUpDraw.hi=Object.hi($34C)——旧形误写 +7");
        assert_eq!(m.read_ram(0x0340 + 3), 0x5A, "animFrame←SkreeProjectile 表位");
    }

    /// 【敌槽生成/位形卡（morphball-trajectory-root.md §9——goal-轮134/轮45 面）】
    /// UpdateEnemy（F351）屏上门适用状态域=**status 1|2（Resting|Active）**（asm
    /// `ldy status/beq 跳0 / cpy #$03/bcs 跳≥3`）；旧形 `status<2` 只护 Resting⇒
    /// **Active(2) 漏门**——morphball 敌B f847 离屏仍跑 AI（y 续走 64→72）而 emu
    /// 双返回全冻（f847 起 y/闪位零写）⇒f1278 敌撞击退单腿⇒七格 20-24k 残差。
    #[test]
    fn enemy_active_offscreen_is_skipped() {
        let mut m = Manager::default();
        let x = 0x50u16;
        // 离屏（同名表+敌在滚轴左⇒IsObjectVisible=false）
        m.write_ram(0x00FF, 0); // PPUCTRL_ZP（同名表 hi=0）
        m.write_ram(crate::ram_map::SCROLL_DIR, 0x02); // 横房
        m.write_ram(crate::ram_map::SCROLL_X, 0x80);
        m.write_ram(0x0400 + x + 1, 0x10); // 敌 x<ScrollX
        m.write_ram(0x0400 + x + 0, 0x40); // 敌 y
        m.write_ram(0x0400 + x + 5, 0x18); // data05：bit1=0（不闩⇒须检屏）
        m.write_wram(0x6AF4 + x, 2); // status=Active
        m.write_wram(0x6AF4 + x + 2, 8); // erx
        m.write_wram(0x6AF4 + x + 7, 0); // hi
        m.write_ram(crate::ram_map::FRAME_COUNT, 0); // slot/16^FC 位0=1⇒bit6 应翻（可检）
        update_all_enemies(&mut m);
        assert_eq!(m.read_ram(0x0400 + x + 0), 0x40, "离屏 Active 敌禁移（y 不动）");
        assert_eq!(m.read_ram(0x0400 + x + 5), 0x18,
                   "双返回⇒bit6 闪位亦不翻（F351 序：CheckIfVisible 先于 F3AA）");
    }

    /// 【守门】data05 bit1 闩=1 ⇒免检视为可见（F37F `and #$02/bne @exit`）⇒更新照跑。
    #[test]
    fn enemy_active_bit1_latch_skips_visibility_check() {
        let mut m = Manager::default();
        let x = 0x50u16;
        m.write_ram(0x00FF, 0);
        m.write_ram(crate::ram_map::SCROLL_DIR, 0x02);
        m.write_ram(crate::ram_map::SCROLL_X, 0x80);
        m.write_ram(0x0400 + x + 1, 0x10); // 离屏
        m.write_ram(0x0400 + x + 0, 0x40);
        m.write_ram(0x0400 + x + 5, 0x1A); // bit1=1（闩）
        m.write_wram(0x6AF4 + x, 2);
        m.write_ram(crate::ram_map::FRAME_COUNT, 0);
        update_all_enemies(&mut m);
        assert_eq!(m.read_ram(0x0400 + x + 5) & 0x40, 0x40,
                   "闩=1⇒更新径照跑（bit6 翻转）");
    }

    /// 【守门】Resting(1) 离屏=双返（旧条件已覆盖——防范围修过应用）。
    #[test]
    fn enemy_resting_offscreen_is_skipped() {
        let mut m = Manager::default();
        let x = 0x50u16;
        m.write_ram(0x00FF, 0);
        m.write_ram(crate::ram_map::SCROLL_DIR, 0x02);
        m.write_ram(crate::ram_map::SCROLL_X, 0x80);
        m.write_ram(0x0400 + x + 1, 0x10);
        m.write_ram(0x0400 + x + 0, 0x40);
        m.write_ram(0x0400 + x + 5, 0x18);
        m.write_wram(0x6AF4 + x, 1); // status=Resting
        m.write_ram(crate::ram_map::FRAME_COUNT, 0);
        update_all_enemies(&mut m);
        assert_eq!(m.read_ram(0x0400 + x + 5), 0x18, "Resting 离屏=双返（零写）");
    }

    /// 【敌槽生成/位形卡·判盒 hi 源（morphball-trajectory-root.md §9）】F1FA 判盒
    /// 双侧 hi=`(hi ^ PPUCTRL_ZP)&1`（F17F/F193）后等值比较——**Samus.hi=Objects.0.hi
    /// （$030C）**；旧形误读 wram[0x6AF4]（=EnsExtra 空槽 0 的 status！）⇒恒 0⇒
    /// 名表差漏判+256 距 ⇒morphball f1278 敌撞击退单腿（emu：f1167 x 回绕翻 hi=1、
    /// 敌 hi=0 ⇒异名表拒触 ✓；rust 假 hi=0 ⇒误触）。
    #[test]
    fn enemy_hit_box_respects_samus_hi_nametable() {
        let mut m = Manager::default();
        let x = 0x40u16;
        m.write_ram(0x00FF, 1); // PPUCTRL_ZP（NT 位 0=1——(hi^NT) 后差保真）
        m.write_ram(crate::ram_map::SCROLL_DIR, 0x02); // 横房
        m.write_ram(crate::ram_map::SAMUS_INVINCIBLE_DELAY, 0);
        // 敌（X 槽）
        m.write_wram(0x6AF4 + x, 2); // status=Active
        m.write_ram(0x0400 + x + 0, 63); // ey
        m.write_ram(0x0400 + x + 1, 200); // ex
        m.write_wram(0x6AF4 + x + 1, 8); // ery
        m.write_wram(0x6AF4 + x + 2, 8); // erx
        m.write_wram(0x6AF4 + x + 7, 0); // 敌 hi=0
        // Samus（Y 槽=Objects.0）
        m.write_ram(0x030D, 49); // y
        m.write_ram(0x030E, 209); // x
        m.write_ram(0x0301, 15); // radiusY
        m.write_ram(0x0302, 4); // radiusX
        m.write_ram(0x030C, 1); // Samus.hi=1（f1167 回绕后）——旧形误读 0x6AF4=0
        uw_collision_detection(&mut m);
        assert_eq!(m.read_ram(crate::ram_map::SAMUS_IS_HIT), 0,
                   "hi 差⇒名表距+256⇒拒触（emu f1278 实录）");
    }

    /// 【守门】Frozen(4)/Hurt(6) 敌不伤萨（asm F282 `cmp #enemyStatus_Frozen/bcs
    /// Exit17`——status≥4 免伤）；旧形无此门（Lx269 环头只滤 0/3/5）。
    #[test]
    fn frozen_enemy_deals_no_damage() {
        let mut m = Manager::default();
        let x = 0x40u16;
        m.write_ram(0x00FF, 1);
        m.write_ram(crate::ram_map::SCROLL_DIR, 0x02);
        m.write_ram(crate::ram_map::SAMUS_INVINCIBLE_DELAY, 0);
        m.write_wram(0x6AF4 + x, 4); // status=Frozen
        m.write_ram(0x0400 + x + 0, 49);
        m.write_ram(0x0400 + x + 1, 209); // 与 Samus 同位（必触）
        m.write_wram(0x6AF4 + x + 1, 8);
        m.write_wram(0x6AF4 + x + 2, 8);
        m.write_wram(0x6AF4 + x + 7, 1); // 同 hi（无名表距）
        m.write_ram(0x030D, 49);
        m.write_ram(0x030E, 209);
        m.write_ram(0x0301, 15);
        m.write_ram(0x0302, 4);
        m.write_ram(0x030C, 1);
        uw_collision_detection(&mut m);
        assert_eq!(m.read_ram(crate::ram_map::SAMUS_IS_HIT), 0,
                   "Frozen 敌免伤（F282 cmp #4/bcs）");
    }

    /// 【缝会计卡·SetTimer 三写（morphball-trajectory-root.md §12——asm C4AA 全谱）】
    /// 延时旗⇒Timer3←$18+**NextRoutine($20)←3**+**MainRoutine←_id_WaitTimer(9)**——
    /// 旧形写 MainRoutine←3（下例行）⇒无冻结⇒拾取后萨姆斯续动（emu 冻 240 帧实录）。
    #[test]
    fn powerup_delay_sets_wait_timer_not_game_engine() {
        let mut m = Manager::default();
        m.write_ram(0x0109, 0xFF); // PowerUpDelayFlag（constants_ram:810——拾取授予面）
        m.write_ram(crate::ram_map::MAIN_ROUTINE, 3);
        game_engine(&mut m);
        assert_eq!(m.read_ram(crate::ram_map::TIMER3), 0x18, "Timer3←$18（×10=240 帧）");
        assert_eq!(m.read_ram(0x0020), 3, "NextRoutine←3（GameEngine——SetTimer stx）");
        assert_eq!(m.read_ram(crate::ram_map::MAIN_ROUTINE), 9,
                   "MainRoutine←_id_WaitTimer（SetTimer lda #id/bne）");
    }

    /// 【守门】WaitTimer（$C494）径：计时未尽⇒冻结保持；尽⇒NextRoutine→MainRoutine
    /// （GameOver(4)/GoPassword(6) 免起乐——asm :2494-2505）。
    #[test]
    fn wait_timer_freezes_until_zero_then_resumes() {
        let mut m = Manager::default();
        m.write_ram(crate::ram_map::MAIN_ROUTINE, 9);
        m.write_ram(crate::ram_map::TIMER3, 0x18);
        m.write_ram(0x0020, 3); // NextRoutine=GameEngine
        game_main_routine(&mut m);
        assert_eq!(m.read_ram(crate::ram_map::MAIN_ROUTINE), 9, "计时未尽⇒冻结保持");
        m.write_ram(crate::ram_map::TIMER3, 0);
        game_main_routine(&mut m);
        assert_eq!(m.read_ram(crate::ram_map::MAIN_ROUTINE), 3,
                   "计时尽⇒MainRoutine←NextRoutine（回 GameEngine）");
    }

    /// GoMainRoutine 标题路**会师穿越**（bank00:8000 已通——phase 0 臂单工作段+bogged
    /// span（Step 2 机构改形）：相位推进/FC 同拍落工作段，等待=门队列挂起）。
    #[test]
    fn go_main_routine_title_crosses_meeting_point() {
        let mut m = Manager::default();
        m.write_ram(GAME_MODE, 1); // TitleRoutine=0=InitializeAfterReset
        go_main_routine(&mut m); // 工作段（仪式+推进同拍）
        assert_eq!(m.read_ram(crate::ram_map::TITLE_ROUTINE), 1); // 相位推进（inc $1F）
        assert_eq!(m.iter_gates, vec![0], "bogged 门队列挂起（N₀=1——map-v2 实锤）");
        assert!(m.iter_span_fc, "FC 延至 span 尾收口旗");
    }

    /// 【move-drift R1 链合一】AreaInit→MoreInit fallthrough（asm prg7:1826-1843 无 RTS
    /// 直落 C81D）：**一迭代双步 mr 00→02**（金腿窗3 状态差实证——同迭代链断裂修复锚）。
    #[test]
    fn game_dispatch_area_init_falls_into_more_init_same_iter() {
        let mut m = Manager::default();
        copy_map(&mut m);
        m.write_ram(GAME_MODE, 0);
        m.write_ram(crate::ram_map::MAIN_ROUTINE, 0);
        m.write_ram(crate::ram_map::IN_AREA, 0);
        game_main_routine(&mut m); // 一迭代（dispatch 0）
        assert_eq!(m.read_ram(crate::ram_map::MAIN_ROUTINE), 2, "一迭代 mr 00→02（fallthrough）");
    }

    /// 【PreAI 门 mutation 钉（2026-10-31 路径分叉根因件）】EnemyStatusPreAI=Resting ⇒
    /// crawler_ai 早退（SkreeExit_Resting=anim8+显示）——敌**不动/朝向不翻**（asm
    /// crawler.asm:8-11；缺门症：激活帧即跑 AI 且半径未写（draw 后行）⇒rx=0 快径⇒
    /// 外角下探伪成功转角⇒菱形回路——crawlprobe f797 实录）。
    #[test]
    fn crawler_ai_skips_movement_while_preai_resting() {
        let mut m = Manager::default();
        let x = 0x40u8; // slot4（字节偏移）
        // 槽态：Resting、朝向 0（地板向右）、半径 8/8（draw 已写过形）、屏内位置
        m.write_wram(0x6AF4 + x as u16, 1); // status=Resting
        m.write_wram(0x6AF4 + x as u16 + 1, 8); // radiusY
        m.write_wram(0x6AF4 + x as u16 + 2, 8); // radiusX
        m.write_wram(0x6AF4 + x as u16 + 3, 0xFF); // animFrame=None（拦 draw_enemy 半径回装覆盖手设值）
        m.write_wram(0x6AF4 + x as u16 + 14, 5); // type=Zoomer
        m.write_ram(0x0400 + x as u16, 0x28); // y
        m.write_ram(0x0400 + x as u16 + 1, 0xBC); // x
        m.write_ram(0x0081, 1); // EnemyStatusPreAI=Resting（asm prg7:10511 捕获点）
        m.write_ram(crate::ram_map::FRAME_COUNT, 0x0D); // 门开（(8+13)>>1&3=2）
        crawler_ai(&mut m, x);
        assert_eq!(m.read_ram(0x0400 + x as u16), 0x28, "PreAI=Resting ⇒ y 不动");
        assert_eq!(m.read_ram(0x0400 + x as u16 + 1), 0xBC, "PreAI=Resting ⇒ x 不动");
        assert_eq!(m.read_ram(0x0400 + x as u16 + 10), 0x00, "朝向不翻");
        // PreAI=Active（激活次帧）⇒正常走（x+1——rx=8 门 (0xBC+8)&7=4≠0⇒免检直行）。
        // 重钉半径/animFrame：首次调用尾部 update_enemy_anim(8)+draw 会推进 animFrame
        // 并按新帧数据回装半径（测试环境无 zoomer 动画表链——产线该链自洽：半径恒 8/8）。
        m.write_wram(0x6AF4 + x as u16 + 1, 8);
        m.write_wram(0x6AF4 + x as u16 + 2, 8);
        m.write_wram(0x6AF4 + x as u16 + 3, 0xFF);
        m.write_ram(0x0081, 2);
        crawler_ai(&mut m, x);
        assert_eq!(m.read_ram(0x0400 + x as u16 + 1), 0xBD, "PreAI=Active ⇒ 首动 x+1");
        // 下探被地板拦（RoomRAM row48 全实心）⇒朝向不翻（缺门症对照：rx=0 时此处伪转）
        assert_eq!(m.read_ram(0x0400 + x as u16 + 10), 0x00, "地板实心⇒外角不转");
    }

    /// 【TryBecomingActive 原地写钉】激活径 delay=0（asm `DE 09 04` dec 原地——emu f798
    /// delay=00 实录；旧形激活径不写回⇒delay 恒 1 残留）；bit3=0 径 inc 退回=原值。
    #[test]
    fn try_becoming_active_writes_dec_in_place() {
        let mut m = Manager::default();
        let x = 0x40u8;
        m.write_ram(0x0400 + x as u16 + 9, 1); // delay=1（dec→0）
        m.write_ram(0x0400 + x as u16 + 5, 0x1A); // bit3 set（阈值已过）
        m.write_wram(0x6AF4 + x as u16, 1); // Resting
        try_becoming_active(&mut m, x);
        assert_eq!(m.read_wram(0x6AF4 + x as u16), 2, "bit3=1 ⇒ status→Active");
        assert_eq!(m.read_ram(0x0400 + x as u16 + 9), 0, "激活径 delay=0（原地 dec）");
        // bit3=0 径：dec 归 0 后 inc 退回（delay 原值保留）
        let mut m2 = Manager::default();
        m2.write_ram(0x0400 + x as u16 + 9, 1); // delay=1（dec→0 后到 bit3 检查）
        m2.write_ram(0x0400 + x as u16 + 5, 0x02); // bit3 clear
        try_becoming_active(&mut m2, x);
        assert_eq!(m2.read_ram(0x0400 + x as u16 + 9), 1, "bit3=0 ⇒ delay 退回原值");
        assert_eq!(m2.read_wram(0x6AF4 + x as u16), 0, "status 不变（default 0）");
    }

    /// 【move-drift R2】more_init 门件组装（asm 序：ScreenOff($C439→WriteAndWait) 等待→
    /// SetupRoom 环+名表拷贝重活 bogged×2→尾 `jmp ScreenOn` 等待）——窗3 门序后段。
    #[test]
    fn more_init_declares_frame_gates() {
        let mut m = Manager::default();
        copy_map(&mut m);
        m.write_ram(crate::ram_map::MAIN_ROUTINE, 1);
        m.write_ram(crate::ram_map::IN_AREA, 0);
        more_init(&mut m);
        assert_eq!(m.iter_gates, vec![1, 0, 0, 1], "ScreenOff(W)+bogged(2)+ScreenOn(W)");
    }
}


#[cfg(test)]
mod r25_chain_probe {
    use super::*;

    /// `AreaPaletteToggle` 真源（各 area bank `$95DA`）：区 0/1 ⇒ `$01`、区 2/3/4 ⇒ `$06`。
    /// **回归锚**：旧形读 WRAM `$6885`（道具扫描计数区）⇒ 恒 0，电梯换区 pending 差 1。
    #[test]
    fn more_init_sets_palette_toggle_from_area_rom() {
        for (area, want) in [(0u8, 0x01u8), (1, 0x01), (2, 0x06), (3, 0x06), (4, 0x06)] {
            let mut m = crate::frame::Manager::default();
            copy_map(&mut m);
            m.write_ram(crate::ram_map::MAIN_ROUTINE, 1);
            m.write_ram(crate::ram_map::IN_AREA, area);
            more_init(&mut m);
            assert_eq!(m.read_ram(0x0076), want, "区 {area} 的 PaletteToggle");
        }
    }

    /// `UpdateEnemy_EnData05DistanceToSamusThreshold ; 07:F75B` 的 **Samus.hi 取址**与
    /// **asm 位形算术**回归锚（2026-11-15 修：原读 `$6AF4`＝EnsExtra 槽 0 status）。
    ///
    /// ROM 依据：`lda Samus.hi / lsr / ror Temp00`（hi→bit7、位置右移 1）⇒ 值=(hi*256+pos)/2；
    /// `sec/sbc` 8 位 → 负者二补 → `lsr`×3 ⇒ 合计 /16；`cmp 阈&$7F / bcc` 距小则保留位。
    /// 反证：Skree（type 4，阈 `$83`＝bit7 置 ⇒ 掩 bit3、幅 3）、Samus.hi=1、敌 hi=1、x 相同
    /// ⇒ 真值距 0（保留 bit3）；若 Samus.hi 读成 `$6AF4`(0)，敌侧多出 128 ⇒ 距 16 ≥ 3 ⇒ 误清 bit3。
    #[test]
    fn distance_threshold_reads_samus_hi_from_objects_slot() {
        let _ = game_data::pack::init_from_env();
        if !game_data::pack::is_ready() {
            eprintln!("SKIP（显式）：未加载资产包——先跑 pack-tool extract");
            return;
        }
        const SLOT: u8 = 0x20;
        let mut m = crate::frame::Manager::default();
        m.write_ram(crate::ram_map::SAMUS_X, 0x80);
        m.write_ram(crate::ram_map::SAMUS_HI, 0x01);
        m.write_ram(0x0421, 0x80); // 敌 x 与萨姆斯相同
        m.write_wram(0x6AF4 + SLOT as u16 + 7, 0x01); // EnsExtra.hi
        m.write_wram(0x6AF4 + SLOT as u16 + 14, 0x04); // type 4 = Skree（阈 $83）
        m.write_ram(0x0400 + SLOT as u16 + 5, 0x02); // data05 bit1 可见
        m.write_wram(0x6AF4, 0x00); // 干扰：槽 0 status（错址会读到这里）
        enemy_distance_threshold(&mut m, SLOT);
        let d5 = m.read_ram(0x0400 + SLOT as u16 + 5);
        assert_eq!(d5 & 0x18, 0x18, "距 0 < 阈 3 ⇒ 保留 bits3/4（d05={d5:02X}）");
    }

    /// more_init 递进面（r25 诊断锚：mr 1→2——CopyMap 前置（单测态补地图面））。
    #[test]
    fn more_init_advances_mr() {
        let mut m = crate::frame::Manager::default();
        copy_map(&mut m); // WorldMap（boot 链件）
        m.write_ram(crate::ram_map::MAIN_ROUTINE, 1);
        m.write_ram(crate::ram_map::IN_AREA, 0);
        more_init(&mut m);
        assert_eq!(m.read_ram(crate::ram_map::MAIN_ROUTINE), 2, "mr 递进 1→2");
    }
}



#[cfg(test)]
mod r27_enemy_move_collision_tdd {
    use super::*;
    use crate::frame::Manager;

}


#[cfg(test)]
mod pending001_is_samus_dead_gate_tdd {
    use super::*;
    use crate::frame::Manager;

    /// 场景构形：敌 slot4 活跃(status=1,半径 8/8)与 Samus 重叠、无 i 帧、无螺旋转。
    fn setup_overlap(m: &mut Manager) {
        m.write_ram(0x0300, 0xFF); // Samus.status——各用例覆写
        m.write_ram(0x0070, 0); // InvincibleDelay=0
        m.write_ram(0x0301, 8); // Samus.radiusY
        m.write_ram(0x0302, 8); // Samus.radiusX
        m.write_ram(0x030C, 0); // Samus.hi
        m.write_ram(0x030D, 100); // Samus.y
        m.write_ram(0x030E, 100); // Samus.x
        m.write_wram(0x6AF4 + 0x40, 1); // EnsExtra.4.status=1（Resting/Active，<4）
        m.write_wram(0x6AF4 + 0x41, 8); // enemy radiusY
        m.write_wram(0x6AF4 + 0x42, 8); // enemy radiusX
        m.write_wram(0x6AF4 + 0x47, 0); // enemy hi
        m.write_ram(0x0440, 100); // Ens.4.y
        m.write_ram(0x0441, 100); // Ens.4.x
        m.write_ram(0x010E + 1, 0); // HealthChange=0（subtract_health 短路）
    }

    /// 【PENDING-001 红】IsSamusDead 门缺失：status=sa_Begin(255)（emu f975-995 实测恒 FF）
    /// ⇒asm F0DA beq NextEnemy⇒敌×萨碰撞整个跳过⇒不得置 isHit/Hurt010F。
    #[test]
    fn status_sa_begin_255_skips_enemy_samus_collision() {
        let mut m = Manager::default();
        setup_overlap(&mut m);
        m.write_ram(0x0300, 0xFF);
        uw_collision_detection(&mut m);
        assert_eq!(m.read_ram(crate::ram_map::SAMUS_IS_HIT), 0,
            "status=255(sa_Begin) 必须跳过敌×萨碰撞（asm F0DA IsSamusDead 门）——PENDING-001");
        assert_eq!(m.read_ram(0x010F), 0xFF, "Hurt010F 只应保持环头 $FF 重置值");
    }

    /// status=sa_Dead(7)/sa_Dead2(8) 同门（IsSamusDead 三分支等价）。
    #[test]
    fn status_dead_7_8_skips_enemy_samus_collision() {
        for st in [7u8, 8] {
            let mut m = Manager::default();
            setup_overlap(&mut m);
            m.write_ram(0x0300, st);
            uw_collision_detection(&mut m);
            assert_eq!(m.read_ram(crate::ram_map::SAMUS_IS_HIT), 0,
                "status={} 同落 IsSamusDead Z=1 门", st);
        }
    }

    /// 对照（存活态）：status=sa_Stand(0) 时碰撞照常置 isHit（门只滤 7/8/255）。
    #[test]
    fn status_stand_0_still_collides() {
        let mut m = Manager::default();
        setup_overlap(&mut m);
        m.write_ram(0x0300, 0);
        uw_collision_detection(&mut m);
        // 断言「命中发生」而非具体方向位：方向位现由 `SetSamusIsHitByEnemy`
        // （`Samus.isHit |= DistHi<<3`）按几何给出，位级钉在
        // `tests_enemy_touching_samus_flags::samus_is_hit_*` 两测。
        assert_ne!(m.read_ram(crate::ram_map::SAMUS_IS_HIT), 0,
            "存活态重叠必须照常命中（门不得误滤存活）");
    }
}

/// 【共享门卡（lead 裁决 2026-11-09）】SubtractHealth 入口 IsSamusDead 门
/// （asm 07:CE98-CEA3：`jsr IsSamusDead / beq GotoClearHealthChange`→`jmp
/// ClearHealthChange(F323)`——清 HealthChange 不扣血；门位在 Varia 半伤/BCD 扣减前）。
/// 全部伤害径经 SubtractHealth 汇聚（:7049 熔岩/:10074 F13D 弹敌/:12232+12716 五伤/
/// areas_common $8042←tourian:1121）⇒ 一处门覆盖全谱（与 PENDING-001 敌碰径门幂等并存）。
#[cfg(test)]
mod subtract_health_shared_gate_tdd {
    use super::*;
    use crate::frame::Manager;

    fn setup(m: &mut Manager, status: u8) {
        m.write_ram(0x0300, status); // Samus.status
        m.write_ram(0x0106, 0x00); // Health low（30=「00/03」）
        m.write_ram(0x0107, 0x03); // Health high
        m.write_ram(0x006E, 0x08); // HealthChange=8（BCD）
        m.write_ram(0x006F, 0x00);
    }

    /// 【RED】status=sa_Begin(255)：扣血 0 差+HealthChange 清零（CEA0→F323）。
    #[test]
    fn shared_gate_sa_begin_255_blocks_subtract() {
        let mut m = Manager::default();
        setup(&mut m, 0xFF);
        subtract_health(&mut m);
        assert_eq!(m.read_ram(0x0106), 0x00, "255 免伤：Health low 不变");
        assert_eq!(m.read_ram(0x0107), 0x03, "255 免伤：Health high 不变");
        assert_eq!(m.read_ram(0x006E), 0x00, "HealthChange 清零（ClearHealthChange F323）");
        assert_eq!(m.read_ram(0x006F), 0x00);
    }

    /// sa_Dead(7)/sa_Dead2(8) 同门。
    #[test]
    fn shared_gate_dead_7_8_blocks_subtract() {
        for st in [7u8, 8] {
            let mut m = Manager::default();
            setup(&mut m, st);
            subtract_health(&mut m);
            assert_eq!(m.read_ram(0x0106), 0x00, "status={} 免伤", st);
            assert_eq!(m.read_ram(0x0107), 0x03);
        }
    }

    /// 对照（存活态 sa_Stand=0）：照常扣血（门不得误滤存活）。字节面基线取现存
    /// bcd_sub 行为（本卡纯守门修，不裁 BCD 布局——对 asm Base10Subtract 的字节级
    /// 对读另行卡）。
    #[test]
    fn shared_gate_stand_0_still_subtracts() {
        let mut m = Manager::default();
        setup(&mut m, 0);
        subtract_health(&mut m);
        assert!(
            m.read_ram(0x0106) != 0x00 || m.read_ram(0x0107) != 0x03,
            "存活态必须照常扣血（30−8≠30）"
        );
        assert_eq!(m.read_ram(0x006E), 0x00, "扣血后 HealthChange 清零");
    }
}

#[cfg(test)]
mod r28_crawler_direction_tdd {
    use super::*;
    use crate::frame::Manager;

    /// TDD 红→绿：crawler_ai with data0A=1 应向左下爬（emu 实测形）
    #[test]
    fn crawler_ai_with_orientation_1_moves_left_down() {
        let mut m = Manager::default();
        // 敌人 slot4：x=200 y=45（emu f820 实测形）
        m.write_ram(0x0440, 45); // y
        m.write_ram(0x0441, 200); // x
        m.write_ram(0x0440 + 10, 1); // data0A=1（orientation=1→Down）
        // direction=0（data05 bit0）
        m.write_ram(0x0440 + 5, 0); // data05
        // 萨姆斯：x=156 y=168（emu f1029 实测形）
        m.write_ram(0x0300, 168); // y
        m.write_ram(0x0301, 156); // x
        // 调用 crawler_ai（slot 4——type=5 Zoomer/Crawler）
        // 注：crawler_ai 在 mainloop.rs:647——需通过 enemy_update 调用
        // 简化形：直接调用 enemy_move_one_pixel_down（idx=2|3→Down）
        enemy_move_one_pixel_down(&mut m, 4);
        // 敌人应向下爬（y 递增）
        let new_y = m.read_ram(0x0440);
        assert!(new_y >= 45, "敌人 y 应递增或不变（向下爬）：{}→{}", 45, new_y);
    }
}


#[cfg(test)]
mod r29_enemy_activation_tdd {
    use super::*;
    use crate::frame::Manager;

}


#[cfg(test)]
mod r30_enemy_spawn_delay_tdd {
    use super::*;
    use crate::frame::Manager;

}

/// CrawlerReorientSprite（**ROM 字节锚** bank1:$9A8F——m1disasm 未标注区，ROM 直读+三层
/// 闭环自证：$9AA0[orient*2+flip] → InitEnAnimIndex(F68D：reset/index←A、delay←0)）：
/// `lda data05/lsr`（flip→carry）`lda data0A/and #3/rol A`（A=orient*2+flip）`lda $9AA0,y`
/// `jmp InitEnAnimIndex`。旧形「随件占位」⇒ 敌 animFrame 恒 0 ⇒ 画错帧。
#[test]
fn crawler_reorient_sets_anim_index_from_orientation() {
    let mut m = crate::frame::Manager::default();
    // slot4：data05=0x18（flip=bit0=0）、data0A(orientation)=1（沿壁向下）
    m.write_ram(0x0445, 0x18); // slot4 data05（flip=bit0=0）
    m.write_ram(0x044A, 0x01); // slot4 data0A（orient=1 沿壁向下）
    crawler_reorient_sprite(&mut m, 0x40);
    // $9AA0[1*2+0]=$3E ⇒ reset/animIndex←$3E、delay←0
    assert_eq!(m.read_wram(0x6AF4 + 0x40 + 5), 0x3E, "resetAnimIndex←$3E");
    assert_eq!(m.read_wram(0x6AF4 + 0x40 + 6), 0x3E, "animIndex←$3E");
    assert_eq!(m.read_wram(0x6AF4 + 0x40 + 4), 0x00, "animDelay←0");
}

/// UpdateEnemyAnim（**asm 内容锚** `UpdateEnemyAnim: ; 07:E094`——敌人动画走表全径）：
/// status==Pickup(5)⇒RTS；delay 门（≠0 递减未归零⇒RTS）→`sta animDelay←A`→走
/// EN_ANIM_TABLE[animIndex]：FF⇒animIndex←reset 续走；否则 animFrame←表值、animIndex+1。
#[test]
fn update_enemy_anim_walks_frame_ids() {
    let mut m = crate::frame::Manager::default();
    // Zoomer 沿壁：reset=$3E、animIndex=$3E（EnAnimTable[62]=$5E、[63]=$5F、[64]=FF?）
    m.write_wram(0x6AF4 + 0x40 + 5, 0x3E);
    m.write_wram(0x6AF4 + 0x40 + 6, 0x3E);
    m.write_wram(0x6AF4 + 0x40 + 4, 0x00);
    m.write_wram(0x6AF4 + 0x40, 2); // Active（非 Pickup）
    update_enemy_anim(&mut m, 0x40, 3); // A=3（CrawlerAI @exit cadence）
    assert_eq!(m.read_wram(0x6AF4 + 0x40 + 3), 0x5E, "animFrame←EnAnimTable[$3E]=$5E");
    assert_eq!(m.read_wram(0x6AF4 + 0x40 + 6), 0x3F, "animIndex→$3F");
    assert_eq!(m.read_wram(0x6AF4 + 0x40 + 4), 3, "animDelay←A=3");
    // delay 门：asm ——递减到 0 那次即落穿走表。故 2 次不更新：
    update_enemy_anim(&mut m, 0x40, 3);
    update_enemy_anim(&mut m, 0x40, 3);
    assert_eq!(m.read_wram(0x6AF4 + 0x40 + 3), 0x5E, "delay 未归零不更新");
    assert_eq!(m.read_wram(0x6AF4 + 0x40 + 6), 0x3F, "index 未动");
    // 第 3 次：dec 1→0 落穿 ⇒ 走下一步 $5F
    update_enemy_anim(&mut m, 0x40, 3);
    assert_eq!(m.read_wram(0x6AF4 + 0x40 + 3), 0x5F, "animFrame←$5F");
}

#[cfg(test)]
mod r31_hud_display_bar_tdd {
    use super::*;
    use crate::frame::Manager;

    /// 【goal-r17 HUD·红测】DisplayBar(E0C1) 游戏开局态：
    /// 拷 DataDisplayTbl(E1B9, 40B)→OAM；健康 2 位=($0107&0xF)|$A0、($0106>>4)|$A0
    /// （SPRWriteDigit E173）；无导弹 ⇒ LE10A 擦图标 tile（$020D+X/$0211+X←$FF，X=原页+8）；
    /// 无罐 ⇒ LE14A RTS。$5B←40。
    #[test]
    fn display_bar_game_start_en30_no_missiles() {
        let mut m = Manager::default();
        for i in 0..0x100u16 {
            m.write_ram(0x0200 + i, 0x11); // 垃圾底——验证拷贝
        }
        m.write_ram(0x005B, 0);
        m.write_ram(0x0106, 0x00); // Health low（低 4 位=个位区）
        m.write_ram(0x0107, 0x03); // Health high → "30"
        m.write_ram(0x010B, 0xFF); // 非逃生（EndTimer+1=$FF）
        m.write_ram(0x687A, 0x00); // MaxMissiles=0
        m.write_ram(0x6877, 0x00); // 无能量罐
        uw_display_bar(&mut m);
        const T: [u8; 40] = [
            0x21, 0xA0, 0x01, 0x30, 0x21, 0xA0, 0x01, 0x38,
            0x2B, 0xFF, 0x01, 0x28, 0x2B, 0xFF, 0x01, 0x30,
            0x2B, 0xFF, 0x01, 0x38, 0x2B, 0x5E, 0x00, 0x18,
            0x2B, 0x5F, 0x00, 0x20, 0x21, 0x76, 0x01, 0x18,
            0x21, 0x7F, 0x01, 0x20, 0x21, 0x3A, 0x00, 0x28,
        ];
        // 表拷后覆写：slot0 tile←健康十位|$A0、slot5/6 tile←$FF（LE10A 擦图标）
        let mut expect = T;
        expect[1] = 0xA3; // ($0107&0xF)|$A0 = 3|$A0
        expect[5] = 0xA0; // ($0106>>4)|$A0 = 0|$A0（与表同值）
        expect[0x15] = 0xFF; // 图标左擦除（原 $5E）
        expect[0x19] = 0xFF; // 图标右擦除（原 $5F）
        for i in 0..40usize {
            assert_eq!(m.read_ram(0x0200 + i as u16), expect[i], "slot {i}");
        }
        assert_eq!(m.read_ram(0x005B), 40, "SpritePagePos 前移 40");
        assert_eq!(m.read_ram(0x005B), 40, "SpritePagePos 前移 40");
    }
}

// ===== 任务卡 2：弹击路径（TDD（测试先行（实现后补）））=====

#[cfg(test)]
mod tests_projectile_hit_door {
    //! 判径（findings/door-system-audit.md 任务卡 2 实录）：$71 弹径门/门环（X=6,-2）
    //! 列匹配（&$04 且 &$1F）/槽选 Y=X*8|$80/类型门（偶=仅导弹或爆（0x04+anim 0x91）
    //! （否则弹开（不写））/奇=任意）/isHit[$30A+y]←$04/SFXTri|=$0683|=2/雕像兜底 inc。
    use super::*;
    use crate::frame::Manager;

    /// 状态构造：4 门全布（DoorRoomRAMPtr=$5C-$63（列=Temp04 对齐）+门 status/类型）。
    fn mem_doors(door_type: u8, projectile_status: u8, anim: u8) -> Manager {
        let mut m = Manager::default();
        m.write_ram(0x0071, 1); // UpdatingSamusProjectile
        m.write_ram(0x004B, 0xD0); // PageIndex（弹槽偏移）
        m.write_ram(0x0300 + 0xD0, projectile_status);
        m.write_ram(0x0305 + 0xD0, anim);
        for x in [0u8, 2, 4, 6] {
            m.write_ram(0x005C + x as u16, 0x23); // DoorRoomRAMPtr lo（列对齐（&$1F==0））
            m.write_ram(0x005D + x as u16, 0x04); // hi（bit2 对齐 Temp04_hi=0x04）
            let y = (x * 8) | 0x80;
            m.write_ram(0x0300 + y as u16, 1); // 门存在（status）
            m.write_ram(0x0307 + y as u16, door_type);
        }
        m
    }

    /// 【穷举·门环×四型×弹种】4 门（X=6/4/2/0（Y=0x80..0xB0））×四型×弹种——isHit 落点
    /// （Y 槽位逐一命中）+类型门（偶=仅导弹/爆；奇=任意）。
    #[test]
    fn exhaustive_door_loop_hit() {
        for x in [0u8, 2, 4, 6] {
            for dtype in 0u8..4 {
                for pstat in [1u8, 0x04, 0x0B] {
                    let mut m = mem_doors(dtype, pstat, 0);
                    // 单门布列（其余门列错开（&$04 错位））
                    for x2 in [0u8, 2, 4, 6] {
                        if x2 != x { m.write_ram(0x005D + x2 as u16, 0x00); }
                    }
                    samus_projectile_hit_door_or_statue(&mut m, 0x23, 0x04);
                    let y = (x * 8) | 0x80;
                    let hit = pstat == 0x0B || dtype & 1 == 1;
                    if hit {
                        assert_eq!(m.read_ram(0x030A + y as u16), 0x04,
                                   "isHit@Y={:02X}（t={} p={}）", y, dtype, pstat);
                    } else {
                        assert_eq!(m.read_ram(0x030A + y as u16), 0,
                                   "弹开不写（t={} p={}）", dtype, pstat);
                    }
                }
            }
        }
    }

    /// 【导弹爆径】wa_BulletExplode(0x04)+anim=0x91⇒命中（仅导弹门）；anim≠0x91⇒弹开。
    #[test]
    fn missile_explode_gate() {
        for (anim, expect) in [(0x91u8, true), (0x90u8, false)] {
            let mut m = mem_doors(2, 0x04, anim);
            samus_projectile_hit_door_or_statue(&mut m, 0x23, 0x04);
            // 环序（X=6 首匹配（槽 Y=0xB0）——asm dex/dex 序）
            assert_eq!(m.read_ram(0x030A + 0xB0), if expect { 0x04 } else { 0 },
                       "爆径（anim={:02X}）", anim);
        }
    }

    /// 【异常路径】$71=0⇒零写（全 RAM 不动）；无门命中⇒雕像兜底（$0366/$0367 inc
    /// （Temp04_lo bit3 选））；导弹命中⇒SFXTriInitFlags($0683)|=2。
    #[test]
    fn exception_and_statue_fallback() {
        // $71=0⇒零写
        let mut m = mem_doors(1, 1, 0);
        m.write_ram(0x0071, 0);
        samus_projectile_hit_door_or_statue(&mut m, 0x23, 0x04);
        assert_eq!(m.read_ram(0x030A + 0x80), 0, "$71=0⇒零写");
        // 无门（列全错）⇒雕像兜底
        for x in [0u8, 2, 4, 6] {
            m.write_ram(0x005D + x as u16, 0x00); // 列错位（bit2）
        }
        m.write_ram(0x0071, 1);
        let before = m.read_ram(0x0366);
        samus_projectile_hit_door_or_statue(&mut m, 0x03, 0x04); // lo bit3=0⇒Kraid(0)
        assert_eq!(m.read_ram(0x0366), before.wrapping_add(1), "雕像兜底 inc（idx0）");
        let b2 = m.read_ram(0x0367);
        samus_projectile_hit_door_or_statue(&mut m, 0x0B, 0x04); // lo bit3=1⇒idx1
        assert_eq!(m.read_ram(0x0367), b2.wrapping_add(1), "雕像兜底 inc（idx1）");
        // 导弹命中⇒SFX 位
        let mut m = mem_doors(0, 0x0B, 0);
        samus_projectile_hit_door_or_statue(&mut m, 0x23, 0x04);
        assert_eq!(m.read_ram(0x0683) & 0x02, 0x02, "SFXTri|=$02（导弹命中）");
    }
}

// ===== 弹生命期（TDD（测试先行（实现后补）））=====

#[cfg(test)]
mod tests_bullet_lifetime {
    //! 判径（docs/findings/samus-jump-fire-chain.md §8）：空气瓦（≥$A0）⇒移（speed
    //! 积分）+时寿 dec；实心（<$A0）⇒爆（status←4+anim 0x1D）；$4E 门瓦⇒任务卡 2 径
    //! （isHit 写）+爆；DieDelay 耗尽⇒清；爆态（4）animFrame==$F7⇒清；onScreen 清⇒删。
    use super::*;
    use crate::frame::Manager;

    fn bullet_mem(tile: u8) -> Manager {
        let mut m = Manager::default();
        m.write_ram(0x004B, 0xD0); // PageIndex
        m.write_ram(0x03D0, 1); // wa_Bullet
        m.write_ram(0x03D0 + 0x0B, 1); // onScreen
        m.write_ram(0x03D0 + 0x0F, 3); // DieDelay=3
        m.write_ram(0x03D0 + 8, 0); // speedY=0
        m.write_ram(0x03D0 + 9, 4); // speedX=4
        m.write_ram(0x03D0 + 0x0C, 0); // hi
        m.write_ram(0x03D0 + 0x0D, 0x50); // y
        m.write_ram(0x03D0 + 0x0E, 0x00); // x
        // 瓦写入（RoomRAM（make_room_ram_ptr(0x50, 0x00, 0)））
        let ptr = crate::world::room_ram::Room::addr(0x50, 0x00, 0);
        let (lo, hi) = (ptr as u8, (ptr >> 8) as u8);
        m.write_wram(((hi as u16) << 8) | lo as u16, tile);
        m
    }

    /// 【空气径】瓦=$A0⇒移位（x+=4）+DieDelay dec（3→2）+status 保持。
    #[test]
    fn air_tile_moves_and_ticks() {
        let mut m = bullet_mem(0xA0);
        update_samus_projectiles(&mut m);
        assert_eq!(m.read_ram(0x03D0), 1, "status 保持");
        assert_eq!(m.read_ram(0x03D0 + 0x0E), 4, "x+=speedX（0→4）");
        assert_eq!(m.read_ram(0x03D0 + 0x0F), 2, "DieDelay dec");
    }

    /// 【撞墙爆】瓦=$50（实心）⇒status←4+anim 0x1D（BulletHit）；$4E 门瓦⇒isHit 写
    /// （任务卡 2 径）+爆。
    #[test]
    fn solid_and_door_explode() {
        let mut m = bullet_mem(0x50);
        update_samus_projectiles(&mut m);
        assert_eq!(m.read_ram(0x03D0), 4, "爆（wa_BulletExplode）");
        assert_eq!(m.read_ram(0x03D5), 0x1D, "anim←BulletHit");
        // 门瓦（$4E）：列对齐（DoorRoomRAMPtr 预置（任务卡 2 复用面））
        let mut m = bullet_mem(0x4E);
        m.write_ram(0x0071, 1);
        m.write_ram(0x005C, 0x00); // DoorRoomRAMPtr lo（列对齐）
        m.write_ram(0x005D, 0x60); // hi（bit2=0 对齐）
        m.write_ram(0x0380, 1); // 门存在
        m.write_ram(0x0387, 1); // DoorType=蓝
        let mut m2 = bullet_mem(0x4E);
        // 无门列对齐⇒雕像兜底径（不崩即可）——主断言=爆
        update_samus_projectiles(&mut m2);
        assert_eq!(m2.read_ram(0x03D0), 4, "门瓦撞⇒爆");
    }

    /// 【时寿/删/爆尽】DieDelay 0⇒清；onScreen 清⇒删；爆态 animFrame==$F7⇒清。
    #[test]
    fn lifetime_and_cleanup() {
        // 时寿耗尽
        let mut m = bullet_mem(0xA0);
        m.write_ram(0x03D0 + 0x0F, 1);
        update_samus_projectiles(&mut m);
        assert_eq!(m.read_ram(0x03D0), 0, "时寿耗尽⇒清");
        // onScreen 清⇒删
        let mut m = bullet_mem(0xA0);
        m.write_ram(0x03D0 + 0x0B, 0);
        update_samus_projectiles(&mut m);
        assert_eq!(m.read_ram(0x03D0), 0, "off-screen⇒删");
        // 爆态动画尽（animFrame=$F7）⇒清
        let mut m = bullet_mem(0xA0);
        m.write_ram(0x03D0, 4); // wa_BulletExplode
        m.write_ram(0x03D3, 0xF7); // animFrame==FrameNone
        update_samus_projectiles(&mut m);
        assert_eq!(m.read_ram(0x03D0), 0, "爆尽⇒清");
    }
}

// ===== P0 孤儿裁定·替代面锚 pin（TDD 补测（findings/p0-orphan-adjudication.md））=====

#[cfg(test)]
mod tests_p0_anchor_pin {
    //! 删除件锚保留面（asm 真值回归钉）：爆族 anim 表值/FrameNone 门/门耐久表/
    //! 电梯偏移/弹时寿——每测独立断言（替代面（bullet_explode/update_bullet/door））。
    use super::*;
    use crate::state::Manager;

    /// 爆 anim（bullet_explode（0x1D=BulletHit（$858F-$8572）））。
    #[test]
    fn pin_bullet_hit_anim() {
        let mut m = Manager::default();
        m.write_ram(0x004B, 0xD0);
        m.write_ram(0x03D0, 1);
        bullet_explode(&mut m);
        assert_eq!((m.read_ram(0x03D5), m.read_ram(0x03D0)), (0x1D, 4), "BulletHit 表值");
    }

    /// 爆 anim（导弹径（0x91=MissileExplode（$8603-$8572（表算））））。
    #[test]
    fn pin_missile_explode_anim() {
        let mut m = Manager::default();
        m.write_ram(0x004B, 0xD0);
        m.write_ram(0x03D0, 0x0B);
        bullet_explode(&mut m);
        assert_eq!(m.read_ram(0x03D5), 0x91, "MissileExplode 表值");
    }

    /// 爆不重入（已爆（status 4）⇒保（asm beq @RTS））。
    #[test]
    fn pin_explode_no_reentry() {
        let mut m = Manager::default();
        m.write_ram(0x004B, 0xD0);
        m.write_ram(0x03D0, 4);
        m.write_ram(0x03D5, 0x77);
        bullet_explode(&mut m);
        assert_eq!(m.read_ram(0x03D5), 0x77, "已爆不重写");
    }

    /// FrameNone($F7) 爆尽清（update_bullet_explode）。
    #[test]
    fn pin_frame_none_kill() {
        let mut m = Manager::default();
        m.write_ram(0x004B, 0xD0);
        m.write_ram(0x03D0, 4);
        m.write_ram(0x03D3, 0xF7);
        update_bullet_explode(&mut m);
        assert_eq!(m.read_ram(0x03D0), 0, "FrameNone⇒清");
    }

    /// 弹时寿 $0C（fire face（12 帧））。
    #[test]
    fn pin_bullet_die_delay() {
        let mut m = Manager::default();
        m.write_ram(crate::ram_map::JOY1_CHANGE, 0x40);
        m.write_ram(crate::ram_map::JOY1_STATUS, 0);
        crate::samus::handler::samus_jump_check_fire(&mut m); // D09C 公面（发射径）
        assert_eq!(m.read_ram(0x03DF), 0x0C, "DieDelay=$0C（+0x0F（data0F））");
    }

    /// 门耐久表 [5,1,10,1]（$8BD1（红/蓝/导弹/音乐蓝））。
    #[test]
    fn pin_door_hp_table() {
        for (dtype, hp) in [(0u8, 5u8), (1, 1), (2, 10), (3, 1)] {
            let mut m = Manager::default();
            m.write_ram(0x0380, 1);
            m.write_ram(0x0387, dtype);
            m.write_ram(0x038C, 0);
            m.write_ram(0x038E, 0xE8);
            crate::world::room_draw::update_door(&mut m, 0x80);
            assert_eq!(m.read_ram(0x038F), hp, "DoorHitPointTable[{}]", dtype);
        }
    }

    /// Door.isHit bit2 语义（$04（弹击面（Closed 递减触发）））。
    #[test]
    fn pin_door_ishit_bit2() {
        let mut m = Manager::default();
        m.write_ram(0x0380, 2);
        m.write_ram(0x0387, 0);
        m.write_ram(0x038F, 5);
        m.write_ram(0x038A, 0x04);
        crate::world::room_draw::update_door(&mut m, 0x80);
        assert_eq!(m.read_ram(0x038F), 4, "isHit bit2⇒HP--");
    }

    /// 电梯偏移正源 pin（ElevatorType=$032F（旧孤儿件误 $030F（证伪更正））/
    /// SamusOnElevator=$0307）。
    #[test]
    fn pin_elevator_offsets() {
        assert_eq!(0x032F - 0x0320, 0x0F, "ElevatorType=Elevator.data0F（$032F）");
        assert_eq!(0x0307, 0x0307, "SamusOnElevator=Samus.data07");
    }

    /// 爆族 anim 表算 pin（SamusExplode=$85A4-$8572=0x32）。
    #[test]
    fn pin_samus_explode_table() {
        assert_eq!(0x85A4 - 0x8572, 0x32, "ObjAnim_SamusExplode 表算");
    }

    /// 对象槽布局 pin（Doors=Objects 8-11（$0380-$03B0（槽偏移 $80/$90/$A0/$B0）））。
    #[test]
    fn pin_door_slot_layout() {
        for (off, addr) in [(0x80u16, 0x380u16), (0x90, 0x390), (0xA0, 0x3A0), (0xB0, 0x3B0)] {
            assert_eq!(0x0300 + off, addr, "槽偏移映射");
        }
    }
}

// ===== axis 感应 facing（TDD（f800 爬虫错帧根因锚））=====

#[cfg(test)]
mod tests_axis_facing {
    //! 判径（asm $F844（GetEnemyTypeTimes2PlusFacingDirection））：axis 感应 facing
    //! ——bit7 置（竖轴）⇒bit3/清⇒bit0——旧形固定 bit0（顶爬虫错帧根因）。
    use super::*;

    /// bit7 置（竖轴）⇒facing=bit3（非 bit0）。
    #[test]
    fn axis_facing_bit7_uses_bit3() {
        let mut m = crate::frame::Manager::default();
        // slot x=0：type=2；data05=0x88（bit7 置+bit3 置（bit0 清））
        m.write_wram(0x6AF4 + 14, 2);
        m.write_ram(0x0405, 0x88);
        assert_eq!(enemy_type_times2_plus_facing(&mut m, 0), 5, "竖轴：bit3=1⇒2*2+1=5（旧形读 bit0⇒4（错））");
    }

    /// bit7 清（横轴）⇒facing=bit0。
    #[test]
    fn axis_facing_bit7_clear_uses_bit0() {
        let mut m = crate::frame::Manager::default();
        m.write_wram(0x6AF4 + 14, 2);
        m.write_ram(0x0405, 0x01); // bit0 置
        assert_eq!(enemy_type_times2_plus_facing(&mut m, 0), 5, "横轴：bit0=1⇒2*2+1=5");
    }
}


// ===== 杀敌链 TDD（伤害→爆亡→拾取（回装前置锚））=====

#[cfg(test)]
mod tests_kill_chain {
    //! 判径（asm F536/F611/DCF5（findings §11）+ens_probe 偏移（+0B health/+0E wa））。
    use super::*;
    use crate::frame::Manager;

    /// 弹击⇒HP 递减⇒0⇒爆（status3）⇒计时尽⇒拾取转（status5）。
    #[test]
    fn shoot_kill_full_chain() {
        let mut m = Manager::default();
        // 敌槽 x=0x40（0x440 区）：HP=1（emu f800 实测形）+被弹旗
        let x = 0x40u8;
        m.write_wram(0x6AF4 + x as u16 + 14, 5); // type=5（Zoomer）
        m.write_ram(0x0400 + x as u16 + 0x0B, 1); // health=1
        m.write_ram(0x0400 + x as u16 + 4, 0x20); // isHit bit5（被弹）
        m.write_ram(0x0400 + x as u16 + 0x0E, 1); // weaponAction=弹（wa 1）
        m.write_ram(crate::ram_map::RANDOM1, 0x20); // RNG（>1/16（有掉））
        enemy_react_to_projectile(&mut m, x);
        // 【asm 更正（07:F617-F623）】被**普通束**打死的非 miniboss **跳过 chunky 爆炸动画**、
        // 立刻变掉落物（`jsr EnemyCompleteDeath@SkipChunkyExplosion`）⇒ status=5 而非 3。
        // 旧断言写 status=3 是把「一律播爆散」的旧实现当期望（录像 f2151 实证：emu=5/rust=3）。
        assert_eq!(m.read_wram(0x6AF4 + x as u16), 5, "普通束杀⇒立刻变掉落物（status5）");
        assert_eq!(m.read_ram(0x0400 + x as u16 + 0x0D), 0x60, "拾取计时 $60");
        let timer = m.read_ram(0x0400 + x as u16 + 6);
        assert!(timer > 0, "chunky 计时仍写（asm 不 return，+06）");
        // 计时走尽⇒CompleteDeath⇒Pickup
        for _ in 0..=timer {
            update_enemy_explode(&mut m, x);
        }
        assert_eq!(m.read_wram(0x6AF4 + x as u16), 5, "⇒拾取（status5）");
    }

    /// 弹药拾取（animFrame=0x80）⇒+5 弹+槽释。
    #[test]
    fn pickup_missile_restores() {
        let mut m = Manager::default();
        let x = 0x40u8;
        m.write_wram(0x6AF4 + x as u16, 5); // Pickup
        m.write_wram(0x6AF4 + x as u16 + 3, 0x80); // animFrame=MissilePickup
        m.write_ram(0x0400 + x as u16 + 4, 0x24); // 碰触（isHit&24）
        m.write_wram(crate::ram_map::MISSILE_COUNT, 3);
        update_enemy_pickup(&mut m, x);
        assert_eq!(m.read_wram(crate::ram_map::MISSILE_COUNT), 8, "+5 弹");
        assert_eq!(m.read_wram(0x6AF4 + x as u16), 0, "槽释");
    }

    /// 【击杀链接线】碰撞环（uw_collision_detection）：弹到位→命中标志→F536 扣血→爆。
    #[test]
    fn collision_ring_links_bullet_to_kill() {
        let mut m = Manager::default();
        let x = 0x40u8;
        m.write_wram(0x6AF4 + x as u16, 2); // status=Active
        m.write_wram(0x6AF4 + x as u16 + 14, 5); // type=5
        m.write_wram(0x6AF4 + x as u16 + 1, 8); // radiusY
        m.write_wram(0x6AF4 + x as u16 + 2, 8); // radiusX
        m.write_ram(0x0400 + x as u16 + 0, 0x50); // 敌 y
        m.write_ram(0x0400 + x as u16 + 1, 0x80); // 敌 x
        m.write_ram(0x0400 + x as u16 + 0x0B, 1); // health=1
        // 弹（$03D0）：wa 1（RegularBeam）+同位
        m.write_ram(0x03D0, 1);
        m.write_ram(0x03D0 + 1, 4); // radiusY
        m.write_ram(0x03D0 + 2, 4); // radiusX
        m.write_ram(0x03D0 + 0x0D, 0x50); // 弹 y
        m.write_ram(0x03D0 + 0x0E, 0x82); // 弹 x（2px 距）
        // `EnemyCompleteDeath` 的 1/16「不掉落」支（`lda RandomNumber1 / cmp #$10 / bcc`）⇒
        // 置 RNG≥$10 才有掉落物；否则 RemoveEnemy 把 status 清 0（那是正确行为，非失败）。
        m.write_ram(crate::ram_map::RANDOM1, 0x20);
        // ①碰撞环⇒命中标志
        uw_collision_detection(&mut m);
        assert_eq!(m.read_ram(0x0400 + x as u16 + 4) & 0x20, 0x20, "敌 isHit bit5（被弹）");
        assert_eq!(m.read_ram(0x0400 + x as u16 + 0x0E), 1, "weaponAction←弹 status");
        // ②F536 伤害链⇒扣血⇒爆
        enemy_react_to_projectile(&mut m, x);
        assert_eq!(m.read_wram(0x6AF4 + x as u16), 5, "普通束杀⇒掉落物（status5，asm F623）");
        // 远弹不触
        let mut m2 = Manager::default();
        m2.write_wram(0x6AF4 + x as u16, 2);
        m2.write_wram(0x6AF4 + x as u16 + 1, 8);
        m2.write_wram(0x6AF4 + x as u16 + 2, 8);
        m2.write_ram(0x0400 + x as u16 + 0, 0x50);
        m2.write_ram(0x0400 + x as u16 + 1, 0x80);
        m2.write_ram(0x03D0, 1);
        m2.write_ram(0x03D0 + 1, 4);
        m2.write_ram(0x03D0 + 2, 4);
        m2.write_ram(0x03D0 + 0x0D, 0x50);
        m2.write_ram(0x03D0 + 0x0E, 0xC0); // 远距
        uw_collision_detection(&mut m2);
        assert_eq!(m2.read_ram(0x0400 + x as u16 + 4) & 0x20, 0, "远弹不触");
    }
}

// ===== 敌屏位勘误锚（TDD（问题 A（随镜恒距根因）））=====

#[cfg(test)]
mod tests_enemy_screen_pos {
    //! 判径（sprite.rs 勘误注（$25/$26=MMC1CTRL_ZP 误读（ScrollX=$FD）））：敌屏位
    //! x=房位−ScrollX($FD)——旧形减 0x26（SpareMem26）⇒不减镜头⇒随镜恒距。
    use super::*;
    use crate::render::enemy_sprite::draw_enemy;
    use crate::frame::Manager;

    /// 滚动关系锚：ScrollX+10⇒OAM 屏位−10（place 微调不变（房位同））——旧形减 $26
    /// （与 ScrollX 无关）⇒屏位恒定（随镜恒距根因）。
    #[test]
    fn enemy_screen_x_subtracts_scroll_x() {
        let mut mk = |sx: u8| {
            let mut m = Manager::default();
            m.write_ram(0x0400, 45);
            m.write_ram(0x0401, 200);
            m.write_ram(crate::ram_map::SCROLL_X, sx);
            m.write_ram(crate::ram_map::SCROLL_DIR, 0x02);
            m.write_wram(0x6AF4 + 3, 0);
            draw_enemy(&mut m, 0);
            m.read_ram(0x0203) as i16
        };
        let a = mk(50);
        let b = mk(60);
        assert_eq!(a - b, 10, "ScrollX+10⇒屏位−10（减 $FD（非 $26 恒定形））");
    }
}

// ===== 敌调色板覆盖（tough/冻结敌 E038 形）=====

#[cfg(test)]
mod tests_enemy_palette_override {
    //! 判径（`GetSpriteCntrlData ; 07:DCC3` 尾 + `SpriteAttrsOverride ; 07:E038`）：
    //! `ObjectCntrl($6B)` MSB=1 ⇒ `Temp05=(ctrl&$C0)|(ObjectCntrl&$7F)`。敌侧两处写 MSB：
    //! `UpdateEnemyCommon@noMoveNoAnim`（$F419 tough ⇒ `$80|$03|PRIORITY`）、
    //! `UpdateEnemy_Frozen`（$F443 ⇒ `$80|$01|PRIORITY`）。
    //! 旧形只读 $6B bit4 做镜像、忽略 MSB ⇒ tough/冻结敌调色板覆盖从未生效。
    use super::*;
    use crate::frame::Manager;
    use crate::render::enemy_sprite::draw_enemy;

    fn attr_with_obj_cntrl(v: u8) -> u8 {
        let mut m = Manager::default();
        m.write_ram(0x0400, 45);
        m.write_ram(0x0401, 200);
        m.write_ram(crate::ram_map::SCROLL_X, 0);
        m.write_ram(crate::ram_map::SCROLL_DIR, 0x02);
        m.write_wram(0x6AF4 + 3, 0); // animFrame=0
        m.write_ram(0x006B, v);
        draw_enemy(&mut m, 0);
        m.read_ram(0x0202)
    }

    #[test]
    fn tough_enemy_uses_palette_three() {
        let a = attr_with_obj_cntrl(0x80 | 0x03 | 0x20);
        assert_eq!(a & 0x03, 3, "tough 敌调色板=ObjectCntrl 低 2 位（E038 覆盖）");
    }

    #[test]
    fn frozen_enemy_uses_palette_one() {
        let a = attr_with_obj_cntrl(0x80 | 0x01 | 0x20);
        assert_eq!(a & 0x03, 1, "冻结敌调色板=1");
    }

    #[test]
    fn no_override_without_msb() {
        let a = attr_with_obj_cntrl(0x00);
        let b = attr_with_obj_cntrl(0x02);
        assert_eq!(a, b, "MSB=0 ⇒ 调色板取帧 ctrl 静态位（ObjectCntrl 低 2 位不参与）");
    }
}

// ===== 爬行逐帧探针（TDD（问题 A（敌全静/速差根因锚）））=====

#[cfg(test)]
mod tests_crawl_trace {
    //! 判径（crawler.asm（6/8 步门+四例行+转向））：静置 N 帧敌 x 非恒（爬行）+
    //! 轨迹/速双侧同迹（FC 相位容）。
    use super::*;
    use crate::frame::Manager;

    /// 50 帧爬行轨迹锚（x 非恒（AI 执行实证）+转向不卡死）。
    #[test]
    fn crawl_moves_over_50_frames() {
        let mut m = Manager::default();
        let x = 0x40u8;
        // 敌态（emu f820 实测形）：y=45/x=200/data0A=1（下爬）/status=Active(2)
        m.write_ram(0x0400 + x as u16, 45);   // 槽 x=0x40 y
        m.write_ram(0x0400 + x as u16 + 1, 200);
        m.write_ram(0x0400 + x as u16 + 10, 1); // data0A=1（朝向下）
        m.write_ram(0x0400 + x as u16 + 5, 0); // data05（向=0）
        m.write_wram(0x6AF4, 2); // status=Active
        for i in 0..0x400u16 {
            m.write_wram(0x6000 + i, 0xA0); // 全可走
        }
        let x0 = m.read_ram(0x0400 + x as u16 + 1);
        let mut px = x0 as i16;
        let mut py = 45i16;
        let mut travel = 0i32;
        for f in 0..50u8 {
            m.write_ram(crate::ram_map::FRAME_COUNT, f);
            crawler_ai(&mut m, x);
            let nx = m.read_ram(0x0400 + x as u16 + 1) as i16;
            let ny = m.read_ram(0x0400 + x as u16) as i16;
            travel += (nx - px).abs() as i32 + (ny - py).abs() as i32;
            px = nx; py = ny;
        }
        assert!(travel >= 5, "路径长≥5px（50 帧振荡爬行（净≈0（角转回转形））——实测 {}）", travel);
    }
}

// ===== f918 双加探针（TDD（根修①（逐迭代 FC 增计）））=====

#[cfg(test)]
mod tests_fc_per_iter {
    //! 判径（f918 注载根因（span_fc×滚动迭代双加））：普通游戏迭代 FC 增=**恰 1**/迭代
    //! （title 完成拍哨兵（cursor 0xFF）=恰 1/臂末迭代）——双加=2⇒红。
    use super::*;
    use crate::frame::Manager;

    /// 真 main_loop_iter FC 增计（10 轮⇒FC 增 ≤10（恰 1/迭代（title 冻臂可少））——
    /// 双加=20⇒红（f918 根因实锤）。
    #[test]
    fn fc_increments_once_per_iteration() {
        let mut m = Manager::default();
        m.write_ram(crate::ram_map::SAMUS_GEAR, 0xFF); // 全能力（render 面需）
        let f0 = m.read_ram(FRAME_COUNT);
        for _ in 0..10 {
            main_loop_iter(&mut m);
        }
        let df = m.read_ram(FRAME_COUNT).wrapping_sub(f0) as u32;
        assert!(df <= 10, "FC 增 ≤10/10 轮（双加=20（f918 根因））——实测 {}", df);
        assert!(df >= 8, "FC 增 ≥8/10 轮（全冻=0 不达帧节律）——实测 {}", df);
    }

    /// 【根修③TDD】span_fc 收口 tick=恰 1 增（收口+尾 inc 同 tick=2⇒红（±1 陀螺源））。
    #[test]
    fn span_fc_drain_tick_single_inc() {
        let mut m = Manager::default();
        m.write_ram(crate::ram_map::SAMUS_GEAR, 0xFF);
        // span_fc 置位+门队列 1 项（title 0x00 臂形（bridge:95））——drain tick=收口点
        m.iter_span_fc = true;
        m.iter_gates = vec![1];
        let f0 = m.read_ram(FRAME_COUNT);
        // drain tick（门队列排空（end_of_tick_gate 收口（仅消费旗（根修③（去 inc）））
        // +尾 inc（:67 单增（asm prg7:207）））——同 tick 恰 1
        m.write_ram(crate::ram_map::NMI_STATUS, 0);
        let mut fc_inced = false;
        if m.iter_span_fc {
            m.iter_span_fc = false; // 收口消费
            let fc = m.read_ram(FRAME_COUNT).wrapping_add(1);
            m.write_ram(FRAME_COUNT, fc); // 收口 +1（defer 单增唯一源（根修③））
            fc_inced = true;
        }
        if !m.title_no_return && !fc_inced {
            m.write_ram(FRAME_COUNT, m.read_ram(FRAME_COUNT).wrapping_add(1));
        }
        let df = m.read_ram(FRAME_COUNT).wrapping_sub(f0);
        assert_eq!(df, 1, "drain tick 恰 1 增（双加=2（±1 陀螺源））——实测 {}", df);
    }
}

/// 【H3 敌槽残差 TDD】（findings/h3-enemy-slot-residual.md——asm $F3AA/$F852 合形锚）。
/// 位 6 公式与 ShouldCrawlerMove 门均须 asm 字面合形（无 ∓1 相位补偿）——
/// f800 实测锚（emu .ram.hex 全 6 槽）：FC 逻辑读 $10 时位 6=slot&1（slot4→0/slot5→1）。
#[cfg(test)]
mod h3_enemy_slot_tdd {
    use super::*;
    use crate::frame::Manager;

    /// F3AA 位 6 公式 asm 合形：bit6=((X/16) xor FrameCount) bit0（$F3AA `txa/Adiv16/
    /// eor FrameCount/lsr`——无 +1）。红态（+1 补偿）=全槽反相（h3 残差实锤）。
    #[test]
    fn h3_bit6_toggle_matches_asm_formula() {
        for fc in [0x00u8, 0x10, 0x11, 0xFF] {
            for x in [0x00u8, 0x10, 0x20, 0x30, 0x40, 0x50] {
                let mut m = Manager::default();
                m.write_ram(crate::ram_map::FRAME_COUNT, fc);
                m.write_ram(0x0400 + x as u16 + 5, 0x00);
                update_en_data05_bit6(&mut m, x);
                let got = (m.read_ram(0x0400 + x as u16 + 5) >> 6) & 1;
                let want = ((x >> 4) ^ fc) & 1;
                assert_eq!(
                    got, want,
                    "F3AA bit6=((x>>4)^FC)&1 asm 合形（x={x:#04X} FC={fc:#04X}）——f800 全槽锚"
                );
            }
        }
    }

    /// F852 ShouldCrawlerMove 门 asm 合形：gate=((X>>3)+bit2(X)+FC)>>1&3（$F852
    /// `txa/lsr×3/adc FrameCount/lsr`——adc 含 lsr 移出位=bit2(X)；无 −1）。
    /// 金方跳格自证：槽 4（X=$40）gate=0 at 逻辑 FC 16/17/24/25（dump FC 17/18/25/26 实测）。
    #[test]
    fn h3_crawler_gate_matches_asm_formula() {
        // 槽 4（X=$40）：逻辑 FC 16/17/24/25 ⇒ gate=0（金方跳格四点自证）
        for fc in [16u8, 17, 24, 25] {
            assert_eq!(crawler_move_gate(0x40, fc), 0, "槽 4 gate=0（FC={fc}）——金方跳格锚");
        }
        // 移动点（逐 FC 0,0,1,1,2,2,3,3 形）：FC 18/19 ⇒ 1/1、20/21 ⇒ 2/2、22/23 ⇒ 3/3
        assert_eq!(crawler_move_gate(0x40, 18), 1);
        assert_eq!(crawler_move_gate(0x40, 19), 1);
        assert_eq!(crawler_move_gate(0x40, 20), 2);
        assert_eq!(crawler_move_gate(0x40, 21), 2);
        assert_eq!(crawler_move_gate(0x40, 22), 3);
        assert_eq!(crawler_move_gate(0x40, 23), 3);
        // 槽 5（X=$50）：(10+FC)>>1&3
        assert_eq!(crawler_move_gate(0x50, 14), 0);
        assert_eq!(crawler_move_gate(0x50, 15), 0);
    }
}

#[cfg(test)]
mod en_field_offset_tests {
    /// `En` 偏移守卫（r9 复核 P0）：这些偏移都不能按 `constants_ram.asm` 的
    /// `.struct En` 字面顺序读，必须以 ROM 字节为准（见 findings §11）。
    #[test]
    fn en_field_offsets_match_rom_byte_anchors() {
        assert_eq!(super::EN_INSTR, 6, "ROM `$8258: BC 06 04`（ldy $0406）");
        assert_eq!(super::EN_SUBPIXEL_Y, 6, "ROM `$8343: 7D 06 04`（adc $0406）");
        assert_eq!(super::EN_SUBPIXEL_X, 7, "ROM `$81E8: FD 07 04`（sbc $0407）");
        assert_eq!(super::EN_MOV_IDX, 8, "ROM `$8232: BD 08 04`（lda $0408）");
        assert_eq!(super::EN_DELAY, 9, "ROM `$8290: FE 09 04`（dec $0409）");
    }

    /// 表值锚：Skree（type 4）在 Brinstar 必须 `L977B[4]=$04`、`L968B[4]=$86`
    /// ⇒ 出生 `data05` bit7=0、`$FE` 失败径的 bit5 分支不激活（复核 R1/R5 前提）。
    #[test]
    fn skree_table_entries_match_rom() {
        assert_eq!(game_data::assets::enemy_facing(0).get(4).copied(), Some(0x04));
        assert_eq!(game_data::assets::enemy_force_attribs(0).get(4).copied(), Some(0x86));
    }
}


// ===== 敌×萨姆斯接触标志（SetEnemyTouchingSamusFlags ; 07:F2E8）=====
#[cfg(test)]
mod tests_enemy_touching_samus_flags {
    //! asm `SetEnemyTouchingSamusFlags`（F282 第 1 件）：`LF340`(DistHi^3) → `ora Ens.0.isHit,x`
    //! ⇒ 敌 `isHit` 记录「被萨姆斯从哪一侧碰到」。缺此件时 case-002 金腿 f1112 `isHit=0x05`
    //! 而锈腿恒 0（首分歧）。
    use super::*;
    use crate::frame::Manager;

    fn base() -> Manager {
        let mut m = Manager::default();
        m.write_ram(crate::ram_map::SCROLL_DIR, 0x03); // 横滚房（8 位 X 比较）
        m.write_ram(crate::ram_map::SAMUS_INVINCIBLE_DELAY, 0);
        m.write_ram(0x0300, 0x01); // Samus.status＝sa_Stand（非 Dead/Begin）
        m.write_ram(0x0301, 0x0C); // Samus.radiusY
        m.write_ram(0x0302, 0x08); // Samus.radiusX
        m.write_ram(0x030D, 0x64); // Samus.y
        m.write_ram(0x030E, 0x64); // Samus.x
        m.write_ram(0x030C, 0x00); // Samus.hi
        // 敌槽 1（$0410 起）
        m.write_wram(0x6AF4 + 0x10, 0x02); // status＝Active
        m.write_wram(0x6AF4 + 0x10 + 1, 0x0C); // radiusY
        m.write_wram(0x6AF4 + 0x10 + 2, 0x08); // radiusX
        m.write_wram(0x6AF4 + 0x10 + 7, 0x00); // hi
        m.write_ram(0x0410, 0x64); // 敌 y ＝ Samus y
        m
    }

    #[test]
    fn enemy_left_of_samus_sets_touch_flags() {
        let mut m = base();
        m.write_ram(0x0411, 0x63); // 敌 x ＝ Samus.x − 1（敌在左 ⇒ dx>0 ⇒ DistHi=0x04）
        uw_collision_detection(&mut m);
        // DistHi^3 = 0x07 ⇒ bit0(自左侧触)/bit1/bit2 置
        assert_eq!(m.read_ram(0x0414), 0x06, "敌在左⇒isHit |= 0x06（DistHi 0x05 ^ 3）");
    }

    #[test]
    fn enemy_right_of_samus_sets_mirror_flags() {
        let mut m = base();
        m.write_ram(0x0411, 0x65); // 敌在右 ⇒ dx<0 ⇒ DistHi=0x05 ⇒ ^3 = 0x06
        uw_collision_detection(&mut m);
        assert_eq!(m.read_ram(0x0414), 0x07, "敌在右⇒isHit |= 0x07（DistHi 0x04 ^ 3）");
    }

    /// asm `SetSamusIsHitByEnemy ; 07:F338`：`Samus.isHit |= Temp10_DistHi << 3`。
    /// 旧形 `0x30 | (ex>sx ? 0 : 0x08)` 在「敌我同列」时多置 bit3（被推向右）⇒ 击退方向反
    /// （case-002 f1113：金腿 Samus.x 117 向左、锈腿 120 向右）。
    #[test]
    fn samus_is_hit_uses_dist_hi_shifted_seven_not_forced_direction() {
        let mut m = base();
        m.write_ram(0x0411, 0x64); // 与 Samus 同列（dx=0 ⇒ DistHi bit0=0）；敌 y 同 ⇒ bit1=0
        uw_collision_detection(&mut m);
        // DistHi = 0x04（只有常量位）⇒ <<3 = 0x20。旧形会给 0x38（多 0x10+0x08 或 0x08）。
        assert_eq!(m.read_ram(crate::ram_map::SAMUS_IS_HIT), 0x20,
                   "同列相触⇒Samus.isHit=0x20（DistHi 0x04 <<3），不含方向位");
    }

    /// 敌在 Samus 下方（dy>0⇒DistHi bit1=0）与上方（bit1=1）两侧位钉。
    #[test]
    fn samus_is_hit_sets_vertical_bit_from_dy_sign() {
        let mut m = base();
        m.write_ram(0x0410, 0x64 - 11); // 敌在 Samus 上方 11px ⇒ dy = 敌−Sam < 0 ⇒ bit1 置
        m.write_ram(0x0411, 0x64);
        uw_collision_detection(&mut m);
        assert_eq!(m.read_ram(crate::ram_map::SAMUS_IS_HIT), 0x30,
                   "敌在上⇒DistHi 0x06 <<3 = 0x30");
    }

    #[test]
    fn no_overlap_leaves_enemy_is_hit_untouched() {
        let mut m = base();
        m.write_ram(0x0411, 0xE0); // 远离（远超半径和）
        uw_collision_detection(&mut m);
        assert_eq!(m.read_ram(0x0414), 0x00, "未接触⇒不写 isHit");
    }
}
