//! game-core::memmap::wave_beam_face —— **wave beam 弹额外面（`$05D0-$05FF`）的 typed 声明层**
//! （B1-c 第三批：把 48 B `Undeclared` 升格为可审计的 `Owner` 段，卡 #83）。
//!
//! 【这一段是什么】wave beam 弹的**轨迹执行状态**：每个 wave 弹槽记住「走到轨迹串第几条
//! 指令、本指令已计时几帧、朝哪个方向」，供 `UpdateWaveBullet ; 07:D52C` 每帧推进弹速。
//!
//! 【为什么 asm 符号写在 `$0500`、本面却从 `$05D0` 起】
//! `constants_ram.asm:991-995` 的注释写明「3 slots of 16 bytes each ($05D0-$05FF)」，但三条
//! 符号声明是 `WeaponWaveInstrID = $0500` / `...Timer = $0501` / `WeaponWaveDir = $0502`，
//! 访问一律写成 `WeaponWaveDir,y` 形态。关键在 `y` 是什么：
//! - `SearchOpenSamusProjectileslot ; 07:D1F7`（`prg7_engine.asm:3818-3824`）从
//!   `ldy #SamusProjectiles.0 - Objects`（= `$D0`，`:3820`）开始、每次 `jsr Yplus16`（`:3824`） ⇒ **y 是 Objects
//!   页内的字节偏移 `$D0/$E0/$F0`**，不是槽号；
//! - 于是 `$0500 + y + {0,1,2}` 落在 `$05D0/$05E0/$05F0` 起的三处，字段偏移 `+0/+1/+2`
//!   ⇒ 实际地址正是 `$05D0-$05F2`，与注释的「3 × 16 B（`$05D0-$05FF`）」逐个对得上。
//! 所以本面 `BASE = 0x05D0`、`SLOT_LEN = 16`、`SLOTS = 3`，地址链 A 级可复算。
//!
//! 【字段证据（A 级：asm 注释 + 写点/读点）】
//! - `+0` `instr_id` = `WeaponWaveInstrID`（`constants_ram.asm:993`「instruction id for movement
//!   string of wave bullet trajectory」）：`CheckHorizontalWaveBulletFire ; 07:D359` 里
//!   `sta WeaponWaveInstrID,y`（`prg7_engine.asm:4067`，初值 `$00` 或 `$0C`，按 `tya / Adiv32`
//!   的进位选）；推进点 `MoveToNextWeaponWaveInstr ; 07:D51F`（`:4327`/`:4329` 两次 `inc`）；
//!   读点 `ldy WeaponWaveInstrID,x`（`:4349`，取轨迹串字节）。
//! - `+1` `instr_timer` = `WeaponWaveInstrTimer`（`:994`「count frames up to current instruction
//!   duration, then increment instr id」）：开火清零 `sta WeaponWaveInstrTimer,y`（`:4059`）；
//!   每帧 `cmp WeaponWaveInstrTimer,x` / `inc WeaponWaveInstrTimer,x`（`:4361`/`:4366`）；
//!   换指令时 `lda #$00 / sta WeaponWaveInstrTimer,x`（`:4331`）。
//! - `+2` `dir` = `WeaponWaveDir`（`:995`「bullet direction…#$00=right, #$01=left, #$02=up」）：
//!   写点 `sta WeaponWaveDir,y`（`:4053`，横向写 `SamusDir`、纵向由 `CheckVerticalWaveBulletFire ; 07:D38A`（`:4074`）先 `lda #$02` 再跳同一写点）；
//!   读点 `lda WeaponWaveDir,x`（`:4340` 选轨迹指针表、`:4386` 判左右翻转 x 速度）。
//! - `+3..+15`：**asm 未命名**（块内只声明三字段，无 `spare` 符号）⇒ 按
//!   `spare_todo: [u8; 13]` 原样保留，标 C 级假设（未取证），不造伪语义名。
//!
//! 【当前 Rust 侧对该段的使用】0 处。`grep -rn "0x05[dD][0-9a-fA-F]" crates/game-core/src/`
//! 无命中；wave beam 弹轨迹尚未端口（`weapon/` 下只有 bomb/projectile，`projectile.rs` 的
//! 弹状态机里没有 wave 分支）。账本里这 48 B 此前的 refs 只有 `asm:…sym` 两行、无 `*.rs`
//! 指针，与「无行为访问点」一致。因此本卡**没有需要收敛的裸地址**：本次只做声明层，
//! 行为侧零改动（将来端口 `UpdateWaveBullet ; 07:D52C` 时按本面访问器读写）。
//!
//! 【分层口径】与 `tile_blast_face` 相同：地址常量与 codec 只活在本层；字节真源仍是
//! `storage`（`Manager`），本结构是按需解码视图，不持有跨帧状态。

use crate::memmap::nesmap::{NesMapped, NesSpan, SpanKey};
use crate::memmap::storage::NesImage;

/// 面基址：`$05D0`（`constants_ram.asm:991-992` 注释的三槽起点；符号基址 `$0500` + 槽偏移
/// `$D0`，见模块头）。
pub const BASE: u16 = 0x05D0;
/// 槽长：16 B（`3 slots of 16 bytes each`）。
pub const SLOT_LEN: u16 = 16;
/// 槽数：3（Samus 弹槽 `SamusProjectiles.0..2` 的偏移 `$D0/$E0/$F0`，`prg7_engine.asm:3820-3824`）。
pub const SLOTS: usize = 3;
/// 面长：3 × 16 = 48 B（`$05D0-$05FF`）。
pub const FACE_LEN: u16 = (SLOTS as u16) * SLOT_LEN;

// 槽内字段偏移（与 asm 的 `WeaponWave*,y` 访问逐项对齐；符号基址 `$0500` + `+off`）。
const F_INSTR_ID: u16 = 0;
const F_INSTR_TIMER: u16 = 1;
const F_DIR: u16 = 2;
const F_SPARE_TODO: u16 = 3;
const SPARE_LEN: usize = 13;

/// 单槽（16 B）的 typed 表示。
///
/// 前 3 B 是 asm 具名字段（A 级），其余 13 B asm 无名（C 级假设：保留位，未取证）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WaveBeamSlot {
    /// +0 `WeaponWaveInstrID`：当前轨迹指令号（movement string 下标；开火初值 `$00`/`$0C`）。
    pub instr_id: u8,
    /// +1 `WeaponWaveInstrTimer`：当前指令已持续帧数（等于指令时长时换下一条指令）。
    pub instr_timer: u8,
    /// +2 `WeaponWaveDir`：弹方向（`#$00`=右 / `#$01`=左 / `#$02`=上，asm 注释原文）。
    pub dir: u8,
    /// +3..+15：asm 无名、C 级假设、未取证 —— 保留原字节，不赋予猜测语义。
    pub spare_todo: [u8; SPARE_LEN],
}

impl WaveBeamSlot {
    /// 槽内字节解码（`bytes` 长度必须 ≥ [`SLOT_LEN`]）。
    pub fn decode(bytes: &[u8]) -> Self {
        debug_assert!(bytes.len() >= SLOT_LEN as usize, "槽解码需要 16 B");
        Self {
            instr_id: bytes[usize::from(F_INSTR_ID)],
            instr_timer: bytes[usize::from(F_INSTR_TIMER)],
            dir: bytes[usize::from(F_DIR)],
            spare_todo: {
                let mut out = [0u8; SPARE_LEN];
                let start = usize::from(F_SPARE_TODO);
                out.copy_from_slice(&bytes[start..start + SPARE_LEN]);
                out
            },
        }
    }

    /// 槽内字节编码（`bytes` 长度必须 ≥ [`SLOT_LEN`]）。
    pub fn encode(&self, bytes: &mut [u8]) {
        debug_assert!(bytes.len() >= SLOT_LEN as usize, "槽编码需要 16 B");
        bytes[usize::from(F_INSTR_ID)] = self.instr_id;
        bytes[usize::from(F_INSTR_TIMER)] = self.instr_timer;
        bytes[usize::from(F_DIR)] = self.dir;
        let start = usize::from(F_SPARE_TODO);
        bytes[start..start + SPARE_LEN].copy_from_slice(&self.spare_todo);
    }
}

/// wave beam 弹额外面（3 槽 × 16 B）的 typed 表示。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WaveBeamFace {
    slots: [WaveBeamSlot; SLOTS],
}

impl WaveBeamFace {
    /// 从 2KB RAM 镜像面解码（`$05D0-$05FF`，地址按 `& 0x7FF` 归一）。
    pub fn decode(ram: &[u8]) -> Self {
        let mut slots = [WaveBeamSlot::default(); SLOTS];
        for (i, slot) in slots.iter_mut().enumerate() {
            let base = usize::from(BASE & 0x7FF) + i * usize::from(SLOT_LEN);
            *slot = WaveBeamSlot::decode(&ram[base..base + usize::from(SLOT_LEN)]);
        }
        Self { slots }
    }

    /// 编码回 2KB RAM 镜像面（只写 `$05D0-$05FF`）。
    pub fn encode(&self, ram: &mut [u8]) {
        for (i, slot) in self.slots.iter().enumerate() {
            let base = usize::from(BASE & 0x7FF) + i * usize::from(SLOT_LEN);
            slot.encode(&mut ram[base..base + usize::from(SLOT_LEN)]);
        }
    }

    /// 槽视图（越界返回 `None`，不 panic）。
    pub fn slot(&self, index: usize) -> Option<&WaveBeamSlot> {
        self.slots.get(index)
    }

    /// 槽可变视图（越界返回 `None`）。
    pub fn slot_mut(&mut self, index: usize) -> Option<&mut WaveBeamSlot> {
        self.slots.get_mut(index)
    }

    /// 槽 → 绝对地址（`$05D0 + index*16`，即 `$D0/$E0/$F0` 三个 Samus 弹槽偏移）。
    pub fn slot_addr(index: usize) -> u16 {
        BASE + (index as u16) * SLOT_LEN
    }

    /// 地址 → 槽号（不在本面返回 `None`）。
    pub fn slot_of(addr: u16) -> Option<usize> {
        let a = addr & 0x7FF;
        if !(BASE..BASE + FACE_LEN).contains(&a) {
            return None;
        }
        Some(usize::from((a - BASE) / SLOT_LEN))
    }

    /// 地址 → 字节（不在本面返回 `None`）；供将来接入 `Manager::read_ram` 分派用。
    pub fn get_byte(&self, addr: u16) -> Option<u8> {
        let index = Self::slot_of(addr)?;
        let off = usize::from((addr & 0x7FF) - Self::slot_addr(index));
        let mut bytes = [0u8; SLOT_LEN as usize];
        self.slots[index].encode(&mut bytes);
        Some(bytes[off])
    }

    /// 字节 → 地址（不在本面返回 `false`，调用方继续往下一域分派）。
    pub fn set_byte(&mut self, addr: u16, v: u8) -> bool {
        let Some(index) = Self::slot_of(addr) else {
            return false;
        };
        let off = usize::from((addr & 0x7FF) - Self::slot_addr(index));
        let mut bytes = [0u8; SLOT_LEN as usize];
        self.slots[index].encode(&mut bytes);
        bytes[off] = v;
        self.slots[index] = WaveBeamSlot::decode(&bytes);
        true
    }
}

impl NesMapped for WaveBeamFace {
    const DISCRIMINATOR: Option<u16> = None;

    /// 3 段 × 16 B 的 `Owner`（`$05D0-$05FF`）。逐槽一段：槽是 asm 的数组元素
    /// （`WeaponWave*,$D0/$E0/$F0`），槽内字段不拆段。
    const SPANS: &'static [(NesSpan, SpanKey)] = &[
        (NesSpan::value(BASE, SLOT_LEN), SpanKey::Owner),
        (NesSpan::value(BASE + 1 * SLOT_LEN, SLOT_LEN), SpanKey::Owner),
        (NesSpan::value(BASE + 2 * SLOT_LEN, SLOT_LEN), SpanKey::Owner),
    ];

    fn project(&self, out: &mut NesImage) {
        self.encode(&mut out.ram);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 面几何必须等于 asm 注释：3 槽 × 16 B、起点 `$05D0`、终点 `$05FF`。
    #[test]
    fn face_geometry_matches_asm_slots() {
        assert_eq!(BASE, 0x05D0, "constants_ram.asm:992「3 slots of 16 bytes each ($05D0-$05FF)」");
        assert_eq!(SLOT_LEN, 16, "同上「16 bytes each」");
        assert_eq!(SLOTS, 3, "同上「3 slots」；对应 Samus 弹槽偏移 $D0/$E0/$F0");
        assert_eq!(FACE_LEN, 48);
        assert_eq!(WaveBeamFace::slot_addr(0), 0x05D0);
        assert_eq!(WaveBeamFace::slot_addr(1), 0x05E0);
        assert_eq!(WaveBeamFace::slot_addr(2), 0x05F0);
        assert_eq!(WaveBeamFace::slot_addr(2) + SLOT_LEN, 0x0600);
    }

    /// 符号基址与槽偏移的换算：`$0500 + $D0/$E0/$F0` 必须正好落在这三段。
    #[test]
    fn asm_symbol_base_plus_projectile_offset_lands_on_these_slots() {
        // constants_ram.asm:993-995 的符号基址。
        const SYM_BASE: u16 = 0x0500;
        // SearchOpenSamusProjectileslot: ldy #SamusProjectiles.0-Objects（$D0），每次 Yplus16。
        for (k, off) in [0xD0u16, 0xE0, 0xF0].into_iter().enumerate() {
            assert_eq!(SYM_BASE + off, WaveBeamFace::slot_addr(k), "槽 {k} 的 asm 访问地址");
            assert!(WaveBeamFace::slot_of(SYM_BASE + off).is_some());
        }
    }

    /// 声明表必须恰好是 3 段 `Owner`、连续覆盖 `$05D0-$0600`、无重叠。
    #[test]
    fn spans_are_three_contiguous_owner_slots() {
        let spans = <WaveBeamFace as NesMapped>::SPANS;
        assert_eq!(spans.len(), SLOTS);
        let mut expect = BASE;
        for (span, key) in spans {
            assert_eq!(span.start, expect, "槽段必须连续");
            assert_eq!(span.len, SLOT_LEN);
            assert!(matches!(key, SpanKey::Owner));
            expect += span.len;
        }
        assert_eq!(expect, BASE + FACE_LEN);
    }

    /// 字段偏移：三个具名字段各占 1 B、`+0/+1/+2`，其余 13 B 原样保留。
    #[test]
    fn field_offsets_match_asm_symbols() {
        let slot = WaveBeamSlot {
            instr_id: 0x11,
            instr_timer: 0x22,
            dir: 0x01,
            spare_todo: [0xB0, 0xB1, 0xB2, 0xB3, 0xB4, 0xB5, 0xB6, 0xB7, 0xB8, 0xB9, 0xBA, 0xBB, 0xBC],
        };
        let mut bytes = [0u8; 16];
        slot.encode(&mut bytes);
        assert_eq!(bytes[0], 0x11, "+0 WeaponWaveInstrID");
        assert_eq!(bytes[1], 0x22, "+1 WeaponWaveInstrTimer");
        assert_eq!(bytes[2], 0x01, "+2 WeaponWaveDir");
        assert_eq!(&bytes[3..], &slot.spare_todo, "+3..+15 原样保留");
        assert_eq!(WaveBeamSlot::decode(&bytes), slot, "decode∘encode 必须还原");
    }

    /// ≥3 字段边界：写一个具名字段不得串到相邻字段/保留区。
    #[test]
    fn named_field_widths_are_one_byte() {
        let mut face = WaveBeamFace::default();
        // 槽 1 的 +0/+1/+2 各写一次，只动 3 B。
        assert!(face.set_byte(0x05E0, 0x07));
        assert!(face.set_byte(0x05E1, 0x09));
        assert!(face.set_byte(0x05E2, 0x02));
        let slot = *face.slot(1).expect("槽 1 存在");
        assert_eq!((slot.instr_id, slot.instr_timer, slot.dir), (0x07, 0x09, 0x02));
        assert_eq!(slot.spare_todo, [0u8; SPARE_LEN], "保留区不得被具名字段写入");
        // 边界：`$05DF` 属槽 0 保留区、`$05F3` 属槽 1 保留区，写入不得越槽。
        assert!(face.set_byte(0x05DF, 0xAA));
        assert!(face.set_byte(0x05F3, 0xBB));
        assert_eq!(face.get_byte(0x05DF), Some(0xAA));
        assert_eq!(face.get_byte(0x05F3), Some(0xBB));
        assert_eq!(face.get_byte(0x05E0), Some(0x07), "槽 1 +0 不被邻槽写入影响");
    }

    /// 任意字节面往返：`encode(decode(ram)) == ram`（含保留区，不丢信息）。
    #[test]
    fn face_round_trip_is_byte_exact() {
        let mut ram = [0u8; 0x800];
        for case in 0..6u32 {
            let mut seed = 0x9E37_79B9u32 ^ case;
            for byte in ram.iter_mut() {
                *byte = match case {
                    0 => 0x00,
                    1 => 0xFF,
                    _ => {
                        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                        (seed >> 24) as u8
                    }
                };
            }
            let face = WaveBeamFace::decode(&ram);
            let mut out = ram;
            face.encode(&mut out);
            assert_eq!(out, ram, "case {case}：面内字节必须逐字节往返");
        }
    }

    /// 面外字节不受影响：`encode` 只写 `$05D0-$05FF`。
    #[test]
    fn encode_touches_only_the_face() {
        let mut ram = [0x5Au8; 0x800];
        let face = WaveBeamFace::decode(&ram);
        ram[usize::from(BASE & 0x7FF) - 1] = 0x11; // $05CF（TileBlasts 尾槽 type）
        ram[usize::from(BASE & 0x7FF) + usize::from(FACE_LEN)] = 0x22; // $0600（MusicSQ1PeriodLow）
        let (before_lo, before_hi) = (
            ram[usize::from(BASE & 0x7FF) - 1],
            ram[usize::from(BASE & 0x7FF) + usize::from(FACE_LEN)],
        );
        face.encode(&mut ram);
        assert_eq!(ram[usize::from(BASE & 0x7FF) - 1], before_lo, "$05CF 不得被写");
        assert_eq!(
            ram[usize::from(BASE & 0x7FF) + usize::from(FACE_LEN)],
            before_hi,
            "$0600 不得被写"
        );
        let after = WaveBeamFace::decode(&ram);
        for i in 0..SLOTS {
            assert_eq!(after.slots[i].instr_id, 0x5A, "槽 {i} 仍应是 0x5A 面");
        }
    }

    /// `get_byte`/`set_byte` 覆盖整面且不越界。
    #[test]
    fn byte_accessors_cover_exactly_the_face() {
        let mut face = WaveBeamFace::default();
        for addr in BASE..BASE + FACE_LEN {
            assert!(face.get_byte(addr).is_some(), "{addr:#06X} 应在面内");
            assert!(face.set_byte(addr, (addr & 0xFF) as u8), "{addr:#06X} 应可写");
        }
        for addr in BASE..BASE + FACE_LEN {
            assert_eq!(face.get_byte(addr), Some((addr & 0xFF) as u8), "{addr:#06X} 值回读");
        }
        assert_eq!(face.get_byte(BASE - 1), None, "$05CF 属 TileBlasts，不属本面");
        assert_eq!(face.get_byte(BASE + FACE_LEN), None, "$0600 属声引擎，不属本面");
        assert!(!face.set_byte(BASE + FACE_LEN, 1), "面外写必须被拒");
        // 镜像归一：`$0DD0` 与 `$05D0` 是同一格（2KB 镜像面）。
        assert_eq!(face.get_byte(0x0DD0), face.get_byte(BASE));
    }

    /// `project` 只写本面字节（`NesMapped` 契约）。
    #[test]
    fn project_writes_only_this_face() {
        let mut ram = [0xA5u8; 0x800];
        let face = WaveBeamFace::decode(&ram);
        let mut img = NesImage {
            ram: [0x3Cu8; 0x800],
            wram: [0u8; 0x2000],
        };
        face.project(&mut img);
        for addr in 0..0x800usize {
            let in_face = (usize::from(BASE)..usize::from(BASE + FACE_LEN)).contains(&addr);
            if in_face {
                assert_eq!(img.ram[addr], 0xA5, "${addr:#06X} 属于本面，应被投影");
            } else {
                assert_eq!(img.ram[addr], 0x3C, "${addr:#06X} 不属于本面，不得被写");
            }
        }
    }
}
