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

use crate::state::Manager;
use crate::sound::addrs;

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

    /// `$0683 SFXTriInitFlags`（岩浆伤径的 `SFX_SamusJump` 落点）。
    pub(crate) fn tri_init_flags(&self) -> u8 {
        self.read_ram(addrs::TRI_INIT)
    }

    pub(crate) fn set_tri_init_flags(&mut self, v: u8) {
        self.write_ram(addrs::TRI_INIT, v);
    }

    /// `$0684 SFXMultiInitFlags`（SFX + 部分音乐 init）。
    pub(crate) fn multi_init_flags(&self) -> u8 {
        self.read_ram(addrs::MULTI_INIT)
    }

    pub(crate) fn set_multi_init_flags(&mut self, v: u8) {
        self.write_ram(addrs::MULTI_INIT, v);
    }
}
