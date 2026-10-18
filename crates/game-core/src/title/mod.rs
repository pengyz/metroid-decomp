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

pub mod sparkle;
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

// ---- 批3·①淡变族（02/03/0D/0E——**同形折叠主件**）----
// 共形骨架=Timer3 门 + FrameCount 掩 + 表步进 + 完成转移（chain 19「同形族」正样）。
// 窗口读表（A9 族口径——L2 案显式 window_bank=0 钉窗）：
//   FADE_TABLE=$8B6D 组合 12B（[0..4]=fade-out 序、[5]=$FF 终止、[6..10]=fade-in 序、
//   [11]=$FF——0D 的 `cmp #$0B` 端测=索引 11 即终止符位，双表单数组索引实证）；
//   PAL_SELECT_TABLE=$8A9A 13B（idx12=$FF；idx10/11 双 0c——调色板 id 重复列，字节为准）。

/// 淡变组合表（bank0:$8B6D 12B）。
pub const FADE_TABLE: [u8; 12] = [0x0D, 0x0E, 0x0F, 0x10, 0x01, 0xFF, 0x01, 0x10, 0x0F, 0x0E, 0x0D, 0xFF];

/// PalSelect 表（bank0:$8A9A 13B）。
pub const PAL_SELECT_TABLE: [u8; 13] = [0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0C, 0xFF];

/// 淡变驱动面（zp 快照）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FadeFace {
    pub timer3: u8,
    pub frame_count: u8,
    pub pal_index: u8,
    pub fade_index: u8,
    pub phase: u8,
    /// PPUCTRL_ZP 镜像输入（02 `and #$FE` 源）。
    pub ppuctrl: u8,
}

/// 淡变结果（写面语义标签经 memmap 映射；y_out=None=透传——LoadPalData/DoFadeOut 的
/// `ldy` 会覆写 Y，端测/门出口不经步进器=透传）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FadeOutcome {
    pub phase: Option<u8>,
    pub a_out: u8,
    /// X 末值（None=透传；07 完成臂=InitCrossMissiles 循环出口 $FF）。
    pub x_out: Option<u8>,
    pub y_out: Option<u8>,
    pub writes: Vec<(&'static str, u8)>,
    /// 槽面写（InitCrossMissiles 标记三元组——memmap::title_state::effect_event 映射）。
    pub slot_writes: Vec<SlotWriteTag>,
}

/// 淡变驱动（4 员同形折叠——非 4 个镜像 fn）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FadeDriver {
    /// 02 FadeInDelay：一次性装配（$FF&=$FE、Timer3←8、PaletteDataIndex←4（`lsr` 复用））。
    DelaySetup,
    /// 03 METROIDFadeIn：Timer3 门 + &$0F 掩 + PalSelect 步进（Z=耗尽）+ 完成⇒Timer3←$20。
    MetroidIn,
    /// 0D MessageFadeIn：Timer3 门 + &7 掩 + 组合表步进 + 端测 idx==$0B。
    MessageIn,
    /// 0E MessageFadeOut：Timer3 门 + &7 掩 + 组合表步进 + 端测 idx==$05。
    MessageOut,
    /// 07 METROIDFadeOut：&7 掩 + 端测 idx==4（完成臂=InitCrossMissiles+Timer3/SlowDelay←8+
    /// SecondCrosshair←0+inc）+ **无条件** DoFadeOut（与 0D/0E 的完成后跳步不对称——asm 字节实证）。
    MetroidOut,
}

impl FadeDriver {
    /// 同形 tick（骨架唯一：门→掩→端/步）。
    pub fn tick(&self, f: &FadeFace) -> FadeOutcome {
        match self {
            Self::DelaySetup => {
                // `lda #$08 / sta Timer3 / lsr`（A=8→4 一指令双用）/`sta PaletteDataIndex`
                FadeOutcome {
                    slot_writes: vec![],
                    x_out: None,
                    phase: Some(f.phase.wrapping_add(1)),
                    a_out: 4,
                    y_out: None,
                    writes: vec![("PPUCTRL_ZP", f.ppuctrl & 0xFE), ("TIMER3", 8),
                                 ("PALETTE_INDEX", 4)],
                }
            }
            Self::MetroidIn => {
                if f.timer3 != 0 {
                    return FadeOutcome { slot_writes: vec![], x_out: None, phase: None, a_out: f.timer3, y_out: None, writes: vec![] };
                }
                let m = f.frame_count & 0x0F;
                if m != 0 {
                    return FadeOutcome { slot_writes: vec![], x_out: None, phase: None, a_out: m, y_out: None, writes: vec![] };
                }
                // LoadPalData 步进（ldy pal_index；Z=表耗尽）
                let t = PAL_SELECT_TABLE[f.pal_index as usize];
                if t == 0xFF {
                    return FadeOutcome { slot_writes: vec![], x_out: None, phase: Some(f.phase.wrapping_add(1)), a_out: 0x20,
                                         y_out: Some(f.pal_index),
                                         writes: vec![("TIMER3", 0x20)] };
                }
                FadeOutcome { slot_writes: vec![], x_out: None, phase: None, a_out: t, y_out: Some(f.pal_index),
                              writes: vec![("PENDING_PAL", t), ("PALETTE_INDEX", f.pal_index.wrapping_add(1))] }
            }
            Self::MetroidOut => {
                let m = f.frame_count & 0x07;
                if m != 0 {
                    return FadeOutcome { slot_writes: vec![], x_out: None, phase: None, a_out: m, y_out: None, writes: vec![] };
                }
                let mut writes: Vec<(&'static str, u8)> = vec![];
                if f.fade_index == 4 {
                    // 完成臂（`20 97 88` InitCrossMissiles + `a9 08 85 2c 85 bc` + `a9 00 85 bf`
                    // + `e6 1f`）——槽面写经 MARKS 展开在 shim
                    let slot_writes = sparkle::init_cross_missiles(f.ppuctrl);
                    writes.push(("TIMER3", 8));
                    writes.push(("CROSS_SLOW_DELAY", 8));
                    writes.push(("SECOND_CROSS", 0));
                    // phase inc 由 tick 统一推
                    // 无条件 DoFadeOut 随后执行
                    let t = FADE_TABLE[f.fade_index as usize];
                    if t != 0xFF {
                        writes.push(("PENDING_PAL", t));
                        writes.push(("FADE_INDEX", f.fade_index.wrapping_add(1)));
                    }
                    // X 末=InitCrossMissiles `dex/bpl` 64 跑出口（$3F 递减至 $FF）
                    return FadeOutcome { slot_writes, x_out: Some(0xFF), phase: Some(f.phase.wrapping_add(1)), a_out: t,
                                         y_out: Some(f.fade_index), writes };
                }
                let t = FADE_TABLE[f.fade_index as usize];
                if t != 0xFF {
                    writes.push(("PENDING_PAL", t));
                    writes.push(("FADE_INDEX", f.fade_index.wrapping_add(1)));
                }
                FadeOutcome { slot_writes: vec![], x_out: None, phase: None, a_out: t, y_out: Some(f.fade_index), writes }
            }
            Self::MessageIn | Self::MessageOut => {
                if f.timer3 != 0 {
                    return FadeOutcome { slot_writes: vec![], x_out: None, phase: None, a_out: f.timer3, y_out: None, writes: vec![] };
                }
                let m = f.frame_count & 0x07;
                if m != 0 {
                    return FadeOutcome { slot_writes: vec![], x_out: None, phase: None, a_out: m, y_out: None, writes: vec![] };
                }
                let end = if *self == Self::MessageIn { 0x0B } else { 0x05 };
                if f.fade_index == end {
                    return if *self == Self::MessageIn {
                        // 端测 0D：FadeDataIndex←0、Timer3←$30（480 帧=48×10）、inc（A=$30）
                        FadeOutcome { slot_writes: vec![], x_out: None, phase: Some(f.phase.wrapping_add(1)), a_out: 0x30, y_out: None,
                                      writes: vec![("FADE_INDEX", 0), ("TIMER3", 0x30)] }
                    } else {
                        // 端测 0E：FadeDataIndex←6（fade-in 序起点）、SpareMemCB←0、inc（A=0）
                        FadeOutcome { slot_writes: vec![], x_out: None, phase: Some(f.phase.wrapping_add(1)), a_out: 0x00, y_out: None,
                                      writes: vec![("FADE_INDEX", 6), ("SPARE_CB", 0)] }
                    };
                }
                // DoFadeOut 步进（ldy fade_index）
                let t = FADE_TABLE[f.fade_index as usize];
                FadeOutcome { slot_writes: vec![], x_out: None, phase: None, a_out: t, y_out: Some(f.fade_index),
                              writes: vec![("PENDING_PAL", t), ("FADE_INDEX", f.fade_index.wrapping_add(1))] }
            }
        }
    }
}

// ---- 批4a·②特效簇（04/05/06——EffectDriver 三臂；06 组合 sparkle::update_sparkle_sprites）----

/// 特效驱动（04/05/06——门/掩/完成转移共形骨架的第二族折叠）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectDriver {
    /// 04 LoadFlashTimer：Timer3 门 + 装配（Timer3←8、inc）。
    FlashTimerSetup,
    /// 05 FlashEffect：&3 掩 + PalSelect 步进 + 完成（idx==4）⇒ LoadSparkleData+Timer3←$18。
    FlashStep,
    /// 06 METROIDSparkle：Timer3 门 + 双 complete 检 + 4 跑运动链。
    SparkleStep,
}

/// 特效面（zp+槽面输入）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EffectFace {
    pub timer3: u8,
    pub frame_count: u8,
    pub pal_index: u8,
    pub phase: u8,
    pub complete0: u8,
    pub complete1: u8,
}

/// 特效结果（写面含槽写——(slot|0x80=SpriteRAM 标记, 偏移, 值)）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectOutcome {
    pub phase: Option<u8>,
    pub a_out: u8,
    pub x_out: Option<u8>,
    pub y_out: Option<u8>,
    pub writes: Vec<SlotWriteTag>,
    /// 05 完成臂：LoadSparkleData 装载面（两槽同值）。
    pub sparkle_init: bool,
}

/// 槽写标签（重导 sparkle 域）。
pub type SlotWriteTag = sparkle::SlotWrite;

impl EffectDriver {
    /// 特效 tick（04/05 无槽写；06 运动链经 sparkle 域）。
    pub fn tick(&self, f: &EffectFace, top: &mut sparkle::SparkleSlot,
                bottom: &mut sparkle::SparkleSlot) -> EffectOutcome {
        match self {
            Self::FlashTimerSetup => {
                if f.timer3 != 0 {
                    return EffectOutcome { phase: None, a_out: f.timer3, x_out: None, y_out: None,
                                           writes: vec![], sparkle_init: false };
                }
                EffectOutcome { phase: Some(f.phase.wrapping_add(1)), a_out: 8, x_out: None, y_out: None,
                                writes: vec![(0x40 | 0, 3, 8)], sparkle_init: false }
            }
            Self::FlashStep => {
                if f.frame_count & 3 != 0 {
                    return EffectOutcome { phase: None, a_out: f.frame_count & 3, x_out: None,
                                           y_out: None, writes: vec![], sparkle_init: false };
                }
                let c1 = f.pal_index & 3;
                let t = PAL_SELECT_TABLE[c1 as usize];
                let mut pending: Option<u8> = None;
                let mut idx = c1;
                if t != 0xFF {
                    pending = Some(t);
                    idx = c1.wrapping_add(1);
                }
                if f.timer3 != 0 {
                    return EffectOutcome { phase: None, a_out: f.timer3, x_out: None,
                                           y_out: Some(c1), writes: vec![], sparkle_init: false, }
                        .with_pal(c1, idx, pending);
                }
                if idx != 4 {
                    return EffectOutcome { phase: None, a_out: idx, x_out: None,
                                           y_out: Some(c1), writes: vec![], sparkle_init: false }
                        .with_pal(c1, idx, pending);
                }
                let mut o = EffectOutcome { phase: Some(f.phase.wrapping_add(1)), a_out: 0x18,
                                            x_out: Some(0xFF), y_out: Some(c1), writes: vec![],
                                            sparkle_init: true };
                o = o.with_pal(c1, idx, pending);
                o
            }
            Self::SparkleStep => {
                if f.timer3 != 0 {
                    return EffectOutcome { phase: None, a_out: f.timer3, x_out: None, y_out: None,
                                           writes: vec![], sparkle_init: false };
                }
                if f.complete0 & f.complete1 == 1 {
                    return EffectOutcome { phase: Some(f.phase.wrapping_add(1)), a_out: 1,
                                           x_out: None, y_out: None, writes: vec![],
                                           sparkle_init: false };
                }
                let r = sparkle::update_sparkle_sprites(top, bottom);
                // X 末=ldx #$10（UpdateSparkleSprites 中点装载）；Y 末=最后 coord `ldy idx/iny`
                EffectOutcome { phase: None, a_out: r.a_out, x_out: Some(0x10), y_out: Some(r.y_out),
                                writes: r.writes, sparkle_init: false }
            }
        }
    }
}

impl EffectOutcome {
    /// 05 调色板面合成（clamp 写+步进写）。
    fn with_pal(mut self, clamped: u8, idx: u8, pending: Option<u8>) -> Self {
        self.writes.push((0x40 | 0, 0, clamped)); // 0x40=PALETTE_INDEX 标记
        self.writes.push((0x40 | 0, 1, idx));
        if let Some(t) = pending {
            self.writes.push((0x40 | 0, 2, t)); // 2=PENDING 标记
        }
        self
    }
}

// ---- 批4b·09 MoreCrosshairs（组合臂：UpdateCrossExplode + FlashIntroScreen 尾跳 /
//      或 inc+死码写面（030D/030E/0306——「abandoned code」残迹仍写，字节实证保真））----

/// 09 组合结果（复用 EffectOutcome 写面）。
pub fn crosshair_continue(flash: u8, pal_index: u8, explode_index: u8, is_updating: u8,
                          frame_count: u8, anim_reset: u8, phase: u8,
                          top: &mut sparkle::SparkleSlot, bottom: &mut sparkle::SparkleSlot) -> EffectOutcome {
    if flash != 0 {
        let (_a1, x1, _y1, mut sw) = sparkle::update_cross_explode(is_updating, explode_index, frame_count);
        let (a2, y2, sw2) = sparkle::flash_intro_screen(pal_index);
        sw.extend(sw2);
        // 尾 `jmp FlashIntroScreen`——A/Y 以 flash 出口为准（ldy 索引覆写 Y）；X 保留 explode
        // 末值（inx 轨迹）。
        return EffectOutcome { phase: None, a_out: a2, x_out: x1, y_out: y2,
                               writes: sw, sparkle_init: false };
    }
    // 死码写面（`a9 60 8d 0d 03 a9 7c 8d 0e 03 ad 05 03 8d 06 03`——abandoned code 残迹）
    EffectOutcome {
        phase: Some(phase.wrapping_add(1)),
        a_out: anim_reset, // 末 `lda $0305 / sta $0306`——A=animResetIndex
        x_out: None,
        y_out: None,
        // 标记 0x02=$0300 块面（off=址内偏移）——死码残迹写（D4 保真：abandoned code 仍写）
        writes: vec![(0x02, 0x0D, 0x60), (0x02, 0x0E, 0x7C), (0x02, 0x06, anim_reset)],
        sparkle_init: false,
    }
}

// ---- 批4b·08 Crosshairs 编排（flash→Timer3 门→完成检（**A 毒化语义**）→运动/爆炸组合）----

/// 08 编排（$8182-$81D0）。结构互核注：**A 毒化**——激活块 `lda #$00/sta $C6` 使随后
/// `and IntroSprs.4.complete` 链恒 0（当帧二组完成判必失败）——字节语义保真。
#[allow(clippy::too_many_arguments)]
pub fn crosshairs_step(flash: u8, timer3: u8, pal_index: u8, second: u8, frame_count: u8,
                       explode_index: u8, is_updating: u8, phase: u8,
                       slots: &mut [sparkle::CrossMissile; 8], slow: &mut u8, spawn: &mut u8,
                       star_src: &[u8]) -> EffectOutcome {
    let mut writes: Vec<SlotWriteTag> = Vec::new();
    let mut y_track: Option<u8> = None;
    let mut x_track: Option<u8> = None;
    if flash != 0 {
        let (a_f, y2, sw) = sparkle::flash_intro_screen(pal_index);
        let _ = a_f;
        y_track = y2;
        writes.extend(sw);
    }
    if timer3 != 0 {
        return EffectOutcome { phase: None, a_out: timer3, x_out: x_track, y_out: y_track,
                               writes, sparkle_init: false };
    }
    let first4 = slots[0].complete & slots[1].complete & slots[2].complete & slots[3].complete;
    if first4 == 0 {
        let (sw, x, y) = sparkle::update_cross_missiles(slots, slow, spawn, frame_count);
        writes.extend(sw);
        x_track = x.or(x_track);
        y_track = y.or(y_track);
        let (a3, x3, y3, sw) = sparkle::update_cross_explode(is_updating, explode_index, frame_count);
        writes.extend(sw);
        return EffectOutcome { phase: None, a_out: a3,
                               x_out: x3.or(x_track), y_out: y3.or(y_track),
                               writes, sparkle_init: false };
    }
    // 第二组激活块（A 毒化）
    let mut a_and = 1u8;
    let mut ex_idx = explode_index;
    let mut ex_flag = is_updating;
    if second == 1 {
        // beq @endIf_B——A=1 入 and 链
    } else {
        writes.push((0x20, 0xBF, second.wrapping_add(1)));
        ex_flag = 1;
        writes.push((0x20, 0xC7, 1));
        writes.push((0x20, 0xC0, 1));
        ex_idx = 0;
        writes.push((0x20, 0xC6, 0));
        a_and = 0; // `lda #$00` 毒化
    }
    let second4 = slots[4].complete & slots[5].complete & slots[6].complete & slots[7].complete;
    if a_and & second4 == 0 {
        let (sw, x, y) = sparkle::update_cross_missiles(slots, slow, spawn, frame_count);
        writes.extend(sw);
        x_track = x.or(x_track);
        y_track = y.or(y_track);
        let (a3, x3, y3, sw) = sparkle::update_cross_explode(ex_flag, ex_idx, frame_count);
        writes.extend(sw);
        return EffectOutcome { phase: None, a_out: a3,
                               x_out: x3.or(x_track), y_out: y3.or(y_track),
                               writes, sparkle_init: false };
    }
    // 末块：LoadStarSprites + 置位 + inc → 爆炸（新置位）
    ex_flag = 1;
    writes.push((0x20, 0xC7, 1));
    writes.push((0x20, 0xC0, 1));
    for y in (0..0xA0u16).rev() {
        writes.push((0x08, (0x60 + y) as u8, star_src.get(y as usize).copied().unwrap_or(0)));
    }
    writes.push((0x20, 0xC8, 0));
    ex_idx = 0;
    writes.push((0x20, 0xC6, 0));
    let phase_new = phase.wrapping_add(1);
    let (a3, x3, y3, sw) = sparkle::update_cross_explode(ex_flag, ex_idx, frame_count);
    writes.extend(sw);
    EffectOutcome { phase: Some(phase_new), a_out: a3,
                    x_out: x3.or(x_track), y_out: y3.or(Some(0xFF)), // LoadStarSprites Y 末=$FF（可被爆炸覆写）
                    writes, sparkle_init: false }
}

// ---- 批5·⑤菜单簇（16/1A L2；15/1B/1C L3 降级（R3 NMI 自旋族）——链规 20 轻仪式件）----

/// 菜单结果（exit=Jump 时停点语义：`jmp InitializeStats`/`jmp CheckPassword`）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuOutcome {
    pub phase: Option<u8>,
    pub a_out: u8,
    pub x_out: Option<u8>,
    pub y_out: Option<u8>,
    pub writes: Vec<(u16, u8)>, // 直址写面（菜单面=RAM/精灵面，已字节定）
    /// true=`jmp` 出口（L2 stops 面）；false=RTS。
    pub jumped: bool,
}

/// StartContTbl（bank0:$9118 2B）。
pub const START_CONT_TBL: [u8; 2] = [0x60, 0x78];

/// 16 ChooseStartContinue（$90D7-$9117 65B）。
/// 结构互核注：**both-quirk**——Joy1Change 同按 START|SELECT（$30）时 `cmp` 两路**皆不入**
/// （掩后 $30≠$10 亦 ≠$20）——字节语义保真。
pub fn menu_choose(joy_change: u8, start_continue: u8) -> MenuOutcome {
    let masked = joy_change & 0x30;
    let mut sc = start_continue;
    let mut w = Vec::new();
    if masked == 0x10 {
        if sc == 0 {
            // `jmp InitializeStats`（$932B）——L2 stops 停点
            // jmp 停点面：A=掩后值（cmp 不动 A）、Y=ldy StartContinue
            return MenuOutcome { phase: None, a_out: masked, x_out: None, y_out: Some(sc),
                                 writes: w, jumped: true };
        }
        w.push((0x001F, 0x17)); // TitleRoutine←_id_LoadPasswordScreen
    }
    if masked == 0x20 {
        sc ^= 1;
        w.push((0x0325, sc));
        w.push((0x0683, 0x08)); // SFXTriInitFlags|=sfxTri_Beep（夹具 0 起=精确值）
    }
    let y = sc;
    w.push((0x0200, START_CONT_TBL[y as usize]));
    w.push((0x0201, 0x6E));
    w.push((0x0202, 0x03));
    w.push((0x0203, 0x50));
    MenuOutcome { phase: None, a_out: 0x50, x_out: None, y_out: Some(y), writes: w, jumped: false }
}

/// 1A WaitForSTART（$9394-$939D 10B）：START 按下→`jmp CheckPassword`（$8C5E 停点）。
pub fn wait_for_start(joy_change: u8) -> MenuOutcome {
    let m = joy_change & 0x10;
    if m == 0 {
        return MenuOutcome { phase: None, a_out: 0, x_out: None, y_out: None,
                             writes: Vec::new(), jumped: false };
    }
    MenuOutcome { phase: None, a_out: 0x10, x_out: None, y_out: None,
                  writes: Vec::new(), jumped: true }
}

// 【批5·L3 降级件（R3 显式——NMI 自旋族，同 01/12 口径）】
// - 15/1B StartContinueScreen（$90BA-$90D0 同码共享）：ClearAll（ScreenOff NMI 自旋+
//   ClearNameTables VRAM+EraseAllSprites）+PreparePPUProcess_+落穿 TurnOnDisplay
//   （NMIOn+**jmp ScreenOn**=WaitNMIPass_ 自旋）——L2 模型内死循环不设案；
// - 1C GameOver（$939E-$93B8）：ClearAll+PreparePPUProcess_+InitPasswordFontGFX+NMIOn+
//   Timer3←$10+TitleRoutine←_id_DisplayPassword+jmp ScreenOn——同上降级。
// 行为面=boot-title L3 场景承载；域实施待 shell 组合点（链规 19 不硬造镜像）。

// ---- 批6a·⑥⑦（19 门面 + 1D EndGame 分发；17/18 深路 L3 降级）----

/// EndStarDataTable（bank0:$9EB8 156B）。
pub const END_STAR_TBL: [u8; 156] = [
    0x08,0x23,0x22,0x10,0x68,0x23,0x23,0x60,0x00,0x23,0x22,0x60,0x7F,0x23,0x23,0x6A,
    0x7F,0x23,0x22,0xD4,0x33,0x23,0x23,0xB2,0x93,0x23,0x22,0x47,0xB3,0x23,0x23,0x95,
    0x0B,0x23,0x22,0xE2,0x1C,0x23,0x23,0x34,0x84,0x23,0x22,0x18,0xB2,0x23,0x23,0xEE,
    0x40,0x23,0x22,0x22,0x5A,0x23,0x23,0x68,0x1A,0x23,0x22,0x90,0xAA,0x23,0x23,0x22,
    0x81,0x24,0x22,0x88,0x6A,0x24,0x23,0xD0,0xA8,0x24,0x22,0xA0,0x10,0x24,0x23,0x70,
    0x15,0x25,0x22,0x42,0x4A,0x25,0x23,0x7D,0x30,0x25,0x22,0x50,0x5A,0x25,0x23,0x49,
    0x50,0x25,0x22,0xB9,0x91,0x25,0x23,0xB0,0x19,0x25,0x22,0xC0,0x53,0x25,0x23,0xBA,
    0xA4,0x25,0x22,0xD6,0x98,0x25,0x23,0x1A,0x68,0x25,0x22,0x0C,0x97,0x25,0x23,0xEA,
    0x33,0x25,0x22,0x92,0x43,0x25,0x23,0x65,0xAC,0x25,0x22,0x4A,0x2A,0x25,0x23,0x71,
    0x7C,0x26,0x22,0xB2,0x73,0x26,0x23,0xE7,0x0C,0x26,0x22,0xAA,
];

/// 19 DisplayPassword（$9359-$937E）门面：Timer3≠0 ⇒ 直出（深路=NMI 自旋族 L3 降级）。
pub fn password_display_gate(timer3: u8) -> MenuOutcome {
    MenuOutcome { phase: None, a_out: timer3, x_out: None, y_out: None,
                  writes: Vec::new(), jumped: false }
}

/// 1D EndGame（$9AA7-$9AC0+JumpEngine 分发）：星精灵装载（156B→$0270）+ 调色板循环
/// （IsCredits 或帧相位门；inc $1C 到 9 回 1）+ JumpEngine 分发（**X/Y 保存恢复**、
/// A=目标高字节——C27C 语义；停点=表目目标）。
pub fn end_game(is_credits: u8, frame_count: u8, palette: u8, room_ptr: u8, x_in: u8, y_in: u8)
    -> MenuOutcome {
    let mut w = Vec::new();
    for y in 0..0x9Cu16 {
        w.push((0x0270 + y, END_STAR_TBL[y as usize]));
    }
    let mut pal = palette;
    if is_credits == 0 && (frame_count & 0x0F) == 0 {
        pal = pal.wrapping_add(1);
        w.push((0x001C, pal));
        if pal == 9 {
            pal = 1;
            w.push((0x001C, pal));
        }
    }
    // JumpEngine（$C27C）：TempX/TempY←jsr 时点 X/Y（**星环出口 Y=$9C**——ldy/iny/cpy 轨迹）；
    // RoutinePtr 末值=目标址（`sta ptr+1/stx ptr`）；分发时 X/Y 恢复、A=目标高字节。
    w.push((0x0069, x_in));
    w.push((0x006A, 0x9C));
    let targets: [u16; 5] = [0x9AD5, 0x9B1C, 0x9B34, 0x9B93, 0x9BCD];
    let tgt = targets[room_ptr as usize % 5];
    w.push((0x000C, (tgt & 0xFF) as u8));
    w.push((0x000D, (tgt >> 8) as u8));
    MenuOutcome { phase: None, a_out: (tgt >> 8) as u8, x_out: Some(x_in), y_out: Some(0x9C),
                  writes: w, jumped: true }
}

// 【批6·L3 降级件（R3 显式）】17 LoadPasswordScreen（$911A-$9146）：ClearAll（ScreenOff
// NMI 自旋）+PreparePPUProcess_+**jmp TurnOnDisplay**（NMIOn+ScreenOn 自旋）；
// 18 EnterPassword（$9147 起）：深路（密码格光标/字符输入面）待 shell 组合点后续批
// （L2 可钉面=START⇒jmp CheckPassword 停点——随 18 专件补）。


// ---- 批6b：ClearAll 共享段面（15/1B/1C/17 共用——ScreenOff spin①+NT/Erase/Dec 面+
//      WaitNMIPass spin②；段=帧界）----

/// ClearAll 段0 事件（ScreenOff 前置：$FE&=~$18+$1A←0）。
pub fn clear_all_seg0_events(ppumask: u8) -> Vec<(u16, u8)> {
    vec![(0x00FE, ppumask & !0x18), (0x001A, 0)]
}

/// ClearAll 段1 事件（ClearNameTables NT0+NT1 面 + EraseAllSprites 256 面 +
/// DecSpriteYCoord 奇偶门面 + $FF&=$FC + $FC/$FD←0 + $1A←0）。
/// 语义注：DecSpriteYCoord=**偶帧才减**（`lda FrameCount/lsr/bcs`）+40 星双面 dec
/// （$6E00+x 源面+$0260+x 精灵面）+SpriteLoadPending←0；TitleRoutine≥0x1D 出。
pub fn clear_all_seg1_events(ram: &[u8], wram: &[u8], frame_count: u8, phase: u8,
                             ppuctrl: u8) -> Vec<(u16, u8)> {
    let mut e = Vec::new();
    // ClearNameTables（title 路 NT0+NT1 两轮；$FF&=~INCR_DOWN(0x04)）
    let ff = ppuctrl & 0xFB;
    e.push((0x0001, 0x01)); e.push((0x0000, 0xFF)); e.push((0x00FF, ff));
    e.push((0x0001, 0x02)); e.push((0x0000, 0xFF)); e.push((0x00FF, ff));
    // EraseAllSprites（$01←2/$00←0 + $0200-$02FF←$F0）
    e.push((0x0001, 0x02)); e.push((0x0000, 0x00));
    for a in 0x0200u16..0x0300 {
        e.push((a, 0xF0));
    }
    // DecSpriteYCoord（title 路 GameMode≠0 链入）：TitleRoutine<0x1D 且 SpriteLoadPending≠0
    // 且偶帧 ⇒ 40 星双面 dec + $C8←0
    let pending = ram[0x00C8 & 0x7FF];
    if phase < 0x1D && pending != 0 && (frame_count & 1) == 0 {
        for i in (0..0xA0u16).step_by(4).rev() {
            let s = wram.get(0x0E00 + i as usize).copied().unwrap_or(0);
            let d = ram.get(0x0260 + i as usize).map(|v| *v).unwrap_or(0);
            e.push((0x6E00 + i, s.wrapping_sub(1)));
            e.push((0x0260 + i, d.wrapping_sub(1)));
        }
        e.push((0x00C8, 0));
    }
    // ClearAll 体尾：$FF&=$FC、$FC/$FD←0、$1A←0（WaitNMIPass 前置）
    let ff2 = ff & 0xFC;
    e.push((0x00FF, ff2)); e.push((0x00FC, 0)); e.push((0x00FD, 0)); e.push((0x001A, 0));
    e
}

/// ClearAll 段2 事件（VBOffAndHorzWrite：$FF&=~$84——PPUCTRL_INCR_DOWN(0x04)|
/// VBLKNMI_ON(0x80)；hardware.asm:18 位值自证）。
pub fn clear_all_seg2_events(ff_in: u8) -> Vec<(u16, u8)> {
    vec![(0x00FF, ff_in & !0x84)]
}

// ==== 会师接线契约（task-23 × frame/ B 跑）====
//
// 【① 分发入口形状】统一入口=**本文件 `dispatch_step`**（phase→臂匹配归 title 域所有；
// 桥侧只解码→调用→应用）。入参 `TitleFace` 由 `memmap::title_state::decode_title_face`
// 解码（star_src=WRAM $6E00-$6E9F 面、ff=zp $FF、joy=Joy1Change $12、…——逐字段注）。
// 【② outcome 应用面】`memmap::title_state::apply_title_step`（标签→址映射统一在
// memmap（two-layer §1）；桥侧亦可直读 step.events 已是 (addr,val) 事件）。
// 【③ 未武装臂】`TitleStep::Unported{ table_addr }` 返回值——**桥侧爆**
// `unported!("bank00:<表址>")`（你的 per-entry 形状照用；表址在本匹配内逐臂给）。
// 五降级件=**占位臂**（RAM 面已武装 + `TitleStep::Shell(Vec<ShellCall>)` 组合点声明——
// spin 点=ShellCall::NmiYield（§8.1 spin 跨帧建模口径：桥侧按帧界让出，下一帧续）；
// PPU/VRAM 效果由你侧 shell 执行——01/12/15/1B/1C 深效果=L3 裁决面）。

/// shell 组合点 ABI（title 域声明、frame/ shell 执行——两层边界）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellCall {
    /// ScreenOff 族（$FE 镜像面+NMI 等待——**NmiYield 语义**）。
    ScreenOff,
    /// ScreenOn 族（同上）。
    ScreenOn,
    /// NMIOn（PPUCTRL_ZP|=$80+NMI 等待）。
    NMIOn,
    /// ClearNameTables（VRAM 清面——你侧 intro.rs 生产端组合）。
    ClearNameTables,
    /// PreparePPUProcess_（VRAM 写串——ptr=内联结构体址，你侧生产端装载）。
    PrepareVram(u16),
    /// InitPasswordFontGFX（图案面）。
    InitPasswordFontGfx,
    /// EraseAllSprites（$0200-$02FF←$F0）。
    EraseAllSprites,
    /// NMI 等待点（§8.1：帧界让出，下帧续）。
    NmiYield,
    /// WaitNMIPass（$1A←0+spin——1 spin）。
    WaitNMIPass,
    /// VBOffAndHorzWrite（$FF&=~$84+PPUCTRL 真写——0 spin）。
    VBOffAndHorzWrite,
    /// DisplayInputCharacters（$940B：$00/$01 指针 RAM 面已含臂事件；字符面 PPU 侧——0 spin）。
    DisplayInputCharacters,
}

/// title 域一 tick 结果（写面=语义标签事件（已映射址）；shell=组合点；exit=分发出口）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TitleStep {
    pub events: Vec<(u16, u8)>,
    pub shell: Vec<ShellCall>,
    pub a_out: Option<u8>,
    pub x_out: Option<u8>,
    pub y_out: Option<u8>,
    /// 跳转出口（jmp InitializeStats/CheckPassword 等——桥侧按停点或转接处理）。
    pub jumped: Option<u16>,
    /// 未武装臂（桥侧 `unported!("bank00:…")` 爆）。
    pub unported: Option<u16>,
    /// 下一帧游标（0=入口完成复位；>0=spin 续点——桥侧存回 FrameMemory 传回）。
    pub next_cursor: u8,
}

/// 解码入参面（memmap::title_state::decode_title_face 产出本型）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TitleFace {
    pub phase: u8,
    /// 入口内跨帧子步游标（spin 件续点；0=入口首段）。
    pub cursor: u8,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub timer3: u8,
    pub frame_count: u8,
    pub joy_change: u8,
    pub ppuctrl: u8,
    pub star_src: [u8; 0xA0],
    pub wram: Vec<u8>,
    pub ram: Vec<u8>,
}

/// **统一分发入口**（B 跑接线点）：phase→臂。已武装臂=批1-6a 交付面；未武装=Unported
/// 表址返回；五降级件=占位臂（RAM 面+Shell 组合点，深效果 L3 裁决面）。
pub fn dispatch_step(face: &TitleFace) -> TitleStep {
    let l2in = crate::L2In {
        ram: face.ram.clone(), wram: face.wram.clone(),
        a: face.a, x: face.x, y: face.y, p: 0x24, window_bank: 0,
    };
    let step_from_l2 = |o: crate::L2Out| TitleStep {
        events: o.writes, shell: vec![], a_out: Some(o.a), x_out: Some(o.x), y_out: Some(o.y),
        jumped: None, unported: None, next_cursor: 0,
    };
    use crate::verify_shim as sh;
    match face.phase {
        0x00 => step_from_l2(sh::title_init_after_reset(&l2in)),
        0x0A | 0x0B => step_from_l2(sh::title_inc(&l2in)),
        0x0C => step_from_l2(sh::title_change_nametable(&l2in)),
        0x0D | 0x0E | 0x02 | 0x03 | 0x07 => step_from_l2(match face.phase {
            0x02 => sh::title_fade_delay(&l2in), 0x03 => sh::title_metroid_fade_in(&l2in),
            0x07 => sh::title_metroid_fade_out(&l2in), 0x0D => sh::title_message_fade_in(&l2in),
            _ => sh::title_message_fade_out(&l2in),
        }),
        0x04 => step_from_l2(sh::title_flash_timer(&l2in)),
        0x05 => step_from_l2(sh::title_flash_effect(&l2in)),
        0x06 => step_from_l2(sh::title_sparkle_step(&l2in)),
        0x08 => step_from_l2(sh::title_crosshairs_step(&l2in)),
        0x09 => step_from_l2(sh::title_crosshair_continue(&l2in)),
        0x0F => step_from_l2(sh::title_replay_delay(&l2in)),
        0x10 => step_from_l2(sh::title_clear_spare(&l2in)),
        0x11 => step_from_l2(sh::title_prep_restart(&l2in)),
        0x13 | 0x14 => step_from_l2(sh::title_no_op(&l2in)),
        0x19 => step_from_l2(sh::title_password_gate(&l2in)),
        // jmp 出口臂（直域调——jumped 面透出；B 跑桥侧按转接/停点处理）
        0x16 => {
            let joy = face.ram[crate::memmap::title_state::JOY1_CHANGE as usize & 0x7FF];
            let sc = face.ram[0x0325 & 0x7FF];
            let o = menu_choose(joy, sc);
            TitleStep { events: o.writes, shell: vec![], a_out: Some(o.a_out),
                        x_out: o.x_out.or(Some(face.x)), y_out: o.y_out.or(Some(face.y)),
                        jumped: if o.jumped { Some(0x932B) } else { None }, unported: None, next_cursor: 0 }
        }
        0x1A => {
            let joy = face.ram[crate::memmap::title_state::JOY1_CHANGE as usize & 0x7FF];
            let o = wait_for_start(joy);
            TitleStep { events: o.writes, shell: vec![], a_out: Some(o.a_out),
                        x_out: o.x_out.or(Some(face.x)), y_out: o.y_out.or(Some(face.y)),
                        jumped: if o.jumped { Some(0x8C5E) } else { None }, unported: None, next_cursor: 0 }
        }
        0x1D => {
            let r = |a: u16| face.ram[a as usize & 0x7FF];
            let o = end_game(r(0x007B), r(crate::ram_map::FRAME_COUNT),
                             r(crate::ram_map::PALETTE_PENDING), r(0x0033), face.x, face.y);
            TitleStep { events: o.writes, shell: vec![], a_out: Some(o.a_out),
                        x_out: o.x_out.or(Some(face.x)), y_out: o.y_out.or(Some(face.y)),
                        jumped: if o.jumped { Some(0x9AD5) } else { None }, unported: None, next_cursor: 0 }
        }
        // ---- 批6b 深路径：12 TitleScreenOff（模式件——1 spin 2 段跨帧）----
        // 段0=$FE 镜像面（ScreenOff 前置）+ScreenOff spin；段1=inc 相位（完成）。
        0x12 => {
            let pm = face.ram[0x00FE & 0x7FF];
            match face.cursor {
                0 => TitleStep {
                    events: vec![(0x00FE, pm & !0x18)], // ScreenOff：~(BG_ON|OBJ_ON)=~%00011000（hardware.asm:37/39）
                    shell: vec![ShellCall::ScreenOff], a_out: None, x_out: None, y_out: None,
                    jumped: None, unported: None, next_cursor: 1,
                },
                _ => TitleStep {
                    events: vec![crate::memmap::title_state::write_event_phase(face.phase.wrapping_add(1))],
                    shell: vec![], a_out: None, x_out: None, y_out: None,
                    jumped: None, unported: None, next_cursor: 0,
                },
            }
        }
        // ---- 批6b：01 DrawIntroBackground（3 段 2 spin——全仪式关键件）----
        // 段0=APU 标志面+ScreenOff（$FE&=~$18+$1A←0+spin①）｜段1=ClearNameTables RAM 面
        // （NT0+NT1 两轮 $01/$00/$FF 面）+PrepareVram($82F4 PPU 侧)+体写+ScreenOn（$FE|=$1E+
        // $1A←0+spin②）｜段2=完成。
        0x01 => {
            let pm = face.ram[0x00FE & 0x7FF];
            let ff = face.ram[0x00FF & 0x7FF];
            match face.cursor {
                0 => TitleStep {
                    events: vec![(0x00F0, 0x10), (0x0684, 0x10),
                                 (0x00FE, pm & !0x18), (0x001A, 0)],
                    shell: vec![ShellCall::ScreenOff], a_out: None, x_out: None, y_out: None,
                    jumped: None, unported: None, next_cursor: 1,
                },
                1 => TitleStep {
                    events: vec![
                        // ClearNameTables（title 路 NT0+NT1 两轮；$FF&=~INCR_DOWN(0x04)）
                        (0x0001, 0x01), (0x0000, 0xFF), (0x00FF, ff & 0xFB),
                        (0x0001, 0x02), (0x0000, 0xFF), (0x00FF, ff & 0xFB),
                        // 体写
                        (crate::ram_map::PALETTE_PENDING, 0x01), (0x00C5, 0x01),
                        (0x00FF, (ff & 0xFB) & 0xFC),
                        crate::memmap::title_state::write_event_phase(face.phase.wrapping_add(1)),
                        (0x00D7, 0),
                        // ScreenOn 面（$FE|=$1E）
                        (0x00FE, pm | 0x1E), (0x001A, 0),
                    ],
                    shell: vec![ShellCall::ClearNameTables, ShellCall::PrepareVram(0x82F4),
                                ShellCall::ScreenOn],
                    a_out: None, x_out: None, y_out: None,
                    jumped: None, unported: None, next_cursor: 2,
                },
                _ => TitleStep {
                    events: vec![], shell: vec![], a_out: None, x_out: None, y_out: None,
                    jumped: None, unported: None, next_cursor: 0,
                },
            }
        }
        // ---- 批6b：15/1B StartContinueScreen（**同码共享臂**（字节级同段 $90BA）——
        //      4 段 3 spin：ClearAll①②+TurnOnDisplay ScreenOn③）----
        0x15 | 0x1B => {
            let pm = face.ram[0x00FE & 0x7FF];
            let ff = face.ram[0x00FF & 0x7FF];
            match face.cursor {
                0 => TitleStep {
                    events: clear_all_seg0_events(pm), shell: vec![ShellCall::ScreenOff],
                    a_out: None, x_out: None, y_out: None,
                    jumped: None, unported: None, next_cursor: 1,
                },
                1 => TitleStep {
                    events: clear_all_seg1_events(&face.ram, &face.wram, face.frame_count,
                                                  face.phase, ff),
                    shell: vec![ShellCall::ClearNameTables, ShellCall::EraseAllSprites,
                                ShellCall::WaitNMIPass],
                    a_out: None, x_out: None, y_out: None,
                    jumped: None, unported: None, next_cursor: 2,
                },
                2 => {
                    // VBOff 面 + PrepareVram($9984) + 体写 + NMIOn 面 + ScreenOn 面 + spin③
                    let mut events = clear_all_seg2_events(ff & 0xFB & 0xFC);
                    events.push((0x0325, 0)); // StartContinue←0
                    events.push((crate::ram_map::PALETTE_PENDING, 0x0D)); // _id_Palette0C+1
                    events.push(crate::memmap::title_state::write_event_phase(0x16)); // ←ChooseStartContinue
                    events.push((0x00FF, (ff & 0xFB & 0xFC & !0x84) | 0x80)); // NMIOn：$FF|=$80
                    events.push((0x00FE, pm | 0x1E)); // ScreenOn：$FE|=$1E
                    events.push((0x001A, 0));
                    TitleStep {
                        events, shell: vec![ShellCall::VBOffAndHorzWrite,
                                            ShellCall::PrepareVram(0x9984),
                                            ShellCall::NMIOn, ShellCall::ScreenOn],
                        a_out: None, x_out: None, y_out: None,
                        jumped: None, unported: None, next_cursor: 3,
                    }
                }
                _ => TitleStep {
                    events: vec![], shell: vec![], a_out: None, x_out: None, y_out: None,
                    jumped: None, unported: None, next_cursor: 0,
                },
            }
        }
        // ---- 批6b：1C GameOver（4 段 3 spin——ClearAll①②+NMIOn/ScreenOn③；体写面：
        //      PrepareVram($93B9)+InitPasswordFontGFX+Timer3←$10+相位←0x19（DisplayPassword））----
        0x1C => {
            let pm = face.ram[0x00FE & 0x7FF];
            let ff = face.ram[0x00FF & 0x7FF];
            match face.cursor {
                0 => TitleStep {
                    events: clear_all_seg0_events(pm), shell: vec![ShellCall::ScreenOff],
                    a_out: None, x_out: None, y_out: None,
                    jumped: None, unported: None, next_cursor: 1,
                },
                1 => TitleStep {
                    events: clear_all_seg1_events(&face.ram, &face.wram, face.frame_count,
                                                  face.phase, ff),
                    shell: vec![ShellCall::ClearNameTables, ShellCall::EraseAllSprites,
                                ShellCall::WaitNMIPass],
                    a_out: None, x_out: None, y_out: None,
                    jumped: None, unported: None, next_cursor: 2,
                },
                2 => {
                    // VBOff+PrepareVram(@VRAMStruct $93B9)+InitPasswordFontGFX+NMIOn+
                    // 体写（Timer3←$10、相位←0x19）+ScreenOn+spin③
                    let mut events = clear_all_seg2_events(ff & 0xFB & 0xFC);
                    events.push((crate::memmap::title_state::TIMER3, 0x10));
                    events.push(crate::memmap::title_state::write_event_phase(0x19));
                    events.push((0x00FF, (ff & 0xFB & 0xFC & !0x84) | 0x80)); // NMIOn
                    events.push((0x00FE, pm | 0x1E)); // ScreenOn
                    events.push((0x001A, 0));
                    TitleStep {
                        events, shell: vec![ShellCall::VBOffAndHorzWrite,
                                            ShellCall::PrepareVram(0x93B9),
                                            ShellCall::InitPasswordFontGfx,
                                            ShellCall::NMIOn, ShellCall::ScreenOn],
                        a_out: None, x_out: None, y_out: None,
                        jumped: None, unported: None, next_cursor: 3,
                    }
                }
                _ => TitleStep {
                    events: vec![], shell: vec![], a_out: None, x_out: None, y_out: None,
                    jumped: None, unported: None, next_cursor: 0,
                },
            }
        }
        // ---- 批6b：17 LoadPasswordScreen（4 段 3 spin——ClearAll①②+TurnOnDisplay③；
        //      体写面：PrepareVram($99E3)+InitPasswordFontGFX+DisplayInputCharacters
        //      （RAM 面=$00/$01 指针写；字符面 PPU 侧）+调色板/光标初始化+inc+尾链）----
        0x17 => {
            let pm = face.ram[0x00FE & 0x7FF];
            let ff = face.ram[0x00FF & 0x7FF];
            match face.cursor {
                0 => TitleStep {
                    events: clear_all_seg0_events(pm), shell: vec![ShellCall::ScreenOff],
                    a_out: None, x_out: None, y_out: None,
                    jumped: None, unported: None, next_cursor: 1,
                },
                1 => TitleStep {
                    events: clear_all_seg1_events(&face.ram, &face.wram, face.frame_count,
                                                  face.phase, ff),
                    shell: vec![ShellCall::ClearNameTables, ShellCall::EraseAllSprites,
                                ShellCall::WaitNMIPass],
                    a_out: None, x_out: None, y_out: None,
                    jumped: None, unported: None, next_cursor: 2,
                },
                2 => {
                    let mut events = clear_all_seg2_events(ff & 0xFB & 0xFC);
                    // DisplayInputCharacters RAM 面（$00/$01 指针）
                    events.push((0x0000, 0)); events.push((0x0001, 0));
                    // 体写（$912C 起字节序：1C←_id_Palette12+1=0x13、$321/$322/$2C←0、
                    // $320←0、$324←0、inc、jmp TurnOnDisplay 尾链）
                    events.push((crate::ram_map::PALETTE_PENDING, 0x13));
                    events.push((0x0321, 0)); events.push((0x0322, 0));
                    events.push((crate::memmap::title_state::TIMER3, 0));
                    events.push((0x0320, 0));
                    events.push((0x0324, 0));
                    events.push(crate::memmap::title_state::write_event_phase(face.phase.wrapping_add(1)));
                    events.push((0x00FF, (ff & 0xFB & 0xFC & !0x84) | 0x80)); // NMIOn
                    events.push((0x00FE, pm | 0x1E)); // ScreenOn
                    events.push((0x001A, 0));
                    TitleStep {
                        events, shell: vec![ShellCall::VBOffAndHorzWrite,
                                            ShellCall::PrepareVram(0x99E3),
                                            ShellCall::InitPasswordFontGfx,
                                            ShellCall::DisplayInputCharacters,
                                            ShellCall::NMIOn, ShellCall::ScreenOn],
                        a_out: None, x_out: None, y_out: None,
                        jumped: None, unported: None, next_cursor: 3,
                    }
                }
                _ => TitleStep {
                    events: vec![], shell: vec![], a_out: None, x_out: None, y_out: None,
                    jumped: None, unported: None, next_cursor: 0,
                },
            }
        }
        // ---- 五降级占位臂（12/01/15/1B/1C/17 已深路径化——0x18 专件=稳定锚随后）----
        0x01 | 0x12 | 0x15 | 0x1B | 0x1C | 0x17 => TitleStep {
            events: vec![], shell: vec![ShellCall::NmiYield], a_out: None, x_out: None,
            y_out: None, jumped: None, next_cursor: 0,
            unported: Some(match face.phase {
                                0x17 => 0x911A, _ => 0x939E,
            }),
        },
        // ---- 真未武装（18 深路专件/0x1E SetTimer 随件）----
        // ---- 批6b 终件：0x18 EnterPassword（单段帧常驻 UI——armed）----
        0x18 => {
            let joy = face.ram[crate::memmap::title_state::JOY1_CHANGE as usize & 0x7FF];
            let joy_retrig = face.ram[0x0016 & 0x7FF];
            let (events, jumped) = enter_password_body(&face.ram, &face.wram, face.frame_count,
                                                       joy, joy_retrig, face.phase);
            TitleStep { events, shell: vec![], a_out: None, x_out: None, y_out: None,
                        jumped, unported: None, next_cursor: 0 }
        }
        // ---- 0x1E SetTimer（随件——`85 2c 86 20 a9 09` 字节真：Timer3←A/NextRoutine←X/
        //      MainRoutine($1E)←_id_WaitTimer(0x09)）----
        0x1E => TitleStep {
            events: vec![(crate::memmap::title_state::TIMER3, face.a),
                         (0x0020, face.x), // NextRoutine
                         (0x001E, 0x09)],  // MainRoutine←WaitTimer
            shell: vec![], a_out: Some(0x09), x_out: Some(face.x), y_out: Some(face.y),
            jumped: None, unported: None, next_cursor: 0,
        },
        // 域外相位（契约违例面——fail-loud 样本）
        _ => TitleStep { events: vec![], shell: vec![], a_out: None, x_out: None,
                         y_out: None,
                         jumped: None, next_cursor: 0, unported: Some(0xC4AA) },
    }
}

#[cfg(test)]
mod dispatch_contract_tests {
    use super::*;

    /// 契约面：已武装臂不回 Unported（00 例）；真未武装回表址（0x18/0x1E 例）。
    #[test]
    fn dispatch_arms_and_unported_faces() {
        let face = TitleFace {
            phase: 0x18, cursor: 0, a: 0, x: 0, y: 0, timer3: 0, frame_count: 0, joy_change: 0,
            ppuctrl: 0x90, star_src: [0; 0xA0], wram: vec![0; 0x2000], ram: vec![0; 0x800],
        };
        let s = dispatch_step(&face);
        assert_eq!(s.unported, None, "0x18 专件应已臂化");
        let mut f2 = face.clone(); f2.phase = 0x1E;
        let s = dispatch_step(&f2);
        assert_eq!(s.unported, None, "0x1E SetTimer 已臂化（title 全清）");
        let mut f4 = face.clone(); f4.phase = 0xFF;
        let s = dispatch_step(&f4);
        assert_eq!(s.unported, Some(0xC4AA), "域外相位=契约违例引爆形");
        let mut f3 = face.clone(); f3.phase = 0x0A;
        let face = f3;
        let s = dispatch_step(&face);
        assert_eq!(s.unported, None);
        assert!(s.events.iter().any(|&(a, _)| a == 0x001F));
    }
}

// ---- 批6b 终件：0x18 EnterPassword 专件（单段 0-spin 帧常驻 UI——类 16 形）----
// 结构互核注：①WritePPUByte 溢出 quirk（缓冲 $4F 满⇒写 0 终止+**双 PLA 断链**——
// 正常流不触发，忠实注记）②缓冲协 X 链 vs WriteVRAMString 重载 $07A0（每行 stx 回写）——
// 字节忠实照建（msg 头 3+15 链写不回写、字符串面重载起点——重叠面=字节真行为，行集裁）
// ③beep 写门（`lda $16/and #$0F/beq`——Joy1Retrig dpad 位门，注称 dpad 触发 ✓ 成立）
// ④A/B 位=Joy1Change 位7/位6（hardware.asm:95-96）。

/// ERROR TRY AGAIN 文（bank0:$8759 15B）。
pub const PW_ERR_MSG: [u8; 15] = [0x0E,0x1B,0x1B,0x18,0x1B,0xFF,0x1D,0x1B,0x22,0xFF,0x0A,0x10,0x0A,0x12,0x17];
/// 空白覆盖文（bank0:$8768 15B 全 $FF）。
pub const PW_NOERR_MSG: [u8; 15] = [0xFF; 15];
/// PasswordRowTbl（bank0:$9998 5×2B 行指针）。
pub const PW_ROW_TBL: [u16; 5] = [0x99A2, 0x99AF, 0x99BC, 0x99C9, 0x99D6];
/// 字符格（bank0:$99A2 起 65B——0x00-0x3F 递序+末位 $FF）。
pub const PW_GRID: [u8; 65] = [
    0x00,0x01,0x02,0x03,0x04,0x05,0x06,0x07,0x08,0x09,0x0A,0x0B,0x0C,0x0D,0x0E,0x0F,
    0x10,0x11,0x12,0x13,0x14,0x15,0x16,0x17,0x18,0x19,0x1A,0x1B,0x1C,0x1D,0x1E,0x1F,
    0x20,0x21,0x22,0x23,0x24,0x25,0x26,0x27,0x28,0x29,0x2A,0x2B,0x2C,0x2D,0x2E,0x2F,
    0x30,0x31,0x32,0x33,0x34,0x35,0x36,0x37,0x38,0x39,0x3A,0x3B,0x3C,0x3D,0x3E,0x3F,
    0xFF,
];
/// CursorPosXTbl（bank0:$92C8 12B）。
pub const CURSOR_X_TBL: [u8; 12] = [0x48,0x50,0x58,0x60,0x68,0x70,0x80,0x88,0x90,0x98,0xA0,0xA8];
/// CharSelectYTbl（bank0:$92B6 5B）。
pub const CHARSEL_Y_TBL: [u8; 5] = [0x77,0x87,0x97,0xA7,0xB7];
/// CharSelectXTbl（bank0:$92BB 13B）。
pub const CHARSEL_X_TBL: [u8; 13] = [0x20,0x30,0x40,0x50,0x60,0x70,0x80,0x90,0xA0,0xB0,0xC0,0xD0,0xE0];

/// 0x18 EnterPassword 帧体（单段；事件=全量 RAM 面；jumped=START⇒$8C5E）。
#[allow(clippy::too_many_arguments)]
pub fn enter_password_body(ram: &[u8], wram: &[u8], frame_count: u8, joy: u8, joy_retrig: u8,
                           phase: u8) -> (Vec<(u16, u8)>, Option<u16>) {
    let mut e = Vec::new();
    // ① EraseAllSprites 面（$01←2/$00←0+$0200-$02FF←$F0+DecSpriteYCoord 奇偶门）
    e.push((0x0001, 0x02)); e.push((0x0000, 0x00));
    for a in 0x0200u16..0x0300 { e.push((a, 0xF0)); }
    let pending = ram[0x00C8 & 0x7FF];
    if phase < 0x1D && pending != 0 && (frame_count & 1) == 0 {
        for i in (0..0xA0u16).step_by(4).rev() {
            let s = wram.get(0x0E00 + i as usize).copied().unwrap_or(0);
            let d = ram.get(0x0260 + i as usize).copied().unwrap_or(0);
            e.push((0x6E00 + i, s.wrapping_sub(1)));
            e.push((0x0260 + i, d.wrapping_sub(1)));
        }
        e.push((0x00C8, 0));
    }
    // ② START 出口（jumped $8C5E——前置写面已含）
    if joy & 0x10 != 0 {
        return (e, Some(0x8C5E));
    }
    // ③ 缓冲头（$1B←1；X 链自 $07A0）+msg 15 文（X 链不回写——字节真）
    e.push((0x001B, 1));
    let idx = ram[0x07A0 & 0x7FF] as u16;
    let msg: &[u8; 15] = if ram[0x002C & 0x7FF] != 0 { &PW_ERR_MSG } else { &PW_NOERR_MSG };
    // 头：$21,$A8,$0F（WritePPUByte 链写）
    let mut x = idx;
    e.push((0x07A1 + (x & 0x7F), 0x21)); x = x.wrapping_add(1);
    e.push((0x07A1 + (x & 0x7F), 0xA8)); x = x.wrapping_add(1);
    e.push((0x07A1 + (x & 0x7F), 0x0F)); x = x.wrapping_add(1);
    for &b in msg.iter() {
        e.push((0x07A1 + (x & 0x7F), b)); x = x.wrapping_add(1);
    }
    // ④ 指针面（$02/$03←msg 源）——err/noerr 两路
    e.push((0x0002, if ram[0x002C & 0x7FF] != 0 { 0x59 } else { 0x68 }));
    e.push((0x0003, 0x87));
    // ⑤ A 路（Joy1Change 位7）
    if joy & 0x80 != 0 {
        e.push((0x0683, ram[0x0683 & 0x7FF] | 0x01)); // SFXTriInitFlags|=beep
        let cursor = ram[0x0320 & 0x7FF];
        // 格映射（$91A8 链：0-5→+9、6-11→+0A、12-17→+3D、18-23→+3E）
        let pos = if cursor >= 0x12 { cursor.wrapping_add(0x3E) }
                  else if cursor >= 0x0C { cursor.wrapping_add(0x3D) }
                  else if cursor >= 0x06 { cursor.wrapping_add(0x0A) }
                  else { cursor.wrapping_add(0x09) };
        e.push((0x0006, pos));
        let row = ram[0x0321 & 0x7FF] as usize & 0x07;
        let tbl = PW_ROW_TBL[row.min(4)];
        e.push((0x0000, (tbl & 0xFF) as u8));
        e.push((0x0001, (tbl >> 8) as u8));
        let col = ram[0x0322 & 0x7FF] as usize;
        let tile = PW_GRID.get(row.min(4) * 13 + col.min(12)).copied().unwrap_or(0xFF);
        e.push((0x0781, tile));       // VRAMStringRAMBuffer
        e.push((0x0780, 0x11));       // 维面（1×1）
        // PrepareWriteVRAMStringRAM：$00←pos/$01←$21/$02←$80/$03←$07
        e.push((0x0000, pos)); e.push((0x0001, 0x21));
        e.push((0x0002, 0x80)); e.push((0x0003, 0x07));
        // WriteVRAMString：$1B←1；$05←1/$04←1；X←$07A0 重载；写 [21,pos,01,tile]；$07A0←X+4
        e.push((0x001B, 1));
        e.push((0x0005, 0x01)); e.push((0x0004, 0x01));
        e.push((0x07A1 + (idx as u16 & 0x7F), 0x21));
        e.push((0x07A1 + ((idx as u16 + 1) & 0x7F), pos));
        e.push((0x07A1 + ((idx as u16 + 2) & 0x7F), 0x01));
        e.push((0x07A1 + ((idx as u16 + 3) & 0x7F), tile));
        e.push((0x07A0, ((idx as u16 + 4) & 0xFF) as u8));
        // PasswordChar[cursor]←tile + 光标进位回绕
        e.push((0x699A + cursor as u16, tile));
        let nc = if cursor >= 0x17 { 0 } else { cursor.wrapping_add(1) };
        e.push((0x0320, nc));
    }
    // ⑥ B 路（位6——退格回绕）
    if joy & 0x40 != 0 {
        let cursor = ram[0x0320 & 0x7FF];
        let nc = cursor.wrapping_sub(1);
        let nc = if cursor == 0 { 0x17 } else { nc };
        e.push((0x0320, nc));
    }
    // ⑦ 光标闪（FrameCount 位3 门）：SpriteRAM.1 面
    let cursor_now = ram[0x0320 & 0x7FF]; // 注：体序=进/退格后重读
    if frame_count & 0x08 != 0 {
        let y = if cursor_now < 0x0C { 0x3F } else { 0x4F };
        e.push((0x0210, y));
        e.push((0x0211, 0x6E));
        e.push((0x0212, 0x20)); // OAMDATA_PRIORITY
        let xi = (if cursor_now >= 0x0C { cursor_now.wrapping_sub(0x0C) } else { cursor_now }) as usize;
        e.push((0x0213, CURSOR_X_TBL.get(xi.min(11)).copied().unwrap_or(0x48)));
    }
    // ⑧ dpad（Joy1Retrig 低半字节门）：beep+行/列回绕+CharSelect 精灵（位3 门）
    let dpad = joy_retrig & 0x0F;
    if dpad != 0 {
        e.push((0x0683, ram[0x0683 & 0x7FF] | 0x08)); // beep（门内——注释 ✓）
        let mut row = ram[0x0321 & 0x7FF];
        let mut col = ram[0x0322 & 0x7FF];
        if dpad & 0x01 != 0 { // right
            col = col.wrapping_add(1);
            if col == 0x0D { col = 0; row = row.wrapping_add(1); if row == 5 { row = 0; } }
            e.push((0x0321, row)); e.push((0x0322, col));
        }
        if dpad & 0x02 != 0 { // left
            if col == 0 { col = 0x0C; row = if row == 0 { 4 } else { row.wrapping_sub(1) }; }
            else { col = col.wrapping_sub(1); }
            e.push((0x0321, row)); e.push((0x0322, col));
        }
        if dpad & 0x04 != 0 { // down
            row = row.wrapping_add(1); if row == 5 { row = 0; }
            e.push((0x0321, row));
        }
        if dpad & 0x08 != 0 { // up
            row = if row == 0 { 4 } else { row.wrapping_sub(1) };
            e.push((0x0321, row));
        }
        if frame_count & 0x08 != 0 {
            e.push((0x0220, CHARSEL_Y_TBL.get(row as usize % 5).copied().unwrap_or(0x77)));
            e.push((0x0221, 0x6E));
            e.push((0x0222, 0x20));
            e.push((0x0223, CHARSEL_X_TBL.get(col as usize % 13).copied().unwrap_or(0x20)));
        }
    }
    (e, None)
}

// ---- 跨域点件：MainTitleRoutine 头（$8000-$8027——START→菜单转移在分派头非臂内）----
// 结构互核注：①转移目标=**0x1B**（StartContinueScreen1B——非 0x15！0x15/0x1B 共体 $90BA
// 双入口）②guard=TitleRoutine < _id_StartContinueScreen15(0x15)（intro 期判定）+
// Joy1Change&BUTTON_START($10) ③转移副作用：SpareMemD1($D1)/BB($BB)/B7($B7)/B8($B8)←0
// 清+PPUCTRL_ZP($FF)&=$FC（名表 0）。

/// MainTitleRoutine 头语义折叠：返回 (转移命中?, 事件, 新 TitleRoutine)。
/// 转移命中⇒事件=四 spare 清+$FF&=$FC；未命中⇒(false, [], 原值)。
pub fn title_tick_head(title_routine: u8, joy_change: u8, ppuctrl: u8)
    -> (bool, Vec<(u16, u8)>, u8) {
    if title_routine >= 0x15 {
        return (false, Vec::new(), title_routine);
    }
    if joy_change & 0x10 == 0 {
        return (false, Vec::new(), title_routine);
    }
    let events = vec![
        (0x00D1, 0), (0x00BB, 0), (0x00B7, 0), (0x00B8, 0), // spare 清
        (0x00FF, ppuctrl & 0xFC),                           // PPUCTRL_ZP&=$FC
    ];
    (true, events, 0x1B)
}

#[cfg(test)]
mod tick_head_tests {
    use super::*;

    /// 头转移三态（guard/副作用/目标 0x1B）。
    #[test]
    fn title_tick_head_three_faces() {
        // guard 未过（已是菜单期）
        let (hit, ev, r) = title_tick_head(0x1B, 0x10, 0x91);
        assert!(!hit && ev.is_empty() && r == 0x1B);
        // START 未按
        let (hit, _, r) = title_tick_head(4, 0x00, 0x91);
        assert!(!hit && r == 4);
        // 转移命中（intro 期+START）
        let (hit, ev, r) = title_tick_head(4, 0x10, 0x91);
        assert!(hit);
        assert_eq!(r, 0x1B);
        assert!(ev.contains(&(0x00D1, 0)));
        assert!(ev.contains(&(0x00FF, 0x90))); // 0x91&0xFC
    }
}
