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
