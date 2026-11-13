//! 世界地图+区域起点（r25 烘焙——tools/bake_area_map.py 生成；**禁手抄**）。
//!
//! ROM 锚（A 级）：WorldMap $A53E（prg0_title.asm:5118——32×32 表）；AreaMapPosX/Y
//! $95D7/$95D8（区 bank 族）。字节自证：
//! `python3 -c "p=open('rom/Metroid (USA).nes','rb').read()[0x10:]; print(p[0x253E:0x255E].hex())"`

/// WorldMap（$A53E——1KB：行=MapY 列=MapX；值=房间号/$FF=无房）。
// `WORLD_MAP` 已迁入资产包（`assets::world_map()`）：ROM 锚 `WorldMap: ; 00:A53E`
// （`refs/m1disasm/SRC/prg0_title.asm:5118`，32×32=1 KB）。本文件只保留**布局知识**。


/// 区域起点（AreaMapPosX/Y $95D7/$95D8 族——序=InArea&$0F：0=Brinstar…4=Ridley）。
pub const AREA_MAP_POS: [(u8, u8); 5] = [(3, 14), (22, 13), (3, 4), (7, 20), (25, 24)];

#[cfg(test)]
mod tests {
    use super::*;

    /// ROM 锚：地图 1KB 尺寸+首行 FF 形+起点（Brinstar x=3/y=0E——金侧 f520 实证）。
    ///
    /// 数据已迁资产包（`assets::world_map()`）⇒ 无包时**显式 SKIP**（不静默通过）。
    #[test]
    fn world_map_rom_anchor() {
        assert_eq!(AREA_MAP_POS[0], (0x03, 0x0E), "Brinstar 起点（MapPos 03/0E）");
        if !crate::assets::is_ready() {
            eprintln!("SKIP（显式）：资产包未加载——先跑 pack-tool extract");
            return;
        }
        let m = crate::assets::world_map();
        assert_eq!(m.len(), 1024);
        assert_eq!(&m[0..4], &[0xFF, 0xFF, 0xFF, 0xFF]);
        // 起始房：MapY=0E/MapX=03 → 地图[0E*32+3]=0x09（房间 9——金侧 RoomPtr=A5D5 对形）
        assert_eq!(m[0x0E * 32 + 3], 0x09);
    }
}
