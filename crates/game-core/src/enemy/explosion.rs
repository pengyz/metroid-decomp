//! 爆炸池：`EnExplosions`（4 槽）。
//!
//! 与敌池/对象池的关键差别：**两半存储步长不同**——
//! 主面 `$04C0 + i*8`（8 B/槽：y/x/.../data05/delay/quantity），
//! 额外面 `$6BB4 + i*16`（16 B/槽：status/radius/anim/hi，与 `EnsExtra` 同布局）。
//! 所以槽号不能套用敌槽的 `x/16`，本模块是唯一做这个换算的地方。
//!
//! status 自成一域（asm 只做 `inc status`：0→1＝占用），**不是** `enemyStatus_*` 七态，
//! 因此刻意不复用 `EnemyStatus`——两套词表合并就会在 0 号槽上互相误判。

use crate::frame::Manager;

/// 主面基址（`EnExplosions.0`）。
pub(crate) const BASE: u16 = 0x04C0;
/// 主面槽步长（8 B）。
pub(crate) const STRIDE: u16 = 8;
/// 槽数（`$04C0-$04DF`）。
pub(crate) const SLOT_COUNT: u8 = 4;
/// 额外面基址（`EnExplosionsExtra.0`）。
pub(crate) const EXTRA_BASE: u16 = 0x6BB4;
/// 额外面槽步长（16 B）。
pub(crate) const EXTRA_STRIDE: u16 = 8;

/// 主面字段偏移（`Ens` 布局的子集，起点 `$04C0`）。
pub(crate) mod expl {
    pub(crate) const Y: u16 = 0;
    pub(crate) const X: u16 = 1;
    pub(crate) const DATA05: u16 = 5;
    pub(crate) const DELAY: u16 = 6;
    pub(crate) const QUANTITY: u16 = 7;
}

/// **额外面步长 = 8**（与主面一致）——`M1_NES_NTSC.sym` 实测：
/// `EnExplosionsExtra.0=$6BB4 / .1=$6BBC / .2=$6BC4 / .3=$6BCC`，`.struct EnExplosionExtra`
/// 只有 8 字节（status/radiusY/radiusX/animFrame/animDelay/resetAnimIndex/animIndex/hi）。
/// 【2026-11-17 真错更正】此前按 16 步长构造句柄 ⇒ 槽 1–3 的 status/anim 全错位
/// （槽 0 因 `0*16==0*8` 恰好正确，故 L4 的 f2152 窗口看不出来——那里只用到槽 0）。
/// 额外面字段偏移（`EnExplosionExtra` 布局，起点 `$6BB4`）。
pub(crate) mod explx {
    pub(crate) const STATUS: u16 = 0;
    pub(crate) const ANIM_FRAME: u16 = 3;
    pub(crate) const ANIM_DELAY: u16 = 4;
    pub(crate) const RESET_ANIM: u16 = 5;
    pub(crate) const ANIM_INDEX: u16 = 6;
    pub(crate) const HI: u16 = 7;
}

/// 槽号 → 敌数组视角的字节偏移（`$00C0 + slot*8`）。
///
/// `UpdateEnemyAnim` / `DrawEnemy` 这些共享例程按「Ens 基址 + 偏移」寻址，
/// 爆炸槽在共享例程眼里就是偏移 `$C0` 起的一段——本函数是唯一的换算点。
pub(crate) fn ens_offset(slot: u8) -> u8 {
    0xC0 + slot * STRIDE as u8
}

/// 一个爆炸槽的借用视图。
pub struct Explosion<'a> {
    pub(crate) mem: &'a mut Manager,
    slot: u8,
}

impl<'a> Manager {
    /// 取爆炸槽句柄。槽号 0..=3，越界是编程错误（fail-loud）。
    pub fn explosion(&mut self, slot: u8) -> Explosion<'_> {
        assert!(slot < SLOT_COUNT, "爆炸槽号越界：{slot}（应为 0..=3）");
        Explosion { mem: self, slot }
    }
}

impl Explosion<'_> {
    #[inline]
    fn addr(&self, off: u16) -> u16 {
        BASE + self.slot as u16 * STRIDE + off
    }

    #[inline]
    fn eaddr(&self, off: u16) -> u16 {
        EXTRA_BASE + self.slot as u16 * EXTRA_STRIDE + off
    }

    /// 池占用标记：`0` = 空槽（`ExplodeEnemy` 找槽只认它）。
    pub fn is_free(&self) -> bool {
        self.mem.read_wram(self.eaddr(explx::STATUS)) == 0
    }

    pub fn status(&self) -> u8 {
        self.mem.read_wram(self.eaddr(explx::STATUS))
    }

    /// `inc status`（asm 原样：0→1＝占用，不映射枚举）。
    pub fn occupy(&mut self) {
        let s = self.status().wrapping_add(1);
        self.mem.write_wram(self.eaddr(explx::STATUS), s);
    }

    /// `RemoveEnemy ; 07:FA18` 的爆炸侧用法：status 清 0。
    pub fn free(&mut self) {
        self.mem.write_wram(self.eaddr(explx::STATUS), 0);
    }

    pub fn delay(&self) -> u8 {
        self.mem.read_ram(self.addr(expl::DELAY))
    }

    pub fn set_delay(&mut self, v: u8) {
        self.mem.write_ram(self.addr(expl::DELAY), v);
    }

    pub fn quantity(&self) -> u8 {
        self.mem.read_ram(self.addr(expl::QUANTITY))
    }

    pub fn set_quantity(&mut self, v: u8) {
        self.mem.write_ram(self.addr(expl::QUANTITY), v);
    }

    pub fn data05(&self) -> u8 {
        self.mem.read_ram(self.addr(expl::DATA05))
    }

    pub fn y(&self) -> u8 {
        self.mem.read_ram(self.addr(expl::Y))
    }

    pub fn x(&self) -> u8 {
        self.mem.read_ram(self.addr(expl::X))
    }

    pub fn set_pos(&mut self, y: u8, x: u8) {
        self.mem.write_ram(self.addr(expl::Y), y);
        self.mem.write_ram(self.addr(expl::X), x);
    }

    pub fn set_y(&mut self, y: u8) {
        self.mem.write_ram(self.addr(expl::Y), y);
    }

    pub fn set_x(&mut self, x: u8) {
        self.mem.write_ram(self.addr(expl::X), x);
    }

    pub fn hi(&self) -> u8 {
        self.mem.read_wram(self.eaddr(explx::HI))
    }

    pub fn set_hi(&mut self, v: u8) {
        self.mem.write_wram(self.eaddr(explx::HI), v);
    }

    /// `InitEnAnimIndex(AreaExplosionAnimIndex)`：reset 与 index 同写、delay 归零。
    pub fn set_anim(&mut self, anim: u8) {
        self.mem.write_wram(self.eaddr(explx::RESET_ANIM), anim);
        self.mem.write_wram(self.eaddr(explx::ANIM_INDEX), anim);
        self.mem.write_wram(self.eaddr(explx::ANIM_DELAY), 0);
    }

    pub fn anim_index(&self) -> u8 {
        self.mem.read_wram(self.eaddr(explx::ANIM_INDEX))
    }
}

#[cfg(test)]
mod tests_explosion_pool {
    use crate::frame::Manager;

    /// 两半存储的**步长相同（都是 8）但基址不同**：槽 1 主面 `$04C8`、额外面 `$6BBC`。
    /// 【2026-11-17 更正】旧版此处断言额外面是 `$6BC4`（16 步长假设）——那是**错的**，
    /// asm `.sym` 实测 `EnExplosionsExtra.1=$6BBC`（槽步长 8，与 `EnExplosionExtra` 8 字节对齐）。
    #[test]
    fn halves_use_different_strides() {
        let mut m = Manager::default();
        m.explosion(1).set_pos(0x50, 0x41);
        m.explosion(1).occupy();
        assert_eq!(m.read_ram(0x04C8), 0x50);
        assert_eq!(m.read_ram(0x04C9), 0x41);
        assert_eq!(m.read_wram(0x6BBC), 1);
        assert!(!m.explosion(1).is_free());
        assert!(m.explosion(0).is_free());
    }

    /// `ens_offset` 是唯一换算点：槽 0 ⇒ `$C0`、槽 3 ⇒ `$D8`。
    #[test]
    fn ens_offset_maps_slots_to_shared_routine_view() {
        assert_eq!(super::ens_offset(0), 0xC0);
        assert_eq!(super::ens_offset(3), 0xD8);
    }

    #[test]
    fn free_clears_status_only() {
        let mut m = Manager::default();
        m.explosion(2).occupy();
        m.explosion(2).set_delay(0x0A);
        m.explosion(2).free();
        assert!(m.explosion(2).is_free());
        assert_eq!(m.explosion(2).delay(), 0x0A, "free 只清 status，不动 delay");
    }
}
