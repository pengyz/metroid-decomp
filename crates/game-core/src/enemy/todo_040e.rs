//! 敌方 $040E 数组域模型（F2CA 族第 5 件载体；地址侧=memmap::enemy_todo_040e）。
//!
//! 命名降级（lead 裁定）：语义**未证**——「state/mode」之说按 `_todo_`+证据注纪律降级
//! （data/naming 台账 N-0007 记 C 级假设）。
//! 触达证据（A 级）：bank07:F2AD `A9 81 9D 0E 04`（LDA #$81 / STA $040E,X）；
//! bank07:F2CF `9D 0E 04`（STA $040E,X ← RAM[$0300+Y]，F2CA 流）。
//! 行为（B 级语义概要级）：直写覆写（非 OR——与 todo_0404 成对偶）。

/// 敌方 $040E 数组（字节偏移契约面；地址域由 memmap::enemy_todo_040e 承载）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnemyTodo040E {
    entries: [u8; 0x100],
}

impl EnemyTodo040E {
    /// 从字节偏移契约面构造（memmap codec 解码入口）。
    pub fn new(entries: [u8; 0x100]) -> Self {
        Self { entries }
    }

    /// 字节偏移契约面只读投影（memmap codec 编码/写集推导入口）。
    pub fn entries(&self) -> &[u8; 0x100] {
        &self.entries
    }

    /// 直写覆写（F2AD/F2CA 行为）：`entries[idx] = v`，返回写入值（=A 终值面）。
    ///
    /// 行为证据（B 级语义概要级）：6502 流 `STA $040E,X`（F2AD 常量 $81；F2CA ← RAM[$0300+Y]）
    /// ——覆写非或。对拍证据：oracle/subroutines/l2_cases.jsonl#gen-f2ad-*/#b-f2ad-*。
    pub fn set_entry(&mut self, idx: u8, v: u8) -> u8 {
        self.entries[idx as usize] = v;
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 覆写语义 + 写事件映射自检。
    #[test]
    fn set_entry_overwrites_and_maps_event() {
        let mut entries = [0u8; 0x100];
        entries[5] = 0xFF;
        let mut e040e = EnemyTodo040E::new(entries);
        let v = e040e.set_entry(5, 0x81);
        assert_eq!(v, 0x81);
        assert_eq!(e040e.entries()[5], 0x81); // 覆写非或（0xFF→0x81）
        assert_eq!(crate::memmap::enemy_todo_040e::write_event(5, v), (crate::memmap::enemy_todo_040e::ENEMY_TODO_040E_BASE + 5, 0x81));
    }
}
