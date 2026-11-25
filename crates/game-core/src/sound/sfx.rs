//! `InitSFXData ; $B452` / `LoadSFXData ; $B368` 的 **typed 施加层**。
//!
//! 与 `mod.rs` 里既有的 `sfx_data_init()` / `*_sfx_init()` 的分工：
//! - 旧件是**纯函数**（入参 → `HandlerOutcome` 事件表），给差分对拍/域测用，不碰 `Manager`；
//! - 本模块把同一语义**施加到 `Manager`**，全部走 `sound::state` 的 typed 访问器
//!   （`sfx_length/sfx_frame/sfx_var/sfx_cont_flags/sfx_used/apu_register_ptr/...`），
//!   以及 `Manager::write_io`（APU `$4000-$4017`，是真实寄存器面、不是 RAM 业务字段）。
//!
//! ROM 依据：`refs/m1disasm/SRC/sound_engine.asm:292-332`（`LoadSFXData` 四入口）、
//! `:482-520`（`InitSFXData`）、`:493`（`UpdateContFlags`）。
//! 通道映射（A 级，模块内既有注释已按七案证反转）：ct 0=Noise `$400C`、1=SQ1 `$4000`、
//! 2=SQ2 `$4004`、3=Tri `$4008`。

use crate::frame::Manager;

/// SFX 四通道。`channel_type` 即 `ChannelType($065C)` / `ChannelType` 入参的取值。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SfxChannel {
    Noise,
    Sq1,
    Sq2,
    Tri,
}

impl SfxChannel {
    /// `ChannelType` 字节 → 通道。`>3` 在 ROM 里是「不使用任何通道」的早退（`InitSFXData`
    /// 的 `rts`），所以这里返回 `None`，由调用方决定是否落 `length`。
    pub fn from_channel_type(ct: u8) -> Option<Self> {
        match ct {
            0 => Some(Self::Noise),
            1 => Some(Self::Sq1),
            2 => Some(Self::Sq2),
            3 => Some(Self::Tri),
            _ => None,
        }
    }

    pub fn channel_type(self) -> u8 {
        match self {
            Self::Noise => 0,
            Self::Sq1 => 1,
            Self::Sq2 => 2,
            Self::Tri => 3,
        }
    }

    /// `LoadSFXData@*` 写的 `APURegisterPtr` 基址。
    pub fn apu_base(self) -> u16 {
        match self {
            Self::Noise => 0x400C,
            Self::Sq1 => 0x4000,
            Self::Sq2 => 0x4004,
            Self::Tri => 0x4008,
        }
    }
}

/// 一次 SFX 装载请求。`record_addr` 是 ROM `SFXData` 窗内的绝对址（`$B200..$B289`），
/// 与 asm 的 `ldy #<SFXData@Xxx`（高字节恒 `$B2`）等价。
#[derive(Clone, Copy, Debug)]
pub struct SfxLoad {
    pub channel: SfxChannel,
    pub record_addr: u16,
}

impl SfxLoad {
    pub fn new(channel: SfxChannel, record_addr: u16) -> Self {
        Self { channel, record_addr }
    }

    /// 4 字节记录（`SFXData` 表内）。
    pub fn record(&self) -> [u8; 4] {
        *crate::sound::sfx4(self.record_addr)
    }
}

/// `LoadSFXData` 的施加结果（读回面——测试/调用方不自己再按地址读）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SfxLoadOut {
    pub apu_base: u16,
    pub record: [u8; 4],
}

/// `LoadSFXData@* ; $B368/$B36C/$B370/$B374`：
/// `APURegisterPtr←基址`、`SFXDataPtr←$B2:low`，然后 4 字节**间接**写 `基址..基址+3`。
pub fn load_sfx_data(mem: &mut Manager, load: SfxLoad) -> SfxLoadOut {
    let base = load.channel.apu_base();
    mem.set_apu_register_ptr((base & 0xFF) as u8, (base >> 8) as u8);
    mem.set_sfx_data_ptr((load.record_addr & 0xFF) as u8, (load.record_addr >> 8) as u8);
    let record = load.record();
    for (i, b) in record.iter().enumerate() {
        mem.write_io(base + i as u16, *b);
    }
    SfxLoadOut { apu_base: base, record }
}

/// `InitSFXData ; $B452` 的施加结果。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SfxInitOutcome {
    pub channel_type: u8,
    pub frames: u8,
    pub load: Option<SfxLoadOut>,
    /// `UpdateContFlags` 之后的 `$0688+ct`。
    pub cont_flags: u8,
    /// `NoiseUsedBySFX[$0652+ct]`（= ct，ROM `sta $0652,x` 且 A=X=ct）。
    pub used: u8,
}

/// `InitSFXData ; $B452`：写 `length[$0660+ct]←frames`，按通道调 `LoadSFXData`，
/// `UpdateContFlags`（`cont[$0688+ct] = (cont & $00) | CurrentSoundFlags`——`and #$00`
/// 是 ROM 的死操作，照建），`used[$0652+ct]←ct`，清 `frame/Var0/Var1/Var2` 与 `$0607`。
///
/// `ct>3`：ROM 在 `length` 之后直接 `rts`（不装载、不清表）。
pub fn init_sfx_data(
    mem: &mut Manager,
    frames: u8,
    channel_type: u8,
    record_addr: u16,
    current_sound_flags: u8,
) -> SfxInitOutcome {
    let ct = channel_type;
    mem.set_sfx_length(ct, frames);
    let Some(channel) = SfxChannel::from_channel_type(ct) else {
        return SfxInitOutcome {
            channel_type: ct,
            frames,
            load: None,
            cont_flags: mem.sfx_cont_flags(ct),
            used: mem.sfx_used(ct),
        };
    };
    let out = load_sfx_data(mem, SfxLoad::new(channel, record_addr));
    // UpdateContFlags ; $B493（and #$00 之后 ora CurrentSoundFlags）。
    let cont = (mem.sfx_cont_flags(ct) & 0x00) | current_sound_flags;
    mem.set_sfx_cont_flags(ct, cont);
    mem.set_sfx_used(ct, ct);
    mem.set_sfx_frame(ct, 0);
    for slot in 0..3 {
        mem.set_sfx_var(slot, ct, 0);
    }
    mem.set_load_music_periods_flag(0);
    SfxInitOutcome {
        channel_type: ct,
        frames,
        load: Some(out),
        cont_flags: cont,
        used: ct,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `LoadSFXData` 取数基座：指针面 + 4 字节 APU 写序，四个通道各一例。
    #[test]
    fn load_sfx_data_writes_pointer_and_four_apu_bytes() {
        for (ct, base, addr, rec) in [
            (SfxChannel::Sq1, 0x4000u16, 0xB211u16, [0x35u8, 0x7F, 0x00, 0xB0]),
            (SfxChannel::Sq2, 0x4004, 0xB285, [0x7F, 0x7F, 0x30, 0xB2]),
            (SfxChannel::Tri, 0x4008, 0xB275, [0x7F, 0x7F, 0x40, 0x08]),
            (SfxChannel::Noise, 0x400C, 0xB285, [0x7F, 0x7F, 0x30, 0xB2]),
        ] {
            let mut m = Manager::default();
            let out = load_sfx_data(&mut m, SfxLoad::new(ct, addr));
            assert_eq!(out.apu_base, base);
            assert_eq!(out.record, rec, "记录 ${addr:04X} 的 4 字节");
            assert_eq!(m.apu_register_ptr(), ((base & 0xFF) as u8, (base >> 8) as u8));
            assert_eq!(m.sfx_data_ptr(), ((addr & 0xFF) as u8, (addr >> 8) as u8));
            for (i, b) in rec.iter().enumerate() {
                assert!(
                    m.apu_log.contains(&(base + i as u16, *b)),
                    "APU ${base:04X}+{i} 应写 {b:#04X}"
                );
            }
        }
    }

    /// 调用者 ①`ScrewAttackSFXInit ; $B52B`：5 帧 + Noise + `$B211`，之后 Var0 被清回 0。
    #[test]
    fn screw_attack_init_5_frames_noise() {
        let mut m = Manager::default();
        let o = init_sfx_data(&mut m, 0x05, 0, 0xB211, 0x44);
        assert_eq!(o.frames, 5);
        assert_eq!(m.sfx_length(0), 5, "length[$0660]");
        assert_eq!(m.sfx_cont_flags(0), 0x44, "cont[$0688]");
        assert_eq!(m.sfx_used(0), 0, "used[$0652]");
        assert_eq!(m.sfx_frame(0), 0, "frame[$0665]");
        assert_eq!(m.sfx_var(0, 0), 0, "Var0[$0670] 清 0");
        assert_eq!(m.load_music_sq12_periods_flag(), 0, "$0607 清 0");
        let out = o.load.expect("Noise 通道应装载");
        assert_eq!(out.apu_base, 0x400C);
        assert!(m.apu_log.contains(&(0x400C, 0x35)));
    }

    /// 调用者 ②`DoorOpenCloseSFXInit ; $B7AC`：0x1F 帧 + Tri(ct=3) + `$B285`。
    #[test]
    fn door_open_close_init_tri_0x1f_frames() {
        let mut m = Manager::default();
        let o = init_sfx_data(&mut m, 0x1F, 3, 0xB285, 0x80);
        assert_eq!(m.sfx_length(3), 0x1F, "length[$0663]");
        assert_eq!(m.sfx_cont_flags(3), 0x80, "cont[$068B]");
        assert_eq!(m.sfx_used(3), 3, "used[$0655]");
        assert_eq!(o.load.unwrap().apu_base, 0x4008, "Tri 基址（映射反转证）");
        assert!(m.apu_log.contains(&(0x4008, 0x7F)));
    }

    /// 调用者 ③`SamusDieSFXInit ; $B8D2`：0x0E 帧 + Tri(ct=3) + `$B275`。
    #[test]
    fn samus_die_init_tri_0x0e_frames() {
        let mut m = Manager::default();
        init_sfx_data(&mut m, 0x0E, 3, 0xB275, 0x01);
        assert_eq!(m.sfx_length(3), 0x0E);
        assert_eq!(m.sfx_frame(3), 0);
        assert!(m.apu_log.contains(&(0x4008, 0x7F)));
    }

    /// `ct>3`：ROM 写完 `length` 就 `rts` —— 不装载、不清表、不置 used。
    #[test]
    fn channel_type_out_of_range_stops_after_length() {
        let mut m = Manager::default();
        m.set_sfx_var(0, 4, 0x33);
        m.set_sfx_frame(4, 0x22);
        let o = init_sfx_data(&mut m, 0x11, 4, 0xB211, 0x44);
        assert!(o.load.is_none(), "无通道 ⇒ 不装载");
        assert_eq!(m.sfx_length(4), 0x11, "length 已写");
        assert_eq!(m.sfx_var(0, 4), 0x33, "表尾未清");
        assert_eq!(m.sfx_frame(4), 0x22, "frame 未清");
        assert!(m.apu_log.is_empty(), "无 APU 写");
    }
}
