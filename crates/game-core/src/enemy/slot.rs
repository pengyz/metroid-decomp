//! 敌槽 16B 状态块域模型（typed view 的语义侧；地址侧=memmap::enemy_slots）。
//!
//! 命名证据（data/naming 台账 N-0001/N-0002）：域几何=WRAM $6AF4-$6BF3（16B×16，
//! 0x6B02-0x6AF4=0x0E=slot 内 offset 14 自证）；字段语义**未证**——触达证据随字段注
//! （A 级地址自证），字段名 `_todo_` 纪律保留（AGENTS §3 禁伪语义命名，lead 裁定宁缺勿滥）。

/// 敌槽状态块（16B）。槽语义视图与字节偏移契约面经 memmap::enemy_slots codec 互转。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnemySlot {
    /// 触达证据（A 级）：bank07:F28E `BD F4 6A` LDA $6AF4,X 后 `C9 04` CMP #$04——语义未证（C 级）。
    pub _todo_00: u8,
    /// 未触达（F2CA 族窗口内无引用）——语义未证。
    pub _todo_01: u8,
    /// 未触达（F2CA 族窗口内无引用）——语义未证。
    pub _todo_02: u8,
    /// 未触达（F2CA 族窗口内无引用）——语义未证。
    pub _todo_03: u8,
    /// 未触达（F2CA 族窗口内无引用）——语义未证。
    pub _todo_04: u8,
    /// 未触达（F2CA 族窗口内无引用）——语义未证。
    pub _todo_05: u8,
    /// 未触达（F2CA 族窗口内无引用）——语义未证。
    pub _todo_06: u8,
    /// 未触达（F2CA 族窗口内无引用）——语义未证。
    pub _todo_07: u8,
    /// 未触达（F2CA 族窗口内无引用）——语义未证。
    pub _todo_08: u8,
    /// 未触达（F2CA 族窗口内无引用）——语义未证。
    pub _todo_09: u8,
    /// 未触达（F2CA 族窗口内无引用）——语义未证。
    pub _todo_0a: u8,
    /// 未触达（F2CA 族窗口内无引用）——语义未证。
    pub _todo_0b: u8,
    /// 未触达（F2CA 族窗口内无引用）——语义未证。
    pub _todo_0c: u8,
    /// 未触达（F2CA 族窗口内无引用）——语义未证。
    pub _todo_0d: u8,
    /// 触达证据（A 级）：bank07:F295 `BD 02 6B` LDA $6B02,X → `8D 0F 01` STA $010F / `A8` TAY——语义未证（C 级）。
    pub _todo_0e: u8,
    /// 未触达（F2CA 族窗口内无引用）——语义未证。
    pub _todo_0f: u8,
}

impl EnemySlot {
    /// 槽字节宽（16B 步长，memmap 侧常量同源互检）。
    pub const SIZE: usize = 16;

    /// 字节偏移口径 → 槽视图。
    pub fn from_bytes(b: [u8; Self::SIZE]) -> Self {
        Self {
            _todo_00: b[0],
            _todo_01: b[1],
            _todo_02: b[2],
            _todo_03: b[3],
            _todo_04: b[4],
            _todo_05: b[5],
            _todo_06: b[6],
            _todo_07: b[7],
            _todo_08: b[8],
            _todo_09: b[9],
            _todo_0a: b[10],
            _todo_0b: b[11],
            _todo_0c: b[12],
            _todo_0d: b[13],
            _todo_0e: b[14],
            _todo_0f: b[15],
        }
    }

    /// 槽视图 → 字节偏移口径。
    pub fn to_bytes(self) -> [u8; Self::SIZE] {
        [
            self._todo_00,
            self._todo_01,
            self._todo_02,
            self._todo_03,
            self._todo_04,
            self._todo_05,
            self._todo_06,
            self._todo_07,
            self._todo_08,
            self._todo_09,
            self._todo_0a,
            self._todo_0b,
            self._todo_0c,
            self._todo_0d,
            self._todo_0e,
            self._todo_0f,
        ]
    }
}
