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

/// 位扫描结果（GetSoundRoutine @loop_B 语义：MSB 优先+表目 2B）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlagScan {
    /// 命中位序（0-7；位7→0 最先）。
    pub bit_index: Option<u8>,
    /// 表目 Y 偏移（bit 序×2——2B handler 指针步进）。
    pub table_offset: u8,
}

/// 位扫描（`asl CurrentSoundFlags / bcs` MSB 优先 8 轮；无旗⇒None）。
pub fn scan_sound_flags(flags: u8) -> FlagScan {
    let mut v = flags;
    for i in 0..8u8 {
        let carry = v & 0x80 != 0;
        v = v.wrapping_shl(1);
        if carry {
            return FlagScan { bit_index: Some(i), table_offset: i.wrapping_mul(2) };
        }
    }
    FlagScan { bit_index: None, table_offset: 0 }
}

/// 6 init 旗清（ClearInitFlags $B3D6 语义面——Noise/SQ1/SQ2/Tri/Multi/Music）。
pub const INIT_FLAG_COUNT: usize = 6;

#[cfg(test)]
mod tests {
    use super::*;

    /// 位扫描 MSB 优先（位7→序0）。
    #[test]
    fn scan_is_msb_first() {
        assert_eq!(scan_sound_flags(0x80).bit_index, Some(0));
        assert_eq!(scan_sound_flags(0x08).bit_index, Some(4));
        assert_eq!(scan_sound_flags(0x01).bit_index, Some(7));
        assert_eq!(scan_sound_flags(0x00).bit_index, None);
        assert_eq!(scan_sound_flags(0x41).bit_index, Some(1)); // 双旗取高
    }

    /// 表目偏移=位序×2（2B 指针步进）。
    #[test]
    fn table_offset_is_bit_times_two() {
        assert_eq!(scan_sound_flags(0x02).table_offset, 12);
    }
}
