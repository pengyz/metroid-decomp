//! `game-core::world` —— 世界域（房间/门——批量改写第二簇+第十四簇）。

pub(crate) mod statue;
pub(crate) mod door;
pub(crate) mod elevator;
pub(crate) mod end_timer;
pub(crate) mod room_load;
pub(crate) mod room_ram;
pub(crate) mod update;
pub(crate) mod camera;
pub(crate) mod object;
pub(crate) mod tile_blast;
pub(crate) mod object_globals;
pub(crate) mod scroll_blocks;
pub(crate) mod room_draw;
pub(crate) mod tourian_quota;

/// 房 5 门格到达性验收（#104）——测试面文件，不计入 `no-raw-addr` 生产面。
#[cfg(test)]
mod tests;
