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


/// MainRoutinePtrTable 烘焙（07:C141——10 项；表址自证）。
const MAIN_ROUTINE_PTR_TABLE: [u16; 10] = [
    0xC801, 0xC81D, 0xC8D1, 0xC92B, 0xC9A6, // AreaInit/MoreInit/SamusInit/GameEngine/PrepareGameOver
    0xC9B1, 0xC9C4, 0xC155, 0xC9D7, 0xC494, // PauseMode/GoPassword/IncrementRoutine/SamusIntro/WaitTimer
];

/// 游戏侧主例程分发（**asm 内容锚**——T45-3 移动臂首靶；C11B+ 面）：
/// START 检查（暂停切换）→`lda MainRoutine($1E)/jsr JumpEngine`→MainRoutinePtrTable
/// 10 项分派。未武装臂=per-entry 爆（移动臂迭代分类面）。
pub fn game_main_routine(mem: &mut FrameMemory) {
    let mr = mem.read_ram(crate::ram_map::MAIN_ROUTINE);
    if (mr as usize) < 10 {
        let target = MAIN_ROUTINE_PTR_TABLE[mr as usize];
        // JumpEngine 指针写（$0C/$0D——同标题面）
        mem.write_ram(0x000C, target as u8);
        mem.write_ram(0x000D, (target >> 8) as u8);
    }
    match mr {
        0 => area_init(mem), // AreaInit（已端口——移动臂迭代驱动首件）
        _ => {
            let loc = format!("bank07:{:04X}", MAIN_ROUTINE_PTR_TABLE.get(mr as usize).copied().unwrap_or(0));
            crate::unported!(&loc); // 其余态（MoreInit 起——迭代分类）
        }
    }
}

/// AreaInit（**asm 内容锚** `AreaInit: ; 07:C801`——游戏侧区域初始化）：
/// `ScrollX($26)/ScrollY($25)←0`/`PPUCTRL_ZP($FF)&=$FC`（名表 0）/`inc MainRoutine`
/// （→MoreInit）/`ABStatus←Joy1Status&(A|B)`/`jsr EraseAllSprites`（C1A3 已折）/
/// `lda #$10/jsr IsEngineRunning`（CA18——页切检桩）。
pub fn area_init(mem: &mut FrameMemory) {
    mem.write_ram(crate::ram_map::SCROLL_X, 0);
    mem.write_ram(crate::ram_map::SCROLL_Y, 0);
    let ff = mem.read_ram(crate::ram_map::PPUCTRL_ZP) & 0xFC;
    mem.write_ram(crate::ram_map::PPUCTRL_ZP, ff);
    mem.write_ram(crate::ram_map::MAIN_ROUTINE, mem.read_ram(crate::ram_map::MAIN_ROUTINE).wrapping_add(1));
    let ab = mem.read_ram(crate::memmap::title_state::JOY1_STATUS) & 0xC0;
    mem.write_ram(0x0027, ab); // ABStatus（:254 族）
    crate::frame::boot::c1a3_unit(mem); // EraseAllSprites（已折）
    crate::unported!("bank07:CA18"); // IsEngineRunning（页切检——随件）
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
        super::title_bridge::main_title_routine(mem); // **会师接线**（bank00:8000 穿越）
        return; // 标题路终（asm：`jmp MainTitleRoutine` 尾跳语义）
    }
    game_main_routine(mem); // 游戏侧分发（C11B+——T45-3 移动臂首靶）
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

    /// GoMainRoutine 标题路**会师穿越**（bank00:8000 已通——phase 0 臂推进效应）。
    #[test]
    fn go_main_routine_title_crosses_meeting_point() {
        let mut m = FrameMemory::default();
        m.write_ram(GAME_MODE, 1); // TitleRoutine=0=InitializeAfterReset
        go_main_routine(&mut m); // 桥穿越（phase 0 臂执行）
        assert_eq!(m.read_ram(crate::ram_map::TITLE_ROUTINE), 1); // 相位推进（inc $1F）
    }
}
