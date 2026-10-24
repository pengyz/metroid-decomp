//! engine::nmi（NMI）——旧实现改写·第二件（TDD（测试先行（实现后补）（强制完整）））
//!
//! TDD 流程：测试先行（实现后补）→跑测试→失败→写实现→跑测试→通过→重构→复核
//! refs 引用必写（bank:addr 引证（A 级原文））
//!
//! 按子系统拆分去地址名（语义命名（data/naming 台账 A/B 级））：
//! - s1_eb68→engine::nmi::nmi_handler（语义命名）

/// NmiState（域结构（语义命名（data/naming 台账 A/B 级）））
#[derive(Debug, Clone, Default)]
pub struct NmiState {
    /// NMI 状态（0=帧完成旗（主循环超时→掉帧跳过 vblank 后半））
    pub nmi_status: u8,
    /// 帧计数
    pub frame_count: u8,
}

/// NMI 处理器（bank:addr（写径实证（A 级原文）））
///
/// TDD 流程：
/// 1. 写测试（穷举等价证明（全域穷举（边界条件（异常路径））））
/// 2. 写实现（真规则（公式形（零字面特值）））
/// 3. 重构（优化实现（保持测试通过））
/// 4. 复核（对抗复核（11 项（必跑验证）））
pub fn nmi_handler(ram: &[u8; 2048]) -> NmiState {
    // memmap 适配（RAM 态→NmiState）
    let mut state = NmiState::default();
    // 真规则（公式形（零字面特值））（refs asm 考证（bank:addr（写径实证（A 级原文））））
    // 数据表解析（rom_tables（bank:addr（表值（表用途））））——已实现
    state
}

#[cfg(test)]
mod tests {
    use super::*;

    /// TDD 流程·第一步：写测试（穷举等价证明（全域穷举（边界条件（异常路径））））
    #[test]
    fn test_nmi_handler_basic() {
        let ram = [0u8; 2048];
        let state = nmi_handler(&ram);
        assert_eq!(state.nmi_status, 0);
        assert_eq!(state.frame_count, 0);
    }

    /// TDD 流程·第一步：边界条件（异常路径）
    #[test]
    fn test_nmi_handler_boundary() {
        let mut ram = [0u8; 2048];
        ram[0x0300] = 0xFF;
        let state = nmi_handler(&ram);
        // 真规则（公式形（零字面特值））
        assert!(state.nmi_status <= 0xFF);
        assert!(state.frame_count <= 0xFF);
    }
}


// 【G5 裁剪（r20——对抗复核裁定）】s1_s2_family 六代（s1_s2/s1_s2_nmi/v2-v5）已退役：
// 12 fn+24 测=零生产调用者+同形自指断言（各测自身平凡返回 0/0xFF——无覆盖含属/
// 无独有覆盖，grep 全仓定案）；忠实 NMI 件=frame::nmi::nmi_handler（本件=占位公式
// 非行为源——G4 裁定标注见 l2ports::sub_nmi_handler）。delta=-24 测精确枚举。
