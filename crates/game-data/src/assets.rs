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

// ── 调色板（格式解释在 extract.rs；各 bank 项数=asm 表长事实） ─────────────

/// PalettePtrTable 各 bank 项数（bank0 标题=19 / bank1-5 区域=28；`prg0_title.asm:3201`）。
pub const PALETTE_ENTRIES: [usize; 6] = [19, 28, 28, 28, 28, 28];

fn pal_bank_span(bank: usize) -> Option<(usize, usize)> {
    if bank >= 6 {
        return None;
    }
    let before: usize = PALETTE_ENTRIES[..bank].iter().sum();
    Some((before, PALETTE_ENTRIES[bank]))
}

/// `PalettePtrTable ; $9560` 该 bank 的指针原值（LE u16 字节面，`2*项数` 长）。
pub fn palette_ptrs(bank: usize) -> &'static [u8] {
    match pal_bank_span(bank) {
        Some((before, n)) => &pack::get(PackKey::PalPtrs)[before * 2..(before + n) * 2],
        None => &[],
    }
}

/// 该 bank 第 idx 条调色板记录流（`[hi][lo][ctrl][data…]`，`hi==0` 含尾）。
pub fn palette_stream(bank: usize, idx: usize) -> Option<&'static [u8]> {
    let (before, n) = pal_bank_span(bank)?;
    if idx >= n {
        return None;
    }
    let offs = pack::get(PackKey::PalOffs);
    let body = pack::get(PackKey::PalData);
    let i = before + idx;
    let o = u32::from_le_bytes(offs[i * 8..i * 8 + 4].try_into().unwrap()) as usize;
    let l = u32::from_le_bytes(offs[i * 8 + 4..i * 8 + 8].try_into().unwrap()) as usize;
    Some(&body[o..o + l])
}

#[cfg(test)]
mod palette_tests {
    use super::*;

    /// sprite blob：三表界 = `$860B-$8572`/`$86DF-$860B`，且覆盖帧 0x28 记录（截断回归锚）。
    #[test]
    fn sprite_blob_covers_frame_records() {
        let sd = sprite_data();
        assert_eq!(sd.len(), 1457, "blob 长度（$8B23-$8572）");
        assert_eq!(OBJ_FRAME_PTR_TABLE - OBJ_ANIM_INDEX_TBL, 0x860B - 0x8572);
        assert_eq!(OBJ_PLACE_PTR_TABLE - OBJ_FRAME_PTR_TABLE, 0x86DF - 0x860B);
        let off = 0x890Ausize - 0x8572;
        assert!(sd.len() > off + 4, "烘培覆盖帧 0x28 记录");
        assert_eq!(&sd[off..off + 5], &[0x04, 0x02, 0x02, 0x30, 0xFF], "ROM $890A 逐字节");
    }

    /// 特项表：五区条数与 asm `SpecItmsTbl_*` 走表一致（15/33/9/35/6 = 98 条），
    /// 且首条与 ROM 同格（BRINSTAR bank1:$A3D6 → y=02 x=03 type=04 args=[03,00,00]）。
    #[test]
    fn item_tables_match_asm_extent_and_first_record() {
        let counts = [15usize, 33, 9, 35, 6];
        let mut total = 0;
        for (area, &n) in counts.iter().enumerate() {
            let items = items(area as u8);
            assert_eq!(items.len(), n, "area{area} 条数");
            total += n;
        }
        assert_eq!(total, 98, "五区合计");
        let br = items(0);
        assert_eq!(
            br[0],
            crate::area::items::SpecialItem {
                map_y: 0x02,
                map_x: 0x03,
                item_type: 0x04,
                args: [0x03, 0x00, 0x00]
            },
            "ROM $A3D6 首条（asm @@x03=$A3D9）"
        );
        assert!(items(5).is_empty(), "越界区返空");
        // 分派号只允许 1..=0x0A（ChooseSpawningRoutine prg7:9322 表内）；
        // 高半字节是 ROM 原值附加位（实测 NORFAIR 有 `$41`，分派侧只看低半字节）。
        for area in 0..5u8 {
            for it in items(area) {
                assert!(
                    (1..=0x0A).contains(&(it.item_type & 0x0F)),
                    "area{area} 未知 it_Type {:#04X}",
                    it.item_type
                );
            }
        }
    }

    /// Samus 三张小表：`ActionTable` 4 B / PntUp 状态表 3 B / `RunAccelerationTbl` 2 B。
    /// 旧 `RUN_ACCEL_TBL [u8;256]` 自 `$CCC0` 起吞了后续代码（`$A6 $4D`=`ldx $4D`），
    /// 本测同时挡住"再退回长窗烘焙"。
    #[test]
    fn samus_tables_match_rom_anchors() {
        assert_eq!(action_table(), &[1, 1, 3, 4], "ActionTable $CC87 = sa_Run/sa_Run/sa_Roll/sa_PntUp");
        assert_eq!(pnt_up_status_table(), &[1, 1, 3], "PntUp 状态表 $D1EB");
        assert_eq!(run_accel_tbl(), &[0x30, 0xD0], "RunAccelerationTbl $CCC0（真长 2 B）");
        assert_eq!(run_accel_tbl().len(), 2, "表界：$CCC0+2 已是 SetSamusRunAccel 之后的代码");
    }

    /// 标题屏 VRAMStruct 四段：各段自界（`hi==0` 终止含尾）+ 首记录 ROM 锚。
    #[test]
    fn title_vram_structs_match_rom_anchors() {
        let intro = vram_intro();
        assert_eq!(intro.len(), 1066, "INTRO_BG $82F4..（含终止）");
        assert_eq!(&intro[0..4], &[0x23, 0xC0, 0x20, 0x00], "首记录 $23C0 属性表 32B");
        assert_eq!(*intro.last().unwrap(), 0x00, "VRAMStructEnd");
        assert_eq!(vram_start_continue().len(), 20, "START_CONTINUE $9984");
        assert_eq!(&vram_start_continue()[0..3], &[0x21, 0x8C, 0x05], "行 $218C 拳\"START\"（asm :3534）");
        assert_eq!(vram_password_please().len(), 36, "PASSWORD_PLEASE $99E3");
        assert_eq!(&vram_password_please()[0..3], &[0x20, 0x88, 0x10], "\"PASS WORD PLEASE\" 行 $2088（16 字符）");
        assert_eq!(vram_game_over().len(), 13, "GAME_OVER $93B9");
        assert_eq!(&vram_game_over()[0..3], &[0x21, 0x8C, 0x09], "\"GAME OVER\" 行 $218C（9 字符）");
        for (name, seg) in [
            ("intro", intro),
            ("start", vram_start_continue()),
            ("pass", vram_password_please()),
            ("over", vram_game_over()),
        ] {
            assert_eq!(*seg.last().unwrap(), 0x00, "{name} 段须以 hi==0 终止含尾");
        }
    }

    /// 声表：SFX 4B 记录窗（`$B211..$B289`，120 B，按绝对址引用）+
    /// 曲头表（`$BD31`，12×13B）与偏移表（`$BBFA`，落在 `sound.region` 窗内）。
    #[test]
    fn sound_tables_match_rom_anchors() {
        assert_eq!(sfx_tables().len(), 0x78, "$B289-$B211");
        assert_eq!(sfx_record4(0xB211).unwrap(), &[0x35, 0x7F, 0x00, 0xB0], "W3[0] ScrewAttack");
        assert_eq!(sfx_record4(0xB225).unwrap(), &[0xC1, 0x89, 0x02, 0x0F], "SamusHit");
        assert_eq!(sfx_record4(0xB279).unwrap(), &[0x09, 0x7F, 0x30, 0x48], "Beep");
        assert_eq!(sfx_record4(0xB285).unwrap(), &[0x7F, 0x7F, 0x30, 0xB2], "W4[6]");
        assert!(sfx_record4(0xB20F).is_none(), "窗外低址（$B200 SFXData 头）");
        assert!(sfx_record4(0xB286).is_none(), "窗外高址（跨窗尾）");
        assert_eq!(song_headers().len(), 12 * 13);
        assert_eq!(&song_headers()[0..5], &[0x0B, 0xFF, 0xF5, 0x00, 0x00], "表首行 ROM $BD31");
        assert_eq!(song_header_offsets().len(), 12);
        assert_eq!(
            &song_header_offsets()[0..6],
            &[0x41, 0x8F, 0x34, 0x27, 0x1A, 0x0D],
            "SongHeaderOffsetTable $BBFA"
        );
    }

    /// 表长事实（asm 锚）：bank0 标题 19 项 / bank1-5 区域 28 项 = 159 项；
    /// 包内 `pal.ptrs` 必须恰为 `2*159` 字节（不自洽即迁移出错）。
    #[test]
    fn palette_table_extent_matches_asm_anchor() {
        let total: usize = PALETTE_ENTRIES.iter().sum();
        assert_eq!(total, 159, "19 + 28*5");
        assert_eq!(palette_ptrs(0).len(), 19 * 2);
        assert_eq!(palette_ptrs(5).len(), 28 * 2);
        assert_eq!(palette_ptrs(6).len(), 0, "越界 bank 返空");
        // 每条流必须自界于 `hi==0`（含尾），且 bank1 数据体非空
        for bank in 0..6 {
            let n = PALETTE_ENTRIES[bank];
            for idx in 0..n {
                let s = palette_stream(bank, idx).expect("段内索引应可取");
                assert_eq!(*s.last().unwrap(), 0x00, "bank{bank} idx{idx} 未自界");
            }
            assert!(palette_stream(bank, n).is_none(), "表外索引必须 None（fail-loud 面）");
        }
        assert!(!palette_stream(1, 0).unwrap().is_empty(), "bank1 数据体非空");
        // suitless Palette1B（bank2 idx27）在位（原烘焙锚测的就地接管）
        assert_ne!(palette_ptrs(2)[27 * 2], 0, "Palette1B 指针低位非零");
    }
}

/// sprite 数据 blob（`common_sprite_data.asm`；区 bank 共载 `$8572`）。
// ---- 区特殊物品表（SpecItmsTbl）--------------------------------------------------
//
// 包里是扁平 6 B 记录 `[map_y, map_x, item_type, arg0, arg1, arg2]`（Y 序链表已在提取侧
// 展平）。这里**一次性**解码成 `SpecialItem` 切片并缓存：消费侧 `ScanForItems` 每进房扫一遍，
// 不宜逐帧分配，也不该把 6 B 记录布局漏给调用方。

static ITEMS: LazyLock<[Vec<crate::area::items::SpecialItem>; 5]> = LazyLock::new(|| {
    let keys = [PackKey::BrItems, PackKey::NoItems, PackKey::KrItems,
                PackKey::ToItems, PackKey::RiItems];
    let mut out: [Vec<crate::area::items::SpecialItem>; 5] = Default::default();
    for (slot, key) in out.iter_mut().zip(keys) {
        let body = pack::get(key);
        assert!(body.len() % 6 == 0, "{}: 记录长 {} 不是 6 的倍数", key.name(), body.len());
        for r in body.chunks_exact(6) {
            slot.push(crate::area::items::SpecialItem {
                map_y: r[0],
                map_x: r[1],
                item_type: r[2],
                args: [r[3], r[4], r[5]],
            });
        }
    }
    out
});

/// 区序（`InArea & $0F`：0=Brinstar / 1=Norfair / 2=Kraid / 3=Tourian / 4=Ridley）→ 特项表。
pub fn items(area: u8) -> &'static [crate::area::items::SpecialItem] {
    ITEMS.get((area & 0x0F) as usize).map(|v| v.as_slice()).unwrap_or(&[])
}

pub fn sprite_data() -> &'static [u8] {
    pack::get(PackKey::SpriteData)
}

/// 声引擎区窗口（`bank00:$B289..$BD09`，2688 B）。
///
/// 这是**地址窗口**而非逻辑表：声链代码以 `win - $B289` 索引区内的块描述子（5B）、
/// 16B 参数表与单字节参数。不要把区内的独立表（乐流/包络/音高表）当成它的子集来读——
/// 那些各有自己的段与访问器。
pub fn sound_region() -> &'static [u8] {
    pack::get(PackKey::SndRegion)
}

/// SFX 4B 记录窗基址（`$B211`）；窗口内记录以**绝对 ROM 址**引用。
pub const SFX_TABLES_ADDR: u16 = 0xB211;

/// 声引擎区窗口基址（`$B289`）；区内寻址一律 `addr - SOUND_REGION_ADDR`。
pub const SOUND_REGION_ADDR: u16 = 0xB289;

/// SFX 4B 记录窗（`$B211..$B289`，120 B）。
pub fn sfx_tables() -> &'static [u8] {
    pack::get(PackKey::SfxTables)
}

/// 窗内 4B 记录（`addr` 为 ROM 绝对址；越界返 `None`——调用点都是表内常量址）。
pub fn sfx_record(addr: u16) -> Option<&'static [u8]> {
    let off = addr.checked_sub(SFX_TABLES_ADDR)? as usize;
    sfx_tables().get(off..off + 4)
}

/// 窗内 4B 记录（定长数组形——`InitSFXData`/`LoadSFXData` 的 src 参数就是这个形状）。
pub fn sfx_record4(addr: u16) -> Option<&'static [u8; 4]> {
    let off = addr.checked_sub(SFX_TABLES_ADDR)? as usize;
    sfx_tables().get(off..off + 4)?.try_into().ok()
}

/// `ActionTable ; 07:CC87`（4 B：`sa_Run/sa_Run/sa_Roll/sa_PntUp`）——
/// `status = ActionTable[BitScan(方向位)]`。
pub fn action_table() -> &'static [u8] {
    pack::get(PackKey::SamusActions)
}

/// PntUp 状态表 @ `07:D1EB`（3 B：右/左→Run、下→Roll）。
pub fn pnt_up_status_table() -> &'static [u8] {
    pack::get(PackKey::SamusPntUp)
}

/// `RunAccelerationTbl ; 07:CCC0`（2 B：`$30` 右 / `$D0` 左）。
/// 旧 `[u8;256]` 烘焙吞了后续代码，已按 asm 真长更正（消费侧只索引 0/1，行为不变）。
pub fn run_accel_tbl() -> &'static [u8] {
    pack::get(PackKey::SamusAccel)
}

/// 标题屏 VRAMStruct blob（四态，各自以 `hi==0` 终止含尾）。
/// `VRAMStructWriteData` 07:C2E4 消费；记录三态 `[hi,lo,ctrl,…]`。
pub fn vram_intro() -> &'static [u8] {
    pack::get(PackKey::VramIntro)
}

pub fn vram_start_continue() -> &'static [u8] {
    pack::get(PackKey::VramStart)
}

pub fn vram_password_please() -> &'static [u8] {
    pack::get(PackKey::VramPass)
}

pub fn vram_game_over() -> &'static [u8] {
    pack::get(PackKey::VramOver)
}

/// `SongHeaders ; $BD31` 12×13B（曲头表本体）。
pub fn song_headers() -> &'static [u8] {
    pack::get(PackKey::SongHdrs)
}

/// `SongHeaderOffsetTable ; $BBFA` 12B（`MusicInitIndex` → 曲头在表内偏移）。
pub fn song_header_offsets() -> &'static [u8] {
    let off = (0xBBFA - SOUND_REGION_ADDR) as usize;
    sound_region().get(off..off + 12).unwrap_or(&[])
}

/// 三表在 blob 内的偏移（`$8572` 基）：anim 索引 / 帧指针 / 放置指针。
pub const OBJ_ANIM_INDEX_TBL: usize = 0x0000; // $8572
pub const OBJ_FRAME_PTR_TABLE: usize = 0x0099; // $860B
pub const OBJ_PLACE_PTR_TABLE: usize = 0x016D; // $86DF
