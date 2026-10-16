//! 会师接线桥（bank00:8000 穿越）：GoMainRoutine 标题路 ↔ title::TitleSequence。
//!
//! **接线形态**（B 跑放行条件——lead 判词）：我侧桥（frame/ 归我）建 phase→臂映射，
//! armed 呼 title 域臂+应用 outcome（memmap::title_state 映射），unarmed 爆表址。
//! 输入契约面：phase 0 star_src=WRAM $6E00 面（InitBank0 星星拷贝源——title 注
//! 「精灵拷贝面（00：$6E00-$6E9F → $0260-$02FF，值源=入参 WRAM）」自证）；
//! 其余臂入参随 port2 契约面确认（未确认=unported 不发明）。

use crate::frame::FrameMemory;
use crate::memmap::title_state;
use crate::ram_map::{PPUCTRL_ZP, TITLE_ROUTINE};
use crate::title::TitleSequence;

/// MainTitleRoutine 分发一步（asm 内容锚 `MainTitleRoutine: ; 00:8000`——START 旁路+
/// `jsr JumpEngine` @L8027；表序=phase）。
pub fn main_title_routine(mem: &mut FrameMemory) {
    let phase = mem.read_ram(TITLE_ROUTINE);
    let mut seq = TitleSequence::new(phase);
    match phase {
        // 批1 助手簇（契约面已文档化）
        0x0A | 0x0B => {
            let o = seq.advance(mem.regs.a);
            apply_helper(mem, &o);
        }
        0x13 | 0x14 => {
            let o = seq.no_op(mem.regs.a);
            apply_helper(mem, &o);
        }
        0x0F => {
            let o = seq.set_replay_delay();
            apply_helper(mem, &o);
        }
        0x10 => {
            let o = seq.clear_spare_and_advance();
            apply_helper(mem, &o);
        }
        // 批2 编排簇——phase 0（InitializeAfterReset）：star_src=WRAM $6E00 面
        0x00 => {
            let star: Vec<u8> = (0x6E00..0x6EA0).map(|a| mem.read_wram(a as u16)).collect();
            let o = seq.init_after_reset(&star);
            apply_orchestration(mem, &o);
        }
        0x0C => {
            let o = seq.change_nametable(mem.read_ram(PPUCTRL_ZP));
            apply_orchestration(mem, &o);
        }
        // 未武装臂（批 2-6 未落/入参契约未确认）——爆表址（B 跑分歧面）
        _ => {
            crate::unported!("bank00:802A-phase");
        }
    }
}

/// HelperOutcome 应用（写事件→mem；「不写≠同值写」面）。
fn apply_helper(mem: &mut FrameMemory, o: &crate::title::HelperOutcome) {
    if let Some(p) = o.phase {
        let (a, v) = title_state::write_event_phase(p);
        mem.write_ram(a, v);
    }
    if let Some(t) = o.timer3 {
        let (a, v) = title_state::write_event_tag("TIMER3", t);
        mem.write_ram(a, v);
    }
    if let Some((cb, c9)) = o.cleared {
        let (a, v) = title_state::write_event_tag("SPARE_CB", cb);
        mem.write_ram(a, v);
        let (a, v) = title_state::write_event_tag("SPARE_C9", c9);
        mem.write_ram(a, v);
    }
    mem.regs.a = o.a_out;
}

/// OrchestrationOutcome 应用（**纯委托 title_state 契约面**：write_event_tag 标签解析+
/// objects_clear_events 复合展开——两层分工：映射在其面/应用在我桥）。
fn apply_orchestration(mem: &mut FrameMemory, o: &crate::title::OrchestrationOutcome) {
    use crate::memmap::title_state;
    if let Some(p) = o.phase {
        let (a, v) = title_state::write_event_phase(p); // 相位专用面（其分工）
        mem.write_ram(a, v);
    }
    for (label, v) in &o.writes {
        if *label == "OBJECTS_CLEAR" {
            for (a, v) in title_state::objects_clear_events() {
                mem.write_ram(a, v);
            }
            continue;
        }
        let (a, v) = title_state::write_event_tag(label, *v);
        mem.write_ram(a, v);
    }
    if let Some(fill) = &o.wram_fill {
        for (a, v) in fill {
            mem.write_wram(*a, *v);
        }
    }
    if let Some(copy) = &o.sprite_copy {
        for (a, v) in copy {
            mem.write_ram(*a, *v);
        }
    }
    mem.regs.a = o.a_out;
    mem.regs.x = o.x_out;
    mem.regs.y = o.y_out;
}

#[cfg(test)]
mod tests {
    /// OBJECTS_CLEAR 复合标签展开（「标签展开在 shim」契约点——$0300-$031F 32 事件）。
    #[test]
    fn objects_clear_label_expands_in_shim() {
        let events = crate::memmap::title_state::objects_clear_events();
        assert_eq!(events.len(), 32);
        assert_eq!(events[0], (0x0300, 0));
        assert_eq!(events[31], (0x031F, 0));
    }
}
