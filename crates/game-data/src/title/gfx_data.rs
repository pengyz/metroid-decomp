//! GFX 装载表 + 数据体 —— **从资产包读取**（ROM 派生数据已迁出源码）。
//!
//! 设计见 `docs/spec/data-code-separation.md`：仓库里只保留段 id 与访问器；
//! ROM 字节由用户本地 `pack-tool extract` 生成 `assets/metroid.pack`。
//!
//! ROM 锚（A 级）：`GFXInfo ; 07:C6E0`（`refs/m1disasm/SRC/prg7_engine.asm:1669`，
//! 7 B/项 = `[bank][src lo/hi][dest lo/hi][size lo/hi]`，共 29 项 203 B）；
//! 数据体由 `CopyGFXBlock ; 07:C7D5` 按描述子从各 bank 逐项搬运。
//!
//! 提取：`game-data` 的 `extract` feature（`crate::extract::sections`）——
//! **与这里的访问器共用同一个 `PackKey`**，不会漂移。

use crate::pack::{self, PackKey};

/// GFX 描述子条目数（`$C6E0-$C7AB` = 203 B = 29 × 7）。
pub const GFX_ENTRIES: usize = 29;

/// 描述子表（29 项 × 7 B，原样 ROM 字节）。
///
/// 缺包/缺段 ⇒ panic 并打印生成方法（fail-loud，见 `pack::get`）。
pub fn gfx_info() -> &'static [u8] {
    let d = pack::get(PackKey::GfxInfo);
    assert_eq!(
        d.len(),
        GFX_ENTRIES * 7,
        "gfx.info 段长度不符（包版本与代码不一致？期望 {} B，实际 {} B）",
        GFX_ENTRIES * 7,
        d.len()
    );
    d
}

/// GFX 数据体（29 项按描述子顺序拼接）。
pub fn gfx_data() -> &'static [u8] {
    pack::get(PackKey::GfxData)
}

/// 第 `id` 项的尺寸（描述子 `+5..+7`，小端）。
pub fn gfx_size(id: usize) -> Option<usize> {
    if id >= GFX_ENTRIES {
        return None;
    }
    let info = gfx_info();
    Some(u16::from_le_bytes([info[id * 7 + 5], info[id * 7 + 6]]) as usize)
}

/// 第 `id` 项在 [`gfx_data`] 内的 `(偏移, 尺寸)`。
///
/// 偏移是**现算**的（前 `id` 项尺寸累加）——原实现是烘焙一张 `GFX_OFFS` 表，
/// 那张表是描述子的纯派生量，属于代码而非数据（分离纪律：能算出来的不存）。
pub fn gfx_offs(id: usize) -> Option<(usize, usize)> {
    if id >= GFX_ENTRIES {
        return None;
    }
    let info = gfx_info();
    let mut off = 0usize;
    for i in 0..id {
        off += u16::from_le_bytes([info[i * 7 + 5], info[i * 7 + 6]]) as usize;
    }
    let size = u16::from_le_bytes([info[id * 7 + 5], info[id * 7 + 6]]) as usize;
    Some((off, size))
}

/// 数据体总长（= 29 项尺寸之和）——用于校验包内段与描述子自洽。
pub fn gfx_data_expected_len() -> usize {
    let info = gfx_info();
    (0..GFX_ENTRIES)
        .map(|i| u16::from_le_bytes([info[i * 7 + 5], info[i * 7 + 6]]) as usize)
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pack_ready() -> bool {
        pack::is_ready()
    }

    /// 段长自洽：29 项尺寸之和 == 数据体长度（无包时显式 SKIP）。
    #[test]
    fn gfx_data_len_matches_info_sizes() {
        if !pack_ready() {
            eprintln!("SKIP（显式）：未加载资产包——先跑 pack-tool extract");
            return;
        }
        assert_eq!(
            gfx_data_expected_len(),
            gfx_data().len(),
            "描述子尺寸和与数据体长度不符（包与代码版本不一致）"
        );
    }

    /// 逐项 offsets 连续且最后一项收尾于数据体末尾。
    #[test]
    fn gfx_offsets_are_contiguous() {
        if !pack_ready() {
            eprintln!("SKIP（显式）：未加载资产包——先跑 pack-tool extract");
            return;
        }
        let mut expect_off = 0usize;
        for id in 0..GFX_ENTRIES {
            let (off, size) = gfx_offs(id).expect("条目应存在");
            assert_eq!(off, expect_off, "第 {id} 项偏移应连续");
            assert!(size > 0, "第 {id} 项尺寸应非零");
            expect_off += size;
        }
        assert_eq!(expect_off, gfx_data().len(), "累计尺寸应等于数据体长度");
        assert!(gfx_offs(GFX_ENTRIES).is_none(), "越界应 None");
    }

    /// 首项尺寸锚（ROM 描述子 entry0 size=`AA 09`=0x09AA？——用现算值反查描述子，防写死）。
    #[test]
    fn first_entry_size_reads_descriptor_bytes() {
        if !pack_ready() {
            eprintln!("SKIP（显式）：未加载资产包——先跑 pack-tool extract");
            return;
        }
        let info = gfx_info();
        let expect = u16::from_le_bytes([info[5], info[6]]) as usize;
        assert_eq!(gfx_size(0), Some(expect));
        assert_eq!(gfx_offs(0), Some((0, expect)));
    }
}
