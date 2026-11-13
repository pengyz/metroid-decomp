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
    // 【W2 暂缓·敌帧/位数据面】不启用：`tools/bake_enemy_gfx.py` 与提交件已漂移
    // （`--check` FAIL；脚本口径 en.places=539457 B vs 提交件 EN_PLACES=204448 B），
    // 且两张指针表存在 34/44 个 `< $8000` 的非指针值（表长口径 $9EE0+256≠$9F0E）。
    // 详见 `docs/findings/enemy-gfx-baker-drift.md`。迁移前必须先定谳"哪份是运行真值"，
    // 否则就是把 1.4 MB 敌动画数据在无证据的情况下换掉。
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

// ---- 敌帧/位数据面（bank1 $9D6A-$9F0E 族；移植自 tools/bake_enemy_gfx.py）--------
//
// ROM 锚（A 级）：`AreaPointers_ROM` 7 字（prg1:88）里的 EnAnimTable `$9D6A` /
// EnFramePtrTable1 `$9DE0` / EnFramePtrTable2 `$9EE0` / EnPlacePtrTable `$9F0E`；
// `CopyAreaPointers $C8B0`（prg7:1936）把它们搬到 RAM `$3B-$48`。
// 帧流=[ctrl][radiusY][radiusX][流…$FF]；位流=y/x 对。
const EN_BANK_BASE: usize = 0x4000; // bank1（Brinstar）切窗口起点
const EN_ANIM_TABLE: u16 = 0x9D6A;
const EN_FRAME_PTRS1: u16 = 0x9DE0;
const EN_FRAME_PTRS2: u16 = 0x9EE0;
const EN_PLACE_PTRS: u16 = 0x9F0E;
const EN_N_PLACES: usize = 128;

/// `(bank1 切窗口) cpu addr → PRG 偏移`，**保留烘焙器的负索引回绕语义**。
///
/// 历史事实（A 级，必须记）：`tools/bake_enemy_gfx.py` 用 Python 列表负索引写这段，
/// `$9EE0`/`$9F0E` 两张表里存在 34/44 个 `< $8000` 的指针值（这些不是合法 ROM 指针，
/// 而是表长口径与 asm 标签不符造成的错位读），Python 的 `prg[负偏移]` 会从 PRG **尾部**
/// 回绕取字节，于是旧常量里混入了尾部字节。W2 只做**数据搬家**，不改数据语义 ⇒ 这里
/// 精确复刻该回绕，保证「包内字节 == 旧常量字节」可机械验证。
/// 表长口径问题另立卡（`docs/findings/enemy-gfx-table-extent.md`）。
fn en_off_wrapped(prg: &[u8], addr: u16) -> usize {
    let raw = EN_BANK_BASE as isize + (addr as isize - 0x8000);
    if raw >= 0 {
        raw as usize
    } else {
        (prg.len() as isize + raw) as usize
    }
}

fn en_off(addr: u16) -> usize {
    EN_BANK_BASE + (addr as usize - 0x8000)
}

/// 帧流长（`[ctrl][ry][rx][流…$FF]`）——解析规则**只在提取侧实现一份**。
///
/// 流形：`<$FC` 单字节 / `=$FC` 偏移 3B / `=$FD` 新 ctrl 2B / `=$FE` 跳位 1B / `=$FF` 尾。
fn frame_end(prg: &[u8], addr: u16) -> Result<usize, String> {
    let o = en_off_wrapped(prg, addr);
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

#[allow(dead_code)] // W2 暂缓（见 docs/findings/enemy-gfx-baker-drift.md）：保留实现，待表长/位件长度规则定谳后启用。
fn extract_enemy_gfx(prg: &[u8]) -> Result<Vec<Section>, String> {
    let mut out = Vec::new();
    out.push(Section { key: PackKey::EnAnimTable, data: en_slice(prg, EN_ANIM_TABLE, 128)? });

    let f1 = en_slice(prg, EN_FRAME_PTRS1, 256)?;
    let f2 = en_slice(prg, EN_FRAME_PTRS2, 256)?;
    let pl = en_slice(prg, EN_PLACE_PTRS, 256)?;
    out.push(Section { key: PackKey::EnFramePtrs1, data: f1.clone() });
    out.push(Section { key: PackKey::EnFramePtrs2, data: f2.clone() });
    out.push(Section { key: PackKey::EnPlacePtrs, data: pl.clone() });

    // 帧数据体（两表各 128 项，共 256）
    let mut frames = Vec::new();
    let mut foffs: Vec<(u32, u32)> = Vec::with_capacity(256);
    for tbl in [&f1, &f2] {
        for i in (0..256).step_by(2) {
            let addr = u16::from_le_bytes([tbl[i], tbl[i + 1]]);
            if addr == 0 {
                foffs.push((0, 0));
                continue;
            }
            let ln = frame_end(prg, addr)?;
            foffs.push((frames.len() as u32, ln as u32));
            frames.extend_from_slice(&en_slice(prg, addr, ln)?);
        }
    }
    out.push(Section { key: PackKey::EnFrames, data: frames });
    out.push(Section { key: PackKey::EnFrameOffs, data: encode_offs(&foffs)? });

    // 位件数据体（128 项，长=至下一指针）
    let addrs: Vec<u16> = (0..256)
        .step_by(2)
        .map(|i| u16::from_le_bytes([pl[i], pl[i + 1]]))
        .collect();
    let mut places = Vec::new();
    let mut poffs: Vec<(u32, u32)> = Vec::with_capacity(EN_N_PLACES);
    for (i, &addr) in addrs.iter().enumerate() {
        if addr == 0 {
            poffs.push((0, 0));
            continue;
        }
        let nxt = if i + 1 < addrs.len() && addrs[i + 1] != 0 {
            addrs[i + 1]
        } else {
            addr.wrapping_add(32)
        };
        let ln = place_end(addr, nxt);
        poffs.push((places.len() as u32, ln as u32));
        places.extend_from_slice(&en_slice(prg, addr, ln)?);
    }
    out.push(Section { key: PackKey::EnPlaces, data: places });
    out.push(Section { key: PackKey::EnPlaceOffs, data: encode_offs(&poffs)? });
    Ok(out)
}

/// 切片（**复刻 Python 切片截断语义**：`prg[o:o+len]` 越界时截到 PRG 末尾，不报错）。
///
/// 项表里记录的 `len` 是**声明长度**（与旧常量一致），数据体按实际可得字节拼接——
/// 与烘焙器逐字节同结果。
fn en_slice(prg: &[u8], addr: u16, len: usize) -> Result<Vec<u8>, String> {
    let o = en_off_wrapped(prg, addr);
    if o >= prg.len() {
        return Ok(Vec::new());
    }
    let end = (o + len).min(prg.len());
    Ok(prg[o..end].to_vec())
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
