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

/// 资产包是否已就绪（**只查状态，不触发加载**）。
pub fn is_ready() -> bool {
    pack::is_ready()
}

/// 先按同一候选路径表惰性加载一次，再回答是否就绪。
///
/// 与取段访问器走同一条加载路径且幂等 ⇒ 返回值只取决于包文件是否存在，与「先跑者
/// 有没有取过段」无关。测试前置门必须用它（用 [`is_ready`] 会让同一测试在串行/并行下
/// 走出不同的 SKIP/执行分支，即执行序敏感）。
pub fn ensure_ready() -> bool {
    pack::ensure_ready()
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

/// 一个区的敌帧/位数据面（`EnAnimTable` / 帧指针 / 位件指针 / 帧体 / 位件体 + 两套索引）。
///
/// **按区**（`docs/findings/per-bank-assets.md` §1）：五区的基址与表长各不相同，
/// 旧形是 bank1 单表 ⇒ 非 Brinstar 敌人的 `animIndex` 查错表。
#[derive(Clone, Copy)]
pub struct EnemySprites {
    pub anim: &'static [u8],
    pub fptr: &'static [u8],
    pub pptr: &'static [u8],
    pub frames: &'static [u8],
    pub places: &'static [u8],
    pub foffs: &'static [u8],
    pub poffs: &'static [u8],
}

static EN_SPR: LazyLock<[EnemySprites; 5]> = LazyLock::new(|| {
    const K: [PackKey; 5] = [
        PackKey::BrEnSprites,
        PackKey::NoEnSprites,
        PackKey::KrEnSprites,
        PackKey::ToEnSprites,
        PackKey::RiEnSprites,
    ];
    K.map(|k| {
        let blob: &'static [u8] = pack::get(k);
        let s = pack::en_spr_sections(blob).expect("敌帧/位复合段布局非法（头 6×u32 + 七块）");
        let cut = |i: usize| -> &'static [u8] { &blob[s[i].0..s[i].0 + s[i].1] };
        EnemySprites {
            anim: cut(0),
            fptr: cut(1),
            pptr: cut(2),
            frames: cut(3),
            places: cut(4),
            foffs: cut(5),
            poffs: cut(6),
        }
    })
});

/// 按区取敌帧/位数据面（`area = InArea & $0F`；越界 fail-loud）。
pub fn enemy_sprites(area: u8) -> EnemySprites {
    let a = area_idx(area)
        .unwrap_or_else(|| panic!("敌帧/位数据面：非法区号 area={area:#04X}（应为 0..=4）"));
    EN_SPR[a as usize]
}

/// 动画表（`EnAnimTable`；`$FF`=动画结束）。
pub fn enemy_anim_table(area: u8) -> &'static [u8] {
    enemy_sprites(area).anim
}

/// 帧指针表（LE u16 项）。
pub fn enemy_frame_ptrs(area: u8) -> &'static [u8] {
    enemy_sprites(area).fptr
}

/// 位件指针表（LE u16 项）。
pub fn enemy_place_ptrs(area: u8) -> &'static [u8] {
    enemy_sprites(area).pptr
}

/// 帧数据体。
pub fn enemy_frames(area: u8) -> &'static [u8] {
    enemy_sprites(area).frames
}

/// 位件数据体。
pub fn enemy_places(area: u8) -> &'static [u8] {
    enemy_sprites(area).places
}

fn offs_at(t: &'static [u8], idx: usize) -> Option<(u32, u32)> {
    let e = t.get(idx * 8..idx * 8 + 8)?;
    Some((
        u32::from_le_bytes([e[0], e[1], e[2], e[3]]),
        u32::from_le_bytes([e[4], e[5], e[6], e[7]]),
    ))
}

/// 第 `idx` 帧项 `(偏移, 长)`——越界 ⇒ `None`。
pub fn enemy_frame_off(area: u8, idx: usize) -> Option<(u32, u32)> {
    offs_at(enemy_sprites(area).foffs, idx)
}

/// 第 `idx` 位件项 `(偏移, 长)`——越界 ⇒ `None`。
pub fn enemy_place_off(area: u8, idx: usize) -> Option<(u32, u32)> {
    offs_at(enemy_sprites(area).poffs, idx)
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
static KR_ROOMS: LazyLock<Vec<&'static [u8]>> = LazyLock::new(|| {
    slices(pack::get(PackKey::KrRoomStreams), pack::get(PackKey::KrRoomOffs), 37)
});
static KR_STRUCTS: LazyLock<Vec<&'static [u8]>> = LazyLock::new(|| {
    slices(pack::get(PackKey::KrStructures), pack::get(PackKey::KrStructOffs), 39)
});
static TO_ROOMS: LazyLock<Vec<&'static [u8]>> = LazyLock::new(|| {
    slices(pack::get(PackKey::ToRoomStreams), pack::get(PackKey::ToRoomOffs), 21)
});
static TO_STRUCTS: LazyLock<Vec<&'static [u8]>> = LazyLock::new(|| {
    slices(pack::get(PackKey::ToStructures), pack::get(PackKey::ToStructOffs), 32)
});
static RI_ROOMS: LazyLock<Vec<&'static [u8]>> = LazyLock::new(|| {
    slices(pack::get(PackKey::RiRoomStreams), pack::get(PackKey::RiRoomOffs), 42)
});
static RI_STRUCTS: LazyLock<Vec<&'static [u8]>> = LazyLock::new(|| {
    slices(pack::get(PackKey::RiStructures), pack::get(PackKey::RiStructOffs), 29)
});

/// 区序号（`InArea & 0x0F`：0=Br/1=No/2=Kr/3=To/4=Ri）——未知区 ⇒ `None`（fail-loud）。
fn area_idx(area: u8) -> Option<u8> {
    match area & 0x0F {
        0..=4 => Some(area & 0x0F),
        _ => None,
    }
}

/// 房流指针表（u16 LE × N；N 由区决定：Brinstar 47 / Norfair 46）。
pub fn room_ptrs(area: u8) -> Option<&'static [u8]> {
    const K: [PackKey; 5] = [PackKey::BrRoomPtrs, PackKey::NoRoomPtrs, PackKey::KrRoomPtrs,
                             PackKey::ToRoomPtrs, PackKey::RiRoomPtrs];
    Some(pack::get(K[area_idx(area)? as usize]))
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
    const K: [PackKey; 5] = [PackKey::BrStructPtrs, PackKey::NoStructPtrs, PackKey::KrStructPtrs,
                             PackKey::ToStructPtrs, PackKey::RiStructPtrs];
    Some(pack::get(K[area_idx(area)? as usize]))
}

/// 元tile 定义（4 B/组：Brinstar 260 B / Norfair 248 B）。
pub fn metatile_defs(area: u8) -> Option<&'static [u8]> {
    const K: [PackKey; 5] = [PackKey::BrMetatiles, PackKey::NoMetatiles, PackKey::KrMetatiles,
                             PackKey::ToMetatiles, PackKey::RiMetatiles];
    Some(pack::get(K[area_idx(area)? as usize]))
}

/// 房流（按房号下标；每段自界于 `$FF`）。
pub fn room_streams(area: u8) -> Option<&'static [&'static [u8]]> {
    match area_idx(area)? {
        0 => Some(&BR_ROOMS),
        1 => Some(&NO_ROOMS),
        2 => Some(&KR_ROOMS),
        3 => Some(&TO_ROOMS),
        _ => Some(&RI_ROOMS),
    }
}

/// 结构体（按结构号下标）。
pub fn structures(area: u8) -> Option<&'static [&'static [u8]]> {
    match area_idx(area)? {
        0 => Some(&BR_STRUCTS),
        1 => Some(&NO_STRUCTS),
        2 => Some(&KR_STRUCTS),
        3 => Some(&TO_STRUCTS),
        _ => Some(&RI_STRUCTS),
    }
}


// ---- 敌九表（五区各一套，地址布局同址、内容按区 bank）----------------------------
//
// asm 形：`LoadEnemy` 按 `EnemyType` 索引各表（`crates/game-core/src/frame/room.rs`），
// 表址 brinstar `prg1_brinstar.asm:195-421`（$962B…$977B）。**表长口径**：健康/强制速度/
// 初始延迟/朝向在 asm 相邻标签间距各 16 B，本仓历史烘焙 64 B（跨表连读）——搬家不改语义，
// 长度另案（`enemy-table-extent`）。
/// 敌表区索引（0=Br/1=No/2=Kr/3=To/4=Ri——`InArea & $0F`）。
fn enemy_table(area: u8, idx: usize) -> &'static [u8] {
    const KEYS: [[PackKey; 12]; 5] = [
        [PackKey::BrEnemyHealth, PackKey::BrEnemyRestingAnim, PackKey::BrEnemyActiveAnim,
         PackKey::BrEnemyActiveAnimInitOffset, PackKey::BrEnemyForceAttribs,
         PackKey::BrEnemyForceSpeedDelay, PackKey::BrEnemyDistanceThreshold,
         PackKey::BrEnemyInitDelay, PackKey::BrEnemyFacing, PackKey::BrEnMovPtrs,
         PackKey::BrEnMovStr, PackKey::BrEnemyDeathAnim],
        [PackKey::NoEnemyHealth, PackKey::NoEnemyRestingAnim, PackKey::NoEnemyActiveAnim,
         PackKey::NoEnemyActiveAnimInitOffset, PackKey::NoEnemyForceAttribs,
         PackKey::NoEnemyForceSpeedDelay, PackKey::NoEnemyDistanceThreshold,
         PackKey::NoEnemyInitDelay, PackKey::NoEnemyFacing, PackKey::NoEnMovPtrs,
         PackKey::NoEnMovStr, PackKey::NoEnemyDeathAnim],
        [PackKey::KrEnemyHealth, PackKey::KrEnemyRestingAnim, PackKey::KrEnemyActiveAnim,
         PackKey::KrEnemyActiveAnimInitOffset, PackKey::KrEnemyForceAttribs,
         PackKey::KrEnemyForceSpeedDelay, PackKey::KrEnemyDistanceThreshold,
         PackKey::KrEnemyInitDelay, PackKey::KrEnemyFacing, PackKey::KrEnMovPtrs,
         PackKey::KrEnMovStr, PackKey::KrEnemyDeathAnim],
        [PackKey::ToEnemyHealth, PackKey::ToEnemyRestingAnim, PackKey::ToEnemyActiveAnim,
         PackKey::ToEnemyActiveAnimInitOffset, PackKey::ToEnemyForceAttribs,
         PackKey::ToEnemyForceSpeedDelay, PackKey::ToEnemyDistanceThreshold,
         PackKey::ToEnemyInitDelay, PackKey::ToEnemyFacing, PackKey::ToEnMovPtrs,
         PackKey::ToEnMovStr, PackKey::ToEnemyDeathAnim],
        [PackKey::RiEnemyHealth, PackKey::RiEnemyRestingAnim, PackKey::RiEnemyActiveAnim,
         PackKey::RiEnemyActiveAnimInitOffset, PackKey::RiEnemyForceAttribs,
         PackKey::RiEnemyForceSpeedDelay, PackKey::RiEnemyDistanceThreshold,
         PackKey::RiEnemyInitDelay, PackKey::RiEnemyFacing, PackKey::RiEnMovPtrs,
         PackKey::RiEnMovStr, PackKey::RiEnemyDeathAnim],
    ];
    // **禁止 `% 5` 静默回绕**：`area` 是 `InArea & $0F`，原版只有 0..=4 有意义。
    // 越界（如 $15/$1F）在旧形里会静默取到别的区的表——那是最难查的一类错：
    // 数据合法、行为错、且不记账。这里改成 fail-loud（`area_idx` 是唯一判据）。
    let a = area_idx(area).unwrap_or_else(|| {
        panic!("敌表访问：非法区号 area={area:#04X}（InArea&$0F 应为 0..=4）")
    });
    pack::get(KEYS[a as usize][idx])
}

/// 五区敌表访问器（`area = InArea & $0F`；未就绪即 panic——调用点只在游戏内）。
pub fn enemy_health(area: u8) -> &'static [u8] { enemy_table(area, 0) }
pub fn enemy_resting_anim(area: u8) -> &'static [u8] { enemy_table(area, 1) }
pub fn enemy_active_anim(area: u8) -> &'static [u8] { enemy_table(area, 2) }
pub fn enemy_active_anim_init_offset(area: u8) -> &'static [u8] { enemy_table(area, 3) }
pub fn enemy_force_attribs(area: u8) -> &'static [u8] { enemy_table(area, 4) }
pub fn enemy_force_speed_delay(area: u8) -> &'static [u8] { enemy_table(area, 5) }
pub fn enemy_distance_threshold(area: u8) -> &'static [u8] { enemy_table(area, 6) }
pub fn enemy_init_delay(area: u8) -> &'static [u8] { enemy_table(area, 7) }
pub fn enemy_facing(area: u8) -> &'static [u8] { enemy_table(area, 8) }
pub fn enemy_movement_ptrs(area: u8) -> &'static [u8] { enemy_table(area, 9) }

/// `EnemyMovementChoices` 的**语义表长**（按区不同，asm 标签间距机械给定）：
/// Br 30 / No 22 / Kr 24 / To 4 / Ri 28（`$97D1` 起，表尾即移动指令串起点）。
pub const EN_MOV_CHOICES_LEN: [usize; 5] = [30, 22, 24, 4, 28];

/// 移动指令串区**基址**（按区不同）：`$97D1 + EN_MOV_CHOICES_LEN[area]`。
/// 即 Br `$97EF` / No `$97E7` / Kr `$97E9` / To `$97D5` / Ri `$97ED`——旧形一律按
/// `$97EF` 取窗与算偏移，No/Kr/To/Ri 的首 6–26 B 被切掉且指针减基址下溢 ⇒
/// 这四区的移动串全部落空（`enemy_get_delta_xy` 读到 `$FF` 回落 ⇒ 敌不移动）。
pub fn enemy_movement_string_base(area: u8) -> u16 {
    0x97D1 + EN_MOV_CHOICES_LEN[(area as usize).min(4)] as u16
}
pub fn enemy_movement_strings(area: u8) -> &'static [u8] { enemy_table(area, 10) }
/// `EnemyDeathAnimIndex ; 各 bank $960B`（32 B，`type*2+朝向` 索引）——**按区不同**。
pub fn enemy_death_anim(area: u8) -> &'static [u8] { enemy_table(area, 11) }

// ---- 敌人专属常量表（ROM 裸表；D 面纪律：Rust 不内嵌，只经本通道读）----------------
//
// 四张表原先以 `const [i8; N]` / `const [u8; N]` 抄在 `game-core/src/enemy/*.rs`。
// 表长由 asm 相邻标签间距机械给定（见 `extract.rs` 的 `ENEMY_ROM_TABLES` 注）。

/// `KraidProjectileOffsetX ; 04:9BD1` 10 B（i8；5 槽 × [朝右, 朝左]）。
pub fn kraid_projectile_offset_x() -> &'static [u8] { pack::get(PackKey::KrProjOffsetX) }

/// `MetroidLatchOffsetY ; 03:99D8` 12 B（i8）。
pub fn metroid_latch_offset_y() -> &'static [u8] { pack::get(PackKey::MetroidLatchOffsetY) }

/// `CannonInstrLists ; 03:9D99` 45 B（`$FF`=回表头，`$F0-$FE`=开火）。
pub fn cannon_instr_lists() -> &'static [u8] { pack::get(PackKey::CannonInstrLists) }

/// `RinkaSpawnPosTbl ; 03:A1DB` 12 B（高 nibble=Y、低 nibble=X）。
pub fn rinka_spawn_pos() -> &'static [u8] { pack::get(PackKey::RinkaSpawnPos) }

/// `CrawlerAnimIndexTable_{AREA}`（8 B，`orientation*2 + direction` 索引）——**按区不同**。
///
/// 表项是 `EnAnim_X - EnAnimTable_{AREA}`，而五区 anim 表基址/内容各不相同，故同一"墙上爬"
/// 语义在不同区落到不同帧号。**Tourian 无该表**（`prg3_tourian.asm` 的 AI 表无 Crawler
/// 类型）⇒ 返回 `None`，消费侧 fail-loud（不编造数据）。
pub fn crawler_anim_table(area: u8) -> Option<&'static [u8]> {
    let key = match area {
        0 => PackKey::BrCrawlerAnim,
        1 => PackKey::NoCrawlerAnim,
        2 => PackKey::KrCrawlerAnim,
        4 => PackKey::RiCrawlerAnim,
        _ => return None,
    };
    pack::try_get(key)
}


/// ## 按区表必须带区：`EnemyDeathAnimIndex` 的回归锚
#[cfg(test)]
mod tests_per_area_tables {
    use super::*;

    /// 敌动画索引窗 `$95DC-$95E5` 五区锚（ROM `bank*0x4000` 实读）。
    ///
    /// `killed` 五区同为 `0x03`；`explosion` Br `0x43` / Kr `0x43` / To `0x21` / No `0x77` / Ri `0x58`。
    /// 越读保真：`data0A ∈ 0..7` 时 `$95E0+y` 会读进 splatter/mellow/**AI 表头**
    /// （ROM bank2 实读 `$95DC..$95E7` = `03 77 53 57 55 59 5b 4f 32 bd 02 6b`）。
    #[test]
    fn falling_anim_byte_reads_past_the_two_entry_word_faithfully() {
        assert_eq!(en_projectile_falling_anim_byte(1, 0), 0x55);
        assert_eq!(en_projectile_falling_anim_byte(1, 1), 0x59);
        assert_eq!(en_projectile_falling_anim_byte(1, 2), 0x5B);
        assert_eq!(en_projectile_falling_anim_byte(1, 6), 0x02); // $95E6：`lda EnsExtra.0.type,x` 的 02
        assert_eq!(en_projectile_falling_anim_byte(1, 7), 0x6B); // $95E7
        // 溅落径：`y = data0A>>1 ∈ 0..3`（`$95E2+y`）
        assert_eq!(en_projectile_splatter_anim_byte(1, 0), 0x5B);
        assert_eq!(en_projectile_splatter_anim_byte(1, 1), 0x4F);
        assert_eq!(en_projectile_splatter_anim_byte(1, 3), 0xBD); // $95E5：`lda` 的 BD
    }

    /// **Norfair 的 `falling/splatter` 非零**（`0x5955/0x4F5B`）——这正是它必须按区的原因：
    /// 若有人换回单表，Norfair 的弹体动画会退化成 Brinstar 的 `0`。
    #[test]
    fn en_anim_idx_rom_anchor_five_areas() {
        init_from_env().ok();
        assert_eq!(en_projectile_killed_anim(0), 0x03);
        assert_eq!(explosion_anim(0), 0x43);
        assert_eq!(explosion_anim(1), 0x77);
        assert_eq!(explosion_anim(2), 0x43);
        assert_eq!(explosion_anim(3), 0x21);
        assert_eq!(explosion_anim(4), 0x58);
        assert_eq!(en_projectile_falling_anim(0), 0x0000);
        assert_eq!(en_projectile_falling_anim(1), 0x5955);
        assert_eq!(en_projectile_splatter_anim(1), 0x4F5B);
        assert_eq!(mellow_anim_index(0), 0x69);
        assert_eq!(mellow_anim_index(4), 0x25);
        assert_ne!(mellow_anim_index(0), mellow_anim_index(4));
    }

    /// 敌辅助表窗（`$96CB` 16 B + `$972B-$97D5` 170 B）五区锚——**形状 + 按区差异**。
    ///
    /// ROM 首字节（`bank*0x4000`）：
    /// `choice[0..4]`：Br `00 03 06 08` / No `12 14 00 00` / Kr `00 03 00 06` / To `00 02 00 00` / Ri `18 1a 00 03`
    /// `accelY[0..4]`：Br `7f 40 30 c0` / No `80 80 00 00` / Kr `7f 70 70 90` / To `18 30 00 c0` / Ri `80 80 00 00`
    /// `choices`：Br `01 01 02 01` / To `00 00 00 01` / Ri `01 04 05 01`
    #[test]
    fn en_aux_tables_rom_anchor_five_areas() {
        init_from_env().ok();
        let want_choice: [[u8; 4]; 5] = [
            [0x00, 0x03, 0x06, 0x08],
            [0x12, 0x14, 0x00, 0x00],
            [0x00, 0x03, 0x00, 0x06],
            [0x00, 0x02, 0x00, 0x00],
            [0x18, 0x1A, 0x00, 0x03],
        ];
        let want_accel_y: [[u8; 4]; 5] = [
            [0x7F, 0x40, 0x30, 0xC0],
            [0x80, 0x80, 0x00, 0x00],
            [0x7F, 0x70, 0x70, 0x90],
            [0x18, 0x30, 0x00, 0xC0],
            [0x80, 0x80, 0x00, 0x00],
        ];
        let want_choices: [[u8; 4]; 5] = [
            [0x01, 0x01, 0x02, 0x01],
            [0x00, 0x02, 0x00, 0x09],
            [0x01, 0x01, 0x02, 0x01],
            [0x00, 0x00, 0x00, 0x01],
            [0x01, 0x04, 0x05, 0x01],
        ];
        for area in 0..5u8 {
            let i = area as usize;
            assert_eq!(enemy_movement_choice_offset(area).len(), 16);
            assert_eq!(en_accel_y(area).len(), 20);
            assert_eq!(en_accel_x(area).len(), 20);
            assert_eq!(en_speed_y(area).len(), 20);
            assert_eq!(en_speed_x(area).len(), 20);
            assert_eq!(en_projectile_rising_anim(area).len(), 16);
            assert_eq!(en_projectile_pos_offset_x(area).len(), 8);
            assert_eq!(en_projectile_pos_offset_y(area).len(), 4);
            assert_eq!(en_projectile_movement_ptrs(area).len(), 8);
            assert_eq!(vram_string_ptrs(area).len(), 34);
            assert_eq!(&enemy_movement_choice_offset(area)[..4], &want_choice[i], "area{area} $96CB");
            assert_eq!(&en_accel_y(area)[..4], &want_accel_y[i], "area{area} $972B");
            assert_eq!(enemy_movement_choices(area).len(), 30);
            assert_eq!(&enemy_movement_choices(area)[..4], &want_choices[i], "area{area} $97D1");
        }
        // 按区不同（若有人把它换回单表，这里必红）
        assert_ne!(en_accel_y(0), en_accel_y(4));
        assert_ne!(enemy_movement_choice_offset(0), enemy_movement_choice_offset(1));
    }

    /// 区头窗 `$95CD-$95D9`（13 B）五区逐字节锚——**消费点 5 处的唯一源**。
    ///
    /// ROM 字节（`bank_source_offset` 同式，逻辑 bank N 落文件第 N+1 块）：
    /// ```text
    /// bank1 01 80 00 2b 2c 28 0b 1c 0a 1a 03 0e b0   (Brinstar)
    /// bank2 08 00 01 10 05 27 04 0f ff ff 16 0d 6e   (Norfair)
    /// bank4 10 00 02 ff ff ff ff ff ff ff 07 14 6e   (Kraid)
    /// bank3 40 00 03 ff ff ff ff ff ff ff 03 04 6e   (Tourian)
    /// bank5 80 00 02 ff ff ff ff ff ff ff 19 18 6e   (Ridley)
    /// ```
    #[test]
    fn area_header_rom_anchor_five_areas() {
        init_from_env().ok();
        // music_Brinstar/`Norfair`/`KraidArea`/`Tourian`/`RidleyArea` = 01/08/10/40/80
        // （constants.asm:166-173）。**旧 Rust 抄本 `AREA_MUSIC_FLAG` 把 Kraid 与 Tourian
        // 写反了**（记成 40/10）——本测按 ROM 字节钉正，并在文档登记该更正。
        let want_music = [0x01u8, 0x08, 0x10, 0x40, 0x80];
        let want_dmg = [0x0080u16, 0x0100, 0x0200, 0x0300, 0x0240];
        let want_rooms = [
            [0x2B, 0x2C, 0x28, 0x0B, 0x1C, 0x0A, 0x1A],
            [0x10, 0x05, 0x27, 0x04, 0x0F, 0xFF, 0xFF],
            [0xFF; 7],
            [0xFF; 7],
            [0xFF; 7],
        ];
        let want_map = [(0x03u8, 0x0Eu8), (0x16, 0x0D), (0x07, 0x14), (0x03, 0x04), (0x19, 0x18)];
        let want_sy = [0xB0u8, 0x6E, 0x6E, 0x6E, 0x6E];
        for area in 0..5u8 {
            let h = area_header(area);
            assert_eq!(h.music_flag, want_music[area as usize], "area{area} AreaMusicFlag $95CD");
            assert_eq!(h.enemy_damage, want_dmg[area as usize], "area{area} AreaEnemyDamage $95CE(word)");
            assert_eq!(h.item_rooms, want_rooms[area as usize], "area{area} AreaItemRoomNumbers $95D0");
            assert_eq!((h.map_x, h.map_y), want_map[area as usize], "area{area} AreaMapPosX/Y $95D7/$95D8");
            assert_eq!(h.samus_y, want_sy[area as usize], "area{area} AreaSamusY $95D9");
        }
    }

    /// 区头访问对非法区号 fail-loud（`InArea&$0F` 只 0..=4）。
    #[test]
    #[should_panic(expected = "非法区号")]
    fn area_header_rejects_area_out_of_range() {
        init_from_env().ok();
        let _ = area_header(0x05);
    }

    /// `EnemyDeathAnimIndex ; 各 bank $960B` **按区不同**（A 级：ROM 逐字节）。
    /// 旧形是单常量取 bank1 ⇒ 其余四区爆体帧错表。此测钉住五区首字节与长度。
    /// `CrawlerAnimIndexTable` 五区锚（ROM 逐字节）：**只有 Tourian 无此表**。
    ///
    /// bank1/bank4 同为 Zoomer/Zeela 值域但表址不同（`01:9AA0` / `04:99EF`），Norfair 是
    /// Nova、Ridley 是 Viola——旧形硬编码 bank1 ⇒ 非 Brinstar 的 Crawler 重取向取错帧。
    #[test]
    fn crawler_anim_table_differs_per_area() {
        init_from_env().ok();
        // 五区首项（`orientation=0,direction=0` = 地面正行）与长度：值来自 ROM 逐字节
        // 实读（`01:9AA0` / `02:99ED` / `04:99EF` / `05:99BE`），此处只钉首项与整表互异
        // ——完整 8 B 表在包里，逐字节由 `pack-tool verify` 对 ROM 校验。
        let br = crawler_anim_table(0).unwrap();
        let no = crawler_anim_table(1).unwrap();
        let kr = crawler_anim_table(2).unwrap();
        let ri = crawler_anim_table(4).unwrap();
        for t in [br, no, kr, ri] {
            assert_eq!(t.len(), 8, "CrawlerAnimIndexTable 8 B（orientation×2+direction）");
        }
        assert_eq!((br[0], no[0], kr[0], ri[0]), (0x35, 0x69, 0x35, 0x4A));
        assert_ne!(br, no, "Norfair 是 Nova 表（≠ Zoomer）");
        assert_ne!(no, ri, "Norfair 与 Ridley 的表必须不同");
        assert_eq!(br, kr, "bank1 与 bank4 的 Zoomer/Zeela 表同值（表址不同）");
        assert!(crawler_anim_table(3).is_none(), "Tourian 无 CrawlerAnimIndexTable（AI 表无该 type）");
    }

    #[test]
    fn enemy_death_anim_differs_per_area() {
        init_from_env().ok();
        let br = enemy_death_anim(0);
        let no = enemy_death_anim(1);
        let kr = enemy_death_anim(2);
        let to = enemy_death_anim(3);
        let ri = enemy_death_anim(4);
        for t in [br, no, kr, to, ri] {
            assert_eq!(t.len(), 32, "EnemyDeathAnimIndex 窗口 32 B（type×2+朝向）");
        }
        // bank1/2/3/4/5 的 $960B 首字节
        assert_eq!((br[0], no[0], kr[0], to[0], ri[0]), (0x27, 0x28, 0x27, 0x08, 0x23));
        // 差异不止首字节：bank1 vs bank3 整表不同
        assert_ne!(br, to, "Brinstar 与 Tourian 的爆体帧表必须不同");
        assert_ne!(no, ri, "Norfair 与 Ridley 的爆体帧表必须不同");
    }

    /// 越界区号必须 **fail-loud**，不能静默回绕到别区的表。
    ///
    /// 旧形 `KEYS[(area & 0x0F) % 5][idx]`：`area=$05` 会取到 Brinstar 的表、
    /// `$0F` 取到 Ridley 的表 —— 数据合法、行为错、且不记账。原版语义里 `InArea & $0F`
    /// 只有 0..=4 有意义，其余取值属于移植侧的错误而不是 ROM 的取值。
    #[test]
    #[should_panic(expected = "非法区号")]
    fn enemy_table_rejects_area_out_of_range() {
        init_from_env().ok();
        let _ = enemy_health(0x05);
    }
}

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
        if !ensure_ready() {
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
        if !ensure_ready() {
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
        if !ensure_ready() {
            eprintln!("SKIP（显式）：资产包未加载——先跑 pack-tool extract");
            return;
        }
        let info = gfx_info();
        let sum: usize = (0..29)
            .map(|i| usize::from(u16::from_le_bytes([info[i * 7 + 5], info[i * 7 + 6]])))
            .sum();
        assert_eq!(sum, gfx_data().len(), "描述子尺寸和与数据体长度不符");
    }

    /// 敌帧/位数据面：**五区各自**的长度必须与 ROM 实读一致
    /// （`docs/findings/per-bank-assets.md` §1；旧形是 bank1 单表）。
    #[test]
    fn enemy_segments_shape_matches_table_extent() {
        if !ensure_ready() {
            eprintln!("SKIP（显式）：资产包未加载——先跑 pack-tool extract");
            return;
        }
        // anim 长 / 帧指针项数 / 位件项数，**按区号 0..4 = Br/No/Kr/To/Ri 排**
        //（`area = InArea & $0F` 是区号，不是 bank 序号）。
        //
        // 取证：docs/findings/per-bank-assets.md §1 按 bank 1..5 列为
        // Br(118/151/16)、No(138/138/14)、**To(38/138/16)**、**Kr(113/151/16)**、Ri(107/138/15)；
        // 区号序与之不同的只有中间两行——Tourian 是区 3、Kraid 是区 2。
        // 本仓库 `assets/metroid.pack` 的 anim 段逐字节可在 ROM 中定位到
        // file bank2 $9D6A(118B) / bank3 $9BDA(138B) / bank5 $9C86(113B) / bank4 $A406(38B) / bank6 $9B85(107B)，
        // 即 KrEnSprites=bank5(Kraid,113)、ToEnSprites=bank4(Tourian,38)。
        // 旧表把这两区写反（按 bank 行序当成区号序），单线程真跑时 `area 2 anim 长 113 != 38` 即此。
        let anim = [118usize, 138, 113, 38, 107];
        let fptr = [151usize, 138, 151, 138, 138];
        let pptr = [16usize, 14, 16, 16, 15];
        for a in 0..5u8 {
            let s = enemy_sprites(a);
            assert_eq!(s.anim.len(), anim[a as usize], "area {a} anim 长");
            assert_eq!(s.fptr.len(), fptr[a as usize] * 2, "area {a} 帧指针项数");
            assert_eq!(s.pptr.len(), pptr[a as usize] * 2, "area {a} 位件指针项数");
            assert!(!s.frames.is_empty() && !s.places.is_empty(), "area {a} 数据体非空");
            assert_eq!(s.foffs.len(), fptr[a as usize] * 8, "area {a} 帧索引项数");
            assert_eq!(s.poffs.len(), pptr[a as usize] * 8, "area {a} 位件索引项数");
        }
        // 与 bank1 相同的 anim 字节数必须 < 118（否则说明五区其实同表 ⇒ 按区没生效）
        let b1 = enemy_sprites(0).anim;
        for a in 1..5u8 {
            let same = b1.iter().zip(enemy_sprites(a).anim).filter(|(x, y)| x == y).count();
            assert!(same < 118, "area {a} 的 anim 表不应与 Brinstar 相同（实测应 <118）");
        }
    }

    /// 首项尺寸必须从描述子 `+5..+7` 读（守 `GFXInfo` 的 7 B 列义——
    /// 曾经误用 `rec[0]` 当尺寸，本断言会红）。
    #[test]
    fn first_entry_size_reads_descriptor_size_column() {
        if !ensure_ready() {
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
        if !ensure_ready() {
            eprintln!("SKIP（显式）：资产包未加载——先跑 pack-tool extract");
            return;
        }
        for a in 0..5u8 {
            let s = enemy_sprites(a);
            assert_eq!(s.fptr.len() % 2, 0, "area {a} 帧指针表应为 u16 项");
            assert_eq!(s.pptr.len() % 2, 0, "area {a} 位件指针表应为 u16 项");
            for (i, w) in s.fptr.chunks_exact(2).enumerate() {
                let p = u16::from_le_bytes([w[0], w[1]]);
                if p == 0 {
                    continue; // 原版表里确有零项（提取时留 (0,0)）
                }
                assert!(p >= 0x8000, "area {a} 帧指针 {i} 非法: {p:04X}");
            }
            for (i, w) in s.pptr.chunks_exact(2).enumerate() {
                let p = u16::from_le_bytes([w[0], w[1]]);
                assert!(p >= 0x8000, "area {a} 位件指针 {i} 非法: {p:04X}");
            }
        }
    }

    /// 敌面项表自洽 + 帧流末字节恒 `$FF`（防「空对空」假绿）——五区逐区。
    #[test]
    fn enemy_offsets_fit_and_streams_end_with_ff() {
        if !ensure_ready() {
            eprintln!("SKIP（显式）：资产包未加载——先跑 pack-tool extract");
            return;
        }
        for a in 0..5u8 {
            let s = enemy_sprites(a);
            let mut checked = 0;
            let mut i = 0;
            while let Some((o, l)) = enemy_frame_off(a, i) {
                if l > 0 {
                    assert!((o + l) as usize <= s.frames.len(), "area {a} 帧项 {i} 越出数据体");
                    assert_eq!(
                        s.frames[(o + l) as usize - 1],
                        0xFF,
                        "area {a} 帧项 {i} 末字节应为 $FF"
                    );
                    checked += 1;
                }
                i += 1;
            }
            assert!(checked > 0, "area {a} 至少要校验到一项");
            assert!(enemy_frame_off(a, i).is_none(), "area {a} 越界索引应 None");
        }
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

    /// `MetroidData ; 07:C5A8`：14 B 全值锚（`InitBank3` 拷到 $77F0-$77FD）。
    ///
    /// asm 逐字节（prg7_engine.asm:1466-1471）：`F8 08`（MetroidRepelSpeed）/
    /// `30 D0 60 A0`（MetroidAccel）/`02 04`（MetroidMaxSpeed）/`00 ×6`（latch 区初值）。
    #[test]
    fn tourian_metroid_data_matches_asm_bytes() {
        let md = tourian_metroid_data();
        assert_eq!(md.len(), 14, "MetroidData 长度");
        assert_eq!(
            md,
            &[0xF8, 0x08, 0x30, 0xD0, 0x60, 0xA0, 0x02, 0x04, 0, 0, 0, 0, 0, 0],
            "MetroidData 全 14 字节"
        );
    }

    /// `AreaPaletteToggle ; 01/02/03/04/05:95DA`（ROM 逐字节 `01 00`×2 / `06 00`×3）——
    /// 区序（0=Br/1=No/2=Kr/3=To/4=Ri）⇒ `[01,01,06,06,06]`。
    /// **回归锚**：它曾由 `crates/game-core/src/frame/mainloop.rs` 误读 WRAM `$6885`
    /// （道具扫描项区）⇒ 恒 0 ⇒ 电梯换区 `PaletteToggle^=7` 后 pending 差 1、载错调色板。
    /// 区序值表与 asm 逐 bank `.byte` 一致（`01 00`×2 / `06 00`×3）。
    ///
    /// **注意**：bank3(Tourian) 与 bank4(Kraid) 的值同为 `$06` ⇒ 二者**互换在值上不可观测**，
    /// 单靠本测钉不住表驱动映射；映射由 `extract.rs` 侧 `AREA_BANKS` 的测试钉
    /// （`extract::tests::area_banks_match_bank_table_rom`，r9 复核要求）。
    #[test]
    fn area_palette_toggle_matches_rom_anchor() {
        let got: Vec<u8> = (0u8..5).map(|a| area_pal_toggle(a).expect("区序 0..4")).collect();
        assert_eq!(got, vec![0x01, 0x01, 0x06, 0x06, 0x06], "AreaPaletteToggle 五区");
        assert_eq!(area_pal_toggle(5), None, "区序 5 表外");
    }

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

    /// 星闪调色板 8×10B（`00:8B0F`）：块首 + 指针表对齐（`$8B0F + idx*10`）。
    #[test]
    fn intro_star_palettes_match_rom_anchors() {
        let pal = intro_star_palettes();
        assert_eq!(pal.len(), 8 * 10, "IntroStarPalette0..7");
        assert_eq!(&pal[0..10], &[0x03, 0x0F, 0x02, 0x13, 0x00, 0x03, 0x00, 0x34, 0x0F, 0x00], "Palette0");
        assert_eq!(&pal[40..50], &[0x03, 0x17, 0x01, 0x14, 0x00, 0x03, 0x10, 0x0F, 0x28, 0x00], "Palette4");
        assert_eq!(&pal[70..80], &[0x03, 0x0F, 0x12, 0x14, 0x00, 0x03, 0x10, 0x24, 0x0F, 0x00], "Palette7");
        // 指针表 `IntroStarPalettePtrTable 00:8AFF` 与块内偏移互核（8 项等距 10 B）。
        for i in 1..8 {
            assert_ne!(&pal[i * 10..i * 10 + 10], &pal[0..10], "Palette{i} 不重复（asm 注释：non-repeating）");
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
        assert_eq!(sfx_tables().len(), 0x7c, "$B289-$B20D（GamePaused 4B + 记录窗 0x78）");
        assert_eq!(sfx_record4(0xB20D).unwrap(), &[0x80, 0x7F, 0x80, 0x48], "SFXData@GamePaused（ROM 实读）");
        assert_eq!(sfx_record4(0xB211).unwrap(), &[0x35, 0x7F, 0x00, 0xB0], "W3[0] ScrewAttack");
        assert_eq!(sfx_record4(0xB225).unwrap(), &[0xC1, 0x89, 0x02, 0x0F], "SamusHit");
        assert_eq!(sfx_record4(0xB279).unwrap(), &[0x09, 0x7F, 0x30, 0x48], "Beep");
        assert_eq!(sfx_record4(0xB285).unwrap(), &[0x7F, 0x7F, 0x30, 0xB2], "W4[6]");
        assert!(sfx_record4(0xB20C).is_none(), "窗外低址（$B200 SFXData 头区）");
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

    /// 敌移动面三件（`areas_common.asm:555 LoadEnemyMovementPtr`）：
    /// 指针表 **36 项 = 72 B**（18 索引 × R/L×2）——2026-11-20 更正：旧注释「真表 36 B」
    /// 把「36 项」写成了字节数，实际只提取了一半，索引 9..17 全部缺指针
    /// （见 `docs/findings/enemy-movement-ptr-truncation.md`）。旧烘焙 138 项仍是过度读。
    /// 串区 `$97EF..$99EF`；`L977B` 复用已迁的 `BrEnemyFacing`（同 64 B）。
    #[test]
    fn enemy_movement_segments_match_asm_anchors() {
        let ptrs = enemy_movement_ptrs(0);
        assert_eq!(ptrs.len(), 72, "EnemyMovementPtrs 18×R/L×u16 = 36 项");
        // 18 索引 × 2 朝向，每一项都必须是串区内的合法指针（无 0、无越界）。
        assert_eq!(ptrs.len() % 4, 0, "每索引 2 个 u16（R/L）");
        let first = u16::from_le_bytes([ptrs[0], ptrs[1]]);
        assert_eq!(first, 0x97EF, "首个指针 = 串区起点 $97EF");
        assert!(ptrs.chunks(2).all(|w| {
            let p = u16::from_le_bytes([w[0], w[1]]) as usize;
            (0x97EF..0x97EF + 512).contains(&p) // 串区窗 512 B（$97EF..$99EF）
        }), "全部指针落在串区内");
        assert_eq!(enemy_movement_strings(0).len(), 512);
        // L977B 与朝向段同源（迁移只改来源不改字节）。
        assert_eq!(enemy_facing(0).len(), 64);
        assert_eq!(enemy_anim_table(0).len(), 118);
    }

    /// 标题屏 17 表锚（`prg0_title.asm` 标签 ↔ 段长；`BOTTOM_SPARKLE_DATA` 真长
    /// 98 B 是本次更正的烘焙漂移——旧 54 B 在表中间插了假终止符）。
    #[test]
    fn title_tables_match_asm_anchors() {
        let exp: [(&str, usize); 17] = [
            ("title.fade", 12), ("title.pal_select", 13), ("title.start_cont", 2),
            ("title.end_star", 156), ("title.pw_err", 15), ("title.pw_rows", 10),
            ("title.pw_grid", 65), ("title.cursor_x", 12), ("title.charsel_y", 5),
            ("title.charsel_x", 13), ("title.sparkle_top", 54), ("title.sparkle_bottom", 98),
            ("title.sparkle_init", 11), ("title.flash_pal", 10), ("title.init_cross", 64),
            ("title.cross_len", 5), ("title.cross_data", 68),
        ];
        let got: [(&'static str, usize); 17] = [
            (PackKey::Fade.name(), title_fade().len()),
            (PackKey::PalSelect.name(), title_pal_select().len()),
            (PackKey::StartCont.name(), title_start_cont().len()),
            (PackKey::EndStar.name(), title_end_star().len()),
            (PackKey::PwErr.name(), title_pw_err().len()),
            (PackKey::PwRows.name(), title_pw_rows().len()),
            (PackKey::PwGrid.name(), title_pw_grid().len()),
            (PackKey::CursorX.name(), title_cursor_x().len()),
            (PackKey::CharselY.name(), title_charsel_y().len()),
            (PackKey::CharselX.name(), title_charsel_x().len()),
            (PackKey::SparkleTop.name(), title_sparkle_top().len()),
            (PackKey::SparkleBottom.name(), title_sparkle_bottom().len()),
            (PackKey::SparkleInit.name(), title_sparkle_init().len()),
            (PackKey::ScreenFlashPal.name(), title_flash_pal().len()),
            (PackKey::InitCross.name(), title_init_cross().len()),
            (PackKey::CrossLen.name(), title_cross_len().len()),
            (PackKey::CrossData.name(), title_cross_data().len()),
        ];
        assert_eq!(got, exp);
        // 行指针表（`PasswordRowTbl ; 00:9998` → 五个 u16 LE，行距 13）。
        let rows = title_pw_rows();
        let addrs: Vec<u16> = (0..5).map(|r| u16::from_le_bytes([rows[r * 2], rows[r * 2 + 1]])).collect();
        assert_eq!(addrs, vec![0x99A2, 0x99AF, 0x99BC, 0x99C9, 0x99D6]);
        assert!(addrs.windows(2).all(|w| w[1] - w[0] == 13));
        // Bottom 表末尾必须是 `SignMagSpeed $00` 终止项（98 B 的第 97 字节为 0）。
        assert_eq!(title_sparkle_bottom()[96], 0x00);
        assert_eq!(title_sparkle_top()[0], 0x01);
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

/// `MetroidData ; 07:C5A8`（14 B）——Tourian 初始化参数。
///
/// `InitBank3 ; 07:C590` 的形状：
/// `ldy #$0D / Lx: lda MetroidData,y / sta MetroidRepelSpeed,y / dey / bpl`
/// ⇒ **倒序逐字节**拷到 `$77F0-$77FD`（MetroidRepelSpeed $77F0 / MetroidAccel $77F2 /
/// MetroidMaxSpeed $77F6 / MetroidLatch0400-0450 $77F8）。只有 latch 区后续会被改写。
pub fn tourian_metroid_data() -> &'static [u8] {
    pack::get(PackKey::TourianMetroidData)
}

/// `AreaPaletteToggle`（5 区，area 序 Br/No/Kr/To/Ri）——`MoreInit 07:C81D` 写
/// `PaletteToggle($0076)`；电梯换区再做 `eor 7`。ROM 值：`[01,01,06,06,06]`。
pub fn area_pal_toggle(area: u8) -> Option<u8> {
    pack::get(PackKey::AreaPalToggle)
        .get(area_idx(area)? as usize)
        .copied()
}

/// 区头参数窗（ROM `$95CD-$95D9`，13 B）——**唯一访问面**。
///
/// 这些值原先在 Rust 侧被抄成三张硬编码表（`AREA_MUSIC_FLAG`/`AREA_MAP_POS`/
/// `AREA_SAMUS_Y`），等于同一份 ROM 数据有两个真相源。现在一律走本函数，
/// 五个消费点（`StartMusic`/`SamusHurt`/`GetRoomNum`/`MoreInit`/`SamusInit`）同源。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AreaHeader {
    /// `AreaMusicFlag ; $95CD`——区域音乐 init flag（`music_{AREA}`）。
    pub music_flag: u8,
    /// `AreaEnemyDamage ; $95CE`（**word**）——区域敌基础伤害，`SamusHurt` 写 `HealthChange`。
    pub enemy_damage: u16,
    /// `AreaItemRoomNumbers ; $95D0`（7 B）——物品房音乐触发房号（`$FF`=无）。
    pub item_rooms: [u8; 7],
    /// `AreaMapPosX ; $95D7`——世界地图上萨姆斯起点 X。
    pub map_x: u8,
    /// `AreaMapPosY ; $95D8`——世界地图上萨姆斯起点 Y。
    pub map_y: u8,
    /// `AreaSamusY ; $95D9`——进入区域时萨姆斯纵向屏幕位置（写 `Samus.y`）。
    pub samus_y: u8,
}

fn area_header_key(area: u8) -> Option<PackKey> {
    match area_idx(area)? {
        0 => Some(PackKey::AreaBrHeader),
        1 => Some(PackKey::AreaNoHeader),
        2 => Some(PackKey::AreaKrHeader),
        3 => Some(PackKey::AreaToHeader),
        4 => Some(PackKey::AreaRiHeader),
        _ => None,
    }
}

/// 读区头参数（ROM `$95CD-$95D9`）。`area` 为 `InArea & $0F`；非法区号 panic（fail-loud）。
pub fn area_header(area: u8) -> AreaHeader {
    let key = area_header_key(area)
        .unwrap_or_else(|| panic!("区头访问：非法区号 area={area:#04X}（InArea&$0F 应为 0..=4）"));
    let b = pack::get(key);
    assert_eq!(b.len(), 13, "区头段长应为 13 B（$95CD-$95D9）");
    AreaHeader {
        music_flag: b[0],
        enemy_damage: u16::from_le_bytes([b[1], b[2]]),
        item_rooms: [b[3], b[4], b[5], b[6], b[7], b[8], b[9]],
        map_x: b[10],
        map_y: b[11],
        samus_y: b[12],
    }
}

/// 敌辅助表窗（ROM `$96CB` 16 B + `$972B-$97D0` 166 B，拼接 182 B）——按区唯一源。
///
/// 组成（偏移以拼接后为基准）：
/// `0..16` `EnemyMovementChoiceOffset` / `16..36` `EnAccelYTable` / `36..56` `EnAccelXTable` /
/// `56..76` `EnSpeedYTable` / `76..96` `EnSpeedXTable` / `96..112` `L977B`（另有 `enemy_facing`）/
/// `112..128` `EnProjectileRisingAnimIndexTable` / `128..136` `EnProjectilePosOffsetX` /
/// `136..140` `EnProjectilePosOffsetY` / `140..148` `EnProjectileMovementPtrTable` /
/// `148..182` `VRAMStringPtrTable`。（`EnemyMovementChoices` 另有独立段，见 `enemy_movement_choices`。）
/// 两块之间的 `$96DB-$972A`（movPtrs/movStr）已有独立段，故不重复。
pub const EN_AUX_CHOICE_LEN: usize = 16;

fn en_aux_key(area: u8) -> Option<PackKey> {
    match area_idx(area)? {
        0 => Some(PackKey::AreaBrEnAux),
        1 => Some(PackKey::AreaNoEnAux),
        2 => Some(PackKey::AreaKrEnAux),
        3 => Some(PackKey::AreaToEnAux),
        4 => Some(PackKey::AreaRiEnAux),
        _ => None,
    }
}

fn en_aux(area: u8) -> &'static [u8] {
    let key = en_aux_key(area)
        .unwrap_or_else(|| panic!("敌辅助表：非法区号 area={area:#04X}（InArea&$0F 应为 0..=4）"));
    let b = pack::get(key);
    assert_eq!(b.len(), 182, "敌辅助表段长应为 182 B");
    b
}

fn en_aux_slice(area: u8, off: usize, len: usize) -> &'static [u8] {
    &en_aux(area)[off..off + len]
}

/// `EnemyMovementChoiceOffset ; $96CB`（16 B）——`EnData08` 索引到 `EnemyMovementChoices` 的偏移。
pub fn enemy_movement_choice_offset(area: u8) -> &'static [u8] {
    en_aux_slice(area, 0, EN_AUX_CHOICE_LEN)
}
/// `EnAccelYTable ; $972B`（20 B）。
pub fn en_accel_y(area: u8) -> &'static [u8] { en_aux_slice(area, 16, 20) }
/// `EnAccelXTable ; $973F`（20 B）。
pub fn en_accel_x(area: u8) -> &'static [u8] { en_aux_slice(area, 36, 20) }
/// `EnSpeedYTable ; $9753`（20 B）。
pub fn en_speed_y(area: u8) -> &'static [u8] { en_aux_slice(area, 56, 20) }
/// `EnSpeedXTable ; $9767`（20 B）。
pub fn en_speed_x(area: u8) -> &'static [u8] { en_aux_slice(area, 76, 20) }
/// `EnProjectileRisingAnimIndexTable ; $978B`（16 B）。
pub fn en_projectile_rising_anim(area: u8) -> &'static [u8] { en_aux_slice(area, 112, 16) }
/// `EnProjectilePosOffsetX ; $979B`（8 B）。
pub fn en_projectile_pos_offset_x(area: u8) -> &'static [u8] { en_aux_slice(area, 128, 8) }
/// `EnProjectilePosOffsetY ; $97A3`（4 B）。
pub fn en_projectile_pos_offset_y(area: u8) -> &'static [u8] { en_aux_slice(area, 136, 4) }
/// `EnProjectileMovementPtrTable ; $97A7`（8 B，4 × u16）。
pub fn en_projectile_movement_ptrs(area: u8) -> &'static [u8] { en_aux_slice(area, 140, 8) }
/// `VRAMStringPtrTable ; $97AF`（34 B，`PtrTableEntryArea` 记录）。
pub fn vram_string_ptrs(area: u8) -> &'static [u8] { en_aux_slice(area, 148, 34) }

/// 瓦片爆用的第 `id` 条 VRAM 字符串（`VRAMStringPtrTable` 下标，0..16）。
///
/// 段布局：`17 x u16`（相对串体起点的偏移）+ 串体；串长由 `(id+1)` 的偏移差给出
/// （最后一串到段尾）。提取侧的连续性自检保证了「偏移差 == 上一串尺寸」。
///
/// 消费点 `GetVRAMStringPtr ; 07:FECC`（`lda VRAMStringPtrTable,y`）。
/// 结局背景名表（`VRAMStruct_EndBackground ; 00:A052`，360 B，`hi==0` 自带终止）。
pub fn end_background() -> &'static [u8] {
    crate::pack::get(crate::pack::PackKey::EndBackground)
}

/// 结局调色板第 `n` 张（`EndGamePalette00..0A ; 00:9F9B`，长度不齐 ⇒ 段内 11×u16 偏移表）。
/// `PaletteDataPending-1` 即 n（`EndGamePaletteWrite 00:9F54` 的取法）。
pub fn end_palette(n: u8) -> Option<&'static [u8]> {
    let seg = crate::pack::get(crate::pack::PackKey::EndPalettes);
    let cnt = 11usize;
    if n as usize >= cnt || seg.len() < cnt * 2 {
        return None;
    }
    let off = |i: usize| u16::from_le_bytes([seg[i * 2], seg[i * 2 + 1]]) as usize;
    let body = cnt * 2;
    let start = body + off(n as usize);
    let end = if (n as usize) + 1 < cnt { body + off(n as usize + 1) } else { seg.len() };
    if end < start || end > seg.len() {
        return None;
    }
    Some(&seg[start..end])
}

/// 结局职员表页（`CreditsPtrTable ; 00:A291`，6 页；页 2 只有终止符故长度为 1）。
pub fn end_credits(page: u8) -> Option<&'static [u8]> {
    let seg = crate::pack::get(crate::pack::PackKey::EndCredits);
    let n = 6usize;
    if page as usize >= n || seg.len() < n * 2 {
        return None;
    }
    let off = |i: usize| u16::from_le_bytes([seg[i * 2], seg[i * 2 + 1]]) as usize;
    let body = n * 2;
    let start = body + off(page as usize);
    let end = if (page as usize) + 1 < n { body + off(page as usize + 1) } else { seg.len() };
    if end < start || end > seg.len() {
        return None;
    }
    Some(&seg[start..end])
}

/// 结局 Samus 第 `idx` 帧精灵数据（`EndSamusAddrTbl ; 00:9D5A` 六指针 → 每帧 54 B）。
/// 段布局 = `6 x u16`（相对帧体起点偏移）+ 帧体，故 `idx` 的切片由相邻偏移差给出。
/// 消费点 `LoadEndSamusSprites ; 00:9C9A`（按 `SpritePointerIndex` 取帧）。
pub fn end_samus(idx: u8) -> Option<&'static [u8]> {
    let seg = crate::pack::get(crate::pack::PackKey::EndSamus);
    const N: usize = 6;
    if idx as usize >= N || seg.len() < N * 2 {
        return None;
    }
    let off = |i: usize| u16::from_le_bytes([seg[i * 2], seg[i * 2 + 1]]) as usize;
    let body = N * 2;
    let start = body + off(idx as usize);
    let end = if (idx as usize) + 1 < N { body + off(idx as usize + 1) } else { seg.len() };
    if end < start || end > seg.len() {
        return None;
    }
    Some(&seg[start..end])
}

/// 结局挥手手型四表（`WavePointerTable ; 00:9D12` 四指针 → 表体连续，段共 64 B）。
/// 消费点 `LoadWaveSprites ; 00:9C7F`：`ldx WaveSpritePointer` 取表，逐字节拷
/// `WaveSpriteCounter` 个字节到 `SpriteRAM.8`（`$0220`）。**四表紧密相邻**，故返回整段
/// 让调用方按 `table*8` 定位（每表 4 项 × 2 B），避免在这里二次切分引入口径漂移。
pub fn end_wave() -> &'static [u8] {
    crate::pack::get(crate::pack::PackKey::EndWave)
}

pub fn vram_string(area: u8, id: u8) -> Option<&'static [u8]> {
    const K: [PackKey; 5] = [
        PackKey::AreaBrVramStrings,
        PackKey::AreaNoVramStrings,
        PackKey::AreaToVramStrings,
        PackKey::AreaKrVramStrings,
        PackKey::AreaRiVramStrings,
    ];
    const COUNT: usize = 17;
    let seg = pack::get(K[area_idx(area)? as usize]);
    let body = seg.get(COUNT * 2..)?;
    let rel = |i: usize| -> Option<usize> {
        let b = seg.get(i * 2..i * 2 + 2)?;
        Some(u16::from_le_bytes([b[0], b[1]]) as usize)
    };
    let id = id as usize;
    if id >= COUNT {
        return None;
    }
    let start = rel(id)?;
    let end = if id + 1 < COUNT { rel(id + 1)? } else { body.len() };
    body.get(start..end)
}
/// `EnAccelY/X` + `EnSpeedY/X` **连续 80 B 窗**（各 20 B）——asm 用同一 `y` 分别查四表，
/// `y` 越界时会读进下一张表，故必须按同一窗口索引而不是切成四个独立切片。
pub fn en_accel_speed_window(area: u8) -> &'static [u8] { en_aux_slice(area, 16, 80) }

/// `EnemyMovementChoices ; $97D1`（30 B）——每种敌型的「移动索引候选集」字符串表。
///
/// 记录格式：`[mask][value…]`，`mask = 可能值数-1`；`EnemyMovementChoiceOffset[type]`
/// 给出记录起点偏移。五区**表长不同**（Br 30 / No 22 / Kr 24 / To 4 / Ri 28），
/// 这里统一按 30 B 窗提取（`$97D1-$97EE`）——Tourian 的 `Rinka` 偏移恰落在表尾，
/// 原版会继续读后续移动串字节，故窗口保留到 `$97EE` 以保持「表外读」也在 ROM 真实数据上。
pub fn enemy_movement_choices(area: u8) -> &'static [u8] {
    let key = match area_idx(area) {
        Some(0) => PackKey::AreaBrEnMovChoices,
        Some(1) => PackKey::AreaNoEnMovChoices,
        Some(2) => PackKey::AreaKrEnMovChoices,
        Some(3) => PackKey::AreaToEnMovChoices,
        Some(4) => PackKey::AreaRiEnMovChoices,
        _ => panic!("移动候选表：非法区号 area={area:#04X}（InArea&$0F 应为 0..=4）"),
    };
    let b = pack::get(key);
    assert_eq!(b.len(), 30, "移动候选表段长应为 30 B");
    b
}

/// `SqueeptSpeedYTable ; 02:9AD2`（4 B：`F6 F8 F6 FA`）——Squeept 跳起初速按
/// movementIndex 查。asm 侧仅 Norfair bank 有（`squeept.asm` 只被 prg2 include）。
pub fn squeept_speed_y(area: u8) -> &'static [u8] {
    if area_idx(area) != Some(1) {
        panic!("Squeept 速度表只存在于 Norfair（bank02:$9AD2），area={area:#04X} 无此段");
    }
    let b = pack::get(PackKey::AreaNoSqueeptSpeedY);
    assert_eq!(b.len(), 4, "Squeept 速度表段长应为 4 B");
    b
}

// ---- 敌弹移动串窗（**按区**）——`EnProjectileMovementPtrTable ; $97A7` 的字指针目标 ----
//
// asm `EnProjectileMovement0..3_{AREA}`（prg1_brinstar.asm:117-125 等 `{AREA}` 宏五区实例化）。
// 记录对：`[时长][符号幅值速度字节]`（SignMagSpeed 宏：高半=dy 符号<<3|幅值，低半=dx 同构），
// `$FF`=串尾（UpdateEnProjectile_Active@processMovementInstruction 07:F9A4-F9C0 读）。
// 指针目标可能落在窗内任意偏移（Tourian/Ridley 的 M0 与 M1 同址别名），
// 故窗界取「四指针目标的并集」，寻址=`指针字 - 窗基址`。

/// 各区移动串窗的 ROM 基址（`en_projectile_move_str(area)` 返回窗的起点地址）。
pub fn en_projectile_move_str_base(area: u8) -> u16 {
    match area_idx(area) {
        Some(0) => 0x9965, // bank01 EnProjectileMovement0
        Some(1) => 0x97F7, // bank02
        Some(2) => 0x98C9, // bank04（Kraid 排在区序 2——bank04）
        Some(3) => 0x97D5, // bank03（Tourian 区序 3——bank03）
        Some(4) => 0x97FD, // bank05
        _ => panic!("敌弹移动串：非法区号 area={area:#04X}（InArea&$0F 应为 0..=4）"),
    }
}

/// 敌弹移动串窗（长度见 pack.rs `Area{Br,No,To,Kr,Ri}EnProjMoveStr` 注释）。
pub fn en_projectile_move_str(area: u8) -> &'static [u8] {
    let key = match area_idx(area) {
        Some(0) => PackKey::AreaBrEnProjMoveStr,
        Some(1) => PackKey::AreaNoEnProjMoveStr,
        Some(2) => PackKey::AreaKrEnProjMoveStr,
        Some(3) => PackKey::AreaToEnProjMoveStr,
        Some(4) => PackKey::AreaRiEnProjMoveStr,
        _ => panic!("敌弹移动串：非法区号 area={area:#04X}（InArea&$0F 应为 0..=4）"),
    };
    pack::get(key)
}

/// `AreaEnProjectileFallingAnimIndex ; $95E0`（word）按 `data0A` **字节**直索引
/// （prg7_engine.asm:11986-11989 `ldy Ens.0.data0A,x / lda AreaEnProjectileFallingAnimIndex,y`）——
/// `data0A∈{0,1}` 落 word 低/高字节；`data0A∈{2,3}` 原版会继续读进
/// `AreaEnProjectileSplatterAnimIndex`（$95E2 word）的低/高字节——窗口内忠实保留该读法。
/// `data0A ∈ 0..7`（`07:F8FB` 的 `& $07`）⇒ 偏移 4..11，全部落在 12 B 窗内；
/// `y ≥ 8` 才会越过 `$95E7`，属调用方错误 ⇒ fail-loud。
pub fn en_projectile_falling_anim_byte(area: u8, y: u8) -> u8 {
    assert!(y < 8, "敌弹下落动画索引越窗（data0A={y}≥8，超出 $95DC-$95E7 窗）");
    en_anim_idx(area)[4 + y as usize]
}

/// `AreaEnProjectileSplatterAnimIndex ; $95E2` 的**字节**直索引（`y = data0A>>1 ∈ 0..3`，
/// `07:FA01-FA07` 的 `lsr / beq / iny`），与 `en_projectile_falling_anim_byte` 同窗。
pub fn en_projectile_splatter_anim_byte(area: u8, dir: u8) -> u8 {
    assert!(dir < 4, "敌弹溅落动画索引越窗（data0A>>1={dir}≥4，超出 $95E2-$95E5）");
    en_anim_idx(area)[6 + dir as usize]
}

// ---- `$95DC-$95E4`（9 B）：弹体/爆炸/梅洛姆**动画索引**（按区） ----
//
// asm `prg1_brinstar.asm:142-152`（`{AREA}` 宏五区实例化，五区同址）：
// `$95DC AreaEnProjectileKilledAnimIndex`(1) / `$95DD AreaExplosionAnimIndex`(1) /
// `$95DE-$95DF` 填充(2) / `$95E0 AreaEnProjectileFallingAnimIndex`(2) /
// `$95E2 AreaEnProjectileSplatterAnimIndex`(2) / `$95E4 AreaMellowAnimIndex`(1)。
//
// 值 = `EnAnim_Xxx_{AREA} - EnAnimTable_{AREA}`，即**本区 anim 表下标**——
// 所以它必须与 `assets::enemy_anim_table(area)` 同区配对，跨区使用会取到别的动画。

fn en_anim_idx_key(area: u8) -> Option<PackKey> {
    match area_idx(area)? {
        0 => Some(PackKey::AreaBrEnAnimIdx),
        1 => Some(PackKey::AreaNoEnAnimIdx),
        2 => Some(PackKey::AreaKrEnAnimIdx),
        3 => Some(PackKey::AreaToEnAnimIdx),
        4 => Some(PackKey::AreaRiEnAnimIdx),
        _ => None,
    }
}

fn en_anim_idx(area: u8) -> &'static [u8] {
    let key = en_anim_idx_key(area)
        .unwrap_or_else(|| panic!("敌动画索引：非法区号 area={area:#04X}（InArea&$0F 应为 0..=4）"));
    let b = pack::get(key);
    assert_eq!(b.len(), 12, "敌动画索引段长应为 12 B（含 AI 表头 3 B）");
    b
}

/// `AreaEnProjectileKilledAnimIndex ; $95DC`（1 B）。
pub fn en_projectile_killed_anim(area: u8) -> u8 { en_anim_idx(area)[0] }
/// `AreaExplosionAnimIndex ; $95DD`（1 B）。
pub fn explosion_anim(area: u8) -> u8 { en_anim_idx(area)[1] }
/// `AreaEnProjectileFallingAnimIndex ; $95E0`（word）。
pub fn en_projectile_falling_anim(area: u8) -> u16 {
    u16::from_le_bytes([en_anim_idx(area)[4], en_anim_idx(area)[5]])
}
/// `AreaEnProjectileSplatterAnimIndex ; $95E2`（word）。
pub fn en_projectile_splatter_anim(area: u8) -> u16 {
    u16::from_le_bytes([en_anim_idx(area)[6], en_anim_idx(area)[7]])
}
/// `AreaMellowAnimIndex ; $95E4`（1 B）。
pub fn mellow_anim_index(area: u8) -> u8 { en_anim_idx(area)[8] }

/// SFX 4B 记录窗基址（`$B211`）；窗口内记录以**绝对 ROM 址**引用。
/// SFX 段窗起点：`SFXData@GamePaused ; $B20D`（暂停音 4 B）——其后紧跟
/// `$B211..$B289` 的 4 B 记录窗（记录仍按绝对地址寻址，故起点下移不影响任何调用点）。
pub const SFX_TABLES_ADDR: u16 = 0xB20D;

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

/// 标题屏 17 张小表（bank0；表址/真长/漂移更正说明见 `extract::TITLE_TABLES`）。
pub fn title_fade() -> &'static [u8] { pack::get(PackKey::Fade) }
pub fn title_pal_select() -> &'static [u8] { pack::get(PackKey::PalSelect) }
pub fn title_start_cont() -> &'static [u8] { pack::get(PackKey::StartCont) }
pub fn title_end_star() -> &'static [u8] { pack::get(PackKey::EndStar) }
pub fn title_pw_err() -> &'static [u8] { pack::get(PackKey::PwErr) }
/// `PasswordRowTbl ; 00:9998`（5×u16 LE：`$99A2/$99AF/$99BC/$99C9/$99D6`）。
pub fn title_pw_rows() -> &'static [u8] { pack::get(PackKey::PwRows) }
/// 密码 5×13 网格（`PasswordRow0..` @ `$99A2`，65 B）。
pub fn title_pw_grid() -> &'static [u8] { pack::get(PackKey::PwGrid) }
pub fn title_cursor_x() -> &'static [u8] { pack::get(PackKey::CursorX) }
pub fn title_charsel_y() -> &'static [u8] { pack::get(PackKey::CharselY) }
pub fn title_charsel_x() -> &'static [u8] { pack::get(PackKey::CharselX) }
pub fn title_sparkle_top() -> &'static [u8] { pack::get(PackKey::SparkleTop) }
/// 98 B（49 条，含 `SignMagSpeed $00` 终止）——旧 54 B 截断是烘焙漂移，已更正。
pub fn title_sparkle_bottom() -> &'static [u8] { pack::get(PackKey::SparkleBottom) }
pub fn title_sparkle_init() -> &'static [u8] { pack::get(PackKey::SparkleInit) }
pub fn title_flash_pal() -> &'static [u8] { pack::get(PackKey::ScreenFlashPal) }
pub fn title_init_cross() -> &'static [u8] { pack::get(PackKey::InitCross) }
pub fn title_cross_len() -> &'static [u8] { pack::get(PackKey::CrossLen) }
pub fn title_cross_data() -> &'static [u8] { pack::get(PackKey::CrossData) }

/// `IntroStarPalette0..7 ; 00:8B0F`（8×10B：星闪调色板，按 `IntroStarOffset` 索引）。
pub fn intro_star_palettes() -> &'static [u8] {
    pack::get(PackKey::IntroStarPal)
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

/// ROM 查询表访问器锚（承接 core 侧已删 `memmap/rom_tables.rs` 的 6 条覆盖——
/// 覆盖对象同名同签名，只是数据源从核心层烘焙改为资产包）。
#[cfg(test)]
mod tests_rom_queries {
    use super::*;

    fn boot() {
        init_from_env().ok();
    }

    /// 窗口参数取数：同 Y 不同窗口命中不同 bank 切片（A9 处置核心行为）。
    #[test]
    fn table_968b_is_window_parameterized() {
        boot();
        assert_eq!(rom_table_968b(7, 0), 0x20);
        assert_eq!(rom_table_968b(7, 1), 0x9A);
        assert_eq!(rom_table_968b(0, 1), 0x1A);
        assert_eq!(rom_table_968b(3, 1), 0xFE);
        assert_eq!(rom_table_968b(6, 1), 0x0E);
    }

    /// 向量逐 bank 锚点（ROM 字节自证值）。
    #[test]
    fn rom_vec_per_bank_anchors() {
        boot();
        assert_eq!(rom_vec(7), (0x03, 0x38));
        assert_eq!(rom_vec(0), (0x3F, 0x00));
        assert_eq!(rom_vec(2), (0x00, 0x01));
        assert_eq!(rom_vec(5), (0x40, 0x02));
        assert_eq!(rom_vec(6), (0x25, 0x30));
    }

    /// Y 索引掩 0x7F（BMI 分流契约面）。
    #[test]
    fn table_968b_y_wraps_low7() {
        boot();
        assert_eq!(rom_table_968b(3, 0x80), rom_table_968b(3, 0x00));
        assert_eq!(rom_table_968b(3, 0xFF), rom_table_968b(3, 0x7F));
    }

    /// 迁移锚：8 bank 切片必须彼此不同（否则「按窗口查表」语义塌成单表）。
    #[test]
    fn rom_table_968b_spans_all_eight_banks() {
        boot();
        let first: Vec<u8> = (0..8).map(|w| rom_table_968b(w, 0)).collect();
        let uniq: std::collections::BTreeSet<u8> = first.iter().copied().collect();
        assert!(uniq.len() >= 3, "8 个窗口首字节应有多样性（实测 {uniq:?}）");
        assert_ne!(rom_table_968b(7, 1), rom_table_968b(0, 1));
    }

    /// 迁移锚：流向量 8 bank 逐值。
    #[test]
    fn rom_vec_matches_all_bank_pairs() {
        boot();
        let expect = [
            (0x3F, 0x00), (0x80, 0x00), (0x00, 0x01), (0x00, 0x03),
            (0x00, 0x02), (0x40, 0x02), (0x25, 0x30), (0x03, 0x38),
        ];
        for (w, e) in expect.iter().enumerate() {
            assert_eq!(rom_vec(w), *e, "bank{w} 向量");
        }
    }

    /// 迁移锚：NT 高位表 `$20/$24/$28/$2C`（bank07:C19F，ClearNameTables 清屏目标）。
    #[test]
    fn nametable_hi_is_the_four_screen_bases() {
        boot();
        assert_eq!(nametable_hi(), &[0x20, 0x24, 0x28, 0x2C]);
    }
}

#[cfg(test)]
mod tests_end_sprites {
    use super::*;

    /// `EndSamusAddrTbl ; 00:9D5A` 六指针：段内偏移表首项应为 0（第一帧紧跟表后），
    /// 六帧各 54 B ⇒ 表体 12 B + 6×54 = 336… 实为 338 B（首帧 56 B，末帧按段尾）。
    #[test]
    fn end_samus_frames_are_sliced_by_pointer_table() {
        let f0 = end_samus(0).expect("frame 0");
        assert!(f0.len() >= 50 && f0.len() <= 60, "frame0 len={}", f0.len());
        assert!(end_samus(6).is_none(), "越界必须 None");
        // 六帧长度合计 == 段尾 - 表体
        let total: usize = (0..6).filter_map(|i| end_samus(i)).map(|f| f.len()).sum();
        assert_eq!(total, 338, "六帧合计应与 SegMeta 的 338 B 帧体一致");
    }

    /// `WavePointerTable ; 00:9D12` 四表紧密相邻（段 64 B = 4 表 × 4 项 × 2 B × 2）。
    #[test]
    fn end_wave_is_four_adjacent_tables() {
        let w = end_wave();
        assert_eq!(w.len(), 64);
        // 每表 4 项 u16，首项应指向表体之后（$9D1A + 32 = $9D3A 区的第一项数据）
        let first = u16::from_le_bytes([w[0], w[1]]);
        assert!(first >= 32, "首项偏移 {first:#06X} 应落在表体之后");
    }
}
