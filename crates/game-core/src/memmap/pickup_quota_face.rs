//! 掉落/拾取临时字段族 `$0093-$0096`（4 B）的 typed 声明层（卡 #141 / #134 收口）。
//!
//! 【这一面是什么】`constants_ram.asm:697-704` 里四个**具名**的掉落配额字节：
//! 导弹/能量拾取的**上限**与**当前已取**计数。ROM 用它们做「每次进门重置、按随机数重算上限、
//! 取满后不再掉落」的节流；`prg7_engine.asm:6035-6040`（Tourian 的 `LDD5B`）会把四个字节
//! 一起写回 `0/1`，正是 #117 实现的那段配额复位。
//!
//! 此前这四个字节落在 `data/status/nes_map.json` 的 `Undeclared`（`$0091-$00FB` 那段 107 B
//! 大段里）。本模块只做两件事——①把 asm 的具名布局写成 typed 结构；②把这 4 B 登记成
//! `Owner`。**行为零变化**：`frame/tick.rs`、`samus/physics.rs`、`samus/tourian` 的读写路径
//! 一行不动（现有的 `frame/memory.rs` 访问器继续是唯一写入面）。
//!
//! 【逐字段读写点（A 级：asm 具名 + 指令上下文）】
//!
//! | 地址 | 名 | 写 | 读 |
//! |---|---|---|---|
//! | `$0093` | `MissilePickupQtyMax` | `areas_common.asm:1333`（进门按 `RandomNumber1&$0F` 重算上限）、`prg7_engine.asm:2004`（MoreInit 写 `$2F`/`$27`）、`prg7_engine.asm:6039`（`LDD5B` 写 1） | `prg7_engine.asm:5991`（`ldy` 比较上限） |
//! | `$0094` | `EnergyPickupQtyMax` | `areas_common.asm:1337`（`(r&$0F)<<1 \| $40`）、`prg7_engine.asm:2005`、`prg7_engine.asm:6040` | `prg7_engine.asm:6005` |
//! | `$0095` | `MissilePickupQtyCur` | `areas_common.asm:1328`（进门清零）、`prg7_engine.asm:5998`（拾取 `inc`）、`prg7_engine.asm:6036`（`LDD5B` 清零） | `prg7_engine.asm:5992` |
//! | `$0096` | `EnergyPickupQtyCur` | `areas_common.asm:1329`（进门清零）、`prg7_engine.asm:6009`（拾取 `inc`）、`prg7_engine.asm:6035`（`LDD5B` 清零） | `prg7_engine.asm:6006` |
//!
//! 【为什么能升 `Owner` 而不是 `Unmodeled`】四个字节都有 `constants_ram.asm` 的具名声明，
//! 且各有明确的写点与读点（上表逐条给了 bank:行号），不是「无名地址段」；也不是
//! `world/object.rs` 那种「整段未 typed 面」——它们是零页里的独立字节。
//!
//! 【没收录的相邻字节（有意的）】`$0091 DoorPaletteChangeDir`、`$0092 MetroidOnSamus`、
//! `$0097 CannonIndex`、`$0098 MotherBrainStatus` 及 `$0099+` 的 union 区不属于「掉落/拾取
//! 临时字段」，留给后续批次；`$0091-$00FB` 大段里其余无名/备用字节同样不并入本面。

use crate::memmap::nesmap::{NesMapped, NesSpan, SpanKey};
use crate::memmap::storage::NesImage;

/// 本面基址（`$0093`，四个字节连续）。
pub const BASE: u16 = 0x0093;

/// 本面声明总长（4 B）。
pub const FACE_LEN: usize = 4;

/// 具名地址表：`(地址, asm 名, 证据源、含读写点)`。
pub const SYMBOLS: [(u16, &str, &str); 4] = [
    (
        0x0093,
        "MissilePickupQtyMax",
        "constants_ram.asm:697；写 areas_common.asm:1333 / prg7_engine.asm:2004,6039；读 prg7_engine.asm:5991",
    ),
    (
        0x0094,
        "EnergyPickupQtyMax",
        "constants_ram.asm:699；写 areas_common.asm:1337 / prg7_engine.asm:2005,6040；读 prg7_engine.asm:6005",
    ),
    (
        0x0095,
        "MissilePickupQtyCur",
        "constants_ram.asm:701；写 areas_common.asm:1328 / prg7_engine.asm:5998,6036；读 prg7_engine.asm:5992",
    ),
    (
        0x0096,
        "EnergyPickupQtyCur",
        "constants_ram.asm:703；写 areas_common.asm:1329 / prg7_engine.asm:6009,6035；读 prg7_engine.asm:6006",
    ),
];

/// 掉落/拾取配额四字节的字段视图（字段顺序 = 地址顺序）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PickupQuotaFace {
    /// `$0093` `MissilePickupQtyMax`：本房间还能掉多少导弹（进门重算，上限 15）。
    pub missile_qty_max: u8,
    /// `$0094` `EnergyPickupQtyMax`：本房间还能掉多少能量（进门重算，`(r<<1)|$40`，上限 159）。
    pub energy_qty_max: u8,
    /// `$0095` `MissilePickupQtyCur`：本房间已掉出的导弹数（进门清零）。
    pub missile_qty_cur: u8,
    /// `$0096` `EnergyPickupQtyCur`：本房间已掉出的能量数（进门清零）。
    pub energy_qty_cur: u8,
}

impl PickupQuotaFace {
    /// 从 RAM 面解码（`ram` 是 `$0000-$07FF` 全页，长度 ≥ `0x800`）。
    ///
    /// 只读本面声明的 4 B，其余字节不碰。
    pub fn decode(ram: &[u8]) -> Self {
        assert!(ram.len() >= 0x800, "掉落/拾取配额面需要 ≥ 0x800 B 的 RAM 切片");
        Self {
            missile_qty_max: ram[0x0093],
            energy_qty_max: ram[0x0094],
            missile_qty_cur: ram[0x0095],
            energy_qty_cur: ram[0x0096],
        }
    }

    /// 编码回 RAM 面（只写本面声明的 4 B）。
    pub fn encode(&self, ram: &mut [u8]) {
        assert!(ram.len() >= 0x800, "掉落/拾取配额面需要 ≥ 0x800 B 的 RAM 切片");
        ram[0x0093] = self.missile_qty_max;
        ram[0x0094] = self.energy_qty_max;
        ram[0x0095] = self.missile_qty_cur;
        ram[0x0096] = self.energy_qty_cur;
    }

    /// 按字节偏移读一个字段（`0..FACE_LEN`）。
    pub fn get_byte(&self, off: usize) -> u8 {
        assert!(off < FACE_LEN, "字段偏移越界：{off}（面长 {FACE_LEN}）");
        match off {
            0 => self.missile_qty_max,
            1 => self.energy_qty_max,
            2 => self.missile_qty_cur,
            _ => self.energy_qty_cur,
        }
    }

    /// 按字节偏移写一个字段（`0..FACE_LEN`）。
    pub fn set_byte(&mut self, off: usize, v: u8) {
        assert!(off < FACE_LEN, "字段偏移越界：{off}（面长 {FACE_LEN}）");
        match off {
            0 => self.missile_qty_max = v,
            1 => self.energy_qty_max = v,
            2 => self.missile_qty_cur = v,
            _ => self.energy_qty_cur = v,
        }
    }

    /// 本面的 `(起始地址, 长度)`——测试与文档用，与 [`NesMapped::SPANS`] 同源。
    pub const SPAN_TABLE: [(u16, u16); 1] = [(BASE, FACE_LEN as u16)];
}

impl NesMapped for PickupQuotaFace {
    const DISCRIMINATOR: Option<u16> = None;

    /// 单段 `Owner`，4 B。
    const SPANS: &'static [(NesSpan, SpanKey)] =
        &[(NesSpan::value(BASE, FACE_LEN as u16), SpanKey::Owner)];

    fn project(&self, out: &mut NesImage) {
        self.encode(&mut out.ram);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ram_with_pattern() -> [u8; 0x800] {
        let mut ram = [0u8; 0x800];
        for (a, b) in ram.iter_mut().enumerate() {
            *b = (a as u8).wrapping_mul(17).wrapping_add(3);
        }
        ram
    }

    /// 段表与 `SPANS` 一致，且合计正好 4 B、落在 `$0093-$0096`。
    #[test]
    fn span_covers_exactly_the_four_named_bytes() {
        assert_eq!(PickupQuotaFace::SPANS.len(), 1);
        let (span, key) = PickupQuotaFace::SPANS[0];
        assert!(matches!(key, SpanKey::Owner));
        assert_eq!((span.start, span.len), (BASE, FACE_LEN as u16));
        assert_eq!(BASE, 0x0093);
        assert_eq!(BASE + FACE_LEN as u16, 0x0097);
        assert_eq!(PickupQuotaFace::SPAN_TABLE[0], (BASE, FACE_LEN as u16));
        // 四个符号逐字节对应，且不越出本段。
        for (i, (addr, name, src)) in SYMBOLS.iter().enumerate() {
            assert_eq!(*addr, BASE + i as u16, "{name} 地址不连续");
            assert!(*addr >= BASE && *addr < BASE + FACE_LEN as u16, "{name} 越界");
            // 证据必须带到 asm 行号与读写点两侧，不许只有名字。
            assert!(src.contains("constants_ram.asm:"), "{name} 缺声明行：{src}");
            assert!(src.contains("写 "), "{name} 缺写点：{src}");
            assert!(src.contains("读 "), "{name} 缺读点：{src}");
        }
    }

    /// 每个符号的声明行号必须真的写着该名字（读源文件校验，不靠记忆）。
    #[test]
    fn symbols_declaration_lines_name_their_symbol() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../refs/m1disasm/SRC/constants_ram.asm");
        let Ok(text) = std::fs::read_to_string(path) else {
            eprintln!("SELFTEST SKIP：refs/m1disasm 未初始化，跳过 asm 回读");
            return;
        };
        for (_, name, src) in SYMBOLS {
            let line = src
                .split("constants_ram.asm:")
                .nth(1)
                .and_then(|s| s.split(|c: char| !c.is_ascii_digit()).next())
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or_else(|| panic!("{name} 证据里没有可解析的行号：{src}"));
            let body = text
                .lines()
                .nth(line - 1)
                .unwrap_or_else(|| panic!("{name} 证据行 {line} 不存在"));
            assert!(body.contains(name), "{name} 的声明行 {line} 不含该名：{body}");
        }
    }

    /// 解码→编码往返：本面 4 B 逐字节相等，面外一律不动。
    #[test]
    fn decode_encode_round_trip_and_isolation() {
        let mut ram = ram_with_pattern();
        let f = PickupQuotaFace {
            missile_qty_max: 0x0A,
            energy_qty_max: 0x53,
            missile_qty_cur: 0x02,
            energy_qty_cur: 0x07,
        };
        f.encode(&mut ram);
        assert_eq!(ram[0x0093], 0x0A);
        assert_eq!(ram[0x0094], 0x53);
        assert_eq!(ram[0x0095], 0x02);
        assert_eq!(ram[0x0096], 0x07);
        assert_eq!(PickupQuotaFace::decode(&ram), f);

        // 面外字节：$0092 与 $0097 必须保持原样。
        let before = ram_with_pattern();
        let mut ram2 = before;
        f.encode(&mut ram2);
        for a in 0..0x800usize {
            let owned = (0x0093..0x0097).contains(&a);
            if !owned {
                assert_eq!(ram2[a], before[a], "面外字节 ${a:04X} 被改写");
            }
        }
    }

    /// 字节访问器与字段顺序一一对应，越界必须 panic（fail-loud）。
    #[test]
    fn byte_accessors_match_field_order_and_reject_out_of_range() {
        let mut f = PickupQuotaFace::default();
        for i in 0..FACE_LEN {
            f.set_byte(i, 0xA0 + i as u8);
            assert_eq!(f.get_byte(i), 0xA0 + i as u8, "偏移 {i}");
        }
        for bad in [FACE_LEN, FACE_LEN + 1, 0xFF] {
            let got = std::panic::catch_unwind(move || PickupQuotaFace::default().get_byte(bad));
            assert!(got.is_err(), "偏移 {bad} 应当 panic");
        }
    }
}
