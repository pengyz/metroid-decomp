//! game-core::memmap::frame_face —— L3 watch 状态向量整面编码（**字段面生成唯一源**，
//! l3-gamecore-leg-verification.md §1 规则 1 / 严重-6 对账闭合件）。
//!
//! watch_snapshot/l3-dump 经本模块编码——废手写切片第二源（frame/memory.rs 旧
//! `hex_upper(&self.ram[…])` 形态退役）。机械契约（区间钉死，严重-5）：
//!
//! | 标签 | CPU 区间 | 编码长度 |
//! |---|---|---|
//! | zp | $0000-$00FF | 512 hex |
//! | iram | $0100-$07FF | 3584 hex |
//! | wram | **$6000-$7FFF 全 8KB** | 16384 hex |
//!
//! 【WRAM 区间钉死（2026-10-15，lead 裁定）】全 8KB=「全状态无隐藏态」原则（RNG 全状态
//! 证明义务同族：软上界留盲区=留隐患）+ FrameMemory 8KB 同框；旧「$6000-$6Cxx」软上界
//! **正式作废**，降级为游戏用区观察注（findings/ram-region-6af4-contradiction.md 定谳的
//! 用区事实不变，watch 面口径独立钉死）。

/// watch 面规范（label/base/range——manifest watch 条目的机械对应源）。
pub struct WatchFace {
    pub label: &'static str,
    /// 区间起（CPU 地址）。
    pub base: u16,
    /// 区间止（含，CPU 地址）。
    pub end: u16,
}

/// 三面规范表（键集白名单 `w` 对象标签的声明源之一；区间钉死=本表）。
pub const WATCH_FACES: [WatchFace; 3] = [
    WatchFace { label: "zp", base: 0x0000, end: 0x00FF },
    WatchFace { label: "iram", base: 0x0100, end: 0x07FF },
    WatchFace { label: "wram", base: 0x6000, end: 0x7FFF },
];

/// 大写 hex 编码（watch 面唯一编码器）。
pub fn hex_upper(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(s, "{b:02X}");
    }
    s
}

/// zp 编码（$0000-$00FF → 512 大写 hex；ram=2KB 归一镜像面切片）。
pub fn encode_zp(ram: &[u8]) -> String {
    hex_upper(&ram[0x000..0x100])
}

/// iram 编码（$0100-$07FF → 3584 大写 hex）。
pub fn encode_iram(ram: &[u8]) -> String {
    hex_upper(&ram[0x100..0x800])
}

/// wram 编码（$6000-$7FFF 全 8KB → 16384 大写 hex；FrameMemory.wram 面）。
pub fn encode_wram(wram: &[u8]) -> String {
    hex_upper(&wram[0x0000..0x2000])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 三面编码长度（512/3584/16384）与大写形态。
    #[test]
    fn face_encodings_match_pinned_lengths() {
        let ram = [0u8; 0x800];
        let wram = [0u8; 0x2000];
        assert_eq!(encode_zp(&ram).len(), 512);
        assert_eq!(encode_iram(&ram).len(), 3584);
        assert_eq!(encode_wram(&wram).len(), 16384);
        let mut r2 = [0u8; 0x800];
        r2[0] = 0xab;
        assert_eq!(encode_zp(&r2), format!("AB{}", "00".repeat(255)));
    }

    /// 区间钉死表（严重-5：WRAM=$6000-$7FFF 全 8KB，软上界死）。
    #[test]
    fn watch_faces_pin_wram_full_8k() {
        let w = &WATCH_FACES[2];
        assert_eq!((w.label, w.base, w.end), ("wram", 0x6000, 0x7FFF));
        assert_eq!(w.end - w.base + 1, 0x2000);
    }
}
