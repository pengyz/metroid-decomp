//! title 数据（数据内容在 game-data（架构约束））。

// pub mod title_data;（W5g：20 常量全数迁资产包，见 assets::title_*）
// area_tables 已退役：AREA_SAMUS_Y/AREA_MUSIC_FLAG 是 ROM 区头 $95D9/$95CD 的抄本，
// 现统一走 assets::area_header()（段 area.{br,no,kr,to,ri}.header）。
pub mod world_map;
// palette_data 已迁资产包（`assets::{palette_ptrs,palette_stream}`）
