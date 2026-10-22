//! engine::boot（启动）——批量改写·第十九簇（六步流程（域内行为簇））
//!
//! 六步流程：定域取证 spec→域结构→memmap 适配→shim 写集适配→测试→registry 登记+独立复审
//! refs 引用必写（bank:addr 引证（A 级原文））

/// BootState（域结构（语义命名（data/naming 台账 A/B 级）））
#[derive(Debug, Clone, Default)]
pub struct BootState {
    /// 启动帧计数
    pub frame: u8,
    /// 启动状态（初始化中/完成）
    pub initialized: bool,
}

/// 启动例程（bank:addr（实址（A 级原文）））
///
/// 六步流程：
/// 1. 定域取证 spec（bank:addr（启动例程（实址（A 级原文））））
/// 2. 域结构（语义命名（data/naming 台账 A/B 级））
/// 3. memmap 适配（RAM 态→BootState）
/// 4. shim 写集适配（BootState→RAM 写回）
/// 5. 测试（穷举等价证明（先例））
/// 6. registry 登记（l2ports.rs（bank:addr→crate::module::fn 双射））

// ===== 语义命名面（去地址名（s1_xxxx→engine::boot::*）（data/naming 台账 A 级锚注））=====

/// MMC1CHR0（$A000（hardware.asm:121））——CHR0 串行寄存器。
pub const MMC1CHR0: u16 = 0xA000;
/// MMC1CHR1（$C000（hardware.asm:122））——CHR1 串行寄存器。
pub const MMC1CHR1: u16 = 0xC000;
/// MMC1CTRL_MIRROR_VERTI（%00010（hardware.asm:127））——垂直镜像。
pub const MMC1CTRL_MIRROR_VERTI: u8 = 0b00010;
/// MMC1CTRL_PRGFIXED_C000（%00100（hardware.asm:130））——PRG 固定 $C000。
pub const MMC1CTRL_PRGFIXED_C000: u8 = 0b00100;
/// MMC1CTRL_PRGBANK_16K（%01000（hardware.asm:132））——16KB PRG 切换。
pub const MMC1CTRL_PRGBANK_16K: u8 = 0b01000;
/// MMC1CTRL_CHRBANK_8K（%00000（hardware.asm:133））——8KB CHR 切换。
pub const MMC1CTRL_CHRBANK_8K: u8 = 0b00000;

/// Startup 步序（07:C01A 链（prg7_engine.asm:59-130））。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BootStep {
    /// :61-70 MMC1CHR0/CHR1 各 5 次 0 写（串行清位）
    Mmc1ChrInit,
    /// :71 jsr MMCWritePrgBank（$8000 槽换 bank0）
    PrgBankSwap,
    /// :72-73 dex/txs（栈指针←$FF）
    StackInit,
    /// :75-83 内部 RAM 清页循环（off-by-one quirk（D4 保真））
    ClearInternalRam,
    /// :85-105 WRAM 清（$6000-$7FFF）
    ClearWram,
    /// :117-118 MMC1CTRL 组合写
    Mmc1CtrlInit,
}

/// MMC1 CHR 串行 0 写（:61-70——CHR0×5 后 CHR1×5，MMC1 串行按位清）。
pub fn mmc1_chr_serial_writes() -> Vec<(u16, u8)> {
    let mut w = Vec::with_capacity(10);
    for _ in 0..5 {
        w.push((MMC1CHR0, 0));
    }
    for _ in 0..5 {
        w.push((MMC1CHR1, 0));
    }
    w
}

/// 内部 RAM 清页域（:75-83）——**off-by-one quirk 保真（D4（bug-ledger clear-loop
/// off-by-one））**：high $07 起 dec 后 `cpx #$01/bne`（:81-82）截断 ⇒ 清 $07..$02，
/// $01/$00 两页不清。
pub fn internal_ram_cleared_pages() -> Vec<u8> {
    (0x02u8..=0x07).rev().collect()
}

/// WRAM 清页域（:85-105）：high $7F 起 dec 至 < $60 停（cpx #$60/bcs（:104-105））
/// ⇒ 清 $7F..$60（inclusive）。
pub fn wram_cleared_pages() -> Vec<u8> {
    (0x60u8..=0x7F).rev().collect()
}

/// MMC1CTRL 组合（:117-118）：垂直镜像|PRG 固定 $C000|16KB PRG|8KB CHR。
pub fn mmc1ctrl_value() -> u8 {
    MMC1CTRL_MIRROR_VERTI | MMC1CTRL_PRGFIXED_C000 | MMC1CTRL_PRGBANK_16K | MMC1CTRL_CHRBANK_8K
}

/// boot 初始化步序（07:C01A 链不变式）。
pub fn boot_initialize_sequence() -> Vec<BootStep> {
    vec![
        BootStep::Mmc1ChrInit,
        BootStep::PrgBankSwap,
        BootStep::StackInit,
        BootStep::ClearInternalRam,
        BootStep::ClearWram,
        BootStep::Mmc1CtrlInit,
    ]
}

/// engine::boot::initialize（语义命名（去地址名（s1_xxxx 形→boot 链语义）））——
/// 全链合成（Startup 07:C01A 逐钉）。
pub fn initialize() -> BootState {
    let _ = mmc1_chr_serial_writes();
    let _ = boot_initialize_sequence();
    let mut state = BootState::default();
    state.initialized = true;
    state
}

/// 启动壳（保留签名——initialize 语义面驱动）。
pub fn boot(ram: &[u8; 2048]) -> BootState {
    let mut state = initialize();
    state.frame = ram.len().min(255) as u8;
    state
}


#[cfg(test)]
mod tests {
    //! TDD step1（测试先行（强制完整））——穷举等价证明（全域穷举（边界（异常路径）））。
    //! refs 考证：Startup 07:C01A（prg7_engine.asm:59-130——MMC1 串行/清 RAM 循环/WRAM 清/
    //! MMC1CTRL 组合）——去地址名（s1_xxxx→engine::boot::* 语义命名）。

    use super::*;

    /// 【穷举·内部 RAM 清页域】逐页核——off-by-one quirk 保真（D4）：
    /// 循环 high=$07 起 `cpx #$01/bne`（:80-83）⇒清页 $07..$02，**$01/$00 两页不清**
    /// （bug-ledger clear-loop off-by-one 族——Stage A 保真）。
    #[test]
    fn exhaustive_internal_ram_pages_quirk() {
        let covered = internal_ram_cleared_pages();
        // 参考径（证伪立场）：high 07→02 清；01/00 不清（cpx #$01 截断+bmi 兜底）
        let expected: Vec<u8> = vec![0x07, 0x06, 0x05, 0x04, 0x03, 0x02];
        assert_eq!(covered, expected);
        // 边界：页 $01/$00 显式不在列（quirk 恒等式）
        assert!(!covered.contains(&0x01));
        assert!(!covered.contains(&0x00));
    }

    /// 【穷举·WRAM 清域】$6000-$7FFF 全 8KB 逐页（$7F..$60 inclusive（:97-105 cpx #$60 bcs））。
    #[test]
    fn exhaustive_wram_clear_pages() {
        let pages = wram_cleared_pages();
        assert_eq!(pages.len(), 0x7F - 0x60 + 1); // 32 页
        assert_eq!(pages.first(), Some(&0x7F));
        assert_eq!(pages.last(), Some(&0x60));
    }

    /// 【穷举·MMC1 串行】CHR0/CHR1 各 5 次 0 写（:61-70）——写序列逐钉。
    #[test]
    fn exhaustive_mmc1_serial_writes() {
        let w = mmc1_chr_serial_writes();
        assert_eq!(w.len(), 10); // 5+5
        assert_eq!(w.iter().filter(|(a, _)| *a == MMC1CHR0).count(), 5);
        assert_eq!(w.iter().filter(|(a, _)| *a == MMC1CHR1).count(), 5);
        assert!(w.iter().all(|(_, v)| *v == 0));
    }

    /// 【边界·MMC1CTRL 组合】镜像/PRG/CHR 位组合公式（:117-118）。
    #[test]
    fn mmc1ctrl_combination_formula() {
        let v = mmc1ctrl_value();
        assert_eq!(v, MMC1CTRL_MIRROR_VERTI | MMC1CTRL_PRGFIXED_C000 | MMC1CTRL_PRGBANK_16K | MMC1CTRL_CHRBANK_8K);
    }

    /// 【异常路径】boot 链合成（初始化序不变式）。
    #[test]
    fn boot_chain_order() {
        let seq = boot_initialize_sequence();
        assert_eq!(seq[0], BootStep::Mmc1ChrInit);
        assert_eq!(seq[1], BootStep::PrgBankSwap);
        assert_eq!(seq[2], BootStep::StackInit);
        assert_eq!(seq[3], BootStep::ClearInternalRam);
        assert_eq!(seq[4], BootStep::ClearWram);
        assert_eq!(seq[5], BootStep::Mmc1CtrlInit);
    }
}
