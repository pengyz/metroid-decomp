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

// ---- 批2·④编排簇（00/0C/11 可对拍；01/12 **L3 降级**（pipeline R3）——见模块底注）----

/// RamValueTbl 填充对（bank00:$80C8 `00 00 / 00 00 / 00 00 / C0 C4`——页 X 取对 (X&3)*2，
/// 前 0x40 字节用对[0]、其后用对[1]；结构互核：模式=页 63/67 前 64B=$C0 其余=$C4）。
const RAM_FILL_PAIRS: [[u8; 2]; 4] = [[0x00, 0x00], [0x00, 0x00], [0x00, 0x00], [0xC0, 0xC4]];

/// 批2 编排结果（写面语义；地址由 memmap::title_state 映射）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrchestrationOutcome {
    /// TitleRoutine 写面（Some=写入值：00/0C=old+1；11 路 B=**置 2**（sty Y=2 非 inc）；路 A=None）。
    pub phase: Option<u8>,
    /// 出口寄存器（X/Y 各臂语义：00 尾 jmp LoadStarSprites 后 X=$68/Y=$FF；11 路 B Y=2）。
    pub a_out: u8,
    pub x_out: u8,
    pub y_out: u8,
    /// zp/杂项写面（addr 为**字节偏移语义标签**经 memmap 映射；(标签, 值)）。
    pub writes: Vec<(&'static str, u8)>,
    /// WRAM 填充面（00：$6000-$67FF 表驱动）。
    pub wram_fill: Option<Vec<(u16, u8)>>,
    /// 精灵拷贝面（00：$6E00-$6E9F → $0260-$02FF，值源=入参 WRAM）。
    pub sprite_copy: Option<Vec<(u16, u8)>>,
}

impl TitleSequence {
    /// 00 InitializeAfterReset（bank00:$8071-$80C7 + 尾 jmp LoadStarSprites $98AE-$98BF）：
    /// zp 状态初始化（16 项）→ WRAM 表驱动填充（$6000-$67FF）→ inc 相位 → 星精灵拷贝
    /// （$6E00-$6E9F → $0260-$02FF）+ SpriteLoadPending←0。
    /// 寄存器尾面：X=$68（页游标 cpx #$68 出）、Y=$FF（精灵拷贝 dey cpy #$FF 出）、A=0。
    pub fn init_after_reset(&mut self, star_src: &[u8]) -> OrchestrationOutcome {
        let mut writes: Vec<(&'static str, u8)> = vec![
            ("SPARE_CF", 2), ("SPARE_CC", 2), ("SPARE_CE", 1), ("SPARE_D1", 1),
            ("SPARE_D0", 0), ("SPARE_CD", 0), ("SPARE_D3", 0), ("NARPASSWORD", 0),
            ("SPARE_CB", 0), ("SPARE_C9", 0), ("INTRO_MUSIC_RESTART", 2),
            ("SPARE_B7", 0), ("SPARE_B8", 0), ("PALETTE_INDEX", 0), ("FLASH_INDEX", 0),
            ("INTRO_STAR_OFFSET", 0), ("FADE_INDEX", 0), ("PTR_LO", 0),
        ];
        // WRAM 表驱动填充（页 $60-$67；模式=RAM_FILL_PAIRS[(页&3)*2 (+1)]）
        let mut wram_fill = Vec::with_capacity(0x800);
        for page in 0x60u16..0x68u16 {
            let pair = (page as usize & 3) * 2;
            for y in 0..0x100u16 {
                let v = if y < 0x40 { RAM_FILL_PAIRS[pair / 2][0] } else { RAM_FILL_PAIRS[pair / 2][1] };
                wram_fill.push(((page << 8) | y, v));
            }
        }
        writes.push(("PTR_HI", 0x67)); // $01 终值（末页游标）
        writes.push(("PTR_LO2", 6));   // $02 终值（末页对偏移）
        self.phase = self.phase.wrapping_add(1);
        // 星精灵拷贝（$6E00+Y → $0260+Y，Y=0x9F..0）+ SpriteLoadPending←0
        let mut sprite_copy = Vec::with_capacity(0xA0);
        for y in 0..0xA0u16 {
            sprite_copy.push((0x0260 + y, star_src.get(y as usize).copied().unwrap_or(0)));
        }
        writes.push(("SPRITE_LOAD_PENDING", 0));
        OrchestrationOutcome {
            phase: Some(self.phase), a_out: 0, x_out: 0x68, y_out: 0xFF,
            writes, wram_fill: Some(wram_fill), sprite_copy: Some(sprite_copy),
        }
    }

    /// 0C ChangeIntroNameTable（bank00:$822E-$8242 21B）：$FF|=1（nametable 0→1 镜像）、
    /// inc 相位、Timer3←$08（80 帧——10 帧/单位二证）、FadeDataIndex←$06、SpareMemC9←0；A=0。
    pub fn change_nametable(&mut self, ff_in: u8) -> OrchestrationOutcome {
        self.phase = self.phase.wrapping_add(1);
        OrchestrationOutcome {
            phase: Some(self.phase), a_out: 0, x_out: 0x11, y_out: 0x22,
            writes: vec![("PPUCTRL_ZP", ff_in | 1), ("TIMER3", 8), ("FADE_INDEX", 6), ("SPARE_C9", 0)],
            wram_fill: None, sprite_copy: None,
        }
    }

    /// 11 PrepIntroRestart（bank00:$82A3-$82EC 74B）双路径：
    /// 路 A（Timer3≠0）=无写直出（A=Timer3）；路 B=状态清（Objects $0300-$031F+11 项 zp 清+
    /// $FF&=$FC+**相位置 2**（sty Y=2）+音乐重启动作（IntroMusicRestart==0⇒SFX 标志←$10+
    /// 计数←2；否则 dec 计数）。
    pub fn prep_restart(&mut self, timer3: u8, ff_in: u8, music_restart: u8) -> OrchestrationOutcome {
        if timer3 != 0 {
            return OrchestrationOutcome {
                phase: None, a_out: timer3, x_out: 0x11, y_out: 0x22,
                writes: vec![], wram_fill: None, sprite_copy: None,
            };
        }
        let mut writes: Vec<(&'static str, u8)> = vec![
            ("SPARE_D2", 0), ("SPARE_BB", 0), ("IS_SAMUS", 0),
        ];
        for i in 0..0x20u16 {
            writes.push(("OBJECTS_CLEAR", 0)); // $0300-$031F 清（标签展开在 shim）
            let _ = i;
        }
        writes.push(("PPUCTRL_ZP", ff_in & 0xFC));
        for tag in ["SPARE_B7", "SPARE_B8", "PALETTE_INDEX", "FLASH_INDEX", "INTRO_STAR_OFFSET",
                    "FADE_INDEX", "SPARE_CD", "JOY1_CHANGE", "JOY1_STATUS", "JOY1_RETRIG", "SPARE_D7"] {
            writes.push((tag, 0));
        }
        writes.push(("SPARE_CE", 1));
        writes.push(("SPARE_CC", 2));
        writes.push(("SPARE_CF", 2));
        // 相位置 2（非 inc——sty Y=2）
        self.phase = 2;
        if music_restart == 0 {
            writes.push(("SFX_MULTI_INIT", 0x10));
            writes.push(("INTRO_MUSIC_RESTART", 2));
            OrchestrationOutcome { phase: Some(2), a_out: 2, x_out: 0x11, y_out: 2,
                                   writes, wram_fill: None, sprite_copy: None }
        } else {
            writes.push(("INTRO_MUSIC_RESTART", music_restart.wrapping_sub(1)));
            OrchestrationOutcome { phase: Some(2), a_out: music_restart, x_out: 0x11, y_out: 2,
                                   writes, wram_fill: None, sprite_copy: None }
        }
    }
}

// 【批2·L3 降级件（pipeline R3 显式，非静默——矩阵格标 L1）】
// - 01 DrawIntroBackground（bank00:$80D0-$80F8）：APU 初始化标志 + ClearNameTables/
//   PreparePPUProcess（PPU VRAM 面）+ ScreenOff/ScreenOn——**NMI 等待自旋**（WaitNMIPass_），
//   L2 单子程序模型内无 NMI 事件=死循环，不设 L2 案；行为面=boot-title L3 场景承载。
// - 12 TitleScreenOff（bank00:$82ED-$82F2）：jsr ScreenOff（$FE 镜像写+**NMI 等待自旋**）
//   + inc 相位——同上降级。
// 域实施待 shell 依赖组合点（VRAM 队列/NMI 等待原语）可复用后补（链规 19 不硬造镜像）。
