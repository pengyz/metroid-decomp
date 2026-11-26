//! 页 1 `$0108-$010F` typed 声明层 + `$01D0-$01FF` 栈的显式 ephemeral 声明——B2-c（#97）。
//!
//! 【为什么只做 8 B】页 1（`$0100-$01FF`）不是纯栈页。按 `docs/spec/stack-face-0108.md`（#86 取证）：
//!
//! | 段 | 地址 | 长度 | 结论 | 本模块怎么处理 |
//! |---|---|---|---|---|
//! | A：具名变量 | `$0108-$010F` | 8 B | 可 typed（8 个具名字段，A 级） | `Owner` 面 + 字段访问器 |
//! | B：空档 | `$0110-$01CF` | 192 B | 不可 typed（零显式访问、10 009 份 trace 全 0） | **不声明**，留 Undeclared（见下） |
//! | C：栈真身 | `$01D0-$01FF` | 48 B | 只能 ephemeral（`php/pha/jsr/rts` 隐式写） | **不声明**，留 Undeclared（见下） |
//!
//! 【段 B 为什么不声明】它是「没有任何游戏语义的空档」：反汇编里对 `$0110-$01FF` 的显式访问
//! 是 0 处（唯一带索引的页 1 写是 `ClearSamusStats` 的 `sta $0100,y`，y 上界 `$0F`），
//! 10 009 份 trace 里这 192 B 全 0。把它切成字段会是伪语义；把它声明成 `Unmodeled`
//! 又会**增加** Unmodeled 计数，而 `tools/nes_map_check.py` 的棘轮只允许 Unmodeled 下降
//! （见该脚本 docstring）。所以最诚实的做法是让它继续留在 Undeclared——账面写「这段没人认领」，
//! 与我们已知的事实完全一致；等哪天发现写点再开卡。
//!
//! 【段 C 为什么不声明（连 Ephemeral 也不声明）】它是 6502 的调用栈（NMI 入口每帧固定压
//! 4 B，`prg7_engine.asm:233-241`），内容每帧变，Rust 侧根本没有这个栈；声明成 `Owner`
//! 等于要求 Rust 维护一个它不需要的栈，字节面必然与 oracle 不同。`SpanKey::Ephemeral`
//! 虽然表达「跨调用不保义」，但 `tools/nes_map_check.py --write` 有转换等式：
//! **Owner/View 的 +N 必须等于 Unmodeled/Undeclared 的 −N**（`ephemeral` 只算「已声明」、
//! 不进 Owner，于是会造出「Undeclared −48 而 Owner +0」的不等式，`--write` 直接拒绝）。
//! 因此本卡只声明段 A；栈继续留在 Undeclared，账面写「这 48 B 没有 typed 归属」，与事实一致。
//!
//! 【字段命名纪律】8 个字段全部来自 `refs/m1disasm/SRC/constants_ram.asm:826-843` 的具名
//! （`.enum $0106 export`），**没有 `_todo_`**。`$010C/$010D` 的**读点未见**是 #86 的 C 级假设
//! （只核了具名引用，没穷举间接读），这里沿用 asm 名，不据此断言「死字节」。

use crate::memmap::nesmap::{NesMapped, NesSpan, SpanKey};
use crate::memmap::storage::NesImage;

/// 段 A 基址（CPU 地址）。
pub const BASE: u16 = 0x0108;
/// 段 A 长度（`$0108-$010F` 含端点）。
pub const FACE_LEN: usize = 8;
/// 段 C（栈）基址。
pub const STACK_BASE: u16 = 0x01D0;
/// 段 C（栈）长度（`$01D0-$01FF`）。
pub const STACK_LEN: usize = 0x30;

/// 段 A 逐字节命名表（`(地址, asm 名, 证据)`）——测试与文档的单一真相源。
pub const SYMBOLS: [(u16, &str, &str); FACE_LEN] = [
    (0x0108, "MiniBossKillDelayFlag", "constants_ram.asm:830"),
    (0x0109, "PowerUpDelayFlag", "constants_ram.asm:831"),
    (0x010A, "EndTimer", "constants_ram.asm:833"),
    (0x010B, "EndTimer+1", "constants_ram.asm:834"),
    (0x010C, "EndTimerEnemyHi", "constants_ram.asm:836"),
    (0x010D, "EndTimerEnemyIsEnabled", "constants_ram.asm:837"),
    (0x010E, "MissileToggle", "constants_ram.asm:839"),
    (0x010F, "SamusHurt010F", "constants_ram.asm:840"),
];

/// 页 1 变量/栈的声明层。
///
/// 字段即字节：`decode`/`encode` 与 `get_byte`/`set_byte` 互为镜像，均以 `&0x7FF`
/// 归一地址访问（与 `snapshot_nes()` 同源同值）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StackPageFace {
    /// `$0108`：Kraid/Ridley 击杀后启动能力音乐与延迟（`prg7:6051` 写、`prg7:2044` 消费后清）。
    pub mini_boss_kill_delay_flag: u8,
    /// `$0109`：拾取道具后启动能力音乐与延迟（`prg7:5650` 写、`prg7:2045` 清）。
    pub power_up_delay_flag: u8,
    /// `$010A`：逃生计时低字节（两位 BCD，`prg3:1421` 设置 / `prg3:1975` 递减）。
    pub end_timer_lo: u8,
    /// `$010B`：逃生计时高字节；`$FF` = 未启用（`prg3:1422` / `prg3:1981`）。
    pub end_timer_hi: u8,
    /// `$010C`：逃生计时敌高字节（`prg3:1428` / `prg3:1517`）；读点未见（C 级）。
    pub end_timer_enemy_hi: u8,
    /// `$010D`：`#00`=无 / `#01`=启用 TIME BOMB 敌（`prg3:884` / `prg3:1425`）；读点未见（C 级）。
    pub end_timer_enemy_is_enabled: u8,
    /// `$010E`：0=弹 / 1=导弹（`prg7:5483` 写；`prg7:2546/3836/4006` 读）。
    pub missile_toggle: u8,
    /// `$010F`：asm 注释自述 **"never read"**，取值随受击来源变化（`prg7` 四处写、零具名读）。
    pub samus_hurt_010f: u8,
}

impl StackPageFace {
    /// 该地址是否落**段 A**（typed 面）内。
    pub fn contains(addr: u16) -> bool {
        addr >= BASE && (addr as usize) < BASE as usize + FACE_LEN
    }

    /// 段 A 字节 → 字段。
    pub fn decode(bytes: &[u8; FACE_LEN]) -> Self {
        Self {
            mini_boss_kill_delay_flag: bytes[0x00],
            power_up_delay_flag: bytes[0x01],
            end_timer_lo: bytes[0x02],
            end_timer_hi: bytes[0x03],
            end_timer_enemy_hi: bytes[0x04],
            end_timer_enemy_is_enabled: bytes[0x05],
            missile_toggle: bytes[0x06],
            samus_hurt_010f: bytes[0x07],
        }
    }

    /// 字段 → 段 A 字节。
    pub fn encode(self) -> [u8; FACE_LEN] {
        [
            self.mini_boss_kill_delay_flag,
            self.power_up_delay_flag,
            self.end_timer_lo,
            self.end_timer_hi,
            self.end_timer_enemy_hi,
            self.end_timer_enemy_is_enabled,
            self.missile_toggle,
            self.samus_hurt_010f,
        ]
    }

    /// 两字节字段：`EndTimer`（`$010A` 低 + `$010B` 高）拼接值。
    pub fn end_timer(self) -> u16 {
        u16::from_le_bytes([self.end_timer_lo, self.end_timer_hi])
    }

    /// 写两字节字段（保持与 `[$010A]=lo`、`[$010B]=hi` 的 RAM 布局一致）。
    pub fn set_end_timer(&mut self, value: u16) {
        let [lo, hi] = value.to_le_bytes();
        self.end_timer_lo = lo;
        self.end_timer_hi = hi;
    }

    /// 地址 → 字节（不在段 A 返回 `None`）。
    pub fn get_byte(self, addr: u16) -> Option<u8> {
        if !Self::contains(addr) {
            return None;
        }
        Some(self.encode()[(addr - BASE) as usize])
    }

    /// 写字节（不在段 A 返回 `false`）。
    pub fn set_byte(&mut self, addr: u16, value: u8) -> bool {
        if !Self::contains(addr) {
            return false;
        }
        let mut bytes = self.encode();
        bytes[(addr - BASE) as usize] = value;
        *self = Self::decode(&bytes);
        true
    }
}

impl NesMapped for StackPageFace {
    const DISCRIMINATOR: Option<u16> = None;

    /// 只有段 A：8 B `Owner`（typed 真相）。
    ///
    /// 段 B（`$0110-$01CF`，192 B 零写空档）与段 C（`$01D0-$01FF`，48 B 6502 栈）
    /// **故意都不声明**：见模块文档（前者无游戏语义，后者是每帧变的 CPU 栈；
    /// 声明 `Ephemeral` 会破坏 `nes_map_check --write` 的「Owner/View 增量 ==
    /// Unmodeled/Undeclared 减量」等式）。
    const SPANS: &'static [(NesSpan, SpanKey)] =
        &[(NesSpan::value(BASE, FACE_LEN as u16), SpanKey::Owner)];

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

    /// 几何与声明：只有段 A 8 B `Owner`；段 B/C 都不在声明里（零写区与栈都不许 typed）。
    #[test]
    fn spans_cover_segment_a_only() {
        assert_eq!(StackPageFace::SPANS.len(), 1);
        let (a, key_a) = &StackPageFace::SPANS[0];
        assert_eq!(*a, NesSpan::value(0x0108, 8));
        assert!(matches!(key_a, SpanKey::Owner));
        for addr in 0x0100..0x0200u16 {
            let in_a = (0x0108..=0x010F).contains(&addr);
            assert_eq!(StackPageFace::contains(addr), in_a, "${addr:04X} 归属不符");
        }
        // 段 B（192 B 零写）与段 C（48 B 栈）必须不被任何声明覆盖。
        assert_eq!(STACK_BASE, 0x01D0);
        assert_eq!(STACK_LEN, 0x30);
        assert_eq!(0x01CFu16 - 0x0110u16 + 1, 192);
    }

    /// 命名表：8 条、地址连续、都指到 `constants_ram.asm`。
    #[test]
    fn symbols_are_contiguous_and_cite_asm() {
        assert_eq!(SYMBOLS.len(), FACE_LEN);
        for (index, (addr, name, src)) in SYMBOLS.iter().enumerate() {
            assert_eq!(*addr, BASE + index as u16, "{name} 地址不连续");
            assert!(src.starts_with("constants_ram.asm:"), "{name} 缺证据：{src}");
            assert!(!name.starts_with("_todo"), "{name} 应为 asm 具名");
        }
    }

    /// 字节镜像：`decode(encode(x)) == x`；`get_byte`/`set_byte` 覆盖整段且不越界。
    #[test]
    fn byte_view_is_consistent() {
        let face = StackPageFace::decode(&[1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(face.encode(), [1, 2, 3, 4, 5, 6, 7, 8]);
        let mut face = face;
        assert_eq!(face.get_byte(0x0108), Some(1));
        assert_eq!(face.get_byte(0x010F), Some(8));
        assert_eq!(face.get_byte(0x0107), None);
        assert_eq!(face.get_byte(0x0110), None);
        assert!(face.set_byte(0x010E, 0xAB));
        assert_eq!(face.missile_toggle, 0xAB);
        assert!(!face.set_byte(0x01D0, 0xFF), "栈区不是 typed 面");
    }

    /// 两字节字段：`EndTimer` 低/高字节与 `u16` 读写互为镜像（≤2 字节字段各一例之一）。
    #[test]
    fn end_timer_is_two_byte_field() {
        let mut face = StackPageFace::default();
        face.set_end_timer(0x1234);
        assert_eq!(face.end_timer_lo, 0x34);
        assert_eq!(face.end_timer_hi, 0x12);
        assert_eq!(face.end_timer(), 0x1234);
        assert_eq!(face.get_byte(0x010A), Some(0x34));
        assert_eq!(face.get_byte(0x010B), Some(0x12));
    }

    /// 与裸字节面（`Manager::read_ram`/`write_ram`）等价：typed 写进去的字节，裸面读出来一样。
    #[test]
    fn typed_field_matches_raw_dispatch() {
        let mut mem = crate::state::Manager::default();
        let mut face = StackPageFace::default();
        // 单字节字段：MissileToggle
        face.missile_toggle = 1;
        mem.write_ram(0x010E, face.missile_toggle);
        assert_eq!(mem.read_ram(0x010E), 1);
        assert_eq!(face.get_byte(0x010E), Some(mem.read_ram(0x010E)));
        // 双字节字段：EndTimer
        face.set_end_timer(0x59_30);
        mem.write_ram(0x010A, face.end_timer_lo);
        mem.write_ram(0x010B, face.end_timer_hi);
        let decoded = StackPageFace::decode(&[
            mem.read_ram(0x0108),
            mem.read_ram(0x0109),
            mem.read_ram(0x010A),
            mem.read_ram(0x010B),
            mem.read_ram(0x010C),
            mem.read_ram(0x010D),
            mem.read_ram(0x010E),
            mem.read_ram(0x010F),
        ]);
        assert_eq!(decoded.end_timer(), 0x59_30);
    }

    /// `project` 只写段 A 的 8 B，不碰栈区。
    #[test]
    fn project_writes_only_segment_a() {
        let mut out = NesImage {
            ram: [0u8; 0x800],
            wram: [0u8; 0x2000],
        };
        let face = StackPageFace::decode(&[9, 8, 7, 6, 5, 4, 3, 2]);
        face.project(&mut out);
        assert_eq!(&out.ram[0x108..0x110], &[9, 8, 7, 6, 5, 4, 3, 2]);
        assert_eq!(out.ram[0x1D0], 0, "栈区不应被 project 改写");
        assert_eq!(out.ram[0x110], 0, "段 B 不应被 project 改写");
    }
}
