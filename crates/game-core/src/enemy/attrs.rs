//! 敌属性数组域模型（F2CA 族首方法载体；地址侧=memmap::enemy_attrs）。
//!
//! 命名证据（data/naming 台账 N-0003/N-0004）：语义=B 级**语义概要级**——F2CA 族切片考证
//! 「ORA/STA $030A,X 自或回写=属性掩码叠加」（decode-sweep r1 §3 + l2ports sub_F270 参考件）；
//! 地址自证（A 级）：bank07:F270 `1D 0A 03` ORA $030A,X / `9D 0A 03` STA $030A,X。
//! 字节偏移契约面：L2 已证任意 X（gen-f270-*/b-f270-* 三方全等），索引为语义槽偏移非地址。

/// 敌属性数组（字节偏移契约面；地址域由 memmap::enemy_attrs 承载）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnemyAttrs {
    entries: [u8; 0x100],
}

impl EnemyAttrs {
    /// 从字节偏移契约面构造（memmap codec 解码入口）。
    pub fn new(entries: [u8; 0x100]) -> Self {
        Self { entries }
    }

    /// 字节偏移契约面只读投影（memmap codec 编码/写集推导入口）。
    pub fn entries(&self) -> &[u8; 0x100] {
        &self.entries
    }

    /// 属性掩码 OR 回写（F270 语义）：`entries[idx] |= mask`，返回**新属性值**。
    ///
    /// 语义证据（B 级语义概要级）：bank07:F270 ORA/STA $030A,X 自或回写——
    /// 6502 流为 `A = A|old; mem = A`，域语义折叠为单向掩码叠加，返回值即 A 终值。
    /// 对拍证据：oracle/subroutines/l2_cases.jsonl#gen-f270-0..7 + #b-f270-* 三方全等。
    pub fn or_attribute(&mut self, idx: u8, mask: u8) -> u8 {
        let v = self.entries[idx as usize] | mask;
        self.entries[idx as usize] = v;
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 同值写仍是写事件（L2 契约面）：b-f270-a0 案教训——纯值 diff 推导漏同值 STA。
    #[test]
    fn or_attribute_noop_write_is_still_event() {
        let mut entries = [0u8; 0x100];
        entries[5] = 170;
        let before = EnemyAttrs::new(entries);
        let mut attrs = before.clone();
        let v = attrs.or_attribute(5, 0); // 同值回写
        assert_eq!(v, 170);
        // 值变化面：无变化
        assert!(crate::memmap::enemy_attrs::diff_writes(&before, &attrs).is_empty());
        // 写事件面：事件存在且经 memmap 映射（0x030A+5=0x030F）
        assert_eq!(crate::memmap::enemy_attrs::write_event(5, v), (0x030F, 170));
    }
}
