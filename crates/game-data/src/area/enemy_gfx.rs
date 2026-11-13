//! 敌帧/位数据面——**只做访问器，数据在资产包里**（段 id 见 `crate::pack::PackKey`）。
//!
//! 设计见 `docs/spec/data-code-separation.md`；数据面来源与表长定谳见
//! `docs/findings/enemy-gfx-baker-drift.md`。ROM 锚：`AreaPointers_ROM`（prg1:88）
//! 四字 EnAnimTable `$9D6A` / EnFramePtrTable1 `$9DE0` / EnFramePtrTable2 `$9EE0` /
//! EnPlacePtrTable `$9F0E`；`CopyAreaPointers $C8B0`（prg7:1936）搬 RAM `$3B-$48`。
//!
//! 表长由相邻标签地址机械给出：anim `$9DE0-$9D6A`=118 项；frame `$9F0E-$9DE0`=302 B
//! =151 项；place 表内 14 指针后即数据 ⇒ 14 项。帧流 `[ctrl][radiusY][radiusX][流…$FF]`。
//!
//! 帧/位项表由提取侧算好放进包（`en.foffs`/`en.poffs`）：帧流自界规则只在 `extract.rs`
//! 实现一份，运行侧不解析（避免第二处实现）。

use crate::pack::{self, PackKey};

/// 动画索引表项数（`$9D6A`→`$9DE0`）。
pub const N_ANIM: usize = 118;
/// 帧指针项数（`$9DE0`→`$9F0E` = 302 B / 2）。
pub const N_FRAMES: usize = 151;
/// 位件指针项数（`$9F0E` 表内 14 个指针后即数据）。
pub const N_PLACES: usize = 14;

/// `EnAnimTable`（`$FF`=动画结束、animIndex 回 reset）。
pub fn anim_table() -> &'static [u8] {
    pack::get(PackKey::EnAnimTable)
}

/// 帧指针表（151 项 ×2 B，LE）。
pub fn frame_ptrs() -> &'static [u8] {
    pack::get(PackKey::EnFramePtrs1)
}

/// 位件指针表（14 项 ×2 B，LE）。
pub fn place_ptrs() -> &'static [u8] {
    pack::get(PackKey::EnPlacePtrs)
}

/// 帧数据体（各帧流按项表顺序拼接）。
pub fn frames() -> &'static [u8] {
    pack::get(PackKey::EnFrames)
}

/// 位件数据体。
pub fn places() -> &'static [u8] {
    pack::get(PackKey::EnPlaces)
}

/// 第 `idx` 帧项 `(偏移, 长)`——越界 ⇒ `None`（旧烘焙器在 151..255 区间的垃圾项就此消失）。
pub fn frame_off(idx: usize) -> Option<(u32, u32)> {
    decode_off(pack::get(PackKey::EnFrameOffs), idx, N_FRAMES)
}

/// 第 `idx` 位件项 `(偏移, 长)`——越界 ⇒ `None`。
pub fn place_off(idx: usize) -> Option<(u32, u32)> {
    decode_off(pack::get(PackKey::EnPlaceOffs), idx, N_PLACES)
}

fn decode_off(table: &'static [u8], idx: usize, n: usize) -> Option<(u32, u32)> {
    if idx >= n {
        return None;
    }
    let base = idx * 8;
    let e = table.get(base..base + 8)?;
    Some((
        u32::from_le_bytes([e[0], e[1], e[2], e[3]]),
        u32::from_le_bytes([e[4], e[5], e[6], e[7]]),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 段形状与 ROM 锚（四表项数 + 首指针合法）。
    #[test]
    fn enemy_gfx_segments_shape() {
        assert_eq!(anim_table().len(), N_ANIM, "EnAnimTable 118 项（$9D6A→$9DE0）");
        assert_eq!(frame_ptrs().len(), N_FRAMES * 2, "帧指针 151 项×2B");
        assert_eq!(place_ptrs().len(), N_PLACES * 2, "位件指针 14 项×2B");
        assert!(!frames().is_empty());
        assert!(!places().is_empty());
    }

    /// 151 帧指针 + 14 位件指针**全部是合法 ROM 指针**（≥ `$8000`）——
    /// 这是表长定谳的机械判据：旧烘焙器按 128 项读时出现 34/44 个伪值，本断言会红。
    #[test]
    fn all_pointer_entries_are_legal_rom_pointers() {
        for (i, w) in frame_ptrs().chunks_exact(2).enumerate() {
            let p = u16::from_le_bytes([w[0], w[1]]);
            assert!(p >= 0x8000, "帧指针 {i} 非法: {p:04X}");
        }
        for (i, w) in place_ptrs().chunks_exact(2).enumerate() {
            let p = u16::from_le_bytes([w[0], w[1]]);
            assert!(p >= 0x8000, "位件指针 {i} 非法: {p:04X}");
        }
    }

    /// 项表自洽：非空项 `(off+len)` 落在数据体内；越界索引返回 `None`。
    #[test]
    fn offsets_fit_and_out_of_range_is_none() {
        for i in 0..N_FRAMES {
            let (o, l) = frame_off(i).expect("帧项齐备");
            if l > 0 {
                assert!((o + l) as usize <= frames().len(), "帧项 {i} 越出数据体");
            }
        }
        for i in 0..N_PLACES {
            let (o, l) = place_off(i).expect("位件项齐备");
            if l > 0 {
                assert!((o + l) as usize <= places().len(), "位件项 {i} 越出数据体");
            }
        }
        assert!(frame_off(N_FRAMES).is_none());
        assert!(place_off(N_PLACES).is_none());
    }

    /// 帧流自界形：非空帧项末字节恒 `$FF`、长 ≥3（防「空对空」假绿）。
    #[test]
    fn frame_streams_end_with_ff() {
        let body = frames();
        let mut checked = 0;
        for i in 0..N_FRAMES {
            let (o, l) = frame_off(i).expect("帧项齐备");
            if l == 0 {
                continue;
            }
            assert_eq!(body[(o + l) as usize - 1], 0xFF, "帧项 {i} 末字节应为 $FF");
            assert!(l >= 3, "帧项 {i} 至少 3B");
            checked += 1;
        }
        assert!(checked > 0, "至少要校验到一项");
    }
}
