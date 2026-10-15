//! game-core::title —— 标题/开场序列域（TitleRoutine 状态机；链规 19 形态：**一个域的
//! 状态机**，非逐入口镜像 fn）。
//!
//! asm 主文档（链规 17 内容锚+结构互核注）：refs/m1disasm/SRC/prg0_title.asm
//! `TitleRoutinePtrTable: ; 00:802A`（31 项：0x00-0x1D=title 域 30 例程 + 0x1E SetTimer
//! 共享助手）；分发壳 `MainTitleRoutine: ; 00:8000`（START 旁路 + L8027 `jsr JumpEngine`）。
//!
//! 【批1·③助手簇】（同码折叠=链规 19 首证，字节级同段实证）：
//! - 0A/0B **同码** `IncTitleRoutine`（bank00:$806E `e6 1f 60` INC $1F/RTS）——状态推进；
//! - 13/14 **同码** `TitleRoutineReturn`（bank00:$82F3 `60` RTS 1B）——空臂；
//! - 0F `DelayIntroReplay`（bank00:$8283 `e6 1f a9 10 85 2c 60`）——推进+Timer3←$10；
//! - 10 `ClearSpareMem`（bank00:$8068 `a9 00 85 cb 85 c9`）——**落穿共享尾**（+推进+RTS，
//!   共享尾结构=F2CA 族同款；L2 面按入口各自钉）。
//!
//! 【结构互核注】①ClearSpareMem 落穿进 $806E 共享尾（ROM `a9 00 85 cb 85 c9 e6 1f 60`
//! 九字节连续实证——入口 0x10 语义=清两址**+推进**）；②DelayIntroReplay「160 frames」注
//! **不是**注释≠语义反例：constants_ram.asm:496 `Timer3 db ;$2C ;Decremented every 10
//! frames after set`——$10=16 单位 ×10 帧=160 帧，注释经 Timer3 递减速率自证成立
//! （第三方注采纳三重自证：字节+递减率+行为）；③SpareMemC9 常量面=dw（$C9/$CA）而
//! ClearSpareMem 只写 $C9 低字节（`85 c9` zp 单址）——写面以字节为准。

/// 状态机骨架（批1 建形，枚举臂随批2-6 逐簇补）：TitleRoutine 相位面。
/// _id 表序=分发序（StartContinueScreen15=0x15 / EndGame=0x1D 锚——128e0a6 自数证）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TitleSequence {
    /// TitleRoutine 相位（zp $1F 面由 memmap::title_state 承载；值=表下标）。
    pub phase: u8,
}

/// 批1 助手簇结果（写事件语义面；地址由 memmap::title_state 映射）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HelperOutcome {
    /// TitleRoutine 写面（Some=inc $1F 新值（**mod 256 回绕**，INC zp 语义）；
    /// **None=本臂不写**——「不写≠同值写」（b-f270-a0 教训族：13/14 空臂无写事件）。
    pub phase: Option<u8>,
    /// Timer3 写值（Some=本臂写入；0F=$10）。
    pub timer3: Option<u8>,
    /// 清零面（Some=ClearSpareMem 臂：($CB,$C9)←(0,0)）。
    pub cleared: Option<(u8, u8)>,
    /// 出口 A（INC 不动 A——0A/0B/13/14=透传；10=LDA #$00→0；0F=LDA #$10→0x10）。
    pub a_out: u8,
}

impl TitleSequence {
    /// 新建（相位=入参快照）。
    pub fn new(phase: u8) -> Self {
        Self { phase }
    }

    /// 状态推进（0A/0B 同码；10/0F 的落穿/内联推进同源——**单一实现**）。
    pub fn advance(&mut self, a_in: u8) -> HelperOutcome {
        self.phase = self.phase.wrapping_add(1);
        HelperOutcome { phase: Some(self.phase), timer3: None, cleared: None, a_out: a_in }
    }

    /// 10 ClearSpareMem：$CB←0、$C9←0 **落穿共享尾推进**（ROM 九字节连续段实证）。
    pub fn clear_spare_and_advance(&mut self) -> HelperOutcome {
        self.phase = self.phase.wrapping_add(1);
        HelperOutcome { phase: Some(self.phase), timer3: None, cleared: Some((0, 0)), a_out: 0 }
    }

    /// 0F DelayIntroReplay：推进 + Timer3←$10（16 单位×10 帧=160 帧延迟）。
    pub fn set_replay_delay(&mut self) -> HelperOutcome {
        self.phase = self.phase.wrapping_add(1);
        HelperOutcome { phase: Some(self.phase), timer3: Some(0x10), cleared: None, a_out: 0x10 }
    }

    /// 13/14 同码空臂（`60` 单字节 RTS）。
    pub fn no_op(&self, a_in: u8) -> HelperOutcome {
        HelperOutcome { phase: None, timer3: None, cleared: None, a_out: a_in }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 0A/0B 同码推进（INC zp 回绕——$FF→$00）。
    #[test]
    fn advance_wraps_mod_256() {
        let mut t = TitleSequence::new(0xFF);
        let o = t.advance(0x77);
        assert_eq!((o.phase, o.a_out), (Some(0x00), 0x77)); // A 透传（INC 不动 A）
    }

    /// 10 落穿语义：清两址+推进+A←0（LDA #$00 起手）。
    #[test]
    fn clear_spare_falls_through_shared_tail() {
        let mut t = TitleSequence::new(0x10);
        let o = t.clear_spare_and_advance();
        assert_eq!((o.phase, o.cleared, o.a_out), (Some(0x11), Some((0, 0)), 0));
    }

    /// 0F：推进+Timer3←$10（160 帧延迟=16×10 帧单位——constants_ram:496 递减率）。
    #[test]
    fn replay_delay_sets_timer3_units() {
        let mut t = TitleSequence::new(0x0F);
        let o = t.set_replay_delay();
        assert_eq!((o.phase, o.timer3, o.a_out), (Some(0x10), Some(0x10), 0x10));
    }

    /// 13/14 空臂：相位不动、无写、A 透传。
    #[test]
    fn no_op_is_no_op() {
        let t = TitleSequence::new(0x13);
        let o = t.no_op(0x42);
        assert_eq!((o.phase, o.timer3, o.cleared, o.a_out), (None, None, None, 0x42));
    }
}
