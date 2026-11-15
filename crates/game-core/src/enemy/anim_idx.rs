//! 敌人动画索引 —— **绑定区的**索引类型（`EnExtra.animIndex / resetAnimIndex`）。
//!
//! # 为什么需要类型
//!
//! `animIndex` 不是全局序号，而是**当前区 `EnAnimTable` 的下标**：同一个字节在
//! Brinstar（`01:9D6A`，118 项）与 Ridley（`05:9B85`，107 项）指向不同动画。
//! 裸 `u8` 在 Rust 里无法阻止「A 区取到的索引拿去查 B 区的表」——这正是
//! `docs/findings/per-bank-assets.md` §3 记录的 P0 缺陷形态（Ridley 的 `$42`
//! 查 bank1 表得 `$FF`，16 次回绕后 `unported!("bank07:E0BC-wrap")`）。
//!
//! 因此：**取索引必须带区**，读表只能经 [`AnimIdx::resolve`]。
//! 本类型刻意 **不实现 `From<u8>`**，也不提供「只给 off 不给 area」的构造器——
//! 想构造就必须显式交代区号，代码评审与编译器一起把「裸 u8 跨区」挡在外面。
//!
//! # 边界
//!
//! RAM 里存的仍然是裸字节（原版就是这样，存档/回读也按字节走），所以
//! 与 RAM 交互处一律经 [`AnimIdx::off`]（写出）与 [`AnimIdx::at`]（读入）。
//! 类型只在 Rust 内部活一段表达式，不改变任何字节。

/// 某区 `EnAnimTable` 的一个下标。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct AnimIdx {
    area: u8,
    off: u8,
}

impl AnimIdx {
    /// 用**显式区号**构造（`area` 取 `InArea & $0F` 的 0..=4）。
    pub(crate) fn at(area: u8, off: u8) -> Self {
        Self { area, off }
    }

    pub(crate) fn area(self) -> u8 {
        self.area
    }

    /// 原始下标字节（与 RAM 中 `animIndex` 同值）。
    pub(crate) fn off(self) -> u8 {
        self.off
    }

    /// **唯一读表入口**：`EnAnimTable(area)[off]`。
    ///
    /// 返回 `None` 表示「不是一条有效动画」：越出该区表长，或表项本身就是 `$FF`
    /// 终止符（asm 里 `$FF` 也让 `animIndex←resetAnimIndex` 续走）。调用方必须
    /// 显式处理，不许静默取 `0xFF` 当帧号——那是旧代码 16 次回绕的来源。
    pub(crate) fn resolve(self) -> Option<u8> {
        let v = *game_data::assets::enemy_anim_table(self.area).get(self.off as usize)?;
        (v != 0xFF).then_some(v)
    }

    /// 前进一格（`inc animIndex` 的语义；区不变）。溢出不回绕到别区，只在本区内 `u8` 回绕。
    pub(crate) fn next(self) -> Self {
        Self { area: self.area, off: self.off.wrapping_add(1) }
    }

    /// 在**本区内**后退 `n` 格（`InitEnActiveAnimIndex` 的 InitOffset 递减）。
    pub(crate) fn back_within_area(self, n: u8) -> Self {
        Self { area: self.area, off: self.off.wrapping_sub(n) }
    }
}

#[cfg(test)]
mod tests_anim_idx {
    use super::*;

    /// 同一个字节在**不同区**解析成不同动画——这条断言就是「裸 u8 跨区用」的
    /// 反例：若解析结果相同，说明拿到的不是按区表（或该区表根本没按区化）。
    #[test]
    fn same_offset_resolves_differently_per_area() {
        if !game_data::assets::is_ready() {
            eprintln!("SKIP（显式）：资产包未加载——先跑 pack-tool extract");
            return;
        }
        let off = 0x42u8; // Ridley 的 Dessgeega 用的那个值
        let ri = AnimIdx::at(4, off).resolve();
        let br = AnimIdx::at(0, off).resolve();
        assert_ne!(ri, br, "area 4 与 area 0 的同一 off 必须解析成不同动画");
        assert!(ri.is_some(), "Ridley 0x42 必须是合法动画（旧 bug 在此返回 0xFF）");
    }

    /// 越界与 `$FF` 表项都必须返回 `None`（不许静默给 `0xFF` 当帧号）。
    #[test]
    fn out_of_range_and_ff_entries_are_none() {
        if !game_data::assets::is_ready() {
            eprintln!("SKIP（显式）：资产包未加载——先跑 pack-tool extract");
            return;
        }
        // Tourian 的 anim 表只有 38 项 ⇒ 索引 200 越界。
        assert_eq!(AnimIdx::at(3, 200).resolve(), None, "越界索引必须 None");
        // Brinstar 表尾部（118 项之后的那些 $FF 终止项）在其中一处必为 None。
        let any_ff = (0..=255u8).any(|o| match AnimIdx::at(0, o).resolve() {
            None => true,
            Some(_) => false,
        });
        assert!(any_ff, "Brinstar 表必须在某些索引上给出 None（越界或 $FF）");
    }

    /// `next` / `back_within_area` 不跨区（类型里的 area 永不改变）。
    #[test]
    fn advancing_never_changes_area() {
        let a = AnimIdx::at(4, 0xFF).next();
        assert_eq!(a.area(), 4, "next 不换区");
        assert_eq!(a.off(), 0x00, "本区内 u8 回绕");
        let b = AnimIdx::at(2, 1).back_within_area(3);
        assert_eq!(b.area(), 2);
        assert_eq!(b.off(), 0xFE);
    }
}
