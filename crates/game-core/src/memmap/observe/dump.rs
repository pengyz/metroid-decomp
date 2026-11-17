//! ram.hex 整面投影 —— **单一来源：`Manager::snapshot_nes()`**。
//!
//! 【这一面是什么】ram.hex 是对拍/取证的字节视图：前 `$0000-$07FF` 是 NES 2KB 镜像，
//! 后段 `$6AF4-$6BD3` 是敌槽与爆炸槽的额外面（`EnsExtra` 12×16 B 从 `$6AF4` 起，
//! 爆炸池额外面 4×8 B 从 `$6BB4` 起，两者首尾相接）。旧形只导出到 `$6B73`
//! （敌槽 0-7 的末尾）——敌槽 8-11 与爆炸槽 1-3 从未进入 ram.hex，是对拍盲区。
//!
//! 【整面单一来源，禁止再混 `read_wram`】整个字符串只取 `snapshot_nes()` 的
//! `ram`/`wram` 镜像。`read_wram` 与快照当前对 typed 字节同值，但快照/回灌语义一旦
//! 分叉（P1 正在收口），混用就会出现「同一份 ram.hex 里两段来自不同时刻/不同真相源」
//! 的静默错位——这类错位不会让任何门禁变红，只会让对拍看起来全绿。

/// 额外面地址区间（闭区间）：`$6AF4-$6BD3`。
///
/// 从实体层的唯一定义处推导，不在这里重写裸地址；`$6BD3` = 爆炸槽 3 的 `hi`
/// （`$6BB4 + 3*8 + 7`），恰好是 `EnsExtra`/爆炸额外面两段的末尾。
pub const EXTRA_RANGE: (u16, u16) = (
    crate::enemy::state::ENS_EXTRA_BASE,
    crate::enemy::explosion::EXTRA_BASE
        + (crate::enemy::explosion::SLOT_COUNT as u16) * crate::enemy::explosion::EXTRA_STRIDE
        - 1,
);

/// dump 的字节数：2KB 镜像 + 额外面长度（`$0000-$07FF` 后接 `$6AF4-$6BD3`）。
pub const DUMP_LEN: usize = 0x800 + (EXTRA_RANGE.1 - EXTRA_RANGE.0 + 1) as usize;

/// 把整面 RAM 导出成**连续大写十六进制**（每字节两位，末尾恰好一个 `\n`）。
///
/// 段序 = `$0000-$07FF`（`img.ram`）→ `$6AF4-$6BD3`（`img.wram` 的对应切片）。
/// 整面只读 [`crate::state::Manager::snapshot_nes`] 的镜像；**禁止**在这里补
/// `read_wram` 之类的第二来源（见模块头）。
pub fn ram_dump_hex(mem: &crate::state::Manager) -> String {
    use std::fmt::Write as _;

    let img = mem.snapshot_nes();
    let mut out = String::with_capacity(DUMP_LEN * 2 + 1);

    for b in &img.ram[..0x800] {
        write!(out, "{b:02X}").expect("写 String 不会失败");
    }

    let (lo, hi) = EXTRA_RANGE;
    let start = (lo - 0x6000) as usize;
    let end = (hi - 0x6000) as usize;
    for b in &img.wram[start..=end] {
        write!(out, "{b:02X}").expect("写 String 不会失败");
    }

    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enemy::explosion::{
        EXTRA_BASE as EXPL_EXTRA_BASE, EXTRA_STRIDE as EXPL_EXTRA_STRIDE,
        SLOT_COUNT as EXPL_SLOT_COUNT,
    };
    use crate::enemy::state::{EnemyStatus, ENS_EXTRA_BASE, SLOT_COUNT as ENEMY_SLOT_COUNT};

    #[test] // 用例1
    fn dump_covers_enemy_slots_8_to_11_and_explosion_slots_1_to_3() {
        let mut m = crate::state::Manager::default();
        // 敌槽 8-11：typed 真相在 `Manager.enemies`，storage 的 wram 不含这些字节。
        for slot in 8..ENEMY_SLOT_COUNT {
            m.enemy(slot).set_status(EnemyStatus::Active);
        }
        // 爆炸槽 1-3：额外面 status 0→1（asm 原样 `inc status`，不是敌人七态）。
        for slot in 1..EXPL_SLOT_COUNT {
            m.explosion(slot).occupy();
        }
        // `$6BD3` = 爆炸槽 3 的 hi，整面最后一个字节（用字面量写，独立于实现常量）。
        m.write_wram(0x6BD3, 0xAB);

        let hex = ram_dump_hex(&m);

        // 长度独立算：2KB 镜像 + 224B 额外面 = 2272B → 4544 hex 字符 + 1 换行。
        assert_eq!(hex.len(), 0x8E0 * 2 + 1, "2KB+224B=2272B → 4545 字符");
        assert_eq!(DUMP_LEN, 0x8E0);
        assert_eq!(hex.trim_end().len(), 0x8E0 * 2, "每字节两位十六进制");
        assert_eq!(hex.matches('\n').count(), 1, "末尾恰好一个换行");
        assert!(hex.ends_with("AB\n"), "最后一个字节 $6BD3 必须落在串尾");
        assert!(
            !hex[..hex.len() - 1].chars().any(|c| c.is_ascii_lowercase()),
            "十六进制必须大写"
        );

        // 下标用实体布局基址换算；**期望地址一律用字面量**（不复用 EXTRA_RANGE/DUMP_LEN）。
        let at = |addr: u16| -> u8 {
            let byte = if addr <= 0x07FF {
                addr as usize
            } else {
                0x800 + (addr - ENS_EXTRA_BASE) as usize
            };
            u8::from_str_radix(&hex[byte * 2..byte * 2 + 2], 16).unwrap()
        };

        for (addr, slot) in [(0x6B74u16, 8u8), (0x6B84, 9), (0x6B94, 10), (0x6BA4, 11)] {
            assert_eq!(at(addr), 2, "敌槽 {slot} 的 status 必须在 dump 内");
        }
        for (addr, slot) in [(0x6BBCu16, 1u8), (0x6BC4, 2), (0x6BCC, 3)] {
            assert_eq!(at(addr), 1, "爆炸槽 {slot} 的 status 必须在 dump 内");
        }
        assert_eq!(at(0x6BD3), 0xAB, "dump 必须覆盖到最后一个字节 $6BD3");
        assert_eq!(
            EXTRA_RANGE,
            (
                ENS_EXTRA_BASE,
                EXPL_EXTRA_BASE + EXPL_SLOT_COUNT as u16 * EXPL_EXTRA_STRIDE - 1
            ),
            "声明的尾界必须等于爆炸池额外面末字节"
        );
    }
}
