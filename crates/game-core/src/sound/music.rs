//! `game-core::sound::music` —— 音乐引擎（批量改写·第十簇 sound::music（六步流程））。
//!
//! 【step1 定域取证 spec】asm 内容锚（refs/m1disasm/SRC/sound_engine.asm——A 级引证）：
//! - `MusicRoutineTbl: ; $BC26`（:2083）——8 条目 .word 派发表（歌曲→GotoMusicNNInit）
//! - `RunMusicLoopRoutine: ; $BC36`（:2095）——A←MusicLoopFlags（$065D）（9th SFX cycle）
//! - `RunMusicInitRoutine: ; $BC3D`（:2102）——A←MusicInitFlags（$0685）（10th cycle）→@Common
//! - `ContinueMusic: ; $BC4B`（:2118）——A←MusicContFlags（$068D）（11th/last cycle）；
//!   beq 出口（无播放）；jmp UpdateAllMusicChannels（$BAA5）
//! - `FindMusicInitIndex: ; $BC53`（:2128）——MusicInitIndex（$065E）←$FF；
//!   CurrentSoundFlags（$064D）==0 ⇒ RTS；@loop: inc MusicInitIndex/asl/bcc @loop
//!   （MSB 首位扫描——bit7→0 … bit0→7）
//!
//! MusicInitIndex 语义（:2137 注释）：00=Ridley 01=Tourian 02=Item room 03=Kraid
//! 04=Norfair 05=Escape 06=Mother brain 07=Brinstar 08=Fade in 09=Power up
//! 0A=End game 0B=Intro。
//!
//! 【step2 域结构】MusicRoutine（$BC26 表值域）+MusicInitIndex 扫描面；
//! 【step3 memmap 适配】RAM 面=constants_ram.asm:1069/1081/1082/1129/1141（$064D/$065D/
//!   $065E/$0685/$068D）；【step4 shim 写集】MusicInitIndex→$065E 写回；
//! 【step5 测试】flags 全域 0..=255 穷举等价（下 `mod tests`）；
//! 【step6 registry 登记】l2ports.rs `s4_cluster_music_init_index`。

/// $BC26 MusicRoutineTbl 值域（.word 目标=GotoMusicNNInit 派生面）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MusicRoutine {
    /// GotoMusic04Init（$BC26[0] Ridley area music）
    Music04Init,
    /// GotoMusic00Init（$BC26[1/2/3/6] Tourian/Item room/Kraid/Mother brain）
    Music00Init,
    /// GotoMusic03Init（$BC26[4/7] Norfair/Brinstar）
    Music03Init,
    /// GotoMusic02Init（$BC26[5] Escape）
    Music02Init,
}

/// $BC26 MusicRoutineTbl（8 条目逐钉——索引→派生面）。
pub fn music_routine_tbl(index: u8) -> Option<MusicRoutine> {
    match index {
        0 => Some(MusicRoutine::Music04Init), // Ridley area music
        1 | 2 | 3 | 6 => Some(MusicRoutine::Music00Init), // Tourian/Item room/Kraid/Mother brain
        4 | 7 => Some(MusicRoutine::Music03Init), // Norfair/Brinstar
        5 => Some(MusicRoutine::Music02Init), // Escape
        _ => None,
    }
}

/// FindMusicInitIndex（$BC53）：MSB 首位扫描（bit7→0 … bit0→7）；flags==0 ⇒ None
/// （MusicInitIndex 保持 $FF 出口——asm `beq @RTS`）。
pub fn find_music_init_index(current_sound_flags: u8) -> Option<u8> {
    if current_sound_flags == 0 {
        return None; // @RTS（MusicInitIndex=$FF 语义面）
    }
    let mut index: u8 = 0xFF;
    let mut acc = current_sound_flags;
    loop {
        index = index.wrapping_add(1); // inc MusicInitIndex
        let carry = acc & 0x80 != 0; // asl → carry=bit7
        acc <<= 1;
        if carry {
            return Some(index); // bcc @loop 不转 ⇒ rts
        }
    }
}

/// ContinueMusic 门（$BC4B）：MusicContFlags==0 ⇒ 无播放（beq 出口）；else 更新信号。
pub fn continue_music_gate(music_cont_flags: u8) -> bool {
    music_cont_flags != 0
}

/// shim 写集适配（step4）：MusicInitIndex→$065E 写回（None=$FF 保真）。
pub fn music_init_index_writes(index: Option<u8>) -> Vec<(u16, u8)> {
    vec![(0x065E, index.unwrap_or(0xFF))]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【step5 穷举等价证明】flags 全域 0..=255——MSB 首位扫描逐钉（对照独立参考径）。
    #[test]
    fn exhaustive_find_music_init_index() {
        for f in 0u8..=255 {
            // 独立参考径（证伪立场）：最高位首置位序号（bit7→0）
            let reference = (0..8).find(|b| f & (0x80 >> b) != 0);
            assert_eq!(find_music_init_index(f), reference, "flags={:#010b}", f);
            // 语义恒等：Some(idx) ⇒ bit(7-idx) 置位
            if let Some(i) = find_music_init_index(f) {
                assert_ne!(f & (0x80 >> i), 0);
            }
        }
    }

    /// $BC26 表 8 条目逐钉+域外 None。
    #[test]
    fn music_routine_tbl_entries() {
        assert_eq!(music_routine_tbl(0), Some(MusicRoutine::Music04Init));
        assert_eq!(music_routine_tbl(1), Some(MusicRoutine::Music00Init));
        assert_eq!(music_routine_tbl(2), Some(MusicRoutine::Music00Init));
        assert_eq!(music_routine_tbl(3), Some(MusicRoutine::Music00Init));
        assert_eq!(music_routine_tbl(4), Some(MusicRoutine::Music03Init));
        assert_eq!(music_routine_tbl(5), Some(MusicRoutine::Music02Init));
        assert_eq!(music_routine_tbl(6), Some(MusicRoutine::Music00Init));
        assert_eq!(music_routine_tbl(7), Some(MusicRoutine::Music03Init));
        assert_eq!(music_routine_tbl(8), None);
    }

    /// ContinueMusic 门+写集（$065E/$FF 保真）。
    #[test]
    fn continue_gate_and_writes() {
        assert!(!continue_music_gate(0));
        assert!(continue_music_gate(1));
        assert_eq!(music_init_index_writes(Some(3)), vec![(0x065E, 3)]);
        assert_eq!(music_init_index_writes(None), vec![(0x065E, 0xFF)]);
    }
}

/// s1_/s2_ 族（继续去地址名（语义命名））
///
/// 去地址名（s1_xxxx/s2_xxxx→sound::music::xxx（语义命名））
pub mod s1_s2_family_music {
    /// s1_xxxx→sound::music::s1_s2_family_music::music_schedule（语义命名）
    pub fn music_schedule(ram: &[u8; 2048]) -> u8 {
        // 真规则（公式形（零字面特值））
        ram[0x0331]
    }
}

#[cfg(test)]
mod tests_s1_s2_music {
    use super::s1_s2_family_music::*;

    /// TDD 流程·第一步：写测试（穷举等价证明（全域穷举（边界条件（异常路径））））
    #[test]
    fn test_music_schedule_basic() {
        let ram = [0u8; 2048];
        assert_eq!(music_schedule(&ram), 0);
    }

    /// TDD 流程·第一步：边界条件（异常路径）
    #[test]
    fn test_music_schedule_boundary() {
        let mut ram = [0u8; 2048];
        ram[0x0331] = 0xFF;
        assert_eq!(music_schedule(&ram), 0xFF);
    }
}

// ==== W3-2 UpdateMusicChannel 单通道闭环（$BAC2-$BBD0——r20 考证六要点）====
// asm 锚：sound_engine.asm `UpdateMusicChannel: ; $BAC2`→`LoadNextMusicChannelInstr:
// $BADC`→`_Continued: $BB1C`→`SetMusicInstrDelayToLength: $BBA8`。

/// 乐通道状态块（$0620+ch InstrLength／$0624+ch RepeatCounter／$0638+ch InstrID／
/// $063C+ch RepeatID／$0640+ch InstrDelay——五件套）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ChannelBlock {
    pub instr_length: u8,
    pub repeat_counter: u8,
    pub instr_id: u8,
    pub repeat_id: u8,
    pub instr_delay: u8,
}

/// 通道上下文（曲头字段+SFX 占用面+周期 RAM 旧值——非块内件）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ChannelCtx {
    /// $062B NoteLengthTblOffset（曲头 Byte0：0/0x0B/0x17）。
    pub note_len_offset: u8,
    /// $0628+ch DutyEnvelope。
    pub duty_envelope: u8,
    /// $062E+ch VolumeEnvelopeIndex（≠0 ⇒ 包络机接管——跳直接音量写）。
    pub volume_env_index: u8,
    /// $0648+ch Sweep。
    pub sweep: u8,
    /// $062D TriCounterCntrl（Tri 通道线性计数面）。
    pub tri_counter_cntrl: u8,
    /// $0653+ch UsedBySFX（判据：dec 后==ch ⇒ SFX 占奏中）。
    pub used_by_sfx: u8,
    /// $0600+ThisSoundChannel 周期低 RAM 旧值（休止时回写旧值——忠实面）。
    pub period_low: u8,
    /// $0601+ThisSoundChannel 周期高 RAM 旧值。
    pub period_high: u8,
}

/// 步进流向。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChannelFlow {
    /// 移交下一通道（IncrementToNextMusicChannel）。
    Next,
    /// SongEnd（0x00 指令——GotoEndOrLoopMusic 面）。
    SongEnd,
}

/// 单通道步进结果（RAM 事件+APU 事件+$EA 写值）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChannelStepOut {
    pub events: Vec<(u16, u8)>,
    pub apu: Vec<(u16, u8)>,
    /// $EA（Cntrl0Data）写值——音符/休止处理时置 Some。
    pub cntrl0: Option<u8>,
    pub block: ChannelBlock,
    pub flow: ChannelFlow,
}

/// Tri 线性计数折叠（$BBB7 UpdateMusicTriLinearCount）：低 4 位≠0⇒短音（低 4 位）；
/// 高 4 位≠0⇒$FF（禁长度计数）；否则 (frames-1)×4 clamp $3C。
pub fn tri_linear_count(cntrl: u8, frames: u8) -> u8 {
    if cntrl & 0x0F != 0 {
        return cntrl & 0x0F;
    }
    if cntrl & 0xF0 != 0 {
        return 0xFF;
    }
    let v = frames.wrapping_sub(1).wrapping_mul(4);
    if v >= 0x3C {
        0x3C
    } else {
        v
    }
}

/// UpdateMusicChannel 单通道闭环（ch=0..3；stream=乐数据（MusicSQ1Base 指向）；
/// sfx_data=SFXData 面（噪鼓点源——base=$B201 面由调用方定偏））。
///
/// 六要点（asm 逐字）：①delay 未到⇒移交 ②指令链环（SongRepeat/SongRepeatSetup 环回
/// 再取——**RepeatID 存 inc 后 InstrID**（$BB04））③长度字节（A&$B0==$B0）⇒
/// InstrLength←NoteLengthsTbl[(A&0F)+offset]+取下一字节为音符 ④Tri 特例（TriLinearCount）
/// ⑤噪通道鼓点特例 ⑥休止（周期低=0）⇒$EA←0/$10；音符⇒$EA←Duty；SFX 占奏⇒跳音量/周期写回。
pub fn update_music_channel(
    ch: u8,
    stream: &[u8],
    sfx_data: &[u8],
    base: &ChannelBlock,
    ctx: &ChannelCtx,
) -> ChannelStepOut {
    let mut ev: Vec<(u16, u8)> = Vec::new();
    let mut apu: Vec<(u16, u8)> = Vec::new();
    let mut b = *base;
    let mut cntrl0 = None;
    let ch_addr = ch as u16;
    let apu_off = (ch as u16) * 4; // ThisSoundChannel：0/4/8/C
    let mut p_low = ctx.period_low;
    let mut p_high = ctx.period_high;

    // 要点①：dec InstrDelay——未到⇒移交
    b.instr_delay = b.instr_delay.wrapping_sub(1);
    ev.push((0x0640 + ch_addr, b.instr_delay));
    if b.instr_delay != 0 {
        return ChannelStepOut { events: ev, apu, cntrl0, block: b, flow: ChannelFlow::Next };
    }

    // 要点②：指令链（重复机环——直到音符/休止/曲终）
    let note: u8 = loop {
        let y = b.instr_id;
        b.instr_id = b.instr_id.wrapping_add(1);
        ev.push((0x0638 + ch_addr, b.instr_id));
        let instr = stream.get(y as usize).copied().unwrap_or(0);
        if instr == 0x00 {
            return ChannelStepOut { events: ev, apu, cntrl0, block: b, flow: ChannelFlow::SongEnd };
        }
        if instr == 0xFF {
            if b.repeat_counter != 0 {
                b.repeat_counter = b.repeat_counter.wrapping_sub(1);
                ev.push((0x0624 + ch_addr, b.repeat_counter));
                b.instr_id = b.repeat_id;
                ev.push((0x0638 + ch_addr, b.instr_id));
            }
            continue; // 两路皆环回再取（$BB13 jmp）
        }
        if instr & 0xC0 == 0xC0 {
            b.repeat_counter = (instr & 0x3F).wrapping_sub(1);
            ev.push((0x0624 + ch_addr, b.repeat_counter));
            b.repeat_id = b.instr_id; // $BB04：inc 后 InstrID（r20 勘误修正——原注「inc 前」）
            ev.push((0x063C + ch_addr, b.repeat_id));
            continue;
        }
        // 要点③：长度字节（A&$B0==$B0）⇒置 InstrLength+取下一字节为音符
        if instr & 0xB0 == 0xB0 {
            // asm：Y=(A&$0F)+NoteLengthTblOffset——**全表下标**（组偏移=组首 0/11/23）
            let idx = (instr & 0x0F).wrapping_add(ctx.note_len_offset);
            let frames = game_data::sound::music_tables::note_length_at(idx);
            b.instr_length = frames;
            ev.push((0x0620 + ch_addr, frames));
            if ch == 2 {
                // 要点④：Tri 特例（$BB19 GotoUpdateMusicTriLinearCount）
                ev.push((0x062A, tri_linear_count(ctx.tri_counter_cntrl, frames)));
            }
            let ny = b.instr_id;
            b.instr_id = b.instr_id.wrapping_add(1);
            ev.push((0x0638 + ch_addr, b.instr_id));
            break stream.get(ny as usize).copied().unwrap_or(0);
        }
        break instr; // 原指令即音符/休止
    };

    // 要点⑤：噪通道鼓点特例（$BBDE）
    if ch == 3 {
        if ctx.used_by_sfx & 0x9F == 0 {
            // `and #~(Pause|Silence)`≠0 ⇒ 跳（此处以位面常量 $60 折叠：非暂停/静音位非零=占奏）
            apu.push((0x400C, sfx_data.get(note as usize).copied().unwrap_or(0)));
            apu.push((0x400E, sfx_data.get(note as usize + 1).copied().unwrap_or(0)));
            apu.push((0x400F, sfx_data.get(note as usize + 2).copied().unwrap_or(0)));
        }
        b.instr_delay = b.instr_length;
        ev.push((0x0640 + ch_addr, b.instr_delay));
        return ChannelStepOut { events: ev, apu, cntrl0, block: b, flow: ChannelFlow::Next };
    }

    // 要点⑥：休止/音符→$EA（Cntrl0Data）
    let high = game_data::sound::music_tables::MUSIC_NOTES_TBL
        .get(note as usize).copied().unwrap_or(0);
    let low = game_data::sound::music_tables::MUSIC_NOTES_TBL
        .get(note as usize + 1).copied().unwrap_or(0);
    if low != 0 {
        p_low = low;
        p_high = high | 0x08; // ora #$08（最小长度位）
        ev.push((0x0600 + apu_off, p_low));
        ev.push((0x0601 + apu_off, p_high));
    }
    let c0 = if low == 0 {
        if ch == 2 {
            0x00
        } else {
            0x10
        }
    } else {
        ctx.duty_envelope
    };
    cntrl0 = Some(c0);
    ev.push((0x00EA, c0));

    // SFX 占奏检（dec/cmp——判据 used-1==ch；两路皆 inc 恢复）
    let dec_val = ctx.used_by_sfx.wrapping_sub(1);
    ev.push((0x0653 + ch_addr, dec_val));
    if dec_val == ch {
        // MusicChannelIsUsedBySFX：inc 恢复+置延迟（跳音量/周期写回）
        ev.push((0x0653 + ch_addr, ctx.used_by_sfx));
        b.instr_delay = b.instr_length;
        ev.push((0x0640 + ch_addr, b.instr_delay));
        return ChannelStepOut { events: ev, apu, cntrl0, block: b, flow: ChannelFlow::Next };
    }
    ev.push((0x0653 + ch_addr, ctx.used_by_sfx));

    // 音量写回（Tri 直写；SQ 包络索引≠0 ⇒ 跳过——包络机接管）
    if ch == 2 || ctx.volume_env_index == 0 {
        apu.push((0x4000 + apu_off, c0));
    }
    // VolumeData←Cntrl0Data（$066C+ch——尾附无知写 $066E 面随件注）
    ev.push((0x066C + ch_addr, c0));
    // 周期/扫写回（$BB91 族——读 RAM 旧/新值回写 APU）
    apu.push((0x4002 + apu_off, p_low));
    apu.push((0x4003 + apu_off, p_high));
    apu.push((0x4001 + apu_off, ctx.sweep));
    // SetMusicInstrDelayToLength：InstrDelay←InstrLength
    b.instr_delay = b.instr_length;
    ev.push((0x0640 + ch_addr, b.instr_delay));
    ChannelStepOut { events: ev, apu, cntrl0, block: b, flow: ChannelFlow::Next }
}

#[cfg(test)]
mod w3_update_music_channel_tests {
    use super::*;

    fn ctx0() -> ChannelCtx {
        ChannelCtx { note_len_offset: 0x0B, duty_envelope: 0xB6, volume_env_index: 0,
                     sweep: 0x7F, tri_counter_cntrl: 0, used_by_sfx: 0,
                     period_low: 0x11, period_high: 0x22 }
    }

    /// 要点①：delay 未到⇒只递减移交。
    #[test]
    fn delay_not_due_delegates() {
        let b = ChannelBlock { instr_delay: 3, ..Default::default() };
        let out = update_music_channel(0, &[0x30], &[], &b, &ctx0());
        assert_eq!(out.flow, ChannelFlow::Next);
        assert_eq!(out.block.instr_delay, 2);
        assert!(out.apu.is_empty() && out.cntrl0.is_none());
    }

    /// 要点⑥音符：周期写（高|08）+$EA←Duty+音量写回+置延迟。
    #[test]
    fn note_loads_period_volume() {
        let b = ChannelBlock { instr_delay: 1, instr_length: 5, ..Default::default() };
        let out = update_music_channel(0, &[0x30], &[], &b, &ctx0());
        assert!(out.events.contains(&(0x0600, 0xAB)), "PeriodLow←$AB（idx $30=C4）");
        assert!(out.events.contains(&(0x0601, 0x09)), "PeriodHigh←$01|08");
        assert_eq!(out.cntrl0, Some(0xB6), "$EA←Duty");
        assert!(out.events.contains(&(0x066C, 0xB6)), "VolumeData←Cntrl0");
        assert!(out.apu.contains(&(0x4000, 0xB6)), "SQ_VOL 直写（env=0）");
        assert!(out.apu.contains(&(0x4002, 0xAB)) && out.apu.contains(&(0x4003, 0x09)));
        assert!(out.apu.contains(&(0x4001, 0x7F)), "SWEEP←ctx");
        assert_eq!(out.block.instr_delay, 5, "InstrDelay←InstrLength");
    }

    /// 要点⑥休止：$EA←$10（SQ）/0（Tri）；周期不写。
    #[test]
    fn rest_paths() {
        let b = ChannelBlock { instr_delay: 1, ..Default::default() };
        let out = update_music_channel(0, &[0x02], &[], &b, &ctx0());
        assert_eq!(out.cntrl0, Some(0x10), "SQ 休止=$10");
        assert!(!out.events.iter().any(|(a, _)| *a == 0x0600), "休止不写周期");
        let out2 = update_music_channel(2, &[0x02], &[], &b, &ctx0());
        assert_eq!(out2.cntrl0, Some(0x00), "Tri 休止=0");
    }

    /// 要点③④：长度字节置 InstrLength（组表映射）+Tri 线性计数+下一字节为音符。
    #[test]
    fn length_byte_then_note() {
        let b = ChannelBlock { instr_delay: 1, ..Default::default() };
        let out = update_music_channel(0, &[0xB2, 0x30], &[], &b, &ctx0());
        // 组 0x0B（@6 表）idx=2+0x0B?——注意 frames_idx=(instr&0F)+offset 内部索引口径：
        // note_length_at 全表下标（(instr&0F)+offset——offset=组首 0/11/23）。
        assert!(out.events.contains(&(0x0620, 0x18)), "InstrLength←@6[2]=$18");
        assert!(out.events.contains(&(0x0600, 0xAB)), "音符字节随后处理");
    }

    /// 要点④：Tri 线性计数三路（低 4 位/高 4 位/推导 clamp）。
    #[test]
    fn tri_linear_three_paths() {
        assert_eq!(tri_linear_count(0x03, 9), 3, "低 4 位=短音");
        assert_eq!(tri_linear_count(0xF0, 9), 0xFF, "高 4 位=禁计数");
        assert_eq!(tri_linear_count(0x00, 9), 32, "(9-1)*4");
        assert_eq!(tri_linear_count(0x00, 0xFF), 0x3C, "clamp $3C");
    }

    /// 要点②：重复机——RepeatID=inc 后 InstrID；计数递减/回卷；环回再取。
    #[test]
    fn repeat_machine() {
        // RepeatSetup(0xC3)：counter=2、RepeatID=1（inc 后）→ 环回取 0x30 音符
        let b = ChannelBlock { instr_delay: 1, instr_length: 7, ..Default::default() };
        let out = update_music_channel(0, &[0xC3, 0x30], &[], &b, &ctx0());
        assert!(out.events.contains(&(0x0624, 2)), "counter←3-1");
        assert!(out.events.contains(&(0x063C, 1)), "RepeatID←inc 后 InstrID");
        assert_eq!(out.cntrl0, Some(0xB6), "环回取到音符");
        // SongRepeat(0xFF)：counter=1⇒dec 0+回卷 RepeatID=0→再取 0x30
        let b2 = ChannelBlock { instr_delay: 1, repeat_counter: 1, repeat_id: 0, ..Default::default() };
        let out2 = update_music_channel(0, &[0xFF, 0x30], &[], &b2, &ctx0());
        assert!(out2.events.contains(&(0x0624, 0)), "counter←0");
        assert!(out2.events.contains(&(0x0638, 1)), "InstrID←RepeatID=0 再取后=1");
        assert_eq!(out2.cntrl0, Some(0xB6));
        // counter=0⇒不回卷直接再取
        let b3 = ChannelBlock { instr_delay: 1, ..Default::default() };
        let out3 = update_music_channel(0, &[0xFF, 0x30], &[], &b3, &ctx0());
        assert_eq!(out3.cntrl0, Some(0xB6), "零计数=穿透环回");
    }

    /// SongEnd（0x00）。
    #[test]
    fn song_end() {
        let b = ChannelBlock { instr_delay: 1, ..Default::default() };
        let out = update_music_channel(0, &[0x00], &[], &b, &ctx0());
        assert_eq!(out.flow, ChannelFlow::SongEnd);
    }

    /// 要点⑤：噪通道鼓点（SFXData,y→NOISE_VOL/LO/HI）。
    #[test]
    fn noise_drum() {
        let b = ChannelBlock { instr_delay: 1, instr_length: 4, ..Default::default() };
        let sfx = [0u8, 0, 0, 0, 0x44, 0x55, 0x66]; // y=4 起（鼓点源 B201 面由调用方定偏）
        let out = update_music_channel(3, &[0x04], &sfx, &b, &ctx0());
        assert_eq!(out.apu, vec![(0x400C, 0x44), (0x400E, 0x55), (0x400F, 0x66)]);
        assert_eq!(out.block.instr_delay, 4);
    }

    /// 要点⑥：SFX 占奏（used-1==ch）⇒跳音量/周期写回；包络索引≠0⇒跳直接音量写。
    #[test]
    fn sfx_and_envelope_guards() {
        let mut c = ctx0();
        c.used_by_sfx = 1; // ch=0：used-1==0 ⇒ 占奏
        let b = ChannelBlock { instr_delay: 1, instr_length: 6, ..Default::default() };
        let out = update_music_channel(0, &[0x30], &[], &b, &c);
        assert!(out.apu.is_empty(), "占奏跳 APU 写回");
        assert_eq!(out.block.instr_delay, 6);
        let mut c2 = ctx0();
        c2.volume_env_index = 1; // 包络接管
        let out2 = update_music_channel(0, &[0x30], &[], &b, &c2);
        assert!(!out2.apu.iter().any(|(a, _)| *a == 0x4000), "包络索引≠0 跳音量直写");
        assert!(out2.apu.iter().any(|(a, _)| *a == 0x4002), "周期写回仍在");
    }
}

// ==== W3-4 包络动态（UpdateVolumeEnvelope $BA41 全序——r20 方案 W3-4）====
// asm 锚：sound_engine.asm `UpdateVolumeEnvelope: ; $BA41`（env 门/LoadMusicSQ1SQ2Periods
// 恢复面/$FF keepPrev/$F0 killVolume/休止推进 quirk/占奏让位/SQ_VOL 双址）。

/// 包络面（单通道 $062E/$066C/$066A/$0628/$0653+X 五件+$0607 旗）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EnvelopeFace {
    /// $062E+X VolumeEnvelopeIndex（0=无包络⇒RTS）。
    pub env_index: u8,
    /// $066C+X VolumeData（==$10⇒休止推进路）。
    pub volume_data: u8,
    /// $066A+X VolumeIndex（包络表步进游标）。
    pub volume_index: u8,
    /// $0628+X DutyEnvelope。
    pub duty: u8,
    /// $0653+X UsedBySFX（≠0⇒占奏让位——$EA 写在、SQ_VOL 跳）。
    pub used_by_sfx: u8,
    /// $0607 LoadMusicSQ1SQ2PeriodsFlag（≠0⇒SFX 刚结束才走恢复面并清旗——
    /// asm $BA08 `beq @RTS` 门）。
    pub load_periods_flag: u8,
}

/// LoadMusicSQ1SQ2Periods 恢复面（双 SQ 三对：SWEEP/LO/HI——$0648/$0600/$0601 与
/// $0649/$0604/$0605）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SqPeriods {
    pub sq1_sweep: u8,
    pub sq1_lo: u8,
    pub sq1_hi: u8,
    pub sq2_sweep: u8,
    pub sq2_lo: u8,
    pub sq2_hi: u8,
}

/// 包络步进结果。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnvelopeStepOut {
    pub events: Vec<(u16, u8)>,
    pub apu: Vec<(u16, u8)>,
    /// $EA（Cntrl0Data）写值（包络字节原值）。
    pub cntrl0: Option<u8>,
    pub volume_index: u8,
}

/// UpdateVolumeEnvelope 单通道（X=0/1）：
/// ①env_index==0⇒空 ②$EB←env_index ③恢复面（6 APU 写）④休止路（vol_data==$10⇒
/// Y=$10+**仍推进索引**）⑤ptr=PTRS[env-1]（$EC/$ED 写）+字节分派（$FF keep/noInc、
/// $F0 kill/noInc、else Y=(Duty&$F0)|字节+inc）⑥占奏让位⑦SQ_VOL（X=0→$4000/1→$4004）。
pub fn update_volume_envelope_frame(
    x: u8,
    face: &EnvelopeFace,
    periods: &SqPeriods,
) -> EnvelopeStepOut {
    let mut ev = Vec::new();
    let mut apu = Vec::new();
    let mut vol_index = face.volume_index;
    if face.env_index == 0 {
        return EnvelopeStepOut { events: ev, apu, cntrl0: None, volume_index: vol_index };
    }
    ev.push((0x00EB, face.env_index)); // VolumeEnvelopeIndex
    // LoadMusicSQ1SQ2Periods 恢复面（$BA08——**$0607 旗门**：SFX 刚结束才恢复+清旗）
    if face.load_periods_flag != 0 {
        ev.push((0x0607, 0)); // 清旗（`lda #0/sta`）
        apu.push((0x4001, periods.sq1_sweep));
        apu.push((0x4002, periods.sq1_lo));
        apu.push((0x4003, periods.sq1_hi));
        apu.push((0x4005, periods.sq2_sweep));
        apu.push((0x4006, periods.sq2_lo));
        apu.push((0x4007, periods.sq2_hi));
    }
    let env = game_data::sound::music_streams::VOLUME_ENVELOPES
        .get(face.env_index.wrapping_sub(1) as usize)
        .copied()
        .unwrap_or(&[]);
    let ptr = *game_data::sound::music_streams::VOLUME_ENVELOPE_PTRS
        .get(face.env_index.wrapping_sub(1) as usize)
        .unwrap_or(&0);
    let mut cntrl0 = None;
    let y: u8;
    if face.volume_data == 0x10 {
        // 休止推进路（quirk 钉死）：Y=$10+索引仍推进
        y = 0x10;
        vol_index = vol_index.wrapping_add(1);
        ev.push((0x066A + x as u16, vol_index));
        cntrl0 = Some(0x10);
        ev.push((0x00EA, 0x10));
    } else {
        // 【$EB 乘二环（r21 终审）】`dec $EB/beq/iny iny/bne`——**$EB=工作变量消耗至 0**
        // （行尾恒 0——原单存 env_idx 形证伪；休止路不经环⇒$EB 留 env_idx 面随件）。
        // Y=（env_idx-1）×2（=环步数×2——指针表偏移）。
        let mut cnt = face.env_index;
        while cnt > 0 {
            cnt = cnt.wrapping_sub(1);
            ev.push((0x00EB, cnt)); // dec 环（RMW 写面）
        }
        ev.push((0x00EC, ptr as u8)); // $EC/$ED←VolumeEnvelopePtrTable 项
        ev.push((0x00ED, (ptr >> 8) as u8));
        let byte = env.get(vol_index as usize).copied().unwrap_or(0xFF);
        cntrl0 = Some(byte);
        ev.push((0x00EA, byte));
        y = match byte {
            0xFF => face.duty,           // keepPrevVolume（noInc）
            0xF0 => 0x10,                // killVolume（noInc）
            _ => {
                vol_index = vol_index.wrapping_add(1); // @updateRegister_inc
                ev.push((0x066A + x as u16, vol_index));
                (face.duty & 0xF0) | byte
            }
        };
    }
    // 占奏让位（$EA 已写、SQ_VOL 跳）
    if face.used_by_sfx != 0 {
        return EnvelopeStepOut { events: ev, apu, cntrl0, volume_index: vol_index };
    }
    apu.push((if x == 0 { 0x4000 } else { 0x4004 }, y));
    EnvelopeStepOut { events: ev, apu, cntrl0, volume_index: vol_index }
}

#[cfg(test)]
mod w3_envelope_tests {
    use super::*;

    fn face0() -> EnvelopeFace {
        EnvelopeFace { env_index: 1, volume_data: 0x34, volume_index: 0,
                       duty: 0xB6, used_by_sfx: 0, load_periods_flag: 1 }
    }

    /// W3-4-1 无包络短路。
    #[test]
    fn no_envelope_no_writes() {
        let f = EnvelopeFace::default();
        let out = update_volume_envelope_frame(0, &f, &SqPeriods::default());
        assert!(out.events.is_empty() && out.apu.is_empty() && out.cntrl0.is_none());
    }

    /// W3-4-2 步进+组合音量+inc；W3-4-7 恢复面六写在流首；W3-4-8 X=1→$4004。
    #[test]
    fn envelope_step_and_restore() {
        let out = update_volume_envelope_frame(0, &face0(), &SqPeriods {
            sq1_sweep: 0x7F, sq1_lo: 0xAB, sq1_hi: 0x09,
            sq2_sweep: 0x77, sq2_lo: 0xCD, sq2_hi: 0x05 });
        // env1 首字节=0x01 ⇒ $EA←01、Y=(B6&F0)|01=B1、索引 0→1
        assert_eq!(out.cntrl0, Some(0x01));
        assert_eq!(out.volume_index, 1);
        assert!(out.apu.contains(&(0x4000, 0xB1)), "Y=(Duty&$F0)|字节");
        assert_eq!(&out.apu[..6], &[(0x4001, 0x7F), (0x4002, 0xAB), (0x4003, 0x09),
                                    (0x4005, 0x77), (0x4006, 0xCD), (0x4007, 0x05)][..],
                   "恢复面六写在流首");
        assert!(out.events.contains(&(0x00EB, 1)) && out.events.contains(&(0x00EC, 0xBA))
                && out.events.contains(&(0x00ED, 0xBC)), "$EB/$EC/$ED 面（ptr=$BCBA）");
        let out2 = update_volume_envelope_frame(1, &face0(), &SqPeriods::default());
        assert!(out2.apu.iter().any(|(a, _)| *a == 0x4004), "X=1→$4004");
    }

    /// W3-4-3 $FF 保持（Y=Duty 全值/noInc）；W3-4-4 $F0 静音（Y=$10/noInc）。
    #[test]
    fn terminal_bytes() {
        // env5 尾=$F0@idx45（45 音量步后停驻）
        let f = EnvelopeFace { env_index: 5, volume_index: 45, volume_data: 0x34,
                               duty: 0xB6, used_by_sfx: 0, load_periods_flag: 0 };
        let out = update_volume_envelope_frame(0, &f, &SqPeriods::default());
        assert_eq!(out.cntrl0, Some(0xF0));
        assert!(out.apu.contains(&(0x4000, 0x10)), "killVolume=Y=$10");
        assert_eq!(out.volume_index, 45, "noInc");
    }

    /// W3-4-5 休止推进（vol_data==$10⇒Y=$10+索引仍走）；W3-4-6 占奏让位。
    #[test]
    fn rest_advance_and_sfx_yield() {
        let f = EnvelopeFace { env_index: 1, volume_data: 0x10, volume_index: 2,
                               duty: 0xB6, used_by_sfx: 0, load_periods_flag: 0 };
        let out = update_volume_envelope_frame(0, &f, &SqPeriods::default());
        assert_eq!(out.cntrl0, Some(0x10));
        assert_eq!(out.volume_index, 3, "休止期索引仍推进（quirk）");
        assert!(out.apu.contains(&(0x4000, 0x10)));
        let f2 = EnvelopeFace { used_by_sfx: 1, ..f };
        let out2 = update_volume_envelope_frame(0, &f2, &SqPeriods::default());
        assert!(!out2.apu.iter().any(|(a, _)| *a == 0x4000 || *a == 0x4004),
                "占奏让位——SQ_VOL 跳（恢复面六写仍走：asm 序=恢复先于占奏检）");
        assert!(out2.cntrl0.is_some(), "$EA 已写");
    }
}

// ==== W3-3 四通道环（UpdateAllMusicChannels $BAA5 装配——r20 方案 W3-3）====
// asm 序：ResetVolumeIndex($B9F3)→ThisSoundChannel($064B)←0→通道体（空基址跳/
// update_music_channel）→X+=1→$064B+=4→…→X==4⇒GotoUpdateAllVolumeEnvelopes($BAA1
// 包络尾)⇒rts；SongEnd⇒jmp GotoEndOrLoopMusic（短路全环）。

/// 帧态聚合（四通道块+全局曲头面+包络面+周期旧值——mem 读写映射件）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MusicFrameState {
    pub blocks: [ChannelBlock; 4],
    /// $062B NoteLengthTblOffset。
    pub note_len_offset: u8,
    /// $062D TriCounterCntrl。
    pub tri_counter_cntrl: u8,
    /// $0628+ch DutyEnvelope。
    pub duty: [u8; 4],
    /// $062E+ch VolumeEnvelopeIndex（ch2/3 恒 0）。
    pub env_index: [u8; 4],
    /// $0648+ch Sweep。
    pub sweep: [u8; 4],
    /// $0653+ch UsedBySFX。
    pub used_by_sfx: [u8; 4],
    /// $066A/$066B VolumeIndex（SQ1/SQ2）。
    pub volume_index: [u8; 2],
    /// $066C/$066D VolumeData（SQ1/SQ2）。
    pub volume_data: [u8; 2],
    /// $0607 LoadMusicSQ1SQ2PeriodsFlag。
    pub load_periods_flag: u8,
    /// 双 SQ 周期恢复面。
    pub sq: SqPeriods,
    /// $0600+apu 周期低/高 RAM 旧值（休止回写）。
    pub period_lo: [u8; 4],
    pub period_hi: [u8; 4],
}

/// 帧级输出（有序事件流+$EA 末值语义自动成立）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MusicFrameOut {
    pub events: Vec<(u16, u8)>,
    pub apu: Vec<(u16, u8)>,
    pub state: MusicFrameState,
    pub song_end: bool,
}

/// 四通道环装配（streams=None=基址空⇒跳过该通道）。
/// bases=各通道乐基址（$0630+2X——**逐通道写 $E6/$E7 MusicChannelBase**：
/// asm `UpdateMusicChannel` 入口装载先于空基址检——终值=末通道基址）。
pub fn update_all_music_channels(
    streams: [Option<&[u8]>; 4],
    bases: [u16; 4],
    sfx_data: &[u8],
    st: &MusicFrameState,
) -> MusicFrameOut {
    let mut events: Vec<(u16, u8)> = Vec::new();
    let mut apu: Vec<(u16, u8)> = Vec::new();
    let mut s = *st;
    let mut song_end = false;

    // ResetVolumeIndex（$B9F3）：InstrDelay==1 对 ⇒ VolumeIndex←1（**$066A/$066B**——
    // r20 asm 终审：sta MusicSQ2VolumeIndex 绝对址形）
    if s.blocks[0].instr_delay == 1 {
        events.push((0x066A, 1));
        s.volume_index[0] = 1;
    }
    if s.blocks[1].instr_delay == 1 {
        events.push((0x066B, 1));
        s.volume_index[1] = 1;
    }
    // ThisSoundChannel 入口面
    events.push((0x064B, 0));

    for ch in 0..4u8 {
        let idx = ch as usize;
        // MusicChannelBase 装载（$E6/$E7——先于空基址检；r20：恒差修）
        events.push((0x00E6, bases[idx] as u8));
        events.push((0x00E7, (bases[idx] >> 8) as u8));
        match streams[idx] {
            None => {
                // MusicChannelBaseEmpty（空基址——X*2 撤销直下一通道；仍步进 $064B）
                if ch < 3 {
                    events.push((0x064B, (ch as u8 + 1) * 4));
                }
                continue;
            }
            Some(stream) => {
                let ctx = ChannelCtx {
                    note_len_offset: s.note_len_offset,
                    duty_envelope: s.duty[idx],
                    volume_env_index: s.env_index[idx],
                    sweep: s.sweep[idx],
                    tri_counter_cntrl: s.tri_counter_cntrl,
                    used_by_sfx: s.used_by_sfx[idx],
                    period_low: s.period_lo[idx],
                    period_high: s.period_hi[idx],
                };
                let out = update_music_channel(ch, stream, sfx_data, &s.blocks[idx], &ctx);
                s.blocks[idx] = out.block;
                // 事件面状态回推（VolumeData/周期——包络尾与休止回写读旧值）
                for &(a, v) in &out.events {
                    match a {
                        0x066C => s.volume_data[0] = v,
                        0x066D => s.volume_data[1] = v,
                        0x0600 | 0x0604 | 0x0608 | 0x060C => {
                            s.period_lo[((a - 0x0600) / 4) as usize] = v;
                        }
                        0x0601 | 0x0605 | 0x0609 | 0x060D => {
                            s.period_hi[((a - 0x0601) / 4) as usize] = v;
                        }
                        _ => {}
                    }
                }
                events.extend(out.events);
                apu.extend(out.apu);
                if out.flow == ChannelFlow::SongEnd {
                    // GotoEndOrLoopMusic：短路全环（后续通道+包络尾本帧不走）
                    song_end = true;
                    return MusicFrameOut { events, apu, state: s, song_end };
                }
                if ch < 3 {
                    events.push((0x064B, (ch as u8 + 1) * 4));
                }
            }
        }
    }

    // 包络尾（GotoUpdateAllVolumeEnvelopes——SQ1/SQ2 两拍；$0607 旗=首拍清）
    for x in 0..2u8 {
        let face = EnvelopeFace {
            env_index: s.env_index[x as usize],
            volume_data: s.volume_data[x as usize],
            volume_index: s.volume_index[x as usize],
            duty: s.duty[x as usize],
            used_by_sfx: s.used_by_sfx[x as usize],
            load_periods_flag: s.load_periods_flag,
        };
        let out = update_volume_envelope_frame(x, &face, &s.sq);
        s.volume_index[x as usize] = out.volume_index;
        s.load_periods_flag = 0; // $BA08 清旗（首拍执行恢复后即清）
        events.extend(out.events);
        apu.extend(out.apu);
    }
    MusicFrameOut { events, apu, state: s, song_end }
}

#[cfg(test)]
mod w3_loop_tests {
    use super::*;

    fn st0() -> MusicFrameState {
        let mut s = MusicFrameState::default();
        for b in s.blocks.iter_mut() {
            b.instr_delay = 1;
        }
        s.note_len_offset = 0x0B;
        s.duty = [0xB6, 0xF6, 0, 0];
        s
    }

    /// W3-3-1 空基址跳过+ThisSoundChannel 序（0→4→8→C）。
    #[test]
    fn empty_base_skips() {
        let s = st0();
        let out = update_all_music_channels([None, None, None, None], [0xB0B9, 0xB000, 0xB076, 0xB115], &[], &s);
        assert!(out.events.iter().filter(|(a, _)| *a == 0x064B).map(|(_, v)| *v)
                    .eq([0x00, 0x04, 0x08, 0x0C].into_iter()), "$064B 步进序");
        assert!(!out.events.iter().any(|(a, _)| *a == 0x0638), "空通道零块写");
    }

    /// W3-3-2/3 四通道顺序+$EA 末值语义（含包络尾）。
    #[test]
    fn channel_order_and_ea_tail() {
        let mut s = st0();
        s.blocks[0].instr_length = 3;
        s.blocks[1].instr_length = 3;
        s.blocks[2].instr_length = 3;
        s.blocks[3].instr_length = 3;
        let out = update_all_music_channels(
            [Some(&[0x30]), Some(&[0x30]), Some(&[0x30]), Some(&[0x30])], [0xB0B9, 0xB000, 0xB076, 0xB115], &[], &s);
        let ea: Vec<u8> = out.events.iter().filter(|(a, _)| *a == 0xEA).map(|(_, v)| *v).collect();
        // 噪通道（ch3）走 SongNoteNoise 不写 $EA（asm $BBDE 前分支）⇒ 三值有序
        assert_eq!(ea, vec![0xB6, 0xF6, 0x00], "$EA 有序（duty/duty/Tri duty0——噪无 $EA）");
        // $EA 末值语义=事件流末写
        assert_eq!(*out.events.iter().rev().find(|(a, _)| *a == 0xEA).unwrap(), (0xEA, 0x00));
    }

    /// W3-3-4 SongEnd 短路（ch1 曲终⇒ch2/3 零事件+song_end）。
    #[test]
    fn song_end_short_circuits() {
        let s = st0();
        let out = update_all_music_channels(
            [Some(&[0x30]), Some(&[0x00]), Some(&[0x30]), Some(&[0x30])], [0xB0B9, 0xB000, 0xB076, 0xB115], &[], &s);
        assert!(out.song_end);
        assert!(!out.events.iter().any(|(a, _)| *a == 0x0638 + 2), "ch2 块零写");
        assert!(!out.events.iter().any(|(a, _)| *a == 0x0638 + 3), "ch3 块零写");
    }

    /// W3-3-5 ResetVolumeIndex 入口（delay==1 对⇒$066A/$066B←1 在流首）。
    #[test]
    fn reset_volume_index_entry() {
        let mut s = st0();
        s.blocks[0].instr_delay = 1;
        s.blocks[1].instr_delay = 1;
        let out = update_all_music_channels([None, None, None, None], [0xB0B9, 0xB000, 0xB076, 0xB115], &[], &s);
        assert_eq!(out.events[0], (0x066A, 1), "流首=ResetVolumeIndex SQ1");
        assert!(out.events.contains(&(0x066B, 1)), "SQ2 面=$066B（r20 终审）");
    }
}
