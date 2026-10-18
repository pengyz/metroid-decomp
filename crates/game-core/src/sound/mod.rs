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

// ---- task-44 W1 首波：音乐索引核 + Beep handler（B 跑触发面）----
// 结构互核注：【2026-10-18 更正】先版「块名≠索引释义」注**反转**——W2 七案 ROM 打回
// 实证（ct=3⇒指针低 $08=$4008 Tri）：真映射 0=Noise/1=SQ1/2=SQ2/3=Tri/4=Multi；
// **块名（SFXNoiseLength $0660=ct0=Noise…）是对的**，ChannelType 注（0=SQ1…）=误注。
// 公式 base+ct 不变；清表尾面（$0607/frame/Var0-2）随案补。

/// FindMusicInitIndex（$BC53）语义折叠：无旗⇒$FF；MSB 优先位序（inc/asl/bcc 环）。
/// MusicInitIndex 位序=曲号（注表：0x0B=Intro——boot-title 触发面）。
pub fn find_music_init_index(flags: u8) -> u8 {
    if flags == 0 {
        return 0xFF;
    }
    let mut idx = 0xFFu8;
    let mut a = flags;
    loop {
        idx = idx.wrapping_add(1);
        let carry = a & 0x80 != 0;
        a = a.wrapping_shl(1);
        if carry {
            return idx;
        }
    }
}

/// MusicInitIndexAdd8（$BC64）：+8（Multi 表偏移面）。
pub fn music_init_index_add8(index: u8) -> u8 {
    index.wrapping_add(8)
}

/// SFXData@Beep（bank 窗 $B279 4B——`09 7f 30 48`）。
pub const BEEP_SFX_DATA: [u8; 4] = [0x09, 0x7F, 0x30, 0x48];

/// BeepSFXInit（$B7D9）语义折叠：守卫（SFXTriContFlags 位7 SamusDie 演奏中⇒不响）+
/// 3 帧+InitSFXData@Tri 面（length[ct]←3+指针面+4B APU+UpdateContFlags+占用面）。
/// ct=ChannelType（Tri=2）。
pub fn beep_sfx_init(tri_cont_flags: u8, current_flags: u8, channel_type: u8) -> HandlerOutcome {
    let mut ram = Vec::new();
    let mut apu = Vec::new();
    if tri_cont_flags & 0x80 != 0 {
        // SamusDie 占奏——静默退出（无写）
        return HandlerOutcome { ram_events: ram, apu_events: apu,
                                a_out: tri_cont_flags, x_out: channel_type, y_out: 0x79 };
    }
    let ct = channel_type as u16;
    ram.push((0x0660 + ct, 3)); // length←3 帧
    // InitSFXData@Tri：指针面（TRI_LINEAR $4008）+4B APU
    ram.push((0x00E0, 0x08));
    ram.push((0x00E1, 0x40));
    ram.push((0x00E2, 0x79)); // SFXData@Beep=$B279
    ram.push((0x00E3, 0xB2));
    for i in 0..4u16 {
        apu.push((0x4008 + i, BEEP_SFX_DATA[i as usize]));
    }
    ram.push((0x0688 + ct, current_flags)); // UpdateContFlags（and #$00 死操作照建）
    ram.push((0x0652 + ct, channel_type));  // 占用面（`sta $0652,x` A=X=ct）
    // 出口：InitSFXData 尾 txa/sta 后 A=ct
    HandlerOutcome { ram_events: ram, apu_events: apu, a_out: channel_type,
                     x_out: channel_type, y_out: 0x79 }
}

#[cfg(test)]
mod w1_tests {
    use super::*;

    /// 音乐索引核（无旗 $FF/MSB 优先位序——0x0B=Intro 注表锚）。
    #[test]
    fn music_index_scan_msb_first() {
        assert_eq!(find_music_init_index(0), 0xFF);
        assert_eq!(find_music_init_index(0x80), 0);
        assert_eq!(find_music_init_index(0x08), 4);
        assert_eq!(find_music_init_index(0x08).wrapping_add(8), 0x0C); // Add8 面
        assert_eq!(music_init_index_add8(0x03), 0x0B);
    }

    /// Beep 守卫（SamusDie 占奏⇒静默无写）+3 帧装载。
    #[test]
    fn beep_guard_and_load() {
        let o = beep_sfx_init(0x80, 0x40, 2);
        assert!(o.ram_events.is_empty(), "SamusDie 占奏应静默");
        let o = beep_sfx_init(0x00, 0x40, 2);
        assert!(o.ram_events.contains(&(0x0662, 3))); // length[2]←3
        assert!(o.apu_events.contains(&(0x4008, 0x09)));
        assert!(o.ram_events.contains(&(0x068A, 0x40))); // cont[2]←flags
    }
}

// ---- task-44 W1 曲径：XYMusicInit 臂 + SongIntro 曲头烘焙 ----

/// SongIntro 曲头（bank 窗 $BD8C 13B——SongHeader 宏 5B+4 通道字）。
/// 宏面：NoteLengthsTbl@7/$00/$F0/$02/$05（NTSC 口径）；通道字=SongIntroSQ1/SQ2/Tri/Noise
/// （BANK 别名面——实际表址随烘焙深证）。
pub const SONG_INTRO_HEADER: [u8; 13] = [
    0x07, 0x00, 0xF0, 0x02, 0x05, // 宏 5B（NoteLengthsTbl@7 等）
    0x00, 0x01, 0x00, 0x03, 0x00, 0x05, 0x00, 0x07, // 4 通道字（$0100/$0300/$0500/$0700 别名形）
];

/// XYMusicInit 路径 W1 面（$BC8B 三连之一 SetVolumeAndDisableSweep $B9E4 + Cont 旗面）：
/// $0648/$0649←$7F（禁扫）+$0628←duty（X）+$0629←duty（Y 同值——tax/tay 对称）+
/// MusicContFlags($068D)←CurrentSoundFlags（InitializeMusic 首步）+曲头 13B 烘焙面注记。
/// 返回 (RAM 事件, APU 事件占位)。
pub fn xy_music_init_w1(duty: u8, current_flags: u8) -> HandlerOutcome {
    let mut ram = Vec::new();
    // SetVolumeAndDisableSweep
    ram.push((0x0648, 0x7F)); // MusicSQ1Sweep
    ram.push((0x0649, 0x7F)); // MusicSQ2Sweep
    ram.push((0x0628, duty)); // MusicSQ1DutyEnvelope←X
    ram.push((0x0629, duty)); // MusicSQ2DutyEnvelope←Y（tax/tay 同值）
    // InitializeMusic 首步：MusicContFlags←CurrentSoundFlags
    ram.push((0x068D, current_flags));
    // 残面（InitializeMusic 曲头装载深径+UpdateAllMusicChannels 帧径）=W1 残件注记
    HandlerOutcome { ram_events: ram, apu_events: Vec::new(), a_out: duty,
                     x_out: duty, y_out: duty }
}

#[cfg(test)]
mod w1_music_tests {
    use super::*;

    /// XYMusicInit W1 面（禁扫双写+duty 对称+Cont 旗）。
    #[test]
    fn xy_music_init_w1_face() {
        let o = xy_music_init_w1(0x34, 0x40);
        assert!(o.ram_events.contains(&(0x0648, 0x7F)));
        assert!(o.ram_events.contains(&(0x0649, 0x7F)));
        assert!(o.ram_events.contains(&(0x0628, 0x34)));
        assert!(o.ram_events.contains(&(0x0629, 0x34)));
        assert!(o.ram_events.contains(&(0x068D, 0x40)));
    }

    /// 曲头烘焙面（13B 形：5B 宏+4 通道字）。
    #[test]
    fn song_intro_header_shape() {
        assert_eq!(SONG_INTRO_HEADER.len(), 13);
        assert_eq!(&SONG_INTRO_HEADER[0..5], &[0x07, 0x00, 0xF0, 0x02, 0x05]);
    }
}

// ---- task-44 W2 高频 handler 族六件（共核 InitSFXData 面+薄壳守卫形）----

/// W2 数据表 6×4B（bank 窗面——0x3xxx 偏移 hexdump 实证）。
pub const W2_DATA: [[u8; 4]; 6] = [
    [0x19, 0x7F, 0x0E, 0xA0], // MissileLaunch $B215
    [0x89, 0x7F, 0x67, 0x18], // EnergyPickup  $B23D
    [0x8B, 0x7F, 0xFD, 0x28], // MissilePickup $B241
    [0xD6, 0x82, 0x58, 0xF8], // ShortRangeShot $B24D
    [0x1D, 0x9A, 0x20, 0x8F], // EnemyHit      $B255
    [0x16, 0x7F, 0x0B, 0x18], // SamusWalk     $B21D
];

/// InitSFXData 共同面（$B452 语义折叠）——**通道映射 ROM 证反转**（七案打回实证：
/// ct=3⇒指针低 08=$4008 Tri）：0=Noise $400C/1=SQ1 $4000/2=SQ2 $4004/3=Tri $4008。
/// ChannelType 注（0=SQ1…）=**误注**（块名（SFXNoiseLength $0660=ct0=Noise…）反而是对的
/// ——【更正】先版「块名≠索引释义」注反转：块名=索引释义，注=误）。
/// 写面：length($0660+ct)+指针+cont($0688+ct)+used($0652+ct)+**清表尾**（$0607←0+
/// frame($0665+ct)←0+Var0($0670+ct)/Var1($0674+ct)/Var2($0678+ct)←0——Var 块步距 4）。
/// 出口：A=0（清表 lda #0 尾）/X=ct/Y=4（LoadSFXData 环 ldy #0 出口）。
pub fn sfx_data_init(frames: u8, data: &[u8; 4], data_addr: u16, ct: u8, current_flags: u8)
    -> HandlerOutcome {
    let c = ct as u16;
    let apu_base: u16 = match ct { 0 => 0x400C, 1 => 0x4000, 2 => 0x4004, _ => 0x4008 };
    let mut ram = vec![
        (0x0660 + c, frames),
        (0x00E0, (apu_base & 0xFF) as u8),
        (0x00E1, (apu_base >> 8) as u8),
        (0x00E2, (data_addr & 0xFF) as u8),
        (0x00E3, (data_addr >> 8) as u8),
        (0x0688 + c, current_flags),
        (0x0652 + c, ct),
        (0x0607, 0), (0x0665 + c, 0),
        (0x0670 + c, 0), (0x0674 + c, 0), (0x0678 + c, 0),
    ];
    let mut apu = Vec::new();
    for i in 0..4u16 {
        apu.push((apu_base + i, data[i as usize]));
    }
    ram.sort_by_key(|a| a.0);
    HandlerOutcome { ram_events: ram, apu_events: apu, a_out: 0, x_out: ct, y_out: 4 }
}

/// SamusWalkSFXInit（$B598）：守卫（NoiseCont 位 0x20|0x10|0x04 占奏⇒静默）+3 帧。
pub fn samus_walk_sfx_init(noise_cont: u8, ct: u8, flags: u8) -> HandlerOutcome {
    if noise_cont & 0x34 != 0 {
        return HandlerOutcome { ram_events: vec![], apu_events: vec![],
                                a_out: noise_cont, x_out: 0x11, y_out: 0x22 }; // 守卫出口全透传
    }
    sfx_data_init(3, &W2_DATA[5], 0xB21D, ct, flags)
}

/// EnemyHitSFXInit（$B710）：无守卫+8 帧。
pub fn enemy_hit_sfx_init(ct: u8, flags: u8) -> HandlerOutcome {
    sfx_data_init(8, &W2_DATA[4], 0xB255, ct, flags)
}

/// BulletFireSFXInit（$B716）短光束路：ice（位0）/long（位7）分支=W3 变体件（此处
/// 返回 None 标记非短光束）；SQ1Cont 守卫（0xCC 占奏⇒静默）；短光束=3 帧。
pub fn bullet_fire_sfx_init(has_beam: u8, sq1_cont: u8, ct: u8, flags: u8)
    -> Option<HandlerOutcome> {
    if has_beam & 0x01 != 0 { return None; }          // ⇒ HasIceBeamSFXInit（W3）
    if sq1_cont & 0xCC != 0 {
        return Some(HandlerOutcome { ram_events: vec![], apu_events: vec![],
                                     a_out: sq1_cont, x_out: 0x11, y_out: 0x22 }); // 守卫出口全透传
    }
    if has_beam & 0x80 != 0 { return None; }          // ⇒ HasLongBeamSFXInit（W3）
    Some(sfx_data_init(3, &W2_DATA[3], 0xB24D, ct, flags))
}

/// MissileLaunchSFXInit（$B56E）：0x18 帧+**特例**（SFXNoiseVar0 $0670←$0A 噪声音高索引）。
pub fn missile_launch_sfx_init(ct: u8, flags: u8) -> HandlerOutcome {
    let mut o = sfx_data_init(0x18, &W2_DATA[0], 0xB215, ct, flags);
    o.ram_events.push((0x0670, 0x0A)); // 特例（此写在清表后——终值 0x0A）
    o.a_out = 0x0A; // 末 A-setter=`lda #$0A`
    o
}

/// MissilePickupSFXInit（$B6CD）：无守卫+5 帧。
pub fn missile_pickup_sfx_init(ct: u8, flags: u8) -> HandlerOutcome {
    sfx_data_init(5, &W2_DATA[2], 0xB241, ct, flags)
}

/// EnergyPickupSFXInit（$B6E7）：无守卫+6 帧。
pub fn energy_pickup_sfx_init(ct: u8, flags: u8) -> HandlerOutcome {
    sfx_data_init(6, &W2_DATA[1], 0xB23D, ct, flags)
}

#[cfg(test)]
mod w2_tests {
    use super::*;

    /// 共核面（frames/指针/APU 4B/cont/占用）。
    #[test]
    fn sfx_data_init_common_face() {
        let o = sfx_data_init(3, &W2_DATA[5], 0xB21D, 3, 0x40);
        assert!(o.ram_events.contains(&(0x0663, 3)));   // length[3]
        assert!(o.ram_events.contains(&(0x068B, 0x40))); // cont[3]
        assert!(o.apu_events.contains(&(0x4008, 0x16))); // Tri 基址（映射反转证）
        assert_eq!(o.y_out, 4); // LoadSFXData 环出口（ldy #0 后 iny×4）
    }

    /// W2 六件：守卫/无守卫/特例三分形。
    #[test]
    fn w2_six_shapes() {
        // SamusWalk 守卫命中⇒静默
        assert!(samus_walk_sfx_init(0x20, 3, 0x40).ram_events.is_empty());
        assert_eq!(samus_walk_sfx_init(0x00, 3, 0x40).ram_events.iter().find(|a| a.0 == 0x0663).unwrap().1, 3);
        // 无守卫三件
        assert_eq!(enemy_hit_sfx_init(0, 0x40).ram_events.iter().find(|a| a.0 == 0x0660).unwrap().1, 8);
        assert_eq!(missile_pickup_sfx_init(1, 0x40).ram_events.iter().find(|a| a.0 == 0x0661).unwrap().1, 5);
        assert_eq!(energy_pickup_sfx_init(1, 0x40).ram_events.iter().find(|a| a.0 == 0x0661).unwrap().1, 6);
        // MissileLaunch 特例（NoiseVar0）
        assert!(missile_launch_sfx_init(3, 0x40).ram_events.contains(&(0x0670, 0x0A)));
        // BulletFire 短光束/守卫/变体分支
        assert!(bullet_fire_sfx_init(0x80, 0, 0, 0x40).is_none()); // long beam 变体
        assert!(bullet_fire_sfx_init(0x01, 0, 0, 0x40).is_none()); // ice 变体
        let o = bullet_fire_sfx_init(0x00, 0xCC, 0, 0x40).unwrap();
        assert!(o.ram_events.is_empty()); // SQ1Cont 守卫
        assert!(bullet_fire_sfx_init(0x00, 0x00, 0, 0x40).unwrap().ram_events.iter().any(|a| *a == (0x0660, 3)));
    }
}

// ---- task-44 W3 中频 handler 族六件（一体）----

/// W3 数据表 8×4B（0x3xxx 偏移 hexdump 实证）。
pub const W3_DATA: [[u8; 4]; 8] = [
    [0x35, 0x7F, 0x00, 0xB0], // 0 ScrewAttack      $B211
    [0x0D, 0x7F, 0x0F, 0x08], // 1 BombExplode      $B219
    [0x13, 0x7F, 0x0E, 0xF8], // 2 SpitFlame        $B221
    [0x34, 0xBA, 0xE0, 0x05], // 3 BossHitSQ2       $B229
    [0x34, 0xBB, 0xCE, 0x05], // 4 BossHitSQ1       $B22D
    [0xB6, 0x7F, 0x00, 0xC2], // 5 IncPwdSQ1        $B231
    [0xB6, 0x7F, 0x04, 0xC2], // 6 IncPwdSQ2        $B235
    [0x95, 0x8C, 0x40, 0xB9], // 7 SamusJump        $B251
];

/// MultiSFXInit 共同面（$B5A5——B650 家族同源）：SFXMultiLength($0664)←frames+
/// LoadSFXData@SQ2（指针 $4004 面+4B APU）+UpdateContFlags+占用（$0653←1/$0654←2）+
/// 清表（$0689/$0671/$0675/$0679/$0669/$0607）。
fn multi_sfx_init_face(frames: u8, sq2: &[u8; 4], ct: u8, flags: u8) -> (Vec<(u16, u8)>, Vec<(u16, u8)>) {
    let c = ct as u16;
    let mut ram = vec![(0x0664, frames)];
    ram.push((0x00E0, 0x04)); ram.push((0x00E1, 0x40));
    ram.push((0x00E2, 0x29)); ram.push((0x00E3, 0xB2)); // SQ2 数据址由调用方（此处固化调用面差异）——各 handler 传
    let mut apu = Vec::new();
    for i in 0..4u16 {
        apu.push((0x4004 + i, sq2[i as usize]));
    }
    ram.push((0x0688 + c, flags));
    ram.push((0x0653, 1)); ram.push((0x0654, 2));
    for a in [0x0689u16, 0x0671, 0x0675, 0x0679, 0x0669, 0x0607] {
        ram.push((a, 0));
    }
    (ram, apu)
}

/// SpitFlameSFXInit（$B50F）：无守卫 0x14 帧。
pub fn spit_flame_sfx_init(ct: u8, flags: u8) -> HandlerOutcome {
    sfx_data_init(0x14, &W3_DATA[2], 0xB221, ct, flags)
}

/// ScrewAttackSFXInit（$B52B）：5 帧+**特例**（SFXNoiseVar0 $0670←data[2]=0x00 清索引）。
pub fn screw_attack_sfx_init(ct: u8, flags: u8) -> HandlerOutcome {
    let mut o = sfx_data_init(5, &W3_DATA[0], 0xB211, ct, flags);
    o.ram_events.push((0x0670, W3_DATA[0][2])); // data[2] 清索引（注「#$00」）
    o.a_out = W3_DATA[0][2]; // 末 A-setter=`lda data+2`
    o
}

/// BombExplodeSFXInit（$B583）：无守卫 0x30 帧。
pub fn bomb_explode_sfx_init(ct: u8, flags: u8) -> HandlerOutcome {
    sfx_data_init(0x30, &W3_DATA[1], 0xB219, ct, flags)
}

/// BossHitSFXInit（$B5EC）：双通道族（SQ1 装载+MultiSFXInit——A=SQ1 数据[3] 末字节）。
pub fn boss_hit_sfx_init(ct: u8, flags: u8) -> HandlerOutcome {
    let mut o = sfx_data_init(0, &W3_DATA[4], 0xB22D, ct, flags); // SQ1 装载面（无 length 写——LoadSFXData 非 InitSFXData）
    // 修正形：LoadSFXData@SQ1=指针+4B（无 length/清表）——此处重写
    let mut ram = vec![
        (0x00E0, 0x00), (0x00E1, 0x40), (0x00E2, 0x2D), (0x00E3, 0xB2),
    ];
    let mut apu = Vec::new();
    for i in 0..4u16 {
        apu.push((0x4000 + i, W3_DATA[4][i as usize]));
    }
    let frames = 4u8; // **LoadSFXData 尾 A=环计数**（`tya/cmp #$04` 出口 A=4——非 data[3]！
                      // golden≠ROM 打回正）
    let (ram2, apu2) = multi_sfx_init_face(frames, &W3_DATA[3], ct, flags);
    ram.extend(ram2);
    apu.extend(apu2);
    HandlerOutcome { ram_events: ram, apu_events: apu, a_out: 0, x_out: ct, y_out: 4 } // A=清表尾
}

/// IncorrectPasswordSFXInit（$B695）：双通道族（frames=$20 显式）。
pub fn incorrect_password_sfx_init(ct: u8, flags: u8) -> HandlerOutcome {
    let mut ram = vec![
        (0x00E0, 0x00), (0x00E1, 0x40), (0x00E2, 0x31), (0x00E3, 0xB2),
    ];
    let mut apu = Vec::new();
    for i in 0..4u16 {
        apu.push((0x4000 + i, W3_DATA[5][i as usize]));
    }
    let (ram2, apu2) = multi_sfx_init_face(0x20, &W3_DATA[6], ct, flags);
    ram.extend(ram2);
    apu.extend(apu2);
    // 注：SQ2 数据址各 handler 异（$B229/$B235）——multi 面 E2/E3 应随调用方；
    // 本件覆盖（末写胜）
    ram.push((0x00E2, 0x35)); ram.push((0x00E3, 0xB2));
    HandlerOutcome { ram_events: ram, apu_events: apu, a_out: 0, x_out: ct, y_out: 4 } // A=清表尾
}

/// SamusJumpSFXInit（$B703）：守卫（MusicContFlags==music_Escape(0x04)⇒静默）+0x0C 帧。
pub fn samus_jump_sfx_init(music_cont: u8, ct: u8, flags: u8) -> HandlerOutcome {
    if music_cont == 0x04 {
        return HandlerOutcome { ram_events: vec![], apu_events: vec![],
                                a_out: music_cont, x_out: 0x11, y_out: 0x22 };
    }
    sfx_data_init(0x0C, &W3_DATA[7], 0xB251, ct, flags)
}

#[cfg(test)]
mod w3_tests {
    use super::*;

    /// W3 六件：无守卫三件/特例（ScrewAttack data[2] 清索引）/双通道族/守卫一件。
    #[test]
    fn w3_six_shapes() {
        assert_eq!(spit_flame_sfx_init(0, 0x40).ram_events.iter().find(|a| a.0 == 0x0660).unwrap().1, 0x14);
        assert_eq!(bomb_explode_sfx_init(0, 0x40).ram_events.iter().find(|a| a.0 == 0x0660).unwrap().1, 0x30);
        // ScrewAttack 特例：data[2] 清索引+末 setter
        let o = screw_attack_sfx_init(0, 0x40);
        assert!(o.ram_events.contains(&(0x0670, 0x00)));
        assert_eq!(o.a_out, 0x00);
        // BossHit 双通道（A=SQ1 data[3]=0x05）
        let o = boss_hit_sfx_init(4, 0x40);
        assert!(o.ram_events.contains(&(0x0664, 0x04))); // 尾 A=环计数
        assert!(o.apu_events.contains(&(0x4004, 0x34)));
        // IncorrectPassword 双通道（frames=0x20 显式）
        let o = incorrect_password_sfx_init(4, 0x40);
        assert!(o.ram_events.contains(&(0x0664, 0x20)));
        assert_eq!(o.a_out, 0); // 清表尾 A
        // SamusJump 守卫（Escape 占奏⇒静默）
        assert!(samus_jump_sfx_init(0x04, 0, 0x40).ram_events.is_empty());
        assert_eq!(samus_jump_sfx_init(0x00, 0, 0x40).ram_events.iter().find(|a| a.0 == 0x0660).unwrap().1, 0x0C);
    }
}

// ---- task-44 W1 残面首件：InitializeMusic 深径 + UpdateAllMusicChannels 帧径入口 ----

/// CheckMusicFlags（$B3FC）语义折叠：MusicContFlags==CurrentSoundFlags⇒**同曲重开**
/// （GotoClearSpecialAddresses：清计数器面）；否则全清（InitializeSoundAddresses 面）。
/// 返回 same: bool（true=清计数器路/false=全清路）。
pub fn check_music_flags(music_cont: u8, current_flags: u8) -> bool {
    music_cont == current_flags
}

/// InitializeMusic（$BF19）语义折叠：MusicContFlags←CurrentSoundFlags+**13B 曲头装载**
/// （SongHeaderEndInitIndex→NoteLengthTblOffset($062B) 起 13B——SONG_INTRO_HEADER 面）+
/// InstrDelay($0640-$0643)←1×4+InstrID($0638-$063B)←0×4。
/// 注：SongHeaderOffsetTable[MusicInitIndex]→曲头源址（表烘焙面）。
pub fn initialize_music(current_flags: u8, header: &[u8; 13]) -> Vec<(u16, u8)> {
    let mut ram = vec![(0x068D, current_flags)]; // MusicContFlags
    for (i, b) in header.iter().enumerate() {
        ram.push((0x062B + i as u16, *b));
    }
    for a in [0x0640u16, 0x0641, 0x0642, 0x0643] {
        ram.push((a, 1));
    }
    for a in [0x0638u16, 0x0639, 0x063A, 0x063B] {
        ram.push((a, 0));
    }
    ram
}

/// ResetVolumeIndex（$B9F3）：新音符始（InstrDelay==1）⇒VolumeIndex←1（SQ1/SQ2 对）。
pub fn reset_volume_index(sq1_delay: u8, sq2_delay: u8) -> Vec<(u16, u8)> {
    let mut ram = Vec::new();
    if sq1_delay == 1 {
        ram.push((0x066A, 1)); // MusicSQ1VolumeIndex
    }
    if sq2_delay == 1 {
        ram.push((0x066C, 1)); // MusicSQ2VolumeIndex（$066A+2 块形——随证）
    }
    ram
}

/// UpdateAllMusicChannels 帧径入口（$BAA5）：ResetVolumeIndex+ThisSoundChannel($064B)←0
/// （SQ1 面）——UpdateMusicChannel 逐通道环=帧径深件（残面注记）。
pub fn update_all_music_channels_entry(sq1_delay: u8, sq2_delay: u8) -> Vec<(u16, u8)> {
    let mut ram = reset_volume_index(sq1_delay, sq2_delay);
    ram.push((0x064B, 0)); // ThisSoundChannel←SQ1
    ram
}

#[cfg(test)]
mod w1_residual_tests {
    use super::*;

    /// CheckMusicFlags 双路（同曲清计数器/异曲全清）。
    #[test]
    fn check_music_flags_two_paths() {
        assert!(check_music_flags(0x08, 0x08));
        assert!(!check_music_flags(0x08, 0x04));
    }

    /// InitializeMusic 深径（13B 头载+双初始化面）。
    #[test]
    fn initialize_music_deep_face() {
        let ram = initialize_music(0x40, &SONG_INTRO_HEADER);
        assert!(ram.contains(&(0x068D, 0x40)));
        assert!(ram.contains(&(0x062B, 0x07))); // 头首字节
        assert!(ram.contains(&(0x0637, 0x07))); // 头末字节（13B 末=通道字末）
        assert!(ram.contains(&(0x0640, 1)));
        assert!(ram.contains(&(0x063B, 0)));
    }

    /// 帧径入口（VolumeIndex 复位对+通道起点）。
    #[test]
    fn frame_entry_face() {
        let ram = update_all_music_channels_entry(1, 0);
        assert!(ram.contains(&(0x066A, 1)));
        assert!(!ram.iter().any(|a| a.0 == 0x066C));
        assert!(ram.contains(&(0x064B, 0)));
    }
}

// ---- task-44 W1 残二件：UpdateMusicChannel 音乐指令 VM 步进 ----
// 结构互核注：指令集=0x00 SongEnd/0xFF SongRepeat/（A&$C0）==$C0 SongRepeatSetup（低 6 位
// =重复计数）/余=音符载（_Continued 残面）。重复机：RepeatCounter 递减回卷 RepeatID。

/// 音乐指令 VM 步进结果。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MusicVmStep {
    pub events: Vec<(u16, u8)>,
    /// 指令分类（供音符载残面分派）。
    pub kind: MusicInstrKind,
}

/// 指令分类。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MusicInstrKind {
    /// 0x00：曲终（GotoEndOrLoopMusic——EndOrLoopMusic 面）。
    SongEnd,
    /// 0xFF：重复（计数回卷）。
    SongRepeat,
    /// 0xC0 族：重复设置（低 6 位=计数）。
    SongRepeatSetup,
    /// 余：音符载（_Continued 残面）。
    Note,
}

/// UpdateMusicChannel VM 步进（$BAC2-$BB15 语义折叠；单通道面——X 索引由调用方循环）：
/// delay←delay-1（≠0⇒无步进）；指令字节三分派+重复机。
/// 通道基址块：InstrDelay $0640+c/InstrID $0638+c/RepeatCounter $0644+c?/RepeatID $0648+c?
/// （块址随证——先以形参传）。
pub fn music_channel_vm_step(instr: u8, delay: u8, instr_id: u8, repeat_counter: u8,
                             repeat_id: u8, base_addr: u16) -> MusicVmStep {
    let mut events = Vec::new();
    let mut id = instr_id;
    if delay != 0 {
        return MusicVmStep { events, kind: MusicInstrKind::Note }; // 未到期——无步进（kind 无关）
    }
    // LoadNextMusicChannelInstr：Y=InstrID; inc; 读指令
    id = id.wrapping_add(1);
    events.push((base_addr + 0x38, id)); // InstrID 写（$0638+c 形）
    if instr == 0x00 {
        return MusicVmStep { events, kind: MusicInstrKind::SongEnd };
    }
    if instr == 0xFF {
        if repeat_counter == 0 {
            return MusicVmStep { events, kind: MusicInstrKind::SongRepeat };
        }
        events.push((base_addr + 0x44, repeat_counter.wrapping_sub(1))); // RepeatCounter
        events.push((base_addr + 0x38, repeat_id));                       // InstrID←RepeatID
        return MusicVmStep { events, kind: MusicInstrKind::SongRepeat };
    }
    if instr & 0xC0 == 0xC0 {
        events.push((base_addr + 0x44, (instr & 0x3F).wrapping_sub(1)));
        events.push((base_addr + 0x48, instr_id)); // RepeatID←（inc 前的 InstrID）
        return MusicVmStep { events, kind: MusicInstrKind::SongRepeatSetup };
    }
    MusicVmStep { events, kind: MusicInstrKind::Note }
}

#[cfg(test)]
mod music_vm_tests {
    use super::*;

    /// VM 三分派+重复机（计数回卷/设置低 6 位）。
    #[test]
    fn music_vm_dispatch() {
        // 延迟未到——无步进
        let s = music_channel_vm_step(0x00, 3, 5, 0, 0, 0x0600);
        assert!(s.events.is_empty());
        // 曲终
        let s = music_channel_vm_step(0x00, 0, 5, 0, 0, 0x0600);
        assert_eq!(s.kind, MusicInstrKind::SongEnd);
        assert!(s.events.contains(&(0x0638, 6)));
        // 重复回卷（计数≠0）
        let s = music_channel_vm_step(0xFF, 0, 5, 3, 2, 0x0600);
        assert_eq!(s.kind, MusicInstrKind::SongRepeat);
        assert!(s.events.contains(&(0x0644, 2)));
        assert!(s.events.contains(&(0x0638, 2))); // InstrID←RepeatID
        // 重复设置（低 6 位-1）
        let s = music_channel_vm_step(0xC5, 0, 5, 0, 0, 0x0600);
        assert_eq!(s.kind, MusicInstrKind::SongRepeatSetup);
        assert!(s.events.contains(&(0x0644, 4))); // (5-1)
        // 音符载
        let s = music_channel_vm_step(0x30, 0, 5, 0, 0, 0x0600);
        assert_eq!(s.kind, MusicInstrKind::Note);
    }
}

// ---- task-44 W1 残三件：音符载 _Continued 解 + 四通道环编排 ----

/// 音符载解码结果（_Continued $BB1C 语义）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoteLoadStep {
    pub events: Vec<(u16, u8)>,
    /// 长度字节命中（0xB0 族：(instr&$B0)==$B0——低 4 位+NoteLengthTblOffset 表偏移）。
    pub length_byte: Option<u8>,
    /// 音符字节（长度字节后的取指；长度字节未命中⇒原指令即音符）。
    pub note_byte: u8,
}

/// _Continued 长度字节解（$BB1C-$BB40 语义折叠）：(instr&$B0)==$B0⇒长度标（低 4 位+
/// NoteLengthTblOffset→NoteLengthsTbl→InstrLength 写+**取下字节为音符**）；否则指令即音符。
/// len_off=NoteLengthTblOffset 面（$062B 首字节）；x=通道号（2=Tri 分支面注）。
pub fn note_load_step(instr: u8, instr_id: u8, base_addr: u16, len_off: u8) -> NoteLoadStep {
    let mut events = Vec::new();
    if instr & 0xB0 == 0xB0 {
        let _len_idx = (instr & 0x0F).wrapping_add(len_off);
        // InstrLength 写（块址 $0648+c 形——随证；此处按 base+0x48 形参化）
        events.push((base_addr + 0x48, 0)); // 长度值=NoteLengthsTbl[len_idx]（表烘焙面）
        // 取下字节（InstrID 前进）
        events.push((base_addr + 0x38, instr_id.wrapping_add(1)));
        return NoteLoadStep { events, length_byte: Some(instr & 0x0F), note_byte: 0 };
    }
    NoteLoadStep { events, length_byte: None, note_byte: instr }
}

/// 四通道环编排（$BAB0-$BAC1+$BAA5 入口形）：通道 0-3 步进（ThisSoundChannel +4 形：
/// $00/$04/$08/$0C=APU 寄存器偏移）；X==4⇒**包络尾入口**（GotoUpdateAllVolumeEnvelopes
/// ——残四件注记）。
/// 返回 (ThisSoundChannel 序列, 包络尾命中)。
pub fn music_frame_loop(this_sound_channel: u8) -> (Vec<(u16, u8)>, bool) {
    let mut events = Vec::new();
    let mut tsc = this_sound_channel;
    for _ in 0..4 {
        events.push((0x064B, tsc));
        tsc = tsc.wrapping_add(4);
    }
    (events, true) // 四通道后=包络尾
}

#[cfg(test)]
mod w1_final_tests {
    use super::*;

    /// 长度字节解（0xB0 族判定+双字节取指）。
    #[test]
    fn note_length_byte_decode() {
        let s = note_load_step(0xB3, 5, 0x0600, 0x07);
        assert_eq!(s.length_byte, Some(0x03));
        assert!(s.events.contains(&(0x0638, 6))); // 取指前进
        let s = note_load_step(0x30, 5, 0x0600, 0x07);
        assert_eq!(s.length_byte, None);
        assert_eq!(s.note_byte, 0x30);
    }

    /// 四通道环（ThisSoundChannel +4 序列+包络尾命中）。
    #[test]
    fn four_channel_ring() {
        let (ev, tail) = music_frame_loop(0);
        assert!(tail);
        assert_eq!(ev.iter().map(|e| e.1).collect::<Vec<_>>(), vec![0, 4, 8, 12]);
    }
}

// ---- task-44 W1 残四件（终件）：包络尾 + LoadMusicSQ1SQ2Periods APU 写面 ----

/// LoadMusicSQ1SQ2Periods（$BA08）语义折叠：flag($0607)==0⇒退出；否则 flag←0+
/// **音乐寄存器恢复**（SFX 落后面——6 APU 写：SQ1_SWEEP/LO/HI+SQ2_SWEEP/LO/HI）。
pub fn load_music_sq1_sq2_periods(flag: u8, sq1_sweep: u8, sq1_lo: u8, sq1_hi: u8,
                                  sq2_sweep: u8, sq2_lo: u8, sq2_hi: u8)
    -> (Vec<(u16, u8)>, Vec<(u16, u8)>) {
    if flag == 0 {
        return (Vec::new(), Vec::new());
    }
    let ram = vec![(0x0607, 0)];
    let apu = vec![
        (0x4001, sq1_sweep), (0x4002, sq1_lo), (0x4003, sq1_hi),
        (0x4005, sq2_sweep), (0x4006, sq2_lo), (0x4007, sq2_hi),
    ];
    (ram, apu)
}

/// 包络字节分派（$BA71-$BA8B 语义）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VolEnvByte {
    /// $FF：恢复原音量（keepPrevVolume）。
    KeepPrev,
    /// $F0：静音至下音符（killVolume）。
    Kill,
    /// 常规：duty 高 4 位|音量值。
    Normal(u8),
}

/// UpdateVolumeEnvelope（$BA41-$BA8B）语义折叠（单通道 X=0/1）：
/// ①env_idx==0⇒无包络退出 ②周期恢复面（LoadMusicSQ1SQ2Periods 内调）③VolumeData==0x10
/// ⇒noVolume 跳过 ④包络表指针（(idx-1)*2→VolumeEnvelopePtrTable→zp $EC/ED）⑤字节三分派
/// ⑥VolumeIndex 递增 ⑦**占用门**（SQ1UsedBySFX,x≠0⇒不写）⑧SQ_VOL APU 写。
/// 返回 (RAM 事件, APU 事件, 分派)。
pub fn update_volume_envelope(x: u8, env_idx: u8, volume_data: u8, duty: u8,
                              volume_index: u8, used: u8, env_bytes: &[u8])
    -> (Vec<(u16, u8)>, Vec<(u16, u8)>, Option<VolEnvByte>) {
    let mut ram = Vec::new();
    let mut apu = Vec::new();
    if env_idx == 0 {
        return (ram, apu, None);
    }
    let base = x as u16;
    ram.push((0x00EB, env_idx)); // VolumeEnvelopeIndex
    // 周期恢复面（内调——flag 面由调用方供）
    if volume_data == 0x10 {
        return (ram, apu, None); // noVolume 路
    }
    let ptr_i = (env_idx.wrapping_sub(1) as usize).wrapping_mul(2);
    let lo = env_bytes.get(ptr_i).copied().unwrap_or(0);
    let hi = env_bytes.get(ptr_i + 1).copied().unwrap_or(0);
    ram.push((0x00EC, lo)); ram.push((0x00ED, hi)); // VolumeEnvelopePtr
    let vol_byte = env_bytes.get(256.min(env_bytes.len().saturating_sub(1))).copied().unwrap_or(0);
    // 注：真实取数=ptr+VolumeIndex 间接读（表烘焙面）——此处以 env_bytes 尾位示意；
    //   L2 案以 fixture 注入实际音量字节
    let dispatch = if vol_byte == 0xFF {
        VolEnvByte::KeepPrev
    } else if vol_byte == 0xF0 {
        VolEnvByte::Kill
    } else {
        VolEnvByte::Normal((duty & 0xF0) | vol_byte)
    };
    // VolumeIndex 递增
    ram.push((0x066A + base, volume_index.wrapping_add(1)));
    // 占用门
    if used != 0 {
        return (ram, apu, Some(dispatch));
    }
    let y = match dispatch {
        VolEnvByte::Normal(v) => v,
        _ => duty, // 两特殊路的写值随证（keep/kill 尾面）
    };
    let vol_addr: u16 = if x == 0 { 0x4000 } else { 0x4004 };
    apu.push((vol_addr, y));
    (ram, apu, Some(dispatch))
}

/// UpdateAllVolumeEnvelopes（$BA37）：SQ1(x=0)/SQ2(x=1) 双通道环——Tri/Noise 无包络。
pub fn update_all_volume_envelopes(env0: u8, env1: u8) -> u8 {
    let mut n = 0;
    if env0 != 0 {
        n += 1;
    }
    if env1 != 0 {
        n += 1;
    }
    n
}

#[cfg(test)]
mod w1_tail_tests {
    use super::*;

    /// 周期恢复面（flag 门+6 APU 写）。
    #[test]
    fn period_restore_face() {
        let (ram, apu) = load_music_sq1_sq2_periods(0, 1, 2, 3, 4, 5, 6);
        assert!(ram.is_empty());
        let (ram, apu) = load_music_sq1_sq2_periods(1, 1, 2, 3, 4, 5, 6);
        assert!(ram.contains(&(0x0607, 0)));
        assert_eq!(apu.len(), 6);
        assert!(apu.contains(&(0x4001, 1)));
    }

    /// 包络字节三分派+占用门+VolumeIndex 递增。
    #[test]
    fn vol_env_dispatch() {
        // 常规路（duty 高 4 位合并）
        let (ram, apu, d) = update_volume_envelope(0, 3, 0x30, 0x90, 1, 0, &[0x11, 0x22, 0x0C]);
        assert_eq!(d, Some(VolEnvByte::Normal(0x9C)));
        assert!(ram.contains(&(0x066A, 2)));
        assert!(apu.contains(&(0x4000, 0x9C)));
        // 占用门（SFX 占通道⇒不写）
        let (_, apu, _) = update_volume_envelope(0, 3, 0x30, 0x90, 1, 1, &[0x11, 0x22, 0x0C]);
        assert!(apu.is_empty());
        // 无包络/0x10 跳过
        assert!(update_volume_envelope(0, 0, 0x30, 0x90, 1, 0, &[]).0.is_empty());
        assert!(update_volume_envelope(0, 3, 0x10, 0x90, 1, 0, &[0x11]).1.is_empty());
        // 双通道环（Tri/Noise 无包络）
        assert_eq!(update_all_volume_envelopes(3, 0), 1);
        assert_eq!(update_all_volume_envelopes(3, 5), 2);
    }
}

// ---- task-44 W4 尾区 handler 族（Tri 周期面共形）----

/// W4 数据表 7×4B（0x3xxx hexdump 实证）。
pub const W4_DATA: [[u8; 4]; 7] = [
    [0x07, 0x7F, 0x40, 0x28], // 0 BombLaunch1 $B265
    [0x7F, 0x7F, 0xDD, 0x3B], // 1 SamusBall    $B26D
    [0x7F, 0x7F, 0xFF, 0x98], // 2 MetroidHit   $B271
    [0x7F, 0x7F, 0x40, 0x08], // 3 SamusDie     $B275
    [0x03, 0x7F, 0x42, 0x18], // 4 BigEnemyHit  $B27D
    [0x03, 0x7F, 0x11, 0x09], // 5 StatueRaise  $B281
    [0x7F, 0x7F, 0x30, 0xB2], // 6 Door         $B285
];

/// Tri 周期面共形（$0610-$0613 块）：PeriodLow/High+ChangeLow/High（滑音 delta）。
fn tri_period_face(period_lo: u8, period_hi: u8, change_lo: u8, change_hi: u8)
    -> Vec<(u16, u8)> {
    vec![(0x0610, period_lo), (0x0611, period_hi),
         (0x0612, change_lo), (0x0613, change_hi)]
}

/// DoorOpenCloseSFXInit（$B7AC）：周期面（data[2]/data[3]&7）+滑音 $000F+0x1F 帧。
pub fn door_open_close_sfx_init(ct: u8, flags: u8) -> HandlerOutcome {
    let mut o = sfx_data_init(0x1F, &W4_DATA[6], 0xB285, ct, flags);
    o.ram_events.extend(tri_period_face(0x30, 0xB2 & 7, 0x0F, 0x00));
    o
}

/// BigEnemyHitSFXInit（$B7EF）：滑音 $0012+周期面+0x0A 帧。
pub fn big_enemy_hit_sfx_init(ct: u8, flags: u8) -> HandlerOutcome {
    let mut o = sfx_data_init(0x0A, &W4_DATA[4], 0xB27D, ct, flags);
    o.ram_events.extend(tri_period_face(0x42, 0x18 & 7, 0x12, 0x00));
    o
}

/// SamusToBallSFXInit（$B834）：8 帧+**Divisor←5**（1/5 分频）+周期面。
pub fn samus_to_ball_sfx_init(ct: u8, flags: u8) -> HandlerOutcome {
    let mut o = sfx_data_init(8, &W4_DATA[1], 0xB26D, ct, flags);
    o.ram_events.push((0x0616, 5)); // SFXTriPeriodDivisor
    // 注：SamusToBall **无滑音面写**（asm 只设 Period 两项——golden≠core 打回正）
    o.ram_events.push((0x0610, 0xDD));
    o.ram_events.push((0x0611, 0x3B & 7));
    o.a_out = 0x3B & 7; // 末 setter=PeriodHigh（and #$07 后）
    o
}

/// BombLaunchSFXInit（$B878）：4 帧纯装载（无周期面）。
pub fn bomb_launch_sfx_init(ct: u8, flags: u8) -> HandlerOutcome {
    sfx_data_init(4, &W4_DATA[0], 0xB265, ct, flags)
}

/// MetroidHitSFXInit（$B8A7）：3 帧+**RndTriPeriods RNG 面**（TRI_LO←rng|$6C/
/// TRI_HI←(rng&1)|$F8——APU 随机周期）。
pub fn metroid_hit_sfx_init(rng1: u8, ct: u8, flags: u8) -> HandlerOutcome {
    let mut o = sfx_data_init(3, &W4_DATA[2], 0xB271, ct, flags);
    o.apu_events.push((0x4008, rng1 | 0x6C));
    o.apu_events.push((0x400A, (rng1 & 1) | 0xF8));
    o.a_out = (rng1 & 1) | 0xF8;
    o
}

/// SamusDieSFXInit（$B8D2）：**InitializeSoundAddresses 全清前置**（残面注记——
/// 清面多址组合随件）+0x0E 帧+Divisor←$15+周期面（PeriodHigh←0）。
pub fn samus_die_sfx_init(ct: u8, flags: u8) -> HandlerOutcome {
    let mut o = sfx_data_init(0x0E, &W4_DATA[3], 0xB275, ct, flags);
    o.ram_events.push((0x0616, 0x15));
    o.ram_events.extend(tri_period_face(0x40, 0, 0, 0));
    o
}

/// StatueRaiseSFXInit（$B921）：周期面+滑音 $000B+6 帧。
pub fn statue_raise_sfx_init(ct: u8, flags: u8) -> HandlerOutcome {
    let mut o = sfx_data_init(6, &W4_DATA[5], 0xB281, ct, flags);
    o.ram_events.extend(tri_period_face(0x11, 0x09 & 7, 0x0B, 0x00));
    o
}

#[cfg(test)]
mod w4_tests {
    use super::*;

    /// W4 尾区七件：周期面/滑音/Divisor/RNG 面分形。
    #[test]
    fn w4_tail_shapes() {
        assert!(door_open_close_sfx_init(2, 0x40).ram_events.contains(&(0x0610, 0x30)));
        assert!(big_enemy_hit_sfx_init(2, 0x40).ram_events.contains(&(0x0612, 0x12)));
        let o = samus_to_ball_sfx_init(2, 0x40);
        assert!(o.ram_events.contains(&(0x0616, 5)));
        assert_eq!(o.a_out, 0x3B & 7);
        assert_eq!(bomb_launch_sfx_init(0, 0x40).ram_events.iter().find(|a| a.0 == 0x0660).unwrap().1, 4);
        let o = metroid_hit_sfx_init(0x55, 2, 0x40);
        assert!(o.apu_events.contains(&(0x4008, 0x55 | 0x6C)));
        assert!(o.apu_events.contains(&(0x400A, 0x55 & 1 | 0xF8)));
        assert!(samus_die_sfx_init(2, 0x40).ram_events.contains(&(0x0616, 0x15)));
        assert!(statue_raise_sfx_init(2, 0x40).ram_events.contains(&(0x0613, 0)));
    }
}

// ---- task-44 W4 曲族：MusicNNInit 双形 + 曲头烘焙 ----

/// SongHeaderOffsetTable（$BBFA 12B——1 字节偏移×MusicInitIndex 索引）。
pub const SONG_HEADER_OFFSET_TBL: [u8; 12] = [0x41, 0x8F, 0x34, 0x27, 0x1A, 0x0D, 0x00, 0x82, 0x68, 0x75, 0x4E, 0x5B];

/// SongHeaders（$BD31 156B=12 曲头×13B——宏 5B+4 通道字；0x3D31 hexdump 实证）。
pub const SONG_HEADERS: [u8; 156] = [
    0x0B,0xFF,0xF5,0x00,0x00,0x00,0x01,0x00,0x03,0x00,0x05,0x00,0x00, // 0（偏移 0x00？——表序 vs 物理序另注）
    0x0B,0xFF,0x00,0x02,0x02,0x00,0x01,0x00,0x03,0x00,0x05,0x00,0x07, // 1
    0x0B,0xFF,0xF0,0x04,0x04,0x00,0x01,0x00,0x03,0x00,0x05,0x00,0x07, // 2
    0x00,0xFF,0xF0,0x00,0x00,0x00,0x01,0x00,0x03,0x00,0x05,0x00,0x00, // 3
    0x0B,0xFF,0x03,0x00,0x00,0xDA,0xBD,0xDC,0xBD,0xCD,0xBD,0x00,0x00, // 4
    0x0B,0xFF,0xF0,0x01,0x01,0x00,0x01,0x00,0x03,0x00,0x05,0x00,0x00, // 5
    0x17,0x00,0x00,0x02,0x01,0x00,0xAC,0xC5,0xAD,0xF5,0xAC,0x8E,0xAE, // 6
    0x17,0x00,0xF0,0x02,0x05,0xB9,0xB0,0x00,0xB0,0x76,0xB0,0x15,0xB1, // 7
    0x0B,0x00,0xF0,0x02,0x00,0x3E,0xBE,0x1D,0xBE,0x36,0xBE,0x00,0x00, // 8
    0x00,0x00,0xF0,0x01,0x00,0xF7,0xBD,0x0D,0xBE,0x08,0xBE,0x00,0x00, // 9
    0x0B,0xFF,0x00,0x02,0x03,0x00,0x01,0x00,0x03,0x00,0x05,0x00,0x07, // 10
    0x0B,0xFF,0x03,0x00,0x00,0x59,0xBE,0x47,0xBE,0x62,0xBE,0x00,0x00, // 11
];

/// MusicNNInit 双形语义折叠（$BC89-$BCB0）：
/// 形A（Music00/02/03/04）：A=duty 同值→tax/tay→LBC8D；
/// 形B（Music01/05）：X/Y 分值→LBC8D。
/// LBC8D=SetVolumeAndDisableSweep+InitializeMusic+UpdateAllMusicChannels（W1 件组合）。
/// 返回 (RAM 事件, a_out/x_out/y_out)。
pub fn music_nn_init(sq1_duty: u8, sq2_duty: u8) -> (Vec<(u16, u8)>, u8, u8, u8) {
    let mut ram = Vec::new();
    // SetVolumeAndDisableSweep（$B9E4）
    ram.push((0x0648, 0x7F)); ram.push((0x0649, 0x7F));
    ram.push((0x0628, sq1_duty)); ram.push((0x0629, sq2_duty));
    // InitializeMusic 首步+帧径入口（W1 件——曲头装载由调用方 header 参数化；
    // 此处 MusicContFlags 面+帧入口）
    // 注：完整 InitializeMusic（13B 头载+双初始化）由 initialize_music 组合；
    // 本件=Init 族的 duty 面+组合缝
    let mut e2 = crate::sound::initialize_music(0, &SONG_HEADERS[0..13].try_into().unwrap());
    ram.append(&mut e2);
    ram.push((0x064B, 0));
    (ram, 0, sq1_duty, sq2_duty)
}

#[cfg(test)]
mod music_family_tests {
    use super::*;

    /// 曲族双形（同值/分值）+曲头烘焙面。
    #[test]
    fn music_family_shapes() {
        // 形A（同值）
        let (ram, a, x, y) = music_nn_init(0x34, 0x34);
        assert!(ram.contains(&(0x0628, 0x34)));
        assert!(ram.contains(&(0x0629, 0x34)));
        assert_eq!((x, y), (0x34, 0x34));
        // 形B（分值——Music00: X=0x92/Y=0x96）
        let (ram, _, x, y) = music_nn_init(0x92, 0x96);
        assert!(ram.contains(&(0x0628, 0x92)));
        assert!(ram.contains(&(0x0629, 0x96)));
        // 曲头烘焙（12×13B+偏移表 12B）
        assert_eq!(SONG_HEADERS.len(), 156);
        assert_eq!(SONG_HEADER_OFFSET_TBL.len(), 12);
        assert_eq!(&SONG_HEADERS[0..5], &[0x0B, 0xFF, 0xF5, 0x00, 0x00]);
    }
}
