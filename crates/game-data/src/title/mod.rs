//! title 数据（数据内容在 game-data（架构约束））。

pub mod title_data;
pub mod vram_structs;
pub mod area_tables; // r25 烘焙（AreaSamusY 五值——SamusInit 面）
pub mod world_map; // r25 烘焙（WorldMap 1KB+区域起点——CopyMap/MoreInit 面）
