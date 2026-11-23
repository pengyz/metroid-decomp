//! game-core::memmap::sound_ram_face —— **声引擎 RAM 面（`$0600-$068D`）的 typed 声明层**。
//!
//! 【这一面是什么】`refs/m1disasm/SRC/constants_ram.asm:999-1165` 的 `.enum $0600 export`：
//! 音乐 4 通道 + SFX 5 路 + 暂停/任务标志的常驻状态。逐字段地址由 asm 顺序累加推算
//! （`db`=1、`dw`=2、`ds N`=N），单测 `offsets_match_asm_enum` 用游标复核到 `$068E`。
//!
//! 【为什么分成两段 span】`$0683/$0684`（`SFXTriInitFlags`/`SFXMultiInitFlags`）**已被**
//! [`crate::sound::state::SoundInitFlags`] 声明为 `Owner`（`crates/game-core/src/sound/state.rs:229`，
//! 2 B）。同一字节约定的 owner 唯一，故本面把它们**排除**：声明两段
//! `$0600-$0682`（131 B）+ `$0685-$068D`（9 B），共 **140 B**；解码/编码也不触碰这两字节
//! （`decode` 不读、`encode` 不写，读写仍归 `SoundInitFlags` 的 typed 访问器）。
//!
//! 【为什么此前不是 `Owner`】行为已移植（`crates/game-core/src/sound/mod.rs` 按裸地址字节读写，
//! `sound/hook.rs:171/200` 分派 `0xB77A/0xB781`），
//! 但 $0600-$0684 在审计里是**未登记洞**、$0685-$068D 落在账本洞 `$0685-$06FF` 内 ⇒ 只算 `Undeclared`。
//!
//! 【`SpareMem*` 的处理】asm 的 6 段 `ds` + 4 个单字节 `SpareMem`（`$0606/$064C/$065F/$066E/$0687`）
//! 以 `[u8; N]`/`u8` 字段留在面内、原字节 round-trip、不赋语义（不猜用途）。它们是同一结构体的
//! 保留位（asm 注释写了对齐理由），拆成独立洞会把这一面切成十几段、失去「一个结构体一块面」的可审计性。
//!
//! 【行为零变化】业务层仍按字节读写；本结构是按需解码/编码视图（同 B1-a/B1-b/B1-c 第一面）。

use crate::memmap::nesmap::{NesMapped, NesSpan, SpanKey};
use crate::memmap::storage::NesImage;

/// 面起点。
pub const BASE: u16 = 0x0600;
/// 面尾（`$068D` 最后一个字节之后）。
pub const FACE_END: u16 = 0x068E;
/// 第一段：`$0600-$0682`（`SoundInitFlags` 之前的全部字节）。
pub const SEG1_START: u16 = 0x0600;
pub const SEG1_LEN: u16 = 0x83;
/// 第二段：`$0685-$068D`（`SoundInitFlags` 之后、`MusicInitFlags` 起到 `MusicContFlags`）。
pub const SEG2_START: u16 = 0x0685;
pub const SEG2_LEN: u16 = 0x09;
/// 本面声明的字节数（两段合计）。
pub const FACE_LEN: u16 = SEG1_LEN + SEG2_LEN;

/// `$0683/$0684` 归 [`crate::sound::state::SoundInitFlags`]，本面不认领、不改写。
pub const SOUND_INIT_FLAGS_START: u16 = 0x0683;
pub const SOUND_INIT_FLAGS_LEN: u16 = 0x02;

/// 逐字段地址（由 asm 顺序累加；`$0600-$068D`，跳过 `$0683/$0684`）。
pub const OFF_MUSICSQ1PERIODLOW: u16 = 0x0600;
pub const OFF_MUSICSQ1PERIODHIGH: u16 = 0x0601;
pub const OFF_SFXPAUSED: u16 = 0x0602;
pub const OFF_PAUSESFXSTATUS: u16 = 0x0603;
pub const OFF_MUSICSQ2PERIODLOW: u16 = 0x0604;
pub const OFF_MUSICSQ2PERIODHIGH: u16 = 0x0605;
pub const OFF_SPAREMEM0606: u16 = 0x0606;
pub const OFF_LOADMUSICSQ1SQ2PERIODSFLAG: u16 = 0x0607;
pub const OFF_MUSICTRIPERIODLOW: u16 = 0x0608;
pub const OFF_MUSICTRIPERIODHIGH: u16 = 0x0609;
pub const OFF_SPAREMEM060A: u16 = 0x060A;
pub const OFF_SFXTRIPERIODLOW: u16 = 0x0610;
pub const OFF_SFXTRIPERIODHIGH: u16 = 0x0611;
pub const OFF_SFXTRICHANGELOW: u16 = 0x0612;
pub const OFF_SFXTRICHANGEHIGH: u16 = 0x0613;
pub const OFF_SFXTRIPERIODDIVIDEDLOW: u16 = 0x0614;
pub const OFF_SFXTRIPERIODDIVIDEDHIGH: u16 = 0x0615;
pub const OFF_SFXTRIPERIODDIVISOR: u16 = 0x0616;
pub const OFF_SFXTRIPERIODREMAINDER: u16 = 0x0617;
pub const OFF_SPAREMEM0618: u16 = 0x0618;
pub const OFF_HASBEAMSFX: u16 = 0x061F;
pub const OFF_MUSICSQ1INSTRLENGTH: u16 = 0x0620;
pub const OFF_MUSICSQ2INSTRLENGTH: u16 = 0x0621;
pub const OFF_MUSICTRIINSTRLENGTH: u16 = 0x0622;
pub const OFF_MUSICNOISEINSTRLENGTH: u16 = 0x0623;
pub const OFF_MUSICSQ1REPEATCOUNTER: u16 = 0x0624;
pub const OFF_MUSICSQ2REPEATCOUNTER: u16 = 0x0625;
pub const OFF_MUSICTRIREPEATCOUNTER: u16 = 0x0626;
pub const OFF_MUSICNOISEREPEATCOUNTER: u16 = 0x0627;
pub const OFF_MUSICSQ1DUTYENVELOPE: u16 = 0x0628;
pub const OFF_MUSICSQ2DUTYENVELOPE: u16 = 0x0629;
pub const OFF_MUSICTRILINEARCOUNT: u16 = 0x062A;
pub const OFF_NOTELENGTHTBLOFFSET: u16 = 0x062B;
pub const OFF_MUSICLOOPSONEND: u16 = 0x062C;
pub const OFF_MUSICTRICOUNTERCNTRL: u16 = 0x062D;
pub const OFF_MUSICSQ1VOLUMEENVELOPEINDEX: u16 = 0x062E;
pub const OFF_MUSICSQ2VOLUMEENVELOPEINDEX: u16 = 0x062F;
pub const OFF_MUSICSQ1BASE: u16 = 0x0630;
pub const OFF_MUSICSQ2BASE: u16 = 0x0632;
pub const OFF_MUSICTRIBASE: u16 = 0x0634;
pub const OFF_MUSICNOISEBASE: u16 = 0x0636;
pub const OFF_MUSICSQ1INSTRID: u16 = 0x0638;
pub const OFF_MUSICSQ2INSTRID: u16 = 0x0639;
pub const OFF_MUSICTRIINSTRID: u16 = 0x063A;
pub const OFF_MUSICNOISEINSTRID: u16 = 0x063B;
pub const OFF_MUSICSQ1REPEATID: u16 = 0x063C;
pub const OFF_MUSICSQ2REPEATID: u16 = 0x063D;
pub const OFF_MUSICTRIREPEATID: u16 = 0x063E;
pub const OFF_MUSICNOISEREPEATID: u16 = 0x063F;
pub const OFF_MUSICSQ1INSTRDELAY: u16 = 0x0640;
pub const OFF_MUSICSQ2INSTRDELAY: u16 = 0x0641;
pub const OFF_MUSICTRIINSTRDELAY: u16 = 0x0642;
pub const OFF_MUSICNOISEINSTRDELAY: u16 = 0x0643;
pub const OFF_SPAREMEM0644: u16 = 0x0644;
pub const OFF_MUSICSQ1SWEEP: u16 = 0x0648;
pub const OFF_MUSICSQ2SWEEP: u16 = 0x0649;
pub const OFF_MUSICTRISWEEP: u16 = 0x064A;
pub const OFF_THISSOUNDCHANNEL: u16 = 0x064B;
pub const OFF_SPAREMEM064C: u16 = 0x064C;
pub const OFF_CURRENTSOUNDFLAGS: u16 = 0x064D;
pub const OFF_SPAREMEM064E: u16 = 0x064E;
pub const OFF_NOISEUSEDBYSFX: u16 = 0x0652;
pub const OFF_SQ1USEDBYSFX: u16 = 0x0653;
pub const OFF_SQ2USEDBYSFX: u16 = 0x0654;
pub const OFF_TRIUSEDBYSFX: u16 = 0x0655;
pub const OFF_SPAREMEM0656: u16 = 0x0656;
pub const OFF_CHANNELTYPE: u16 = 0x065C;
pub const OFF_MUSICLOOPFLAGS: u16 = 0x065D;
pub const OFF_MUSICINITINDEX: u16 = 0x065E;
pub const OFF_SPAREMEM065F: u16 = 0x065F;
pub const OFF_SFXNOISELENGTH: u16 = 0x0660;
pub const OFF_SFXSQ1LENGTH: u16 = 0x0661;
pub const OFF_SFXSQ2LENGTH: u16 = 0x0662;
pub const OFF_SFXTRILENGTH: u16 = 0x0663;
pub const OFF_SFXMULTILENGTH: u16 = 0x0664;
pub const OFF_SFXNOISEFRAME: u16 = 0x0665;
pub const OFF_SFXSQ1FRAME: u16 = 0x0666;
pub const OFF_SFXSQ2FRAME: u16 = 0x0667;
pub const OFF_SFXTRIFRAME: u16 = 0x0668;
pub const OFF_SFXMULTIFRAME: u16 = 0x0669;
pub const OFF_MUSICSQ1VOLUMEINDEX: u16 = 0x066A;
pub const OFF_MUSICSQ2VOLUMEINDEX: u16 = 0x066B;
pub const OFF_MUSICSQ1VOLUMEDATA: u16 = 0x066C;
pub const OFF_MUSICSQ2VOLUMEDATA: u16 = 0x066D;
pub const OFF_SPAREMEM066E: u16 = 0x066E;
pub const OFF_SFXNOISEVAR0: u16 = 0x0670;
pub const OFF_SFXSQ1VAR0: u16 = 0x0671;
pub const OFF_SFXSQ2VAR0: u16 = 0x0672;
pub const OFF_SFXTRIVAR0: u16 = 0x0673;
pub const OFF_SFXNOISEVAR1: u16 = 0x0674;
pub const OFF_SFXSQ1VAR1: u16 = 0x0675;
pub const OFF_SFXSQ2VAR1: u16 = 0x0676;
pub const OFF_SFXTRIVAR1: u16 = 0x0677;
pub const OFF_SFXNOISEVAR2: u16 = 0x0678;
pub const OFF_SFXSQ1VAR2: u16 = 0x0679;
pub const OFF_SFXSQ2VAR2: u16 = 0x067A;
pub const OFF_SFXTRIVAR2: u16 = 0x067B;
pub const OFF_SPAREMEM067C: u16 = 0x067C;
pub const OFF_SFXNOISEINITFLAGS: u16 = 0x0680;
pub const OFF_SFXSQ1INITFLAGS: u16 = 0x0681;
pub const OFF_SFXSQ2INITFLAGS: u16 = 0x0682;
pub const OFF_MUSICINITFLAGS: u16 = 0x0685;
pub const OFF_SCREWATTACK0686: u16 = 0x0686;
pub const OFF_SPAREMEM0687: u16 = 0x0687;
pub const OFF_SFXNOISECONTFLAGS: u16 = 0x0688;
pub const OFF_SFXSQ1CONTFLAGS: u16 = 0x0689;
pub const OFF_SFXSQ2CONTFLAGS: u16 = 0x068A;
pub const OFF_SFXTRICONTFLAGS: u16 = 0x068B;
pub const OFF_SFXMULTICONTFLAGS: u16 = 0x068C;
pub const OFF_MUSICCONTFLAGS: u16 = 0x068D;

/// 声引擎 RAM 面的 typed 表示（字段顺序 = asm 顺序；`db`=u8 / `dw`=u16(LE) / `ds N`=[u8;N]）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SoundRamFace {
    /// `MusicSQ1PeriodLow`（`$0600`，1 B）——Loaded into SQ1_LO when playing music
    pub music_sq1_period_low: u8,
    /// `MusicSQ1PeriodHigh`（`$0601`，1 B）——Loaded into SQ1_HI when playing music
    pub music_sq1_period_high: u8,
    /// `SFXPaused`（`$0602`，1 B）——0=Game not paused, 1=Game paused
    pub s_f_x_paused: u8,
    /// `PauseSFXStatus`（`$0603`，1 B）——Plays PauseMusic SFX if less than #$12
    pub pause_sfx_status: u8,
    /// `MusicSQ2PeriodLow`（`$0604`，1 B）——Loaded into SQ2_LO when playing music
    pub music_sq2_period_low: u8,
    /// `MusicSQ2PeriodHigh`（`$0605`，1 B）——Loaded into SQ2_HI when playing music
    pub music_sq2_period_high: u8,
    /// `SpareMem0606`（`$0606`，1 B）——Padding so that MusicTriPeriodLow is 4-byte aligned for ThisSoundChannel
    pub spare_mem0606: u8,
    /// `LoadMusicSQ1SQ2PeriodsFlag`（`$0607`，1 B）——1=data needs to be written, 0=no data to write
    pub load_music_sq1_sq2_periods_flag: u8,
    /// `MusicTriPeriodLow`（`$0608`，1 B）——Loaded into TRI_LO when playing music
    pub music_tri_period_low: u8,
    /// `MusicTriPeriodHigh`（`$0609`，1 B）——Loaded into TRI_HI when playing music
    pub music_tri_period_high: u8,
    /// `SpareMem060A`（`$060A`，6 B）——$060A
    pub spare_mem060_a: [u8; 6],
    /// `SFXTriPeriodLow`（`$0610`，1 B）——Stores triangle SFX period low for processing
    pub s_f_x_tri_period_low: u8,
    /// `SFXTriPeriodHigh`（`$0611`，1 B）——Stroes triangle SFX period high for processing
    pub s_f_x_tri_period_high: u8,
    /// `SFXTriChangeLow`（`$0612`，1 B）——Stores triangle SFX change in period low
    pub s_f_x_tri_change_low: u8,
    /// `SFXTriChangeHigh`（`$0613`，1 B）——Stores triangle SFX change in period high
    pub s_f_x_tri_change_high: u8,
    /// `SFXTriPeriodDividedLow`（`$0614`，1 B）——Low result of DivideSFXTriPeriod division. Used as SFXTriChangeLow.
    pub s_f_x_tri_period_divided_low: u8,
    /// `SFXTriPeriodDividedHigh`（`$0615`，1 B）——High result of DivideSFXTriPeriod division. Used as SFXTriChangeHigh.
    pub s_f_x_tri_period_divided_high: u8,
    /// `SFXTriPeriodDivisor`（`$0616`，1 B）——Used in DivideSFXTriPeriod as divisor for TriPeriod values.
    pub s_f_x_tri_period_divisor: u8,
    /// `SFXTriPeriodRemainder`（`$0617`，1 B）——Used in DivideSFXTriPeriod as remainder in the steps for division.
    pub s_f_x_tri_period_remainder: u8,
    /// `SpareMem0618`（`$0618`，7 B）——$0618
    pub spare_mem0618: [u8; 7],
    /// `HasBeamSFX`（`$061F`，1 B）——Bit 7 set=has long beam, bit 0 set=has ice beam
    pub has_beam_sfx: u8,
    /// `MusicSQ1InstrLength`（`$0620`，1 B）——Holds number of frames to play SQ1 channel data
    pub music_sq1_instr_length: u8,
    /// `MusicSQ2InstrLength`（`$0621`，1 B）——Holds number of frames to play SQ2 channel data
    pub music_sq2_instr_length: u8,
    /// `MusicTriInstrLength`（`$0622`，1 B）——Holds number of frames to play Triangle channel data
    pub music_tri_instr_length: u8,
    /// `MusicNoiseInstrLength`（`$0623`，1 B）——Holds number of frames to play Noise channel data
    pub music_noise_instr_length: u8,
    /// `MusicSQ1RepeatCounter`（`$0624`，1 B）——Number of times to repeat SQ1 music loop
    pub music_sq1_repeat_counter: u8,
    /// `MusicSQ2RepeatCounter`（`$0625`，1 B）——Number of times to repeat SQ2 music loop
    pub music_sq2_repeat_counter: u8,
    /// `MusicTriRepeatCounter`（`$0626`，1 B）——Number of times to repeat Triangle music loop
    pub music_tri_repeat_counter: u8,
    /// `MusicNoiseRepeatCounter`（`$0627`，1 B）——Number of times to repeat Noise music loop
    pub music_noise_repeat_counter: u8,
    /// `MusicSQ1DutyEnvelope`（`$0628`，1 B）——Loaded into SQ1_VOL when playing music
    pub music_sq1_duty_envelope: u8,
    /// `MusicSQ2DutyEnvelope`（`$0629`，1 B）——Loaded into SQ2_VOL when playing music
    pub music_sq2_duty_envelope: u8,
    /// `MusicTriLinearCount`（`$062A`，1 B）——disable\enable counter, linear count length
    pub music_tri_linear_count: u8,
    /// `NoteLengthTblOffset`（`$062B`，1 B）——Stores the offset to find proper note length table
    pub note_length_tbl_offset: u8,
    /// `MusicLoopsOnEnd`（`$062C`，1 B）——0=Music does not repeat when it ends, Nonzero=music repeats
    pub music_loops_on_end: u8,
    /// `MusicTriCounterCntrl`（`$062D`，1 B）——$X0=disable length cntr
    pub music_tri_counter_cntrl: u8,
    /// `MusicSQ1VolumeEnvelopeIndex`（`$062E`，1 B）——Entry number in VolumeEnvelopePtrTable for SQ1
    pub music_sq1_volume_envelope_index: u8,
    /// `MusicSQ2VolumeEnvelopeIndex`（`$062F`，1 B）——Entry number in VolumeEnvelopePtrTable for SQ2
    pub music_sq2_volume_envelope_index: u8,
    /// `MusicSQ1Base`（`$0630`，2 B）——Low byte of base address for SQ1 music data
    pub music_sq1_base: u16,
    /// `MusicSQ2Base`（`$0632`，2 B）——Low byte of base address for SQ2 music data
    pub music_sq2_base: u16,
    /// `MusicTriBase`（`$0634`，2 B）——Low byte of base address for Triangle music data
    pub music_tri_base: u16,
    /// `MusicNoiseBase`（`$0636`，2 B）——Low byte of base address for Noise music data
    pub music_noise_base: u16,
    /// `MusicSQ1InstrID`（`$0638`，1 B）——Index to find SQ1 sound data instruction. Base=$630,$631
    pub music_sq1_instr_i_d: u8,
    /// `MusicSQ2InstrID`（`$0639`，1 B）——Index to find SQ2 sound data instruction. Base=$632,$633
    pub music_sq2_instr_i_d: u8,
    /// `MusicTriInstrID`（`$063A`，1 B）——Index to find Tri sound data instruction. Base=$634,$635
    pub music_tri_instr_i_d: u8,
    /// `MusicNoiseInstrID`（`$063B`，1 B）——Index to find Noise sound data instruction. Base=$636,$637
    pub music_noise_instr_i_d: u8,
    /// `MusicSQ1RepeatID`（`$063C`，1 B）——SQ1 repeat start index
    pub music_sq1_repeat_i_d: u8,
    /// `MusicSQ2RepeatID`（`$063D`，1 B）——SQ2 repeat start index
    pub music_sq2_repeat_i_d: u8,
    /// `MusicTriRepeatID`（`$063E`，1 B）——Triangle repeat start index
    pub music_tri_repeat_i_d: u8,
    /// `MusicNoiseRepeatID`（`$063F`，1 B）——Noise repeat start index
    pub music_noise_repeat_i_d: u8,
    /// `MusicSQ1InstrDelay`（`$0640`，1 B）——Decrements every sq1 frame. When 0, load new data
    pub music_sq1_instr_delay: u8,
    /// `MusicSQ2InstrDelay`（`$0641`，1 B）——Decrements every sq2 frame. when 0, load new data
    pub music_sq2_instr_delay: u8,
    /// `MusicTriInstrDelay`（`$0642`，1 B）——Decrements every triangle frame. When 0, load new data
    pub music_tri_instr_delay: u8,
    /// `MusicNoiseInstrDelay`（`$0643`，1 B）——Decrements every noise frame. When 0, load new data
    pub music_noise_instr_delay: u8,
    /// `SpareMem0644`（`$0644`，4 B）——$0644
    pub spare_mem0644: [u8; 4],
    /// `MusicSQ1Sweep`（`$0648`，1 B）——Value is loaded into SQ1_SWEEP when playing music
    pub music_sq1_sweep: u8,
    /// `MusicSQ2Sweep`（`$0649`，1 B）——Value is loaded into SQ2_SWEEP when playing music
    pub music_sq2_sweep: u8,
    /// `MusicTriSweep`（`$064A`，1 B）——Loaded into TRI_UNUSED(not used)
    pub music_tri_sweep: u8,
    /// `ThisSoundChannel`（`$064B`，1 B）——Least sig. byte of current channel(00,04,08 or 0C)
    pub this_sound_channel: u8,
    /// `SpareMem064C`（`$064C`，1 B）——（asm 无注释）
    pub spare_mem064_c: u8,
    /// `CurrentSoundFlags`（`$064D`，1 B）——Stores flags of SFX/music currently being processed.
    pub current_sound_flags: u8,
    /// `SpareMem064E`（`$064E`，4 B）——$064E
    pub spare_mem064_e: [u8; 4],
    /// `NoiseUsedBySFX`（`$0652`，1 B）——Noise in use? (Not used)
    pub noise_used_by_sfx: u8,
    /// `SQ1UsedBySFX`（`$0653`，1 B）——1=SQ1 channel being used by SFX, 0=not in use
    pub s_q1_used_by_sfx: u8,
    /// `SQ2UsedBySFX`（`$0654`，1 B）——2=SQ2 channel being used by SFX, 0=not in use
    pub s_q2_used_by_sfx: u8,
    /// `TriUsedBySFX`（`$0655`，1 B）——3=Triangle channel being used by SFX, 0=not in use
    pub tri_used_by_sfx: u8,
    /// `SpareMem0656`（`$0656`，6 B）——$0656
    pub spare_mem0656: [u8; 6],
    /// `ChannelType`（`$065C`，1 B）——Stores channel type being processed(0,1,2,3 or 4)
    pub channel_type: u8,
    /// `MusicLoopFlags`（`$065D`，1 B）——Stores flags of music to loop
    pub music_loop_flags: u8,
    /// `MusicInitIndex`（`$065E`，1 B）——index for loading $62B thru $637(base=$BD31).
    pub music_init_index: u8,
    /// `SpareMem065F`（`$065F`，1 B）——（asm 无注释）
    pub spare_mem065_f: u8,
    /// `SFXNoiseLength`（`$0660`，1 B）——Stores number of frames to play Noise SFX
    pub s_f_x_noise_length: u8,
    /// `SFXSQ1Length`（`$0661`，1 B）——Stores number of frames to play SQ1 SFX
    pub s_f_x_sq1_length: u8,
    /// `SFXSQ2Length`（`$0662`，1 B）——Stores number of frames to play SQ2 SFX
    pub s_f_x_sq2_length: u8,
    /// `SFXTriLength`（`$0663`，1 B）——Stores number of frames to play Triangle SFX
    pub s_f_x_tri_length: u8,
    /// `SFXMultiLength`（`$0664`，1 B）——Stores number of frames to play Multi SFX
    pub s_f_x_multi_length: u8,
    /// `SFXNoiseFrame`（`$0665`，1 B）——Stores current frame number for noise SFX
    pub s_f_x_noise_frame: u8,
    /// `SFXSQ1Frame`（`$0666`，1 B）——Stores current frame number for sq1 SFX
    pub s_f_x_sq1_frame: u8,
    /// `SFXSQ2Frame`（`$0667`，1 B）——Stores current frame number for SQ2 SFX
    pub s_f_x_sq2_frame: u8,
    /// `SFXTriFrame`（`$0668`，1 B）——Stores current frame number for triangle SFX
    pub s_f_x_tri_frame: u8,
    /// `SFXMultiFrame`（`$0669`，1 B）——Stores current frame number for Multi SFX
    pub s_f_x_multi_frame: u8,
    /// `MusicSQ1VolumeIndex`（`$066A`，1 B）——Stores index to SQ1 volume data in a volume data tbl
    pub music_sq1_volume_index: u8,
    /// `MusicSQ2VolumeIndex`（`$066B`，1 B）——Stores index to SQ2 volume data in a volume data tbl
    pub music_sq2_volume_index: u8,
    /// `MusicSQ1VolumeData`（`$066C`，1 B）——stores duty cycle and this frame volume data of SQ1
    pub music_sq1_volume_data: u8,
    /// `MusicSQ2VolumeData`（`$066D`，1 B）——Stores duty cycle and this frame volume data of SQ2
    pub music_sq2_volume_data: u8,
    /// `SpareMem066E`（`$066E`，2 B）——$066E is unknowingly written to by "sta MusicSQ1VolumeData,x", but never read
    pub spare_mem066_e: u16,
    /// `SFXNoiseVar0`（`$0670`，1 B）——Stores additional info for Noise SFX
    pub s_f_x_noise_var0: u8,
    /// `SFXSQ1Var0`（`$0671`，1 B）——Stores additional info for SQ1 SFX or multi SFX
    pub s_f_x_sq1_var0: u8,
    /// `SFXSQ2Var0`（`$0672`，1 B）——Stores additional info for SQ2 SFX
    pub s_f_x_sq2_var0: u8,
    /// `SFXTriVar0`（`$0673`，1 B）——Stores additional info for triangle SFX
    pub s_f_x_tri_var0: u8,
    /// `SFXNoiseVar1`（`$0674`，1 B）——Stores additional info for Noise SFX
    pub s_f_x_noise_var1: u8,
    /// `SFXSQ1Var1`（`$0675`，1 B）——Stores additional info for SQ1 SFX or multi SFX
    pub s_f_x_sq1_var1: u8,
    /// `SFXSQ2Var1`（`$0676`，1 B）——Stores additional info for SQ2 SFX
    pub s_f_x_sq2_var1: u8,
    /// `SFXTriVar1`（`$0677`，1 B）——Stores additional info for triangle SFX
    pub s_f_x_tri_var1: u8,
    /// `SFXNoiseVar2`（`$0678`，1 B）——Contains extra data for screw attack SFX
    pub s_f_x_noise_var2: u8,
    /// `SFXSQ1Var2`（`$0679`，1 B）——Stores additional info for SQ1 SFX or multi SFX
    pub s_f_x_sq1_var2: u8,
    /// `SFXSQ2Var2`（`$067A`，1 B）——Stores additional info for SQ2 SFX
    pub s_f_x_sq2_var2: u8,
    /// `SFXTriVar2`（`$067B`，1 B）——Stores additional info for triangle SFX
    pub s_f_x_tri_var2: u8,
    /// `SpareMem067C`（`$067C`，4 B）——$067C
    pub spare_mem067_c: [u8; 4],
    /// `SFXNoiseInitFlags`（`$0680`，1 B）——Initialization flags for noise SFX
    pub s_f_x_noise_init_flags: u8,
    /// `SFXSQ1InitFlags`（`$0681`，1 B）——Initialization flags for SQ1 SFX
    pub s_f_x_sq1_init_flags: u8,
    /// `SFXSQ2InitFlags`（`$0682`，1 B）——Initialization flags for SQ2 SFX(never used)
    pub s_f_x_sq2_init_flags: u8,
    /// `MusicInitFlags`（`$0685`，1 B）——Music init flags
    pub music_init_flags: u8,
    /// `ScrewAttack0686`（`$0686`，1 B）——（asm 无注释）
    pub screw_attack0686: u8,
    /// `SpareMem0687`（`$0687`，1 B）——（asm 无注释）
    pub spare_mem0687: u8,
    /// `SFXNoiseContFlags`（`$0688`，1 B）——Continuation flags for noise SFX
    pub s_f_x_noise_cont_flags: u8,
    /// `SFXSQ1ContFlags`（`$0689`，1 B）——Continuation flags for SQ1 SFX
    pub s_f_x_sq1_cont_flags: u8,
    /// `SFXSQ2ContFlags`（`$068A`，1 B）——Continuation flags for SQ2 SFX (never used)
    pub s_f_x_sq2_cont_flags: u8,
    /// `SFXTriContFlags`（`$068B`，1 B）——Continuation flags for Triangle SFX
    pub s_f_x_tri_cont_flags: u8,
    /// `SFXMultiContFlags`（`$068C`，1 B）——Continuation flags for Multi SFX
    pub s_f_x_multi_cont_flags: u8,
    /// `MusicContFlags`（`$068D`，1 B）——Stores the flag of the current music being played
    pub music_cont_flags: u8,
}

impl SoundRamFace {
    /// 从 2KB RAM 镜像解码（跳过 `$0683/$0684`——那两字节的真相在 `SoundInitFlags`）。
    pub fn decode(ram: &[u8]) -> Self {
        let at = usize::from(BASE & 0x7FF);
        let bytes = &ram[at..at + usize::from(FACE_END - BASE)];
        let music_sq1_period_low = bytes[0x00];
        let music_sq1_period_high = bytes[0x01];
        let s_f_x_paused = bytes[0x02];
        let pause_sfx_status = bytes[0x03];
        let music_sq2_period_low = bytes[0x04];
        let music_sq2_period_high = bytes[0x05];
        let spare_mem0606 = bytes[0x06];
        let load_music_sq1_sq2_periods_flag = bytes[0x07];
        let music_tri_period_low = bytes[0x08];
        let music_tri_period_high = bytes[0x09];
        let mut spare_mem060_a = [0u8; 6];
        spare_mem060_a.copy_from_slice(&bytes[0x0A..0x0A + 6]);
        let s_f_x_tri_period_low = bytes[0x10];
        let s_f_x_tri_period_high = bytes[0x11];
        let s_f_x_tri_change_low = bytes[0x12];
        let s_f_x_tri_change_high = bytes[0x13];
        let s_f_x_tri_period_divided_low = bytes[0x14];
        let s_f_x_tri_period_divided_high = bytes[0x15];
        let s_f_x_tri_period_divisor = bytes[0x16];
        let s_f_x_tri_period_remainder = bytes[0x17];
        let mut spare_mem0618 = [0u8; 7];
        spare_mem0618.copy_from_slice(&bytes[0x18..0x18 + 7]);
        let has_beam_sfx = bytes[0x1F];
        let music_sq1_instr_length = bytes[0x20];
        let music_sq2_instr_length = bytes[0x21];
        let music_tri_instr_length = bytes[0x22];
        let music_noise_instr_length = bytes[0x23];
        let music_sq1_repeat_counter = bytes[0x24];
        let music_sq2_repeat_counter = bytes[0x25];
        let music_tri_repeat_counter = bytes[0x26];
        let music_noise_repeat_counter = bytes[0x27];
        let music_sq1_duty_envelope = bytes[0x28];
        let music_sq2_duty_envelope = bytes[0x29];
        let music_tri_linear_count = bytes[0x2A];
        let note_length_tbl_offset = bytes[0x2B];
        let music_loops_on_end = bytes[0x2C];
        let music_tri_counter_cntrl = bytes[0x2D];
        let music_sq1_volume_envelope_index = bytes[0x2E];
        let music_sq2_volume_envelope_index = bytes[0x2F];
        let music_sq1_base = u16::from_le_bytes([bytes[0x30], bytes[0x30 + 1]]);
        let music_sq2_base = u16::from_le_bytes([bytes[0x32], bytes[0x32 + 1]]);
        let music_tri_base = u16::from_le_bytes([bytes[0x34], bytes[0x34 + 1]]);
        let music_noise_base = u16::from_le_bytes([bytes[0x36], bytes[0x36 + 1]]);
        let music_sq1_instr_i_d = bytes[0x38];
        let music_sq2_instr_i_d = bytes[0x39];
        let music_tri_instr_i_d = bytes[0x3A];
        let music_noise_instr_i_d = bytes[0x3B];
        let music_sq1_repeat_i_d = bytes[0x3C];
        let music_sq2_repeat_i_d = bytes[0x3D];
        let music_tri_repeat_i_d = bytes[0x3E];
        let music_noise_repeat_i_d = bytes[0x3F];
        let music_sq1_instr_delay = bytes[0x40];
        let music_sq2_instr_delay = bytes[0x41];
        let music_tri_instr_delay = bytes[0x42];
        let music_noise_instr_delay = bytes[0x43];
        let mut spare_mem0644 = [0u8; 4];
        spare_mem0644.copy_from_slice(&bytes[0x44..0x44 + 4]);
        let music_sq1_sweep = bytes[0x48];
        let music_sq2_sweep = bytes[0x49];
        let music_tri_sweep = bytes[0x4A];
        let this_sound_channel = bytes[0x4B];
        let spare_mem064_c = bytes[0x4C];
        let current_sound_flags = bytes[0x4D];
        let mut spare_mem064_e = [0u8; 4];
        spare_mem064_e.copy_from_slice(&bytes[0x4E..0x4E + 4]);
        let noise_used_by_sfx = bytes[0x52];
        let s_q1_used_by_sfx = bytes[0x53];
        let s_q2_used_by_sfx = bytes[0x54];
        let tri_used_by_sfx = bytes[0x55];
        let mut spare_mem0656 = [0u8; 6];
        spare_mem0656.copy_from_slice(&bytes[0x56..0x56 + 6]);
        let channel_type = bytes[0x5C];
        let music_loop_flags = bytes[0x5D];
        let music_init_index = bytes[0x5E];
        let spare_mem065_f = bytes[0x5F];
        let s_f_x_noise_length = bytes[0x60];
        let s_f_x_sq1_length = bytes[0x61];
        let s_f_x_sq2_length = bytes[0x62];
        let s_f_x_tri_length = bytes[0x63];
        let s_f_x_multi_length = bytes[0x64];
        let s_f_x_noise_frame = bytes[0x65];
        let s_f_x_sq1_frame = bytes[0x66];
        let s_f_x_sq2_frame = bytes[0x67];
        let s_f_x_tri_frame = bytes[0x68];
        let s_f_x_multi_frame = bytes[0x69];
        let music_sq1_volume_index = bytes[0x6A];
        let music_sq2_volume_index = bytes[0x6B];
        let music_sq1_volume_data = bytes[0x6C];
        let music_sq2_volume_data = bytes[0x6D];
        let spare_mem066_e = u16::from_le_bytes([bytes[0x6E], bytes[0x6E + 1]]);
        let s_f_x_noise_var0 = bytes[0x70];
        let s_f_x_sq1_var0 = bytes[0x71];
        let s_f_x_sq2_var0 = bytes[0x72];
        let s_f_x_tri_var0 = bytes[0x73];
        let s_f_x_noise_var1 = bytes[0x74];
        let s_f_x_sq1_var1 = bytes[0x75];
        let s_f_x_sq2_var1 = bytes[0x76];
        let s_f_x_tri_var1 = bytes[0x77];
        let s_f_x_noise_var2 = bytes[0x78];
        let s_f_x_sq1_var2 = bytes[0x79];
        let s_f_x_sq2_var2 = bytes[0x7A];
        let s_f_x_tri_var2 = bytes[0x7B];
        let mut spare_mem067_c = [0u8; 4];
        spare_mem067_c.copy_from_slice(&bytes[0x7C..0x7C + 4]);
        let s_f_x_noise_init_flags = bytes[0x80];
        let s_f_x_sq1_init_flags = bytes[0x81];
        let s_f_x_sq2_init_flags = bytes[0x82];
        let music_init_flags = bytes[0x85];
        let screw_attack0686 = bytes[0x86];
        let spare_mem0687 = bytes[0x87];
        let s_f_x_noise_cont_flags = bytes[0x88];
        let s_f_x_sq1_cont_flags = bytes[0x89];
        let s_f_x_sq2_cont_flags = bytes[0x8A];
        let s_f_x_tri_cont_flags = bytes[0x8B];
        let s_f_x_multi_cont_flags = bytes[0x8C];
        let music_cont_flags = bytes[0x8D];
        Self {
            music_sq1_period_low,
            music_sq1_period_high,
            s_f_x_paused,
            pause_sfx_status,
            music_sq2_period_low,
            music_sq2_period_high,
            spare_mem0606,
            load_music_sq1_sq2_periods_flag,
            music_tri_period_low,
            music_tri_period_high,
            spare_mem060_a,
            s_f_x_tri_period_low,
            s_f_x_tri_period_high,
            s_f_x_tri_change_low,
            s_f_x_tri_change_high,
            s_f_x_tri_period_divided_low,
            s_f_x_tri_period_divided_high,
            s_f_x_tri_period_divisor,
            s_f_x_tri_period_remainder,
            spare_mem0618,
            has_beam_sfx,
            music_sq1_instr_length,
            music_sq2_instr_length,
            music_tri_instr_length,
            music_noise_instr_length,
            music_sq1_repeat_counter,
            music_sq2_repeat_counter,
            music_tri_repeat_counter,
            music_noise_repeat_counter,
            music_sq1_duty_envelope,
            music_sq2_duty_envelope,
            music_tri_linear_count,
            note_length_tbl_offset,
            music_loops_on_end,
            music_tri_counter_cntrl,
            music_sq1_volume_envelope_index,
            music_sq2_volume_envelope_index,
            music_sq1_base,
            music_sq2_base,
            music_tri_base,
            music_noise_base,
            music_sq1_instr_i_d,
            music_sq2_instr_i_d,
            music_tri_instr_i_d,
            music_noise_instr_i_d,
            music_sq1_repeat_i_d,
            music_sq2_repeat_i_d,
            music_tri_repeat_i_d,
            music_noise_repeat_i_d,
            music_sq1_instr_delay,
            music_sq2_instr_delay,
            music_tri_instr_delay,
            music_noise_instr_delay,
            spare_mem0644,
            music_sq1_sweep,
            music_sq2_sweep,
            music_tri_sweep,
            this_sound_channel,
            spare_mem064_c,
            current_sound_flags,
            spare_mem064_e,
            noise_used_by_sfx,
            s_q1_used_by_sfx,
            s_q2_used_by_sfx,
            tri_used_by_sfx,
            spare_mem0656,
            channel_type,
            music_loop_flags,
            music_init_index,
            spare_mem065_f,
            s_f_x_noise_length,
            s_f_x_sq1_length,
            s_f_x_sq2_length,
            s_f_x_tri_length,
            s_f_x_multi_length,
            s_f_x_noise_frame,
            s_f_x_sq1_frame,
            s_f_x_sq2_frame,
            s_f_x_tri_frame,
            s_f_x_multi_frame,
            music_sq1_volume_index,
            music_sq2_volume_index,
            music_sq1_volume_data,
            music_sq2_volume_data,
            spare_mem066_e,
            s_f_x_noise_var0,
            s_f_x_sq1_var0,
            s_f_x_sq2_var0,
            s_f_x_tri_var0,
            s_f_x_noise_var1,
            s_f_x_sq1_var1,
            s_f_x_sq2_var1,
            s_f_x_tri_var1,
            s_f_x_noise_var2,
            s_f_x_sq1_var2,
            s_f_x_sq2_var2,
            s_f_x_tri_var2,
            spare_mem067_c,
            s_f_x_noise_init_flags,
            s_f_x_sq1_init_flags,
            s_f_x_sq2_init_flags,
            music_init_flags,
            screw_attack0686,
            spare_mem0687,
            s_f_x_noise_cont_flags,
            s_f_x_sq1_cont_flags,
            s_f_x_sq2_cont_flags,
            s_f_x_tri_cont_flags,
            s_f_x_multi_cont_flags,
            music_cont_flags
        }
    }

    /// 编码回 2KB RAM 镜像（只写本面两段，不触碰 `$0683/$0684`）。
    pub fn encode(&self, ram: &mut [u8]) {
        let at = usize::from(BASE & 0x7FF);
        let bytes = &mut ram[at..at + usize::from(FACE_END - BASE)];
        bytes[0x00] = self.music_sq1_period_low;
        bytes[0x01] = self.music_sq1_period_high;
        bytes[0x02] = self.s_f_x_paused;
        bytes[0x03] = self.pause_sfx_status;
        bytes[0x04] = self.music_sq2_period_low;
        bytes[0x05] = self.music_sq2_period_high;
        bytes[0x06] = self.spare_mem0606;
        bytes[0x07] = self.load_music_sq1_sq2_periods_flag;
        bytes[0x08] = self.music_tri_period_low;
        bytes[0x09] = self.music_tri_period_high;
        // SpareMem060A: 6 B 未解释，原字节 round-trip（不赋义）。
        bytes[0x0A..0x0A + 6].copy_from_slice(&self.spare_mem060_a);
        bytes[0x10] = self.s_f_x_tri_period_low;
        bytes[0x11] = self.s_f_x_tri_period_high;
        bytes[0x12] = self.s_f_x_tri_change_low;
        bytes[0x13] = self.s_f_x_tri_change_high;
        bytes[0x14] = self.s_f_x_tri_period_divided_low;
        bytes[0x15] = self.s_f_x_tri_period_divided_high;
        bytes[0x16] = self.s_f_x_tri_period_divisor;
        bytes[0x17] = self.s_f_x_tri_period_remainder;
        // SpareMem0618: 7 B 未解释，原字节 round-trip（不赋义）。
        bytes[0x18..0x18 + 7].copy_from_slice(&self.spare_mem0618);
        bytes[0x1F] = self.has_beam_sfx;
        bytes[0x20] = self.music_sq1_instr_length;
        bytes[0x21] = self.music_sq2_instr_length;
        bytes[0x22] = self.music_tri_instr_length;
        bytes[0x23] = self.music_noise_instr_length;
        bytes[0x24] = self.music_sq1_repeat_counter;
        bytes[0x25] = self.music_sq2_repeat_counter;
        bytes[0x26] = self.music_tri_repeat_counter;
        bytes[0x27] = self.music_noise_repeat_counter;
        bytes[0x28] = self.music_sq1_duty_envelope;
        bytes[0x29] = self.music_sq2_duty_envelope;
        bytes[0x2A] = self.music_tri_linear_count;
        bytes[0x2B] = self.note_length_tbl_offset;
        bytes[0x2C] = self.music_loops_on_end;
        bytes[0x2D] = self.music_tri_counter_cntrl;
        bytes[0x2E] = self.music_sq1_volume_envelope_index;
        bytes[0x2F] = self.music_sq2_volume_envelope_index;
        bytes[0x30..0x30 + 2].copy_from_slice(&self.music_sq1_base.to_le_bytes());
        bytes[0x32..0x32 + 2].copy_from_slice(&self.music_sq2_base.to_le_bytes());
        bytes[0x34..0x34 + 2].copy_from_slice(&self.music_tri_base.to_le_bytes());
        bytes[0x36..0x36 + 2].copy_from_slice(&self.music_noise_base.to_le_bytes());
        bytes[0x38] = self.music_sq1_instr_i_d;
        bytes[0x39] = self.music_sq2_instr_i_d;
        bytes[0x3A] = self.music_tri_instr_i_d;
        bytes[0x3B] = self.music_noise_instr_i_d;
        bytes[0x3C] = self.music_sq1_repeat_i_d;
        bytes[0x3D] = self.music_sq2_repeat_i_d;
        bytes[0x3E] = self.music_tri_repeat_i_d;
        bytes[0x3F] = self.music_noise_repeat_i_d;
        bytes[0x40] = self.music_sq1_instr_delay;
        bytes[0x41] = self.music_sq2_instr_delay;
        bytes[0x42] = self.music_tri_instr_delay;
        bytes[0x43] = self.music_noise_instr_delay;
        // SpareMem0644: 4 B 未解释，原字节 round-trip（不赋义）。
        bytes[0x44..0x44 + 4].copy_from_slice(&self.spare_mem0644);
        bytes[0x48] = self.music_sq1_sweep;
        bytes[0x49] = self.music_sq2_sweep;
        bytes[0x4A] = self.music_tri_sweep;
        bytes[0x4B] = self.this_sound_channel;
        bytes[0x4C] = self.spare_mem064_c;
        bytes[0x4D] = self.current_sound_flags;
        // SpareMem064E: 4 B 未解释，原字节 round-trip（不赋义）。
        bytes[0x4E..0x4E + 4].copy_from_slice(&self.spare_mem064_e);
        bytes[0x52] = self.noise_used_by_sfx;
        bytes[0x53] = self.s_q1_used_by_sfx;
        bytes[0x54] = self.s_q2_used_by_sfx;
        bytes[0x55] = self.tri_used_by_sfx;
        // SpareMem0656: 6 B 未解释，原字节 round-trip（不赋义）。
        bytes[0x56..0x56 + 6].copy_from_slice(&self.spare_mem0656);
        bytes[0x5C] = self.channel_type;
        bytes[0x5D] = self.music_loop_flags;
        bytes[0x5E] = self.music_init_index;
        bytes[0x5F] = self.spare_mem065_f;
        bytes[0x60] = self.s_f_x_noise_length;
        bytes[0x61] = self.s_f_x_sq1_length;
        bytes[0x62] = self.s_f_x_sq2_length;
        bytes[0x63] = self.s_f_x_tri_length;
        bytes[0x64] = self.s_f_x_multi_length;
        bytes[0x65] = self.s_f_x_noise_frame;
        bytes[0x66] = self.s_f_x_sq1_frame;
        bytes[0x67] = self.s_f_x_sq2_frame;
        bytes[0x68] = self.s_f_x_tri_frame;
        bytes[0x69] = self.s_f_x_multi_frame;
        bytes[0x6A] = self.music_sq1_volume_index;
        bytes[0x6B] = self.music_sq2_volume_index;
        bytes[0x6C] = self.music_sq1_volume_data;
        bytes[0x6D] = self.music_sq2_volume_data;
        bytes[0x6E..0x6E + 2].copy_from_slice(&self.spare_mem066_e.to_le_bytes());
        bytes[0x70] = self.s_f_x_noise_var0;
        bytes[0x71] = self.s_f_x_sq1_var0;
        bytes[0x72] = self.s_f_x_sq2_var0;
        bytes[0x73] = self.s_f_x_tri_var0;
        bytes[0x74] = self.s_f_x_noise_var1;
        bytes[0x75] = self.s_f_x_sq1_var1;
        bytes[0x76] = self.s_f_x_sq2_var1;
        bytes[0x77] = self.s_f_x_tri_var1;
        bytes[0x78] = self.s_f_x_noise_var2;
        bytes[0x79] = self.s_f_x_sq1_var2;
        bytes[0x7A] = self.s_f_x_sq2_var2;
        bytes[0x7B] = self.s_f_x_tri_var2;
        // SpareMem067C: 4 B 未解释，原字节 round-trip（不赋义）。
        bytes[0x7C..0x7C + 4].copy_from_slice(&self.spare_mem067_c);
        bytes[0x80] = self.s_f_x_noise_init_flags;
        bytes[0x81] = self.s_f_x_sq1_init_flags;
        bytes[0x82] = self.s_f_x_sq2_init_flags;
        bytes[0x85] = self.music_init_flags;
        bytes[0x86] = self.screw_attack0686;
        bytes[0x87] = self.spare_mem0687;
        bytes[0x88] = self.s_f_x_noise_cont_flags;
        bytes[0x89] = self.s_f_x_sq1_cont_flags;
        bytes[0x8A] = self.s_f_x_sq2_cont_flags;
        bytes[0x8B] = self.s_f_x_tri_cont_flags;
        bytes[0x8C] = self.s_f_x_multi_cont_flags;
        bytes[0x8D] = self.music_cont_flags;
    }

    /// 地址是否落在本面的**两段**内（`$0683/$0684` 不算）。
    pub fn contains(addr: u16) -> bool {
        let a = addr & 0x7FF;
        (SEG1_START..SEG1_START + SEG1_LEN).contains(&a)
            || (SEG2_START..SEG2_START + SEG2_LEN).contains(&a)
    }

    /// 地址 → 字节（不在本面返回 `None`）。
    pub fn get_byte(&self, addr: u16) -> Option<u8> {
        let a = addr & 0x7FF;
        if !Self::contains(a) {
            return None;
        }
        let mut ram = [0u8; 0x800];
        self.encode(&mut ram);
        Some(ram[usize::from(a)])
    }

    /// 字节 → 地址（不在本面返回 `false`，调用方继续往下一域分派）。
    pub fn set_byte(&mut self, addr: u16, v: u8) -> bool {
        let a = addr & 0x7FF;
        if !Self::contains(a) {
            return false;
        }
        let mut ram = [0u8; 0x800];
        self.encode(&mut ram);
        ram[usize::from(a)] = v;
        *self = Self::decode(&ram);
        true
    }
}

impl Default for SoundRamFace {
    fn default() -> Self {
        Self::decode(&[0u8; 0x800])
    }
}

impl NesMapped for SoundRamFace {
    const DISCRIMINATOR: Option<u16> = None;

    /// 两段 `Owner`（`$0600-$0682` 131 B + `$0685-$068D` 9 B = 140 B）；
    /// `$0683/$0684` 留给 `SoundInitFlags`（owner 唯一）。
    const SPANS: &'static [(NesSpan, SpanKey)] = &[
        (NesSpan::value(SEG1_START, SEG1_LEN), SpanKey::Owner),
        (NesSpan::value(SEG2_START, SEG2_LEN), SpanKey::Owner),
    ];

    fn project(&self, out: &mut NesImage) {
        self.encode(&mut out.ram);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> [u8; 0x800] {
        let mut ram = [0x3Cu8; 0x800];
        let at = usize::from(BASE & 0x7FF);
        for (i, b) in ram[at..at + usize::from(FACE_END - BASE)].iter_mut().enumerate() {
            *b = (i as u8).wrapping_mul(29).wrapping_add(5);
        }
        ram
    }

    #[test]
    fn offsets_match_asm_enum() {
        let offsets: [u16; 110] = [OFF_MUSICSQ1PERIODLOW, OFF_MUSICSQ1PERIODHIGH, OFF_SFXPAUSED, OFF_PAUSESFXSTATUS, OFF_MUSICSQ2PERIODLOW, OFF_MUSICSQ2PERIODHIGH, OFF_SPAREMEM0606, OFF_LOADMUSICSQ1SQ2PERIODSFLAG, OFF_MUSICTRIPERIODLOW, OFF_MUSICTRIPERIODHIGH, OFF_SPAREMEM060A, OFF_SFXTRIPERIODLOW, OFF_SFXTRIPERIODHIGH, OFF_SFXTRICHANGELOW, OFF_SFXTRICHANGEHIGH, OFF_SFXTRIPERIODDIVIDEDLOW, OFF_SFXTRIPERIODDIVIDEDHIGH, OFF_SFXTRIPERIODDIVISOR, OFF_SFXTRIPERIODREMAINDER, OFF_SPAREMEM0618, OFF_HASBEAMSFX, OFF_MUSICSQ1INSTRLENGTH, OFF_MUSICSQ2INSTRLENGTH, OFF_MUSICTRIINSTRLENGTH, OFF_MUSICNOISEINSTRLENGTH, OFF_MUSICSQ1REPEATCOUNTER, OFF_MUSICSQ2REPEATCOUNTER, OFF_MUSICTRIREPEATCOUNTER, OFF_MUSICNOISEREPEATCOUNTER, OFF_MUSICSQ1DUTYENVELOPE, OFF_MUSICSQ2DUTYENVELOPE, OFF_MUSICTRILINEARCOUNT, OFF_NOTELENGTHTBLOFFSET, OFF_MUSICLOOPSONEND, OFF_MUSICTRICOUNTERCNTRL, OFF_MUSICSQ1VOLUMEENVELOPEINDEX, OFF_MUSICSQ2VOLUMEENVELOPEINDEX, OFF_MUSICSQ1BASE, OFF_MUSICSQ2BASE, OFF_MUSICTRIBASE, OFF_MUSICNOISEBASE, OFF_MUSICSQ1INSTRID, OFF_MUSICSQ2INSTRID, OFF_MUSICTRIINSTRID, OFF_MUSICNOISEINSTRID, OFF_MUSICSQ1REPEATID, OFF_MUSICSQ2REPEATID, OFF_MUSICTRIREPEATID, OFF_MUSICNOISEREPEATID, OFF_MUSICSQ1INSTRDELAY, OFF_MUSICSQ2INSTRDELAY, OFF_MUSICTRIINSTRDELAY, OFF_MUSICNOISEINSTRDELAY, OFF_SPAREMEM0644, OFF_MUSICSQ1SWEEP, OFF_MUSICSQ2SWEEP, OFF_MUSICTRISWEEP, OFF_THISSOUNDCHANNEL, OFF_SPAREMEM064C, OFF_CURRENTSOUNDFLAGS, OFF_SPAREMEM064E, OFF_NOISEUSEDBYSFX, OFF_SQ1USEDBYSFX, OFF_SQ2USEDBYSFX, OFF_TRIUSEDBYSFX, OFF_SPAREMEM0656, OFF_CHANNELTYPE, OFF_MUSICLOOPFLAGS, OFF_MUSICINITINDEX, OFF_SPAREMEM065F, OFF_SFXNOISELENGTH, OFF_SFXSQ1LENGTH, OFF_SFXSQ2LENGTH, OFF_SFXTRILENGTH, OFF_SFXMULTILENGTH, OFF_SFXNOISEFRAME, OFF_SFXSQ1FRAME, OFF_SFXSQ2FRAME, OFF_SFXTRIFRAME, OFF_SFXMULTIFRAME, OFF_MUSICSQ1VOLUMEINDEX, OFF_MUSICSQ2VOLUMEINDEX, OFF_MUSICSQ1VOLUMEDATA, OFF_MUSICSQ2VOLUMEDATA, OFF_SPAREMEM066E, OFF_SFXNOISEVAR0, OFF_SFXSQ1VAR0, OFF_SFXSQ2VAR0, OFF_SFXTRIVAR0, OFF_SFXNOISEVAR1, OFF_SFXSQ1VAR1, OFF_SFXSQ2VAR1, OFF_SFXTRIVAR1, OFF_SFXNOISEVAR2, OFF_SFXSQ1VAR2, OFF_SFXSQ2VAR2, OFF_SFXTRIVAR2, OFF_SPAREMEM067C, OFF_SFXNOISEINITFLAGS, OFF_SFXSQ1INITFLAGS, OFF_SFXSQ2INITFLAGS, OFF_MUSICINITFLAGS, OFF_SCREWATTACK0686, OFF_SPAREMEM0687, OFF_SFXNOISECONTFLAGS, OFF_SFXSQ1CONTFLAGS, OFF_SFXSQ2CONTFLAGS, OFF_SFXTRICONTFLAGS, OFF_SFXMULTICONTFLAGS, OFF_MUSICCONTFLAGS];
        let widths: [u16; 110] = [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 6, 1, 1, 1, 1, 1, 1, 1, 1, 7, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 4, 1, 1, 1, 1, 1, 1, 4, 1, 1, 1, 1, 6, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 4, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1];
        let mut cursor = BASE;
        for (off, w) in offsets.iter().zip(widths.iter()) {
            if cursor == SOUND_INIT_FLAGS_START {
                cursor += SOUND_INIT_FLAGS_LEN;  // 跳过 SoundInitFlags 的 2 B
            }
            assert_eq!(*off, cursor, "字段地址必须按 asm 顺序连续（跳过 $0683/$0684）");
            cursor += w;
        }
        assert_eq!(cursor, FACE_END, "面尾必须落在 $068E");
        assert_eq!(SEG1_LEN + SEG2_LEN + SOUND_INIT_FLAGS_LEN, FACE_END - BASE);
    }

    #[test]
    fn spans_cover_exactly_the_two_segments() {
        assert_eq!(SoundRamFace::SPANS.len(), 2);
        let (s1, k1) = SoundRamFace::SPANS[0];
        let (s2, k2) = SoundRamFace::SPANS[1];
        assert!(matches!(k1, SpanKey::Owner) && matches!(k2, SpanKey::Owner));
        assert_eq!((s1.start, s1.len), (0x0600, 0x83));
        assert_eq!((s2.start, s2.len), (0x0685, 0x09));
        assert_eq!(s1.start + s1.len + SOUND_INIT_FLAGS_LEN, s2.start, "两段之间只隔 $0683/$0684");
    }

    #[test]
    fn round_trip_is_byte_exact_outside_sound_init_flags() {
        for ram in [sample(), [0u8; 0x800], [0xFFu8; 0x800]] {
            let face = SoundRamFace::decode(&ram);
            let mut out = [0x3Cu8; 0x800];
            face.encode(&mut out);
            let at = usize::from(BASE & 0x7FF);
            // 两段逐字节相等。
            assert_eq!(
                &out[at..at + usize::from(SEG1_LEN)],
                &ram[at..at + usize::from(SEG1_LEN)]
            );
            let a2 = usize::from(SEG2_START & 0x7FF);
            assert_eq!(
                &out[a2..a2 + usize::from(SEG2_LEN)],
                &ram[a2..a2 + usize::from(SEG2_LEN)]
            );
            // `$0683/$0684` 原样不动（哪怕是 0x3C 的底板值）。
            assert_eq!(out[0x0683usize], 0x3C);
            assert_eq!(out[0x0684usize], 0x3C);
        }
    }

    #[test]
    fn encode_touches_only_the_two_segments() {
        let face = SoundRamFace::decode(&sample());
        let mut ram = [0x3Cu8; 0x800];
        face.encode(&mut ram);
        assert!(ram[..usize::from(BASE)].iter().all(|b| *b == 0x3C));
        assert!(ram[usize::from(FACE_END)..].iter().all(|b| *b == 0x3C));
        assert_eq!(ram[0x0683], 0x3C, "$0683 不属本面");
        assert_eq!(ram[0x0684], 0x3C, "$0684 不属本面");
    }

    #[test]
    fn byte_accessors_cover_exactly_the_two_segments() {
        let face = SoundRamFace::decode(&sample());
        for addr in (SEG1_START..SEG1_START + SEG1_LEN).chain(SEG2_START..SEG2_START + SEG2_LEN) {
            assert!(face.get_byte(addr).is_some(), "{addr:#06X} 应可读");
        }
        assert!(face.get_byte(0x0683).is_none(), "$0683 归 SoundInitFlags");
        assert!(face.get_byte(0x0684).is_none(), "$0684 归 SoundInitFlags");
        assert!(face.get_byte(BASE - 1).is_none());
        assert!(face.get_byte(FACE_END).is_none());

        let mut mutable = SoundRamFace::default();
        for addr in SEG1_START..SEG1_START + SEG1_LEN {
            assert!(mutable.set_byte(addr, 0x5A));
        }
        for addr in SEG2_START..SEG2_START + SEG2_LEN {
            assert!(mutable.set_byte(addr, 0x5A));
        }
        assert!(!mutable.set_byte(0x0683, 1), "$0683 拒绝写入");
        assert_eq!(mutable.music_sq1_period_low, 0x5A);
        assert_eq!(mutable.music_cont_flags, 0x5A);
    }

    #[test]
    fn business_layer_reads_the_same_bytes() {
        assert_eq!(OFF_HASBEAMSFX, 0x061F);
        assert_eq!(OFF_THISSOUNDCHANNEL, 0x064B);
        assert_eq!(OFF_MUSICINITFLAGS, 0x0685);
        assert_eq!(OFF_MUSICCONTFLAGS, 0x068D);
        assert!(SoundRamFace::contains(0x061F) && SoundRamFace::contains(0x068D));
        assert!(!SoundRamFace::contains(0x0683) && !SoundRamFace::contains(0x0684));
    }

    #[test]
    fn project_writes_only_this_face() {
        let face = SoundRamFace::decode(&sample());
        let mut img = NesImage {
            ram: [0x3Cu8; 0x800],
            wram: [0u8; 0x2000],
        };
        face.project(&mut img);
        assert!(img.wram.iter().all(|b| *b == 0));
        assert!(img.ram[..usize::from(BASE)].iter().all(|b| *b == 0x3C));
        assert!(img.ram[usize::from(FACE_END)..].iter().all(|b| *b == 0x3C));
        assert_eq!(img.ram[0x0683], 0x3C);
        assert_eq!(img.ram[0x0684], 0x3C);
    }
}
