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
