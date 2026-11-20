//! 声引擎的**旗标面**（`$0602` / `$064D` / `$065C` / `$065E` / `$0680-$0684`）。
//!
//! 这些字节是 `SoundEngine ; 07:B3B4` 的输入面：谁置了哪一位，决定这一帧哪个
//! cycle 例程被调用。它们在 asm 里是全局（`constants_ram.asm:761-769/982/1069-1127`），
//! 收在这里的目的与 `world/object_globals.rs` 相同——**给出唯一读写点与语义名**，
//! 而不是把全局包装成「对象」。
//!
//! 语义（A 级，`refs/m1disasm/SRC/sound_engine.asm`）：
//! - `$0602 SFXPaused`：暂停音效态（`SoundEngine_GameIsPaused ; $B399` 自增驱动）
//! - `$064D CurrentSoundFlags`：位扫描暂存（`asl/bcs` **MSB 优先**，位 7 最高优先）
//! - `$065C ChannelType`：0=SQ1 / 1=SQ2 / 2=Tri / 3=Noise
//! - `$065E MusicInitIndex`：Multi 特例的曲目索引（双步：`FindMusicInitIndex` + `+8`）
//! - `$0680-$0684`：五个 init 旗（Noise/SQ1/SQ2/Tri/Multi）——`SoundEngine` 末尾
//!   `ClearInitFlags ; $B3D6` 一次清六旗

use crate::memmap::nesmap::{NesMapped, NesSpan, SpanKey};
use crate::sound::addrs;
use crate::state::Manager;

impl Manager {
    /// `$0602 SFXPaused`——暂停音效态。
    pub(crate) fn sfx_paused(&self) -> u8 {
        self.read_ram(addrs::PAUSED)
    }

    pub(crate) fn set_sfx_paused(&mut self, v: u8) {
        self.write_ram(addrs::PAUSED, v);
    }

    /// `$064D CurrentSoundFlags`——GetSoundRoutine 的位扫描暂存。
    pub(crate) fn current_sound_flags(&self) -> u8 {
        self.read_ram(addrs::CURRENT_FLAGS)
    }

    pub(crate) fn set_current_sound_flags(&mut self, v: u8) {
        self.write_ram(addrs::CURRENT_FLAGS, v);
    }

    /// `$065C ChannelType`——当前 cycle 的通道类型。
    pub(crate) fn channel_type(&self) -> u8 {
        self.read_ram(addrs::CHANNEL_TYPE)
    }

    pub(crate) fn set_channel_type(&mut self, v: u8) {
        self.write_ram(addrs::CHANNEL_TYPE, v);
    }

    /// `$065E MusicInitIndex`——Multi 特例曲目索引。
    pub(crate) fn music_init_index(&self) -> u8 {
        self.read_ram(addrs::MUSIC_INIT_INDEX)
    }

    pub(crate) fn set_music_init_index(&mut self, v: u8) {
        self.write_ram(addrs::MUSIC_INIT_INDEX, v);
    }

    /// `$0680 SFXNoiseInitFlags`（位 0 = 静音音乐，`SoundEngine` 分路依据）。
    pub(crate) fn noise_init_flags(&self) -> u8 {
        self.read_ram(addrs::NOISE_INIT)
    }

    pub(crate) fn set_noise_init_flags(&mut self, v: u8) {
        self.write_ram(addrs::NOISE_INIT, v);
    }

    /// `$0681 SFXSQ1InitFlags`。
    pub(crate) fn sq1_init_flags(&self) -> u8 {
        self.read_ram(addrs::SQ1_INIT)
    }

    pub(crate) fn set_sq1_init_flags(&mut self, v: u8) {
        self.write_ram(addrs::SQ1_INIT, v);
    }

    // ===== SFX 数据面访问器（task-SFX-1——hook.rs cont/init 臂的唯一裸读出口）=====

    /// `$061F HasBeamSFX`（bit7=LongBeam / bit0=IceBeam）。
    pub(crate) fn has_beam_sfx(&self) -> u8 {
        self.read_ram(addrs::HAS_BEAM_SFX)
    }

    /// `$061F HasBeamSFX` 写侧（冰束命中置 bit0；long beam 走 bit7）。
    pub(crate) fn set_has_beam_sfx(&mut self, v: u8) {
        self.write_ram(addrs::HAS_BEAM_SFX, v);
    }

    /// `SFXNoiseFrame + ct`（$0665+ct——multi 槽 ct=4→$0669）。
    pub(crate) fn sfx_frame(&self, ct: u8) -> u8 {
        self.read_ram(addrs::SFX_FRAME_BASE + ct as u16)
    }

    /// `SFXNoiseLength + ct`（$0660+ct——multi 槽 ct=4→$0664）。
    pub(crate) fn sfx_length(&self, ct: u8) -> u8 {
        self.read_ram(addrs::SFX_LENGTH_BASE + ct as u16)
    }

    /// `SFXNoiseVar{0,1,2} + ct`（$0670/$0674/$0678 + ct——slot 0..2）。
    pub(crate) fn sfx_var(&self, slot: u8, ct: u8) -> u8 {
        let base = match slot {
            0 => addrs::SFX_VAR0_BASE,
            1 => addrs::SFX_VAR1_BASE,
            _ => addrs::SFX_VAR2_BASE,
        };
        self.read_ram(base + ct as u16)
    }

    /// `SFXTriPeriodLow/High`（$0610/$0611）。
    pub(crate) fn sfx_tri_period(&self) -> (u8, u8) {
        (
            self.read_ram(addrs::TRI_PERIOD_LOW),
            self.read_ram(addrs::TRI_PERIOD_HIGH),
        )
    }

    /// `SFXTriChangeLow/High`（$0612/$0613）。
    pub(crate) fn sfx_tri_change(&self) -> (u8, u8) {
        (
            self.read_ram(addrs::TRI_CHANGE_LOW),
            self.read_ram(addrs::TRI_CHANGE_HIGH),
        )
    }

    /// `SFXTriDivisor`（$0616）。
    pub(crate) fn sfx_tri_divisor(&self) -> u8 {
        self.read_ram(addrs::TRI_DIVISOR)
    }

    /// Tri 四元组（PeriodLow/High + ChangeLow/High——tri cont 臂一次读全）。
    pub(crate) fn sfx_tri_period_and_change(&self) -> (u8, u8, u8, u8) {
        let (plo, phi) = self.sfx_tri_period();
        let (clo, chi) = self.sfx_tri_change();
        (plo, phi, clo, chi)
    }

    /// `$068D MusicContFlags`。
    /// `$0607 LoadMusicSQ1SQ2PeriodsFlag`（End*SFX 的 inc 计数器——复核 F6 读面）。
    pub(crate) fn load_music_sq12_periods_flag(&self) -> u8 {
        self.read_ram(addrs::LOAD_MUSIC_PERIODS_FLAG)
    }
    pub(crate) fn music_cont_flags(&self) -> u8 {
        self.read_ram(addrs::MUSIC_CONT_FLAGS)
    }

    /// `$0688+ct SFX*ContFlags`。
    pub(crate) fn sfx_cont_flags(&self, ct: u8) -> u8 {
        self.read_ram(addrs::CONT_FLAGS_BASE + ct as u16)
    }

    /// `$0683 SFXTriInitFlags`（岩浆伤径的 `SFX_SamusJump` 落点）。
    pub(crate) fn tri_init_flags(&self) -> u8 {
        self.sound_init.tri()
    }

    pub(crate) fn set_tri_init_flags(&mut self, v: u8) {
        self.sound_init.set_tri(v);
    }

    /// `$0684 SFXMultiInitFlags`（SFX + 部分音乐 init）。
    pub(crate) fn multi_init_flags(&self) -> u8 {
        self.sound_init.multi()
    }

    pub(crate) fn set_multi_init_flags(&mut self, v: u8) {
        self.sound_init.set_multi(v);
    }
}

/// `$0683/$0684` 两条 init 旗的 **typed 真源**。
///
/// 证据（A 级）：`refs/m1disasm/SRC/constants_ram.asm` 的 `SFXTriInitFlags ; 06:0683` /
/// `SFXMultiInitFlags ; 06:0684`；`sound_engine.asm` `SoundEngine ; 07:B3B4` 主循环
/// 以这两字节决定 TriInit/MultiInit cycle 是否本帧执行，末尾 `ClearInitFlags ; $B3D6`
/// 一次清六旗（其中含这两条）。两者在 asm 里是相邻的独立 `db`，故用两个 u8 字段。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SoundInitFlags {
    tri: u8,
    multi: u8,
}

impl SoundInitFlags {
    /// 迁入 typed 的地址清单（`restore_mss` 回灌/清零与 `snapshot_nes` 投影共用）。
    pub const ADDRS: [u16; 2] = [addrs::TRI_INIT, addrs::MULTI_INIT];

    pub fn tri(&self) -> u8 {
        self.tri
    }

    pub fn set_tri(&mut self, v: u8) {
        self.tri = v;
    }

    pub fn multi(&self) -> u8 {
        self.multi
    }

    pub fn set_multi(&mut self, v: u8) {
        self.multi = v;
    }

    /// 地址 → 字节（不在本域返回 `None`）。
    pub fn get_byte(&self, addr: u16) -> Option<u8> {
        if addr == Self::ADDRS[0] {
            Some(self.tri)
        } else if addr == Self::ADDRS[1] {
            Some(self.multi)
        } else {
            None
        }
    }

    /// 字节 → 地址（不在本域返回 `false`，调用方继续往下一域分派）。
    pub fn set_byte(&mut self, addr: u16, v: u8) -> bool {
        if addr == Self::ADDRS[0] {
            self.tri = v;
            true
        } else if addr == Self::ADDRS[1] {
            self.multi = v;
            true
        } else {
            false
        }
    }
}

impl NesMapped for SoundInitFlags {
    const DISCRIMINATOR: Option<u16> = None;

    /// 一段 2 B `Owner`（`$0683-$0684`）——asm 侧两条相邻的独立 init 旗。
    const SPANS: &'static [(NesSpan, SpanKey)] = &[(NesSpan::value(Self::ADDRS[0], 2), SpanKey::Owner)];

    fn project(&self, out: &mut crate::memmap::storage::NesImage) {
        let lo = (Self::ADDRS[0] & 0x7FF) as usize;
        out.ram[lo] = self.tri;
        out.ram[lo + 1] = self.multi;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::Manager;

    /// `$0683/$0684` 的写径/读径都落 typed；字节面不持有 typed 副本，投影同源。
    #[test]
    fn init_flags_are_typed() {
        let mut m = Manager::default();
        assert_eq!(m.tri_init_flags(), 0);
        m.set_tri_init_flags(0x08);
        assert_eq!(m.read_ram(0x0683), 0x08);
        assert_eq!(m.ram()[0x0683], 0, "字节面不持有 typed 字节");
        m.write_ram(0x0684, 0x10);
        assert_eq!(m.multi_init_flags(), 0x10);
        assert_eq!(m.read_ram(0x0684), 0x10);
        let img = m.snapshot_nes();
        assert_eq!(img.ram[0x0683], 0x08, "投影与 typed 同源");
        assert_eq!(img.ram[0x0684], 0x10);
    }

    /// `restore_mss` 回灌后 typed 与字节输入一致、storage 清零（清单覆盖本域）。
    #[test]
    fn init_flags_are_restored_and_cleared_from_mss() {
        let mut ram = vec![0u8; 0x800];
        ram[0x0683] = 0x80;
        ram[0x0684] = 0x01;
        let wram = vec![0u8; 0x2000];
        let m = Manager::restore_mss(&ram, &wram, crate::state::Registers::default());
        assert_eq!(m.tri_init_flags(), 0x80);
        assert_eq!(m.multi_init_flags(), 0x01);
        assert_eq!(m.ram()[0x0683], 0, "typed 格在 storage 清零");
        assert_eq!(m.ram()[0x0684], 0);
    }
}
