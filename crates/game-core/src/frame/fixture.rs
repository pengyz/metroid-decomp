//! fixture 注入相位契约（采集腿 ⇄ emu 腿对齐）。
//!
//! 【两套相位必须分清】同一份输入在两侧的「显示帧」定义不同：
//!
//! | 腿 | fixture 生效点 | 帧 F 的画面/名表内容 |
//! |---|---|---|
//! | emu（`tools/scenario_capture.lua`） | `endFrame(F)` 回调**帧末**（work(F) 已跑完） | work(F) 之后 |
//! | rust（`bin/rust_screenshot.rs`） | 循环头、`tick(N)` 之前 | `capture(F) = tick(F-1)` ⇒ work(F-1) |
//!
//! 于是「同帧注入」(`frame >= trigger`) 恰好让两侧的**显示帧**对齐：rust 在循环头 F 注入 ⇒
//! work(F) 消费 ⇒ 其画面出现在 `capture(F+1)`，与 emu `endFrame(F)` 之后那一屏等价。
//!
//! 【为什么必须留这条注释】2026-11-20 曾按「emu 在 F 注入、引擎 work(F+1) 才消费」的
//! **状态行**口径把判定改成 `frame > trigger`，结果 47 房扫描里 21 个原本 0px 的房
//! 全部变成 `dx = -2` 整屏漂移（`shift.explains = true`，~10k px），只有 4 个原本
//! dx=+2 的房转正 ⇒ **全局平移一帧不是 dx=2 族的根因**（证据见
//! `docs/findings/room-sweep-fixture-phase.md`）。状态行与像素面是两个判据，L4 裁的是像素。
//!
//! 结论：注入判定保持 `frame >= trigger`（once 语义由调用方去重表保证）。

/// 循环头注入判定：循环帧 `frame` 是否应落 `trigger` 帧声明的 fixture。
///
/// 返回 true 表示「本循环头写 RAM」——等价于 emu 腿 `endFrame(trigger)` 的写入在
/// **显示帧**上对齐（见模块文档的两腿相位表）。
#[inline]
pub fn fixture_due(frame: usize, trigger: usize) -> bool {
    frame >= trigger
}

#[cfg(test)]
mod tests {
    use super::fixture_due;

    /// 契约锚：注入帧当拍就注入（emu 在 endFrame(F) 写、rust 在循环头 F 写，
    /// 两者的**显示帧**因此对齐）。改成 `>` 会让整屏落后一帧（21 房 dx=-2 实录）。
    #[test]
    fn injection_frame_applies_on_the_same_loop_frame() {
        assert!(fixture_due(900, 900));
    }

    /// 注入帧之前不注入（避免提前一帧）。
    #[test]
    fn earlier_frames_do_not_apply() {
        assert!(!fixture_due(899, 900));
        assert!(!fixture_due(0, 1));
    }

    /// 跳帧也不丢（once 语义由调用方 `fixture_applied` 去重表保证）。
    #[test]
    fn later_frames_still_apply() {
        assert!(fixture_due(1500, 900));
    }

    /// 多 fixture 各自独立：同一帧对不同 trigger 的判定互不影响。
    #[test]
    fn multiple_triggers_are_independent() {
        let triggers = [900usize, 920];
        let due: Vec<bool> = triggers.iter().map(|t| fixture_due(901, *t)).collect();
        assert_eq!(due, vec![true, false]);
        let due: Vec<bool> = triggers.iter().map(|t| fixture_due(921, *t)).collect();
        assert_eq!(due, vec![true, true]);
    }
}
