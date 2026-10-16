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
