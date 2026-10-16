//! 主循环族（bank07:C0BC 体；two-layer 域模块——frame/ 互斥面）。
//!
//! **asm 内容锚+结构互核注**（施工令②精化版）——
//! refs/m1disasm/SRC/prg7_engine.asm 主循环体（C0BC 族）：`jsr CheckBankSwitch`→
//! `jsr UpdateTimer`→`jsr GoMainRoutine`→`inc FrameCount`→`lda #$00/sta NMIStatus`→
//! spin（`@loop_waitNMIEnd: tay/lda NMIStatus/bne`）→`jsr RandomNumbers`→`jmp MainLoop`。
//!
//! **tick 内序**（契约 §8.1）：NMI 尾写 1（唤醒）→ RandomNumbers → C0BC 体 → 清 0 →
//! spin yield——本模块=一迭代；**spin=跨帧 yield**（引擎相位机）。
//!
//! **会师点**：GoMainRoutine 标题路=`jmp MainTitleRoutine`（**00:8000**——port2 task-23
//! title 簇头；TitleRoutinePtrTable 31 项分发表）——分发壳↔方法集对表点。

use super::memory::FrameMemory;
use crate::ram_map::{
    FRAME_COUNT, GAME_MODE, NMI_STATUS, RANDOM1, RANDOM2, TIMER1, TIMER2,
    TIMER3, TIMER_DELAY,
};

/// 主循环一迭代（C0BC 体；spin 唤醒已由本 tick NMI 完成）。
pub fn main_loop_iter(mem: &mut FrameMemory) {
    random_numbers(mem); // C000（唤醒后首件）
    super::boot::c4de_unit(mem); // CheckBankSwitch（已端口复用）
    update_timer(mem); // C266
    go_main_routine(mem); // C114（分发壳——会师点）
    mem.write_ram(FRAME_COUNT, mem.read_ram(FRAME_COUNT).wrapping_add(1)); // `inc FrameCount`
    mem.write_ram(NMI_STATUS, 0); // 清 0（spin 入口）
    // spin yield（跨帧——引擎相位机）
}

/// RandomNumbers 单元（**asm 内容锚+结构互核注**）——
/// refs/m1disasm/SRC/prg7_engine.asm `RandomNumbers: ; 07:C000`（注「RandomNumber1 is
/// increased by #$19 every frame / RandomNumber2 … by #$5F every frame」；体：
/// `ldx #$05` 循环内 `RandomNumber1+=5/RandomNumber2+=$13`×5=**每调 +$19/+$5F**）。
///
/// **结构互核注**：注释值=循环体×5 的合成值（$05×5=$19 ✓/$13×5=$5F ✓——注与体互证）；
/// A 终值=RandomNumber1。
pub fn random_numbers(mem: &mut FrameMemory) {
    let n1 = mem.read_ram(RANDOM1).wrapping_add(0x19);
    let n2 = mem.read_ram(RANDOM2).wrapping_add(0x5F);
    mem.write_ram(RANDOM1, n1);
    mem.write_ram(RANDOM2, n2);
    mem.regs.a = n1;
}

/// UpdateTimer 单元（**asm 内容锚**）——`UpdateTimer: ; 07:C266`：TimerDelay 预分频
/// （dec/bpl——10 帧节拍，归零重置 9 并含 Timer3）→ Timer1/2/3 非零递减。
pub fn update_timer(mem: &mut FrameMemory) {
    let delay = mem.read_ram(TIMER_DELAY).wrapping_sub(1);
    mem.write_ram(TIMER_DELAY, delay);
    let last = if (delay as i8) >= 0 { 1 } else {
        mem.write_ram(TIMER_DELAY, 0x09);
        2
    };
    for i in (0..=last).rev() {
        let addr = TIMER1 + i as u16;
        let v = mem.read_ram(addr);
        if v != 0 {
            mem.write_ram(addr, v - 1);
        }
    }
}

/// GoMainRoutine 分发壳（**asm 内容锚+结构互核注**）——`GoMainRoutine: ; 07:C114`：
/// GameMode≠0 ⇒ `jmp MainTitleRoutine`（**00:8000 会师点**——port2 title 簇头）；
/// GameMode=0 ⇒ START 检查/主例程分发（游戏面——task-11 面 fail-loud）。
pub fn go_main_routine(mem: &mut FrameMemory) {
    if mem.read_ram(GAME_MODE) != 0 {
        crate::unported!("bank00:8000"); // MainTitleRoutine（会师点——title 簇分发）
    }
    crate::unported!("bank07:C11B-game"); // 游戏面（START 检查/主例程分发——task-11 面）
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RandomNumbers 合成值自证（+$19/+$5F——注×5 互证面）。
    #[test]
    fn random_numbers_accumulates() {
        let mut m = FrameMemory::default();
        m.write_ram(RANDOM1, 0x11); // boot 初值（C0AC 自证）
        m.write_ram(RANDOM2, 0xFF);
        random_numbers(&mut m);
        assert_eq!(m.read_ram(RANDOM1), 0x2A);
        assert_eq!(m.read_ram(RANDOM2), 0x5E);
        assert_eq!(m.regs.a, 0x2A);
    }

    /// UpdateTimer 预分频+三计时器递减。
    #[test]
    fn update_timer_prescaler_and_decrement() {
        let mut m = FrameMemory::default();
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

    /// GoMainRoutine 标题路引爆会师点（bank00:8000）。
    #[test]
    fn go_main_routine_title_detonates_at_meeting_point() {
        let mut m = FrameMemory::default();
        m.write_ram(GAME_MODE, 1);
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| go_main_routine(&mut m)));
        let err = r.unwrap_err();
        let msg = err.downcast_ref::<String>().cloned()
            .or_else(|| err.downcast_ref::<&'static str>().map(|s| (*s).to_string()))
            .unwrap_or_default();
        assert!(msg.contains("unported @ bank00:8000"), "会师点失准: {msg}");
    }
}
