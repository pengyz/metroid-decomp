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
    let phase = mem.read_ram(TITLE_ROUTINE);
    let mut star_src = [0u8; 0xA0];
    for (i, b) in star_src.iter_mut().enumerate() {
        *b = mem.read_wram(0x6E00 + i as u16);
    }
    let face = crate::title::TitleFace {
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
    }
    let step = crate::title::dispatch_step(&face);
    mem.title_cursor = step.next_cursor; // 跨帧游标存回（>0=spin 续点/0=完成复位）
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
        } else {
            let loc = format!("bank00:{addr:04X}-jump");
            crate::unported!(&loc); // 其余跳转出口（CheckPassword 转接面——随接）
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 统一入口穿越（phase 0 臂推进——dispatch_step 面）。
    #[test]
    fn bridge_crosses_via_unified_entry() {
        let mut m = FrameMemory::default();
        main_title_routine(&mut m); // phase 0（InitializeAfterReset）
        assert_eq!(m.read_ram(TITLE_ROUTINE), 1); // 相位推进
    }

    /// OBJECTS_CLEAR 复合标签展开（「标签展开在 shim」契约点——$0300-$031F 32 事件）。
    #[test]
    fn objects_clear_label_expands_in_shim() {
        let events = crate::memmap::title_state::objects_clear_events();
        assert_eq!(events.len(), 32);
        assert_eq!(events[0], (0x0300, 0));
        assert_eq!(events[31], (0x031F, 0));
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
