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
    out.extend(extract_enemy_gfx(prg)?);
    // 世界地图：`WorldMap: ; 00:A53E`（prg0_title.asm:5118）32×32=1024 B，bank0。
    // `CopyMap $A93E`（prg0_title.asm:5153）运行时把它拷到 WorldMapRAM `$7000-$73FF`。
    out.push(Section { key: PackKey::WorldMap, data: bank_slice(prg, 0, WORLD_MAP_ADDR, WORLD_MAP_LEN)? });
    Ok(out)
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

const EN_BANK_BASE: usize = 0x4000; // bank1（Brinstar）切窗口起点
const EN_ANIM_TABLE: u16 = 0x9D6A;
const EN_ANIM_LEN: usize = 118;
const EN_FRAME_PTRS: u16 = 0x9DE0;
const EN_FRAME_LEN: usize = 151;
const EN_PLACE_PTRS: u16 = 0x9F0E;
const EN_PLACE_LEN: usize = 14;

/// 按 (bank, cpu addr) 取 ROM 字节（bank0-6 落 `$8000` 窗口，bank7 落 `$C000`）。
fn bank_slice(prg: &[u8], bank: u8, addr: u16, len: usize) -> Result<Vec<u8>, String> {
    let o = bank_source_offset(bank, addr)?;
    prg.get(o..o + len)
        .map(|s| s.to_vec())
        .ok_or_else(|| format!("ROM 越界 bank{bank}:{addr:04X} len={len}"))
}

fn en_off(addr: u16) -> Result<usize, String> {
    if addr < 0x8000 {
        return Err(format!("敌数据指针非法（< $8000）: {addr:04X}"));
    }
    Ok(EN_BANK_BASE + (addr as usize - 0x8000))
}

/// 帧流长（`[ctrl][ry][rx][流…$FF]`）——解析规则**只在提取侧实现一份**。
///
/// 流形：`<$FC` 单字节 / `=$FC` 偏移 3B / `=$FD` 新 ctrl 2B / `=$FE` 跳位 1B / `=$FF` 尾。
fn frame_end(prg: &[u8], addr: u16) -> Result<usize, String> {
    let o = en_off(addr)?;
    let mut n = 3usize;
    while n < 256 {
        let b = *prg.get(o + n).ok_or_else(|| format!("帧流越界 addr={addr:04X} n={n}"))?;
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

/// 位件长（至下一指针；退化情形给 16B 下界）——与烘焙器同口径。
fn place_end(addr: u16, nxt: u16) -> usize {
    if nxt > addr {
        (nxt as usize - addr as usize).max(4)
    } else {
        16
    }
}

#[allow(dead_code)] // W2 暂缓（见 docs/findings/enemy-gfx-baker-drift.md）：待 L4/语义基线复核后启用。
fn extract_enemy_gfx(prg: &[u8]) -> Result<Vec<Section>, String> {
    let mut out = Vec::new();
    out.push(Section { key: PackKey::EnAnimTable, data: en_slice(prg, EN_ANIM_TABLE, EN_ANIM_LEN)? });

    let f = en_slice(prg, EN_FRAME_PTRS, EN_FRAME_LEN * 2)?;
    out.push(Section { key: PackKey::EnFramePtrs1, data: f.clone() });
    out.push(Section { key: PackKey::EnFramePtrs2, data: Vec::new() }); // Brinstar 单帧表（151 项已全在 fptr1）

    let pl = en_slice(prg, EN_PLACE_PTRS, EN_PLACE_LEN * 2)?;
    out.push(Section { key: PackKey::EnPlacePtrs, data: pl.clone() });

    // 帧数据体（151 项）
    let mut frames = Vec::new();
    let mut foffs: Vec<(u32, u32)> = Vec::with_capacity(EN_FRAME_LEN);
    for i in (0..EN_FRAME_LEN * 2).step_by(2) {
        let addr = u16::from_le_bytes([f[i], f[i + 1]]);
        if addr == 0 {
            foffs.push((0, 0));
            continue;
        }
        let ln = frame_end(prg, addr)?;
        foffs.push((frames.len() as u32, ln as u32));
        frames.extend_from_slice(&en_slice(prg, addr, ln)?);
    }
    out.push(Section { key: PackKey::EnFrames, data: frames });
    out.push(Section { key: PackKey::EnFrameOffs, data: encode_offs(&foffs)? });

    // 位件数据体（14 项，长=至下一指针）
    let addrs: Vec<u16> = (0..EN_PLACE_LEN * 2)
        .step_by(2)
        .map(|i| u16::from_le_bytes([pl[i], pl[i + 1]]))
        .collect();
    let mut places = Vec::new();
    let mut poffs: Vec<(u32, u32)> = Vec::with_capacity(EN_PLACE_LEN);
    for (i, &addr) in addrs.iter().enumerate() {
        if addr == 0 {
            poffs.push((0, 0));
            continue;
        }
        // 末项无后继指针 ⇒ 走 `place_end` 的退化分支（16B 下界）。
        // **不改成 addr+32**：旧烘焙器在此处得到 16（其后继是错位读来的伪指针 ⇒ nxt<addr
        // ⇒ 退化分支），本波只做搬家 ⇒ 保持 16 才能逐项字节相等。
        // 真值待考：帧数据首指针 `$9FC2` ⇒ 末位件实际只到 `$9FC1`（=4B），
        // 即旧数据在末项上**多读了 12B 帧数据**；是否影响渲染取决于是否有房间用 index 13，
        // 另立卡（见 findings §6）。
        let nxt = if i + 1 < addrs.len() && addrs[i + 1] != 0 {
            addrs[i + 1]
        } else {
            0
        };
        let ln = place_end(addr, nxt);
        poffs.push((places.len() as u32, ln as u32));
        places.extend_from_slice(&en_slice(prg, addr, ln)?);
    }
    out.push(Section { key: PackKey::EnPlaces, data: places });
    out.push(Section { key: PackKey::EnPlaceOffs, data: encode_offs(&poffs)? });
    Ok(out)
}

fn en_slice(prg: &[u8], addr: u16, len: usize) -> Result<Vec<u8>, String> {
    let o = en_off(addr)?;
    prg.get(o..o + len)
        .map(|s| s.to_vec())
        .ok_or_else(|| format!("敌数据越界 addr={addr:04X} off={o} len={len}"))
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
        SegMeta { key: PackKey::EnAnimTable, kind: "bytes", elem: 1, count: EN_ANIM_LEN,
                  rom: "bank01:$9D6A", note: "EnAnimTable（$FF=动画结束）" },
        SegMeta { key: PackKey::EnFramePtrs1, kind: "u16-table", elem: 2, count: EN_FRAME_LEN,
                  rom: "bank01:$9DE0", note: "帧指针 151 项（$9DE0→$9F0E=302B）" },
        SegMeta { key: PackKey::EnFramePtrs2, kind: "u16-table", elem: 2, count: 0,
                  rom: "bank01:$9EE0", note: "**空**：$9EE0 处无独立表（旧烘焙器 128 项读法即 bug 源）" },
        SegMeta { key: PackKey::EnPlacePtrs, kind: "u16-table", elem: 2, count: EN_PLACE_LEN,
                  rom: "bank01:$9F0E", note: "位件指针 14 项（表内 14 指针后即数据）" },
        SegMeta { key: PackKey::EnFrames, kind: "bytes", elem: 1, count: 0,
                  rom: "bank01:$9FC2+", note: "帧流拼接（各自界于 $FF）" },
        SegMeta { key: PackKey::EnPlaces, kind: "bytes", elem: 1, count: 0,
                  rom: "bank01:$9F2E+", note: "位件流拼接" },
        SegMeta { key: PackKey::EnFrameOffs, kind: "offs-table", elem: 8, count: EN_FRAME_LEN,
                  rom: "(派生)", note: "(off u32, len u32) LE ×151" },
        SegMeta { key: PackKey::EnPlaceOffs, kind: "offs-table", elem: 8, count: EN_PLACE_LEN,
                  rom: "(派生)", note: "(off u32, len u32) LE ×14" },
        SegMeta { key: PackKey::WorldMap, kind: "bytes", elem: 1, count: WORLD_MAP_LEN,
                  rom: "bank00:$A53E", note: "WorldMap 32×32（$FF=无房；CopyMap 源）" },
    ];
    v
}

/// 生成 meta JSON 文本（自带缩进，`pack-tool list` 原样打印即可读）。
pub fn build_meta_json(sections: &[Section], rom_crc32: u32, rom_size: u32) -> Vec<u8> {
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
    out.into_bytes()
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
}
