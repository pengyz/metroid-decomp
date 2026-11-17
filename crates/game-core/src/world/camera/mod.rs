//! 相机（滚动）状态：`ScrollX/ScrollY/ScrollDir/MapPosX/MapPosY/SamusScrX/SamusScrY`。
//!
//! 不变量：`ScrollX` 是**当前列内的像素偏移**，`MapPosX` 是**房间内的列号**——
//! 两者必须同步推进（`ScrollRight` 在新列时 `inc MapPosX` 再 `inc ScrollX`；
//! 到界时把 `MapPosX` 复位）。任何单独改其中一个的写法都会让映射与像素脱钩，
//! 症状是"滚动量对了但房间位置错了"（L4 上表现为背景整体错位、HUD 正常）。

pub(crate) mod scroll;

use crate::frame::Manager;
use crate::memmap::nesmap::{NesSpan, SpanKey};
use crate::ram_map::{
    MAP_POS_X, MAP_POS_Y, SAMUS_SCR_X, SAMUS_SCR_Y, SCROLL_BLOCK_NT3, SCROLL_DIR, SCROLL_X,
    SCROLL_Y,
};

/// `ScrollDir`：0=上、1=下、2=左、3=右（asm `@currentlyScrollingUp` 起的四态）。
pub(crate) mod dir {
    pub(crate) const UP: u8 = 0;
    pub(crate) const DOWN: u8 = 1;
    pub(crate) const LEFT: u8 = 2;
    pub(crate) const RIGHT: u8 = 3;
}

/// 相机借用视图。
pub struct Camera<'a> {
    pub(crate) mem: &'a mut Manager,
}

impl<'a> Manager {
    /// 取相机句柄（短生命周期，现场派生）。
    pub fn camera(&mut self) -> Camera<'_> {
        Camera { mem: self }
    }
}

impl Camera<'_> {
    pub fn scroll_x(&self) -> u8 {
        self.mem.read_ram(crate::ram_map::SCROLL_X)
    }

    pub fn set_scroll_x(&mut self, v: u8) {
        self.mem.write_ram(crate::ram_map::SCROLL_X, v);
    }

    pub fn scroll_y(&self) -> u8 {
        self.mem.read_ram(crate::ram_map::SCROLL_Y)
    }

    pub fn set_scroll_y(&mut self, v: u8) {
        self.mem.write_ram(crate::ram_map::SCROLL_Y, v);
    }

    pub fn dir(&self) -> u8 {
        self.mem.read_ram(crate::ram_map::SCROLL_DIR)
    }

    pub fn set_dir(&mut self, v: u8) {
        self.mem.write_ram(crate::ram_map::SCROLL_DIR, v);
    }

    pub fn map_x(&self) -> u8 {
        self.mem.read_ram(crate::ram_map::MAP_POS_X)
    }

    pub fn set_map_x(&mut self, v: u8) {
        self.mem.write_ram(crate::ram_map::MAP_POS_X, v);
    }

    pub fn map_y(&self) -> u8 {
        self.mem.read_ram(crate::ram_map::MAP_POS_Y)
    }

    pub fn set_map_y(&mut self, v: u8) {
        self.mem.write_ram(crate::ram_map::MAP_POS_Y, v);
    }

    pub fn samus_scr_x(&self) -> u8 {
        self.mem.read_ram(crate::ram_map::SAMUS_SCR_X)
    }

    pub fn set_samus_scr_x(&mut self, v: u8) {
        self.mem.write_ram(crate::ram_map::SAMUS_SCR_X, v);
    }

    pub fn samus_scr_y(&self) -> u8 {
        self.mem.read_ram(crate::ram_map::SAMUS_SCR_Y)
    }

    pub fn set_samus_scr_y(&mut self, v: u8) {
        self.mem.write_ram(crate::ram_map::SAMUS_SCR_Y, v);
    }

    /// 名表切换计数（`ScrollBlockOnNameTable3`）：横滚奇偶决定写哪张名表。
    pub fn block_nt3(&self) -> u8 {
        self.mem.read_ram(crate::ram_map::SCROLL_BLOCK_NT3)
    }

    pub fn set_block_nt3(&mut self, v: u8) {
        self.mem.write_ram(crate::ram_map::SCROLL_BLOCK_NT3, v);
    }
}

#[cfg(test)]
mod tests_camera {
    use crate::frame::Manager;
    use crate::world::camera::dir;

    #[test]
    fn camera_reads_and_writes_named_addresses() {
        let mut m = Manager::default();
        m.camera().set_scroll_x(0x7F);
        m.camera().set_map_x(5);
        m.camera().set_dir(dir::RIGHT);
        assert_eq!(m.read_ram(crate::ram_map::SCROLL_X), 0x7F);
        assert_eq!(m.camera().map_x(), 5);
        assert_eq!(m.camera().dir(), dir::RIGHT);
    }
}

// ─────────────────────────── P2：`NesMapped` 声明 ───────────────────────────

/// 相机域未 typed 的缺口说明。
const CAM_UNMODELED: &str = "相机域未 typed：滚动/地图位/屏幕位真相在字节面（crates/game-core/src/world/camera/mod.rs:124）";

/// 相机域 `NesMapped` 声明：8 个访问器各 1 B，全部未 typed（0 个 owner）。
///
/// 这 8 B 散在零页（`$0049`/`$004F-$0052`/`$006C`/`$00FC-$00FD`），不在对象/敌域任何槽里——
/// 所以必须显式登记，否则它们就是"没人解释的字节"（覆盖度门的空洞）。
impl<'a> crate::memmap::nesmap::NesMapped for Camera<'a> {
    const DISCRIMINATOR: Option<u16> = None;

    const SPANS: &'static [(NesSpan, SpanKey)] = &[
        (
            NesSpan::value(SCROLL_X, 1),
            SpanKey::Unmodeled(CAM_UNMODELED),
        ),
        (
            NesSpan::value(SCROLL_Y, 1),
            SpanKey::Unmodeled(CAM_UNMODELED),
        ),
        (
            NesSpan::value(SCROLL_DIR, 1),
            SpanKey::Unmodeled(CAM_UNMODELED),
        ),
        (
            NesSpan::value(MAP_POS_X, 1),
            SpanKey::Unmodeled(CAM_UNMODELED),
        ),
        (
            NesSpan::value(MAP_POS_Y, 1),
            SpanKey::Unmodeled(CAM_UNMODELED),
        ),
        (
            NesSpan::value(SAMUS_SCR_X, 1),
            SpanKey::Unmodeled(CAM_UNMODELED),
        ),
        (
            NesSpan::value(SAMUS_SCR_Y, 1),
            SpanKey::Unmodeled(CAM_UNMODELED),
        ),
        (
            NesSpan::value(SCROLL_BLOCK_NT3, 1),
            SpanKey::Unmodeled(CAM_UNMODELED),
        ),
    ];

    /// 0 个 `Owner` 段 ⇒ 无可投影的真相。
    fn project(&self, _out: &mut crate::memmap::storage::NesImage) {}
}
