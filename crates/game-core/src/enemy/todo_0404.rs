//! 敌方 $0404 数组域模型（F2CA 族第 2/3 件载体；地址侧=memmap::enemy_todo_0404）。
//!
//! 命名降级（lead 裁定）：语义**未证**——「coords」之说按 `_todo_`+证据注纪律降级
//! （data/naming 台账 N-0005 记 C 级假设：coords 假设待语义考证后按 rename 流程升级）。
//! 触达证据（A 级）：bank07:F2D8 `1D 04 04`/`9D 04 04`（ORA/STA $0404,X）；
//! bank07:F2DF `19 04 04`/`99 04 04`（ORA/STA $0404,Y，LDA zp $10 起手）。
//! 行为（B 级语义概要级）：OR 回写（与 EnemyAttrs::or_attribute 同形——掩码来源由 shim 侧定）。

/// 敌方 $0404 数组（字节偏移契约面；地址域由 memmap::enemy_todo_0404 承载）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnemyTodo0404 {
    entries: [u8; 0x100],
}

impl EnemyTodo0404 {
    /// 从字节偏移契约面构造（memmap codec 解码入口）。
    pub fn new(entries: [u8; 0x100]) -> Self {
        Self { entries }
    }

    /// 字节偏移契约面只读投影（memmap codec 编码/写集推导入口）。
    pub fn entries(&self) -> &[u8; 0x100] {
        &self.entries
    }

    /// OR 回写（F2D8/F2DF 共同行为）：`entries[idx] |= mask`，返回**新值**（=A 终值）。
    ///
    /// 行为证据（B 级语义概要级）：6502 流 `A = A|old; mem = A`（F2D8）与
    /// `A = RAM[$10]|old; mem = A`（F2DF）折叠为同一域行为，掩码来源差异由 shim 侧表达。
    /// 对拍证据：oracle/subroutines/l2_cases.jsonl#gen-f2d8-*/#b-f2d8-* 与
    /// #gen-f2df-*/#b-f2df-* 三方全等。同值写仍是写事件（b-f270-a0 案教训）。
    pub fn or_writeback(&mut self, idx: u8, mask: u8) -> u8 {
        let v = self.entries[idx as usize] | mask;
        self.entries[idx as usize] = v;
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 同值写仍是写事件（L2 契约面）：b-f270-a0 同族教训在 $0404 面复刻自检。
    #[test]
    fn or_writeback_noop_write_is_still_event() {
        let mut entries = [0u8; 0x100];
        entries[5] = 170;
        let before = EnemyTodo0404::new(entries);
        let mut v0404 = before.clone();
        let v = v0404.or_writeback(5, 0); // 同值回写
        assert_eq!(v, 170);
        assert!(crate::memmap::enemy_todo_0404::diff_writes(&before, &v0404).is_empty());
        assert_eq!(crate::memmap::enemy_todo_0404::write_event(5, v), (crate::memmap::enemy_todo_0404::ENEMY_TODO_0404_BASE + 5, 170));
    }
}
