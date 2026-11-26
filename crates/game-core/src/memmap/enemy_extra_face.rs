//! game-core::memmap::enemy_extra_face —— **敌人额外面 `$6AFB-$6BD3`（140 B）的 typed 声明层**（B2-b）。
//!
//! 【这一面是什么】WRAM 里三段结构体的**尾部字节**，长期挂在 `nes-map` 的 Unmodeled 上：
//! - `EnsExtra`（`$6AF4-$6B53`）：6 槽 × 16 B，每槽前 7 B（status..animIndex）已由
//!   `crate::enemy::state` 的 `ensx` 以 typed `Owner` 声明，尾 9 B 在本面；
//! - `EnProjectilesExtra`（`$6B54-$6BB3`）：同上，6 槽 × 16 B，尾 9 B 在本面；
//! - `EnExplosionsExtra`（`$6BB4-$6BD3`）：4 槽 × 8 B，整槽在本面。
//!
//! 槽起点：尾段第 k 段 = `$6AFB + 16k`（k=0..11，故 k=5 到 `$6B4B`、k=6 起 `$6B54`、
//! k=11 到 `$6BAB..$6BB3`）；爆炸段 = `$6BB4 + 8k`（k=0..3，末槽到 `$6BD3`）。
//! 合计 12×9 + 4×8 = **140 B**。
//!
//! 【命名与证据】字段名不是本模块取的：`refs/m1disasm/SRC/constants_ram.asm:125-160`
//! 的 `.struct EnExtra`（16 字段）与 `:161-170` 的 `.struct EnExplosionExtra`（8 字段）
//! 给出顺序与名字；槽数由 `:1294-1296` 的 `instanceof` 声明（6/6/4）确定。
//! 逐段语义、写点与运行期分布见 `docs/spec/b2b-extra-6afb.md`（#96）。
//!
//! 【寻址形态】ROM 里这些字节**不按槽基址直接寻址**，而是统一走
//! `EnsExtra.0.<field>,x`（x = 槽相对 `Ens` 的偏移：敌 0x00-0x5F、弹 0x60-0xAF、
//! 爆 0xC0-0xF8），见 `docs/spec/b2b-extra-6afb.md` §3。故本面只提供
//! 「槽 + 字段」与「按绝对地址」两种访问，不假装它们是独立的全局变量。
//!
//! 【不变量】本面只做**声明与按需解码**：字节真源仍是 `storage`；`decode`/`project`
//! 以 `$6000-$7FFF` 的 WRAM 视图为准（索引 = 地址 − `$6000`），与 `snapshot_nes()` 同源。

use crate::memmap::nesmap::{NesMapped, NesSpan, SpanKey};
use crate::memmap::storage::NesImage;

/// 敌/弹额外面尾段起点（槽 0 的尾 9 B 首字节）。
///
/// 由 `EnsExtra` 槽基址 + 槽头 7 B 推导，不重复写字面量（地址真源仍在
/// `crate::enemy::state::ENS_EXTRA_BASE`）。
pub const TAIL_BASE: u16 = crate::enemy::state::ENS_EXTRA_BASE + 7;
/// 敌/弹额外面槽步长（每槽 16 B）。
pub const TAIL_STRIDE: u16 = 16;
/// 尾段槽数（EnsExtra 6 + EnProjectilesExtra 6）。
pub const TAIL_SLOTS: usize = 12;
/// 每段尾字节数（16 B 槽的后 9 B）。
pub const TAIL_LEN: usize = 9;

/// 爆炸额外面起点（槽 0 首字节）。
///
/// 直接引用 `crate::enemy::explosion::EXTRA_BASE`（同一基址的唯一定义处）。
pub const EXPL_BASE: u16 = crate::enemy::explosion::EXTRA_BASE;
/// 爆炸槽步长（每槽 8 B）。
pub const EXPL_STRIDE: u16 = 8;
/// 爆炸槽数。
pub const EXPL_SLOTS: usize = 4;
/// 每槽字节数（`EnExplosionExtra` 整槽）。
pub const EXPL_LEN: usize = 8;

/// 本面合计字节数（12×9 + 4×8 = 140）。
pub const FACE_LEN: usize = TAIL_SLOTS * TAIL_LEN + EXPL_SLOTS * EXPL_LEN;

/// `EnsExtra` / `EnProjectilesExtra` 的槽尾 9 B（`constants_ram.asm:125-160` 后 9 字段）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EnemyExtraTail {
    /// 槽所在 nametable 高字节（`hi`）。
    pub hi: u8,
    /// Y 小数值（`subPixelY`）。
    pub sub_pixel_y: u8,
    /// X 小数值（`subPixelX`）。
    pub sub_pixel_x: u8,
    /// Y 加速度（`accelY`）。
    pub accel_y: u8,
    /// X 加速度（`accelX`）。
    pub accel_x: u8,
    /// asm 自述 Unused（`spare1C`）；10 009 份采集里恒 0，保留位不赋语义。
    pub spare_1c: u8,
    /// 跳跃位移（`jumpDsplcmnt`，有符号）。
    pub jump_displacement: u8,
    /// 敌人类型（`type`，低 6 位 + specialAttribs 高位）。
    pub kind: u8,
    /// 敌人数据字节 1F（`data1F`，位含义按敌种不同）。
    pub data_1f: u8,
}

impl EnemyExtraTail {
    /// 本槽尾段字节数。
    pub const LEN: usize = TAIL_LEN;

    /// 按 asm 字段顺序展开成 9 B。
    pub fn to_bytes(self) -> [u8; TAIL_LEN] {
        [
            self.hi,
            self.sub_pixel_y,
            self.sub_pixel_x,
            self.accel_y,
            self.accel_x,
            self.spare_1c,
            self.jump_displacement,
            self.kind,
            self.data_1f,
        ]
    }

    /// 按 asm 字段顺序从 9 B 解出。
    pub fn from_bytes(b: &[u8; TAIL_LEN]) -> Self {
        Self {
            hi: b[0],
            sub_pixel_y: b[1],
            sub_pixel_x: b[2],
            accel_y: b[3],
            accel_x: b[4],
            spare_1c: b[5],
            jump_displacement: b[6],
            kind: b[7],
            data_1f: b[8],
        }
    }
}

/// `EnExplosionsExtra` 的整槽 8 B（`constants_ram.asm:161-170`）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EnemyExplosionExtra {
    /// 槽状态（0 = 空槽）。
    pub status: u8,
    /// Y 半径（`radiusY`）。
    pub radius_y: u8,
    /// X 半径（`radiusX`）。
    pub radius_x: u8,
    /// 当前动画帧（`animFrame`）。
    pub anim_frame: u8,
    /// 动画计时（`animDelay`）。
    pub anim_delay: u8,
    /// 动画复位帧（`resetAnimIndex`）。
    pub reset_anim_index: u8,
    /// 动画帧索引（`animIndex`）。
    pub anim_index: u8,
    /// 槽所在 nametable 高字节（`hi`）。
    pub hi: u8,
}

impl EnemyExplosionExtra {
    /// 本槽字节数。
    pub const LEN: usize = EXPL_LEN;

    /// 按 asm 字段顺序展开成 8 B。
    pub fn to_bytes(self) -> [u8; EXPL_LEN] {
        [
            self.status,
            self.radius_y,
            self.radius_x,
            self.anim_frame,
            self.anim_delay,
            self.reset_anim_index,
            self.anim_index,
            self.hi,
        ]
    }

    /// 按 asm 字段顺序从 8 B 解出。
    pub fn from_bytes(b: &[u8; EXPL_LEN]) -> Self {
        Self {
            status: b[0],
            radius_y: b[1],
            radius_x: b[2],
            anim_frame: b[3],
            anim_delay: b[4],
            reset_anim_index: b[5],
            anim_index: b[6],
            hi: b[7],
        }
    }
}

/// 敌人额外面（13 段，140 B）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EnemyExtraFace {
    /// 12 段尾 9 B（EnsExtra 6 + EnProjectilesExtra 6）。
    pub tails: [EnemyExtraTail; TAIL_SLOTS],
    /// 4 段爆炸额外面（每段 8 B）。
    pub explosions: [EnemyExplosionExtra; EXPL_SLOTS],
}

impl EnemyExtraFace {
    /// 尾段槽 k 的基址（`$6AFB + 16k`）。
    pub const fn tail_base(slot: usize) -> u16 {
        TAIL_BASE + TAIL_STRIDE * slot as u16
    }

    /// 爆炸槽 k 的基址（`$6BB4 + 8k`）。
    pub const fn explosion_base(slot: usize) -> u16 {
        EXPL_BASE + EXPL_STRIDE * slot as u16
    }

    /// 地址落在哪一段：`(段基址, 段长)`。
    pub fn segment_of(addr: u16) -> Option<(u16, usize)> {
        let a = addr as usize;
        for k in 0..TAIL_SLOTS {
            let base = Self::tail_base(k) as usize;
            if a >= base && a < base + TAIL_LEN {
                return Some((base as u16, TAIL_LEN));
            }
        }
        for k in 0..EXPL_SLOTS {
            let base = Self::explosion_base(k) as usize;
            if a >= base && a < base + EXPL_LEN {
                return Some((base as u16, EXPL_LEN));
            }
        }
        None
    }

    /// 地址是否属于本面。
    pub fn contains(addr: u16) -> bool {
        Self::segment_of(addr).is_some()
    }

    /// 槽尾视图（越界 `None`）。
    pub fn tail(&self, slot: usize) -> Option<&EnemyExtraTail> {
        self.tails.get(slot)
    }

    /// 槽尾可变视图（越界 `None`）。
    pub fn tail_mut(&mut self, slot: usize) -> Option<&mut EnemyExtraTail> {
        self.tails.get_mut(slot)
    }

    /// 爆炸槽视图（越界 `None`）。
    pub fn explosion(&self, slot: usize) -> Option<&EnemyExplosionExtra> {
        self.explosions.get(slot)
    }

    /// 爆炸槽可变视图（越界 `None`）。
    pub fn explosion_mut(&mut self, slot: usize) -> Option<&mut EnemyExplosionExtra> {
        self.explosions.get_mut(slot)
    }

    /// 按绝对地址读一字节（越界 `None`）。
    pub fn get_byte(&self, addr: u16) -> Option<u8> {
        let (base, _len) = Self::segment_of(addr)?;
        let off = (addr - base) as usize;
        if let Some(k) = Self::tail_slot_of(addr) {
            return Some(self.tails[k].to_bytes()[off]);
        }
        let k = Self::explosion_slot_of(addr)?;
        Some(self.explosions[k].to_bytes()[off])
    }

    /// 按绝对地址写一字节；越界返回 `false`（不静默丢弃）。
    pub fn set_byte(&mut self, addr: u16, value: u8) -> bool {
        let Some(base) = Self::segment_of(addr).map(|(b, _)| b) else {
            return false;
        };
        let off = (addr - base) as usize;
        if let Some(k) = Self::tail_slot_of(addr) {
            let mut b = self.tails[k].to_bytes();
            b[off] = value;
            self.tails[k] = EnemyExtraTail::from_bytes(&b);
            return true;
        }
        if let Some(k) = Self::explosion_slot_of(addr) {
            let mut b = self.explosions[k].to_bytes();
            b[off] = value;
            self.explosions[k] = EnemyExplosionExtra::from_bytes(&b);
            return true;
        }
        false
    }

    /// 地址若落在尾段，返回槽号。
    fn tail_slot_of(addr: u16) -> Option<usize> {
        (0..TAIL_SLOTS).find(|&k| {
            let base = Self::tail_base(k);
            addr >= base && addr < base + TAIL_LEN as u16
        })
    }

    /// 地址若落在爆炸段，返回槽号。
    fn explosion_slot_of(addr: u16) -> Option<usize> {
        (0..EXPL_SLOTS).find(|&k| {
            let base = Self::explosion_base(k);
            addr >= base && addr < base + EXPL_LEN as u16
        })
    }

    /// 从 8 KB WRAM 视图解出本面（索引 = 地址 − `$6000`；越界按 0 填）。
    pub fn decode(wram: &[u8]) -> Self {
        let mut face = Self::default();
        for k in 0..TAIL_SLOTS {
            let base = Self::tail_base(k) as usize - 0x6000;
            let mut b = [0u8; TAIL_LEN];
            for (i, byte) in b.iter_mut().enumerate() {
                *byte = wram.get(base + i).copied().unwrap_or(0);
            }
            face.tails[k] = EnemyExtraTail::from_bytes(&b);
        }
        for k in 0..EXPL_SLOTS {
            let base = Self::explosion_base(k) as usize - 0x6000;
            let mut b = [0u8; EXPL_LEN];
            for (i, byte) in b.iter_mut().enumerate() {
                *byte = wram.get(base + i).copied().unwrap_or(0);
            }
            face.explosions[k] = EnemyExplosionExtra::from_bytes(&b);
        }
        face
    }

    /// 把本面写进 8 KB WRAM 视图（越界跳过）。
    pub fn encode(&self, wram: &mut [u8]) {
        for k in 0..TAIL_SLOTS {
            let base = Self::tail_base(k) as usize - 0x6000;
            for (i, byte) in self.tails[k].to_bytes().iter().enumerate() {
                if let Some(dst) = wram.get_mut(base + i) {
                    *dst = *byte;
                }
            }
        }
        for k in 0..EXPL_SLOTS {
            let base = Self::explosion_base(k) as usize - 0x6000;
            for (i, byte) in self.explosions[k].to_bytes().iter().enumerate() {
                if let Some(dst) = wram.get_mut(base + i) {
                    *dst = *byte;
                }
            }
        }
    }
}

impl NesMapped for EnemyExtraFace {
    /// 无同址状态重解释 ⇒ 无判别字段。
    const DISCRIMINATOR: Option<u16> = None;

    /// 13 段全部是真相面（typed 字段即权威）。
    const SPANS: &'static [(NesSpan, SpanKey)] = &[
        (NesSpan::value(TAIL_BASE, TAIL_LEN as u16), SpanKey::Owner),
        (
            NesSpan::value(TAIL_BASE + TAIL_STRIDE, TAIL_LEN as u16),
            SpanKey::Owner,
        ),
        (
            NesSpan::value(TAIL_BASE + TAIL_STRIDE * 2, TAIL_LEN as u16),
            SpanKey::Owner,
        ),
        (
            NesSpan::value(TAIL_BASE + TAIL_STRIDE * 3, TAIL_LEN as u16),
            SpanKey::Owner,
        ),
        (
            NesSpan::value(TAIL_BASE + TAIL_STRIDE * 4, TAIL_LEN as u16),
            SpanKey::Owner,
        ),
        (
            NesSpan::value(TAIL_BASE + TAIL_STRIDE * 5, TAIL_LEN as u16),
            SpanKey::Owner,
        ),
        (
            NesSpan::value(TAIL_BASE + TAIL_STRIDE * 6, TAIL_LEN as u16),
            SpanKey::Owner,
        ),
        (
            NesSpan::value(TAIL_BASE + TAIL_STRIDE * 7, TAIL_LEN as u16),
            SpanKey::Owner,
        ),
        (
            NesSpan::value(TAIL_BASE + TAIL_STRIDE * 8, TAIL_LEN as u16),
            SpanKey::Owner,
        ),
        (
            NesSpan::value(TAIL_BASE + TAIL_STRIDE * 9, TAIL_LEN as u16),
            SpanKey::Owner,
        ),
        (
            NesSpan::value(TAIL_BASE + TAIL_STRIDE * 10, TAIL_LEN as u16),
            SpanKey::Owner,
        ),
        (
            NesSpan::value(TAIL_BASE + TAIL_STRIDE * 11, TAIL_LEN as u16),
            SpanKey::Owner,
        ),
        (
            NesSpan::value(EXPL_BASE, (EXPL_SLOTS * EXPL_LEN) as u16),
            SpanKey::Owner,
        ),
    ];

    /// 单向投影：把本面 140 B 写进 WRAM 视图。
    fn project(&self, out: &mut NesImage) {
        self.encode(&mut out.wram);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn face_covers_thirteen_segments_and_ends_at_6bd3() {
        assert_eq!(FACE_LEN, 140, "12×9 + 4×8 = 140 B");
        // 第一段首字节 / 末字节。
        assert_eq!(EnemyExtraFace::tail_base(0), 0x6AFB);
        assert!(EnemyExtraFace::contains(0x6AFB));
        assert!(EnemyExtraFace::contains(0x6B03), "槽 0 尾 9 B 末字节");
        assert_eq!(EnemyExtraFace::tail_base(5), 0x6B4B);
        // 扁平槽：`$6AFB + 16k`。k=6 的槽头是 `$6B54`（EnProjectilesExtra 槽 0 的
        // status..animIndex 7 B，由 `Enemy` 面登记），尾 9 B 从 `$6B5B` 起。
        assert_eq!(EnemyExtraFace::tail_base(6), 0x6B5B);
        assert!(!EnemyExtraFace::contains(0x6B54), "槽头 7 B 不属本面");
        assert_eq!(EnemyExtraFace::tail_base(11), 0x6BAB);
        // 爆炸段。
        assert_eq!(EnemyExtraFace::explosion_base(0), EXPL_BASE);
        assert!(EnemyExtraFace::contains(EXPL_BASE));
        assert!(EnemyExtraFace::contains(0x6BD3), "末槽末字节");
        assert_eq!(EnemyExtraFace::explosion_base(3), EXPL_BASE + 3 * EXPL_STRIDE);
        // 面外。
        assert!(!EnemyExtraFace::contains(0x6AFA));
        assert!(!EnemyExtraFace::contains(0x6BD4));
        assert!(
            !EnemyExtraFace::contains(crate::enemy::state::ENS_EXTRA_BASE),
            "槽头 7 B 属 Enemy 既有面"
        );
    }

    #[test]
    fn byte_roundtrip_and_out_of_range_rejected() {
        let mut face = EnemyExtraFace::default();
        assert!(face.set_byte(0x6AFB, 0x5A), "槽 0 hi");
        assert!(face.set_byte(0x6BB7, 0x07), "爆炸槽 0 animFrame = $6BB4+3");
        assert_eq!(face.get_byte(0x6AFB), Some(0x5A));
        assert_eq!(face.get_byte(0x6BB7), Some(0x07));
        assert_eq!(face.tail(0).unwrap().hi, 0x5A);
        assert_eq!(face.explosion(0).unwrap().anim_frame, 0x07);
        // 越界：面外地址/越界槽一律拒绝，不静默写。
        assert!(!face.set_byte(crate::enemy::state::ENS_EXTRA_BASE, 1));
        assert!(!face.set_byte(0x6BD4, 1));
        assert_eq!(face.get_byte(crate::enemy::state::ENS_EXTRA_BASE), None);
        assert_eq!(face.get_byte(0x6BD4), None);
        assert!(face.tail(TAIL_SLOTS).is_none());
        assert!(face.explosion(EXPL_SLOTS).is_none());
    }

    #[test]
    fn decode_encode_roundtrips_through_wram_view() {
        let mut wram = [0u8; 0x2000];
        let mut face = EnemyExtraFace::default();
        face.tails[0] = EnemyExtraTail {
            hi: 0x01,
            sub_pixel_y: 0x02,
            sub_pixel_x: 0x03,
            accel_y: 0x04,
            accel_x: 0x05,
            spare_1c: 0x00,
            jump_displacement: 0x06,
            kind: 0x09,
            data_1f: 0xC0,
        };
        face.explosions[3] = EnemyExplosionExtra {
            status: 0x02,
            radius_y: 0x04,
            radius_x: 0x04,
            anim_frame: 0x42,
            anim_delay: 0x01,
            reset_anim_index: 0x4D,
            anim_index: 0x4E,
            hi: 0x01,
        };
        face.encode(&mut wram);
        // 字节落点：槽 0 尾在 wram[0x0AFB]，爆炸槽 3 在 wram[0x0BCC]。
        assert_eq!(wram[0x0AFB], 0x01);
        assert_eq!(wram[0xB03], 0xC0, "data1F 在槽 0 尾第 9 字节");
        assert_eq!(wram[0xBCC], 0x02);
        assert_eq!(wram[0xBD3], 0x01, "爆炸槽 3 hi 在末字节");
        assert_eq!(EnemyExtraFace::decode(&wram), face);
    }

    #[test]
    fn spans_claim_exactly_the_face() {
        let total: usize = EnemyExtraFace::SPANS
            .iter()
            .map(|(s, _)| usize::from(s.len))
            .sum();
        assert_eq!(total, FACE_LEN, "13 段合计必须正好 140 B");
        for (span, key) in EnemyExtraFace::SPANS {
            assert!(
                matches!(key, SpanKey::Owner),
                "本面全部是真相面（{:?} 不是 Owner）",
                span.start
            );
            assert!(EnemyExtraFace::contains(span.start));
            assert!(EnemyExtraFace::contains(span.start + span.len - 1));
        }
    }
}
