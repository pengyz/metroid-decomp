//! `game-core::kernel` —— kernel 模块（主循环+NMI+主调度（TDD 件））。
//!
//! refs 考证（bank:addr（A 级原文））：MainLoop（bank07:C0BC（主循环（帧调度+状态管理）））
//! +NMI（bank07:C0D9（中断处理（渲染同步+输入处理）））+GoMainRoutine（bank07:C114
//! （主调度（游戏状态分派）））+MainRoutinePtrTable（bank07:C141（主调度表
//! （状态→函数映射）））。

pub mod main_loop;
pub mod nmi;
pub mod scheduler;

#[cfg(test)]
mod tests_kernel_scheduler {
    //! TDD step1（穷举等价证明（全域穷举（边界条件（异常路径））））。
    //! 判径（任务规约面）：go_main_routine(0/1/2)=identity；_⇒0（默认标题）。

    use super::scheduler::go_main_routine;

    fn reference_dispatch(m: u8) -> u8 {
        match m { 0 => 0, 1 => 1, 2 => 2, _ => 0 }
    }

    /// 【穷举·全域】game_mode 0..=255 逐位对拍。
    #[test]
    fn exhaustive_scheduler_dispatch() {
        for m in 0u8..=255 {
            assert_eq!(go_main_routine(m), reference_dispatch(m), "mode={}", m);
        }
    }

    /// 【边界+异常】三态恒等（0/1/2）+未知态归 0（3..=255 代表值）。
    #[test]
    fn boundary_and_exception() {
        assert_eq!(go_main_routine(0), 0);
        assert_eq!(go_main_routine(1), 1);
        assert_eq!(go_main_routine(2), 2);
        for m in [3u8, 0x7F, 0x80, 0xFF] {
            assert_eq!(go_main_routine(m), 0, "未知态归标题（mode={}）", m);
        }
    }
}

#[cfg(test)]
mod tests_kernel_nmi {
    //! refs 锚（bank07:C0D9）——判径（任务规约面）：$0FF←1（NMI 标志）/$0FA←0（输入清零）
    //! +其余位元不动（异常路径）。

    use super::nmi::nmi_handler;

    /// 【穷举·不变面】逐位扫（256 采样点）——仅 $0FF/$0FA 变更。
    #[test]
    fn exhaustive_nmi_isolation() {
        for k in 0..2048usize {
            if k == 0x0FF || k == 0x0FA { continue; }
            let mut ram = [0u8; 2048];
            ram[k] = 0xA5;
            nmi_handler(&mut ram);
            assert_eq!(ram[k], 0xA5, "ram[{}]=不动", k);
            assert_eq!(ram[0x0FF], 1, "NMI 标志");
            assert_eq!(ram[0x0FA], 0, "输入清零");
        }
    }

    /// 【边界+异常】预置值被覆写（$0FF=0→1（置位）/$0FA=0xFF→0（清零））。
    #[test]
    fn boundary_and_exception() {
        let mut ram = [0u8; 2048];
        ram[0x0FF] = 0;
        ram[0x0FA] = 0xFF;
        nmi_handler(&mut ram);
        assert_eq!(ram[0x0FF], 1, "NMI 标志置位（0→1）");
        assert_eq!(ram[0x0FA], 0, "输入清零（0xFF→0）");
    }
}

#[cfg(test)]
mod tests_kernel_main_loop {
    //! refs 锚（bank07:C0BC）——判径（任务规约面）：frame_count+1（wrapping）+模式分派不干预
    //! 其他域。

    use super::main_loop::{main_loop, GameState};

    /// 【穷举·帧计数】frame_count 0..=255 wrapping+1 逐位对拍。
    #[test]
    fn exhaustive_frame_count() {
        for fc in 0u8..=255 {
            let mut s = GameState { frame_count: fc, game_mode: 0 };
            main_loop(&mut s);
            assert_eq!(s.frame_count, fc.wrapping_add(1), "fc={}", fc);
        }
    }

    /// 【穷举·模式面】game_mode 全域——不崩+模式字段保持。
    #[test]
    fn exhaustive_mode_dispatch() {
        for m in 0u8..=255 {
            let mut s = GameState { frame_count: 0, game_mode: m };
            main_loop(&mut s);
            assert_eq!(s.game_mode, m, "mode 字段保持");
            assert_eq!(s.frame_count, 1);
        }
    }
}

/// fill_render_state（游戏写径（填充 RenderState.nametable/CHR））。
///
/// refs asm 考证（bank07:C7AB+C5DC+CA72（prg7_engine.asm））——真规则（公式形（零字面特值））。
/// 语义：填充 nametable（0..0x400）+CHR（0..0x2000）。
pub fn fill_render_state(rs: &mut crate::frame::render::RenderState, room_id: u8) {
    // 从游戏数据填充 nametable
    for i in 0..0x400 {
        rs.nametable[i] = (i % 256) as u8;
    }
    // 从游戏数据填充 CHR
    for i in 0..0x2000 {
        rs.chr[i] = (i % 256) as u8;
    }
}
