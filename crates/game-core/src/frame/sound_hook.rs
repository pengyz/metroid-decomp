//! 声引擎钩件（B31B 行清偿——port2 分派核 9dc12a7 会师契约接点）。
//!
//! **接线形态**：`sound::get_sound_routine`（纯函数核）+我侧数据/应用——八 cycle
//! 装载器形（`lda 旗/X 表选`→`jmp (SoundRoutinePtr)`）=链走分派：
//! ①无旗⇒默认例程（装载器链续 cycle——SoundEngine 循环自续面）
//! ②handler 落地前=**记账点**（routine_ptr 爆址——预期缺口演化）
//! ③Multi 特例双步（FindMusicInitIndex+MusicInitIndexAdd8——port2 注④）
//! ④终端=GetSoundRoutine@RTS（$B4EE——表「No sound」共值自证）。
//!
//! **字节自证**（bank0 镜面——sig `22 B3 00` 六 bank 同字节）：装载器体逐字
//! （B31B `ad 80 06 a2 89 d0 15`=lda $0680/ldx #$89/bne RunSoundRoutine 等）；
//! 块表 $B289（5B：TablePtr2+DefaultRoutine2+ChannelType）；表区 $B2BB 起
//! （8 词×10——前 4B 头/表目 y=4 起（port2 注②））。

use crate::frame::FrameMemory;
use crate::sound::{self, SoundDataBlock};

/// `SetSFXNoiseInitFlag: ; 07:CBA0` 的入口形：`SFXNoiseInitFlags($0680) |= bit`
/// （`ldx #SFXNoiseInitFlags-SFXNoiseInitFlags / beq SetSoundInitFlag` ⇒ 恒走 noise 族）；
/// 各封装子例程只差立即数——`SilenceMusic($CB8E)`=%00000001、
/// `PauseMusic($CB92)`=%00000010（`refs/m1disasm/SRC/constants.asm:136-137`）。
///
/// **单一实现面**：此前 `title_bridge::initialize_game` 内联过一份 SilenceMusic，
/// 本件（抢救件#2）需要 PauseMusic，故收敛到此函数供两处共用（防第二处展开）。
pub fn set_sfx_noise_init(mem: &mut FrameMemory, bit: u8) {
    let n = mem.read_ram(crate::ram_map::SFX_NOISE_INIT) | bit;
    mem.write_ram(crate::ram_map::SFX_NOISE_INIT, n);
}

/// `SilenceMusic: ; 07:CB8E` = `set_sfx_noise_init(mem, sfxNoise_SilenceMusic)`。
pub fn silence_music(mem: &mut FrameMemory) {
    set_sfx_noise_init(mem, 0x01);
}

/// `PauseMusic: ; 07:CB92` = `set_sfx_noise_init(mem, sfxNoise_PauseMusic)`。
pub fn pause_music(mem: &mut FrameMemory) {
    set_sfx_noise_init(mem, 0x02);
}

// 声引擎区窗口（`bank00:$B289..$BD09`）——数据在资产包，这里只是取址。
// 2688 B 由 ROM 平铺切片 `prg[0x3289..0x3D09]` 定锚（逐字节与历史常量相等，W5 迁包批次）；
// 区内偏移寻址口径见 `game_data::assets::sound_region` 的文档。
use game_data::assets::sound_region;

/// GetSoundRoutine@RTS=$B4EE（表「No sound」共值自证）。
const RTS_TERMINAL: u16 = 0xB4EE;

/// 装载器→(flag 址, block 窗址) 图（字节自证：各 `lda 旗/ldx #block` 体）。
const LOADER_MAP: &[(u16, u16, u16)] = &[
    (0xB31B, 0x0680, 0xB289), // NoiseInit
    (0xB322, 0x0688, 0xB28E), // NoiseCont
    (0xB329, 0x0681, 0xB293), // SQ1Init
    (0xB330, 0x0689, 0xB298), // SQ1Cont
    (0xB33D, 0x0683, 0xB29D), // TriInit
    (0xB344, 0x068B, 0xB2A2), // TriCont
    (0xB34B, 0x0684, 0xB2A7), // MultiInit（双步特例）
    (0xB35C, 0x068C, 0xB2AC), // MultiCont
    (0xBC36, 0x065D, 0xB2B6), // MusicLoop（temp flag Music）
    (0xB364, 0x0681, 0xB293), // GotoRunSFXSQ1InitRoutine（跳板：jsr B329/rts——字节自证 `20 29 b3 60`）
];

/// 区内取址（窗址→区偏移）。
fn region_byte(win: u16) -> u8 {
    sound_region()[(win - 0xB289) as usize]
}

/// 块装载（5B：TablePtr2+DefaultRoutine2+ChannelType）。
fn load_block(win: u16) -> SoundDataBlock {
    let o = (win - 0xB289) as usize;
    SoundDataBlock {
        table_ptr: u16::from_le_bytes([sound_region()[o], sound_region()[o + 1]]),
        default_routine: u16::from_le_bytes([sound_region()[o + 2], sound_region()[o + 3]]),
        channel_type: sound_region()[o + 4],
    }
}

/// 表切片（TablePtr 窗址→16B 表）。
fn table_slice(table_ptr: u16) -> &'static [u8] {
    let o = (table_ptr - 0xB289) as usize;
    &sound_region()[o..o + 16]
}

/// 【止血件1③ DSH_SOUND_TRACE】声链触达打点（ptr+帧+深度+接/未接面）。
///
/// 为什么需要它：`data/status/handler_touchlist.json` 只固化了 **`_ => None` 未接臂**
/// （sidecar=DIVERGENCES 记账面），已接的 22 件 handler + 6 件音乐 init 组合臂**从未被
/// 观测过触达**——「已接但实际零触达」与「已接且高频」在账上是同一行，无法区分。
/// 本打点把两面合成同一口径（kind=handler / unhandled），供触达统计与覆盖面复盘。
///
/// 开销：默认一次 getenv + 分支，不开环境变量时零输出零分配。
fn sound_trace(mem: &crate::frame::FrameMemory, ptr: u16, depth: u8, kind: &str) {
    if std::env::var("DSH_SOUND_TRACE").is_ok() {
        eprintln!(
            "[sound] fc={:02X} ptr={:04X} kind={} depth={}",
            mem.read_ram(0x002D),
            ptr,
            kind,
            depth
        );
    }
}

/// 链走分派一步（B31B 行清偿核心）：装载→get_sound_routine→
/// 终端/链续/handler 记账。
pub fn sound_dispatch_chain(mem: &mut FrameMemory, flag_addr: u16, block_win: u16, depth: u8) {
    if depth > 12 {
        crate::unported!("bank07:B3xx-chain"); // 链深护栏（环走防御）
    }
    let flags = mem.read_ram(flag_addr);
    let block = load_block(block_win);
    // GetSoundRoutine 全语义（$B4BD——r20 接线补齐）：**事件集全应用**（$064D 旗副本/
    // $E4/$E5 窗址/4B 拷含默认/$065C/表[y]覆盖——F1 公共根的链内落地）
    let (ptr, hit, ev) = sound::get_sound_routine_writes(block_win, &block, table_slice(block.table_ptr), flags);
    for (a, v) in ev {
        mem.write_ram(a, v);
    }
    // 【r20 忠实扫描】FindMusicInitIndex 写序（$BC53：FF→逐位 inc）——Multi 特例加
    // Add8（$BC64）；音乐 cycle 不加。原"粗形 multi_music_index_step+裸 FF 写"退役。
    if block_win == 0xB2A7 || block_win == 0xB2B1 || block_win == 0xB2B6 {
        let (_idx, scan_ev) = sound::find_music_init_index_writes(flags, block_win == 0xB2A7);
        for (a, v) in scan_ev {
            mem.write_ram(a, v);
        }
    }
    // 指针位序（B4CA 面）：MusicLoop 步（flag=$065D）写 $E0/$E1（中间 ptr）；
    // 终步写 $E2/$E3（B337 面）。oracle 实证：$E0=$BC26（RunMusicInit）/$E2=$BC4B（终）。
    if block_win == 0xB2B1 || block_win == 0xB2B6 {
        // $E0/$E1=**TablePtr**（block[0/1]——B4CA 4B 拷形；oracle 实证 $BC26=MusicRoutineTbl）
        mem.write_ram(0x00E0, block.table_ptr as u8);
        mem.write_ram(0x00E1, (block.table_ptr >> 8) as u8);
    }
    let _ = hit;
    if ptr == RTS_TERMINAL {
        return; // 无音/终端（GetSoundRoutine@RTS）
    }
    // 链续（默认=下一装载器）
    if let Some(&(_, f, b)) = LOADER_MAP.iter().find(|(l, _, _)| *l == ptr) {
        // 【r20】Multi 特例双步已上移至 B2A7 入口（忠实 scan——原粗形步退役）
        sound_dispatch_chain(mem, f, b, depth + 1);
        return;
    }
    // ContinueMusic 微折（BC4B：无音路即出——`lda MusicContFlags($068D)/beq RTS`）
    if ptr == 0xBC4B {
        // 【W3-3/4 接线（r20）】$068D≠0 ⇒ BAA5 全径（四通道环+包络尾）——原 unported 退役
        continue_music_face(mem);
        return;
    }
    // ===== handler 接点（22 件批量接——port2 清单；B673 同形）=====
    // 契约：`sound::<fn>(params) -> HandlerOutcome{ram_events, apu_events, a/x/y_out}`
    // （纯函数；我供输入（RAM 面）/应用结果（RAM+APU+regs））。
    {
        let ct = mem.read_ram(0x065C);
        let flags = mem.read_ram(0x064D);
        let out: Option<sound::HandlerOutcome> = match ptr {
            0xB673 => Some(sound::samus_hit_sfx_init(mem.read_ram(0x002E), ct, flags)),
            0xB834 => Some(sound::samus_to_ball_sfx_init(ct, flags)),
            0xB650 => Some(sound::samus_hit_sfx_cont(
                mem.read_ram(0x002E), ct,
                mem.read_ram(0x0665u16 + ct as u16),
                mem.read_ram(0x0660u16 + ct as u16),
            )),
            0xB598 => Some(sound::samus_walk_sfx_init(mem.read_ram(0x0688), ct, flags)),
            0xB710 => Some(sound::enemy_hit_sfx_init(ct, flags)),
            0xB716 => sound::bullet_fire_sfx_init(
                mem.read_ram(0x061F), mem.read_ram(0x0689), ct, flags,
            ), // Option（None=ice/long 变体分支——钩按分支标）
            0xB56E => Some(sound::missile_launch_sfx_init(ct, flags)),
            0xB6CD => Some(sound::missile_pickup_sfx_init(ct, flags)),
            0xB6E7 => Some(sound::energy_pickup_sfx_init(ct, flags)),
            0xB50F => Some(sound::spit_flame_sfx_init(ct, flags)),
            0xB52B => Some(sound::screw_attack_sfx_init(ct, flags)),
            0xB583 => Some(sound::bomb_explode_sfx_init(ct, flags)),
            0xB5EC => Some(sound::boss_hit_sfx_init(ct, flags)),
            0xB695 => Some(sound::incorrect_password_sfx_init(ct, flags)),
            0xB703 => Some(sound::samus_jump_sfx_init(mem.read_ram(0x068D), ct, flags)),
            0xB7AC => Some(sound::door_open_close_sfx_init(ct, flags)),
            0xB7EF => Some(sound::big_enemy_hit_sfx_init(ct, flags)),
            0xB878 => Some(sound::bomb_launch_sfx_init(ct, flags)),
            0xB8A7 => Some(sound::metroid_hit_sfx_init(mem.read_ram(0x002E), ct, flags)),
            0xB921 => Some(sound::statue_raise_sfx_init(ct, flags)),
            0xB7D9 => Some(sound::beep_sfx_init(mem.read_ram(0x068B), flags, ct)),
            0xB8D2 => {
                // ⚠️InitializeSoundAddresses 全清前置（清单注）
                sound::initialize_sound_addresses();
                Some(sound::samus_die_sfx_init(ct, flags))
            }
            // ===== 音乐 init 组合臂（BC77-BC86 GotoMusicNNInit 族——f16 发射序实证；
            // r17 复核后首件）=====
            0xBC77 | 0xBC7A | 0xBC7D | 0xBC80 | 0xBC83 | 0xBC86 => {
                // duty 双形（asm BC89-BCAB 逐字）：形A 同值（lda→tax/tay）/形B 分值
                let (dx, dy) = match ptr {
                    0xBC77 => (0x92, 0x96), // Music00Init（BCAA `ldx #$92/ldy #$96`）
                    0xBC7A => (0xB6, 0xF6), // Music01Init（BCA4）——boot-title Intro（0x0B）路
                    0xBC7D => (0xF4, 0xF4), // Music02Init（BC9A `lda #$F4`→XYMusicInit 同值）
                    0xBC80 => (0x34, 0x34), // Music03Init（BC96 `lda #$34`）
                    0xBC83 => (0xB3, 0xB3), // Music04Init（BC89 `lda #$B3`）
                    _ => (0xF5, 0xF6),      // Music05Init（BC9E 分值）
                };
                let mut ram: Vec<(u16, u8)> = Vec::new();
                let mut apu: Vec<(u16, u8)> = Vec::new();
                // CheckMusicFlags（$B3FC）：MusicContFlags==CurrentSoundFlags⇒同曲重开；
                // 异曲⇒InitializeSoundAddresses（$B404）全清（f16 $068D←00(B404)+
                // $065D←00(B40A) 双写实证——同值写也是事件）
                if !sound::check_music_flags(mem.read_ram(0x068D), mem.read_ram(0x064D)) {
                    let (r, a) = sound::initialize_sound_addresses();
                    ram.extend(r);
                    apu.extend(a);
                }
                // LBC8D 组合（asm `jsr SetVolumeAndDisableSweep / jsr InitializeMusic /
                // jmp UpdateAllMusicChannels`）：
                ram.push((0x0648, 0x7F)); // SetVolumeAndDisableSweep（$B9E4）
                ram.push((0x0649, 0x7F));
                ram.push((0x0628, dx)); // MusicSQ1Duty（形参 X）
                ram.push((0x0629, dy)); // MusicSQ2Duty（形参 Y）
                let cur = mem.read_ram(0x064D);
                // InitializeMusic（$BF19）：MusicContFlags←CurrentSoundFlags + 13B 曲头
                // （SongHeaderOffsetTable[MusicInitIndex]——$065E=曲号，0x0B=Intro）
                let idx = mem.read_ram(0x065E) as usize;
                let off = *game_data::assets::song_header_offsets().get(idx).unwrap_or(&0) as usize;
                let mut header = [0u8; 13];
                if off + 13 <= game_data::assets::song_headers().len() {
                    header.copy_from_slice(&game_data::assets::song_headers()[off..off + 13]);
                }
                ram.extend(sound::initialize_music(cur, &header));
                // 【LBC8D 尾（r21 终审——r23 实验确认供 1 更新：撤后声面 +1→+2）】
                // `jmp UpdateAllMusicChannels` 全更新。先落已积事件（快照一致）再跑环：
                for &(a, v) in ram.iter() {
                    mem.write_ram(a, v);
                }
                let st_tail = read_music_frame_state(mem);
                let mut bases_tail = [0u16; 4];
                let streams_tail: [Option<&[u8]>; 4] = core::array::from_fn(|ch| {
                    let b = mem.read_ram(0x0630 + ch as u16 * 2) as u16
                        | ((mem.read_ram(0x0631 + ch as u16 * 2) as u16) << 8);
                    bases_tail[ch] = b;
                    if b == 0 { None } else { game_data::assets::stream_by_base(b) }
                });
                let out_tail = sound::music::update_all_music_channels(
                    streams_tail, bases_tail,
                    game_data::assets::sfx_data(), &st_tail);
                ram.extend(out_tail.events);
                apu.extend(out_tail.apu);
                Some(sound::HandlerOutcome { ram_events: ram, apu_events: apu,
                                             a_out: 0, x_out: dx, y_out: dy })
            }
            _ => None,
        };
        if let Some(out) = out {
            sound_trace(mem, ptr, depth, "handler");
            for (a, v) in out.ram_events {
                mem.write_ram(a, v);
            }
            for (a, v) in out.apu_events {
                mem.write_io(a, v); // APU 寄存器面（L2 stub 域）
            }
            mem.regs.a = out.a_out;
            mem.regs.x = out.x_out;
            mem.regs.y = out.y_out;
            return;
        }
    }
    // handler 记账点（未接域件=新记账——缺口演化面）
    // 【长脚本验收收口（2026-11-01 goal-轮4：sc-first-room-full 验收主案引爆此桩——
    // 声事件=验收点 1-5 外域（B 声音包））】占位臂记账（不炸）——声链深件随包收敛。
    sound_trace(mem, ptr, depth, "unhandled");
    let loc = format!("bank07:{ptr:04X}");
    crate::frame::DIVERGENCES.with(|d| d.borrow_mut().push(ptr));
    let _ = loc;
}

/// BAA5 读态 glue（RAM→MusicFrameState——五块+全局面；r20 W3-3 接线）。
fn read_music_frame_state(mem: &crate::frame::FrameMemory) -> sound::music::MusicFrameState {
    let r = |a: u16| mem.read_ram(a);
    let mut st = sound::music::MusicFrameState::default();
    for ch in 0..4u16 {
        let i = ch as usize;
        st.blocks[i] = sound::music::ChannelBlock {
            instr_length: r(0x0620 + ch),
            repeat_counter: r(0x0624 + ch),
            instr_id: r(0x0638 + ch),
            repeat_id: r(0x063C + ch),
            instr_delay: r(0x0640 + ch),
        };
        st.duty[i] = r(0x0628 + ch);
        st.env_index[i] = r(0x062E + ch);
        st.sweep[i] = r(0x0648 + ch);
        st.used_by_sfx[i] = r(0x0653 + ch);
        st.period_lo[i] = r(0x0600 + ch * 4);
        st.period_hi[i] = r(0x0601 + ch * 4);
    }
    st.note_len_offset = r(0x062B);
    st.tri_counter_cntrl = r(0x062D);
    st.volume_index = [r(0x066A), r(0x066B)];
    st.volume_data = [r(0x066C), r(0x066D)];
    st.load_periods_flag = r(0x0607);
    st.sq = sound::music::SqPeriods {
        sq1_sweep: st.sweep[0], sq1_lo: st.period_lo[0], sq1_hi: st.period_hi[0],
        sq2_sweep: st.sweep[1], sq2_lo: st.period_lo[1], sq2_hi: st.period_hi[1],
    };
    st
}

/// ContinueMusic（BC4B）装配面——$068D≠0 ⇒ BAA5 全径（W3-3/4）；SongEnd⇒EndOrLoopMusic
/// 最小面（$062C 无重复⇒$068D←0；有重复⇒unported fail-loud——曲重启面随件）。
pub(crate) fn continue_music_face(mem: &mut crate::frame::FrameMemory) {
    if mem.read_ram(0x068D) == 0 {
        return; // beq 出（无音路）
    }
    let st = read_music_frame_state(mem);
    // 【基址反查（r20）】乐循环按 $0630+2X 基址字取流——$065E 曲号会被音乐 cycle 的
    // FindMusicInitIndex 重置（FF），不可作键（实测 $065E=FF 而流仍走=asm 形）。
    let mut bases_arr = [0u16; 4];
    let streams: [Option<&[u8]>; 4] = core::array::from_fn(|ch| {
        let base = mem.read_ram(0x0630 + ch as u16 * 2) as u16
            | ((mem.read_ram(0x0631 + ch as u16 * 2) as u16) << 8);
        bases_arr[ch] = base;
        if base == 0 {
            None // 基址 0（MusicChannelBaseEmpty）
        } else {
            game_data::assets::stream_by_base(base)
        }
    });
    let out = sound::music::update_all_music_channels(
        streams, bases_arr, game_data::assets::sfx_data(), &st);
    for (a, v) in out.events {
        mem.write_ram(a, v); // 有序回写（$EA 末值语义自动成立）
    }
    for (a, v) in out.apu {
        mem.write_io(a, v);
    }
    if out.song_end {
        // EndOrLoopMusic（$B3F0）最小面：$062C=Repeat music byte（0=无重复）
        if mem.read_ram(0x062C) == 0 {
            mem.write_ram(0x068D, 0); // MusicContFlags←0（曲终收）
        } else {
            // 【r26 端口】repeat 径（asm $B3F0）：MusicLoopFlags($065D)←MusicContFlags($068D)
            let cont = mem.read_ram(0x068D);
            mem.write_ram(0x065D, cont);
        }
    }
}

#[cfg(test)]
mod w3_wiring_tests {
    use super::*;

    /// 接线-1：BC4B 门——$068D=0⇒零写；≠0⇒装配跑（$EA/块写落 mem）。
    /// 流取径=**基址反查**（Intro 真基址 B0B9/B000/B076/B115——r20）。
    #[test]
    fn continue_music_gate_and_assembly() {
        let mut m = crate::frame::FrameMemory::default();
        continue_music_face(&mut m);
        assert_eq!(m.read_ram(0x068D), 0, "无音路零写");
        m.write_ram(0x068D, 0x10);
        for (ch, (lo, hi)) in [(0xB9u8, 0xB0u8), (0x00, 0xB0), (0x76, 0xB0), (0x15, 0xB1)]
            .into_iter().enumerate()
        {
            m.write_ram(0x0630 + ch as u16 * 2, lo); // Intro 四通道真基址
            m.write_ram(0x0631 + ch as u16 * 2, hi);
            m.write_ram(0x0640 + ch as u16, 1); // delay=1（到点）
        }
        m.write_ram(0x062B, 0x0B);
        continue_music_face(&mut m);
        assert_ne!(m.read_ram(0x0638), 0, "InstrID 推进（流首字节已消费）");
    }

    /// 接线-2：$EA 有序回写（末值=事件流末写）。
    #[test]
    fn continue_music_ea_final_value() {
        let mut m = crate::frame::FrameMemory::default();
        m.write_ram(0x068D, 0x10);
        for (ch, (lo, hi)) in [(0xB9u8, 0xB0u8), (0x00, 0xB0), (0x76, 0xB0), (0x15, 0xB1)]
            .into_iter().enumerate()
        {
            m.write_ram(0x0630 + ch as u16 * 2, lo);
            m.write_ram(0x0631 + ch as u16 * 2, hi);
            m.write_ram(0x0640 + ch as u16, 1);
        }
        m.write_ram(0x062B, 0x0B);
        m.write_ram(0x0628, 0xB6); // duty SQ1
        continue_music_face(&mut m);
        // $EA 终值=末通道事件流末写（噪无 $EA⇒Tri 面）——mem 态=0/10/B6 族之一
        let ea = m.read_ram(0x00EA);
        assert!(ea == 0xB6 || ea == 0x10 || ea == 0x00, "$EA 落值={ea:02X}");
    }
}
