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

pub(crate) mod state;

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
    /// MusicInitFlags=$0685（音乐 init 旗——RunMusicInitRoutine $BC3D 周期）。
    pub use crate::ram_map::MUSIC_INIT as MUSIC_INIT_FLAGS; // $0685 单名（reviewer-music F2：别名收敛到 ram_map）
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
    /// MusicContFlags=$068D（正在播放的曲目旗）。
    pub const MUSIC_CONT_FLAGS: u16 = 0x068D;
    /// RandomNumber1=$2E（读取面复用 `Manager::random_number1`——frame/memory.rs 既有实现）。
    #[allow(dead_code)]
    pub const RANDOM_NUMBER1: u16 = 0x002E;
    /// LoadMusicSQ1SQ2PeriodsFlag=$0607（End*SFX 后音乐周期恢复记账）。
    pub const LOAD_MUSIC_PERIODS_FLAG: u16 = 0x0607;
    /// SFXTriPeriodLow/High=$0610/$0611。
    pub const TRI_PERIOD_LOW: u16 = 0x0610;
    pub const TRI_PERIOD_HIGH: u16 = 0x0611;
    /// SFXTriChangeLow/High=$0612/$0613（16bit 增减步长）。
    pub const TRI_CHANGE_LOW: u16 = 0x0612;
    pub const TRI_CHANGE_HIGH: u16 = 0x0613;
    /// SFXTriDivisor=$0616（除法面）。
    pub const TRI_DIVISOR: u16 = 0x0616;
    /// HasBeamSFX=$061F（bit7=LongBeam/bit0=IceBeam——BulletFire 变体）。
    pub const HAS_BEAM_SFX: u16 = 0x061F;
    /// Noise/SQ1/SQ2/Tri UsedBySFX=$0652-$0655。
    pub const NOISE_USED_BY_SFX: u16 = 0x0652;
    pub const SQ1_USED_BY_SFX: u16 = 0x0653;
    pub const SQ2_USED_BY_SFX: u16 = 0x0654;
    pub const TRI_USED_BY_SFX: u16 = 0x0655;
    /// SFXLength 块基址 $0660+ct（multi 槽 ct=4→$0664）。
    pub const SFX_LENGTH_BASE: u16 = 0x0660;
    /// SFXFrame 块基址 $0665+ct（multi 槽 ct=4→$0669）。
    pub const SFX_FRAME_BASE: u16 = 0x0665;
    /// SFXVar0/Var1/Var2 块基址 $0670/$0674/$0678（+ct；multi 无槽）。
    pub const SFX_VAR0_BASE: u16 = 0x0670;
    pub const SFX_VAR1_BASE: u16 = 0x0674;
    pub const SFX_VAR2_BASE: u16 = 0x0678;
    /// SFXNoiseContFlags 块基址 $0688+ct（Noise/SQ1/SQ2/Tri/Multi/MusicCont）。
    pub const CONT_FLAGS_BASE: u16 = 0x0688;
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
/// 【r17 ROM 终审更正】`cmp #$10`+`iny/iny`×=**8 位全扫**（位7..0，y=0..14）；
/// 表目 Y 偏移=2×(7-位号)（bit7→y=0 … bit0→y=14）——**无 +4**（旧注「表前 4B 双字/
/// 扫描自 y=4」漏看 `ldy #$00` 复位——prg[0x34BD] 字节实证：`a0 00 … c9 10 d0 f4`）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlagScan {
    /// 命中位号（7..0；位7 最先）。
    pub bit_number: Option<u8>,
    /// 表目 Y 偏移（2×(7-位号)——`(SoundRoutineTablePtr),y` 读面）。
    pub table_offset: u8,
}

/// 位扫描（`asl CurrentSoundFlags / bcs` MSB 优先 **8 轮**；无旗⇒None）。
pub fn scan_sound_flags(flags: u8) -> FlagScan {
    let mut v = flags;
    for i in 0..8u8 {
        let bit = 7 - i;
        let carry = v & 0x80 != 0;
        v = v.wrapping_shl(1);
        if carry {
            return FlagScan { bit_number: Some(bit), table_offset: i.wrapping_mul(2) };
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

/// GetSoundRoutine 全语义（$B4BD-$B4EE——r17 ROM 终审，F1/F2/F3 公共根）：
/// ①`sta $064D`（A=本周期旗副本——FindMusicInitIndex 的真实扫描源）②4B 拷
/// （TablePtr/DefaultRoutine→$E0-$E3——**默认先入 $E2/$E3**，探针 f16 双写实证）
/// +块[4]→$065C ③8 位 MSB 扫描：命中⇒表[y]双字**覆盖** $E2/$E3 ④pha/pla 恢复 $064D。
/// （block_win=块窗址——`stx $E4`（X=窗址 lo）/`$E5←$B2`（高字节恒量））。
/// 返回=（解析 routine=$E2/$E3 终值, 命中表目 Y, 写事件集）。
pub fn get_sound_routine_writes(block_win: u16, block: &SoundDataBlock, handler_table: &[u8], flags: u8)
    -> (u16, Option<u8>, Vec<(u16, u8)>) {
    let mut ev = Vec::new();
    ev.push((0x064D, flags)); // sta CurrentSoundFlags（旗副本——F1 根）
    ev.push((0x00E4, block_win as u8)); // stx $E4（块窗址 lo）
    ev.push((0x00E5, 0xB2)); // ldy #>GetSoundRoutineData / sty $E5（高字节恒 $B2）
    ev.push((0x00E0, block.table_ptr as u8));
    ev.push((0x00E1, (block.table_ptr >> 8) as u8));
    ev.push((0x00E2, block.default_routine as u8)); // 默认先入 $E2/$E3
    ev.push((0x00E3, (block.default_routine >> 8) as u8));
    ev.push((0x065C, block.channel_type));
    let scan = scan_sound_flags(flags);
    match scan.bit_number {
        None => (block.default_routine, None, ev),
        Some(_) => {
            let y = scan.table_offset as usize;
            let lo = handler_table.get(y).copied().unwrap_or(0);
            let hi = handler_table.get(y + 1).copied().unwrap_or(0);
            ev.push((0x00E2, lo)); // @SoundFlagFound：表[y]→$E2/$E3（覆盖默认）
            ev.push((0x00E3, hi));
            (u16::from_le_bytes([lo, hi]), Some(scan.table_offset), ev)
        }
    }
}

/// GetSoundRoutine 核（$B4BD 语义折叠——解析面）：同 `get_sound_routine_writes`，
/// 不出写事件。返回 (SoundRoutinePtr, 命中表目 Y)。
pub fn get_sound_routine(block: &SoundDataBlock, handler_table: &[u8], flags: u8)
    -> (u16, Option<u8>) {
    let (ptr, y, _) = get_sound_routine_writes(0, block, handler_table, flags);
    (ptr, y)
}

/// Multi 特例双步（FindMusicInitIndex $BC53+MusicInitIndexAdd8 $BC64）。
/// 【r17 降格（复核 F1）】**语义粗形非 asm 等价**（粗形 max(base)+8 于 f16 点给 0x07，
/// asm 真值 0x0B）——留档对照用；asm 等价件=`find_music_init_index_writes`。
pub fn multi_music_index_step(music_init_flags: u8, index: u8) -> u8 {
    let scan = scan_sound_flags(music_init_flags);
    let base = scan.bit_number.unwrap_or(0);
    index.max(base).wrapping_add(8) // 语义粗形——已降格（见上注）
}

/// FindMusicInitIndex+Add8 写序（asm 等价——$BC53/$BC64，复核 F1）：
/// `$065E←FF`（lda #$FF/sta）→ 若 `$064D`(CurrentSoundFlags)==0 ⇒止（$065E=FF）；
/// 否则 @loop：`inc $065E / asl / bcc @loop`——**每中间值写一次**（探针 f16 实录
/// FF,00,01,02,03 序）；Multi 特例随后 Add8：`$065E+=8`（再写一次）。
/// 返回（最终索引, 写事件集）。
pub fn find_music_init_index_writes(current_sound_flags: u8, add8: bool) -> (u8, Vec<(u16, u8)>) {
    let mut ev = vec![(0x065E, 0xFF)];
    let mut idx: u8 = 0xFF;
    if current_sound_flags == 0 {
        return (0xFF, ev); // beq @RTS（MusicInitIndex 保持 $FF）
    }
    let mut v = current_sound_flags;
    loop {
        idx = idx.wrapping_add(1); // inc MusicInitIndex
        let carry = v & 0x80 != 0; // asl（位7→carry）
        v = v.wrapping_shl(1);
        ev.push((0x065E, idx));
        if carry {
            break; // bcc @loop 出
        }
    }
    if add8 {
        idx = idx.wrapping_add(8); // MusicInitIndexAdd8（$BC64：clc/adc #8）
        ev.push((0x065E, idx));
    }
    (idx, ev)
}

/// 6 init 旗清（ClearInitFlags $B3D6 语义面——Noise/SQ1/SQ2/Tri/Multi/Music）。
pub const INIT_FLAG_COUNT: usize = 6;

#[cfg(test)]
mod tests {
    use super::*;

    /// 位扫描 MSB 优先 **8 位**（r17 ROM 终审：`cmp #$10`+`iny iny`=位7..0 全扫——
    /// prg[0x34BD+0x1B..0x2E] `a0 00 ad 4d 06 48 0e 4d 06 b0 0c c8 c8 98 c9 10 d0 f4`）。
    #[test]
    fn scan_is_msb_first_eight_bits() {
        assert_eq!(scan_sound_flags(0x80).bit_number, Some(7));
        assert_eq!(scan_sound_flags(0x08).bit_number, Some(3));
        assert_eq!(scan_sound_flags(0x04).bit_number, Some(2));
        assert_eq!(scan_sound_flags(0x01).bit_number, Some(0)); // 位0 也扫（旧 6 位注证伪）
        assert_eq!(scan_sound_flags(0x02).bit_number, Some(1));
        assert_eq!(scan_sound_flags(0x00).bit_number, None);
        assert_eq!(scan_sound_flags(0x44).bit_number, Some(6)); // 双旗取高
    }

    /// 表目偏移=2×(7-位号)（y=0 起——**无 +4**：4B 双字是拷贝进 $E0-$E3 不占表位；
    /// 旧「表前 4B/扫描自 y=4」注=漏看 `ldy #$00` 复位，r17 证伪更正）。
    #[test]
    fn table_offset_from_zero() {
        assert_eq!(scan_sound_flags(0x80).table_offset, 0);
        assert_eq!(scan_sound_flags(0x40).table_offset, 2);
        assert_eq!(scan_sound_flags(0x01).table_offset, 14);
    }

    /// handler 解析（无旗⇒默认例程续下一 cycle；命中⇒表[y]覆盖 $E2/$E3——
    /// 探针 f16 双写实证：4B 拷先落默认 B35C 再被表目 BC7A 覆盖）。
    #[test]
    fn routine_resolution_default_and_hit() {
        let b = SoundDataBlock { table_ptr: 0x8000, default_routine: 0xB322, channel_type: 3 };
        let tbl = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA];
        assert_eq!(get_sound_routine(&b, &tbl, 0x00), (0xB322, None));
        // 位7 命中→表目 y=0：0x2211
        assert_eq!(get_sound_routine(&b, &tbl, 0x80), (0x2211, Some(0)));
        // 位2 命中→表目 y=10（tbl 短表外⇒0）
        assert_eq!(get_sound_routine(&b, &tbl, 0x04).1, Some(10));
    }

    /// GetSoundRoutine 写事件面（$064D 旗副本/4B 拷含默认/覆盖序——F1 公共根；
    /// $E4=块窗址 lo（r20 修））。
    #[test]
    fn get_sound_routine_write_face() {
        let b = SoundDataBlock { table_ptr: 0xBC06, default_routine: 0xB35C, channel_type: 2 };
        let tbl = [0x7A, 0xBC];
        let (ptr, hit, ev) = get_sound_routine_writes(0xB2A7, &b, &tbl, 0x10);
        assert!(ev.contains(&(0x064D, 0x10)), "sta CurrentSoundFlags 旗副本");
        assert!(ev.contains(&(0x00E4, 0xA7)), "stx $E4=块窗址 lo（$B2A7→$A7）");
        assert!(ev.contains(&(0x00E2, 0x5C)), "默认先入 $E2");
        assert_eq!(hit, Some(6)); // 位4→y=6（短表外 0）
        assert_eq!(ptr, 0);
        // 位7 命中（y=0→BC7A）：覆盖序=事件尾两条
        let (ptr2, _, ev2) = get_sound_routine_writes(0xB2A7, &b, &tbl, 0x80);
        assert_eq!(ptr2, 0xBC7A);
        assert_eq!(&ev2[ev2.len() - 2..], &[(0x00E2, 0x7A), (0x00E3, 0xBC)][..]);
    }

    /// FindMusicInitIndex+Add8 写序（FF→逐位 inc→Add8——探针 f16 实录序钉死；
    /// f16 旗=$064D=0x10 位4：4 轮 inc→0x03→Add8→0x0B Intro）。
    #[test]
    fn find_music_init_index_write_sequence() {
        let (idx, ev) = find_music_init_index_writes(0x10, true);
        assert_eq!(idx, 0x0B); // 位4→3，Add8→0x0B（Intro——探针 f16 终值）
        assert_eq!(ev, vec![(0x065E, 0xFF), (0x065E, 0x00), (0x065E, 0x01),
                            (0x065E, 0x02), (0x065E, 0x03), (0x065E, 0x0B)]);
        let (idx2, ev2) = find_music_init_index_writes(0x00, false);
        assert_eq!(idx2, 0xFF, "无旗⇒$FF 出（beq @RTS）");
        assert_eq!(ev2, vec![(0x065E, 0xFF)]);
    }
}

// ---- task-30 handler 细行首件：B673 SamusHitSFXInit（SFX 处理器族起点）----
// asm 主文档：sound_engine.asm `SamusHitSFXInit: ; $B673`（镜面 sig 三银行自证）。
// 结构互核注：①LoadSFXData=4 字节**间接写 APU 寄存器**（`(SFXDataPtr),y → (APURegisterPtr),y`）
// ②zp $E0-$E5 **双语义并用**（表指针族↔APU/Data 指针族——时分复用面）③RNG 消费
// （RandomNumber1 $2E——task-20 域面）④UpdateContFlags `and #$00` 死操作（asm 注
// 「was this value non-zero at some point」——字节真照建）。

/// SFXData@SamusHitSQ1SQ2（bank 窗 $B225 4B——`c1 89 02 0f`）。
/// SFX 4B 记录取件（ROM `$B211..$B289` 窗内绝对址；`InitSFXData` 的 src 参数
/// 就是 ROM 址，故这里以地址而非数组索引定位——地址即 asm 锚）。
fn sfx4(addr: u16) -> &'static [u8; 4] {
    game_data::assets::sfx_record4(addr)
        .unwrap_or_else(|| panic!("SFX 记录址 ${addr:04X} 不在 $B20D..$B289 窗内"))
}

/// `LoadSFXData@SQ1 ; $B368`（**asm 内容锚**）：`APURegisterPtr←$4000`、
/// `SFXDataPtr←$B2:Y`，随后 4 字节**间接**写 `$4000..$4003`。
/// 调用形（asm `$B3AF`）：`ldy #<SFXData@Xxx` 只传低字节——高字节恒 `>SFXData($B2)`。
pub(crate) fn load_sfx_data_sq1(mem: &mut crate::frame::Manager, low: u8) {
    mem.write_ram(0x00E0, 0x00); // APURegisterPtr ← $4000（<SQ1_VOL / >SQ1_VOL）
    mem.write_ram(0x00E1, 0x40);
    mem.write_ram(0x00E2, low); // SFXDataPtr ← $B2:low
    mem.write_ram(0x00E3, 0xB2);
    let rec = sfx4(0xB200 | low as u16);
    for i in 0..4u16 {
        mem.write_io(0x4000 + i, rec[i as usize]);
    }
}

/// SamusHit 音效 4B 记录（ROM `$B225`——`InitSFXData` 的 src 址，窗内寻址）。
fn samus_hit_sfx_data() -> &'static [u8; 4] {
    crate::sound::sfx4(0xB225)
}

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
        apu.push((0x4000 + i, samus_hit_sfx_data()[i as usize]));
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
        apu.push((0x4004 + i, samus_hit_sfx_data()[i as usize]));
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
            apu.push((base + i, samus_hit_sfx_data()[i as usize]));
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

/// MusicInitIndexAdd8（$BC64）：+8（Multi 表偏移面）。
pub fn music_init_index_add8(index: u8) -> u8 {
    index.wrapping_add(8)
}

/// 【r18 U4 归并】原 `find_music_init_index(flags)->u8`（本文件）已删——正典=
/// `sound::music::find_music_init_index`（Option 形——穷举参照测在册）；
/// 写序面=`find_music_init_index_writes`（FF→逐位inc→Add8 全序）。

/// SFXData@Beep（bank 窗 $B279 4B——`09 7f 30 48`）。
/// 蜂鸣音效 4B 记录（ROM `$B279`）。
fn beep_sfx_data() -> &'static [u8; 4] {
    crate::sound::sfx4(0xB279)
}

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
        apu.push((0x4008 + i, beep_sfx_data()[i as usize]));
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
    /// 【r18 U4】改指正典 sound::music::find_music_init_index（Option 形）——断言保持。
    #[test]
    fn music_index_scan_msb_first() {
        let idx = |f: u8| crate::sound::music::find_music_init_index(f).unwrap_or(0xFF);
        assert_eq!(idx(0), 0xFF);
        assert_eq!(idx(0x80), 0);
        assert_eq!(idx(0x08), 4);
        assert_eq!(idx(0x08).wrapping_add(8), 0x0C); // Add8 面
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

}

// ---- task-44 W2 高频 handler 族六件（共核 InitSFXData 面+薄壳守卫形）----

/// W2 数据表 6×4B（bank 窗面——0x3xxx 偏移 hexdump 实证）。

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
    sfx_data_init(3, sfx4(0xB21D), 0xB21D, ct, flags)
}

/// EnemyHitSFXInit（$B710）：无守卫+8 帧。
pub fn enemy_hit_sfx_init(ct: u8, flags: u8) -> HandlerOutcome {
    sfx_data_init(8, sfx4(0xB255), 0xB255, ct, flags)
}

/// BulletFireSFXInit（$B716）——**task-SFX-1 重写退役**：旧 Option-None 形不符 asm
/// `jmp HasIceBeamSFXInit/$B75E`、`jmp HasLongBeamSFXInit/$B72F` 的 jmp 语义
/// （变体是内部跳转目标，非调用方分支）；本件已按 asm 分支序（①lsr/bcs ice
/// ②0xCC 守卫 ③asl/bcs long ④短光束 3 帧）重写为 HandlerOutcome（见下方
/// bullet_fire_sfx_init 全文实现，单一实现面铁律）。

/// MissileLaunchSFXInit（$B56E）：0x18 帧+**特例**（SFXNoiseVar0 $0670←$0A 噪声音高索引）。
pub fn missile_launch_sfx_init(ct: u8, flags: u8) -> HandlerOutcome {
    let mut o = sfx_data_init(0x18, sfx4(0xB215), 0xB215, ct, flags);
    o.ram_events.push((0x0670, 0x0A)); // 特例（此写在清表后——终值 0x0A）
    o.a_out = 0x0A; // 末 A-setter=`lda #$0A`
    o
}

/// MissilePickupSFXInit（$B6CD）：无守卫+5 帧。
pub fn missile_pickup_sfx_init(ct: u8, flags: u8) -> HandlerOutcome {
    sfx_data_init(5, sfx4(0xB241), 0xB241, ct, flags)
}

/// EnergyPickupSFXInit（$B6E7）：无守卫+6 帧。
pub fn energy_pickup_sfx_init(ct: u8, flags: u8) -> HandlerOutcome {
    sfx_data_init(6, sfx4(0xB23D), 0xB23D, ct, flags)
}

#[cfg(test)]
mod w2_tests {
    use super::*;

    /// 共核面（frames/指针/APU 4B/cont/占用）。
    #[test]
    fn sfx_data_init_common_face() {
        let o = sfx_data_init(3, sfx4(0xB21D), 0xB21D, 3, 0x40);
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
        // BulletFire 短光束/守卫/变体分支（task-SFX-1：jmp 变体形——ice/long 返各自
        // init 事件面，长度 7（$B75E/$B72F 均 7 帧）；守卫=空事件；短光束=3 帧）
        assert!(bullet_fire_sfx_init(0x80, 0, 0, 0x40).ram_events.iter().any(|a| *a == (0x0660, 7))); // long beam 变体（$B72F）
        assert!(bullet_fire_sfx_init(0x01, 0, 0, 0x40).ram_events.iter().any(|a| *a == (0x0660, 7))); // ice 变体（$B75E）
        let o = bullet_fire_sfx_init(0x00, 0xCC, 0, 0x40);
        assert!(o.ram_events.is_empty()); // SQ1Cont 守卫
        assert!(bullet_fire_sfx_init(0x00, 0x00, 0, 0x40).ram_events.iter().any(|a| *a == (0x0660, 3)));
    }
}

// ---- task-44 W3 中频 handler 族六件（一体）----

/// W3 数据表 8×4B（0x3xxx 偏移 hexdump 实证）。

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
    sfx_data_init(0x14, sfx4(0xB221), 0xB221, ct, flags)
}

/// ScrewAttackSFXInit（$B52B）：5 帧+**特例**（SFXNoiseVar0 $0670←data[2]=0x00 清索引）。
pub fn screw_attack_sfx_init(ct: u8, flags: u8) -> HandlerOutcome {
    let mut o = sfx_data_init(5, sfx4(0xB211), 0xB211, ct, flags);
    o.ram_events.push((0x0670, sfx4(0xB211)[2])); // data[2] 清索引（注「#$00」）
    o.a_out = sfx4(0xB211)[2]; // 末 A-setter=`lda data+2`
    o
}

/// BombExplodeSFXInit（$B583）：无守卫 0x30 帧。
pub fn bomb_explode_sfx_init(ct: u8, flags: u8) -> HandlerOutcome {
    sfx_data_init(0x30, sfx4(0xB219), 0xB219, ct, flags)
}

/// BossHitSFXInit（$B5EC）：双通道族（SQ1 装载+MultiSFXInit——A=SQ1 数据[3] 末字节）。
pub fn boss_hit_sfx_init(ct: u8, flags: u8) -> HandlerOutcome {
    let o = sfx_data_init(0, sfx4(0xB22D), 0xB22D, ct, flags); // SQ1 装载面（无 length 写——LoadSFXData 非 InitSFXData）
    // 修正形：LoadSFXData@SQ1=指针+4B（无 length/清表）——此处重写
    let mut ram = vec![
        (0x00E0, 0x00), (0x00E1, 0x40), (0x00E2, 0x2D), (0x00E3, 0xB2),
    ];
    let mut apu = Vec::new();
    for i in 0..4u16 {
        apu.push((0x4000 + i, sfx4(0xB22D)[i as usize]));
    }
    let frames = 4u8; // **LoadSFXData 尾 A=环计数**（`tya/cmp #$04` 出口 A=4——非 data[3]！
                      // golden≠ROM 打回正）
    let (ram2, apu2) = multi_sfx_init_face(frames, sfx4(0xB229), ct, flags);
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
        apu.push((0x4000 + i, sfx4(0xB231)[i as usize]));
    }
    let (ram2, apu2) = multi_sfx_init_face(0x20, sfx4(0xB235), ct, flags);
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
    sfx_data_init(0x0C, sfx4(0xB251), 0xB251, ct, flags)
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
/// （SongHeaderEndInitIndex→NoteLengthTblOffset($062B) 起 13B——曲头行面）+
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
/// 【r20 asm 终审修正】SQ2 面坐标=**$066B**（MusicSQ2VolumeIndex——旧注「$066A+2 块形」
/// 证伪：constants_ram:1099 逐字+`sta MusicSQ2VolumeIndex` 绝对址形（无 ,x））。
pub fn reset_volume_index(sq1_delay: u8, sq2_delay: u8) -> Vec<(u16, u8)> {
    let mut ram = Vec::new();
    if sq1_delay == 1 {
        ram.push((0x066A, 1)); // MusicSQ1VolumeIndex
    }
    if sq2_delay == 1 {
        ram.push((0x066B, 1)); // MusicSQ2VolumeIndex（$066B——r20 终审）
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
        // 曲头 13B 形（5B 宏 + 4 通道字）：手写 fixture，非 ROM 表行（表行见 assets::song_headers）。
        let header: [u8; 13] = [0x07, 0x00, 0xF0, 0x02, 0x05, 0x00, 0x01, 0x00, 0x03, 0x00, 0x05, 0x00, 0x07];
        let ram = initialize_music(0x40, &header);
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
    // 真实取数：`ldy MusicSQ1VolumeIndex,x / lda (VolumeEnvelopePtr),y`
    // （sound_engine.asm:1676-1680）——`env_idx-1` 选表，`volume_index` 选步。
    // 旧实现取 `env_bytes` 末字节当音量（占位），会让 $4000 的 duty/音量组合失真。
    let _ = env_bytes; // 参数保留：包络体单源在 game_data（assets::volume_envelope）
    let env = game_data::assets::volume_envelope(env_idx.wrapping_sub(1) as usize);
    let vol_byte = env.get(volume_index as usize).copied().unwrap_or(0);
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
        // 常规路（duty 高 4 位 + **真实包络字节**）
        // 音量字节来自 `VolumeEnvelopePtrTable[env_idx-1]` 表体的 `volume_index` 位：
        // env_idx=3 ⇒ 第 3 张表（ROM 实读首字节 `00 0D 09 07 …`），index=1 ⇒ `$0D`
        // ⇒ `$90 | $0D = $9D`。旧期望 `$9C` 是「取 env_bytes 末字节」占位实现的残留。
        let (ram, apu, d) = update_volume_envelope(0, 3, 0x30, 0x90, 1, 0, &[0x11, 0x22, 0x0C]);
        assert_eq!(d, Some(VolEnvByte::Normal(0x9D)));
        assert!(ram.contains(&(0x066A, 2)));
        assert!(apu.contains(&(0x4000, 0x9D)));
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

/// Tri 周期面共形（$0610-$0613 块）：PeriodLow/High+ChangeLow/High（滑音 delta）。
fn tri_period_face(period_lo: u8, period_hi: u8, change_lo: u8, change_hi: u8)
    -> Vec<(u16, u8)> {
    vec![(0x0610, period_lo), (0x0611, period_hi),
         (0x0612, change_lo), (0x0613, change_hi)]
}

/// DoorOpenCloseSFXInit（$B7AC）：周期面（data[2]/data[3]&7）+滑音 $000F+0x1F 帧。
pub fn door_open_close_sfx_init(ct: u8, flags: u8) -> HandlerOutcome {
    let mut o = sfx_data_init(0x1F, sfx4(0xB285), 0xB285, ct, flags);
    o.ram_events.extend(tri_period_face(0x30, 0xB2 & 7, 0x0F, 0x00));
    o
}

/// BigEnemyHitSFXInit（$B7EF）：滑音 $0012+周期面+0x0A 帧。
pub fn big_enemy_hit_sfx_init(ct: u8, flags: u8) -> HandlerOutcome {
    let mut o = sfx_data_init(0x0A, sfx4(0xB27D), 0xB27D, ct, flags);
    o.ram_events.extend(tri_period_face(0x42, 0x18 & 7, 0x12, 0x00));
    o
}

/// SamusToBallSFXInit（$B834）：8 帧+**Divisor←5**（1/5 分频）+周期面。
pub fn samus_to_ball_sfx_init(ct: u8, flags: u8) -> HandlerOutcome {
    let mut o = sfx_data_init(8, sfx4(0xB26D), 0xB26D, ct, flags);
    o.ram_events.push((0x0616, 5)); // SFXTriPeriodDivisor
    // 注：SamusToBall **无滑音面写**（asm 只设 Period 两项——golden≠core 打回正）
    o.ram_events.push((0x0610, 0xDD));
    o.ram_events.push((0x0611, 0x3B & 7));
    o.a_out = 0x3B & 7; // 末 setter=PeriodHigh（and #$07 后）
    o
}

/// BombLaunchSFXInit（$B878）：4 帧纯装载（无周期面）。
pub fn bomb_launch_sfx_init(ct: u8, flags: u8) -> HandlerOutcome {
    sfx_data_init(4, sfx4(0xB265), 0xB265, ct, flags)
}

/// MetroidHitSFXInit（$B8A7）：3 帧+**RndTriPeriods RNG 面**（TRI_LO←rng|$6C/
/// TRI_HI←(rng&1)|$F8——APU 随机周期）。
pub fn metroid_hit_sfx_init(rng1: u8, ct: u8, flags: u8) -> HandlerOutcome {
    let mut o = sfx_data_init(3, sfx4(0xB271), 0xB271, ct, flags);
    // asm RndTriPeriods ; $B8C3（sound_engine.asm:1402-1410）：
    //   ora #$6C / sta TRI_LO($400A)；and #$01 / ora #$F8 / sta TRI_HI($400B)
    // 【复核 F1 更正】旧形写 $4008/$400A 整体错位一格（hardware.asm:69 TRI_LO=$400A）。
    o.apu_events.push((0x400A, rng1 | 0x6C));
    o.apu_events.push((0x400B, (rng1 & 1) | 0xF8));
    o.a_out = (rng1 & 1) | 0xF8;
    o
}

/// SamusDieSFXInit（$B8D2）：**InitializeSoundAddresses 全清前置**（残面注记——
/// 清面多址组合随件）+0x0E 帧+Divisor←$15+周期面（PeriodHigh←0）。
pub fn samus_die_sfx_init(ct: u8, flags: u8) -> HandlerOutcome {
    let mut o = sfx_data_init(0x0E, sfx4(0xB275), 0xB275, ct, flags);
    o.ram_events.push((0x0616, 0x15));
    o.ram_events.extend(tri_period_face(0x40, 0, 0, 0));
    o
}

/// StatueRaiseSFXInit（$B921）：周期面+滑音 $000B+6 帧。
pub fn statue_raise_sfx_init(ct: u8, flags: u8) -> HandlerOutcome {
    let mut o = sfx_data_init(6, sfx4(0xB281), 0xB281, ct, flags);
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
        assert!(o.apu_events.contains(&(0x400A, 0x55 | 0x6C))); // 复核 F1：TRI_LO=$400A
        assert!(o.apu_events.contains(&(0x400B, 0x55 & 1 | 0xF8))); // 复核 F1：TRI_HI=$400B
        assert!(samus_die_sfx_init(2, 0x40).ram_events.contains(&(0x0616, 0x15)));
        assert!(statue_raise_sfx_init(2, 0x40).ram_events.contains(&(0x0613, 0)));
    }
}

// ---- task-44 W4 曲族：MusicNNInit 双形 + 曲头烘焙 ----

/// SongHeaderOffsetTable（$BBFA 12B——1 字节偏移×MusicInitIndex 索引）。

/// SongHeaders（$BD31 156B=12 曲头×13B——宏 5B+4 通道字；0x3D31 hexdump 实证）。

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
    let mut e2 = crate::sound::initialize_music(0, &game_data::assets::song_headers()[0..13].try_into().unwrap());
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
        assert_eq!(game_data::assets::song_headers().len(), 156);
        assert_eq!(game_data::assets::song_header_offsets().len(), 12);
        assert_eq!(&game_data::assets::song_headers()[0..5], &[0x0B, 0xFF, 0xF5, 0x00, 0x00]);
    }
}

// ---- task-44 W4 收官：InitializeSoundAddresses 全清面 + GotoMusic 6 桩 ----

/// InitializeSoundAddresses（$B404）语义折叠——三清子件组合：
/// ①ClearMusicAndSFXAddresses（$B41D）：占用 3+LoadPeriodsFlag+cont 5+MusicCont=**10 写**
/// ②ClearSounds（$B43E）：SQ1/SQ2/NOISE_VOL←$10+TRI_LINEAR/DMC_RAW←0=**5 APU 写**
/// ③ClearSpecialAddresses（$B40E）：TriCounterCntrl/SFXPaused/MusicLoopFlags/
/// MusicLoopsOnEnd=**4 写**。
/// 返回 (RAM 事件, APU 事件)。
pub fn initialize_sound_addresses() -> (Vec<(u16, u8)>, Vec<(u16, u8)>) {
    let mut ram = Vec::new();
    // ① 音乐/SFX 地址清
    for a in [0x0653u16, 0x0654, 0x0655, 0x0607,
              0x0688, 0x0689, 0x068A, 0x068B, 0x068C, 0x068D] {
        ram.push((a, 0));
    }
    // ③ 特殊地址清
    for a in [0x062Du16, 0x0602, 0x065D, 0x062C] {
        ram.push((a, 0));
    }
    // ② 声道寄存器清（APU 面）
    let apu = vec![
        (0x4000, 0x10), (0x4004, 0x10), (0x400C, 0x10),
        (0x4008, 0x00), (0x4011, 0x00),
    ];
    (ram, apu)
}

/// GotoMusic00-05Init 跳板 6 桩（$BC77-$BC86——`jmp MusicNNInit` 语义=L2 面直调目标）。
/// 返回目标址（L2 案按 jumped 面）。
pub fn goto_music_init(index: u8) -> Option<u16> {
    match index {
        0 => Some(0xBCAA), // Music00Init
        1 => Some(0xBCA4), // Music01Init
        2 => Some(0xBC9A), // Music02Init
        3 => Some(0xBC96), // Music03Init
        4 => Some(0xBC89), // Music04Init
        5 => Some(0xBC9E), // Music05Init
        _ => None,
    }
}

#[cfg(test)]
mod w4_final_tests {
    use super::*;

    /// 全清面（14 RAM+5 APU）。
    #[test]
    fn clear_all_sound_addresses() {
        let (ram, apu) = initialize_sound_addresses();
        assert_eq!(ram.len(), 14);
        assert_eq!(apu.len(), 5);
        assert!(ram.contains(&(0x0653, 0)));
        assert!(ram.contains(&(0x068D, 0)));
        assert!(ram.contains(&(0x062C, 0)));
        assert!(apu.contains(&(0x4000, 0x10)));
        assert!(apu.contains(&(0x4011, 0)));
    }

    /// GotoMusic 6 桩（jmp 目标形）。
    #[test]
    fn goto_music_stubs() {
        assert_eq!(goto_music_init(0), Some(0xBCAA));
        assert_eq!(goto_music_init(5), Some(0xBC9E));
        assert_eq!(goto_music_init(6), None);
    }
}
// ---- task-SFX-1：SFX cont 公共底座（sound_engine.asm 逐行读第二遍，2026-11-20）----
//
// asm 锚面（bank0 镜面，`refs/m1disasm/SRC/sound_engine.asm`）：
// - `IncrementSFXFrame ; $B4A9`：`inc SFXNoiseFrame,x / cmp SFXNoiseLength,x / bne @RTS /
//   lda #0 / sta`——帧+1；==length⇒复位 0 且 **A=0**（调用方 `bne`=未终）。
// - `UpdateContFlags ; $B493`：`SFXNoiseContFlags[ChannelType] ← CurrentSoundFlags`
//   （`and #$00` 死操作照建——结果恒等于 CurrentSoundFlags）。
// - `ClearCurrentSoundFlagsAndContFlags ; $B4A2`：`$064D←0 → UpdateContFlags（写 0）`。
// - `EndNoiseSFX ; $B58F` / `EndSQ1SFX ; $B6F2` / `EndTriSFX ; $B896` /
//   `EndMultiSFX ; $B5CD`：四终面（端口序逐 asm）。
// - `IncreaseSFXTriPeriod ; $B978` / `DecreaseSFXTriPeriod ; $B98C` /
//   `DivideSFXTriPeriod ; $B9A0` / `WriteSFXTriPeriod ; $B869`：Tri 周期算术族。
// - `IncrementNoisePeriod ; $B564` / `WriteSQ1SQ2PeriodLow ; $B62C` /
//   `LoadSQ1PeriodLow ; $B7A0`：周期写底座。
// - `LoadSFXData@Common ; $B376`：指针面+4B 间接写（init/cont 双向共用）。
//
// APU 寄存器址（hardware.asm A 级）：SQ1 $4000-3 / SQ2 $4004-7 / Tri $4008-9,A,B /
// Noise $400C-F（NOISE_LO=$400E / **TRI_HI=$400B**）。

/// `IncrementSFXFrame ; $B4A9`：帧+1（RAM 事件）；==length⇒复位 0 且终帧。
/// 返回（帧事件集, 是否终帧）。写序：`inc`（frame+1）→ 终帧再 `sta #0`。
pub fn increment_sfx_frame(ct: u8, frame: u8, length: u8) -> (Vec<(u16, u8)>, bool) {
    let fa = 0x0665u16 + ct as u16;
    let next = frame.wrapping_add(1);
    let mut ev = vec![(fa, next)];
    if next == length {
        ev.push((fa, 0));
        return (ev, true);
    }
    (ev, false)
}

/// `UpdateContFlags ; $B493` 写面：`SFXNoiseContFlags+ct ← CurrentSoundFlags`
/// （`and #$00` 死操作照建）。
pub fn update_cont_flags(ct: u8, current_flags: u8) -> (u16, u8) {
    (0x0688u16 + ct as u16, current_flags)
}

/// `ClearCurrentSoundFlagsAndContFlags ; $B4A2`：`$064D←0` → UpdateContFlags（写 0）。
pub fn clear_current_sound_flags_and_cont_flags(ct: u8) -> Vec<(u16, u8)> {
    vec![(0x064D, 0), (0x0688 + ct as u16, 0)]
}

/// `EndNoiseSFX ; $B58F`：ClearFlags → `NOISE_VOL←$10`（asm 序：先旗后寄）。
pub fn end_noise_sfx(ct: u8) -> (Vec<(u16, u8)>, Vec<(u16, u8)>) {
    let ram = clear_current_sound_flags_and_cont_flags(ct);
    (ram, vec![(0x400C, 0x10)])
}

/// `EndSQ1SFX ; $B6F2`：`SQ1_VOL←$10` → `SQ1UsedBySFX←0` → ClearFlags →
/// `inc LoadMusicSQ1SQ2PeriodsFlag`（asm 序）。
pub fn end_sq1_sfx(ct: u8, music_flag: u8) -> (Vec<(u16, u8)>, Vec<(u16, u8)>) {
    let mut ram = vec![(0x0653, 0)];
    ram.extend(clear_current_sound_flags_and_cont_flags(ct));
    // asm `inc LoadMusicSQ1SQ2PeriodsFlag`【复核 F6】旧形恒写 1（连续 End 未消费时真值 2）
    ram.push((0x0607, music_flag.wrapping_add(1)));
    (ram, vec![(0x4000, 0x10)])
}

/// `EndTriSFX ; $B896`：`TRI_LINEAR←0` → `TriUsedBySFX←0` → `TRI_HI($400B)←$18` →
/// ClearFlags（asm 序：APU/占用交错面）。
pub fn end_tri_sfx(ct: u8) -> (Vec<(u16, u8)>, Vec<(u16, u8)>) {
    let ram = vec![(0x0655, 0)];
    let mut ram2 = ram;
    ram2.extend(clear_current_sound_flags_and_cont_flags(ct));
    (ram2, vec![(0x4008, 0x00), (0x400B, 0x18)])
}

/// `EndMultiSFX ; $B5CD`：双 VOL 静音+双 SWEEP 禁（APU 面）→ ClearFlags →
/// SQ1/SQ2 占用清 → `inc LoadMusicSQ1SQ2PeriodsFlag`。
pub fn end_multi_sfx(ct: u8, music_flag: u8) -> (Vec<(u16, u8)>, Vec<(u16, u8)>) {
    let apu = vec![(0x4000, 0x10), (0x4004, 0x10), (0x4001, 0x7F), (0x4005, 0x7F)];
    let mut ram = clear_current_sound_flags_and_cont_flags(ct);
    ram.push((0x0653, 0));
    ram.push((0x0654, 0));
    ram.push((0x0607, music_flag.wrapping_add(1))); // inc【复核 F6】
    (ram, apu)
}

/// `IncreaseSFXTriPeriod ; $B978`：16bit `Period + Change`（adc 链）。
/// 返回（新 Low, 新 High, RAM 事件）。
pub fn increase_sfx_tri_period(plo: u8, phi: u8, clo: u8, chi: u8)
    -> (u8, u8, Vec<(u16, u8)>) {
    let lo = plo.wrapping_add(clo);
    let hi = phi.wrapping_add(chi).wrapping_add(if lo < plo { 1 } else { 0 });
    (lo, hi, vec![(0x0610, lo), (0x0611, hi)])
}

/// `DecreaseSFXTriPeriod ; $B98C`：16bit `Period - Change`（sbc 链）。
pub fn decrease_sfx_tri_period(plo: u8, phi: u8, clo: u8, chi: u8)
    -> (u8, u8, Vec<(u16, u8)>) {
    let lo = plo.wrapping_sub(clo);
    let hi = phi.wrapping_sub(chi).wrapping_sub(if plo < clo { 1 } else { 0 });
    (lo, hi, vec![(0x0610, lo), (0x0611, hi)])
}

/// `DivideSFXTriPeriod ; $B9A0`：16bit 移位减除（restoring division，除数=SFXTriPeriodDivisor）。
/// 【进位入注】入口 carry 实跑恒 0（三条调用路径都从 `IncrementSFXFrame` 未终帧
/// `cmp` 出——frame'<length⇒carry=0），初轮 `rol Period` 移入 0。
/// 返回（DividedLow, DividedHigh, Remainder）。Remainder=$0617 终值
/// 【复核 F3 补】asm :1557 `sta #$00`+每轮 `rol/sbc/sta`，残值留在 RAM。
pub fn divide_sfx_tri_period(divisor: u8, plo: u8, phi: u8) -> (u8, u8, u8) {
    let mut p = ((phi as u16) << 8) | plo as u16;
    let mut rem: u8 = 0;
    // 初轮 `rol SFXTriPeriodLow / rol SFXTriPeriodHigh`：C(入口)→p b0、p b15→C。
    // 【进位入注】入口 carry 实跑恒 0（三条调用路径都从 IncrementSFXFrame 未终帧
    // cmp 出——frame'<length⇒carry=0）。
    let mut c = ((p >> 15) & 1) as u8;
    p = (p << 1) & 0xFFFF;
    for _ in 0..16u8 {
        // `rol SFXTriPeriodRemainder`：C（**上轮 rol High 的 p b15 出位**）→rem b0
        rem = (rem << 1) | (c & 1);
        // `cmp Divisor / bcc @endIf_A / sbc Divisor`：rem>=div ⇒ 减且 C=1（sbc 无借位）
        c = if rem >= divisor {
            rem = rem.wrapping_sub(divisor);
            1
        } else {
            0
        };
        // `rol PeriodLow / rol PeriodHigh`：C(商位)→p b0；p b15 出位→下轮 rol Remainder
        let out = ((p >> 15) & 1) as u8;
        p = ((p << 1) | (c as u16)) & 0xFFFF;
        c = out;
    }
    ((p & 0xFF) as u8, (p >> 8) as u8, rem)
}

/// `WriteSFXTriPeriod ; $B869`：`TRI_LO←PeriodLow / TRI_HI($400B)←PeriodHigh|0x08`。
pub fn write_sfx_tri_period(plo: u8, phi: u8) -> Vec<(u16, u8)> {
    vec![(0x400A, plo), (0x400B, phi | 0x08)]
}

/// `IncrementNoisePeriod ; $B564`：`Var0+1 → RAM → NOISE_LO($400E)`。
/// va=SFXNoiseVar0 地址（ct 面 `0x0670+ct`——Noise 族 ct=0）。
pub fn increment_noise_period(va: u16, var0: u8) -> (Vec<(u16, u8)>, Vec<(u16, u8)>) {
    let v = var0.wrapping_add(1);
    (vec![(va, v)], vec![(0x400E, v)])
}

/// `WriteSQ1SQ2PeriodLow ; $B62C`：`SQ2_LO←Var1 / SQ1_LO←Var2`。
pub fn write_sq1_sq2_period_low(var1: u8, var2: u8) -> Vec<(u16, u8)> {
    vec![(0x4006, var1), (0x4002, var2)]
}

/// `LoadSFXData@Common ; $B376` 事件面：指针面（APURegisterPtr/SFXDataPtr zp 4 写）+
/// 4 字节间接 APU 写（`base` 寄存器基址、`addr` ROM 数据址——窗内寻址）。
pub(crate) fn load_sfx_data_events(base: u16, addr: u16)
    -> (Vec<(u16, u8)>, Vec<(u16, u8)>) {
    let rec = sfx4(addr);
    let mut ram = vec![
        (0x00E0, (base & 0xFF) as u8),
        (0x00E1, (base >> 8) as u8),
        (0x00E2, (addr & 0xFF) as u8),
        (0x00E3, (addr >> 8) as u8),
    ];
    let mut apu = Vec::new();
    for i in 0..4u16 {
        apu.push((base + i, rec[i as usize]));
    }
    (ram, apu)
}

/// 声引擎区表切片（`sound_region()` 窗内 `addr` 起取 `len` 字节——表真长逐 asm）。
fn region_tbl(addr: u16, len: usize) -> &'static [u8] {
    let o = (addr - game_data::assets::SOUND_REGION_ADDR) as usize;
    &game_data::assets::sound_region()[o..o + len]
}

/// `SpitFlamesTbl ; $B4FB`（20B 包络——SpitFlameSFXCont 用）。
fn spit_flames_tbl() -> &'static [u8] { region_tbl(0xB4FB, 20) }

/// `BossHitSFXDataTbl ; $B63C`（36B——BossHitSFXCont VOL 面）。
fn boss_hit_sfx_data_tbl() -> &'static [u8] { region_tbl(0xB63C, 36) }

/// `MissilePickupSFXTbl ; $B6A9`（7B，末 `$00` 终止）。
fn missile_pickup_sfx_tbl() -> &'static [u8] { region_tbl(0xB6A9, 7) }

/// `@IceBeamTbl ; $B778`（2B：`$93/$81`）。
fn ice_beam_tbl() -> &'static [u8] { region_tbl(0xB778, 2) }

/// `WaveBeamSFXPeriodLowTbl ; $B7A7`（2B：`$58/$6F`）。
fn wave_beam_period_low_tbl() -> &'static [u8] { region_tbl(0xB7A7, 2) }

/// `WaveBeamSFXVolumeTbl ; $B7A9`（3B：`$93/$91/$00`）。
fn wave_beam_volume_tbl() -> &'static [u8] { region_tbl(0xB7A9, 3) }

// ---- task-SFX-1：SQ1 init 变体件（表内目+B716 内部分支目标）----
// asm 锚：`HasLongBeamSFXInit ; $B72F`（7 帧 LongRangeShot）/`MetalSFXInit ; $B735`
// （0x0B 帧）/`OutOfPipeSFXInit ; $B73C`（escape⇒TimeBombTick 7 帧否则 OutOfPipe 0x16 帧）/
// `HasIceBeamSFXInit ; $B75E`（7 帧 IceBeam）/`WaveBeamSFXInit ; $B77A`（8 帧）。
// 注：HasIce/HasLong 是 BulletFireSFXInit 内部 `jmp` 目标（不出现在表里——asm jmp 形）。

/// `HasLongBeamSFXInit ; $B72F`：7 帧 LongRangeShot。
pub fn has_long_beam_sfx_init(ct: u8, flags: u8) -> HandlerOutcome {
    sfx_data_init(7, sfx4(0xB249), 0xB249, ct, flags)
}

/// `MetalSFXInit ; $B735`：0x0B 帧。
pub fn metal_sfx_init(ct: u8, flags: u8) -> HandlerOutcome {
    sfx_data_init(0x0B, sfx4(0xB245), 0xB245, ct, flags)
}

/// `OutOfPipeSFXInit ; $B73C`：escape 音乐占奏⇒TimeBombTick 7 帧；否则 OutOfPipe 0x16 帧。
pub fn out_of_pipe_sfx_init(music_cont: u8, ct: u8, flags: u8) -> HandlerOutcome {
    if music_cont == 0x04 {
        sfx_data_init(7, sfx4(0xB239), 0xB239, ct, flags)
    } else {
        sfx_data_init(0x16, sfx4(0xB259), 0xB259, ct, flags)
    }
}

/// `HasIceBeamSFXInit ; $B75E`：7 帧 IceBeam。
pub fn has_ice_beam_sfx_init(ct: u8, flags: u8) -> HandlerOutcome {
    sfx_data_init(7, sfx4(0xB261), 0xB261, ct, flags)
}

/// `WaveBeamSFXInit ; $B77A`：8 帧。
pub fn wave_beam_sfx_init(ct: u8, flags: u8) -> HandlerOutcome {
    sfx_data_init(8, sfx4(0xB25D), 0xB25D, ct, flags)
}

/// `BulletFireSFXInit ; $B716` 重写（asm 分支序逐字）：
/// ①`HasBeamSFX lsr bcs`⇒HasIceBeamSFXInit（jmp）②SQ1Cont 守卫
/// `& (MissilePickup|EnergyPickup|OutOfPipe|EnemyHit)`=0xCC⇒静默退出
/// ③`HasBeamSFX asl bcs`⇒HasLongBeamSFXInit（jmp）④短光束 3 帧。
pub fn bullet_fire_sfx_init(has_beam: u8, sq1_cont: u8, ct: u8, flags: u8)
    -> HandlerOutcome {
    if has_beam & 0x01 != 0 {
        return has_ice_beam_sfx_init(ct, flags); // `lsr/bcs HasIceBeamSFXInit`
    }
    if sq1_cont & 0xCC != 0 {
        return HandlerOutcome { ram_events: vec![], apu_events: vec![],
                                a_out: sq1_cont, x_out: 0x11, y_out: 0x22 }; // 守卫出口全透传
    }
    if has_beam & 0x80 != 0 {
        return has_long_beam_sfx_init(ct, flags); // `asl/bcs HasLongBeamSFXInit`
    }
    sfx_data_init(3, sfx4(0xB24D), 0xB24D, ct, flags)
}

// ---- task-SFX-1：SFX cont 全族（noise 4 / sq1 6 / tri 8 / multi 3，逐 asm 语义）----
// 锚：`SFXNoiseContRoutineTbl ; $B2CB`（ScrewAttack B539/MissileLaunch B57B/Noise B58A×2/
// SpitFlame B516）；`SFXSQ1ContRoutineTbl ; $B2EB`（MissilePickup B6B0/EnergyPickup B6D3/
// SQ1 B6ED/BulletFire B74F/WaveBeam B781）；`SFXTriContRoutineTbl ; $B30B`
// （SamusDie B8ED/Door B7CB/MetroidHit B8B1/StatueRaise B940/Beep B7E7/BigEnemyHit B80E/
// SamusToBall B84F/BombLaunch B87F）；`SFXMultiContRoutineTbl ; $BC16`
// （SamusHit B650/BossHit B5F6/IncorrectPassword B6A1）。
// 【复核 F5 更正】框架序（`RunSFXNoiseContRoutine ; $B322` 族，sound_engine.asm:223-245）：
// `lda ContFlags / ldx #<data / bne RunSoundRoutine($B337)` → `jsr GetSoundRoutine($B4BD)` →
// `jmp (SoundRoutinePtr)`——**无后置 UpdateContFlags**；cont 旗只由 handler 内部写。
// 本文件 cont 函数只产 handler 面；装载/分派在 hook.rs。

/// `SpitFlameSFXCont ; $B516`：终帧⇒EndNoiseSFX；否则 `NOISE_VOL←SpitFlamesTbl[Var0]`、
/// `Var0+1`（asm 序：lda tbl[y]→sta NOISE_VOL→inc Var0）。
pub fn spit_flame_sfx_cont(var0: u8, frame: u8, length: u8) -> HandlerOutcome {
    let ct = 0u8;
    let (mut ram, ended) = increment_sfx_frame(ct, frame, length);
    let mut apu: Vec<(u16, u8)> = Vec::new();
    if ended {
        let (r, a) = end_noise_sfx(ct);
        ram.extend(r);
        apu.extend(a);
        return HandlerOutcome { ram_events: ram, apu_events: apu, a_out: 0, x_out: 0, y_out: 0 };
    }
    let env = spit_flames_tbl()[var0 as usize];
    apu.push((0x400C, env));
    ram.push((0x0670, var0.wrapping_add(1)));
    HandlerOutcome { ram_events: ram, apu_events: apu, a_out: env, x_out: ct, y_out: length }
}

/// `ScrewAttackSFXCont ; $B539`：`Var2<2` 阶段（每 5 帧 `Var2+1`，终帧才走）；
/// 之后每帧 `IncrementNoisePeriod`；终帧 `Var0-=3 / Var1+1 / ==0x0F⇒EndNoiseSFX`。
pub fn screw_attack_sfx_cont(var0: u8, var1: u8, var2: u8, frame: u8, length: u8)
    -> HandlerOutcome {
    let ct = 0u8;
    let va = 0x0670u16;
    let (mut ram, ended) = increment_sfx_frame(ct, frame, length);
    let mut apu: Vec<(u16, u8)> = Vec::new();
    if var2 < 2 {
        // part1：终帧才 Var2++（未终纯帧计数）
        if ended {
            ram.push((0x0678, var2 + 1));
        }
        let a = if ended { 0 } else { 1 };
        return HandlerOutcome { ram_events: ram, apu_events: apu, a_out: a, x_out: ct, y_out: length };
    }
    if !ended {
        let (r, a) = increment_noise_period(va, var0);
        ram.extend(r);
        apu.extend(a);
        return HandlerOutcome { ram_events: ram, apu_events: apu, a_out: 1, x_out: ct, y_out: length };
    }
    // 终帧收尾
    let nv = var0.wrapping_sub(3);
    ram.push((va, nv));
    let nv1 = var1.wrapping_add(1);
    ram.push((0x0674, nv1));
    if nv1 == 0x0F {
        let (r, a) = end_noise_sfx(ct);
        ram.extend(r);
        apu.extend(a);
    }
    HandlerOutcome { ram_events: ram, apu_events: apu, a_out: nv1, x_out: ct, y_out: length }
}

/// `MissileLaunchSFXCont ; $B57B`：未终⇒`IncrementNoisePeriod`；终⇒EndNoiseSFX。
pub fn missile_launch_sfx_cont(var0: u8, frame: u8, length: u8) -> HandlerOutcome {
    let ct = 0u8;
    let (mut ram, ended) = increment_sfx_frame(ct, frame, length);
    let mut apu: Vec<(u16, u8)> = Vec::new();
    if ended {
        let (r, a) = end_noise_sfx(ct);
        ram.extend(r);
        apu.extend(a);
        return HandlerOutcome { ram_events: ram, apu_events: apu, a_out: 0, x_out: 0, y_out: 0 };
    }
    let (r, a) = increment_noise_period(0x0670, var0);
    ram.extend(r);
    apu.extend(a);
    HandlerOutcome { ram_events: ram, apu_events: apu, a_out: 1, x_out: ct, y_out: length }
}

/// `NoiseSFXCont ; $B58A`：等结束⇒EndNoiseSFX（MissileLaunch/SpitFlame 守卫目）。
pub fn noise_sfx_cont(frame: u8, length: u8) -> HandlerOutcome {
    let ct = 0u8;
    let (mut ram, ended) = increment_sfx_frame(ct, frame, length);
    let mut apu: Vec<(u16, u8)> = Vec::new();
    if ended {
        let (r, a) = end_noise_sfx(ct);
        ram.extend(r);
        apu.extend(a);
        return HandlerOutcome { ram_events: ram, apu_events: apu, a_out: 0, x_out: 0, y_out: 0 };
    }
    HandlerOutcome { ram_events: ram, apu_events: apu, a_out: 1, x_out: ct, y_out: length }
}

/// `BossHitSFXCont ; $B5F6`：**不调 IncrementSFXFrame**（帧计数不参与）。
/// `Var0+1→BossHitSFXDataTbl→SQ1_VOL+SQ2_VOL 双写`；==0x14⇒EndMulti；
/// `>=6`⇒`Var2←(rng|0x10)&0x7F`（cmp#6 carry=1，`rol Var2`→`Var1=Var2*2+1`）；
/// 否则 `Var1+=2/Var2+=2`；两支都落 `WriteSQ1SQ2PeriodLow`（SQ2_LO←Var1/SQ1_LO←Var2）。
pub fn boss_hit_sfx_cont(rng1: u8, var0: u8, var1: u8, var2: u8, music_flag: u8) -> HandlerOutcome {
    let ct = 4u8;
    let nv0 = var0.wrapping_add(1);
    let mut ram = vec![(0x0671, nv0)]; // `inc SFXSQ1Var0`（BossHit 用 SQ1 槽固定址）
    let mut apu = Vec::new();
    let v = boss_hit_sfx_data_tbl()[nv0 as usize];
    apu.push((0x4000, v));
    apu.push((0x4004, v));
    if nv0 == 0x14 {
        let (r, a) = end_multi_sfx(ct, music_flag);
        ram.extend(r);
        apu.extend(a);
        return HandlerOutcome { ram_events: ram, apu_events: apu, a_out: 0, x_out: 0, y_out: 0 };
    }
    let (nv1, nv2, a_out);
    if nv0 >= 6 {
        // `ora #$10 / and #$7F / sta Var2 / rol Var2`（carry=1，Var1=Var2*2+1）
        nv2 = (rng1 | 0x10) & 0x7F;
        nv1 = ((nv2 << 1) | 1) & 0xFF;
        a_out = nv2;
    } else {
        nv1 = var1.wrapping_add(2);
        nv2 = var2.wrapping_add(2);
        a_out = nv2;
    }
    ram.push((0x0675, nv1));
    ram.push((0x0679, nv2));
    apu.extend(write_sq1_sq2_period_low(nv1, nv2));
    HandlerOutcome { ram_events: ram, apu_events: apu, a_out, x_out: ct, y_out: 0 }
}

/// `IncorrectPasswordSFXCont ; $B6A1`：等结束⇒EndMultiSFX。
pub fn incorrect_password_sfx_cont(frame: u8, length: u8, music_flag: u8) -> HandlerOutcome {
    let ct = 4u8;
    let (mut ram, ended) = increment_sfx_frame(ct, frame, length);
    let mut apu: Vec<(u16, u8)> = Vec::new();
    if ended {
        let (r, a) = end_multi_sfx(ct, music_flag);
        ram.extend(r);
        apu.extend(a);
        return HandlerOutcome { ram_events: ram, apu_events: apu, a_out: 0, x_out: 0, y_out: 0 };
    }
    HandlerOutcome { ram_events: ram, apu_events: apu, a_out: 1, x_out: ct, y_out: length }
}

/// `MissilePickupSFXCont ; $B6B0`：未终⇒RTS；终：`tbl[Var0]==0⇒EndSQ1SFX`；
/// 否则 `SQ1_LO←tbl[Var0] / SQ1_HI←data[3]=0x28 / Var0+1`。
pub fn missile_pickup_sfx_cont(var0: u8, frame: u8, length: u8, music_flag: u8) -> HandlerOutcome {
    let ct = 1u8;
    let (mut ram, ended) = increment_sfx_frame(ct, frame, length);
    let mut apu: Vec<(u16, u8)> = Vec::new();
    if ended {
        let v = missile_pickup_sfx_tbl()[var0 as usize];
        if v == 0 {
            let (r, a) = end_sq1_sfx(ct, music_flag);
            ram.extend(r);
            apu.extend(a);
            return HandlerOutcome { ram_events: ram, apu_events: apu, a_out: 0, x_out: 0, y_out: 0 };
        }
        apu.push((0x4002, v));
        apu.push((0x4003, 0x28)); // $B241 data[3]（SQ1_HI 定高）
        ram.push((0x0671, var0.wrapping_add(1)));
        return HandlerOutcome { ram_events: ram, apu_events: apu, a_out: v, x_out: ct, y_out: length };
    }
    HandlerOutcome { ram_events: ram, apu_events: apu, a_out: 1, x_out: ct, y_out: length }
}

/// `EnergyPickupSFXCont ; $B6D3`：未终⇒RTS；终：`Var0+1==3⇒EndSQ1SFX`；
/// 否则 `LoadSFXData@SQ1 EnergyPickup($B23D)`（4B 重载 4000 面）。
pub fn energy_pickup_sfx_cont(var0: u8, frame: u8, length: u8, music_flag: u8) -> HandlerOutcome {
    let ct = 1u8;
    let (mut ram, ended) = increment_sfx_frame(ct, frame, length);
    let mut apu: Vec<(u16, u8)> = Vec::new();
    if ended {
        let nv0 = var0.wrapping_add(1);
        ram.push((0x0671, nv0));
        if nv0 == 3 {
            let (r, a) = end_sq1_sfx(ct, music_flag);
            ram.extend(r);
            apu.extend(a);
            return HandlerOutcome { ram_events: ram, apu_events: apu, a_out: 0, x_out: 0, y_out: 0 };
        }
        let (pr, pa) = load_sfx_data_events(0x4000, 0xB23D);
        ram.extend(pr);
        apu.extend(pa);
        return HandlerOutcome { ram_events: ram, apu_events: apu, a_out: nv0, x_out: ct, y_out: length };
    }
    HandlerOutcome { ram_events: ram, apu_events: apu, a_out: 1, x_out: ct, y_out: length }
}

/// `SQ1SFXCont ; $B6ED`：等结束⇒EndSQ1SFX（SQ1 init 表 4/5/6 目）。
pub fn sq1_sfx_cont(frame: u8, length: u8, music_flag: u8) -> HandlerOutcome {
    let ct = 1u8;
    let (mut ram, ended) = increment_sfx_frame(ct, frame, length);
    let mut apu: Vec<(u16, u8)> = Vec::new();
    if ended {
        let (r, a) = end_sq1_sfx(ct, music_flag);
        ram.extend(r);
        apu.extend(a);
        return HandlerOutcome { ram_events: ram, apu_events: apu, a_out: 0, x_out: 0, y_out: 0 };
    }
    HandlerOutcome { ram_events: ram, apu_events: apu, a_out: 1, x_out: ct, y_out: length }
}

/// `HasIceBeamSFXCont ; $B765`：终⇒EndSQ1SFX；否则
/// `SQ1_LO←@IceBeamTbl[Var0&1]($93/$81)`→`LoadSQ1PeriodLow($B7A0: sta/Var0+1)`。
pub fn has_ice_beam_sfx_cont(var0: u8, frame: u8, length: u8, music_flag: u8) -> HandlerOutcome {
    let ct = 1u8;
    let (mut ram, ended) = increment_sfx_frame(ct, frame, length);
    let mut apu: Vec<(u16, u8)> = Vec::new();
    if ended {
        let (r, a) = end_sq1_sfx(ct, music_flag);
        ram.extend(r);
        apu.extend(a);
        return HandlerOutcome { ram_events: ram, apu_events: apu, a_out: 0, x_out: 0, y_out: 0 };
    }
    let v = ice_beam_tbl()[(var0 & 1) as usize];
    apu.push((0x4002, v));
    ram.push((0x0671, var0.wrapping_add(1)));
    HandlerOutcome { ram_events: ram, apu_events: apu, a_out: v, x_out: ct, y_out: length }
}

/// `BulletFireSFXCont ; $B74F`：ice 位⇒HasIceBeamSFXCont（jmp）；否则等结束⇒EndSQ1SFX。
pub fn bullet_fire_sfx_cont(has_beam: u8, var0: u8, frame: u8, length: u8, music_flag: u8)
    -> HandlerOutcome {
    if has_beam & 0x01 != 0 {
        return has_ice_beam_sfx_cont(var0, frame, length, music_flag);
    }
    sq1_sfx_cont(frame, length, music_flag)
}

/// `WaveBeamSFXCont ; $B781`：终⇒`SQ1_VOL←WaveBeamSFXVolumeTbl[Var1]($93/$91/$00)`、
/// `Var1+1`，音量 `$00`⇒EndSQ1SFX；未终⇒`SQ1_LO←LoTbl[Var0&1]($58/$6F)`/`Var0+1`。
pub fn wave_beam_sfx_cont(var0: u8, var1: u8, frame: u8, length: u8, music_flag: u8) -> HandlerOutcome {
    let ct = 1u8;
    let (mut ram, ended) = increment_sfx_frame(ct, frame, length);
    let mut apu: Vec<(u16, u8)> = Vec::new();
    if ended {
        let vol = wave_beam_volume_tbl()[var1 as usize];
        apu.push((0x4000, vol));
        ram.push((0x0675, var1.wrapping_add(1)));
        if vol == 0 {
            let (r, a) = end_sq1_sfx(ct, music_flag);
            ram.extend(r);
            apu.extend(a);
            return HandlerOutcome { ram_events: ram, apu_events: apu, a_out: 0, x_out: 0, y_out: 0 };
        }
        return HandlerOutcome { ram_events: ram, apu_events: apu, a_out: vol, x_out: ct, y_out: length };
    }
    let v = wave_beam_period_low_tbl()[(var0 & 1) as usize];
    apu.push((0x4002, v));
    ram.push((0x0671, var0.wrapping_add(1)));
    HandlerOutcome { ram_events: ram, apu_events: apu, a_out: v, x_out: ct, y_out: length }
}

/// `DoorOpenCloseSFXCont ; $B7CB`：终⇒EndTriSFX；
/// 否则 `DecreaseSFXTriPeriod→WriteSFXTriPeriod`。
pub fn door_open_close_sfx_cont(plo: u8, phi: u8, clo: u8, chi: u8, frame: u8, length: u8)
    -> HandlerOutcome {
    let ct = 3u8; // Tri（GetSoundRoutineData @SFXTriCont .byte $03）
    let (mut ram, ended) = increment_sfx_frame(ct, frame, length);
    let mut apu: Vec<(u16, u8)> = Vec::new();
    if ended {
        let (r, a) = end_tri_sfx(ct);
        ram.extend(r);
        apu.extend(a);
        return HandlerOutcome { ram_events: ram, apu_events: apu, a_out: 0, x_out: 0, y_out: 0 };
    }
    let (nplo, nphi, ev) = decrease_sfx_tri_period(plo, phi, clo, chi);
    ram.extend(ev);
    apu.extend(write_sfx_tri_period(nplo, nphi));
    HandlerOutcome { ram_events: ram, apu_events: apu, a_out: nphi, x_out: ct, y_out: length }
}

/// `BeepSFXCont ; $B7E7`：等结束⇒EndTriSFX。
pub fn beep_sfx_cont(frame: u8, length: u8) -> HandlerOutcome {
    let ct = 3u8; // Tri（GetSoundRoutineData @SFXTriCont .byte $03）
    let (mut ram, ended) = increment_sfx_frame(ct, frame, length);
    let mut apu: Vec<(u16, u8)> = Vec::new();
    if ended {
        let (r, a) = end_tri_sfx(ct);
        ram.extend(r);
        apu.extend(a);
        return HandlerOutcome { ram_events: ram, apu_events: apu, a_out: 0, x_out: 0, y_out: 0 };
    }
    HandlerOutcome { ram_events: ram, apu_events: apu, a_out: 1, x_out: ct, y_out: length }
}

/// `BigEnemyHitSFXCont ; $B80E`：终⇒EndTriSFX；否则 `IncreaseSFXTriPeriod` 后
/// `TRI_LO←(PeriodLow&0xC3)|(rng&0x3C) / TRI_HI($400B)←PeriodHigh|0x40`（噪声抖动面）。
pub fn big_enemy_hit_sfx_cont(rng1: u8, plo: u8, phi: u8, clo: u8, chi: u8,
                              frame: u8, length: u8) -> HandlerOutcome {
    let ct = 3u8; // Tri（GetSoundRoutineData @SFXTriCont .byte $03）
    let (mut ram, ended) = increment_sfx_frame(ct, frame, length);
    let mut apu: Vec<(u16, u8)> = Vec::new();
    if ended {
        let (r, a) = end_tri_sfx(ct);
        ram.extend(r);
        apu.extend(a);
        return HandlerOutcome { ram_events: ram, apu_events: apu, a_out: 0, x_out: 0, y_out: 0 };
    }
    let (nplo, nphi, ev) = increase_sfx_tri_period(plo, phi, clo, chi);
    ram.extend(ev);
    // asm（sound_engine.asm:1276-1278）：`lda RandomNumber1 / and #$3C / sta SFXTriVar0($0673)`
    // 【复核 F2 补】RAM 面事件此前缺失。
    ram.push((0x0673, rng1 & 0x3C));
    apu.push((0x400A, (nplo & 0xC3) | (rng1 & 0x3C)));
    apu.push((0x400B, nphi | 0x40));
    HandlerOutcome { ram_events: ram, apu_events: apu, a_out: nphi, x_out: ct, y_out: length }
}

/// `SamusToBallSFXCont ; $B84F`：终⇒EndTriSFX；否则
/// `DivideSFXTriPeriod→Change←Divided→DecreaseSFXTriPeriod→WriteSFXTriPeriod`。
pub fn samus_to_ball_sfx_cont(divisor: u8, plo: u8, phi: u8, clo: u8, chi: u8,
                              frame: u8, length: u8) -> HandlerOutcome {
    let ct = 3u8; // Tri（GetSoundRoutineData @SFXTriCont .byte $03）
    let (mut ram, ended) = increment_sfx_frame(ct, frame, length);
    let mut apu: Vec<(u16, u8)> = Vec::new();
    if ended {
        let (r, a) = end_tri_sfx(ct);
        ram.extend(r);
        apu.extend(a);
        return HandlerOutcome { ram_events: ram, apu_events: apu, a_out: 0, x_out: 0, y_out: 0 };
    }
    let (dlo, dhi, rem) = divide_sfx_tri_period(divisor, plo, phi);
    ram.push((0x0617, rem)); // SFXTriPeriodRemainder 终值【复核 F3】
    ram.push((0x0614, dlo));
    ram.push((0x0615, dhi));
    ram.push((0x0612, dlo)); // `sta ChangeLow / sta ChangeHigh`
    ram.push((0x0613, dhi));
    let (nplo, nphi, ev) = decrease_sfx_tri_period(plo, phi, dlo, dhi);
    ram.extend(ev);
    apu.extend(write_sfx_tri_period(nplo, nphi));
    HandlerOutcome { ram_events: ram, apu_events: apu, a_out: nphi, x_out: ct, y_out: length }
}

/// `BombLaunchSFXCont ; $B87F`：未终⇒RTS；终：`Var0+1==2⇒EndTriSFX`；
/// 否则 `LoadSFXData@Tri BombLaunch2($B269)`（4008 面 4B 重载）。
pub fn bomb_launch_sfx_cont(var0: u8, frame: u8, length: u8) -> HandlerOutcome {
    let ct = 3u8;
    let (mut ram, ended) = increment_sfx_frame(ct, frame, length);
    let mut apu: Vec<(u16, u8)> = Vec::new();
    if ended {
        let nv0 = var0.wrapping_add(1);
        ram.push((0x0673, nv0));
        if nv0 == 2 {
            let (r, a) = end_tri_sfx(ct);
            ram.extend(r);
            apu.extend(a);
            return HandlerOutcome { ram_events: ram, apu_events: apu, a_out: 0, x_out: 0, y_out: 0 };
        }
        let (pr, pa) = load_sfx_data_events(0x4008, 0xB269);
        ram.extend(pr);
        apu.extend(pa);
        return HandlerOutcome { ram_events: ram, apu_events: apu, a_out: nv0, x_out: ct, y_out: length };
    }
    HandlerOutcome { ram_events: ram, apu_events: apu, a_out: 1, x_out: ct, y_out: length }
}

/// `@RndTriPeriods ; $B8C3`：`TRI_LO←rng|0x6C / TRI_HI($400B)←(rng&1)|0xF8`。
fn rnd_tri_periods(rng1: u8) -> Vec<(u16, u8)> {
    vec![(0x400A, rng1 | 0x6C), (0x400B, (rng1 & 1) | 0xF8)]
}

/// `MetroidHitSFXCont ; $B8B1`：帧复位⇒`@RndTriPeriods`；否则 `Var0+1`，
/// `==9⇒EndTriSFX`。
pub fn metroid_hit_sfx_cont(rng1: u8, var0: u8, frame: u8, length: u8) -> HandlerOutcome {
    let ct = 3u8;
    let (mut ram, ended) = increment_sfx_frame(ct, frame, length);
    let mut apu: Vec<(u16, u8)> = Vec::new();
    if ended {
        apu.extend(rnd_tri_periods(rng1));
        return HandlerOutcome { ram_events: ram, apu_events: apu, a_out: rng1, x_out: ct, y_out: length };
    }
    let nv0 = var0.wrapping_add(1);
    ram.push((0x0673, nv0));
    if nv0 == 9 {
        let (r, a) = end_tri_sfx(ct);
        ram.extend(r);
        apu.extend(a);
        return HandlerOutcome { ram_events: ram, apu_events: apu, a_out: 0, x_out: 0, y_out: 0 };
    }
    HandlerOutcome { ram_events: ram, apu_events: apu, a_out: nv0, x_out: ct, y_out: length }
}

/// `SamusDieSFXCont ; $B8ED`：未终⇒`Divide→Change←Divided→Increase→Write`；
/// 终⇒`Change←$0020→Decrease→Var0+1（==6⇒EndTriSFX）`。
pub fn samus_die_sfx_cont(divisor: u8, var0: u8, plo: u8, phi: u8, clo: u8, chi: u8,
                          frame: u8, length: u8) -> HandlerOutcome {
    let ct = 3u8; // Tri（GetSoundRoutineData @SFXTriCont .byte $03）
    let (mut ram, ended) = increment_sfx_frame(ct, frame, length);
    let mut apu: Vec<(u16, u8)> = Vec::new();
    if !ended {
        let (dlo, dhi, _) = divide_sfx_tri_period(divisor, plo, phi);
        ram.push((0x0614, dlo));
        ram.push((0x0615, dhi));
        ram.push((0x0612, dlo));
        ram.push((0x0613, dhi));
        let (nplo, nphi, ev) = increase_sfx_tri_period(plo, phi, dlo, dhi);
        ram.extend(ev);
        apu.extend(write_sfx_tri_period(nplo, nphi));
        return HandlerOutcome { ram_events: ram, apu_events: apu, a_out: nphi, x_out: ct, y_out: length };
    }
    ram.push((0x0612, 0x20));
    ram.push((0x0613, 0x00));
    let (nplo, nphi, ev) = decrease_sfx_tri_period(plo, phi, 0x20, 0x00);
    ram.extend(ev);
    let nv0 = var0.wrapping_add(1);
    ram.push((0x0673, nv0));
    if nv0 == 6 {
        let (r, a) = end_tri_sfx(ct);
        ram.extend(r);
        apu.extend(a);
        return HandlerOutcome { ram_events: ram, apu_events: apu, a_out: 0, x_out: 0, y_out: 0 };
    }
    HandlerOutcome { ram_events: ram, apu_events: apu, a_out: nphi, x_out: ct, y_out: length }
}

/// `StatueRaiseSFXCont ; $B940`：未终⇒`Decrease→Write`；终⇒`Var0+1（==9⇒EndTriSFX）`；
/// 否则 `Change=$0025→Increase→恢复 Change→Write`（保护旧 Change 值）。
pub fn statue_raise_sfx_cont(var0: u8, plo: u8, phi: u8, clo: u8, chi: u8,
                             frame: u8, length: u8) -> HandlerOutcome {
    let ct = 3u8; // Tri（GetSoundRoutineData @SFXTriCont .byte $03）
    let (mut ram, ended) = increment_sfx_frame(ct, frame, length);
    let mut apu: Vec<(u16, u8)> = Vec::new();
    if !ended {
        let (nplo, nphi, ev) = decrease_sfx_tri_period(plo, phi, clo, chi);
        ram.extend(ev);
        apu.extend(write_sfx_tri_period(nplo, nphi));
        return HandlerOutcome { ram_events: ram, apu_events: apu, a_out: nphi, x_out: ct, y_out: length };
    }
    let nv0 = var0.wrapping_add(1);
    ram.push((0x0673, nv0));
    if nv0 == 9 {
        let (r, a) = end_tri_sfx(ct);
        ram.extend(r);
        apu.extend(a);
        return HandlerOutcome { ram_events: ram, apu_events: apu, a_out: 0, x_out: 0, y_out: 0 };
    }
    // Change=$0025（临时）→ Increase → 恢复 Change → Write
    ram.push((0x0612, 0x25));
    ram.push((0x0613, 0x00));
    let (nplo, nphi, ev) = increase_sfx_tri_period(plo, phi, 0x25, 0x00);
    ram.extend(ev);
    ram.push((0x0612, clo)); // 恢复旧 Change
    ram.push((0x0613, chi));
    apu.extend(write_sfx_tri_period(nplo, nphi));
    HandlerOutcome { ram_events: ram, apu_events: apu, a_out: nphi, x_out: ct, y_out: length }
}

#[cfg(test)]
mod task_sfx1_tests {
    use super::*;

    /// 跳表分派映射钉测：六 cont 表目→asm 锚址（表区窗平铺读——sound_region 真源）。
    #[test]
    fn cont_table_dispatch_mapping() {
        let sr = game_data::assets::sound_region();
        let base = game_data::assets::SOUND_REGION_ADDR;
        let tbl = |a: u16| &sr[(a - base) as usize..(a - base) as usize + 16];
        // 表内目=标签起 8×2B（asm L151-215 逐 word 实读——GetSoundRoutine 的 y+2 扫描
        // 偏移是旗位扫描面，非表存储序）
        let ent = |a: u16, i: usize| {
            u16::from_le_bytes([tbl(a)[i * 2], tbl(a)[i * 2 + 1]])
        };
        // SFXNoiseContRoutineTbl ; $B2CB（L151 实读）
        assert_eq!(ent(0xB2CB, 1), 0xB539); // ScrewAttack
        assert_eq!(ent(0xB2CB, 2), 0xB57B); // MissileLaunch
        assert_eq!(ent(0xB2CB, 3), 0xB58A); // Noise
        assert_eq!(ent(0xB2CB, 5), 0xB516); // SpitFlame
        // SFXSQ1ContRoutineTbl ; $B2EB
        assert_eq!(ent(0xB2EB, 0), 0xB6B0); // MissilePickup
        assert_eq!(ent(0xB2EB, 1), 0xB6D3); // EnergyPickup
        assert_eq!(ent(0xB2EB, 3), 0xB74F); // BulletFire
        assert_eq!(ent(0xB2EB, 7), 0xB781); // WaveBeam
        // SFXTriContRoutineTbl ; $B30B
        assert_eq!(ent(0xB30B, 0), 0xB8ED); // SamusDie
        assert_eq!(ent(0xB30B, 1), 0xB7CB); // Door
        assert_eq!(ent(0xB30B, 2), 0xB8B1); // MetroidHit
        assert_eq!(ent(0xB30B, 3), 0xB940); // StatueRaise
        assert_eq!(ent(0xB30B, 5), 0xB80E); // BigEnemyHit
        assert_eq!(ent(0xB30B, 6), 0xB84F); // SamusToBall
        assert_eq!(ent(0xB30B, 7), 0xB87F); // BombLaunch
        // SFXMultiContRoutineTbl ; $BC16（目 5-7）
        assert_eq!(ent(0xBC16, 5), 0xB650); // SamusHit
        assert_eq!(ent(0xBC16, 6), 0xB5F6); // BossHit
        assert_eq!(ent(0xBC16, 7), 0xB6A1); // IncorrectPassword
    }

    /// cont 帧计数衰减：increment→终帧复位→End 面（MissileLaunch 全生命周期）。
    #[test]
    fn cont_frame_decay_to_end() {
        // $B56E init：0x18 帧、Var0←0x0A
        let mut o = missile_launch_sfx_init(0, 0x40);
        assert!(o.ram_events.contains(&(0x0660, 0x18)));
        assert!(o.ram_events.contains(&(0x0670, 0x0A)));
        // 中间帧：IncrementNoisePeriod（Var0 增→NOISE_LO 同值）
        o = missile_launch_sfx_cont(0x0A, 0x05, 0x18);
        assert!(o.ram_events.contains(&(0x0665, 0x06))); // 帧推进
        assert!(o.ram_events.contains(&(0x0670, 0x0B))); // Var0+1
        assert!(o.apu_events.contains(&(0x400E, 0x0B))); // NOISE_LO
        // 终帧（frame+1==0x18）：复位+EndNoiseSFX（NOISE_VOL←$10+旗清）
        o = missile_launch_sfx_cont(0x17, 0x17, 0x18);
        assert!(o.ram_events.contains(&(0x0665, 0)));   // 复位
        assert!(o.ram_events.contains(&(0x0688, 0)));   // NoiseCont 旗清（$0688+0）
        assert!(o.ram_events.contains(&(0x064D, 0)));
        assert!(o.apu_events.contains(&(0x400C, 0x10))); // NOISE_VOL 静音
    }

    /// 典型 SFX init 寄存器写序列钉测：DoorOpenClose（Tri 初始化 4B 写序+周期参量）。
    #[test]
    fn door_init_register_write_pin() {
        let o = door_open_close_sfx_init(3, 0x40); // Tri ct=3（GetSoundRoutineData）
        // 4B 顺序写 $4008..$400B（LoadSFXData@Tri Door $B285 数据窗）
        let apu4: Vec<(u16, u8)> = o.apu_events.iter().take(4).cloned().collect();
        let rec = *game_data::assets::sfx_record4(0xB285).as_ref().unwrap();
        let expect: Vec<(u16, u8)> = (0u16..4).map(|i| (0x4008 + i, rec[i as usize])).collect();
        assert_eq!(apu4, expect, "Door 4B 写序");
        // 周期参量：Low←data[2]=0x30、High←data[3]&7、Change=$000F、长度 0x1F
        assert!(o.ram_events.contains(&(0x0663, 0x1F))); // length[ct=3=Tri]
        assert!(o.ram_events.contains(&(0x0610, rec[2])));
        assert!(o.ram_events.contains(&(0x0611, rec[3] & 7)));
        assert!(o.ram_events.contains(&(0x0612, 0x0F)));
        assert!(o.ram_events.contains(&(0x0613, 0x00)));
    }

    /// Tri 周期算术钉测：16bit increase/decrease/divide（$B978/$B98C/$B9A0）。
    #[test]
    fn tri_period_arithmetic_pin() {
        // increase：0x4218 + 0x0012 = 0x422A
        let (lo, hi, _) = increase_sfx_tri_period(0x18, 0x42, 0x12, 0x00);
        assert_eq!((lo, hi), (0x2A, 0x42));
        // decrease：0x4218 - 0x000F = 0x4209
        let (lo, hi, _) = decrease_sfx_tri_period(0x18, 0x42, 0x0F, 0x00);
        assert_eq!((lo, hi), (0x09, 0x42));
        // divide：4020h(=16416) / 15h(=21) = 781=30Dh——SamusDie 初始周期形
        let (dlo, dhi, _) = divide_sfx_tri_period(0x15, 0x20, 0x40);
        assert_eq!((dlo, dhi), (0x0D, 0x03)); // 16416/21=781
        // write 面：TRI_LO←plo / TRI_HI($400B)←phi|0x08
        assert_eq!(write_sfx_tri_period(0x2A, 0x42), vec![(0x400A, 0x2A), (0x400B, 0x4A)]);
    }

    /// WaveBeamCont 音量阶梯钉测（$B781：VolTbl $93/$91/$00，$00⇒EndSQ1SFX）。
    #[test]
    fn wave_beam_volume_ladder() {
        // 终帧+Var1=0 → VOL←$93；Var1=2 → VOL←$00 ⇒ EndSQ1SFX
        let o = wave_beam_sfx_cont(0, 0, 7, 8, 0);
        assert!(o.apu_events.contains(&(0x4000, 0x93)));
        assert!(o.ram_events.contains(&(0x0675, 1)));
        let o = wave_beam_sfx_cont(0, 2, 7, 8, 0);
        assert!(o.apu_events.contains(&(0x4000, 0x00)));
        assert!(o.ram_events.contains(&(0x0689, 0))); // SQ1Cont 旗清（$0688+1）
        assert!(o.apu_events.contains(&(0x4000, 0x10))); // EndSQ1 静音在 VOL 写后
    }

    /// BossHitCont 特例钉测（$B5F6：不 IncrementSFXFrame；Var0>=6 随机周期形）。
    #[test]
    fn boss_hit_cont_random_period_form() {
        // Var0=5→6（进入随机支）：tbl[6] 双 VOL + Var2=(rng|0x10)&0x7F、Var1=Var2*2+1
        let rng = 0x21u8;
        let v2 = (rng | 0x10) & 0x7F; // 0x31
        let v1 = (v2 << 1) | 1;       // 0x63
        let o = boss_hit_sfx_cont(rng, 5, 0x30, 0x2E, 0);
        let vol = boss_hit_sfx_data_tbl()[6];
        assert!(o.apu_events.contains(&(0x4000, vol)) && o.apu_events.contains(&(0x4004, vol)));
        assert!(o.ram_events.contains(&(0x0671, 6)));
        assert!(o.apu_events.contains(&(0x4006, v1)) && o.apu_events.contains(&(0x4002, v2)));
        // Var0=0x14 → EndMultiSFX
        let o = boss_hit_sfx_cont(rng, 0x13, 0, 0, 0);
        assert!(o.apu_events.contains(&(0x4001, 0x7F)));
        assert!(o.ram_events.contains(&(0x068C, 0)));
    }
}

pub mod music;

pub(crate) mod hook;
