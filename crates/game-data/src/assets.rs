//! 资产包接口——**数据访问的唯一对外面**。
//!
//! 设计见 `docs/spec/data-code-separation.md`。本模块只有三件职责：
//! ①生命周期（`init_from_env`/`is_ready`）②按段取出字节 ③把「字节 → 结构化项」的
//! **最薄**解码（LE u16/u32 对）暴露给引擎。
//!
//! **格式知识不在这里**：表长为什么是 118/151/14、帧流怎么自界、指针表为什么只能
//! 读前 N 项、段隔离怎么保证——全部在提取侧 `crate::extract`（`pack-tool` 腿），
//! 编译期由 `#[cfg(feature = "extract")]` 隔离，运行期不存在。本文件**不写字节字面量**。
//!
//! 逐面 ROM 锚（bank:addr）见各函数文档；等价性判据=包内字节 == 旧提交件常量字节。

use crate::pack::{self, PackKey};

// ---- 生命周期 -------------------------------------------------------------------

/// 从候选路径加载资产包（`$METROID_PACK` → `assets/metroid.pack` → 编译期源目录 → exe 同级）。
pub fn init_from_env() -> Result<(), pack::PackError> {
    pack::init_from_env()
}

/// 资产包是否已就绪（未就绪时任何取段都会 fail-loud）。
pub fn is_ready() -> bool {
    pack::is_ready()
}

// ---- gfx（`GFXInfo` 29 项 + 数据体）-----------------------------------------------

/// `GFXInfo` 全表 29 项 × 7 B（`LoadGFX: ; 07:C7AB` 的 `ldy #7*id / lda GFXInfo,y`）。
pub fn gfx_info() -> &'static [u8] {
    pack::get(PackKey::GfxInfo)
}

/// 29 项图形数据体的拼接平面（偏移由 [`gfx_offs`] 给出）。
pub fn gfx_data() -> &'static [u8] {
    pack::get(PackKey::GfxData)
}

/// 第 `id` 项图形块的 `(偏移, 尺寸)`——`id` 越界 ⇒ `None`。
pub fn gfx_offs(id: usize) -> Option<(usize, usize)> {
    pack::gfx_block_offs(id)
}

/// 第 `id` 项图形块（`CopyGFXBlock: ; 07:C7D5` 的源字节）。
pub fn gfx_block(id: usize) -> Option<&'static [u8]> {
    let (off, size) = gfx_offs(id)?;
    let data = gfx_data();
    data.get(off..off + size)
}

// ---- 敌帧/位件（bank1 `$9D6A`-`$9F0E` 族）----------------------------------------

/// `EnAnimTable`（`$FF`=动画结束、animIndex 回 reset）。
pub fn enemy_anim_table() -> &'static [u8] {
    pack::get(PackKey::EnAnimTable)
}

/// 帧指针表（LE u16 项，项数见 [`crate::extract`] 的表长定谳）。
pub fn enemy_frame_ptrs() -> &'static [u8] {
    pack::get(PackKey::EnFramePtrs1)
}

/// 位件指针表（LE u16 项）。
pub fn enemy_place_ptrs() -> &'static [u8] {
    pack::get(PackKey::EnPlacePtrs)
}

/// 帧数据体。
pub fn enemy_frames() -> &'static [u8] {
    pack::get(PackKey::EnFrames)
}

/// 位件数据体。
pub fn enemy_places() -> &'static [u8] {
    pack::get(PackKey::EnPlaces)
}

/// 第 `idx` 帧项 `(偏移, 长)`——越界 ⇒ `None`。
pub fn enemy_frame_off(idx: usize) -> Option<(u32, u32)> {
    pack::offs_entry(PackKey::EnFrameOffs, idx)
}

/// 第 `idx` 位件项 `(偏移, 长)`——越界 ⇒ `None`。
pub fn enemy_place_off(idx: usize) -> Option<(u32, u32)> {
    pack::offs_entry(PackKey::EnPlaceOffs, idx)
}

// ---- 世界地图 -------------------------------------------------------------------

/// `WorldMap` 1 KB（`WorldMap: ; 00:A53E`；32×32 表，行=MapY 列=MapX，`$FF`=无房）。
pub fn world_map() -> &'static [u8] {
    pack::get(PackKey::WorldMap)
}


// ---- 区域房流（Brinstar / Norfair）------------------------------------------------
//
// 变长数组的索引（`(off,len)`）在包内；这里用 `LazyLock` 一次性切成
// `&'static [&'static [u8]]`，让消费侧保持「按下标取一段」的写法而不必自己解码。

use std::sync::LazyLock;

fn slices(body: &'static [u8], offs: &'static [u8], n: usize) -> Vec<&'static [u8]> {
    let mut v = Vec::with_capacity(n);
    for i in 0..n {
        let Some(e) = offs.get(i * 8..i * 8 + 8) else { break };
        let o = u32::from_le_bytes([e[0], e[1], e[2], e[3]]) as usize;
        let l = u32::from_le_bytes([e[4], e[5], e[6], e[7]]) as usize;
        match body.get(o..o + l) {
            Some(s) => v.push(s),
            None => break,
        }
    }
    v
}

static BR_ROOMS: LazyLock<Vec<&'static [u8]>> = LazyLock::new(|| {
    slices(pack::get(PackKey::BrRoomStreams), pack::get(PackKey::BrRoomOffs), 47)
});
static BR_STRUCTS: LazyLock<Vec<&'static [u8]>> = LazyLock::new(|| {
    slices(pack::get(PackKey::BrStructures), pack::get(PackKey::BrStructOffs), 50)
});
static NO_ROOMS: LazyLock<Vec<&'static [u8]>> = LazyLock::new(|| {
    slices(pack::get(PackKey::NoRoomStreams), pack::get(PackKey::NoRoomOffs), 46)
});
static NO_STRUCTS: LazyLock<Vec<&'static [u8]>> = LazyLock::new(|| {
    slices(pack::get(PackKey::NoStructures), pack::get(PackKey::NoStructOffs), 49)
});

/// 区序号（`InArea & 0x0F`：0=Brinstar / 1=Norfair）——其余区未迁包 ⇒ `None`。
fn area_idx(area: u8) -> Option<u8> {
    match area & 0x0F {
        0 => Some(0),
        1 => Some(1),
        _ => None,
    }
}

/// 房流指针表（u16 LE × N；N 由区决定：Brinstar 47 / Norfair 46）。
pub fn room_ptrs(area: u8) -> Option<&'static [u8]> {
    match area_idx(area)? {
        0 => Some(pack::get(PackKey::BrRoomPtrs)),
        _ => Some(pack::get(PackKey::NoRoomPtrs)),
    }
}

/// 第 `idx` 个房指针（LE u16）——越界/未迁区 ⇒ `None`。
pub fn room_ptr(area: u8, idx: usize) -> Option<u16> {
    let t = room_ptrs(area)?;
    let e = t.get(idx * 2..idx * 2 + 2)?;
    Some(u16::from_le_bytes([e[0], e[1]]))
}

/// 第 `idx` 个结构体指针（LE u16）。
pub fn struct_ptr(area: u8, idx: usize) -> Option<u16> {
    let t = struct_ptrs(area)?;
    let e = t.get(idx * 2..idx * 2 + 2)?;
    Some(u16::from_le_bytes([e[0], e[1]]))
}

/// 结构体指针表（Brinstar 50 / Norfair 49）。
pub fn struct_ptrs(area: u8) -> Option<&'static [u8]> {
    match area_idx(area)? {
        0 => Some(pack::get(PackKey::BrStructPtrs)),
        _ => Some(pack::get(PackKey::NoStructPtrs)),
    }
}

/// 元tile 定义（4 B/组：Brinstar 260 B / Norfair 248 B）。
pub fn metatile_defs(area: u8) -> Option<&'static [u8]> {
    match area_idx(area)? {
        0 => Some(pack::get(PackKey::BrMetatiles)),
        _ => Some(pack::get(PackKey::NoMetatiles)),
    }
}

/// 房流（按房号下标；每段自界于 `$FF`）。
pub fn room_streams(area: u8) -> Option<&'static [&'static [u8]]> {
    match area_idx(area)? {
        0 => Some(&BR_ROOMS),
        _ => Some(&NO_ROOMS),
    }
}

/// 结构体（按结构号下标）。
pub fn structures(area: u8) -> Option<&'static [&'static [u8]]> {
    match area_idx(area)? {
        0 => Some(&BR_STRUCTS),
        _ => Some(&NO_STRUCTS),
    }
}


// ---- 敌九表（当前只有 Brinstar 一套；其余四区待迁）--------------------------------
//
// asm 形：`LoadEnemy` 按 `EnemyType` 索引各表（`crates/game-core/src/frame/room.rs`），
// 表址 brinstar `prg1_brinstar.asm:195-421`（$962B…$977B）。**表长口径**：健康/强制速度/
// 初始延迟/朝向在 asm 相邻标签间距各 16 B，本仓历史烘焙 64 B（跨表连读）——搬家不改语义，
// 长度另案（`enemy-table-extent`）。
macro_rules! enemy_table {
    ($fname:ident, $key:ident) => {
        /// Brinstar 敌表（未就绪即 panic：调用点只在游戏内，早已 `init_from_env`）。
        pub fn $fname() -> &'static [u8] {
            pack::get(PackKey::$key)
        }
    };
}
enemy_table!(enemy_health, BrEnemyHealth);
enemy_table!(enemy_resting_anim, BrEnemyRestingAnim);
enemy_table!(enemy_active_anim, BrEnemyActiveAnim);
enemy_table!(enemy_active_anim_init_offset, BrEnemyActiveAnimInitOffset);
enemy_table!(enemy_force_attribs, BrEnemyForceAttribs);
enemy_table!(enemy_force_speed_delay, BrEnemyForceSpeedDelay);
enemy_table!(enemy_distance_threshold, BrEnemyDistanceThreshold);
enemy_table!(enemy_init_delay, BrEnemyInitDelay);
enemy_table!(enemy_facing, BrEnemyFacing);


// ---- ROM 查询表（窗口参数化）------------------------------------------------------
//
// asm `bank07:F29E` `LDA $968B,Y`（属性表，Y<0x80）/ `bank07:F2ED` 搬 `$95CE/$95CF`
// 流向量 / `bank07:C19F` NT 高位表。窗口语义=**按区域 bank 查表**（decode-sweep r3
// 动态定谳：game 期 `$8000` 窗口=banks1-5），故按 8 bank 全量存，`window_bank` 取数。

/// `$968B` 属性表（`window_bank` × 128B）。
pub fn rom_table_968b(window_bank: usize, y: u8) -> u8 {
    let t = pack::get(PackKey::RomTable968B);
    t[window_bank * 0x80 + (y & 0x7F) as usize]
}

/// `$95CE/$95CF` 流向量 `[lo, hi]`。
pub fn rom_vec(window_bank: usize) -> (u8, u8) {
    let t = pack::get(PackKey::RomVec95CE);
    (t[window_bank * 2], t[window_bank * 2 + 1])
}

/// PPU NT 高位表（`$20/$24/$28/$2C`）。
pub fn nametable_hi() -> &'static [u8] {
    pack::get(PackKey::NametableHi)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 包未就绪时也算通过的形状测试集（各段长度自洽）。
    #[test]
    fn segments_have_expected_shapes() {
        if !is_ready() {
            eprintln!("SKIP（显式）：资产包未加载——先跑 pack-tool extract");
            return;
        }
        assert_eq!(gfx_info().len(), 29 * 7, "GFXInfo 29 项×7B");
        assert!(!gfx_data().is_empty());
        assert!(gfx_block(0).is_some(), "0 号图形块应存在");
        assert!(gfx_block(29).is_none(), "越界 id 应 None");
        assert_eq!(world_map().len(), 1024, "WorldMap 1 KB");
    }

    /// GFX：描述子尺寸和 == 数据体长度，且偏移连续（形状自洽）。
    #[test]
    fn gfx_offsets_are_contiguous_and_complete() {
        if !is_ready() {
            eprintln!("SKIP（显式）：资产包未加载——先跑 pack-tool extract");
            return;
        }
        let mut expect = 0usize;
        for id in 0..29 {
            let (off, size) = gfx_offs(id).expect("29 项应齐备");
            assert_eq!(off, expect, "第 {id} 项偏移应连续");
            assert!(size > 0, "第 {id} 项尺寸应非零");
            expect += size;
        }
        assert_eq!(expect, gfx_data().len(), "累计尺寸应等于数据体长度");
    }

    /// GFX：描述子尺寸和 == 数据体长度（段与描述子自洽；包/代码版本不一致即红）。
    #[test]
    fn gfx_data_len_matches_info_sizes() {
        if !is_ready() {
            eprintln!("SKIP（显式）：资产包未加载——先跑 pack-tool extract");
            return;
        }
        let info = gfx_info();
        let sum: usize = (0..29)
            .map(|i| usize::from(u16::from_le_bytes([info[i * 7 + 5], info[i * 7 + 6]])))
            .sum();
        assert_eq!(sum, gfx_data().len(), "描述子尺寸和与数据体长度不符");
    }

    /// 敌面段形状：118 动画项 / 151 帧指针×2B / 14 位件指针×2B（表长定谳的形状面）。
    #[test]
    fn enemy_segments_shape_matches_table_extent() {
        if !is_ready() {
            eprintln!("SKIP（显式）：资产包未加载——先跑 pack-tool extract");
            return;
        }
        assert_eq!(enemy_anim_table().len(), 118, "EnAnimTable 118 项（$9D6A→$9DE0）");
        assert_eq!(enemy_frame_ptrs().len(), 151 * 2, "帧指针 151 项×2B");
        assert_eq!(enemy_place_ptrs().len(), 14 * 2, "位件指针 14 项×2B");
        assert!(!enemy_frames().is_empty() && !enemy_places().is_empty());
    }

    /// 首项尺寸必须从描述子 `+5..+7` 读（守 `GFXInfo` 的 7 B 列义——
    /// 曾经误用 `rec[0]` 当尺寸，本断言会红）。
    #[test]
    fn first_entry_size_reads_descriptor_size_column() {
        if !is_ready() {
            eprintln!("SKIP（显式）：资产包未加载——先跑 pack-tool extract");
            return;
        }
        let info = gfx_info();
        let expect = usize::from(u16::from_le_bytes([info[5], info[6]]));
        let (off, size) = gfx_offs(0).expect("0 号项应存在");
        assert_eq!(off, 0);
        assert_eq!(size, expect, "0 号尺寸应等于描述子 +5..+7");
    }

    /// 敌面：151 帧指针 + 14 位件指针**全部是合法 ROM 指针**（≥ `$8000`）——
    /// 表长定谳的机械判据（旧 128/128/128 读法会产生 34/44 个伪值）。
    #[test]
    fn enemy_pointers_are_legal_rom_pointers() {
        if !is_ready() {
            eprintln!("SKIP（显式）：资产包未加载——先跑 pack-tool extract");
            return;
        }
        let fp = enemy_frame_ptrs();
        let pp = enemy_place_ptrs();
        assert_eq!(fp.len() % 2, 0, "帧指针表应为 u16 项");
        assert_eq!(pp.len() % 2, 0, "位件指针表应为 u16 项");
        for (i, w) in fp.chunks_exact(2).enumerate() {
            let p = u16::from_le_bytes([w[0], w[1]]);
            assert!(p >= 0x8000, "帧指针 {i} 非法: {p:04X}");
        }
        for (i, w) in pp.chunks_exact(2).enumerate() {
            let p = u16::from_le_bytes([w[0], w[1]]);
            assert!(p >= 0x8000, "位件指针 {i} 非法: {p:04X}");
        }
    }

    /// 敌面项表自洽 + 帧流末字节恒 `$FF`（防「空对空」假绿）。
    #[test]
    fn enemy_offsets_fit_and_streams_end_with_ff() {
        if !is_ready() {
            eprintln!("SKIP（显式）：资产包未加载——先跑 pack-tool extract");
            return;
        }
        let body = enemy_frames();
        let mut checked = 0;
        let mut i = 0;
        while let Some((o, l)) = enemy_frame_off(i) {
            if l > 0 {
                assert!((o + l) as usize <= body.len(), "帧项 {i} 越出数据体");
                assert_eq!(body[(o + l) as usize - 1], 0xFF, "帧项 {i} 末字节应为 $FF");
                checked += 1;
            }
            i += 1;
        }
        assert!(checked > 0, "至少要校验到一项");
        assert!(enemy_frame_off(i).is_none(), "越界索引应 None");
    }
}

/// `MusicNotesTbl ; $BE77`（64 音符×2B：高/低 period 对；低字节 0=休止）。
pub fn music_notes() -> &'static [u8] {
    pack::get(PackKey::MusicNotes)
}

/// `NoteLengthsTbl ; $BEF7` @4 组（11B，PowerUp/Kraid）——组偏移=曲头 Byte0 `$00`。
pub fn note_lengths_4() -> &'static [u8] {
    pack::get(PackKey::NoteLen4)
}

/// 同表 @6 组（12B，区域曲族）——曲头 Byte0 `$0B`。
pub fn note_lengths_6() -> &'static [u8] {
    pack::get(PackKey::NoteLen6)
}

/// 同表 @7 组（12B，intro/end）——曲头 Byte0 `$17`。
pub fn note_lengths_7() -> &'static [u8] {
    pack::get(PackKey::NoteLen7)
}

/// 三组长度表按全表下标直索（asm `adc NoteLengthTblOffset` 直索全表，35B 连续）。
/// 组界：`@4` 11B / `@6` 12B / `@7` 12B（=曲头 Byte0 `$00/$0B/$17`）。
pub fn note_length_at(index: u8) -> u8 {
    let i = index as usize;
    if i < 11 {
        note_lengths_4()[i]
    } else if i < 23 {
        note_lengths_6()[i - 11]
    } else {
        note_lengths_7().get(i - 23).copied().unwrap_or(0)
    }
}

// ── 乐数据流 / 包络（格式解释在 extract.rs；本层只做解码） ──────────────────

fn decode_slices(body: &'static [u8], offs: &'static [u8], n: usize) -> Vec<&'static [u8]> {
    (0..n)
        .map(|i| {
            let o = u32::from_le_bytes(offs[i * 8..i * 8 + 4].try_into().unwrap()) as usize;
            let l = u32::from_le_bytes(offs[i * 8 + 4..i * 8 + 8].try_into().unwrap()) as usize;
            &body[o..o + l]
        })
        .collect()
}

static SONG_STREAMS: LazyLock<Vec<&'static [u8]>> =
    LazyLock::new(|| decode_slices(pack::get(PackKey::SongStreams), pack::get(PackKey::SongStreamOffs), 48));
static VOLUME_ENVELOPES: LazyLock<Vec<&'static [u8]>> =
    LazyLock::new(|| decode_slices(pack::get(PackKey::EnvBody), pack::get(PackKey::EnvOffs), 5));

/// 12 曲×4 通道基址字（头[5..13] 原值，LE u16；0=空通道）。
pub fn stream_bases() -> &'static [u8] {
    pack::get(PackKey::StreamBases)
}

/// 按曲号（`SongHeaderOffsetTable` 序）×通道（0=SQ1/1=SQ2/2=Tri/3=Noise）取流。
pub fn song_stream(song: usize, ch: usize) -> Option<&'static [u8]> {
    if song >= 12 || ch >= 4 {
        return None;
    }
    let s = SONG_STREAMS[song * 4 + ch];
    if s.is_empty() { None } else { Some(s) }
}

/// 按**基址**反查流（忠实径：乐循环读 `$0630+2X` 基址字取流——曲号会被
/// `FindMusicInitIndex` 重置，不可作键）。
pub fn stream_by_base(base: u16) -> Option<&'static [u8]> {
    let b = stream_bases();
    for i in 0..48 {
        let v = u16::from_le_bytes(b[i * 2..i * 2 + 2].try_into().unwrap());
        if v == base && base != 0 {
            return song_stream(i / 4, i % 4);
        }
    }
    None
}

/// `VolumeEnvelopePtrTable ; $BCB0`（5 word 原值）。
pub fn volume_envelope_ptrs() -> &'static [u8] {
    pack::get(PackKey::EnvPtrs)
}

/// 第 i 条音量包络（至 `$FF`/`$F0` 含尾）；i 越界返空切片。
pub fn volume_envelope(i: usize) -> &'static [u8] {
    VOLUME_ENVELOPES.get(i).copied().unwrap_or(&[])
}

/// `SFXData ; $B200` 鼓点表（16B）。
pub fn sfx_data() -> &'static [u8] {
    pack::get(PackKey::SfxData)
}
