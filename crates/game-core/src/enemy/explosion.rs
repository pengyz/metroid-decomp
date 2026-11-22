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
use crate::memmap::nesmap::{NesSpan, SpanKey};

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

    #[inline]
    fn extra_read(&self, off: u16) -> u8 {
        self.mem.read_wram(self.eaddr(off))
    }

    #[inline]
    fn extra_write(&mut self, off: u16, v: u8) {
        self.mem.write_wram(self.eaddr(off), v);
    }

    /// 动画四元组整读（与 `write_anim` 配对）。
    pub(crate) fn anim(&self) -> crate::enemy::state::Anim {
        crate::enemy::state::Anim {
            frame: self.extra_read(explx::ANIM_FRAME),
            delay: self.extra_read(explx::ANIM_DELAY),
            reset: self.extra_read(explx::RESET_ANIM),
            index: self.extra_read(explx::ANIM_INDEX),
        }
    }

    pub(crate) fn write_anim(&mut self, a: crate::enemy::state::Anim) {
        self.extra_write(explx::ANIM_FRAME, a.frame);
        self.extra_write(explx::ANIM_DELAY, a.delay);
        self.extra_write(explx::RESET_ANIM, a.reset);
        self.extra_write(explx::ANIM_INDEX, a.index);
    }

    /// 爆炸池的 `UpdateEnemyAnim` 侧（**无 Pickup 门**——池 status 是"占用"词表）。
    pub(crate) fn advance_anim(&mut self, table: &[u8], a: u8) -> bool {
        match self.anim().step(table, a) {
            crate::enemy::state::AnimStep::Tick(d) => {
                self.extra_write(explx::ANIM_DELAY, d);
                false
            }
            crate::enemy::state::AnimStep::Advance { delay, frame, index } => {
                let reset = self.extra_read(explx::RESET_ANIM);
                self.write_anim(crate::enemy::state::Anim { frame, delay, reset, index });
                true
            }
            crate::enemy::state::AnimStep::Broken { idx, reset, len } => {
                {
                    crate::gap::record("bank07:E0BC-wrap");
                    panic!("unported @ bank07:E0BC-wrap idx={idx:02X} reset={reset:02X} len={len}");
                }
            }
        }
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


#[cfg(test)]
mod tests_pool_separation {
    use super::*;
    use crate::frame::Manager;

    /// **爆炸槽 1–3 的动画面必须读写自己的额外面**（`$6BBC`/`$6BC4`/`$6BCC`，8 步长）。
    /// 旧形按 16 步长构造句柄 ⇒ 槽 0 因 `0*16==0*8` 恰好对，槽 1 起全错位；此测是那次
    /// 真错的回归钉（`b5c34f18` 更正 EXTRA_STRIDE 16→8 的后续覆盖）。
    #[test]
    fn each_slot_animates_its_own_extra_face() {
        for slot in 0..SLOT_COUNT {
            let mut m = Manager::default();
            m.explosion(slot).occupy();
            m.explosion(slot).set_anim(0x00); // resetAnim/animIndex 同写、delay 归零
            // A=1 ⇒ 第一跳走表，animFrame 取表值、animIndex 自增
            crate::enemy::anim_tick::update_explosion_anim(&mut m, slot, 1);
            let base = EXTRA_BASE + slot as u16 * EXTRA_STRIDE;
            assert_eq!(
                m.read_wram(base + explx::ANIM_DELAY),
                1,
                "槽 {slot} 的 animDelay 必须写在自己的额外面（{base:#06X}）"
            );
            assert!(
                m.read_wram(base + explx::ANIM_FRAME) != 0xFF,
                "槽 {slot} 的 animFrame 必须被走表推进"
            );
        }
    }

    /// 两池步长不同这件事本身就是最容易写错的地方：敌槽 16、爆炸槽 8。
    #[test]
    fn explosion_stride_differs_from_enemy_stride() {
        assert_eq!(STRIDE, 8);
        assert_eq!(EXTRA_STRIDE, 8, "额外面与主面同步长（asm EnExplosionExtra 8 字节）");
        assert_ne!(STRIDE, crate::enemy::state::ENEMY_STRIDE, "敌池 16 / 爆炸池 8");
    }
}

// ─────────────────────────── P2：`NesMapped` 声明 ───────────────────────────

/// 爆炸主面未 typed 的缺口说明。
const EXPL_MAIN_UNMODELED: &str = "爆炸主面未 typed：y/x/data05/delay/quantity 真相在字节面（crates/game-core/src/enemy/explosion.rs:53）";
/// 爆炸额外面未 typed 的缺口说明。
const EXPL_EXTRA_UNMODELED: &str =
    "爆炸额外面未 typed：status/animFrame/animDelay/resetAnimIndex/animIndex/hi 真相在字节面（crates/game-core/src/enemy/explosion.rs:53）";

/// 爆炸域两面（`$04C0-$04DF` 主面 + `$6BB4-$6BD3` 额外面，各 4 槽 × 8 B）的声明。
///
/// 全部未 typed（0 个 owner）——与 `typed-domain-migration` 的 I-1 结论一致：爆炸池
/// 是**故意**留在字节面的域（`Explosion` 句柄只做读写视图）。
impl<'a> crate::memmap::nesmap::NesMapped for Explosion<'a> {
    const DISCRIMINATOR: Option<u16> = None;

    const SPANS: &'static [(NesSpan, SpanKey)] = &[
        (
            NesSpan::value(BASE, SLOT_COUNT as u16 * STRIDE),
            SpanKey::Unmodeled(EXPL_MAIN_UNMODELED),
        ),
        (
            NesSpan::value(EXTRA_BASE, SLOT_COUNT as u16 * EXTRA_STRIDE),
            SpanKey::Unmodeled(EXPL_EXTRA_UNMODELED),
        ),
    ];

    /// 0 个 `Owner` 段 ⇒ 无可投影的真相（爆炸池的真相在字节面）。
    fn project(&self, _out: &mut crate::memmap::storage::NesImage) {}
}
