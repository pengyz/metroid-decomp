//! ROM → 资产包的提取逻辑（**`extract` feature**，仅 `pack-tool` 使用）。
//!
//! 纪律（见 `docs/spec/data-code-separation.md` §3）：**一个段只有一处定义、一处提取、
//! 一处读取**。段 id 定义在 [`crate::pack::PackKey`]，本文件只负责「从 ROM 切哪一段」，
//! 消费侧只负责「从包里读哪一段」——三者共用同一个 id，不许各写一份。
//!
//! 本文件允许接收 `rom: &[u8]`（`tools-boundary` 门禁白名单）；其余 crate 一律不许。

use crate::pack::{crc32, PackKey, ENTRY_LEN, HEADER_LEN, MAGIC, VERSION};

/// 期望的 ROM 整文件 crc32（`rom/Metroid (USA).nes`，NES USA/NTSC；AGENTS §0 的 sha256
/// 锚点的 crc32 对应值由 `gate rom` 维护）。提取时校验，不符即拒绝。
pub const EXPECTED_ROM_CRC32: u32 = 0xA2C8_9CB9;

/// 期望的 ROM 整文件长度（131 088 B = 16 B iNES 头 + 128 KB PRG；**CHR 是 RAM**
/// ⇒ 文件里没有 CHR 段——AGENTS §0「PRG 128KB + CHR-RAM 8KB」）。
pub const EXPECTED_ROM_SIZE: u32 = 131_088;

/// 一个待写入包的段。
pub struct Section {
    /// 段 id。
    pub key: PackKey,
    /// 段字节。
    pub data: Vec<u8>,
}

/// ROM → 全部段（按 `PackKey` 顺序）。
///
/// 新增段时在这里加一个 `push_section`，并在 `PackKey` 加 id、在消费侧加访问器。
pub fn sections(rom: &[u8]) -> Result<Vec<Section>, String> {
    if rom.len() < 16 {
        return Err(format!("ROM 太短（{} B，至少要 16 B 头）", rom.len()));
    }
    let prg = &rom[16..];
    let mut out = Vec::new();
    out.push(Section { key: PackKey::GfxInfo, data: extract_gfx_info(prg)? });
    out.push(Section { key: PackKey::GfxData, data: extract_gfx_data(prg)? });
    out.extend(extract_enemy_sprites_all(prg)?);
    // 世界地图：`WorldMap: ; 00:A53E`（prg0_title.asm:5118）32×32=1024 B，bank0。
    // `CopyMap $A93E`（prg0_title.asm:5153）运行时把它拷到 WorldMapRAM `$7000-$73FF`。
    out.push(Section { key: PackKey::WorldMap, data: bank_slice(prg, 0, WORLD_MAP_ADDR, WORLD_MAP_LEN)? });
    for spec in [&AREA_BR, &AREA_NO, &AREA_KR, &AREA_TO, &AREA_RI] {
        out.extend(extract_area(prg, spec)?);
    }
    out.extend(extract_enemy_tables_all(prg)?);
    out.extend(extract_crawler_anim_tables(prg)?);
    out.extend(extract_rom_tables(prg)?);
    out.extend(extract_music_tables(prg)?);
    out.extend(extract_music_streams(prg)?);
    out.extend(extract_palette_data(prg)?);
    out.extend(extract_sprite_data(prg)?);
    out.extend(extract_sound_region(prg)?);
    out.extend(extract_items(prg)?);
    out.extend(extract_sfx_tables(prg)?);
    out.extend(extract_song_headers(prg)?);
    out.extend(extract_title_vram(prg)?);
    out.extend(extract_samus_tables(prg)?);
    out.extend(extract_intro_star_palettes(prg)?);
    out.extend(extract_title_tables(prg)?);
    out.extend(extract_tourian_metroid_data(prg)?);
    out.extend(extract_area_pal_toggle(prg)?);
    out.extend(extract_area_vram_strings(prg)?);
    out.extend(extract_area_headers(prg)?);
    out.extend(extract_en_aux(prg)?);
    out.extend(extract_en_anim_idx(prg)?);
    out.extend(extract_en_mov_choices(prg)?);
    out.extend(extract_squeept_speed_y(prg)?);
    out.extend(extract_en_proj_move_str(prg)?);
    Ok(out)
}

/// 敌弹移动串窗（**按区**）——`EnProjectileMovementPtrTable ; $97A7` 的字指针目标并集。
/// asm 依据：prg1_brinstar.asm:117-125 等（`{AREA}` 宏五区实例化，`EnProjectileMovement0..3`）；
/// 消费点 prg7_engine.asm:11915-11930（`lda EnProjectileMovementPtrTable,y` → `lda (Temp0A),y`
/// 读 `[时长][符号幅值速度字节]` 对，`$FF`=串尾重启）。
/// 窗界（ROM 实证 bank*0x4000+addr-0x8000 偏移逐字节对拍）：
/// Br bank01:$9965(60B) / No bank02:$97F7(60B) / To bank03:$97D5(7B，M0/M1 同址别名)
/// / Kr bank04:$98C9(60B) / Ri bank05:$97FD(45B，M0 同址别名)。
struct EnProjMoveStrSpec {
    key: PackKey,
    bank: u8,
    addr: u16,
    len: usize,
    rom: &'static str,
    want: &'static [u8],
}

const EN_PROJ_MOVE_STR_SPECS: [EnProjMoveStrSpec; 5] = [
    // Br：fireball 首对=04 B3（时长4，dy=-5，dx=+3）。 机械自检改采样锚（前两字节 + 中位锚 + 尾字节）：全窗 60 B 不再
    // 进仓（data-boundary 阈值 200）；addr/len + pack verify（段==ROM 重算）+ 采样锚
    // 三重机械锚定，全窗字节已用 python 文件偏移公式（16+bank*0x4000+addr-0x8000）
    // 逐字节人工对拍过（commit Evidence 留痕）。
    EnProjMoveStrSpec {
        key: PackKey::AreaBrEnProjMoveStr,
        bank: 1,
        addr: 0x9965,
        len: 60,
        rom: "bank01:$9965",
        want: &[0x04, 0xB3, 0x07, 0xC2, 0x72, 0xFF],
    },
    // Norfair fireball 首对=0A D3（时长10，dy=-6，dx=+3）。 机械自检改采样锚（前两字节 + 中位锚 + 尾字节）：全窗 60 B 不再
    // 进仓（data-boundary 阈值 200）；addr/len + pack verify（段==ROM 重算）+ 采样锚
    // 三重机械锚定，全窗字节已用 python 文件偏移公式（16+bank*0x4000+addr-0x8000）
    // 逐字节人工对拍过（commit Evidence 留痕）。
    EnProjMoveStrSpec {
        key: PackKey::AreaNoEnProjMoveStr,
        bank: 2,
        addr: 0x97F7,
        len: 60,
        rom: "bank02:$97F7",
        want: &[0x0A, 0xD3, 0x07, 0xC2, 0x72, 0xFF],
    },
    // Tourian：M0/M1 同址别名（$97D5），窗=50 22 FF 50 30 FF FF。 机械自检改采样锚（前两字节 + 中位锚 + 尾字节）：全窗 60 B 不再
    // 进仓（data-boundary 阈值 200）；addr/len + pack verify（段==ROM 重算）+ 采样锚
    // 三重机械锚定，全窗字节已用 python 文件偏移公式（16+bank*0x4000+addr-0x8000）
    // 逐字节人工对拍过（commit Evidence 留痕）。
    EnProjMoveStrSpec {
        key: PackKey::AreaToEnProjMoveStr,
        bank: 3,
        addr: 0x97D5,
        len: 7,
        rom: "bank03:$97D5",
        want: &[0x50, 0x22, 0x50, 0x30, 0xFF, 0xFF],
    },
    // Kraid 区：与 Brinstar 同字节（ROM 对拍证实）。 机械自检改采样锚（前两字节 + 中位锚 + 尾字节）：全窗 60 B 不再
    // 进仓（data-boundary 阈值 200）；addr/len + pack verify（段==ROM 重算）+ 采样锚
    // 三重机械锚定，全窗字节已用 python 文件偏移公式（16+bank*0x4000+addr-0x8000）
    // 逐字节人工对拍过（commit Evidence 留痕）。
    EnProjMoveStrSpec {
        key: PackKey::AreaKrEnProjMoveStr,
        bank: 4,
        addr: 0x98C9,
        len: 60,
        rom: "bank04:$98C9",
        want: &[0x04, 0xB3, 0x07, 0xC2, 0x72, 0xFF],
    },
    // Ridley 区：M0 空别名到 M1（$97FD），三串 polyp 15B。 机械自检改采样锚（前两字节 + 中位锚 + 尾字节）：全窗 60 B 不再
    // 进仓（data-boundary 阈值 200）；addr/len + pack verify（段==ROM 重算）+ 采样锚
    // 三重机械锚定，全窗字节已用 python 文件偏移公式（16+bank*0x4000+addr-0x8000）
    // 逐字节人工对拍过（commit Evidence 留痕）。
    EnProjMoveStrSpec {
        key: PackKey::AreaRiEnProjMoveStr,
        bank: 5,
        addr: 0x97FD,
        len: 45,
        rom: "bank05:$97FD",
        want: &[0x09, 0xC2, 0x12, 0x06, 0x72, 0xFF],
    },
];

fn extract_en_proj_move_str(prg: &[u8]) -> Result<Vec<Section>, String> {
    let mut out = Vec::new();
    for spec in &EN_PROJ_MOVE_STR_SPECS {
        let body = bank_slice(prg, spec.bank, spec.addr, spec.len)?;
        // 机械自检（采样锚形）：首两字节（时长+速度对，SignMagSpeed 宏展开错位即全错）
        // + 中位锚 + 尾两字节。addr/len 定位 + pack verify（段==ROM 重算）共同锚定全窗。
        let w = spec.want;
        let n = w.len();
        let m = spec.len / 2;
        let ok = body.len() == spec.len
            && w.len() == 6
            && body[0] == w[0]
            && body[1] == w[1]
            && body[m] == w[2]
            && body[m + 1] == w[3]
            && body[spec.len - 2] == w[4]
            && body[spec.len - 1] == w[5]
            && w[5] == 0xFF;
        if !ok {
            return Err(format!(
                "{} 敌弹移动串采样锚不符：head={:02X}{:02X} mid@{m}={:02X}{:02X} tail={:02X}{:02X} != asm 锚 {:02X?}",
                spec.rom, body[0], body[1], body[m], body[m + 1], body[spec.len - 2], body[spec.len - 1], w
            ));
        }
        out.push(Section { key: spec.key, data: body.to_vec() });
    }
    Ok(out)
}

/// `SqueeptSpeedYTable ; 02:9AD2`（4 B：`F6 F8 F6 FA`）。
/// asm 依据：`squeept.asm`（`SqueeptAIRoutine 02:9A64` 内 `lda SqueeptSpeedYTable,y`）
/// **只被 prg2_norfair.asm:675 include** ⇒ 仅 Norfair bank 有此段，按区单段不循环五区。
const SQUEEPT_SPEED_Y_ADDR: u16 = 0x9AD2;
const SQUEEPT_SPEED_Y_LEN: usize = 4;

fn extract_squeept_speed_y(prg: &[u8]) -> Result<Vec<Section>, String> {
    let body = bank_slice(prg, 2, SQUEEPT_SPEED_Y_ADDR, SQUEEPT_SPEED_Y_LEN)?;
    // 机械自检：asm 标注的字节逐一对拍（表只有 4 项，错位即全错）。
    const WANT: [u8; 4] = [0xF6, 0xF8, 0xF6, 0xFA];
    if body != WANT {
        return Err(format!(
            "bank02:$9AD2 SqueeptSpeedYTable 字节不符：{:02X?} != asm 标注 {:02X?}",
            body, WANT
        ));
    }
    Ok(vec![Section { key: PackKey::AreaNoSqueeptSpeedY, data: body.to_vec() }])
}

/// `GFXInfo`（`07:C6E0`，29 项 × 7 B）——原样字节。
fn extract_gfx_info(prg: &[u8]) -> Result<Vec<u8>, String> {
    let off = gfx_info_offset();
    let end = off + GFX_ENTRIES * 7;
    if end > prg.len() {
        return Err(format!("GFXInfo 越界：需要 {end} B，PRG {} B", prg.len()));
    }
    Ok(prg[off..end].to_vec())
}

/// GFX 数据体：按 info 的 `[bank][src][dest][size]` 逐项切 ROM 再拼接。
fn extract_gfx_data(prg: &[u8]) -> Result<Vec<u8>, String> {
    let info = extract_gfx_info(prg)?;
    let mut data = Vec::new();
    for i in 0..GFX_ENTRIES {
        let e = &info[i * 7..(i + 1) * 7];
        let bank = e[0];
        let src = u16::from_le_bytes([e[1], e[2]]);
        let size = u16::from_le_bytes([e[5], e[6]]) as usize;
        let o = bank_source_offset(bank, src)?;
        let end = o
            .checked_add(size)
            .ok_or_else(|| format!("GFX 项 {i} 越界（src={src:04X} size={size}）"))?;
        if end > prg.len() {
            return Err(format!("GFX 项 {i} 越界：需要 {end} B，PRG {} B", prg.len()));
        }
        data.extend_from_slice(&prg[o..end]);
    }
    Ok(data)
}

// ---- 敌帧/位数据面（bank1 $9D6A-$9FBE 族；移植自 tools/bake_enemy_gfx.py）--------
//
// ROM 锚（A 级）：`AreaPointers_ROM` 7 字（prg1:88）给出四张表的 CPU 地址
// EnAnimTable `$9D6A` / EnFramePtrTable1 `$9DE0` / EnFramePtrTable2 `$9EE0` /
// EnPlacePtrTable `$9F0E`；`CopyAreaPointers $C8B0`（prg7:1936）搬到 RAM `$3B-$48`。
//
// **表长由相邻标签地址机械给出**（本波定谳，取代旧烘焙器的 128/128/128 假设）：
//   anim  `$9DE0 - $9D6A` = 118 B  ⇒ **118 项**（旧烘焙读 128 ⇒ 末 10 项实为帧表头部）
//   frame `$9F0E - $9DE0` = 302 B  ⇒ **151 项**（旧烘焙读 2×128=256 ⇒ 105 项越界）
//   place `$9F0E` 表内 14 个指针后即数据（首指针 `$9F2E`）⇒ **14 项**
//   帧指针 151 项、place 指针 14 项**全部 ≥ $8000**（合法 ROM 指针）；
//   旧烘焙器按 128 项读把相邻数据当指针，才出现 34/44 个 `< $8000` 的伪值。
// 帧流=[ctrl][radiusY][radiusX][流…$FF]；位流=y/x 对。
/// `WorldMap` 的 CPU 地址（`refs/m1disasm/SRC/prg0_title.asm:5118`）与长度。
const WORLD_MAP_ADDR: u16 = 0xA53E;
const WORLD_MAP_LEN: usize = 1024;

/// 按 `(bank, cpu addr)` 取 ROM 字节（bank0-6 落 `$8000` 窗口，bank7 落 `$C000`）。
fn bank_slice(prg: &[u8], bank: u8, addr: u16, len: usize) -> Result<Vec<u8>, String> {
    let o = bank_source_offset(bank, addr)?;
    prg.get(o..o + len)
        .map(|s| s.to_vec())
        .ok_or_else(|| format!("ROM 越界 bank{bank}:{addr:04X} len={len}"))
}

/// 按 `(bank, cpu addr)` 取**敌数据窗口**的 PRG 偏移（bank0-6 落 `$8000` 窗口）。
fn en_off_b(bank: u8, addr: u16) -> Result<usize, String> {
    if addr < 0x8000 {
        return Err(format!("敌数据指针非法（< $8000）: bank{bank}:{addr:04X}"));
    }
    bank_source_offset(bank, addr)
}

fn en_slice_b(prg: &[u8], bank: u8, addr: u16, len: usize) -> Result<Vec<u8>, String> {
    let o = en_off_b(bank, addr)?;
    prg.get(o..o + len)
        .map(|s| s.to_vec())
        .ok_or_else(|| format!("敌数据越界 bank{bank}:{addr:04X} off={o} len={len}"))
}

/// 帧流长（`[ctrl][ry][rx][流…$FF]`）——解析规则**只在提取侧实现一份**。
///
/// 流形：`<$FC` 单字节 / `=$FC` 偏移 3B / `=$FD` 新 ctrl 2B / `=$FE` 跳位 1B / `=$FF` 尾。
fn frame_end_b(prg: &[u8], bank: u8, addr: u16) -> Result<usize, String> {
    let o = en_off_b(bank, addr)?;
    let mut n = 3usize;
    while n < 256 {
        let b = *prg
            .get(o + n)
            .ok_or_else(|| format!("帧流越界 bank{bank}:{addr:04X} n={n}"))?;
        if b == 0xFF {
            return Ok(n + 1);
        }
        n += match b {
            0xFC => 3,
            0xFD => 2,
            _ => 1,
        };
    }
    Ok(n)
}

/// 位件长（至下一指针；退化情形给 16B 下界）——与旧烘焙器同口径（见 findings §6 的真值欠账）。
fn place_end(addr: u16, nxt: u16) -> usize {
    if nxt > addr {
        (nxt as usize - addr as usize).max(4)
    } else {
        16
    }
}

/// 一个区的敌帧/位数据面基址与表长（`docs/findings/per-bank-assets.md` §1，ROM 实读）。
///
/// 三条机械关系在五区全部成立：`anim_len = fptr - anim`、`fptr_entries = (place - fptr)/2`、
/// `place_cnt = (place 表首指针 - place)/2`（提取时会逐条断言，防手抄漂移）。
#[derive(Clone, Copy)]
pub struct EnemySpriteSpec {
    pub bank: u8,
    pub anim: u16,
    pub anim_len: usize,
    pub fptr: u16,
    pub fptr_entries: usize,
    pub place: u16,
    pub place_cnt: usize,
}

/// 五区（area 序 Br/No/Kr/To/Ri；bank 序 `BankTable 07:CA30 = 02 03 05 04 06` 减 1）。
pub const ENEMY_SPRITES: [EnemySpriteSpec; 5] = [
    EnemySpriteSpec { bank: 1, anim: 0x9D6A, anim_len: 118, fptr: 0x9DE0, fptr_entries: 151, place: 0x9F0E, place_cnt: 16 },
    EnemySpriteSpec { bank: 2, anim: 0x9BDA, anim_len: 138, fptr: 0x9C64, fptr_entries: 138, place: 0x9D78, place_cnt: 14 },
    EnemySpriteSpec { bank: 3, anim: 0xA406, anim_len: 38,  fptr: 0xA42C, fptr_entries: 138, place: 0xA540, place_cnt: 16 },
    EnemySpriteSpec { bank: 4, anim: 0x9C86, anim_len: 113, fptr: 0x9CF7, fptr_entries: 151, place: 0x9E25, place_cnt: 16 },
    EnemySpriteSpec { bank: 5, anim: 0x9B85, anim_len: 107, fptr: 0x9BF0, fptr_entries: 138, place: 0x9D04, place_cnt: 15 },
];

/// 敌帧/位数据面：**每区一套**复合段（布局见 `pack::en_spr_sections`）。
///
/// 旧形是 bank1 单表（`EN_ANIM_TABLE/EN_FRAME_PTRS/EN_PLACE_PTRS` 硬编码）⇒ 非 Brinstar
/// 敌人的 `animIndex` 去查了 Brinstar 的表：Ridley 的 Dessgeega 取 `0x42`，`bank1[0x42]`
/// 是 `0xFF` ⇒ 16 次原地回绕，落 `unported!("bank07:E0BC-wrap")`。
fn extract_enemy_sprites_all(prg: &[u8]) -> Result<Vec<Section>, String> {
    const KEYS: [PackKey; 5] = [
        PackKey::BrEnSprites,
        PackKey::NoEnSprites,
        PackKey::KrEnSprites,
        PackKey::ToEnSprites,
        PackKey::RiEnSprites,
    ];
    let mut out = Vec::with_capacity(5);
    for (i, spec) in ENEMY_SPRITES.iter().enumerate() {
        let b = spec.bank;
        // 机械自检：表长与相邻标签地址一致（防手抄漂移；不等即拒出包）。
        let anim_end = spec.anim.wrapping_add(spec.anim_len as u16);
        if anim_end != spec.fptr {
            return Err(format!(
                "敌帧表基址不自洽（area {i}）：anim {:#06X}+{} != fptr {:#06X}",
                spec.anim, spec.anim_len, spec.fptr
            ));
        }
        let fptr_end = spec.fptr.wrapping_add((spec.fptr_entries * 2) as u16);
        if fptr_end != spec.place {
            return Err(format!(
                "敌帧指针表不自洽（area {i}）：fptr {:#06X}+{}B != pptr {:#06X}",
                spec.fptr,
                spec.fptr_entries * 2,
                spec.place
            ));
        }

        let anim = en_slice_b(prg, b, spec.anim, spec.anim_len)?;
        let f = en_slice_b(prg, b, spec.fptr, spec.fptr_entries * 2)?;
        let pl = en_slice_b(prg, b, spec.place, spec.place_cnt * 2)?;

        // place 项数自检：表首指针必须等于 place 基址 + 2*项数。
        let first_place = u16::from_le_bytes([pl[0], pl[1]]);
        let expect = spec.place + (spec.place_cnt * 2) as u16;
        if first_place != expect {
            return Err(format!(
                "place 表项数不自洽（area {i}）：首指针 {first_place:#06X} != {expect:#06X}"
            ));
        }

        // 帧数据体：逐指针取流长（`$0000` 项留 (0,0) —— 原版表里确有零项）。
        let mut frames = Vec::new();
        let mut foffs: Vec<(u32, u32)> = Vec::with_capacity(spec.fptr_entries);
        for k in (0..spec.fptr_entries * 2).step_by(2) {
            let addr = u16::from_le_bytes([f[k], f[k + 1]]);
            if addr == 0 {
                foffs.push((0, 0));
                continue;
            }
            let ln = frame_end_b(prg, b, addr)?;
            foffs.push((frames.len() as u32, ln as u32));
            frames.extend_from_slice(&en_slice_b(prg, b, addr, ln)?);
        }

        // 位件数据体：长=至下一指针（末项走退化分支）。
        let addrs: Vec<u16> = (0..spec.place_cnt * 2)
            .step_by(2)
            .map(|k| u16::from_le_bytes([pl[k], pl[k + 1]]))
            .collect();
        let mut places = Vec::new();
        let mut poffs: Vec<(u32, u32)> = Vec::with_capacity(spec.place_cnt);
        for (k, &addr) in addrs.iter().enumerate() {
            if addr == 0 {
                poffs.push((0, 0));
                continue;
            }
            let nxt = addrs.get(k + 1).copied().unwrap_or(0);
            let ln = place_end(addr, nxt);
            poffs.push((places.len() as u32, ln as u32));
            places.extend_from_slice(&en_slice_b(prg, b, addr, ln)?);
        }

        let foffs = encode_offs(&foffs)?;
        let poffs = encode_offs(&poffs)?;

        // 复合段 = 头 6×u32 + 七块定序拼接（布局见 pack::en_spr_sections）。
        let mut blob = Vec::with_capacity(24 + anim.len() + f.len() + pl.len());
        for v in [
            spec.anim_len as u32,
            spec.fptr_entries as u32,
            spec.place_cnt as u32,
            frames.len() as u32,
            places.len() as u32,
            0u32,
        ] {
            blob.extend_from_slice(&v.to_le_bytes());
        }
        blob.extend_from_slice(&anim);
        blob.extend_from_slice(&f);
        blob.extend_from_slice(&pl);
        blob.extend_from_slice(&frames);
        blob.extend_from_slice(&places);
        blob.extend_from_slice(&foffs);
        blob.extend_from_slice(&poffs);
        out.push(Section { key: KEYS[i], data: blob });
    }
    Ok(out)
}

/// `(off,len)` 表 → LE u32 对字节流。
fn encode_offs(offs: &[(u32, u32)]) -> Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity(offs.len() * 8);
    for (o, l) in offs {
        out.extend_from_slice(&o.to_le_bytes());
        out.extend_from_slice(&l.to_le_bytes());
    }
    Ok(out)
}

/// `GFXInfo` 在 **PRG（去掉 16 B 头）** 内的偏移：bank7 固定窗口 `$C000` ⇒ `$C6E0`。
pub const fn gfx_info_offset() -> usize {
    7 * 0x4000 + (0xC6E0 - 0xC000)
}

/// GFX 描述子条目数（`$C6E0-$C7AB` = 203 B = 29 × 7）。
pub const GFX_ENTRIES: usize = 29;

/// 把一个 `(bank, cpu addr)` 换算成 PRG 内偏移。
///
/// bank 7 固定映射 `$C000-$FFFF`；bank 0-6 映射 `$8000-$BFFF`。
pub fn bank_source_offset(bank: u8, addr: u16) -> Result<usize, String> {
    let base = if bank == 7 {
        if addr < 0xC000 {
            return Err(format!("bank7 地址 {addr:04X} < $C000（固定窗口外）"));
        }
        (addr - 0xC000) as usize
    } else {
        if addr < 0x8000 {
            return Err(format!("bank{bank} 地址 {addr:04X} < $8000（切 bank 窗口外）"));
        }
        (addr - 0x8000) as usize
    };
    Ok(bank as usize * 0x4000 + base)
}

/// 段清单序列化成包字节（格式见 [`crate::pack`] 模块文档）。
pub fn write_pack(sections: &[Section], rom_crc32: u32, rom_size: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity(HEADER_LEN + sections.len() * ENTRY_LEN);
    out.extend_from_slice(&MAGIC);
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // flags
    out.extend_from_slice(&rom_crc32.to_le_bytes());
    out.extend_from_slice(&rom_size.to_le_bytes());
    out.extend_from_slice(&(sections.len() as u16).to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // reserved
    out.extend_from_slice(&0u32.to_le_bytes()); // reserved2
    out.extend_from_slice(&0u32.to_le_bytes()); // reserved3
    let payload_off = (HEADER_LEN + sections.len() * ENTRY_LEN) as u32;
    out.extend_from_slice(&payload_off.to_le_bytes());
    debug_assert_eq!(out.len(), HEADER_LEN);
    let mut off = 0u32;
    for s in sections {
        out.extend_from_slice(&s.key.id().to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // flags
        out.extend_from_slice(&off.to_le_bytes());
        out.extend_from_slice(&(s.data.len() as u32).to_le_bytes());
        out.extend_from_slice(&crc32(&s.data).to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes()); // reserved
        off += s.data.len() as u32;
    }
    for s in sections {
        out.extend_from_slice(&s.data);
    }
    out
}


// ---- 段元信息（JSON，打进包里，**只由 pack-tool 读写**）----------------------------
//
// 纪律：表长/编码/自界规则这些「格式知识」的唯一权威是**本文件的常量与提取函数**；
// meta JSON 是它们的**派生投影**，用于自描述 + 漂移检测，不是第二真相源。
// `pack-tool verify` 会校验「count × elem == 段实测长度」，不一致即硬错。

/// 一段的元信息（生成 JSON 用；`elem`=单元素字节数，`count`=元素数，均来自本文件常量）。
struct SegMeta {
    key: PackKey,
    kind: &'static str,
    elem: usize,
    count: usize,
    rom: &'static str,
    note: &'static str,
}

fn seg_meta() -> Vec<SegMeta> {
    let v = vec![
        SegMeta { key: PackKey::GfxInfo, kind: "gfx-desc", elem: 7, count: GFX_ENTRIES,
                  rom: "bank07:$C6E0", note: "GFXInfo 29 项×7B（LoadGFX 表间接读）" },
        SegMeta { key: PackKey::GfxData, kind: "gfx-data", elem: 1, count: 0,
                  rom: "bank00-07:按描述子", note: "29 项图形块拼接（count 由 Info 尺寸和决定）" },
        // 敌帧/位数据面：**每区一套复合段**（头 6×u32 + anim/fptr/pptr/frames/places/foffs/poffs）。
        SegMeta { key: PackKey::BrEnSprites, kind: "en-spr", elem: 1, count: 0,
                  rom: "bank01:$9D6A+", note: "Brinstar anim118/fptr151/place16（规格见 ENEMY_SPRITES[0]）" },
        SegMeta { key: PackKey::NoEnSprites, kind: "en-spr", elem: 1, count: 0,
                  rom: "bank02:$9BDA+", note: "Norfair anim138/fptr138/place14（ENEMY_SPRITES[1]）" },
        SegMeta { key: PackKey::KrEnSprites, kind: "en-spr", elem: 1, count: 0,
                  rom: "bank04:$9C86+", note: "Kraid anim113/fptr151/place16（ENEMY_SPRITES[2]）" },
        SegMeta { key: PackKey::ToEnSprites, kind: "en-spr", elem: 1, count: 0,
                  rom: "bank03:$A406+", note: "Tourian anim38/fptr138/place16（ENEMY_SPRITES[3]）" },
        SegMeta { key: PackKey::RiEnSprites, kind: "en-spr", elem: 1, count: 0,
                  rom: "bank05:$9B85+", note: "Ridley anim107/fptr138/place15（ENEMY_SPRITES[4]）" },
        SegMeta { key: PackKey::WorldMap, kind: "bytes", elem: 1, count: WORLD_MAP_LEN,
                  rom: "bank00:$A53E", note: "WorldMap 32×32（$FF=无房；CopyMap 源）" },
        SegMeta { key: PackKey::BrRoomPtrs, kind: "u16-table", elem: 2, count: 47, rom: "bank01:$A314", note: "Brinstar RoomPtrTable" },
        SegMeta { key: PackKey::BrStructPtrs, kind: "u16-table", elem: 2, count: 50, rom: "bank01:$A372", note: "Brinstar StructPtrTable" },
        SegMeta { key: PackKey::BrMetatiles, kind: "bytes", elem: 4, count: 65, rom: "bank01:$AEF0", note: "Brinstar 元tile 4B 组×65" },
        SegMeta { key: PackKey::BrRoomStreams, kind: "bytes", elem: 1, count: 0, rom: "bank01:各指针", note: "Brinstar 房流拼接（各自界 $FF）" },
        SegMeta { key: PackKey::BrRoomOffs, kind: "offs-table", elem: 8, count: 47, rom: "(派生)", note: "×47 (off,len)" },
        SegMeta { key: PackKey::BrStructures, kind: "bytes", elem: 1, count: 0, rom: "bank01:各指针", note: "Brinstar 结构体拼接" },
        SegMeta { key: PackKey::BrStructOffs, kind: "offs-table", elem: 8, count: 50, rom: "(派生)", note: "×50 (off,len)" },
        SegMeta { key: PackKey::NoRoomPtrs, kind: "u16-table", elem: 2, count: 46, rom: "bank02:$A21B", note: "Norfair RoomPtrTable" },
        SegMeta { key: PackKey::NoStructPtrs, kind: "u16-table", elem: 2, count: 49, rom: "bank02:$A277", note: "Norfair StructPtrTable" },
        SegMeta { key: PackKey::NoMetatiles, kind: "bytes", elem: 4, count: 62, rom: "bank02:$AEEC", note: "Norfair 元tile 4B 组×62" },
        SegMeta { key: PackKey::NoRoomStreams, kind: "bytes", elem: 1, count: 0, rom: "bank02:各指针", note: "Norfair 房流拼接" },
        SegMeta { key: PackKey::NoRoomOffs, kind: "offs-table", elem: 8, count: 46, rom: "(派生)", note: "×46 (off,len)" },
        SegMeta { key: PackKey::NoStructures, kind: "bytes", elem: 1, count: 0, rom: "bank02:各指针", note: "Norfair 结构体拼接" },
        SegMeta { key: PackKey::BrEnemyDeathAnim, kind: "bytes", elem: 1, count: 32, rom: "bank01:960B", note: "EnemyDeathAnimIndex（type×2+向；按区不同）" },
        SegMeta { key: PackKey::NoEnemyDeathAnim, kind: "bytes", elem: 1, count: 32, rom: "bank02:960B", note: "EnemyDeathAnimIndex（type×2+向；按区不同）" },
        SegMeta { key: PackKey::KrEnemyDeathAnim, kind: "bytes", elem: 1, count: 32, rom: "bank04:960B", note: "EnemyDeathAnimIndex（type×2+向；按区不同）" },
        SegMeta { key: PackKey::ToEnemyDeathAnim, kind: "bytes", elem: 1, count: 32, rom: "bank03:960B", note: "EnemyDeathAnimIndex（type×2+向；按区不同）" },
        SegMeta { key: PackKey::RiEnemyDeathAnim, kind: "bytes", elem: 1, count: 32, rom: "bank05:960B", note: "EnemyDeathAnimIndex（type×2+向；按区不同）" },
        SegMeta { key: PackKey::BrCrawlerAnim, kind: "bytes", elem: 1, count: 8, rom: "bank01:9AA0", note: "CrawlerAnimIndexTable（orient×2+flip；按区不同）" },
        SegMeta { key: PackKey::NoCrawlerAnim, kind: "bytes", elem: 1, count: 8, rom: "bank02:99ED", note: "CrawlerAnimIndexTable（Nova 表；按区不同）" },
        SegMeta { key: PackKey::KrCrawlerAnim, kind: "bytes", elem: 1, count: 8, rom: "bank04:99EF", note: "CrawlerAnimIndexTable（Zeela 表；按区不同）" },
        SegMeta { key: PackKey::RiCrawlerAnim, kind: "bytes", elem: 1, count: 8, rom: "bank05:99BE", note: "CrawlerAnimIndexTable（Viola 表；按区不同）" },
        SegMeta { key: PackKey::BrEnemyHealth, kind: "bytes", elem: 1, count: 64, rom: "bank01:962B", note: "EnemyHealthTbl（asm 标签间距见 note）" },
        SegMeta { key: PackKey::BrEnemyRestingAnim, kind: "bytes", elem: 1, count: 32, rom: "bank01:963B", note: "EnemyRestingAnimIndex（asm 标签间距见 note）" },
        SegMeta { key: PackKey::BrEnemyActiveAnim, kind: "bytes", elem: 1, count: 32, rom: "bank01:965B", note: "EnemyActiveAnimIndex（asm 标签间距见 note）" },
        SegMeta { key: PackKey::BrEnemyActiveAnimInitOffset, kind: "bytes", elem: 1, count: 16, rom: "bank01:967B", note: "InitOffset（asm 标签间距见 note）" },
        SegMeta { key: PackKey::BrEnemyForceAttribs, kind: "bytes", elem: 1, count: 16, rom: "bank01:968B", note: "ForceAttribs（asm 标签间距见 note）" },
        SegMeta { key: PackKey::BrEnemyForceSpeedDelay, kind: "bytes", elem: 1, count: 64, rom: "bank01:969B", note: "ForceSpeedDelay（asm 标签间距见 note）" },
        SegMeta { key: PackKey::BrEnemyDistanceThreshold, kind: "bytes", elem: 1, count: 16, rom: "bank01:96AB", note: "DistanceThreshold（asm 标签间距见 note）" },
        SegMeta { key: PackKey::BrEnemyInitDelay, kind: "bytes", elem: 1, count: 64, rom: "bank01:96BB", note: "InitDelay（asm 标签间距见 note）" },
        SegMeta { key: PackKey::BrEnemyFacing, kind: "bytes", elem: 1, count: 64, rom: "bank01:977B", note: "Facing（asm 标签间距见 note）" },
        SegMeta { key: PackKey::BrEnMovPtrs, kind: "words", elem: 2, count: 36, rom: "bank01:96DB", note: "EnemyMovementPtrs 18 索引 × R/L = 36 项 (72 B)；idx36 起全 $0000（见 docs/findings/enemy-movement-ptr-truncation.md）" },
        SegMeta { key: PackKey::BrEnMovStr, kind: "bytes", elem: 1, count: 512, rom: "bank01:97EF", note: "移动指令串窗 $97EF..$99EF（覆盖被引用上界）" },
        // ---- Kr（bank4）房流与敌表 ----
        SegMeta { key: PackKey::KrRoomPtrs, kind: "u16-table", elem: 2, count: 37, rom: "bank04:A1D5", note: "RoomPtrTable" },
        SegMeta { key: PackKey::KrStructPtrs, kind: "u16-table", elem: 2, count: 39, rom: "bank04:A21F", note: "StructPtrTable" },
        SegMeta { key: PackKey::KrMetatiles, kind: "bytes", elem: 4, count: 41, rom: "bank04:AC32", note: "元tile 4B 组×41" },
        SegMeta { key: PackKey::KrRoomStreams, kind: "bytes", elem: 1, count: 0, rom: "bank04:各指针", note: "房流拼接（各自界 $FF）" },
        SegMeta { key: PackKey::KrRoomOffs, kind: "offs-table", elem: 8, count: 37, rom: "(派生)", note: "×37 (off,len)" },
        SegMeta { key: PackKey::KrStructures, kind: "bytes", elem: 1, count: 0, rom: "bank04:各指针", note: "结构体拼接" },
        SegMeta { key: PackKey::KrStructOffs, kind: "offs-table", elem: 8, count: 39, rom: "(派生)", note: "×39 (off,len)" },
        SegMeta { key: PackKey::KrEnemyHealth, kind: "bytes", elem: 1, count: 64, rom: "bank04:962B", note: "EnemyHealthTbl（窗口；语义 16）" },
        SegMeta { key: PackKey::KrEnemyRestingAnim, kind: "bytes", elem: 1, count: 32, rom: "bank04:963B", note: "EnemyRestingAnimIndex" },
        SegMeta { key: PackKey::KrEnemyActiveAnim, kind: "bytes", elem: 1, count: 32, rom: "bank04:965B", note: "EnemyActiveAnimIndex" },
        SegMeta { key: PackKey::KrEnemyActiveAnimInitOffset, kind: "bytes", elem: 1, count: 16, rom: "bank04:967B", note: "EnemyActiveAnimIndexInitOffset" },
        SegMeta { key: PackKey::KrEnemyForceAttribs, kind: "bytes", elem: 1, count: 16, rom: "bank04:968B", note: "L968B 强制属性" },
        SegMeta { key: PackKey::KrEnemyForceSpeedDelay, kind: "bytes", elem: 1, count: 64, rom: "bank04:969B", note: "ForceSpeedTowardsSamusDelayTbl（窗口；语义 16）" },
        SegMeta { key: PackKey::KrEnemyDistanceThreshold, kind: "bytes", elem: 1, count: 16, rom: "bank04:96AB", note: "EnemyDistanceToSamusThreshold" },
        SegMeta { key: PackKey::KrEnemyInitDelay, kind: "bytes", elem: 1, count: 64, rom: "bank04:96BB", note: "EnemyInitDelayTbl（窗口；语义 32）" },
        SegMeta { key: PackKey::KrEnemyFacing, kind: "bytes", elem: 1, count: 64, rom: "bank04:977B", note: "L977B 朝向表" },
        SegMeta { key: PackKey::KrEnMovPtrs, kind: "words", elem: 2, count: 36, rom: "bank04:96DB", note: "EnemyMovementPtrs 18 索引 × R/L = 36 项 (72 B)" },
        SegMeta { key: PackKey::KrEnMovStr, kind: "bytes", elem: 1, count: 512, rom: "bank04:97EF", note: "移动指令串窗" },
        // ---- To（bank3）房流与敌表 ----
        SegMeta { key: PackKey::ToRoomPtrs, kind: "u16-table", elem: 2, count: 21, rom: "bank03:A7D1", note: "RoomPtrTable" },
        SegMeta { key: PackKey::ToStructPtrs, kind: "u16-table", elem: 2, count: 32, rom: "bank03:A7FB", note: "StructPtrTable" },
        SegMeta { key: PackKey::ToMetatiles, kind: "bytes", elem: 4, count: 54, rom: "bank03:AE49", note: "元tile 4B 组×54" },
        SegMeta { key: PackKey::ToRoomStreams, kind: "bytes", elem: 1, count: 0, rom: "bank03:各指针", note: "房流拼接（各自界 $FF）" },
        SegMeta { key: PackKey::ToRoomOffs, kind: "offs-table", elem: 8, count: 21, rom: "(派生)", note: "×21 (off,len)" },
        SegMeta { key: PackKey::ToStructures, kind: "bytes", elem: 1, count: 0, rom: "bank03:各指针", note: "结构体拼接" },
        SegMeta { key: PackKey::ToStructOffs, kind: "offs-table", elem: 8, count: 32, rom: "(派生)", note: "×32 (off,len)" },
        SegMeta { key: PackKey::ToEnemyHealth, kind: "bytes", elem: 1, count: 64, rom: "bank03:962B", note: "EnemyHealthTbl（窗口；语义 16）" },
        SegMeta { key: PackKey::ToEnemyRestingAnim, kind: "bytes", elem: 1, count: 32, rom: "bank03:963B", note: "EnemyRestingAnimIndex" },
        SegMeta { key: PackKey::ToEnemyActiveAnim, kind: "bytes", elem: 1, count: 32, rom: "bank03:965B", note: "EnemyActiveAnimIndex" },
        SegMeta { key: PackKey::ToEnemyActiveAnimInitOffset, kind: "bytes", elem: 1, count: 16, rom: "bank03:967B", note: "EnemyActiveAnimIndexInitOffset" },
        SegMeta { key: PackKey::ToEnemyForceAttribs, kind: "bytes", elem: 1, count: 16, rom: "bank03:968B", note: "L968B 强制属性" },
        SegMeta { key: PackKey::ToEnemyForceSpeedDelay, kind: "bytes", elem: 1, count: 64, rom: "bank03:969B", note: "ForceSpeedTowardsSamusDelayTbl（窗口；语义 16）" },
        SegMeta { key: PackKey::ToEnemyDistanceThreshold, kind: "bytes", elem: 1, count: 16, rom: "bank03:96AB", note: "EnemyDistanceToSamusThreshold" },
        SegMeta { key: PackKey::ToEnemyInitDelay, kind: "bytes", elem: 1, count: 64, rom: "bank03:96BB", note: "EnemyInitDelayTbl（窗口；语义 32）" },
        SegMeta { key: PackKey::ToEnemyFacing, kind: "bytes", elem: 1, count: 64, rom: "bank03:977B", note: "L977B 朝向表" },
        SegMeta { key: PackKey::ToEnMovPtrs, kind: "words", elem: 2, count: 36, rom: "bank03:96DB", note: "EnemyMovementPtrs 18 索引 × R/L = 36 项 (72 B)" },
        SegMeta { key: PackKey::ToEnMovStr, kind: "bytes", elem: 1, count: 512, rom: "bank03:97EF", note: "移动指令串窗" },
        // ---- Ri（bank5）房流与敌表 ----
        SegMeta { key: PackKey::RiRoomPtrs, kind: "u16-table", elem: 2, count: 42, rom: "bank05:A17F", note: "RoomPtrTable" },
        SegMeta { key: PackKey::RiStructPtrs, kind: "u16-table", elem: 2, count: 29, rom: "bank05:A1D3", note: "StructPtrTable" },
        SegMeta { key: PackKey::RiMetatiles, kind: "bytes", elem: 4, count: 33, rom: "bank05:AB23", note: "元tile 4B 组×33" },
        SegMeta { key: PackKey::RiRoomStreams, kind: "bytes", elem: 1, count: 0, rom: "bank05:各指针", note: "房流拼接（各自界 $FF）" },
        SegMeta { key: PackKey::RiRoomOffs, kind: "offs-table", elem: 8, count: 42, rom: "(派生)", note: "×42 (off,len)" },
        SegMeta { key: PackKey::RiStructures, kind: "bytes", elem: 1, count: 0, rom: "bank05:各指针", note: "结构体拼接" },
        SegMeta { key: PackKey::RiStructOffs, kind: "offs-table", elem: 8, count: 29, rom: "(派生)", note: "×29 (off,len)" },
        SegMeta { key: PackKey::RiEnemyHealth, kind: "bytes", elem: 1, count: 64, rom: "bank05:962B", note: "EnemyHealthTbl（窗口；语义 16）" },
        SegMeta { key: PackKey::RiEnemyRestingAnim, kind: "bytes", elem: 1, count: 32, rom: "bank05:963B", note: "EnemyRestingAnimIndex" },
        SegMeta { key: PackKey::RiEnemyActiveAnim, kind: "bytes", elem: 1, count: 32, rom: "bank05:965B", note: "EnemyActiveAnimIndex" },
        SegMeta { key: PackKey::RiEnemyActiveAnimInitOffset, kind: "bytes", elem: 1, count: 16, rom: "bank05:967B", note: "EnemyActiveAnimIndexInitOffset" },
        SegMeta { key: PackKey::RiEnemyForceAttribs, kind: "bytes", elem: 1, count: 16, rom: "bank05:968B", note: "L968B 强制属性" },
        SegMeta { key: PackKey::RiEnemyForceSpeedDelay, kind: "bytes", elem: 1, count: 64, rom: "bank05:969B", note: "ForceSpeedTowardsSamusDelayTbl（窗口；语义 16）" },
        SegMeta { key: PackKey::RiEnemyDistanceThreshold, kind: "bytes", elem: 1, count: 16, rom: "bank05:96AB", note: "EnemyDistanceToSamusThreshold" },
        SegMeta { key: PackKey::RiEnemyInitDelay, kind: "bytes", elem: 1, count: 64, rom: "bank05:96BB", note: "EnemyInitDelayTbl（窗口；语义 32）" },
        SegMeta { key: PackKey::RiEnemyFacing, kind: "bytes", elem: 1, count: 64, rom: "bank05:977B", note: "L977B 朝向表" },
        SegMeta { key: PackKey::RiEnMovPtrs, kind: "words", elem: 2, count: 36, rom: "bank05:96DB", note: "EnemyMovementPtrs 18 索引 × R/L = 36 项 (72 B)" },
        SegMeta { key: PackKey::RiEnMovStr, kind: "bytes", elem: 1, count: 512, rom: "bank05:97EF", note: "移动指令串窗" },
        // ---- Norfair 敌表（bank02，同址布局）----
        SegMeta { key: PackKey::NoEnemyHealth, kind: "bytes", elem: 1, count: 64, rom: "bank02:962B", note: "EnemyHealthTbl（窗口；语义 16）" },
        SegMeta { key: PackKey::NoEnemyRestingAnim, kind: "bytes", elem: 1, count: 32, rom: "bank02:963B", note: "EnemyRestingAnimIndex" },
        SegMeta { key: PackKey::NoEnemyActiveAnim, kind: "bytes", elem: 1, count: 32, rom: "bank02:965B", note: "EnemyActiveAnimIndex" },
        SegMeta { key: PackKey::NoEnemyActiveAnimInitOffset, kind: "bytes", elem: 1, count: 16, rom: "bank02:967B", note: "EnemyActiveAnimIndexInitOffset" },
        SegMeta { key: PackKey::NoEnemyForceAttribs, kind: "bytes", elem: 1, count: 16, rom: "bank02:968B", note: "L968B 强制属性" },
        SegMeta { key: PackKey::NoEnemyForceSpeedDelay, kind: "bytes", elem: 1, count: 64, rom: "bank02:969B", note: "ForceSpeedTowardsSamusDelayTbl（窗口；语义 16）" },
        SegMeta { key: PackKey::NoEnemyDistanceThreshold, kind: "bytes", elem: 1, count: 16, rom: "bank02:96AB", note: "EnemyDistanceToSamusThreshold" },
        SegMeta { key: PackKey::NoEnemyInitDelay, kind: "bytes", elem: 1, count: 64, rom: "bank02:96BB", note: "EnemyInitDelayTbl（窗口；语义 32）" },
        SegMeta { key: PackKey::NoEnemyFacing, kind: "bytes", elem: 1, count: 64, rom: "bank02:977B", note: "L977B 朝向表" },
        SegMeta { key: PackKey::NoEnMovPtrs, kind: "words", elem: 2, count: 36, rom: "bank02:96DB", note: "EnemyMovementPtrs 18 索引 × R/L = 36 项 (72 B)" },
        SegMeta { key: PackKey::NoEnMovStr, kind: "bytes", elem: 1, count: 512, rom: "bank02:97EF", note: "移动指令串窗" },
        SegMeta { key: PackKey::RomTable968B, kind: "bytes", elem: 0x80, count: 8, rom: "各 bank $968B", note: "窗口参数化属性表 8×128B" },
        SegMeta { key: PackKey::RomVec95CE, kind: "bytes", elem: 2, count: 8, rom: "各 bank $95CE", note: "F2ED 流向量 [lo,hi]×8" },
        SegMeta { key: PackKey::NametableHi, kind: "bytes", elem: 1, count: 4, rom: "bank07:$C19F", note: "NT 高位表 20/24/28/2C" },
        SegMeta { key: PackKey::NoStructOffs, kind: "offs-table", elem: 8, count: 49, rom: "(派生)", note: "×49 (off,len)" },
        // ---- 音乐/精灵/调色板/声引擎区（W4b-e 各批的段此前缺 meta，v2 硬错后补齐）----
        SegMeta { key: PackKey::MusicNotes, kind: "pitch-table", elem: 1, count: 128, rom: "bank00:$BE77", note: "MusicNotesTbl 音高表（APU 周期 lo）" },
        SegMeta { key: PackKey::NoteLen4, kind: "bytes", elem: 1, count: 11, rom: "bank00:$BEF7", note: "NoteLengthsTbl 组0（曲头 Byte0=$00，11 项）" },
        SegMeta { key: PackKey::NoteLen6, kind: "bytes", elem: 1, count: 12, rom: "bank00:$BF02", note: "NoteLengthsTbl 组1（曲头 Byte0=$0B，12 项）" },
        SegMeta { key: PackKey::NoteLen7, kind: "bytes", elem: 1, count: 12, rom: "bank00:$BF0E", note: "NoteLengthsTbl 组2（曲头 Byte0=$17，12 项）" },
        SegMeta { key: PackKey::SongStreams, kind: "bytes", elem: 1, count: 0, rom: "bank00:$BD31 SongHeaders", note: "12 曲×4 通道指令流拼接（自界 $00 含尾）" },
        SegMeta { key: PackKey::SongStreamOffs, kind: "offs-table", elem: 8, count: 48, rom: "(派生)", note: "×48 (off,len) u32 LE" },
        SegMeta { key: PackKey::StreamBases, kind: "u16-table", elem: 2, count: 48, rom: "SongHeaders[5..13]", note: "48 条流基址（含历史 quirk：部分为音乐 RAM 地址 $0100/$0300/$0500/$0700）" },
        SegMeta { key: PackKey::EnvPtrs, kind: "u16-table", elem: 2, count: 5, rom: "bank00:$BCB0", note: "VolumeEnvelopePtrTable 5 word" },
        SegMeta { key: PackKey::EnvBody, kind: "bytes", elem: 1, count: 0, rom: "bank00:$BCBA 起", note: "5 条包络拼接（自界 $FF/$F0 含尾）" },
        SegMeta { key: PackKey::EnvOffs, kind: "offs-table", elem: 8, count: 5, rom: "(派生)", note: "×5 (off,len) u32 LE" },
        SegMeta { key: PackKey::SfxData, kind: "bytes", elem: 1, count: 16, rom: "bank00:$B200", note: "SFXData 16 B" },
        SegMeta { key: PackKey::PalPtrs, kind: "u16-table", elem: 2, count: 159, rom: "各 bank $9560", note: "PalettePtrTable 六 bank 拼接（19+28×5）" },
        SegMeta { key: PackKey::PalData, kind: "bytes", elem: 1, count: 0, rom: "各 bank VRAMStruct 记录流", note: "六 bank 记录流拼接（hi==0 终止含尾）" },
        SegMeta { key: PackKey::PalOffs, kind: "offs-table", elem: 8, count: 159, rom: "(派生)", note: "×159 (off,len) u32 LE" },
        SegMeta { key: PackKey::SpriteData, kind: "bytes", elem: 1, count: SPRITE_BLOB_LEN, rom: "bank01:$8572", note: "OBJ 精灵 blob（三表界 $0000/$0099/$016D）" },
        SegMeta { key: PackKey::SndRegion, kind: "bytes", elem: 1, count: SOUND_REGION_LEN, rom: "bank00:$B289", note: "声引擎区窗口 $B289..$BD09（区内偏移寻址）" },
        SegMeta { key: PackKey::SfxTables, kind: "bytes", elem: 1, count: SFX_TABLES_LEN, rom: "bank00:$B20D", note: "SFXData@GamePaused(4B) + 4B 记录窗 $B211..$B289，按绝对地址引用" },
        SegMeta { key: PackKey::SongHdrs, kind: "song-hdr", elem: 13, count: N_SONGS, rom: "bank00:$BD31", note: "SongHeaders 12×13B（流/偏移另有段）" },
        SegMeta { key: PackKey::BrItems, kind: "item-rec", elem: 6, count: 0, rom: "bank01:$A3D6", note: "SpecItmsTbl_BRINSTAR（Y 链→扁平 6B 记录）" },
        SegMeta { key: PackKey::VramIntro, kind: "vram-struct", elem: 1, count: 1066, rom: "bank00:$82F4", note: "INTRO_BG（hi==0 终止含尾）" },
        SegMeta { key: PackKey::VramStart, kind: "vram-struct", elem: 1, count: 20, rom: "bank00:$9984", note: "START_CONTINUE（hi==0 终止含尾）" },
        SegMeta { key: PackKey::VramPass, kind: "vram-struct", elem: 1, count: 36, rom: "bank00:$99E3", note: "PASSWORD_PLEASE（hi==0 终止含尾）" },
        SegMeta { key: PackKey::VramOver, kind: "vram-struct", elem: 1, count: 13, rom: "bank00:$93B9", note: "GAME_OVER（hi==0 终止含尾）" },

        SegMeta { key: PackKey::SamusActions, kind: "u8", elem: 1, count: 4, rom: "bank07:$CC87", note: "ActionTable（sa_Run/sa_Run/sa_Roll/sa_PntUp）" },
        SegMeta { key: PackKey::SamusPntUp, kind: "u8", elem: 1, count: 3, rom: "bank07:$D1EB", note: "PntUp 状态表（01 01 03）" },
        SegMeta { key: PackKey::SamusAccel, kind: "u8", elem: 1, count: 2, rom: "bank07:$CCC0", note: "RunAccelerationTbl（$30/$D0；旧 256B 烘焙吞代码已更正）" },        SegMeta { key: PackKey::NoItems, kind: "item-rec", elem: 6, count: 0, rom: "bank02:$A2D9", note: "SpecItmsTbl_NORFAIR" },

        SegMeta { key: PackKey::IntroStarPal, kind: "pal10", elem: 10, count: 8, rom: "bank00:$8B0F", note: "IntroStarPalette0..7（星闪；表尾 $8B5F=DoFadeOut）" },        SegMeta { key: PackKey::KrItems, kind: "item-rec", elem: 6, count: 0, rom: "bank04:$A26D", note: "SpecItmsTbl_KRAID" },

        SegMeta { key: PackKey::Fade, kind: "title-tbl", elem: 1, count: 12, rom: "bank00:$8B6D", note: "FadeOutPaletteData" },
        SegMeta { key: PackKey::PalSelect, kind: "title-tbl", elem: 1, count: 13, rom: "bank00:$8A9A", note: "PalSelect" },
        SegMeta { key: PackKey::StartCont, kind: "title-tbl", elem: 1, count: 2, rom: "bank00:$9118", note: "StartContTbl" },
        SegMeta { key: PackKey::EndSamus, kind: "offs+blob", elem: 1, count: 0, rom: "bank00:$9D5A->$9D66", note: "EndSamusAddrTbl 六指针 + 338 B 帧体" },
        SegMeta { key: PackKey::EndWave, kind: "bytes", elem: 1, count: END_WAVE_LEN, rom: "bank00:$9D1A", note: "挥手手型四表 $9D1A..$9D5A" },
        SegMeta { key: PackKey::EndStar, kind: "title-tbl", elem: 1, count: 156, rom: "bank00:$9EB8", note: "EndStar" },
        SegMeta { key: PackKey::EndBackground, kind: "vram-struct", elem: 1, count: 360, rom: "bank00:$A052", note: "EndBackground（hi==0 终止含尾）" },
        SegMeta { key: PackKey::EndPalettes, kind: "offs+blob", elem: 1, count: 0, rom: "bank00:$9F9B", note: "EndGamePalette00..0A（11×u16 + 183 B 结构体，长度不齐）" },
        SegMeta { key: PackKey::EndCredits, kind: "offs+blob", elem: 1, count: 0, rom: "bank00:$A2E9", note: "CreditsPtrTable 六页（6×u16 + 102 B 页体）" },
        SegMeta { key: PackKey::PwErr, kind: "title-tbl", elem: 1, count: 15, rom: "bank00:$8759", note: "PwErrMsg" },
        SegMeta { key: PackKey::PwRows, kind: "title-tbl", elem: 1, count: 10, rom: "bank00:$9998", note: "PasswordRowTbl 5×u16" },
        SegMeta { key: PackKey::PwGrid, kind: "title-tbl", elem: 1, count: 65, rom: "bank00:$99A2", note: "PasswordRow 网格 5×13" },
        SegMeta { key: PackKey::CursorX, kind: "title-tbl", elem: 1, count: 12, rom: "bank00:$92C8", note: "CursorX" },
        SegMeta { key: PackKey::CharselY, kind: "title-tbl", elem: 1, count: 5, rom: "bank00:$92B6", note: "CharselY" },
        SegMeta { key: PackKey::CharselX, kind: "title-tbl", elem: 1, count: 13, rom: "bank00:$92BB", note: "CharselX" },
        SegMeta { key: PackKey::SparkleTop, kind: "title-tbl", elem: 1, count: 54, rom: "bank00:$89B3", note: "TopSparkle 27 条" },
        SegMeta { key: PackKey::SparkleBottom, kind: "title-tbl", elem: 1, count: 98, rom: "bank00:$89E9", note: "BottomSparkle 49 条含终止（旧 54B 截断已更正）" },
        SegMeta { key: PackKey::SparkleInit, kind: "title-tbl", elem: 1, count: 11, rom: "bank00:$87C4", note: "InitSparkleDataTbl" },
        SegMeta { key: PackKey::ScreenFlashPal, kind: "title-tbl", elem: 1, count: 10, rom: "bank00:$8ABD", note: "ScreenFlashPal" },
        SegMeta { key: PackKey::InitCross, kind: "title-tbl", elem: 1, count: 64, rom: "bank00:$88BE", note: "InitCrossTbl" },
        SegMeta { key: PackKey::CrossLen, kind: "title-tbl", elem: 1, count: 5, rom: "bank00:$89AA", note: "CrossExplodeLengthTbl" },
        SegMeta { key: PackKey::CrossData, kind: "title-tbl", elem: 1, count: 68, rom: "bank00:$8A4B", note: "CrossExplodeDataTbl" },        SegMeta { key: PackKey::ToItems, kind: "item-rec", elem: 6, count: 0, rom: "bank03:$A83B", note: "SpecItmsTbl_TOURIAN" },
        SegMeta { key: PackKey::RiItems, kind: "item-rec", elem: 6, count: 0, rom: "bank05:$A20D", note: "SpecItmsTbl_RIDLEY" },
        SegMeta { key: PackKey::TourianMetroidData, kind: "bytes", elem: 1, count: 14, rom: "bank07:$C5A8",
                  note: "MetroidData（InitBank3 逐字节拷到 $77F0-$77FD）" },
        SegMeta { key: PackKey::AreaPalToggle, kind: "bytes", elem: 1, count: 5, rom: "bank1/2/3/4/5:$95DA",
                  note: "AreaPaletteToggle 5 区（Br/No=01，Kr/To/Ri=06）——MoreInit→PaletteToggle" },
        SegMeta { key: PackKey::AreaBrHeader, kind: "bytes", elem: 13, count: 1, rom: "bank01:$95CD", note: "Brinstar 区头（music/damage/itemrooms/mappos/samusY）" },
        SegMeta { key: PackKey::AreaNoHeader, kind: "bytes", elem: 13, count: 1, rom: "bank02:$95CD", note: "Norfair 区头" },
        SegMeta { key: PackKey::AreaKrHeader, kind: "bytes", elem: 13, count: 1, rom: "bank04:$95CD", note: "Kraid 区头" },
        SegMeta { key: PackKey::AreaToHeader, kind: "bytes", elem: 13, count: 1, rom: "bank03:$95CD", note: "Tourian 区头" },
        SegMeta { key: PackKey::AreaRiHeader, kind: "bytes", elem: 13, count: 1, rom: "bank05:$95CD", note: "Ridley 区头" },
        SegMeta { key: PackKey::AreaBrEnAux, kind: "bytes", elem: 1, count: 0, rom: "bank01:$96CB+$972B", note: "Brinstar 敌辅助表（16 B choice offset + 170 B 窗）" },
        SegMeta { key: PackKey::AreaNoEnAux, kind: "bytes", elem: 1, count: 0, rom: "bank02:$96CB+$972B", note: "Norfair 敌辅助表" },
        SegMeta { key: PackKey::AreaKrEnAux, kind: "bytes", elem: 1, count: 0, rom: "bank04:$96CB+$972B", note: "Kraid 敌辅助表" },
        SegMeta { key: PackKey::AreaToEnAux, kind: "bytes", elem: 1, count: 0, rom: "bank03:$96CB+$972B", note: "Tourian 敌辅助表" },
        SegMeta { key: PackKey::AreaRiEnAux, kind: "bytes", elem: 1, count: 0, rom: "bank05:$96CB+$972B", note: "Ridley 敌辅助表" },
        SegMeta { key: PackKey::AreaBrEnAnimIdx, kind: "bytes", elem: 1, count: 9, rom: "bank01:$95DC", note: "弹体/爆炸/梅洛姆动画索引" },
        SegMeta { key: PackKey::AreaNoEnAnimIdx, kind: "bytes", elem: 1, count: 9, rom: "bank02:$95DC", note: "弹体/爆炸/梅洛姆动画索引" },
        SegMeta { key: PackKey::AreaKrEnAnimIdx, kind: "bytes", elem: 1, count: 9, rom: "bank04:$95DC", note: "弹体/爆炸/梅洛姆动画索引" },
        SegMeta { key: PackKey::AreaToEnAnimIdx, kind: "bytes", elem: 1, count: 9, rom: "bank03:$95DC", note: "弹体/爆炸/梅洛姆动画索引" },
        SegMeta { key: PackKey::AreaRiEnAnimIdx, kind: "bytes", elem: 1, count: 9, rom: "bank05:$95DC", note: "弹体/爆炸/梅洛姆动画索引" },
        SegMeta { key: PackKey::AreaBrEnMovChoices, kind: "bytes", elem: 1, count: 30, rom: "bank01:$97D1", note: "移动索引候选集（表长 30）" },
        SegMeta { key: PackKey::AreaNoEnMovChoices, kind: "bytes", elem: 1, count: 30, rom: "bank02:$97D1", note: "移动索引候选集（表长 22）" },
        SegMeta { key: PackKey::AreaKrEnMovChoices, kind: "bytes", elem: 1, count: 30, rom: "bank04:$97D1", note: "移动索引候选集（表长 24）" },
        SegMeta { key: PackKey::AreaToEnMovChoices, kind: "bytes", elem: 1, count: 30, rom: "bank03:$97D1", note: "移动索引候选集（表长 4）" },
        SegMeta { key: PackKey::AreaRiEnMovChoices, kind: "bytes", elem: 1, count: 30, rom: "bank05:$97D1", note: "移动索引候选集（表长 28）" },
        SegMeta { key: PackKey::AreaNoSqueeptSpeedY, kind: "bytes", elem: 1, count: 4, rom: "bank02:$9AD2", note: "SqueeptSpeedYTable（跳起初速 F6 F8 F6 FA；asm 仅 prg2 include）" },
        SegMeta { key: PackKey::AreaBrEnProjMoveStr, kind: "bytes", elem: 1, count: 60, rom: "bank01:$9965", note: "敌弹移动串 M0-M3（fireball 04 B3… / polyp 09 C2…；$FF=尾）" },
        SegMeta { key: PackKey::AreaNoEnProjMoveStr, kind: "bytes", elem: 1, count: 60, rom: "bank02:$97F7", note: "敌弹移动串 M0-M3（fireball 0A D3…）" },
        SegMeta { key: PackKey::AreaToEnProjMoveStr, kind: "bytes", elem: 1, count: 7, rom: "bank03:$97D5", note: "敌弹移动串（M0/M1 同址别名；50 22 FF 50 30 FF FF）" },
        SegMeta { key: PackKey::AreaKrEnProjMoveStr, kind: "bytes", elem: 1, count: 60, rom: "bank04:$98C9", note: "敌弹移动串 M0-M3（同 Brinstar 字节）" },
        SegMeta { key: PackKey::AreaRiEnProjMoveStr, kind: "bytes", elem: 1, count: 45, rom: "bank05:$97FD", note: "敌弹移动串（M0 同址别名；polyp 三串）" },
        SegMeta { key: PackKey::AreaBrVramStrings, kind: "offs+blob", elem: 1, count: 0,
                  rom: "bank01:$97AF->串体", note: "Brinstar 瓦片爆 17 串（头 34B 偏移 + 串体）" },
        SegMeta { key: PackKey::AreaNoVramStrings, kind: "offs+blob", elem: 1, count: 0,
                  rom: "bank02:$97AF->串体", note: "Norfair 瓦片爆 17 串" },
        SegMeta { key: PackKey::AreaToVramStrings, kind: "offs+blob", elem: 1, count: 0,
                  rom: "bank03:$97AF->串体", note: "Tourian 瓦片爆 17 串" },
        SegMeta { key: PackKey::AreaKrVramStrings, kind: "offs+blob", elem: 1, count: 0,
                  rom: "bank04:$97AF->串体", note: "Kraid 瓦片爆 17 串" },
        SegMeta { key: PackKey::AreaRiVramStrings, kind: "offs+blob", elem: 1, count: 0,
                  rom: "bank05:$97AF->串体", note: "Ridley 瓦片爆 17 串" },
    ];
    v
}

/// 生成 meta JSON 文本（自带缩进，`pack-tool list` 原样打印即可读）。
pub fn build_meta_json(sections: &[Section], rom_crc32: u32, rom_size: u32)
    -> Result<Vec<u8>, String> {
    let metas = seg_meta();
    let mut out = String::with_capacity(4096);
    out.push_str("{\n");
    out.push_str("  \"format\": \"metroid-asset-pack\",\n");
    out.push_str(&format!("  \"version\": {VERSION},\n"));
    out.push_str(&format!(
        "  \"rom\": {{\"crc32\": \"{rom_crc32:08X}\", \"size\": {rom_size}}},\n"));
    out.push_str("  \"sections\": [\n");
    let mut first = true;
    for m in &metas {
        let Some(sec) = sections.iter().find(|s| s.key == m.key) else { continue };
        if !first {
            out.push_str(",\n");
        }
        first = false;
        let count = if m.count > 0 { m.count } else if m.elem > 1 { sec.data.len() / m.elem } else { sec.data.len() };
        out.push_str(&format!(
            "    {{\"key\": {}, \"name\": \"{}\", \"kind\": \"{}\", \"elem\": {}, \"count\": {}, \"bytes\": {}, \"crc32\": \"{:08X}\", \"rom\": \"{}\", \"note\": \"{}\"}}",
            m.key.id(), m.key.name(), m.kind, m.elem, count, sec.data.len(),
            crc32(&sec.data), m.rom, m.note
        ));
    }
    out.push_str("\n  ]\n}\n");
    // 缺 meta 的段不许静默跳过（自描述缺一段 = 漂移检测盲区，fail-loud）
    let missing: Vec<&str> = sections
        .iter()
        .filter(|s| s.key != PackKey::Meta && !metas.iter().any(|m| m.key == s.key))
        .map(|s| s.key.name())
        .collect();
    if !missing.is_empty() {
        return Err(format!("meta 缺段（须在 seg_meta 补条目）：{missing:?}"));
    }
    Ok(out.into_bytes())
}


// ---- 区域房流（Brinstar bank1 / Norfair bank2）------------------------------------
//
// 表长由土址机械给出（见两区 RoomPtrTable/StructPtrTable 标签地址）：
//   Brinstar: room_ptrs $A314(47 项) / struct_ptrs $A372(50 项) / metatiles $AEF0(260 B)
//   Norfair : room_ptrs $A21B(46 项) / struct_ptrs $A277(49 项) / metatiles $AEEC(248 B)
// 房流/结构体**自界**：从指针起读到 `$FF`（`DrawRoom`/`DrawStruct` 终止符）。
// 变长数组必须同时写 `(off u32, len u32)` 索引段——运行侧无表即无法定位第 N 项。

struct AreaSpec {
    bank: u8,
    room_ptrs: u16,
    n_rooms: usize,
    struct_ptrs: u16,
    n_structs: usize,
    metatiles: u16,
    n_metatiles: usize,
    keys: [PackKey; 7],
}

/// Brinstar（bank1）。
const AREA_BR: AreaSpec = AreaSpec {
    bank: 1, room_ptrs: 0xA314, n_rooms: 47, struct_ptrs: 0xA372, n_structs: 50,
    metatiles: 0xAEF0, n_metatiles: 260,
    keys: [PackKey::BrRoomPtrs, PackKey::BrStructPtrs, PackKey::BrMetatiles,
           PackKey::BrRoomStreams, PackKey::BrRoomOffs, PackKey::BrStructures, PackKey::BrStructOffs],
};

/// Norfair（bank2）。
const AREA_NO: AreaSpec = AreaSpec {
    bank: 2, room_ptrs: 0xA21B, n_rooms: 46, struct_ptrs: 0xA277, n_structs: 49,
    metatiles: 0xAEEC, n_metatiles: 248,
    keys: [PackKey::NoRoomPtrs, PackKey::NoStructPtrs, PackKey::NoMetatiles,
           PackKey::NoRoomStreams, PackKey::NoRoomOffs, PackKey::NoStructures, PackKey::NoStructOffs],
};


/// Kraid（bank4）——`AreaPointers_ROM` 四项（prg4_kraid.asm:93-99）：
/// room $A1D5/37 项（→$A21F）、struct $A21F/39 项（→$A26D=SpecItmsTbl）、
/// metatiles $AC32/164 B（asm 41 组 ×4）。
const AREA_KR: AreaSpec = AreaSpec {
    bank: 4, room_ptrs: 0xA1D5, n_rooms: 37, struct_ptrs: 0xA21F, n_structs: 39,
    metatiles: 0xAC32, n_metatiles: 164,
    keys: [PackKey::KrRoomPtrs, PackKey::KrStructPtrs, PackKey::KrMetatiles,
           PackKey::KrRoomStreams, PackKey::KrRoomOffs, PackKey::KrStructures, PackKey::KrStructOffs],
};

/// Tourian（bank3）——room $A7D1/21（→$A7FB）、struct $A7FB/32（→$A83B=SpecItmsTbl）、
/// metatiles $AE49/216 B（asm 54 组 ×4）。
const AREA_TO: AreaSpec = AreaSpec {
    bank: 3, room_ptrs: 0xA7D1, n_rooms: 21, struct_ptrs: 0xA7FB, n_structs: 32,
    metatiles: 0xAE49, n_metatiles: 216,
    keys: [PackKey::ToRoomPtrs, PackKey::ToStructPtrs, PackKey::ToMetatiles,
           PackKey::ToRoomStreams, PackKey::ToRoomOffs, PackKey::ToStructures, PackKey::ToStructOffs],
};

/// Ridley（bank5）——room $A17F/42（→$A1D3）、struct $A1D3/29（→$A20D=SpecItmsTbl）、
/// metatiles $AB23/132 B（asm 33 组 ×4）。
const AREA_RI: AreaSpec = AreaSpec {
    bank: 5, room_ptrs: 0xA17F, n_rooms: 42, struct_ptrs: 0xA1D3, n_structs: 29,
    metatiles: 0xAB23, n_metatiles: 132,
    keys: [PackKey::RiRoomPtrs, PackKey::RiStructPtrs, PackKey::RiMetatiles,
           PackKey::RiRoomStreams, PackKey::RiRoomOffs, PackKey::RiStructures, PackKey::RiStructOffs],
};

/// 读到 `$FF`（含）为止；找不到终止符 ⇒ 硬错（宁可拒绝出包也不静默截断）。
fn read_until_ff(prg: &[u8], start: usize) -> Result<Vec<u8>, String> {
    let tail = prg.get(start..).ok_or_else(|| format!("自界越界：起点 {start}"))?;
    let end = tail
        .iter()
        .position(|&b| b == 0xFF)
        .ok_or_else(|| format!("自界失配：{start:#06X} 起无 $FF 终止符"))?;
    Ok(tail[..=end].to_vec())
}

/// 指针表 → 拼流 + `(off,len)` 索引段。
fn extract_streams(
    prg: &[u8], bank: u8, ptr_addr: u16, n: usize,
) -> Result<(Vec<u8>, Vec<u8>), String> {
    let ptrs = bank_slice(prg, bank, ptr_addr, n * 2)?;
    let mut body = Vec::new();
    let mut offs = Vec::with_capacity(n * 8);
    for i in 0..n {
        let p = u16::from_le_bytes([ptrs[i * 2], ptrs[i * 2 + 1]]);
        let o = bank_source_offset(bank, p)?;
        let one = read_until_ff(prg, o)?;
        offs.extend_from_slice(&(body.len() as u32).to_le_bytes());
        offs.extend_from_slice(&(one.len() as u32).to_le_bytes());
        body.extend_from_slice(&one);
    }
    Ok((body, offs))
}

fn extract_area(prg: &[u8], a: &AreaSpec) -> Result<Vec<Section>, String> {
    let k = &a.keys;
    let (rooms, room_offs) = extract_streams(prg, a.bank, a.room_ptrs, a.n_rooms)?;
    let (structs, struct_offs) = extract_streams(prg, a.bank, a.struct_ptrs, a.n_structs)?;
    Ok(vec![
        Section { key: k[0], data: bank_slice(prg, a.bank, a.room_ptrs, a.n_rooms * 2)? },
        Section { key: k[1], data: bank_slice(prg, a.bank, a.struct_ptrs, a.n_structs * 2)? },
        Section { key: k[2], data: bank_slice(prg, a.bank, a.metatiles, a.n_metatiles)? },
        Section { key: k[3], data: rooms },
        Section { key: k[4], data: room_offs },
        Section { key: k[5], data: structs },
        Section { key: k[6], data: struct_offs },
    ])
}



/// 各区敌表（**地址五区同构**——`EnemyHealthTbl ; NN:962B` 等标签在 bank1-5 同址，
/// 长度按 asm 标签间距/字节数机械给出，见 `docs/findings/enemy-table-extent.md`）：
/// health 16（$962B→$963B）/ resting 32 / active 32 / init-offset 16 / force-attribs 16 /
/// force-speed 16（$969B→$96AB）/ distance 16 / init-delay 32（$96BB→$96DB）/
/// facing 64（$977B 起，窗口）/ mov-ptrs 72（36 项 × 2 B）/ mov-str 512。
const ENEMY_TABLE_LAYOUT: [(u16, usize); 12] = [
    // **表长 vs 窗口**（enemy-table-extent 疑点定谳）：asm 标签间距给的是**语义表长**
    // （health 16 / force-speed 16 / init-delay 32），但 `LoadEnemy` 用 `type & $3F`
    // （0..63）索引 health —— 原版读的是**连续 ROM**，越过 16 B 后读到的是紧随其后的
    // 敌人动画表。为复刻这一跨表连读，段按**窗口**取（64/64/64），与历史烘焙一致；
    // 语义长度写在 `docs/findings/enemy-table-extent.md`。
    (0x962B, 64), // EnemyHealthTbl（语义 16；窗口 64 覆盖 type&$3F 连读）
    (0x963B, 32), // EnemyRestingAnimIndex
    (0x965B, 32), // EnemyActiveAnimIndex
    (0x967B, 16), // EnemyActiveAnimIndexInitOffset
    (0x968B, 16), // L968B（强制属性；bank4 该标签跨 32 B=并入 force-speed）
    (0x969B, 64), // EnemyForceSpeedTowardsSamusDelayTbl（语义 16；窗口 64）
    (0x96AB, 16), // EnemyDistanceToSamusThreshold
    (0x96BB, 64), // EnemyInitDelayTbl（语义 32；窗口 64）
    (0x977B, 64), // L977B 朝向表（窗口）
    (0x96DB, 72), // EnemyMovementPtrs（18 索引 × R/L × 2 B = 36 项；idx36 起全 $0000）
    (0x97EF, 512), // 移动指令串窗
    (0x960B, 32), // EnemyDeathAnimIndex（**按区不同**：bank1/3 首字节 0x27/0x08）

];

/// 五区敌表段键（顺序与 [`ENEMY_TABLE_LAYOUT`] 一一对应）。
fn enemy_keys(area: usize) -> [PackKey; 12] {
    match area {
        0 => [PackKey::BrEnemyHealth, PackKey::BrEnemyRestingAnim, PackKey::BrEnemyActiveAnim,
              PackKey::BrEnemyActiveAnimInitOffset, PackKey::BrEnemyForceAttribs,
              PackKey::BrEnemyForceSpeedDelay, PackKey::BrEnemyDistanceThreshold,
              PackKey::BrEnemyInitDelay, PackKey::BrEnemyFacing, PackKey::BrEnMovPtrs,
              PackKey::BrEnMovStr, PackKey::BrEnemyDeathAnim],
        1 => [PackKey::NoEnemyHealth, PackKey::NoEnemyRestingAnim, PackKey::NoEnemyActiveAnim,
              PackKey::NoEnemyActiveAnimInitOffset, PackKey::NoEnemyForceAttribs,
              PackKey::NoEnemyForceSpeedDelay, PackKey::NoEnemyDistanceThreshold,
              PackKey::NoEnemyInitDelay, PackKey::NoEnemyFacing, PackKey::NoEnMovPtrs,
              PackKey::NoEnMovStr, PackKey::NoEnemyDeathAnim],
        2 => [PackKey::KrEnemyHealth, PackKey::KrEnemyRestingAnim, PackKey::KrEnemyActiveAnim,
              PackKey::KrEnemyActiveAnimInitOffset, PackKey::KrEnemyForceAttribs,
              PackKey::KrEnemyForceSpeedDelay, PackKey::KrEnemyDistanceThreshold,
              PackKey::KrEnemyInitDelay, PackKey::KrEnemyFacing, PackKey::KrEnMovPtrs,
              PackKey::KrEnMovStr, PackKey::KrEnemyDeathAnim],
        3 => [PackKey::ToEnemyHealth, PackKey::ToEnemyRestingAnim, PackKey::ToEnemyActiveAnim,
              PackKey::ToEnemyActiveAnimInitOffset, PackKey::ToEnemyForceAttribs,
              PackKey::ToEnemyForceSpeedDelay, PackKey::ToEnemyDistanceThreshold,
              PackKey::ToEnemyInitDelay, PackKey::ToEnemyFacing, PackKey::ToEnMovPtrs,
              PackKey::ToEnMovStr, PackKey::ToEnemyDeathAnim],
        _ => [PackKey::RiEnemyHealth, PackKey::RiEnemyRestingAnim, PackKey::RiEnemyActiveAnim,
              PackKey::RiEnemyActiveAnimInitOffset, PackKey::RiEnemyForceAttribs,
              PackKey::RiEnemyForceSpeedDelay, PackKey::RiEnemyDistanceThreshold,
              PackKey::RiEnemyInitDelay, PackKey::RiEnemyFacing, PackKey::RiEnMovPtrs,
              PackKey::RiEnMovStr, PackKey::RiEnemyDeathAnim],
    }
}

/// 五区敌表提取（bank = 1..5 顺序；段内容 = `bank_slice(bank, addr, len)`）。
fn extract_enemy_tables_all(prg: &[u8]) -> Result<Vec<Section>, String> {
    let mut out = Vec::with_capacity(60);
    // **区序 → bank**：0=Br(1) 1=No(2) 2=Kr(4) 3=To(3) 4=Ri(5)——即 `BankTable 07:CA30`
    // 的 `02 03 05 04 06` 减 1。**不能按 bank 数字升序**：升序会把 Kraid(bank4) 与
    // Tourian(bank3) 的表互换（`enemy_keys` 的区序是 InArea 序，不是 bank 序）。
    for (i, bank) in [1u8, 2, 4, 3, 5].into_iter().enumerate() {
        let keys = enemy_keys(i);
        for (j, (addr, len)) in ENEMY_TABLE_LAYOUT.iter().enumerate() {
            out.push(Section { key: keys[j], data: bank_slice(prg, bank, *addr, *len)? });
        }
    }
    Ok(out)
}

/// Brinstar 敌九表（`prg1_brinstar.asm` 连续标签——地址由标签机械给出）。
/// **表长口径**：健康/强制速度/初始延迟/朝向在 asm 里各 16 B（$962B→$963B 等相邻标签间距），
/// 本仓历史烘焙为 64 B（跨表连读）。搬家不改语义 ⇒ 仍按提交件的声明长度取，等长在
/// `docs/findings/enemy-table-extent.md`（待建）另案收敛。



/// `$968B` 属性表逐 bank（8×128B；源 `prg[w*0x4000 + 0x168B ..]`——
/// 窗口 CPU `$8000+off` 映射，见 `rom_tables.rs` 模块注的 decode-sweep r3 定谳）。
fn extract_rom_tables(prg: &[u8]) -> Result<Vec<Section>, String> {
    let mut tbl = Vec::with_capacity(8 * 0x80);
    let mut vecs = Vec::with_capacity(8 * 2);
    for w in 0..8usize {
        let base = w * 0x4000;
        let t = prg.get(base + 0x168B..base + 0x168B + 0x80)
            .ok_or_else(|| format!("$968B 表越界（bank{w}）"))?;
        tbl.extend_from_slice(t);
        let v = prg.get(base + 0x15CE..base + 0x15CE + 2)
            .ok_or_else(|| format!("$95CE 向量越界（bank{w}）"))?;
        vecs.extend_from_slice(v);
    }
    Ok(vec![
        Section { key: PackKey::RomTable968B, data: tbl },
        Section { key: PackKey::RomVec95CE, data: vecs },
        Section { key: PackKey::NametableHi, data: vec![0x20, 0x24, 0x28, 0x2C] },
    ])
}


/// sound 域平铺偏移：`文件偏移 = addr - $8000`（模块注自证 `$B31B↔prg[0x331B]`）。
const SOUND_FLAT_BASE: usize = 0x8000;

fn flat_slice(prg: &[u8], addr: u16, len: usize) -> Result<Vec<u8>, String> {
    let off = (addr as usize)
        .checked_sub(SOUND_FLAT_BASE)
        .ok_or_else(|| format!("sound 域越界：{addr:#06X} < $8000"))?;
    let s = prg
        .get(off..off + len)
        .ok_or_else(|| format!("sound 域越界：{addr:#06X}+{len}"))?;
    Ok(s.to_vec())
}

/// 音乐四表（`MusicNotesTbl ; $BE77` 64 音符×2B / `NoteLengthsTbl ; $BEF7` 三组 11+12+12）。
fn extract_music_tables(prg: &[u8]) -> Result<Vec<Section>, String> {
    Ok(vec![
        Section { key: PackKey::MusicNotes, data: flat_slice(prg, 0xBE77, 128)? },
        Section { key: PackKey::NoteLen4, data: flat_slice(prg, 0xBEF7, 11)? },
        Section { key: PackKey::NoteLen6, data: flat_slice(prg, 0xBEF7 + 11, 12)? },
        Section { key: PackKey::NoteLen7, data: flat_slice(prg, 0xBEF7 + 23, 12)? },
    ])
}


/// 乐流/包络（`SongHeaderOffsetTable ; $BBFA` 12B → `SongHeaders ; $BD31` 12×13B →
/// 头[5..13]=4 通道基址字 → 流=基址起至 `0x00`(SongEnd) 含尾；
/// `VolumeEnvelopePtrTable ; $BCB0` 5 word → 包络至 `$FF`/`$F0` 含尾）。
///
/// **历史 quirk（必须复刻，否则数据不等价）**：部分曲目的"基址"是音乐 RAM 地址
/// （`$0100/$0300/$0500/$0700`），`base - $8000` 为负 —— 旧烘焙器以 Python 负索引回绕
/// 取到 PRG 尾部数据（`prg[len+off]`）。这是自 `bake_music_streams.py` 起的既有行为，
/// 本函数按同一口径复刻；真值待独立卡考证（登记于 findings）。
const SONG_HDR_OFF_TBL: usize = 0x3BFA;
const SONG_HEADERS: usize = 0x3D31;
const ENV_PTRS_TBL: usize = 0x3CB0;
const N_SONGS: usize = 12;
const N_ENVS: usize = 5;
const SFX_DATA_OFF: usize = 0x3200;

fn snd_off(prg: &[u8], addr: u16) -> usize {
    let raw = addr as i64 - 0x8000;
    if raw < 0 {
        (prg.len() as i64 + raw) as usize
    } else {
        raw as usize
    }
}

/// 自 `addr` 起读到终止字节集合之一（含尾；上限 cap；越界即截断）。
fn read_until_any(prg: &[u8], addr: u16, terms: &[u8], cap: usize) -> Vec<u8> {
    let off = snd_off(prg, addr);
    let tail = prg.get(off..).unwrap_or(&[]);
    let mut out = Vec::new();
    for &b in tail.iter().take(cap) {
        out.push(b);
        if terms.contains(&b) {
            break;
        }
    }
    out
}

/// SFX 4B 记录窗 `$B211..$B289`（120 B）：`$B200` 起 `SFXData` 16 B 之后，三族记录表
/// （W2 `$B215`起 / W3 `$B211`起 / W4 `$B265`起）+ SamusHit `$B225` / Beep `$B279`。
/// 记录**按绝对地址引用**（`sfx_data_init` 的 src 参数即 ROM 址），故整窗入包，消费侧
/// 用 `addr - $B211` 索引（零散切片会让地址↔偏移换算散落各处）。
/// `$B20D..$B289`：`SFXData@GamePaused ; $B20D`（4 B，暂停音）**在前**，随后是
/// 4 B 记录窗 `$B211..$B289`。窗口自 $B20D 起，按绝对地址减 `SFX_TABLES_ADDR` 寻址。
const SFX_TABLES_FLAT: usize = 0x320D;
const SFX_TABLES_LEN: usize = 0x7c; // $B20D..$B289（GamePaused 4B + 记录窗 0x78）
const SONG_HEADERS_LEN: usize = N_SONGS * 13; // 156

/// 标题屏 VRAMStruct blob（`VRAMStructWriteData` 07:C2E4 的源；记录三态
/// `[hi,lo,ctrl,…]`、`hi==0` 终止、ctrl bit7=增量32/bit6=RLE/低 6 位计数(0⇒256)）。
/// 四态各自自界（含终止符），故按段独立（不再拼接，消费侧无需偏移表）。
/// 表址：`$82F4`/`$9984`/`$99E3`/`$93B9`——均 bank0 平铺。
const TITLE_VRAM: [(PackKey, u16, usize, &str); 5] = [
    (PackKey::VramIntro, 0x82F4, 1066, "INTRO_BG $82F4"),
    (PackKey::VramStart, 0x9984, 20, "START_CONTINUE $9984"),
    (PackKey::VramPass, 0x99E3, 36, "PASSWORD_PLEASE $99E3"),
    (PackKey::VramOver, 0x93B9, 13, "GAME_OVER $93B9"),
    // 结局背景 `VRAMStruct_EndBackground ; 00:A052`：17 条记录 + 终止符 = 360 B。
    (PackKey::EndBackground, 0xA052, 360, "END_BACKGROUND $A052"),
    // 结局调色板 11 张（`EndGamePalette00..0A ; 00:9F9B..A052`）：长度不齐 ⇒ 走
    // `extract_end_palettes` 的偏移表，故不进本表。
];

/// Samus 三张小表（bank7 固定窗 `$C000` ⇒ `addr` 直接过 `bank_slice(prg, 7, …)`）。
/// - `ActionTable ; 07:CC87` **4 B**（`.byte sa_Run/sa_Run/sa_Roll/sa_PntUp` = `01 01 03 04`）。
/// - PntUp 状态表 **3 B** @ `07:D1EB`（`01 01 03`；第 4 字节 `$A5` 已是
///   `FireSamusProjectile` 首码——表界由指令流互核）。
/// - `RunAccelerationTbl ; 07:CCC0` **2 B**（`.byte $30` 右 / `.byte $D0` 左）。
///   **历史缺陷更正**：旧 `RUN_ACCEL_TBL: [u8; 256]` 自 $CCC0 起吞了后续**代码**
///   （`$A6 $4D`=`ldx $4D` …）；消费侧只取 `[dir]`（dir∈{0,1}）故行为未受影响，
///   但把一个代码窗当数据表烘焙是错的，此处按 asm 真长 2 B 收口。
const SAMUS_TABLES: [(PackKey, u16, usize, &str); 3] = [
    (PackKey::SamusActions, 0xCC87, 4, "ActionTable $CC87"),
    (PackKey::SamusPntUp, 0xD1EB, 3, "PntUp 状态表 $D1EB"),
    (PackKey::SamusAccel, 0xCCC0, 2, "RunAccelerationTbl $CCC0"),
];

/// `MetroidData: ; 07:C5A8`（prg7_engine.asm:1466-1471）：14 B，`InitBank3` 尾
/// `ldy #$0D / lda MetroidData,y / sta MetroidRepelSpeed,y / dey / bpl` 逐字节拷到
/// `$77F0-$77FD`（MetroidRepelSpeed/MetroidAccel/MetroidMaxSpeed/MetroidLatch0400-0450）。
/// bank7 窗口（`$C000` 固定段）⇒ `bank_slice(prg, 7, …)`。
const TOURIAN_METROID_DATA_ADDR: u16 = 0xC5A8;
const TOURIAN_METROID_DATA_LEN: usize = 14;

/// `IntroStarPalette0..7 ; 00:8B0F`（8×10B 连续，`StarPaletteSwitch 00:8AC7` 经
/// `IntroStarPalettePtrTable 00:8AFF` 间接取；表尾 `00:8B5F` 即 `DoFadeOut`）。
const INTRO_STAR_PAL_ADDR: u16 = 0x8B0F;
const INTRO_STAR_PAL_LEN: usize = 8 * 10;

/// 标题屏 17 张小表（bank0 平铺；`(key, 表址, 真长, asm 标签)`）。
///
/// 两处**烘焙漂移更正**（旧常量与 ROM 不符，本次以 ROM/asm 为真）：
/// - `BOTTOM_SPARKLE_DATA`：asm `BottomSparkleDataTbl ; 00:89E9` 共 **49 条**
///   （`SignMagSpeed` 2 B/条 + 若干 `.byte $FF, -N` 直写），末尾 `SignMagSpeed $00,0,0`
///   即终止项 ⇒ 真长 **98 B**。旧常量只 54 B 且尾部塞 `00 00 00 00`，等于在表中间
///   插了个假终止符（下标 50 处 `b1==0` ⇒ 提前 complete）。
/// - `PW_ROW_TBL` 是 `.word PasswordRow0..4`（`PasswordRowTbl ; 00:9998` 10 B）；
///   旧常量以 `u16` 数组保存同一组值，字节序一致。
const TITLE_TABLES: [(PackKey, u16, usize, &str); 17] = [
    (PackKey::Fade, 0x8B6D, 12, "FadeOutPaletteData $8B6D"),
    (PackKey::PalSelect, 0x8A9A, 13, "PalSelect $8A9A"),
    (PackKey::StartCont, 0x9118, 2, "StartContTbl $9118"),
    (PackKey::EndStar, 0x9EB8, 156, "EndStar $9EB8"),
    (PackKey::PwErr, 0x8759, 15, "PwErrMsg $8759"),
    (PackKey::PwRows, 0x9998, 10, "PasswordRowTbl $9998（5×u16）"),
    (PackKey::PwGrid, 0x99A2, 65, "PasswordRow0..（5×13 B 网格）"),
    (PackKey::CursorX, 0x92C8, 12, "CursorX $92C8"),
    (PackKey::CharselY, 0x92B6, 5, "CharselY $92B6"),
    (PackKey::CharselX, 0x92BB, 13, "CharselX $92BB"),
    (PackKey::SparkleTop, 0x89B3, 54, "TopSparkleDataTbl $89B3（27 条）"),
    (PackKey::SparkleBottom, 0x89E9, 98, "BottomSparkleDataTbl $89E9（49 条含终止）"),
    (PackKey::SparkleInit, 0x87C4, 11, "InitSparkleDataTbl $87C4"),
    (PackKey::ScreenFlashPal, 0x8ABD, 10, "ScreenFlashPal $8ABD"),
    (PackKey::InitCross, 0x88BE, 64, "InitCrossTbl $88BE"),
    (PackKey::CrossLen, 0x89AA, 5, "CrossExplodeLengthTbl $89AA"),
    (PackKey::CrossData, 0x8A4B, 68, "CrossExplodeDataTbl $8A4B"),
];

fn extract_title_tables(prg: &[u8]) -> Result<Vec<Section>, String> {
    let mut out = Vec::with_capacity(TITLE_TABLES.len());
    for (key, addr, len, label) in TITLE_TABLES {
        out.push(Section { key, data: bank_slice(prg, 0, addr, len).map_err(|e| format!("{label}: {e}"))? });
    }
    Ok(out)
}

fn extract_intro_star_palettes(prg: &[u8]) -> Result<Vec<Section>, String> {
    Ok(vec![Section {
        key: PackKey::IntroStarPal,
        data: bank_slice(prg, 0, INTRO_STAR_PAL_ADDR, INTRO_STAR_PAL_LEN)?,
    }])
}

fn extract_samus_tables(prg: &[u8]) -> Result<Vec<Section>, String> {
    let mut out = Vec::with_capacity(SAMUS_TABLES.len());
    for (key, addr, len, label) in SAMUS_TABLES {
        out.push(Section { key, data: bank_slice(prg, 7, addr, len).map_err(|e| format!("{label}: {e}"))? });
    }
    Ok(out)
}

/// 结局职员表：`CreditsPtrTable ; 00:A291` 六指针（`$A2E9/$A2FB/$A31A/$A31B/$A32D/$A339`）
/// ⇒ 页体 `$A2E9..$A34F` = **102 B**（页 2 只有终止符 `00`，长度 1 B）。段布局 =
/// 6×u16 相对偏移 + 页体；每页都以 `hi==0` 自界（提取时逐页核，防越界）。
const CREDITS_BASE: u16 = 0xA2E9;
const CREDITS_PAGES: [u16; 6] = [0xA2E9, 0xA2FB, 0xA31A, 0xA31B, 0xA32D, 0xA339];
const CREDITS_END: u16 = 0xA34F;

/// 结局调色板 11 张（`EndGamePalette00..0A ; 00:9F9B..A052`）。**长度不齐**（每张是
/// `$3F00` 写结构，记录数不同：36/12/12/12/…/27/21 B），故按「逐张 hi==0 扫描」求偏移，
/// 段布局 = 11×u16 相对偏移 + 183 B 页体；消费侧按 `PaletteDataPending-1` 取第 N 张。
const END_PAL_BASE: u16 = 0x9F9B;
const END_PAL_END: u16 = 0xA052;
const END_PAL_COUNT: usize = 11;

fn extract_end_palettes(prg: &[u8]) -> Result<Section, String> {
    let mut offs: Vec<u16> = Vec::new();
    let mut p = END_PAL_BASE;
    while p < END_PAL_END {
        offs.push(p - END_PAL_BASE);
        loop {
            let hi = bank_slice(prg, 0, p, 1)?[0];
            if hi == 0 { p += 1; break; }
            let ctrl = bank_slice(prg, 0, p + 2, 1)?[0];
            let n = if ctrl & 0x3F == 0 { 256u16 } else { (ctrl & 0x3F) as u16 };
            let step = 3 + if ctrl & 0x40 != 0 { 1 } else { n };
            p = p.checked_add(step).ok_or("结局调色板越界")?;
            if p > END_PAL_END { return Err("结局调色板结构越界".into()); }
        }
    }
    if offs.len() != END_PAL_COUNT {
        return Err(format!("结局调色板张数 {} ≠ {}", offs.len(), END_PAL_COUNT));
    }
    let total = (END_PAL_END - END_PAL_BASE) as usize;
    let body = bank_slice(prg, 0, END_PAL_BASE, total)?;
    let mut data = Vec::with_capacity(offs.len() * 2 + body.len());
    for o in offs { data.extend_from_slice(&o.to_le_bytes()); }
    data.extend_from_slice(&body);
    Ok(Section { key: PackKey::EndPalettes, data })
}

/// 结局 Samus 六帧精灵：`EndSamusAddrTbl ; 00:9D5A` 六指针（`$9D66/$9D9C/$9DD2/$9E08/$9E3E/$9E74`）
/// ⇒ 帧体 `$9D66..$9EB8` = **338 B**（每帧 54 B，8 组 y/pattern + 终止）。段布局 =
/// 6×u16 相对偏移 + 帧体；每帧自身以 `$FF` 界（提取时只核区间端点，不重解语义）。
const END_SAMUS_TBL: u16 = 0x9D5A;
const END_SAMUS_FRAMES: [u16; 6] = [0x9D66, 0x9D9C, 0x9DD2, 0x9E08, 0x9E3E, 0x9E74];
const END_SAMUS_END: u16 = 0x9EB8;

fn extract_end_samus(prg: &[u8]) -> Result<Section, String> {
    let mut data = Vec::new();
    for &f in END_SAMUS_FRAMES.iter() {
        data.extend_from_slice(&(f - END_SAMUS_FRAMES[0]).to_le_bytes());
    }
    data.extend_from_slice(&bank_slice(prg, 0, END_SAMUS_FRAMES[0], (END_SAMUS_END - END_SAMUS_FRAMES[0]) as usize)?);
    // 表本身必须是六条指针且与帧体一致（fail-loud）。
    for (i, &f) in END_SAMUS_FRAMES.iter().enumerate() {
        let raw = bank_slice(prg, 0, END_SAMUS_TBL + (i as u16) * 2, 2)?;
        let ptr = u16::from_le_bytes([raw[0], raw[1]]);
        if ptr != f {
            return Err(format!("EndSamusAddrTbl[{i}] = {ptr:04X} ≠ 期望 {f:04X}"));
        }
    }
    Ok(Section { key: PackKey::EndSamus, data })
}

/// 结局挥手手型四表：`$9D1A..$9D5A` = 64 B（每表 4 精灵 × 4 B）。
const END_WAVE_ADDR: u16 = 0x9D1A;
const END_WAVE_LEN: usize = 0x40;

fn extract_end_wave(prg: &[u8]) -> Result<Section, String> {
    Ok(Section { key: PackKey::EndWave, data: bank_slice(prg, 0, END_WAVE_ADDR, END_WAVE_LEN)? })
}

fn extract_end_credits(prg: &[u8]) -> Result<Section, String> {
    let mut offs: Vec<u16> = Vec::with_capacity(CREDITS_PAGES.len());
    let mut blob: Vec<u8> = Vec::new();
    for (i, &page) in CREDITS_PAGES.iter().enumerate() {
        offs.push((page - CREDITS_BASE) + (page - CREDITS_BASE != 0 && false) as u16); // 占位（下行重算）
        offs[i] = page - CREDITS_BASE;
        // 逐页自界：扫到 hi==0（含尾）。
        let mut p = page;
        let mut len = 0usize;
        loop {
            let hi = bank_slice(prg, 0, p, 1)?[0];
            if hi == 0 {
                len += 1;
                break;
            }
            let ctrl = bank_slice(prg, 0, p + 2, 1)?[0];
            let n = if ctrl & 0x3F == 0 { 256usize } else { (ctrl & 0x3F) as usize };
            let step = 3 + if ctrl & 0x40 != 0 { 1 } else { n };
            p = p
                .checked_add(step as u16)
                .ok_or_else(|| format!("职员表页 {i} 越界"))?;
            len += step;
            if len > 4096 {
                return Err(format!("职员表页 {i} 未自界（>4096 B）"));
            }
        }
        let want = if i + 1 < CREDITS_PAGES.len() {
            (CREDITS_PAGES[i + 1] - page) as usize
        } else {
            (CREDITS_END - page) as usize
        };
        if len != want {
            return Err(format!("职员表页 {i} 长度 {len} ≠ 指针区间 {want}"));
        }
    }
    let total = (CREDITS_END - CREDITS_BASE) as usize;
    let page_data = bank_slice(prg, 0, CREDITS_BASE, total)?;
    blob.extend_from_slice(&page_data);
    let mut data = Vec::with_capacity(offs.len() * 2 + blob.len());
    for o in offs {
        data.extend_from_slice(&o.to_le_bytes());
    }
    data.extend_from_slice(&blob);
    Ok(Section { key: PackKey::EndCredits, data })
}

fn extract_title_vram(prg: &[u8]) -> Result<Vec<Section>, String> {
    let mut out = Vec::with_capacity(TITLE_VRAM.len());
    for (key, addr, len, label) in TITLE_VRAM {
        let data = bank_slice(prg, 0, addr, len)?;
        if data.last() != Some(&0x00) && data.last() != Some(&0xFF) {
            // VRAMStruct 以 hi==0 终止（含尾）；末字节非 0 说明长度切错（fail-loud）。
            return Err(format!("{label} 末字节非终止：{:02X}", data.last().copied().unwrap_or(0)));
        }
        out.push(Section { key, data });
    }
    out.push(extract_end_palettes(prg)?);
    out.push(extract_end_credits(prg)?);
    out.push(extract_end_samus(prg)?);
    out.push(extract_end_wave(prg)?);
    Ok(out)
}

/// `MetroidData ; 07:C5A8`（14 B）——`InitBank3` 初始化 Tourian 声/物理参数。
fn extract_tourian_metroid_data(prg: &[u8]) -> Result<Vec<Section>, String> {
    Ok(vec![Section {
        key: PackKey::TourianMetroidData,
        data: bank_slice(prg, 7, TOURIAN_METROID_DATA_ADDR, TOURIAN_METROID_DATA_LEN)
            .map_err(|e| format!("MetroidData $C5A8: {e}"))?,
    }])
}

/// `AreaPaletteToggle`（各 area bank `$95DA`，1 B/区）——`MoreInit 07:C81D` 读它写入
/// `PaletteToggle($0076)`；电梯换区 `PaletteToggle eor 7` 后再进 `PaletteDataPending`。
/// **取值（ROM 逐字节自证）**：bank1/2=`01`（Palette00+1）、bank3/4/5=`06`（Palette05+1）。
/// area 序 0=Br/1=No/2=Kr/3=To/4=Ri ⇒ ROM bank [1,2,4,3,5]（`BankTable 07:CA30`）。
/// `BankTable ; 07:CA30`（bank7 固定窗）——5 个区各自的 bank **+1**（ROM 字节
/// `02 03 05 04 06`；`CheckBankSwitch ; 07:C4DE` 用它做 `SwitchBank` 参数）。
pub const BANK_TABLE_ADDR: u16 = 0xCA30;

/// 区序（0=Br/1=No/2=Kr/3=To/4=Ri）→ ROM bank，**从 ROM 现算**（不手抄）：
/// `AREA_BANKS[i] = BankTable[i] - 1`。
///
/// 为什么必须现算：`AreaPaletteToggle` 的 bank3(Tourian) 与 bank4(Kraid) 值同为 `$06`
/// ⇒ 表驱动顺序写反在运行时段值与所有消费者**都看不出来**（r9 复核 P1-③）。
/// 手抄的常量只能钉住「常量与注释一致」；从 ROM 读 `BankTable` 才钉住「与 ROM 同源」。
pub fn area_banks(prg: &[u8]) -> Result<[u8; 5], String> {
    let raw = bank_slice(prg, 7, BANK_TABLE_ADDR, 5)
        .map_err(|e| format!("BankTable 07:CA30: {e}"))?;
    let mut out = [0u8; 5];
    for (i, b) in raw.iter().enumerate() {
        if *b == 0 {
            return Err(format!("BankTable[{i}] = 0（应 = bank+1）"));
        }
        out[i] = b - 1;
    }
    Ok(out)
}

/// 瓦片爆的 VRAM 字符串（**按区**）——`VRAMStringPtrTable ; 各 bank $97AF`（17 项 u16）
/// 指向的串体，`DrawTileBlast ; 07:FEDC` 经 `GetVRAMStringPtr ; 07:FECC` 取用。
///
/// 段布局（每区一段）：`17 x u16`（相对**串体起点**的偏移，LE）+ 串体。
/// 串体界 = `ptr[0]` 到 `ptr[16] + 1 + height*width`（头字节 `%hhhhwwww`）。
/// 提取侧三条机械自检：①指针单调不减 ②每串头字节高/低半字节非 0
/// ③相邻串必须首尾相接（asm 里就是顺序排布，错位即整块图案错）。
const VRAM_STRING_PTR_ADDR: u16 = 0x97AF;
const VRAM_STRING_COUNT: usize = 17;

fn extract_area_vram_strings(prg: &[u8]) -> Result<Vec<Section>, String> {
    let banks = area_banks(prg)?;
    let keys = [
        PackKey::AreaBrVramStrings,
        PackKey::AreaNoVramStrings,
        PackKey::AreaToVramStrings,
        PackKey::AreaKrVramStrings,
        PackKey::AreaRiVramStrings,
    ];
    let mut out = Vec::with_capacity(5);
    for (area, bank) in banks.iter().enumerate() {
        let tbl = bank_slice(prg, *bank, VRAM_STRING_PTR_ADDR, VRAM_STRING_COUNT * 2)
            .map_err(|e| format!("VRAMStringPtrTable area{area} bank{bank}: {e}"))?;
        let mut ptrs = [0u16; VRAM_STRING_COUNT];
        for (i, p) in ptrs.iter_mut().enumerate() {
            *p = u16::from_le_bytes([tbl[i * 2], tbl[i * 2 + 1]]);
        }
        let base = ptrs[0];
        let mut rels = [0u16; VRAM_STRING_COUNT];
        let mut body: Vec<u8> = Vec::new();
        let mut aliased = false;
        for i in 0..VRAM_STRING_COUNT {
            let p = ptrs[i];
            if aliased {
                // 表尾的别名项（原版把越界 id 指到字符串区之后，运行时读到别表）——
                // 记成零长（相对偏移 = 串体长），`assets::vram_string` 返回空切片。
                rels[i] = body.len() as u16;
                continue;
            }
            let hdr = bank_slice(prg, *bank, p, 1)
                .map_err(|e| format!("VRAMString area{area} id{i} 头字节 ${p:04X}: {e}"))?[0];
            let (h, w) = ((hdr >> 4) as usize, (hdr & 0x0F) as usize);
            if h == 0 || w == 0 {
                // 别名起点：此后所有项都必须指向同一地址（表尾同址别名），否则是坏数据。
                for q in ptrs.iter().skip(i) {
                    if *q != p {
                        return Err(format!(
                            "VRAMString area{area} id{i} 起为别名（${p:04X}）但后续指针不同（${q:04X}）"
                        ));
                    }
                }
                aliased = true;
                rels[i] = body.len() as u16;
                continue;
            }
            if i > 0 {
                let prev_end = base as usize + body.len();
                if p as usize != prev_end {
                    return Err(format!(
                        "VRAMString area{area} id{i} ${p:04X} 与前串末尾（${prev_end:04X}）不连续"
                    ));
                }
            }
            rels[i] = (p - base) as u16;
            let size = 1 + h * w;
            let bytes = bank_slice(prg, *bank, p, size)
                .map_err(|e| format!("VRAMString area{area} id{i} 体 ${p:04X}+{size}: {e}"))?;
            body.extend_from_slice(&bytes);
        }
        let mut data = Vec::with_capacity(VRAM_STRING_COUNT * 2 + body.len());
        for r in rels.iter() {
            data.extend_from_slice(&r.to_le_bytes());
        }
        data.extend_from_slice(&body);
        out.push(Section { key: keys[area], data });
    }
    Ok(out)
}

fn extract_area_pal_toggle(prg: &[u8]) -> Result<Vec<Section>, String> {
    const ADDR: u16 = 0x95DA;
    let banks = area_banks(prg)?;
    let mut body = Vec::with_capacity(5);
    for (area, bank) in banks.iter().enumerate() {
        let b = bank_slice(prg, *bank, ADDR, 1)
            .map_err(|e| format!("AreaPaletteToggle area{area} bank{bank} $95DA: {e}"))?;
        body.push(b[0]);
    }
    Ok(vec![Section { key: PackKey::AreaPalToggle, data: body }])
}

/// `CrawlerAnimIndexTable_{AREA}`（8 B，`Y = orientation*2 + direction` 索引）。
///
/// **值随区不同**：每一项是 `EnAnim_X - EnAnimTable_{AREA}`，而各区 `EnAnimTable` 基址/内容
/// 不同（bank1 `9D6A` / bank2 `9BDA` / bank4 `9C86` / bank5 `9B85`）——同一"墙上爬"语义
/// 在五区拿到的是不同帧号。旧形把 bank1（Zoomer）的 `35 35 3E 38 3B 3B 38 3E` 写死在
/// `crawler.rs` ⇒ 非 Brinstar 的 Crawler 重取向写错 `animIndex/resetAnimIndex`（实测
/// Norfair room 42 f120：emu `$72/$74` vs rust `$3E/$40`，OAM 8 项全错、87 px）。
///
/// **Tourian 无段**：`prg3_tourian.asm` 的 AI 表没有 Crawler 类型（该区 Crawler 不可达），
/// 不为它编造数据；消费侧按 `Option` fail-loud。
fn extract_crawler_anim_tables(prg: &[u8]) -> Result<Vec<Section>, String> {
    // (area 序 = InArea&$0F, bank, 表址)——地址取自各区 `enemy_sprite_data.asm` 标签。
    const SPEC: [(usize, u8, u16, PackKey); 4] = [
        (0, 1, 0x9AA0, PackKey::BrCrawlerAnim),
        (1, 2, 0x99ED, PackKey::NoCrawlerAnim),
        (2, 4, 0x99EF, PackKey::KrCrawlerAnim),
        (4, 5, 0x99BE, PackKey::RiCrawlerAnim),
    ];
    let banks = area_banks(prg)?;
    let mut out = Vec::with_capacity(SPEC.len());
    for (area, bank, addr, key) in SPEC {
        // 机械自检：区→bank 映射必须与 `BankTable 07:CA30` 推导一致（防手抄错区）。
        if banks[area] != bank {
            return Err(format!(
                "CrawlerAnimTable area{area}: bank 期望 {bank}，BankTable 给 {}",
                banks[area]
            ));
        }
        let data = bank_slice(prg, bank, addr, 8)
            .map_err(|e| format!("CrawlerAnimTable area{area} bank{bank} ${addr:04X}: {e}"))?;
        out.push(Section { key, data });
    }
    Ok(out)
}

/// 区头参数窗 `$95CD-$95D9`（13 B）——**消费点全在 bank7**（固定 bank 读当前映射区）：
/// - `$95CD AreaMusicFlag` → `StartMusic 07:D93A` 的 music flag
/// - `$95CE AreaEnemyDamage`（word）→ `SamusHurt 07:F306` 写 `HealthChange($6E)`
/// - `$95D0 AreaItemRoomNumbers`（7 B）→ `GetRoomNum 07:E758` 物品房音乐判据
/// - `$95D7/$95D8 AreaMapPosX/Y` → `MoreInit 07:C81D` 写 `MapPosX/Y`
/// - `$95D9 AreaSamusY` → `SamusInit 07:C907` 写 `Samus.y`
/// 13 B 到 `$95DA AreaPaletteToggle` 为止恰好连续（不含后者——它已单段）。
pub const AREA_HEADER_ADDR: u16 = 0x95CD;
pub const AREA_HEADER_LEN: usize = 13;

fn area_header_keys(area: usize) -> PackKey {
    match area {
        0 => PackKey::AreaBrHeader,
        1 => PackKey::AreaNoHeader,
        2 => PackKey::AreaKrHeader,
        3 => PackKey::AreaToHeader,
        4 => PackKey::AreaRiHeader,
        _ => unreachable!("area 序只 0..=4"),
    }
}

fn extract_area_headers(prg: &[u8]) -> Result<Vec<Section>, String> {
    let banks = area_banks(prg)?;
    let mut out = Vec::with_capacity(5);
    for (area, bank) in banks.iter().enumerate() {
        let b = bank_slice(prg, *bank, AREA_HEADER_ADDR, AREA_HEADER_LEN)
            .map_err(|e| format!("区头 area{area} bank{bank} $95CD: {e}"))?;
        out.push(Section { key: area_header_keys(area), data: b.to_vec() });
    }
    Ok(out)
}

/// 敌辅助表窗：`$96CB` 16 B（`EnemyMovementChoiceOffset`）+ `$972B-$97D5` 170 B
/// （`EnAccelY/XTable`、`EnSpeedY/XTable`、`L977B`、弹道动画/偏移/指针、`VRAMStringPtrTable`、
/// `EnemyMovementChoices`）。两块**不连续**（中间 $96DB-$972A 是 movPtrs/movStr，已有独立段），
/// 因此拼接成 182 B：前 16 B = choice offset，其后 = `$972B-$97D0` 的连续窗。
/// （`$97D1` 起的 `EnemyMovementChoices` 另有独立段 `area.{x}.en_mov_choices`，不重复。）
pub const EN_AUX_CHOICE_ADDR: u16 = 0x96CB;
pub const EN_AUX_CHOICE_LEN: usize = 16;
pub const EN_AUX_BODY_ADDR: u16 = 0x972B;
pub const EN_AUX_BODY_LEN: usize = 166;
pub const EN_AUX_LEN: usize = EN_AUX_CHOICE_LEN + EN_AUX_BODY_LEN;

fn en_aux_keys(area: usize) -> PackKey {
    match area {
        0 => PackKey::AreaBrEnAux,
        1 => PackKey::AreaNoEnAux,
        2 => PackKey::AreaKrEnAux,
        3 => PackKey::AreaToEnAux,
        4 => PackKey::AreaRiEnAux,
        _ => unreachable!("area 序只 0..=4"),
    }
}

fn extract_en_aux(prg: &[u8]) -> Result<Vec<Section>, String> {
    let banks = area_banks(prg)?;
    let mut out = Vec::with_capacity(5);
    for (area, bank) in banks.iter().enumerate() {
        let mut body = en_slice_b(prg, *bank, EN_AUX_CHOICE_ADDR, EN_AUX_CHOICE_LEN)
            .map_err(|e| format!("敌辅助表 area{area} bank{bank} $96CB: {e}"))?;
        body.extend(
            en_slice_b(prg, *bank, EN_AUX_BODY_ADDR, EN_AUX_BODY_LEN)
                .map_err(|e| format!("敌辅助表 area{area} bank{bank} $972B: {e}"))?,
        );
        if body.len() != EN_AUX_LEN {
            return Err(format!("敌辅助表 area{area} 长度 {}≠{EN_AUX_LEN}", body.len()));
        }
        out.push(Section { key: en_aux_keys(area), data: body });
    }
    Ok(out)
}

/// 每区 `$95DC-$95E4`（9 B）：`killed(1) explosion(1) pad(2) falling(2) splatter(2) mellow(1)`。
/// asm 依据 `prg1_brinstar.asm:142-152`（`{AREA}` 宏五区实例化，五区同址）。
const EN_ANIM_IDX_ADDR: u16 = 0x95DC;
const EN_ANIM_IDX_LEN: usize = 9;

fn en_anim_idx_keys(area: usize) -> PackKey {
    match area {
        0 => PackKey::AreaBrEnAnimIdx,
        1 => PackKey::AreaNoEnAnimIdx,
        2 => PackKey::AreaKrEnAnimIdx,
        3 => PackKey::AreaToEnAnimIdx,
        _ => PackKey::AreaRiEnAnimIdx,
    }
}

fn extract_en_anim_idx(prg: &[u8]) -> Result<Vec<Section>, String> {
    let mut out = Vec::with_capacity(5);
    let banks = area_banks(prg)?;
    for (area, &bank) in banks.iter().enumerate() {
        let body = bank_slice(prg, bank, EN_ANIM_IDX_ADDR, EN_ANIM_IDX_LEN)?;
        if body.len() != EN_ANIM_IDX_LEN {
            return Err(format!("bank{bank}:$95DC 段长 {} != 9", body.len()));
        }
        out.push(Section { key: en_anim_idx_keys(area), data: body.to_vec() });
    }
    Ok(out)
}

/// 每区 `$97D1-$97EE`（30 B）：`EnemyMovementChoices`。
/// 表长按 asm 标签机械自证（每条记录 `1 + mask + 1` 字节）：
/// Br 30（$97EF 起是移动串）/ No 22（$97E7）/ Kr 24（$97E9）/ To 4（$97D5）/ Ri 28（$97ED）。
const EN_MOV_CHOICES_ADDR: u16 = 0x97D1;
const EN_MOV_CHOICES_LEN: usize = 30;

fn en_mov_choices_keys(area: usize) -> PackKey {
    match area {
        0 => PackKey::AreaBrEnMovChoices,
        1 => PackKey::AreaNoEnMovChoices,
        2 => PackKey::AreaKrEnMovChoices,
        3 => PackKey::AreaToEnMovChoices,
        _ => PackKey::AreaRiEnMovChoices,
    }
}

fn extract_en_mov_choices(prg: &[u8]) -> Result<Vec<Section>, String> {
    let banks = area_banks(prg)?;
    let mut out = Vec::with_capacity(5);
    for (area, &bank) in banks.iter().enumerate() {
        let body = bank_slice(prg, bank, EN_MOV_CHOICES_ADDR, EN_MOV_CHOICES_LEN)?;
        // 机械自检：由偏移表推出的表尾必须与 asm 标签给出的位置一致（Br 30/No 22/Kr 24/To 4/Ri 28）
        let offs = bank_slice(prg, bank, 0x96CB, 16)?;
        let want = match area {
            0 => 30,
            1 => 22,
            2 => 24,
            3 => 4,
            _ => 28,
        };
        let mut end = 0usize;
        for &o in offs.iter() {
            let o = o as usize;
            // 只统计落在 asm 标注表内的记录。Tourian 的 type4(Rinka) 偏移 = 表长 4（越界），
            // 原版会把它当记录读后续移动串字节（quirk），不能参与表长推导。
            if o < want {
                let mask = body[o] as usize;
                end = end.max(o + mask + 1 + 1);
            }
        }
        if end != want {
            return Err(format!("bank{bank}:$97D1 移动候选表推得表长 {end} != asm 标注 {want}"));
        }
        out.push(Section { key: en_mov_choices_keys(area), data: body.to_vec() });
    }
    Ok(out)
}

fn extract_sfx_tables(prg: &[u8]) -> Result<Vec<Section>, String> {
    let body = prg
        .get(SFX_TABLES_FLAT..SFX_TABLES_FLAT + SFX_TABLES_LEN)
        .ok_or("SFX 记录窗 $B20D..$B289 越界")?;
    Ok(vec![Section { key: PackKey::SfxTables, data: body.to_vec() }])
}

fn extract_song_headers(prg: &[u8]) -> Result<Vec<Section>, String> {
    let body = prg
        .get(SONG_HEADERS..SONG_HEADERS + SONG_HEADERS_LEN)
        .ok_or("SongHeaders $BD31（12×13B）越界")?;
    Ok(vec![Section { key: PackKey::SongHdrs, data: body.to_vec() }])
}

fn extract_music_streams(prg: &[u8]) -> Result<Vec<Section>, String> {
    let mut streams = Vec::new();
    let mut offs = Vec::new();
    let mut bases = Vec::with_capacity(N_SONGS * 8);
    for s in 0..N_SONGS {
        let ho = *prg
            .get(SONG_HDR_OFF_TBL + s)
            .ok_or_else(|| format!("SongHeaderOffsetTable 越界：{s}"))?
            as usize;
        let hdr = prg
            .get(SONG_HEADERS + ho..SONG_HEADERS + ho + 13)
            .ok_or_else(|| format!("SongHeaders 越界：song {s} off {ho}"))?;
        for c in 0..4 {
            let base = hdr[5 + c * 2] as u16 | ((hdr[6 + c * 2] as u16) << 8);
            bases.extend_from_slice(&base.to_le_bytes());
            let st = if base == 0 {
                Vec::new()
            } else {
                read_until_any(prg, base, &[0x00], 1024)
            };
            offs.extend_from_slice(&(streams.len() as u32).to_le_bytes());
            offs.extend_from_slice(&(st.len() as u32).to_le_bytes());
            streams.extend_from_slice(&st);
        }
    }
    let mut env_ptr_raw = Vec::with_capacity(N_ENVS * 2);
    let mut env_body = Vec::new();
    let mut env_offs = Vec::new();
    for i in 0..N_ENVS {
        let lo = *prg.get(ENV_PTRS_TBL + i * 2).ok_or("VolumeEnvelopePtrTable 越界")?;
        let hi = *prg.get(ENV_PTRS_TBL + i * 2 + 1).ok_or("VolumeEnvelopePtrTable 越界")?;
        env_ptr_raw.extend_from_slice(&[lo, hi]);
        let addr = lo as u16 | ((hi as u16) << 8);
        let e = read_until_any(prg, addr, &[0xFF, 0xF0], 64);
        env_offs.extend_from_slice(&(env_body.len() as u32).to_le_bytes());
        env_offs.extend_from_slice(&(e.len() as u32).to_le_bytes());
        env_body.extend_from_slice(&e);
    }
    let sfx = prg
        .get(SFX_DATA_OFF..SFX_DATA_OFF + 16)
        .ok_or("SFXData $B200 越界")?
        .to_vec();
    Ok(vec![
        Section { key: PackKey::SongStreams, data: streams },
        Section { key: PackKey::SongStreamOffs, data: offs },
        Section { key: PackKey::StreamBases, data: bases },
        Section { key: PackKey::EnvPtrs, data: env_ptr_raw },
        Section { key: PackKey::EnvBody, data: env_body },
        Section { key: PackKey::EnvOffs, data: env_offs },
        Section { key: PackKey::SfxData, data: sfx },
    ])
}


/// 调色板（`PalettePtrTable ; $9560` 各 bank 同址；**项数按 bank 异**——bank0 标题 19 项 /
/// bank1-5 区域 28 项，`prg0_title.asm:3201` 与 `prg2_norfair.asm:53-80`）。
/// 每条指针 → 一条 VRAMStruct 记录流 `[hi][lo][ctrl][data…]`，`hi==0` 为终止（含该字节）；
/// 六 bank 的流拼接为单条 `pal.data`，`pal.offs` 记各条在体内的 `(off,len)`。
const PAL_TABLE_FLAT: usize = 0x1560; // bank0 $9560（平铺口径，各 bank +0x4000）
pub const PAL_N: [usize; 6] = [19, 28, 28, 28, 28, 28];

/// VRAMStruct 记录流（`hi==0` 终止含尾；上限 256 B 防御）。
fn vram_stream(prg: &[u8], off: usize) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    let mut o = off;
    loop {
        let hi = *prg.get(o).ok_or_else(|| format!("调色板流越界：{o:#06X}"))?;
        out.push(hi);
        if hi == 0 {
            return Ok(out);
        }
        let lo = *prg.get(o + 1).ok_or("调色板流越界(lo)")?;
        let ctrl = *prg.get(o + 2).ok_or("调色板流越界(ctrl)")?;
        out.push(lo);
        out.push(ctrl);
        let mut count = (ctrl & 0x3F) as usize;
        if count == 0 {
            count = 256;
        }
        if ctrl & 0x40 != 0 {
            out.push(*prg.get(o + 3).ok_or("调色板流越界(rle)")?);
            o += 4;
        } else {
            let seg = prg.get(o + 3..o + 3 + count).ok_or("调色板流越界(data)")?;
            out.extend_from_slice(seg);
            o += 3 + count;
        }
        if out.len() > 4096 {
            return Err(format!("调色板流未终止（{off:#06X}）"));
        }
    }
}

fn extract_palette_data(prg: &[u8]) -> Result<Vec<Section>, String> {
    let mut ptrs = Vec::new();
    let mut body = Vec::new();
    let mut offs = Vec::new();
    for (bank, n) in PAL_N.iter().enumerate() {
        let boff = bank * 0x4000;
        for i in 0..*n {
            let o = PAL_TABLE_FLAT + boff + i * 2;
            let lo = *prg.get(o).ok_or("PalettePtrTable 越界")?;
            let hi = *prg.get(o + 1).ok_or("PalettePtrTable 越界")?;
            let addr = lo as u16 | ((hi as u16) << 8);
            ptrs.extend_from_slice(&[lo, hi]);
            let st = vram_stream(prg, boff + (addr as usize - 0x8000))?;
            offs.extend_from_slice(&(body.len() as u32).to_le_bytes());
            offs.extend_from_slice(&(st.len() as u32).to_le_bytes());
            body.extend_from_slice(&st);
        }
    }
    Ok(vec![
        Section { key: PackKey::PalPtrs, data: ptrs },
        Section { key: PackKey::PalData, data: body },
        Section { key: PackKey::PalOffs, data: offs },
    ])
}


/// 声引擎区（`sound_engine.asm` 的 `$B289` 起连续窗口）。
///
/// 这一段是**地址窗口**而非逻辑表：`sound_hook.rs` 用区内偏移取块描述子（5B）、
/// 16B 表、以及各类单字节参数，全部以 `win - $B289` 寻址。长度 2688 B = `$B289..$BD09`
/// （末项 `$BD08` 一带的三角/噪声参数表尾），由旧常量实测长度与 ROM 平铺切片逐字节相等定锚：
/// `prg[0x3289..0x3D09]`（bank0 平铺，`addr - $8000`）。
const SOUND_REGION_FLAT: usize = 0x3289; // bank0 $B289
const SOUND_REGION_LEN: usize = 2688;    // 0xA80，$B289→$BD09

fn extract_sound_region(prg: &[u8]) -> Result<Vec<Section>, String> {
    let off = SOUND_REGION_FLAT;
    if off + SOUND_REGION_LEN > prg.len() {
        return Err(format!("声引擎区越界：需要 {} B，PRG {} B",
                           off + SOUND_REGION_LEN, prg.len()));
    }
    if (prg.len() - off) < SOUND_REGION_LEN {
        return Err("声引擎区越界".to_string());
    }
    Ok(vec![Section { key: PackKey::SndRegion,
                      data: prg[off..off + SOUND_REGION_LEN].to_vec() }])
}

// ---- 区特殊物品表（SpecItmsTbl）----------------------------------------------------
//
// 格式（`refs/m1disasm/*/items.asm:38-54` 注 + ROM 走表实证）：
//   Y 组：`<MapY> / .word <下一 Y 组地址>`（Y 序链表，`$FFFF` 终）
//   X 条目：`<MapX> / <下一条目相对偏移 | $FF 末条> / <条目数据…$00 终>`
//   条目数据：it_Type（低半字节 = ChooseSpawningRoutine 分派号，prg7:9322）+ 参数
//   （Squeept=2 / PowerUp=2 / Mellow=0 / Elevator=1 / MBCannon=1 / MB=0 / Zebetite=0 /
//    Rinka=0 / Door=1 / Palette=0）
// 输出：扁平 6 B 记录 `[map_y, map_x, item_type, arg0, arg1, arg2]`——Y 序链表在语义上
// 等价于「按 (map_y,map_x) 分组」，消费侧 `ScanForItems(RD98)` 就是按当前图格匹配。

/// 区序（`InArea & $0F`，与 `BankTable $CA30` 一致）：bank / 表址 / 段键。
const AREA_ITEMS: [(u8, u16, PackKey, &str); 5] = [
    (1, 0xA3D6, PackKey::BrItems, "SpecItmsTbl_BRINSTAR"),
    (2, 0xA2D9, PackKey::NoItems, "SpecItmsTbl_NORFAIR"),
    (4, 0xA26D, PackKey::KrItems, "SpecItmsTbl_KRAID"),
    (3, 0xA83B, PackKey::ToItems, "SpecItmsTbl_TOURIAN"),
    (5, 0xA20D, PackKey::RiItems, "SpecItmsTbl_RIDLEY"),
];

/// it_Type 低半字节 → 参数字节数（未知类型 ⇒ 硬错，不猜长度）。
fn item_arg_len(t: u8) -> Option<usize> {
    Some(match t {
        0x01 => 2, // Squeept
        0x02 => 2, // PowerUp（pu_XXX + 位置字节）
        0x03 => 0, // Mellow
        0x04 => 1, // Elevator
        0x05 => 1, // MBCannon
        0x06 => 0, // MotherBrain
        0x07 => 0, // Zebetite
        0x08 => 0, // RinkaSpawner
        0x09 => 1, // Door
        0x0A => 0, // Palette
        _ => return None,
    })
}

fn extract_items(prg: &[u8]) -> Result<Vec<Section>, String> {
    let mut out = Vec::with_capacity(AREA_ITEMS.len());
    for (bank, addr, key, label) in AREA_ITEMS {
        let base = bank_source_offset(bank, addr)?;
        let mut recs = Vec::new();
        let mut y_off = base;
        let mut seen_y: Vec<usize> = Vec::new();
        loop {
            if seen_y.contains(&y_off) {
                return Err(format!("{label}: Y 链成环（off {y_off:#06X}）"));
            }
            seen_y.push(y_off);
            let map_y = *prg.get(y_off).ok_or_else(|| format!("{label}: Y 组越界"))?;
            let lo = *prg.get(y_off + 1).ok_or_else(|| format!("{label}: Y 链越界"))? as usize;
            let hi = *prg.get(y_off + 2).ok_or_else(|| format!("{label}: Y 链越界"))? as usize;
            let next_y = lo | (hi << 8);
            let mut x_off = y_off + 3;
            let mut seen_x: Vec<usize> = Vec::new();
            loop {
                if seen_x.contains(&x_off) {
                    return Err(format!("{label}: X 链成环（off {x_off:#06X}）"));
                }
                seen_x.push(x_off);
                let map_x = *prg.get(x_off).ok_or_else(|| format!("{label}: 条目越界"))?;
                let next_rel = *prg.get(x_off + 1)
                    .ok_or_else(|| format!("{label}: 条目越界"))?;
                let mut p = x_off + 2;
                loop {
                    let raw = *prg.get(p).ok_or_else(|| format!("{label}: 条目数据越界"))?;
                    if raw == 0x00 {
                        break;
                    }
                    let t = raw & 0x0F;
                    let n = item_arg_len(t)
                        .ok_or_else(|| format!("{label}: 未知 it_Type {t:#04X}（off {p:#06X}）"))?;
                    let mut args = [0u8; 3];
                    for (k, slot) in args.iter_mut().enumerate().take(n) {
                        *slot = *prg.get(p + 1 + k)
                            .ok_or_else(|| format!("{label}: 参数越界（off {p:#06X}）"))?;
                    }
                    recs.extend_from_slice(&[map_y, map_x, raw, args[0], args[1], args[2]]);
                    p += 1 + n;
                }
                if next_rel == 0xFF {
                    break;
                }
                x_off += next_rel as usize;
            }
            if next_y == 0xFFFF {
                break;
            }
            y_off = bank_source_offset(bank, next_y as u16)?;
        }
        out.push(Section { key, data: recs });
    }
    Ok(out)
}

/// sprite 数据 blob（`common_sprite_data.asm`，区 bank 共载 `$8572` 起）。
/// 体内三表界：`ObjectAnimIndexTbl $8572` / `ObjFramePtrTable $860B` / `ObjPlacePtrTable $86DF`，
/// 其后为帧记录 `ObjFrame_*` / 放置记录 `ObjPlace_*`（指针内嵌自洽）。
///
/// 长度口径（沿用原烘焙器，附来源）：asm 尾注最大址 `$8B03` + 末记录 32 B 裕 = `$8B23`，
/// 即 `0x8B23 - 0x8572 = 1457` B。多切无害（消费端一律按表指针索引，不读尾）。
const SPRITE_BLOB_FLAT: usize = 0x4572; // bank1 $8572（区 bank 共载同址）
const SPRITE_BLOB_LEN: usize = 1457;

fn extract_sprite_data(prg: &[u8]) -> Result<Vec<Section>, String> {
    let end = SPRITE_BLOB_FLAT + SPRITE_BLOB_LEN;
    let data = prg
        .get(SPRITE_BLOB_FLAT..end)
        .ok_or("sprite 数据 blob 越界")?
        .to_vec();
    Ok(vec![Section { key: PackKey::SpriteData, data }])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// bank 换算：bank7 固定窗口 / bank0-6 切窗口 / 越界拒绝。
    #[test]
    fn bank_source_offset_windows() {
        assert_eq!(bank_source_offset(7, 0xC6E0).unwrap(), 7 * 0x4000 + 0x6E0);
        assert_eq!(bank_source_offset(6, 0x8000).unwrap(), 6 * 0x4000);
        assert_eq!(bank_source_offset(0, 0xA441).unwrap(), 0x2441);
        assert!(bank_source_offset(7, 0x8000).is_err(), "bank7 低址应拒绝");
        assert!(bank_source_offset(3, 0x7000).is_err(), "切 bank 低址应拒绝");
    }

    /// 包往返：写出来的包能被 `PackView` 解析并逐段取回。
    #[test]
    fn write_then_parse_roundtrip() {
        let secs = vec![
            Section { key: PackKey::GfxInfo, data: vec![1u8, 2, 3] },
            Section { key: PackKey::GfxData, data: vec![9u8; 500] },
        ];
        let bytes = write_pack(&secs, 0x1234_5678, 999);
        let leaked: &'static [u8] = Box::leak(bytes.into_boxed_slice());
        let p = crate::pack::PackView::parse(leaked, Some(0x1234_5678)).expect("往返解析应成功");
        assert_eq!(p.get(PackKey::GfxInfo), Some(&[1u8, 2, 3][..]));
        assert_eq!(p.get(PackKey::GfxData).map(|s| s.len()), Some(500));
    }

    /// 自界含终止符：`$FF` 是块尾的一部分（房流/结构体的长度口径）。
    #[test]
    fn read_until_ff_includes_terminator() {
        let prg = [0u8, 1, 2, 0xFF, 3];
        assert_eq!(read_until_ff(&prg, 0).unwrap(), vec![0, 1, 2, 0xFF]);
        // 无终止符 = 硬错（不得静默返回整段——缺证据 fail-loud）
        assert!(read_until_ff(&[1u8, 2, 3], 0).is_err());
        // 起点越界同错
        assert!(read_until_ff(&[1u8], 5).is_err());
    }

    /// 变长块提取：指针表 → 拼接体 + `(off,len)` 索引，两者自洽可解码。
    #[test]
    fn extract_streams_builds_body_and_index() {
        // 合成窗口：$8000 起两指针（$8004/$8007），数据 $8004: 01 02 FF；$8007: 03 FF
        let mut win = vec![0x04, 0x80, 0x07, 0x80];
        win.extend_from_slice(&[0x01, 0x02, 0xFF]);
        win.extend_from_slice(&[0x03, 0xFF]);
        // bank0 的切窗口起点就是 $8000（bank7 是 $C000 固定窗口，不能拿 $8000 当起点）
        let (body, offs) = extract_streams(&win, 0, 0x8000, 2).expect("两指针应可提取");
        assert_eq!(body, vec![0x01, 0x02, 0xFF, 0x03, 0xFF]);
        assert_eq!(offs.len(), 2 * 8);
        let o0 = u32::from_le_bytes(offs[0..4].try_into().unwrap());
        let l0 = u32::from_le_bytes(offs[4..8].try_into().unwrap());
        let o1 = u32::from_le_bytes(offs[8..12].try_into().unwrap());
        let l1 = u32::from_le_bytes(offs[12..16].try_into().unwrap());
        assert_eq!((o0, l0, o1, l1), (0, 3, 3, 2));
    }

    /// `AREA_BANKS` 必须等于 `BankTable ; 07:CA30`（ROM 字节 `02 03 05 04 06`）减 1：
    /// area 序 Br/No/Kr/To/Ri ⇒ bank `1/2/4/3/5`（`BankTable` 值 = bank+1）。
    ///
    /// **为什么必须在提取侧钉**：`AreaPaletteToggle` 的 bank3(Tourian) 与 bank4(Kraid)
    /// 值同为 `$06` ⇒ 表驱动顺序若写反，运行时段值与所有消费者**都看不出来**；
    /// 只有把「区序→bank」的映射本身钉在 asm 锚上，错序才会红（r9 复核要求）。
    #[test]
    fn area_banks_derive_from_rom_bank_table() {
        // 手工锚（A 级）：ROM `07:CA30` = `02 03 05 04 06` ⇒ bank = 值-1 = [1,2,4,3,5]。
        // 解析面从 ROM 现读（`area_banks`）——两者一致性由本测钉住；若将来有人改死表，
        // 只有「与 ROM 同源」这层能发现。
        let rom = match std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../rom/Metroid (USA).nes")) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("跳过：无 ROM（{e}）——本测需真 ROM 才能验 BankTable 同源");
                return;
            }
        };
        let prg = &rom[0x10..];
        assert_eq!(crate::extract::area_banks(prg).unwrap(), [1u8, 2, 4, 3, 5]);
        // 逐 bank 自证：AreaPaletteToggle（$95DA）= bank1/2 → 01、bank3/4/5 → 06
        let pal = extract_area_pal_toggle(prg).unwrap();
        assert_eq!(pal[0].data, vec![0x01, 0x01, 0x06, 0x06, 0x06]);
    }

    /// 重复 key 必须在解析侧被拒（段名唯一是包格式不变式）。
    #[test]
    fn duplicate_section_keys_are_rejected() {
        let secs = vec![
            Section { key: PackKey::Meta, data: b"{}".to_vec() },
            Section { key: PackKey::Meta, data: b"{}".to_vec() },
        ];
        let bytes = write_pack(&secs, 1, 1);
        let leaked: &'static [u8] = Box::leak(bytes.into_boxed_slice());
        assert!(crate::pack::PackView::parse(leaked, None).is_err(), "重复段应被拒");
    }
}
