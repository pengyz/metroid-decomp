//! game-core::memmap::samus_projectile_face —— **敌人弹槽族 `SamusProjectiles`（`$03D0-$03FF`）的 typed 声明层**
//! （卡 #138：把 48 B `Unmodeled` 升格为可审计的 `Owner` 段）。
//!
//! 【这一段是什么】Samus 武器弹（beam/missile/bomb）的**槽状态**：每个弹槽 16 B，
//! 布局就是 `Object` 结构本体（状态、半径、动画、速度、命中位、nametable、y/x）。
//! asm 的原名是 `SamusProjectiles .instanceof Object 3 startfrom 0`
//! （`refs/m1disasm/SRC/constants_ram.asm:877`），起始槽是 `$D0`（= `$03D0`），
//! 三个槽 ⇒ `$03D0-$03FF`（`constants_ram.asm:854-877` 的「slot D to F is for samus weapon projectiles」）。
//!
//! 【为什么能升 Owner（证据级 A）】
//! - 结构定义：`.struct Object` 逐字段具名，`constants_ram.asm:217-240`；字段在槽内偏移
//!   `+0..+15` 与 asm 的 `;$0300..$030E` 注释一一对应（见 `FIELDS` 每行证据）。
//! - 槽实例：`SamusProjectiles .instanceof Object 3 startfrom 0`（`:877`），
//!   调用侧按 `SamusProjectiles.0 - Objects`（= `$D0`）再 `Yplus16` 遍历：
//!   `prg7_engine.asm:3703/3709/3715`（找空槽）、`:3820`（从槽 0 起）、`:3840`、`:4009`、`:4036`、
//!   `:9299-9303`（三槽逐个处理）、`:10086`、`:10222`（循环上界 `SamusProjectiles - Objects + sizeof - sizeof.0`）。
//! - 行为面：`UpdateSamusProjectiles ; 07:D4BF`（`prg7_engine.asm:4273`，主循环 `:2500 jsr`）每帧推进三槽。
//!
//! 【本面只做声明】`world/`、`weapon/`、`samus/` 的读写路径一行不动；本面不接管访问，
//! 也不改任何裸地址。字节真源仍是 `storage`（`Manager`），本结构是按需解码视图。
//!
//! 【与相邻面的边界】
//! - `$0300-$0316`：Samus 本体（`Samus instanceof Object`，`constants_ram.asm:867`），
//!   由既有 Samus 面登记，本面不碰。
//! - `$0317-$03CF`：仍是 `Unmodeled`（Samus/电梯/雕像/门等 16 B 槽的其余字节），
//!   按「一次一片、可实测下降」留给后续批次；本面只认领最后 48 B 的弹槽三槽。
//! - `$05D0-$05FF`：wave beam 弹**轨迹**额外面（`wave_beam_face`），不是 Object 槽本体。

use crate::memmap::nesmap::{NesMapped, NesSpan, SpanKey};
use crate::memmap::storage::NesImage;

/// 面基址：`$03D0` = `Objects + $D0`（`SamusProjectiles.0`，`constants_ram.asm:877`）。
pub const BASE: u16 = 0x03D0;
/// 槽长：16 B（`Object` 结构 16 字段，`constants_ram.asm:217-240`）。
pub const SLOT_LEN: u16 = 16;
/// 槽数：3（`SamusProjectiles .instanceof Object 3`，`constants_ram.asm:877`）。
pub const SLOTS: usize = 3;
/// 面长：3 × 16 = 48 B（`$03D0-$03FF`）。
pub const FACE_LEN: u16 = (SLOTS as u16) * SLOT_LEN;

/// 槽内字段表：`(槽内偏移, 字段名, asm 证据)`。
///
/// 字段名与顺序照 `constants_ram.asm:217-240` 的 `.struct Object`；证据列指向该字段的
/// *声明行*（asm 注释里的 `;$0300..$030E` 是「槽 0 的绝对地址」，本表用槽内偏移表达）。
pub const FIELDS: [(u16, &str, &str); SLOT_LEN as usize] = [
    (0x00, "status", "constants_ram.asm:218"),
    (0x01, "radiusY", "constants_ram.asm:219"),
    (0x02, "radiusX", "constants_ram.asm:220"),
    (0x03, "animFrame", "constants_ram.asm:221"),
    (0x04, "animDelay", "constants_ram.asm:222"),
    (0x05, "animResetIndex", "constants_ram.asm:223"),
    (0x06, "animIndex", "constants_ram.asm:224"),
    (0x07, "data07", "constants_ram.asm:225"),
    (0x08, "speedY", "constants_ram.asm:226"),
    (0x09, "speedX", "constants_ram.asm:227"),
    (0x0A, "isHit", "constants_ram.asm:228"),
    (0x0B, "onScreen", "constants_ram.asm:235"),
    (0x0C, "hi", "constants_ram.asm:236"),
    (0x0D, "y", "constants_ram.asm:237"),
    (0x0E, "x", "constants_ram.asm:238"),
    (0x0F, "data0F", "constants_ram.asm:239"),
];

/// 敌人弹槽族（`SamusProjectiles.0..2`）的声明层。
///
/// 结构体字段即字节：`decode`/`encode` 与 `get_byte`/`set_byte` 互为镜像，均以
/// `&0x7FF` 归一地址访问（与 `snapshot_nes()` 同源同值）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SamusProjectileFace {
    /// 三个弹槽（槽 0=$03D0、槽 1=$03E0、槽 2=$03F0），槽内布局见 [`FIELDS`]。
    pub slots: [[u8; SLOT_LEN as usize]; SLOTS],
}

impl SamusProjectileFace {
    /// 从 48 B 解码；短数组右侧补 0（便于测试与半截快照）。
    pub fn decode(bytes: &[u8]) -> Self {
        let mut slots = [[0u8; SLOT_LEN as usize]; SLOTS];
        for (i, slot) in slots.iter_mut().enumerate() {
            for (j, byte) in slot.iter_mut().enumerate() {
                *byte = bytes.get(i * SLOT_LEN as usize + j).copied().unwrap_or(0);
            }
        }
        Self { slots }
    }

    /// 编码回 48 B（槽序 0→2、槽内偏移 0→15）。
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(FACE_LEN as usize);
        for slot in &self.slots {
            out.extend_from_slice(slot);
        }
        out
    }

    /// 面内是否包含该地址（`$03D0-$03FF`）。
    pub fn contains(addr: u16) -> bool {
        addr >= BASE && addr < BASE + FACE_LEN
    }

    /// 该地址的 `(槽号, 槽内偏移, 字段名)`；越界返回 `None`。
    pub fn field_at(addr: u16) -> Option<(usize, u16, &'static str)> {
        if !Self::contains(addr) {
            return None;
        }
        let rel = addr - BASE;
        let slot = (rel / SLOT_LEN) as usize;
        let off = rel % SLOT_LEN;
        let name = FIELDS
            .iter()
            .find(|(o, _, _)| *o == off)
            .map(|(_, n, _)| *n)?;
        Some((slot, off, name))
    }

    /// 读一个字节；越界返回 `None`。
    pub fn get_byte(&self, addr: u16) -> Option<u8> {
        let (slot, off, _) = Self::field_at(addr)?;
        Some(self.slots[slot][off as usize])
    }

    /// 写一个字节；越界返回 `false`。
    pub fn set_byte(&mut self, addr: u16, value: u8) -> bool {
        let Some((slot, off, _)) = Self::field_at(addr) else {
            return false;
        };
        self.slots[slot][off as usize] = value;
        true
    }
}

impl NesMapped for SamusProjectileFace {
    /// Object 槽没有单一判别源（`status` 是槽自身状态，不是「这是哪张面的判别字段」）。
    const DISCRIMINATOR: Option<u16> = None;

    /// 整段连续 48 B 登记为 `Owner`：三槽 × `Object` 16 字段，字段级 asm 具名。
    const SPANS: &'static [(NesSpan, SpanKey)] =
        &[(NesSpan::value(BASE, FACE_LEN), SpanKey::Owner)];

    fn project(&self, out: &mut NesImage) {
        let ram_len = out.ram.len();
        for (offset, byte) in self.encode().iter().enumerate() {
            let index = (BASE as usize + offset) & (ram_len - 1);
            out.ram[index] = *byte;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 几何与声明：48 B 一段 `Owner`，`contains` 边界正确。
    #[test]
    fn geometry_and_span() {
        assert_eq!(SLOT_LEN, 16);
        assert_eq!(SLOTS, 3);
        assert_eq!(FACE_LEN, 48);
        assert_eq!(SamusProjectileFace::SPANS.len(), 1);
        let (span, key) = &SamusProjectileFace::SPANS[0];
        assert_eq!(*span, NesSpan::value(0x03D0, 48));
        assert!(matches!(key, SpanKey::Owner));
        assert!(SamusProjectileFace::contains(0x03D0));
        assert!(SamusProjectileFace::contains(0x03FF));
        assert!(!SamusProjectileFace::contains(0x03CF));
        assert!(!SamusProjectileFace::contains(BASE + FACE_LEN));
    }

    /// 字段表覆盖槽内 16 字节、偏移连续、证据非空，且证据行号真的写着该字段名。
    ///
    /// 后一条是 #131 的教训：只断言「证据非空」抓不到行号偏位，必须逐行读 asm 比对内容。
    /// `refs/` 是 submodule，缺失时显式跳过（打印 SKIP 原因，不静默通过）。
    #[test]
    fn fields_are_complete_and_evidence_lines_name_them() {
        assert_eq!(FIELDS.len(), SLOT_LEN as usize);
        let asm_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../refs/m1disasm/SRC/constants_ram.asm"
        );
        let asm = std::fs::read_to_string(asm_path).ok();
        let lines: Vec<&str> = asm.as_deref().map(|s| s.lines().collect()).unwrap_or_default();
        for (i, (off, name, evidence)) in FIELDS.iter().enumerate() {
            assert_eq!(*off, i as u16, "第 {i} 项槽内偏移不连续");
            assert!(!name.is_empty(), "+{off:#04x} 缺字段名");
            assert!(!evidence.is_empty(), "+{off:#04x} 缺证据指针");
            let Some(asm_text) = asm.as_deref() else {
                eprintln!("SKIP fields_are_complete_and_evidence_lines_name_them：{asm_path} 不可读，refs 子模块未初始化");
                return;
            };
            let _ = asm_text;
            let line_no: usize = evidence
                .split_once("constants_ram.asm:")
                .and_then(|(_, n)| n.trim().parse().ok())
                .unwrap_or_else(|| panic!("+{off:#04x} 证据格式应为 constants_ram.asm:N：{evidence}"));
            let line = lines
                .get(line_no - 1)
                .unwrap_or_else(|| panic!("+{off:#04x} 证据行 {line_no} 越界"));
            assert!(
                line.contains(name),
                "+{off:#04x} 证据行 {line_no} 不含字段名 {name}：{line}"
            );
        }
    }

    /// 解码/编码往返一致，且跨槽不串。
    #[test]
    fn decode_encode_roundtrip() {
        let bytes: Vec<u8> = (0..FACE_LEN).map(|i| (i as u8).wrapping_mul(7).wrapping_add(3)).collect();
        let face = SamusProjectileFace::decode(&bytes);
        assert_eq!(face.encode(), bytes);
        assert_eq!(face.slots[0][0x00], bytes[0x00]);
        assert_eq!(face.slots[1][0x00], bytes[0x10]);
        assert_eq!(face.slots[2][0x0F], bytes[0x2F]);
    }

    /// 字节访问器：槽/偏移换算正确、越界拒绝。
    #[test]
    fn byte_accessors_map_slot_offsets() {
        let mut face = SamusProjectileFace::default();
        assert!(face.set_byte(0x03E0, 0x2A)); // 槽 1 status
        assert_eq!(face.slots[1][0], 0x2A);
        assert_eq!(face.get_byte(0x03E0), Some(0x2A));
        assert_eq!(SamusProjectileFace::field_at(0x03FF), Some((2, 0x0F, "data0F")));
        assert_eq!(SamusProjectileFace::field_at(0x03D0), Some((0, 0x00, "status")));
        assert_eq!(SamusProjectileFace::field_at(0x03CF), None);
        assert_eq!(SamusProjectileFace::field_at(BASE + FACE_LEN), None);
        assert!(!face.set_byte(BASE + FACE_LEN, 0));
    }
}
