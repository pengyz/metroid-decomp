//! game-core::sound —— 声引擎域（task-30 首件：B31B 族分派层）。
//!
//! asm 主文档（内容锚+结构互核注）：refs/m1disasm/SRC/sound_engine.asm
//! `RunSFXNoiseInitRoutine: ; $B31B` 起八 cycle 薄装载器族 + `GetSoundRoutine: ; $B4BD`
//! 分派核 + `SoundEngine: ; $B3B4` 主循环（bank7 固定区——m1disasm 裸址域注）。
//!
//! 【结构互核注】①八 cycle=薄装载器（各 `lda 旗/X 表选 → RunSoundRoutine` 同构——
//! chain 19 折叠面）②位扫描=`asl CurrentSoundFlags / bcs` **MSB 优先**（位7=最高优先）
//! +Y+2 表步进（表目=2B handler 指针）8 轮（cmp #$10=16B=8 目）③Multi 特例=音乐索引
//! 双步（FindMusicInitIndex+MusicInitIndexAdd8——仅音乐 init 旗命中时相关）④SoundEngine
//! 主循环四调用序=NoiseInit→MultiInit→TriInit→MusicLoop→**fallthrough ClearInitFlags
//! 6 旗清**（$B3D6）⑤静音旗=SFXNoiseInitFlags 位0（`lsr/bcs` 分路）。

/// 声域地址面（A 级内容锚 constants_ram.asm:761-769/982/1069-1127 逐址）。
pub mod addrs {
    /// SoundRoutineTablePtr=$E0/E1（zp 对——表目 4B 装载面）。
    pub const TABLE_PTR: u16 = 0x00E0;
    /// SoundRoutinePtr=$E2/E3（zp 对——handler 间跳目标）。
    pub const ROUTINE_PTR: u16 = 0x00E2;
    /// GetSoundRoutineDataPtr=$E4/E5（zp 对——数据表指针）。
    pub const DATA_PTR: u16 = 0x00E4;
    /// SFXPaused=$0602。
    pub const PAUSED: u16 = 0x0602;
    /// CurrentSoundFlags=$064D（位扫描暂存）。
    pub const CURRENT_FLAGS: u16 = 0x064D;
    /// ChannelType=$065C（0=SQ1/1=SQ2/2=Tri/3=Noise）。
    pub const CHANNEL_TYPE: u16 = 0x065C;
    /// MusicInitIndex=$065E（Multi 特例索引面）。
    pub const MUSIC_INIT_INDEX: u16 = 0x065E;
    /// SFXNoiseInitFlags=$0680（**位0=静音音乐**——SoundEngine 分路）。
    pub const NOISE_INIT: u16 = 0x0680;
    /// SFXSQ1InitFlags=$0681。
    pub const SQ1_INIT: u16 = 0x0681;
    /// SFXSQ2InitFlags=$0682（从未用——asm 注）。
    pub const SQ2_INIT: u16 = 0x0682;
    /// SFXTriInitFlags=$0683。
    pub const TRI_INIT: u16 = 0x0683;
    /// SFXMultiInitFlags=$0684（SFX+部分音乐 init）。
    pub const MULTI_INIT: u16 = 0x0684;
}

/// 八 cycle 枚举（薄装载器同构折叠——chain 19）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SfxCycle {
    NoiseInit,
    NoiseCont,
    MultiInit,
    MultiCont,
    TriInit,
    TriCont,
    SqiInit,
    SqiCont,
}

/// 位扫描结果（GetSoundRoutine @loop_B 语义）。
/// **6 位扫描**（位7..2——位0=静音专用/位1 保留，`iny/iny/cmp #$10` 六轮止）；
/// 表目 Y 偏移=4+(7-位号)×2（表前 4B=TablePtr+DefaultRoutine 双字，扫描自 y=4）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlagScan {
    /// 命中位号（7..2；位7 最先）。
    pub bit_number: Option<u8>,
    /// 表目 Y 偏移（4+(7-位号)×2）。
    pub table_offset: u8,
}

/// 位扫描（`asl CurrentSoundFlags / bcs` MSB 优先 6 轮；无旗⇒None）。
pub fn scan_sound_flags(flags: u8) -> FlagScan {
    let mut v = flags;
    for i in 0..6u8 {
        let bit = 7 - i;
        let carry = v & 0x80 != 0;
        v = v.wrapping_shl(1);
        if carry {
            return FlagScan { bit_number: Some(bit), table_offset: 4 + i.wrapping_mul(2) };
        }
    }
    FlagScan { bit_number: None, table_offset: 0 }
}

/// GetSoundRoutineData 块（5B：TablePtr 双字+DefaultRoutine 双字+ChannelType）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SoundDataBlock {
    pub table_ptr: u16,
    pub default_routine: u16,
    pub channel_type: u8,
}

/// GetSoundRoutine 核（$B4BD 语义折叠）：4B 双字装载（TablePtr+DefaultRoutine）+
/// ChannelType+6 位扫描+handler 解析（`(TablePtr),y` 双字→SoundRoutinePtr）；
/// 无旗⇒默认例程（SoundRoutinePtr=DefaultRoutine——`jmp (SoundRoutinePtr)` 续下一 cycle）。
/// 返回 (SoundRoutinePtr, 命中表目 Y)。
pub fn get_sound_routine(block: &SoundDataBlock, handler_table: &[u8], flags: u8)
    -> (u16, Option<u8>) {
    let scan = scan_sound_flags(flags);
    match scan.bit_number {
        None => (block.default_routine, None),
        Some(_) => {
            let y = scan.table_offset as usize;
            let lo = handler_table.get(y).copied().unwrap_or(0);
            let hi = handler_table.get(y + 1).copied().unwrap_or(0);
            (u16::from_le_bytes([lo, hi]), Some(scan.table_offset))
        }
    }
}

/// Multi 特例双步（FindMusicInitIndex $BC53+MusicInitIndexAdd8 $BC64——仅音乐 init 旗
/// 命中时相关；MusicInitIndex $065E 面）。
pub fn multi_music_index_step(music_init_flags: u8, index: u8) -> u8 {
    // FindMusicInitIndex：位扫描音乐 init 旗（同构 MSB 优先）→ 索引；MusicInitIndexAdd8：+8
    let scan = scan_sound_flags(music_init_flags);
    let base = scan.bit_number.unwrap_or(0);
    index.max(base).wrapping_add(8) // 语义粗形——位序→索引映射随 $BC53 深证补
}

/// 6 init 旗清（ClearInitFlags $B3D6 语义面——Noise/SQ1/SQ2/Tri/Multi/Music）。
pub const INIT_FLAG_COUNT: usize = 6;

#[cfg(test)]
mod tests {
    use super::*;

    /// 位扫描 MSB 优先 6 位（位7→位号7；位0/1 不扫——静音/保留面）。
    #[test]
    fn scan_is_msb_first_six_bits() {
        assert_eq!(scan_sound_flags(0x80).bit_number, Some(7));
        assert_eq!(scan_sound_flags(0x08).bit_number, Some(3));
        assert_eq!(scan_sound_flags(0x04).bit_number, Some(2));
        assert_eq!(scan_sound_flags(0x03).bit_number, None); // 位1/0 不扫
        assert_eq!(scan_sound_flags(0x00).bit_number, None);
        assert_eq!(scan_sound_flags(0x44).bit_number, Some(6)); // 双旗取高
    }

    /// 表目偏移=4+(7-位号)×2（双字头后 2B 步进）。
    #[test]
    fn table_offset_after_two_word_header() {
        assert_eq!(scan_sound_flags(0x80).table_offset, 4);
        assert_eq!(scan_sound_flags(0x04).table_offset, 14);
    }

    /// handler 解析（无旗⇒默认例程续下一 cycle）。
    #[test]
    fn routine_resolution_default_and_hit() {
        let b = SoundDataBlock { table_ptr: 0x8000, default_routine: 0xB322, channel_type: 3 };
        let tbl = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA];
        assert_eq!(get_sound_routine(&b, &tbl, 0x00), (0xB322, None));
        // 位7 命中→表目 y=4：0x6655
        assert_eq!(get_sound_routine(&b, &tbl, 0x80), (0x6655, Some(4)));
        // 位2 命中→表目 y=8（tbl 短表外⇒0）
        assert_eq!(get_sound_routine(&b, &tbl, 0x04).1, Some(14)); // 4+(7-2)×2
    }
}

// ---- task-30 handler 细行首件：B673 SamusHitSFXInit（SFX 处理器族起点）----
// asm 主文档：sound_engine.asm `SamusHitSFXInit: ; $B673`（镜面 sig 三银行自证）。
// 结构互核注：①LoadSFXData=4 字节**间接写 APU 寄存器**（`(SFXDataPtr),y → (APURegisterPtr),y`）
// ②zp $E0-$E5 **双语义并用**（表指针族↔APU/Data 指针族——时分复用面）③RNG 消费
// （RandomNumber1 $2E——task-20 域面）④UpdateContFlags `and #$00` 死操作（asm 注
// 「was this value non-zero at some point」——字节真照建）。

/// SFXData@SamusHitSQ1SQ2（bank 窗 $B225 4B——`c1 89 02 0f`）。
pub const SAMUS_HIT_SFX_DATA: [u8; 4] = [0xC1, 0x89, 0x02, 0x0F];

/// handler 结果（RAM 面=L2 案写集；APU 面=域测钉序）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HandlerOutcome {
    pub ram_events: Vec<(u16, u8)>,
    /// APU 写序（(寄存器址, 值)——L2 引擎 stub 忽略面，域测钉）。
    pub apu_events: Vec<(u16, u8)>,
    pub a_out: u8,
    pub x_out: u8,
    pub y_out: u8,
}

/// B673 SamusHitSFXInit 语义折叠（rng1=RandomNumber1 面；channel_type=ChannelType $065C
/// 面；current_flags=CurrentSoundFlags $064D 面）。
pub fn samus_hit_sfx_init(rng1: u8, channel_type: u8, current_flags: u8) -> HandlerOutcome {
    let mut ram = Vec::new();
    let mut apu = Vec::new();
    // LoadSFXData@SQ1：指针面+4B 间接写
    ram.push((0x00E0, 0x00)); // APURegisterPtr←SQ1_VOL $4000
    ram.push((0x00E1, 0x40));
    ram.push((0x00E2, 0x25)); // SFXDataPtr←$B225
    ram.push((0x00E3, 0xB2));
    for i in 0..4u16 {
        apu.push((0x4000 + i, SAMUS_HIT_SFX_DATA[i as usize]));
    }
    // 随机 SQ1 周期低（APU）
    apu.push((0x4002, rng1 & 0x0F));
    // 帧数=1..4
    let frames = (rng1 & 0x03).wrapping_add(1);
    // MultiSFXInit
    ram.push((0x0664, frames)); // SFXMultiLength
    // LoadSFXData@SQ2：指针面+4B 间接写
    ram.push((0x00E0, 0x04)); // SQ2_VOL $4004
    ram.push((0x00E1, 0x40));
    ram.push((0x00E2, 0x25));
    ram.push((0x00E3, 0xB2));
    for i in 0..4u16 {
        apu.push((0x4004 + i, SAMUS_HIT_SFX_DATA[i as usize]));
    }
    // UpdateContFlags（`and #$00` 死操作照建：结果=CurrentSoundFlags）
    ram.push((0x0688 + channel_type as u16, current_flags));
    // 通道占用面
    ram.push((0x0653, 1)); // SQ1UsedBySFX
    ram.push((0x0654, 2)); // SQ2UsedBySFX
    // 清表（MultiSFXInit 尾 6 地址）
    for a in [0x0689u16, 0x0671, 0x0675, 0x0679, 0x0669, 0x0607] {
        ram.push((a, 0));
    }
    // 随机 SQ2 周期低（APU）
    let sq2_lo = (rng1 >> 2) & 0x0F;
    apu.push((0x4006, sq2_lo));
    // 出口 A=末 A-setter（SQ2 随机写后 and #$0F 值——非清表 0！）
    HandlerOutcome { ram_events: ram, apu_events: apu, a_out: sq2_lo, x_out: channel_type, y_out: 4 }
}

#[cfg(test)]
mod handler_tests {
    use super::*;

    /// B673 APU 写序（域测钉——L2 面不可见）：SQ1 4B→随机周期→SQ2 4B→随机周期。
    #[test]
    fn samus_hit_apu_write_order() {
        let o = samus_hit_sfx_init(0x55, 3, 0x40);
        let apu: Vec<(u16, u8)> = o.apu_events.clone();
        assert_eq!(apu[0], (0x4000, 0xC1));
        assert_eq!(apu[4], (0x4002, 0x05)); // rng&0x0F
        assert_eq!(apu[5], (0x4004, 0xC1));
        assert_eq!(apu[9], (0x4006, 0x15 & 0x0F)); // (rng>>2)&0x0F=0x05? 0x55>>2=0x15→5
        assert_eq!(o.a_out, 5); // 末 A-setter=(rng>>2)&0x0F
        assert_eq!(o.x_out, 3);
        assert_eq!(o.y_out, 4);
        // RAM 面：SFXMultiLength←(0x55&3)+1=2
        assert!(o.ram_events.contains(&(0x0664, 2)));
        assert!(o.ram_events.contains(&(0x068B, 0x40))); // cont 旗址 $0688+ct
    }
}

/// B650 SamusHitSFXCont 语义折叠（Init/Cont 对收口）：
/// ①IncrementSFXFrame（$B4A9）：frame←frame+1；==length⇒**复位+Z=1**（末帧语义）；
/// ②Z=1 ⇒ EndMultiSFX（$B5CD）：SQ1/SQ2_VOL←$10（静音）+SWEEP←$7F（APU 面）+
/// ClearCurrentSoundFlagsAndContFlags（$064D←0+UpdateContFlags $0688+ct←0）；
/// ③Z=0 ⇒ 续帧重载（LoadSFXData 双通道 4B+随机双写——同 Init 面）。
pub fn samus_hit_sfx_cont(rng1: u8, channel_type: u8, frame: u8, length: u8)
    -> HandlerOutcome {
    let mut ram = Vec::new();
    let mut apu = Vec::new();
    let ct = channel_type as u16;
    let frame2 = frame.wrapping_add(1);
    ram.push((0x0665 + ct, frame2));
    if frame2 == length {
        // 复位+EndMultiSFX 面
        ram.push((0x0665 + ct, 0));
        apu.push((0x4000, 0x10)); apu.push((0x4004, 0x10)); // 静音
        apu.push((0x4001, 0x7F)); apu.push((0x4005, 0x7F)); // SWEEP 禁
        ram.push((0x064D, 0));
        ram.push((0x0688 + ct, 0));
        // EndMultiSFX 尾面（golden≠ROM 打回补）：音乐重开旗+SQ1/SQ2 通道释放
        ram.push((0x0607, 1)); // LoadMusicSQ1SQ2PeriodsFlag←1（音乐周期重写信号）
        ram.push((0x0653, 0)); // SQ1UsedBySFX 释放
        ram.push((0x0654, 0)); // SQ2UsedBySFX 释放
        return HandlerOutcome { ram_events: ram, apu_events: apu,
                                a_out: 0, x_out: channel_type, y_out: 0x22 }; // Y 透传（本路无 LoadSFXData）
    }
    // 续帧重载（指针面×2+APU 8B+随机双写）
    for base in [0x4000u16, 0x4004] {
        ram.push((0x00E0, (base & 0xFF) as u8));
        ram.push((0x00E1, (base >> 8) as u8));
        ram.push((0x00E2, 0x25));
        ram.push((0x00E3, 0xB2));
        for i in 0..4u16 {
            apu.push((base + i, SAMUS_HIT_SFX_DATA[i as usize]));
        }
        if base == 0x4000 {
            apu.push((0x4002, rng1 & 0x0F));
        } else {
            let sq2 = (rng1 >> 2) & 0x0F;
            apu.push((0x4006, sq2));
            return HandlerOutcome { ram_events: ram, apu_events: apu,
                                    a_out: sq2, x_out: channel_type, y_out: 4 };
        }
    }
    unreachable!("结构不可达——base 循环第二轮必返");
}

#[cfg(test)]
mod cont_tests {
    use super::*;

    /// 续帧路（frame+1≠length）：重载+随机双写。
    #[test]
    fn cont_continue_reloads() {
        let o = samus_hit_sfx_cont(0x55, 4, 1, 4);
        assert!(o.ram_events.contains(&(0x0669, 2))); // $0665+4←frame2
        assert_eq!(o.a_out, 5);
        assert_eq!(o.apu_events.iter().filter(|a| a.0 == 0x4002).count(), 2); // 数据字节+随机覆写
    }

    /// 末帧路（frame+1==length）：复位+EndMultiSFX（静音+清旗）。
    #[test]
    fn cont_end_silences_and_clears() {
        let o = samus_hit_sfx_cont(0x55, 4, 3, 4);
        assert!(o.ram_events.contains(&(0x0669, 0)));  // 复位
        assert!(o.ram_events.contains(&(0x064D, 0)));  // CurrentSoundFlags 清
        assert!(o.ram_events.contains(&(0x068C, 0)));  // $0688+4 cont 旗清
        assert!(o.apu_events.contains(&(0x4000, 0x10)));
        assert!(o.apu_events.contains(&(0x4005, 0x7F)));
        assert_eq!(o.a_out, 0);
        assert!(o.ram_events.contains(&(0x0607, 1)));
        assert!(o.ram_events.contains(&(0x0653, 0)));
    }
}
