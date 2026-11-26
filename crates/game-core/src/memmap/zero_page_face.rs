//! 零页 `$0000-$0032`（51 B）typed 声明层——B1-b 第二批（卡 #81）。
//!
//! 【为什么在 `memmap/`】本层只做「字节 → 字段」的**声明与编解码**，不驱动任何读写；
//! 行为侧（`frame/tick.rs` 的 `FrameCount` 自增、`frame/nmi.rs` 的 `NMIStatus` 握手、
//! `input/` 的按键位面、`title/` 的 `TitleRoutine` 等）仍走各自裸地址，本模块不接管。
//! 这与 `memmap/mod.rs` 的分层口径一致：「语义面不在这里，地址常量只活在本层」。
//!
//! 【取证口径（A/B 级，逐字节）】全段 51 B 逐字节对过
//! `refs/m1disasm/SRC/constants_ram.asm` 的零页声明块（`.enum $12 export` 起，
//! `:483-527`）与 `crates/game-core/src/ram_map.rs` 的同址常量：
//!
//! | 地址 | asm 名（constants_ram.asm） | 证据级 |
//! |---|---|---|
//! | `$00` | `Temp00_PositionX`（`:473`） | B（**暂存别名**，见下注） |
//! | `$01` | `Temp01_FullEnergyTanks`（`:474`） | B（暂存别名） |
//! | `$02` | 无名（暂存） | — |
//! | `$03` | `Temp03_EnergyTankCount`（`:475`） | B（暂存别名） |
//! | `$04-$09` | 无名（暂存） | — |
//! | `$0A` | `Temp0A_Divisor`（`:470`） | B（暂存别名） |
//! | `$0B` | 无名（暂存） | — |
//! | `$0C-$0D` | `JumpEngineRoutinePtr` dw（`:477-478`） | A |
//! | `$0E-$10` | 无名（暂存） | — |
//! | `$11` | `Temp11_FrameIndex`（`:301`） | B（暂存别名） |
//! | `$12-$19` | Joy1/2Change、Joy1/2Status、Joy1/2Retrig、RetrigDelay1/2（`:487-494`） | A |
//! | `$1A-$20` | NMIStatus、PPUDataPending、PaletteDataPending、GameMode、MainRoutine、TitleRoutine、NextRoutine（`:496-502`） | A |
//! | `$21-$22` | `SpareMem21 dw`（`:504`） | A（asm 自述 spare） |
//! | `$23-$25` | CurrentBank、BankSwitchPending、MMC1CTRL_ZP（`:506-508`） | A |
//! | `$26-$27` | `SpareMem26 dw`（`:510`） | A（asm 自述 spare） |
//! | `$28` | `SwitchUpperBits`（`:512`） | A |
//! | `$29-$2C` | TimerDelay、Timer1、Timer2、Timer3（`:515-518`） | A |
//! | `$2D` | `FrameCount`（`:520`） | A |
//! | `$2E-$2F` | RandomNumber1/2（`:522-523`） | A |
//! | `$30` | `SpareMem30`（`:525`，asm 注「Written to, but never accessed.」） | A（spare） |
//! | `$31` | `GamePaused`（`:526`） | A |
//! | `$32` | `SpareMem32`（`:528`） | A（spare） |
//!
//! 【暂存区为什么只有别名】`$00-$11` 是 6502 零页暂存（`TempNN_xxx` 的 `xxx` 是该地址在
//! **某个具体调用点**的角色，换调用点含义就变）。因此本模块按「字节存在」声明，
//! 字段名 `<addr>_temp_<nn>` 只锁地址、不锁语义；真正语义归各自的调用点。**没有**把这些
//! 别名当成「该地址的唯一含义」写进字段（那样就是伪语义命名）。
//!
//! 【与既有面的边界】本段此前是 Undeclared 空洞（`nes_map.json` 记 51 B，refs 见
//! `ram_map.rs:38/46/82/89`）；`NMIStatus`/`FrameCount`/`PPUMASK` 等**另有**字节面访问器
//! （`frame/memory.rs` 的 `nmi_status()/frame_count()`），本面只声明布局，不替换那些访问器。

use crate::memmap::nesmap::{NesMapped, NesSpan, SpanKey};
use crate::memmap::storage::NesImage;

/// 段基址（CPU 地址）。
pub const BASE: u16 = 0x0000;
/// 段长（`$0000-$0032` 含端点）。
pub const FACE_LEN: u16 = 0x0033;

/// 命名地址表（`(偏移, asm 名, 来源)`）——测试与文档的单一真相源。
pub const SYMBOLS: [(u16, &str, &str); 41] = [
    (0x00, "Temp00_PositionX", "constants_ram.asm:473"),
    (0x01, "Temp01_FullEnergyTanks", "constants_ram.asm:474"),
    (0x03, "Temp03_EnergyTankCount", "constants_ram.asm:475"),
    (0x0A, "Temp0A_Divisor", "constants_ram.asm:470"),
    (0x0C, "JumpEngineRoutinePtr", "constants_ram.asm:477"),
    (0x11, "Temp11_FrameIndex", "constants_ram.asm:301"),
    (0x12, "Joy1Change", "constants_ram.asm:487"),
    (0x13, "Joy2Change", "constants_ram.asm:488"),
    (0x14, "Joy1Status", "constants_ram.asm:489"),
    (0x15, "Joy2Status", "constants_ram.asm:490"),
    (0x16, "Joy1Retrig", "constants_ram.asm:491"),
    (0x17, "Joy2Retrig", "constants_ram.asm:492"),
    (0x18, "RetrigDelay1", "constants_ram.asm:493"),
    (0x19, "RetrigDelay2", "constants_ram.asm:494"),
    (0x1A, "NMIStatus", "constants_ram.asm:496"),
    (0x1B, "PPUDataPending", "constants_ram.asm:497"),
    (0x1C, "PaletteDataPending", "constants_ram.asm:499"),
    (0x1D, "GameMode", "constants_ram.asm:500"),
    (0x1E, "MainRoutine", "constants_ram.asm:501"),
    (0x1F, "TitleRoutine", "constants_ram.asm:502"),
    (0x20, "NextRoutine", "constants_ram.asm:503"),
    (0x21, "SpareMem21", "constants_ram.asm:504"),
    (0x23, "CurrentBank", "constants_ram.asm:506"),
    (0x24, "BankSwitchPending", "constants_ram.asm:507"),
    (0x25, "MMC1CTRL_ZP", "constants_ram.asm:508"),
    (0x26, "SpareMem26", "constants_ram.asm:510"),
    (0x28, "SwitchUpperBits", "constants_ram.asm:512"),
    (0x29, "TimerDelay", "constants_ram.asm:515"),
    (0x2A, "Timer1", "constants_ram.asm:516"),
    (0x2B, "Timer2", "constants_ram.asm:517"),
    (0x2C, "Timer3", "constants_ram.asm:518"),
    (0x2D, "FrameCount", "constants_ram.asm:520"),
    (0x2E, "RandomNumber1", "constants_ram.asm:522"),
    (0x2F, "RandomNumber2", "constants_ram.asm:523"),
    (0x30, "SpareMem30", "constants_ram.asm:525"),
    (0x31, "GamePaused", "constants_ram.asm:526"),
    (0x32, "SpareMem32", "constants_ram.asm:528"),
    // 常用同址常量（Rust 侧既有名字，与 asm 名并存，供 grep 交叉核对）。
    (0x1A, "NMI_STATUS", "ram_map.rs:34"),
    (0x1D, "GAME_MODE", "ram_map.rs:42"),
    (0x1F, "TITLE_ROUTINE", "ram_map.rs:38"),
    (0x2D, "FRAME_COUNT", "ram_map.rs:50"),
];

/// 零页 `$0000-$0032` 的字段视图。
///
/// 字段顺序即内存顺序；每字段宽度与 `constants_ram.asm` 的 `db`/`dw` 一致，
/// 空格（暂存/备用）也显式占位，保证 `encode()` 与 `decode()` 逐字节可逆。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ZeroPageFace {
    /// `$00` `Temp00_PositionX`（暂存别名，调用点语义）。
    pub temp_00: u8,
    /// `$01` `Temp01_FullEnergyTanks`（暂存别名）。
    pub temp_01: u8,
    /// `$02` 无名暂存。
    pub temp_02: u8,
    /// `$03` `Temp03_EnergyTankCount`（暂存别名）。
    pub temp_03: u8,
    /// `$04-$09` 无名暂存（6 B）。
    pub temp_04_to_09: [u8; 6],
    /// `$0A` `Temp0A_Divisor`（暂存别名）。
    pub temp_0a: u8,
    /// `$0B` 无名暂存。
    pub temp_0b: u8,
    /// `$0C-$0D` `JumpEngineRoutinePtr`（小端）。
    pub jump_engine_routine_ptr: u16,
    /// `$0E-$10` 无名暂存（3 B）。
    pub temp_0e_to_10: [u8; 3],
    /// `$11` `Temp11_FrameIndex`（暂存别名）。
    pub temp_11: u8,
    /// `$12` 手柄 1 本帧变化位（bit7 A … bit0 Right）。
    pub joy1_change: u8,
    /// `$13` 手柄 2 本帧变化位。
    pub joy2_change: u8,
    /// `$14` 手柄 1 当前按下位。
    pub joy1_status: u8,
    /// `$15` 手柄 2 当前按下位。
    pub joy2_status: u8,
    /// `$16` 手柄 1 需重触发位。
    pub joy1_retrig: u8,
    /// `$17` 手柄 2 需重触发位。
    pub joy2_retrig: u8,
    /// `$18` 手柄 1 重触发延时计数。
    pub retrig_delay1: u8,
    /// `$19` 手柄 2 重触发延时计数。
    pub retrig_delay2: u8,
    /// `$1A` `0`=NMI 进行中，非 0=NMI 未在进行。
    pub nmi_status: u8,
    /// `$1B` PPU 数据待写标志。
    pub ppu_data_pending: u8,
    /// `$1C` 调色板待写标志（`1` → 第 `0` 组）。
    pub palette_data_pending: u8,
    /// `$1D` `0`=游戏中，`1`=标题/密码屏。
    pub game_mode: u8,
    /// `$1E` 主循环例程号（`5`=暂停，`3`=引擎运行）。
    pub main_routine: u8,
    /// `$1F` 标题屏子例程号。
    pub title_routine: u8,
    /// `$20` WaitTimer 到期后要跳转的下一个例程。
    pub next_routine: u8,
    /// `$21-$22` `SpareMem21`（asm 自述 spare）。
    pub spare_21: u16,
    /// `$23` 当前下半区 bank（0–7）。
    pub current_bank: u8,
    /// `$24` 待切 bank（实际 bank = 值 − 1）。
    pub bank_switch_pending: u8,
    /// `$25` 待写入 MMC1 寄存器 0 的位。
    pub mmc1_ctrl: u8,
    /// `$26-$27` `SpareMem26`（asm 自述 spare）。
    pub spare_26: u16,
    /// `$28` MMC1 寄存器 3 的高位暂存（正常恒 0）。
    pub switch_upper_bits: u8,
    /// `$29` 每帧倒计时（9→0）。
    pub timer_delay: u8,
    /// `$2A` 每帧递减。
    pub timer1: u8,
    /// `$2B` 每帧递减。
    pub timer2: u8,
    /// `$2C` 每 10 帧递减。
    pub timer3: u8,
    /// `$2D` 每帧自增，256 帧回绕。
    pub frame_count: u8,
    /// `$2E` 随机数。
    pub random_number1: u8,
    /// `$2F` 随机数。
    pub random_number2: u8,
    /// `$30` `SpareMem30`（asm 注：有写无读）。
    pub spare_30: u8,
    /// `$31` `0`=运行中，`1`=暂停。
    pub game_paused: u8,
    /// `$32` `SpareMem32`（asm 自述 spare）。
    pub spare_32: u8,
}

impl ZeroPageFace {
    /// 从 RAM 面解码（`bytes` 必须是本段起始处的切片，长度 ≥ [`FACE_LEN`]）。
    pub fn decode(bytes: &[u8]) -> Self {
        assert!(bytes.len() >= FACE_LEN as usize, "零页面需要 51 B 切片");
        let mut f = Self::default();
        f.temp_00 = bytes[0x00];
        f.temp_01 = bytes[0x01];
        f.temp_02 = bytes[0x02];
        f.temp_03 = bytes[0x03];
        f.temp_04_to_09.copy_from_slice(&bytes[0x04..0x0A]);
        f.temp_0a = bytes[0x0A];
        f.temp_0b = bytes[0x0B];
        f.jump_engine_routine_ptr = u16::from_le_bytes([bytes[0x0C], bytes[0x0D]]);
        f.temp_0e_to_10.copy_from_slice(&bytes[0x0E..0x11]);
        f.temp_11 = bytes[0x11];
        f.joy1_change = bytes[0x12];
        f.joy2_change = bytes[0x13];
        f.joy1_status = bytes[0x14];
        f.joy2_status = bytes[0x15];
        f.joy1_retrig = bytes[0x16];
        f.joy2_retrig = bytes[0x17];
        f.retrig_delay1 = bytes[0x18];
        f.retrig_delay2 = bytes[0x19];
        f.nmi_status = bytes[0x1A];
        f.ppu_data_pending = bytes[0x1B];
        f.palette_data_pending = bytes[0x1C];
        f.game_mode = bytes[0x1D];
        f.main_routine = bytes[0x1E];
        f.title_routine = bytes[0x1F];
        f.next_routine = bytes[0x20];
        f.spare_21 = u16::from_le_bytes([bytes[0x21], bytes[0x22]]);
        f.current_bank = bytes[0x23];
        f.bank_switch_pending = bytes[0x24];
        f.mmc1_ctrl = bytes[0x25];
        f.spare_26 = u16::from_le_bytes([bytes[0x26], bytes[0x27]]);
        f.switch_upper_bits = bytes[0x28];
        f.timer_delay = bytes[0x29];
        f.timer1 = bytes[0x2A];
        f.timer2 = bytes[0x2B];
        f.timer3 = bytes[0x2C];
        f.frame_count = bytes[0x2D];
        f.random_number1 = bytes[0x2E];
        f.random_number2 = bytes[0x2F];
        f.spare_30 = bytes[0x30];
        f.game_paused = bytes[0x31];
        f.spare_32 = bytes[0x32];
        f
    }

    /// 编码回 RAM 面（只写本段 51 B）。
    pub fn encode(&self, bytes: &mut [u8]) {
        assert!(bytes.len() >= FACE_LEN as usize, "零页面需要 51 B 切片");
        bytes[0x00] = self.temp_00;
        bytes[0x01] = self.temp_01;
        bytes[0x02] = self.temp_02;
        bytes[0x03] = self.temp_03;
        bytes[0x04..0x0A].copy_from_slice(&self.temp_04_to_09);
        bytes[0x0A] = self.temp_0a;
        bytes[0x0B] = self.temp_0b;
        let p = self.jump_engine_routine_ptr.to_le_bytes();
        bytes[0x0C] = p[0];
        bytes[0x0D] = p[1];
        bytes[0x0E..0x11].copy_from_slice(&self.temp_0e_to_10);
        bytes[0x11] = self.temp_11;
        bytes[0x12] = self.joy1_change;
        bytes[0x13] = self.joy2_change;
        bytes[0x14] = self.joy1_status;
        bytes[0x15] = self.joy2_status;
        bytes[0x16] = self.joy1_retrig;
        bytes[0x17] = self.joy2_retrig;
        bytes[0x18] = self.retrig_delay1;
        bytes[0x19] = self.retrig_delay2;
        bytes[0x1A] = self.nmi_status;
        bytes[0x1B] = self.ppu_data_pending;
        bytes[0x1C] = self.palette_data_pending;
        bytes[0x1D] = self.game_mode;
        bytes[0x1E] = self.main_routine;
        bytes[0x1F] = self.title_routine;
        bytes[0x20] = self.next_routine;
        let s = self.spare_21.to_le_bytes();
        bytes[0x21] = s[0];
        bytes[0x22] = s[1];
        bytes[0x23] = self.current_bank;
        bytes[0x24] = self.bank_switch_pending;
        bytes[0x25] = self.mmc1_ctrl;
        let s = self.spare_26.to_le_bytes();
        bytes[0x26] = s[0];
        bytes[0x27] = s[1];
        bytes[0x28] = self.switch_upper_bits;
        bytes[0x29] = self.timer_delay;
        bytes[0x2A] = self.timer1;
        bytes[0x2B] = self.timer2;
        bytes[0x2C] = self.timer3;
        bytes[0x2D] = self.frame_count;
        bytes[0x2E] = self.random_number1;
        bytes[0x2F] = self.random_number2;
        bytes[0x30] = self.spare_30;
        bytes[0x31] = self.game_paused;
        bytes[0x32] = self.spare_32;
    }

    /// 本段是否包含该 CPU 地址。
    pub fn contains(addr: u16) -> bool {
        (BASE..BASE + FACE_LEN).contains(&addr)
    }

    /// 单字节读（地址不在本段 ⇒ `None`）。
    pub fn get_byte(&self, addr: u16) -> Option<u8> {
        if !Self::contains(addr) {
            return None;
        }
        let mut buf = [0u8; FACE_LEN as usize];
        self.encode(&mut buf);
        Some(buf[(addr - BASE) as usize])
    }

    /// 单字节写（地址不在本段 ⇒ `false`）。
    pub fn set_byte(&mut self, addr: u16, v: u8) -> bool {
        if !Self::contains(addr) {
            return false;
        }
        let mut buf = [0u8; FACE_LEN as usize];
        self.encode(&mut buf);
        buf[(addr - BASE) as usize] = v;
        *self = Self::decode(&buf);
        true
    }
}

impl NesMapped for ZeroPageFace {
    /// 单一实体，无判别子（`None`）。
    const DISCRIMINATOR: Option<u16> = None;

    /// 整段一次性认领为 `Owner`（与 `object_slot_face.rs` 的逐槽细分不同：
    /// 零页是同一张派遣表，不需要再切）。
    const SPANS: &'static [(NesSpan, SpanKey)] =
        &[(NesSpan::value(BASE, FACE_LEN), SpanKey::Owner)];

    fn project(&self, out: &mut NesImage) {
        self.encode(&mut out.ram[BASE as usize & 0x7FF..(BASE as usize & 0x7FF) + FACE_LEN as usize]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn img() -> NesImage {
        NesImage {
            ram: [0x3C; 0x800],
            wram: [0u8; 0x2000],
        }
    }

    /// 几何：段长 51 B、覆盖 `$0000-$0032`，与 `nes_map.json` 记的空洞一致。
    #[test]
    fn face_geometry_matches_ledger_hole() {
        assert_eq!(BASE, 0x0000);
        assert_eq!(FACE_LEN, 0x0033);
        assert_eq!(FACE_LEN - BASE, 51);
        assert!(ZeroPageFace::contains(0x00));
        assert!(ZeroPageFace::contains(0x32));
        assert!(!ZeroPageFace::contains(0x33), "$0033 是 RoomPtr，不在本段");
    }

    /// 声明：只有一段 `Owner`，长度 51 B。
    #[test]
    fn spans_claim_exactly_the_face() {
        assert_eq!(ZeroPageFace::SPANS.len(), 1);
        let (span, key) = &ZeroPageFace::SPANS[0];
        assert_eq!(*span, NesSpan::value(0x0000, 0x0033));
        assert!(matches!(key, SpanKey::Owner));
    }

    /// 命名表：每个地址都能在 asm 里找到声明行，且偏移不越界。
    #[test]
    fn symbols_are_inside_face_and_cite_asm() {
        for (off, name, src) in SYMBOLS {
            assert!(off < FACE_LEN, "{name} 偏移 {off:#04X} 越界");
            assert!(
                src.starts_with("constants_ram.asm:") || src.starts_with("ram_map.rs:"),
                "{name} 缺证据指针：{src}"
            );
        }
        // 具名 37 个地址 + 4 个 Rust 同址别名 = 41 条
        assert_eq!(SYMBOLS.len(), 41);
    }

    /// 编解码逐字节可逆（全 0 / 全 FF / LCG 随机）。
    #[test]
    fn face_round_trip_is_byte_exact() {
        for seed in [0u8, 0xFF] {
            let src = [seed; FACE_LEN as usize];
            let f = ZeroPageFace::decode(&src);
            let mut back = [seed ^ 0x5A; FACE_LEN as usize];
            f.encode(&mut back);
            assert_eq!(back, src, "seed {seed:#04X} 往返不一致");
        }
        let mut x: u32 = 0x1234_5678;
        for _ in 0..4 {
            let mut src = [0u8; FACE_LEN as usize];
            for b in src.iter_mut() {
                x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                *b = (x >> 24) as u8;
            }
            let f = ZeroPageFace::decode(&src);
            let mut back = [0u8; FACE_LEN as usize];
            f.encode(&mut back);
            assert_eq!(back, src);
        }
    }

    /// `encode` 只碰本段 51 B，段外一个字节都不动。
    #[test]
    fn encode_touches_only_the_face() {
        let f = ZeroPageFace::decode(&[0xFF; FACE_LEN as usize]);
        let mut buf = [0x00u8; 0x100];
        f.encode(&mut buf);
        assert!(buf[..51].iter().all(|b| *b == 0xFF));
        assert!(buf[51..].iter().all(|b| *b == 0x00), "$0033 以后不许被改");
    }

    /// 字节访问器恰好覆盖 51 B（`$0033` 返回 `None`/`false`）。
    #[test]
    fn byte_accessors_cover_exactly_the_face() {
        let mut f = ZeroPageFace::default();
        for off in 0..FACE_LEN {
            assert!(f.set_byte(off, off as u8), "{off:#06X} 应可写");
            assert_eq!(f.get_byte(off), Some(off as u8));
        }
        assert_eq!(f.get_byte(FACE_LEN), None, "$0033 不属本段");
        assert!(!f.set_byte(FACE_LEN, 0xAA));
    }

    /// 关键字段边界（≥3 个字段）：具名字段的宽度与 asm `db`/`dw` 一致。
    #[test]
    fn named_field_widths_match_asm() {
        let mut f = ZeroPageFace::default();
        // $0C-$0D JumpEngineRoutinePtr dw：小端写入，两字节都变，$0B/$0E 不动。
        f.set_byte(0x0C, 0x34);
        f.set_byte(0x0D, 0x12);
        assert_eq!(f.jump_engine_routine_ptr, 0x1234);
        assert_eq!(f.get_byte(0x0B), Some(0));
        assert_eq!(f.get_byte(0x0E), Some(0));
        // $21-$22 SpareMem21 dw 同理。
        f.set_byte(0x21, 0xFF);
        f.set_byte(0x22, 0x00);
        assert_eq!(f.spare_21, 0x00FF);
        assert_eq!(f.get_byte(0x20), Some(0));
        assert_eq!(f.get_byte(0x23), Some(0));
        // $2D FrameCount 单字节：写 0xFF 只影响本字节，不与 $2E 混。
        f.set_byte(0x2D, 0xFF);
        assert_eq!(f.frame_count, 0xFF);
        assert_eq!(f.random_number1, 0x00);
        // $31 GamePaused 单字节边界（段尾前一字节）。
        f.set_byte(0x31, 0x01);
        assert_eq!(f.game_paused, 0x01);
        assert_eq!(f.spare_32, 0x00);
    }

    /// 投影：只写本段 51 B，其余保持哨兵值。
    #[test]
    fn project_writes_only_this_face() {
        let mut out = img();
        let f = ZeroPageFace::decode(&[0xA5; FACE_LEN as usize]);
        f.project(&mut out);
        assert!(out.ram[..51].iter().all(|b| *b == 0xA5));
        assert!(out.ram[51..].iter().all(|b| *b == 0x3C), "段外必须保持哨兵");
        assert!(out.wram.iter().all(|b| *b == 0x00), "零页面不碰 WRAM");
    }
}
