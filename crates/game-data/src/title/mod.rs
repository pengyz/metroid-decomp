//! title 数据（数据内容在 game-data（架构约束））。

pub mod title_data;
pub mod vram_structs;
pub mod area_tables; // r25 烘焙（AreaSamusY 五值——SamusInit 面）
pub mod world_map;
pub mod palette_data;
pub mod gfx_data; // r25 烘焙（GFXInfo 29 项——LoadGFX 面） // r25 烘焙（六 bank 调色板——NMI 调色板径） // r25 烘焙（WorldMap 1KB+区域起点——CopyMap/MoreInit 面）
