//! 私有存储层（`Manager` 的内层，**未来被 typed struct 替换的那一层**）。
//!
//! 设计契约（r-架构清理，用户裁定 m25964/m26000）：
//! - 本类型只被 `Manager` 持有，**字段不对外暴露**；外部一律经 `Manager::ram()/wram()`
//!   （只读）或域视图（`manager.samus()` 等）访问。
//! - 替换点：当日志域逐步变成真正的 Rust 结构（`struct SamusState { status: SamusStatus, .. }`），
//!   本类型逐域退场；**上层 API 不变**，判据由 `Manager::snapshot_nes()` 的字节投影保证。
//! - 因此这里刻意**不做**任何语义封装（不叫 Ram、不叫 NesBus）：它就是"今天用来装状态的
//!   字节容器"，语义全部在上层实体 `state` 模块。

/// NES 字节容器：CPU 地址空间里我们实际持有的两块可写内存。
///
/// - `ram`：2KB 归一镜像面（`$0000-$07FF`，含 OAM 影子 `$0200`、栈 `$0100`）。
/// - `wram`：8KB 电池备份区（`$6000-$7FFF`，道具位图/房间缓冲等）。
///
/// 不持有 `regs`/建模旗：那些是 `Manager` 自己的字段（它们不是 NES 内存，是模拟器状态）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NesStorage {
    pub(crate) ram: [u8; 0x800],
    pub(crate) wram: [u8; 0x2000],
}

/// NES 字节布局快照（**单向投影**：当前存储 → NES 字节）。
///
/// 只服务对拍/诊断（`ram.hex`/`ppu.hex` 腿与 `--state-out`），不是第二个真相源；
/// 域 typed 化后，这就是"状态 → NES 字节"的唯一出口，据此可机械化证明"换实现无行为变化"。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NesImage {
    pub ram: [u8; 0x800],
    pub wram: [u8; 0x2000],
}

impl Default for NesStorage {
    fn default() -> Self {
        Self { ram: [0; 0x800], wram: [0; 0x2000] }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::Manager;

    #[test]
    fn snapshot_nes_projects_ram_and_wram_bytes() {
        let mut m = Manager::default();
        m.write_ram(0x0300, 9);
        m.write_wram(0x6AF4 + 0x0F, 0x40);
        let img = m.snapshot_nes();
        assert_eq!(img.ram[0x0300], 9, "RAM 字节投影：$0300 应等于写入值");
        assert_eq!(img.wram[0x6AF4 + 0x0F - 0x6000], 0x40, "WRAM 字节投影：$6B03 → 索引 $0B03");
        assert_eq!(img.ram.len(), 0x800);
        assert_eq!(img.wram.len(), 0x2000);
    }

    #[test]
    fn snapshot_nes_is_a_projection_not_a_second_truth() {
        let mut m = Manager::default();
        m.ram_mut()[0x0106] = 0x20;
        m.ram_mut()[0x0107] = 0x01;
        assert_eq!(m.samus().health(), 0x0120, "域视图读的是同一份存储");
        let img = m.snapshot_nes();
        assert_eq!((img.ram[0x0106], img.ram[0x0107]), (0x20, 0x01));
    }

    #[test]
    fn nes_storage_is_private_to_manager() {
        // 编译期契约：外部拿不到 NesStorage 字段，只能走 ram()/wram()/snapshot_nes()。
        // 【2026-11-17 更正】旧版只在全零 Manager 上断言 `snapshot_nes().ram == *ram()`，
        // 那是一条**空断言**（typed 字段一旦非零该等式即为假）。真相是：
        // 字节面**不**持有 typed 字段（`write_ram` 对 typed 地址提前 return），
        // 该 22 字节的唯一真相在 `Manager.samus`，`snapshot_nes()` 才是字节面。
        let mut m = Manager::default();
        m.ram_mut()[0x0200] = 0xAA; // 非 typed 面
        m.samus().set_status(crate::samus::state::SamusStatus::Roll);
        let img = m.snapshot_nes();
        assert_eq!(img.ram[0x0200], 0xAA, "非 typed 面直通");
        assert_eq!(img.ram[0x0300], 3, "typed 字段由投影产出");
        assert_eq!(m.read_ram(0x0300), 3, "读口与投影同源");
        assert_eq!(m.ram()[0x0300], 0, "字节面不持有 typed 字段（分层的代价，显式锁定）");
    }

    /// **写口双写防线**：`write_ram` 命中 typed 面时必须提前 return，
    /// 不得"既写 typed 又写字节面"——否则字节面会悄悄变成第二真相源
    /// （读径仍走 typed、`snapshot_nes` 仍用 typed 覆盖，所有既有测试都会绿）。
    #[test]
    fn write_path_does_not_mirror_typed_bytes_into_storage() {
        let mut m = Manager::default();
        m.write_ram(0x0300, 3); // typed：status
        m.write_ram(0x0308, 0xFE); // typed：speedY（i8 -2）
        m.write_wram(0x6AF4, 5); // typed：敌槽 0 的 status（0..=6 才是 typed 面）
        assert_eq!(m.read_wram(0x6AF4), 5);
        assert_eq!(m.read_ram(0x0300), 3);
        assert_eq!(m.ram()[0x0300], 0, "write_ram 写 typed 后，字节面不得留副本");
        assert_eq!(m.ram()[0x0308], 0, "同上（速度面）");
        assert_eq!(m.wram()[0x6AF4 - 0x6000], 0, "write_wram 同病同查（$6AF4+14 是 type，不在 typed 面，故不查）");
    }

    /// 2KB 镜像别名不得绕过 typed 面：`$0B00` 与 `$0300` 是同一格。
    #[test]
    fn mirrored_alias_does_not_bypass_typed() {
        let mut m = Manager::default();
        m.write_ram(0x0B00, 3); // 镜像地址写
        assert_eq!(m.read_ram(0x0300), 3, "镜像写必须落到 typed 真相面");
        assert_eq!(m.read_ram(0x0B00), 3, "镜像读同源");
        assert_eq!(m.snapshot_nes().ram[0x0300], 3, "投影同步");
    }
}
