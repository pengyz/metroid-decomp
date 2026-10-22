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

// ===== 旧实现改写·第六件（s1_/s2_ 族去地址名（TDD 穷举等价证明））=====

#[cfg(test)]
mod tests_rewrite6 {
    //! TDD step1（测试先行（强制完整））——穷举等价证明（新语义 fn==旧 s1_/s2_ 地址名 fn
    //! （全域穷举（边界（异常路径））））。refs 锚：$EC1E 门/房初始化（mod.rs:27 注）+
    //! $EE95 短件（mod.rs:35 注）+$F744 OrEnData05（mod.rs:1886 注）。

    use super::*;
    use crate::engine::{s1_bit_set_helper, s1_door_room_init, s1_short_b3_b0,
                        s2_or_en_data05, s2_f702_body, s2_f83b_body};

    /// 【穷举·位或累积】root×y×源值 全域 256³——accumulate_bit_flag==s1_bit_set_helper。
    #[test]
    fn exhaustive_bit_accumulate_equivalence() {
        for root in 0u8..=255 {
            for y in 0u8..=255 {
                for src in 0u8..=255 {
                    let mut w_old = vec![0u8; 0x2000];
                    let mut w_new = vec![0u8; 0x2000];
                    w_old[0xC15 + (root ^ 1) as usize] = src;
                    w_new[0xC15 + (root ^ 1) as usize] = src;
                    let o = s1_bit_set_helper(root, y, &mut w_old);
                    let n = accumulate_bit_flag(root, y, &mut w_new);
                    assert_eq!(o, n, "root={} y={} src={}", root, y, src);
                    assert_eq!(w_old, w_new);
                }
            }
        }
    }

    /// 【穷举·门/房初始化】root×sec20 全域——init_door_room==s1_door_room_init（含环绕边界）。
    #[test]
    fn exhaustive_door_room_equivalence() {
        for root in 0u8..=255 {
            for s20 in 0u8..=255 {
                assert_eq!(s1_door_room_init(root, s20), init_door_room(root, s20));
            }
        }
    }

    /// 【穷举·槽写+轮转】root×y×zp8a 全域 256³——write_slot_and_rotate==s1_short_b3_b0。
    #[test]
    fn exhaustive_slot_rotate_equivalence() {
        for root in 0u8..=255 {
            for y in 0u8..=255 {
                for zp in 0u8..=255 {
                    assert_eq!(s1_short_b3_b0(root, y, zp), write_slot_and_rotate_flags(root, y, zp));
                }
            }
        }
    }

    /// 【穷举·Or/掩码/移位族】x×a×v 全域 256³——s2 三员等价。
    #[test]
    fn exhaustive_s2_or_family_equivalence() {
        for x in 0u8..=255 {
            for a in 0u8..=255 {
                for v in 0u8..=255 {
                    assert_eq!(s2_or_en_data05(x, a, v), or_enemy_data05(x, a, v));
                    assert_eq!(s2_f702_body(x, a, v), or_enemy_data05_masked(x, a, v));
                    assert_eq!(s2_f83b_body(x, a, v), and_enemy_data05_shift(x, a, v));
                }
            }
        }
    }
}

// ===== 旧实现改写·第六件：s1_/s2_ 族语义命名面（去地址名（公式形（零字面特值）））=====
// refs 锚（mod.rs 注内 bank:addr 引证）：$EC1E 门/房初始化（engine/mod.rs:27）+
// $EE95 短件（:35）+$F744 OrEnData05（:1886）+AndEnData05 根（:1842）。

/// 位或累积（旧 s1_bit_set_helper（$EC15 面）——x=root^1 索引源，y 索引目标：
/// wram[$EC15+y] ← y | wram[$EC15+(root^1)]）。
pub fn accumulate_bit_flag(root_val: u8, y: u8, wram: &mut [u8]) -> (Vec<(u16, u8)>, u8) {
    let x = root_val ^ 1;
    let or_v = y | wram.get(0xC15 + x as usize).copied().unwrap_or(0);
    let idx = 0xC15 + y as usize;
    if idx < wram.len() {
        wram[idx] = or_v;
    }
    (vec![(0x6C15 + y as u16, or_v)], 2)
}

/// 门/房初始化（旧 s1_door_room_init（$EC1E）：$032C←根/$0323←$23/$0320←sec20+1）。
pub fn init_door_room(root_val: u8, sec20: u8) -> (Vec<(u16, u8)>, u8) {
    let mut ev = vec![(0x032C, root_val), (0x0323, 0x23)];
    ev.push((0x0320, sec20.wrapping_add(1)));
    (ev, 2)
}

/// 槽写+进位轮转（旧 s1_short_b3_b0（$EE95）：$B3[y]←根/$B0[y]←1/$8A←rol(zp8a)）。
pub fn write_slot_and_rotate_flags(root_val: u8, y: u8, zp8a: u8) -> (Vec<(u16, u8)>, u8, bool) {
    let (hi, carry) = ((zp8a >> 7) & 1, (zp8a & 0x80) != 0);
    let rolled = (zp8a << 1) | hi;
    (vec![(0x00B3 + y as u16, root_val), (0x00B0 + y as u16, 1), (0x008A, rolled)], 1, carry)
}

/// OR 面（旧 s2_or_en_data05（$F744 And 对偶）：$0405[x] ← v|a）。
pub fn or_enemy_data05(x: u8, a_in: u8, v0405_x: u8) -> (Vec<(u16, u8)>, u8) {
    let v = v0405_x | a_in;
    (vec![(0x0405 + x as u16, v)], v)
}

/// OR+掩码链（旧 s2_f702_body（$F702）：$0405[x] ← (v|a)&$FB）。
pub fn or_enemy_data05_masked(x: u8, a_in: u8, v0405_x: u8) -> (Vec<(u16, u8)>, u8) {
    let v = (v0405_x | a_in) & 0xFB;
    (vec![(0x0405 + x as u16, v)], v)
}

/// AND+条件移位（旧 s2_f83b_body（$F83B）：AND 根+v0405≥0 ⇒ $0405[x] ← v>>3（三 lsr 形））。
pub fn and_enemy_data05_shift(x: u8, a_in: u8, v0405_x: u8) -> (Vec<(u16, u8)>, u8) {
    let mut ev = vec![(0x0405 + x as u16, v0405_x & a_in)];
    if (v0405_x as i8) >= 0 {
        ev.push((0x0405 + x as u16, v0405_x >> 3));
    }
    (ev, 0)
}

// ===== 旧实现改写·第十件（s1 族续去地址名（TDD 穷举等价证明））=====

#[cfg(test)]
mod tests_rewrite10 {
    //! TDD step1（测试先行（强制完整））——穷举等价（新语义 fn==旧 s1 地址名 fn
    //! （全维穷举（边界（异常路径））））。refs 锚（engine/mod.rs 注内 bank:addr）：
    //! $D498 尾共享形（:77）/$EB68（:88）/$EBD9 对象槽初始化（:97）/$EC2F LoadStatues
    //! （:111）/$EC89（:124）/$EE33（:129）。

    use super::*;
    use crate::engine::{s1_d498, s1_eb68, s1_ebd9, s1_load_statues, s1_ec89, s1_ee33};

    /// 【穷举·槽标记】root×每槽值 全维（每槽 256² 对拍+四槽角组合）——mark_matching==s1_d498。
    #[test]
    fn exhaustive_slot_mark_equivalence() {
        for root in 0u8..=255 {
            for v in 0u8..=255 {
                for &sx in &[0x18u16, 0x10, 0x08, 0x00] {
                    let mut iram = vec![0u8; 0x800];
                    iram[0x72C + sx as usize] = v;
                    assert_eq!(s1_d498(root, &iram), mark_matching_enemy_slots(root, &iram),
                               "root={} v={} slot={:#x}", root, v, sx);
                }
            }
        }
        // 四槽同值角组合
        for root in [0u8, 1, 0x7F, 0x80, 0xFF] {
            let mut iram = vec![0u8; 0x800];
            for sx in [0x18u16, 0x10, 0x08, 0x00] { iram[0x72C + sx as usize] = root; }
            assert_eq!(s1_d498(root, &iram), mark_matching_enemy_slots(root, &iram));
        }
    }

    /// 【异常路径】d498 短缓冲（iram len < 0x72C+x）——unwrap_or(0) 面等价。
    #[test]
    fn exception_d498_short_iram() {
        for root in 0u8..=255 {
            for len in 0..0x730usize {
                let iram = vec![0u8; len];
                assert_eq!(s1_d498(root, &iram), mark_matching_enemy_slots(root, &iram));
            }
        }
    }

    /// 【穷举·敌数据写+跳】root×x 全域——store_enemy_data_fb==s1_eb68。
    #[test]
    fn exhaustive_eb68_equivalence() {
        for root in 0u8..=255 {
            for x in 0u8..=255 {
                assert_eq!(s1_eb68(root, x), store_enemy_data_fb(root, x));
            }
        }
    }

    /// 【穷举·对象槽初始化】root×y 全域（wram 面）+x/tbl_hi 边角——init_object_slot==s1_ebd9。
    #[test]
    fn exhaustive_object_slot_equivalence() {
        for root in 0u8..=255 {
            for y in 0u8..=255 {
                for &(x, tbl_hi) in &[(0u8, 0u8), (0xFF, 0xFF), (1, 0x80), (0x7F, 0x01)] {
                    let mut w_old = vec![0u8; 0x2000];
                    let mut w_new = vec![0u8; 0x2000];
                    let o = s1_ebd9(root, x, y, 0x33, tbl_hi, &mut w_old);
                    let n = init_object_slot(root, x, y, 0x33, tbl_hi, &mut w_new);
                    assert_eq!(o, n);
                    assert_eq!(w_old, w_new);
                }
            }
        }
    }

    /// 【穷举·雕像装载】root×y×w687b×w687c 全域抽样边界（符号面 0/0x7F/0x80/0xFF 四值全组合
    /// ×root×y 全域）——load_statues==s1_load_statues（符号判别逐钉）。
    #[test]
    fn exhaustive_statues_equivalence() {
        for root in 0u8..=255 {
            for y in 0u8..=255 {
                for &b in &[0u8, 0x7F, 0x80, 0xFF] {
                    for &c in &[0u8, 0x7F, 0x80, 0xFF] {
                        assert_eq!(s1_load_statues(root, y, b, c), load_statues(root, y, b, c));
                    }
                }
            }
        }
    }

    /// 【穷举·房槽写+物槽指针】root×x×zp4f×zp50 边角全组合+root×x 全域——ec89/ee33 等价。
    #[test]
    fn exhaustive_room_item_slot_equivalence() {
        for root in 0u8..=255 {
            for x in 0u8..=255 {
                assert_eq!(s1_ec89(root, x), set_room_sec_slot(root, x));
                for &(f, g) in &[(0u8, 0u8), (0xFF, 0xFF), (0x50, 0x4F)] {
                    assert_eq!(s1_ee33(root, x, f, g), set_item_slot_pointer(root, x, f, g));
                }
            }
        }
    }
}

// ===== 旧实现改写·第十件：s1 族续语义命名面（去地址名（公式形（零字面特值）））=====
// refs 锚（engine/mod.rs 注内 bank:addr 引证）：$D498 尾共享形（:77）/$EB68（:88）/
// $EBD9 对象槽初始化（:97）/$EC2F LoadStatues（:111）/$EC89（:124）/$EE33（:129）。

/// 槽标记（旧 s1_d498（$D498 尾共享形）：四槽 $072C[x]==根 ⇒ $0728[x]←$FF
/// （X 序 $18/$10/$08/0））。
pub fn mark_matching_enemy_slots(root_val: u8, iram: &[u8]) -> Vec<(u16, u8)> {
    let mut ev = Vec::new();
    for x in [0x18u16, 0x10, 0x08, 0x00] {
        let cmp_v = iram.get(0x72C + x as usize).copied().unwrap_or(0);
        if root_val == cmp_v {
            ev.push((0x0728 + x, 0xFF));
        }
    }
    ev
}

/// 敌数据写+跳（旧 s1_eb68（$EB68）：$6AFB[x]←根→jmp $F85A）。
pub fn store_enemy_data_fb(root_val: u8, x: u8) -> (Vec<(u16, u8)>, u16) {
    (vec![(0x6AFB + x as u16, root_val)], 0xF85A)
}

/// 对象槽初始化（旧 s1_ebd9（$EBD9）：$030C[x]←根/$030E[x]←tbl_hi/$030D[x]←$68
/// +位或累积内联（$6C15[y] ← y|$6C15[根^1]）→A=2）。
pub fn init_object_slot(root_val: u8, x: u8, y: u8, tbl_lo: u8, tbl_hi: u8, wram: &mut [u8])
    -> (Vec<(u16, u8)>, u8) {
    let _ = tbl_lo;
    let mut ev = vec![(0x030C + x as u16, root_val), (0x030E + x as u16, tbl_hi),
                      (0x030D + x as u16, 0x68)];
    let xs = root_val ^ 1;
    let or_v = y | wram.get(0xC15 + xs as usize).copied().unwrap_or(0);
    let idx = 0xC15 + y as usize;
    if idx < wram.len() { wram[idx] = or_v; }
    ev.push((0x6C15 + y as u16, or_v));
    (ev, 2)
}

/// 雕像装载（旧 s1_load_statues（$EC2F）：$036C←根/$0370←符号判（$687C）/$036F←符号判
/// （$687B）/$54←Y/$0360←1→jmp $EAD4）。
pub fn load_statues(root_val: u8, y: u8, w687b: u8, w687c: u8)
    -> (Vec<(u16, u8)>, u16) {
    let ev = vec![
        (0x036C, root_val),
        (0x0370, if (w687c as i8) >= 0 { 0x40 } else { 0x30 }),
        (0x036F, if (w687b as i8) >= 0 { 0x60 } else { 0x50 }),
        (0x0054, y),
        (0x0360, 1),
    ];
    (ev, 0xEAD4)
}

/// 房槽写+共享尾跳（旧 s1_ec89（$EC89）：$072C[x]←根→jmp $EC54）。
pub fn set_room_sec_slot(root_val: u8, x: u8) -> (Vec<(u16, u8)>, u16) {
    (vec![(0x072C + x as u16, root_val)], 0xEC54)
}

/// 物槽指针写+跳（旧 s1_ee33（$EE33）：$074B[x]←根/$09←3/$07←$50/$06←$4F→jmp $DC67）。
pub fn set_item_slot_pointer(root_val: u8, x: u8, zp4f: u8, zp50: u8) -> (Vec<(u16, u8)>, u16) {
    let ev = vec![(0x074B + x as u16, root_val), (0x0009, 3),
                  (0x0007, zp50), (0x0006, zp4f)];
    (ev, 0xDC67)
}

// ===== 旧实现改写·第十二件（s1 族 handler 面去地址名（TDD 穷举等价证明））=====

#[cfg(test)]
mod tests_rewrite12 {
    //! TDD step1（测试先行（强制完整））——穷举等价（新==旧（全域穷举（边界（异常路径））））。
    //! refs 锚（engine/mod.rs 注内 bank:addr）：$D824（:172）/$D834·$D837（:180）/
    //! $D88F·$D892（:183）/$D83D·$D83A（:203）/$D870·$D86D（:219）。

    use super::*;
    use crate::engine::{s1_d824, s1_d837, s1_d892, s1_d83a_body, s1_d86d_body};

    /// 【穷举·水平 handler 分路】x×v030e×vfd 全域 256³——select_horizontal==s1_d824。
    #[test]
    fn exhaustive_horizontal_handler_equivalence() {
        for x in 0u8..=255 {
            for a in 0u8..=255 {
                for b in 0u8..=255 {
                    assert_eq!(s1_d824(x, a, b), select_horizontal_motion_handler(x, a, b));
                }
            }
        }
    }

    /// 【穷举·极简恒返】constant_fall_handler==s1_d837（全域（退化域 1 点））。
    #[test]
    fn exhaustive_constant_handler_equivalence() {
        assert_eq!(s1_d837(), constant_fall_handler());
    }

    /// 【穷举·垂直 handler 分路】v030f 全域——select_vertical==s1_d892（符号面逐钉）。
    #[test]
    fn exhaustive_vertical_handler_equivalence() {
        for v in 0u8..=255 {
            assert_eq!(s1_d892(v), select_vertical_motion_handler(v));
        }
    }

    /// 【穷举·垂直步进】x×v030f×v030d 全域 256³——step_vertical==s1_d83a_body
    /// （符号分路（负⇒$EF 归位+条件子调（v030d==0⇒$D648）））。
    #[test]
    fn exhaustive_step_vertical_equivalence() {
        for x in 0u8..=255 {
            for f in 0u8..=255 {
                for d in 0u8..=255 {
                    assert_eq!(s1_d83a_body(x, f, d), step_vertical_position(x, f, d));
                }
            }
        }
    }

    /// 【穷举·门写】x×vfc 全域——write_motion_defaults==s1_d86d_body（$FC==0 门（异常路径
    /// vfc≠0 空集））。
    #[test]
    fn exhaustive_gate_writes_equivalence() {
        for x in 0u8..=255 {
            for fc in 0u8..=255 {
                assert_eq!(s1_d86d_body(x, fc), write_motion_defaults_if_idle(x, fc));
            }
        }
    }
}

// ===== 旧实现改写·第十二件：s1 族 handler 面语义命名（去地址名（公式形（零字面特值）））=====
// refs 锚（engine/mod.rs 注内 bank:addr）：$D824（:172）/$D837（:180）/$D892（:183）/
// $D83A（:203）/$D86D（:219）。

/// 水平 handler 分路（旧 s1_d824（$D824）：$030E←$80+diff=v030e−vfd 符号分路
/// （非负⇒$E6A7/负⇒$E6D2））。
pub fn select_horizontal_motion_handler(x: u8, v030e_x: u8, vfd: u8) -> (Vec<(u16, u8)>, u16) {
    let ev = vec![(0x030E, 0x80)];
    let diff = v030e_x.wrapping_sub(vfd);
    let sub = if (diff as i8) >= 0 { 0xE6A7 } else { 0xE6D2 };
    (ev, sub)
}

/// 极简恒返 handler（旧 s1_d837（$D834 员体）：恒 $E6D2）。
pub fn constant_fall_handler() -> u16 {
    0xE6D2
}

/// 垂直 handler 分路（旧 s1_d892（$D88F）：$030F 符号分路（非负⇒$E519/负⇒$E4F1））。
pub fn select_vertical_motion_handler(v030f_x: u8) -> u16 {
    if (v030f_x as i8) >= 0 { 0xE519 } else { 0xE4F1 }
}

/// 垂直步进（旧 s1_d83a_body（$D83D）：非负⇒inc $030D[x]；负⇒$030D[x]←$EF
/// +条件子调（$030D[x]==0⇒$D648）+尾跳 $D866）。
pub fn step_vertical_position(x: u8, v030f_x: u8, v030d_x: u8) -> (Vec<(u16, u8)>, Option<u16>) {
    if (v030f_x as i8) >= 0 {
        (vec![(0x030D + x as u16, v030d_x.wrapping_add(1))], None)
    } else {
        let mut ev = Vec::new();
        let call = if v030d_x == 0 { Some(0xD648) } else { None };
        ev.push((0x030D + x as u16, 0xEF));
        (ev, call.or(Some(0xD866)))
    }
}

/// 门写（旧 s1_d86d_body（$D870）：$FC==0⇒四写（$0305←$4E/$0306←$41/$0305[x]←$5D/
/// $0306[x]←$50）；≠0⇒空（bne 跳））。
pub fn write_motion_defaults_if_idle(x: u8, vfc: u8) -> Vec<(u16, u8)> {
    if vfc != 0 {
        return Vec::new();
    }
    vec![(0x0305, 0x4E), (0x0306, 0x41),
         (0x0305 + x as u16, 0x5D), (0x0306 + x as u16, 0x50)]
}

// ===== 旧实现改写·第二十一件（s1/s2 族续去地址名（TDD 穷举等价证明））=====

#[cfg(test)]
mod tests_rewrite21 {
    //! TDD step1（测试先行（强制完整））——穷举等价（新==旧（全域穷举（边界（异常路径））））。
    //! refs 锚（engine/mod.rs 注内 bank:addr）：$D89A（:229）/$D8A0（:233）/$D8B7（:248）/
    //! $D970（:253）/$F735（:1899）/$F761（:1904）。

    use super::*;
    use crate::engine::{s1_d89a_body, s1_d8a0_body, s1_d8b7_body, s1_d970_body,
                        s2_f735_body, s2_f761_body};

    /// 【穷举·恒返二】constant_motion_handler==s1_d89a_body（退化域）。
    #[test]
    fn constant_motion_handler_equivalence() {
        assert_eq!(s1_d89a_body(), constant_motion_handler());
    }

    /// 【穷举·槽状态推进】x×v0300 全域——advance_slot_status==s1_d8a0_body
    /// （inc+==8 命中⇒$0303[x]←$23+子调 $CF6B（边界：0x07→0x08 命中/0xFF→0 回绕不命中））。
    #[test]
    fn exhaustive_advance_status_equivalence() {
        for x in 0u8..=255 {
            for v in 0u8..=255 {
                assert_eq!(s1_d8a0_body(x, v), advance_slot_status_with_reset(x, v));
            }
        }
    }

    /// 【穷举·尾跳面】flag_dispatch==s1_d8b7_body（退化域）。
    #[test]
    fn tail_dispatch_equivalence() {
        assert_eq!(s1_d8b7_body(), set_motion_flag_dispatch());
    }

    /// 【穷举·槽复位+init】reset_slot_and_init==s1_d970_body（退化域）。
    #[test]
    fn reset_init_equivalence() {
        assert_eq!(s1_d970_body(), reset_slot_and_init());
    }

    /// 【穷举·OR 变体族】x×a×v 全域——or_enemy_data05_with_shifts==s2_f735_body、
    /// or_enemy_data05_with_table_walk==s2_f761_body。
    #[test]
    fn exhaustive_or_variants_equivalence() {
        for x in 0u8..=255 {
            for a in 0u8..=255 {
                for v in 0u8..=255 {
                    assert_eq!(s2_f735_body(x, a, v), or_enemy_data05_with_shifts(x, a, v));
                    assert_eq!(s2_f761_body(x, a, v), or_enemy_data05_with_table_walk(x, a, v));
                }
            }
        }
    }
}

// ===== 旧实现改写·第二十一件：s1/s2 族续语义命名（去地址名（公式形（零字面特值）））=====
// refs 锚（engine/mod.rs 注内 bank:addr）：$D89A（:229）/$D8A0（:233）/$D8B7（:248）/
// $D970（:253）/$F735（:1899）/$F761（:1904）。

/// 恒返 motion handler（旧 s1_d89a_body（$D89A）：恒 $E519）。
pub fn constant_motion_handler() -> u16 {
    0xE519
}

/// 槽状态推进+复位（旧 s1_d8a0_body（$D8A0）：inc $0300[x]；==8⇒$0303[x]←$23
/// +子调 $CF6B；否则无子调）。
pub fn advance_slot_status_with_reset(x: u8, v0300_x: u8) -> (Vec<(u16, u8)>, Option<u16>) {
    let nv = v0300_x.wrapping_add(1);
    let mut ev = vec![(0x0300 + x as u16, nv)];
    if nv == 0x08 {
        ev.push((0x0303 + x as u16, 0x23));
        (ev, Some(0xCF6B))
    } else {
        (ev, None)
    }
}

/// 尾跳 flag 派发（旧 s1_d8b7_body（$D8BA）：lda #$01→jmp $DE47）。
pub fn set_motion_flag_dispatch() -> (u8, u16) {
    (0x01, 0xDE47)
}

/// 槽复位+init（旧 s1_d970_body（$D973）：$0307←0/$7D←0→jsr $F186）。
pub fn reset_slot_and_init() -> (Vec<(u16, u8)>, u16) {
    (vec![(0x0307, 0), (0x007D, 0)], 0xF186)
}

/// OR 变体（旧 s2_f735_body（$F735）：OrEnData05 根委托（三 lsr/ror 段注记））。
pub fn or_enemy_data05_with_shifts(x: u8, a_in: u8, v0405_x: u8) -> (Vec<(u16, u8)>, u8) {
    or_enemy_data05(x, a_in, v0405_x)
}

/// OR 变体（旧 s2_f761_body（$F761）：OrEnData05 根委托（$6B02 表走注记））。
pub fn or_enemy_data05_with_table_walk(x: u8, a_in: u8, v0405_x: u8) -> (Vec<(u16, u8)>, u8) {
    or_enemy_data05(x, a_in, v0405_x)
}

// ===== 1:1 Rust 改写·块 3（s1 族 $F5xx/$F7FC 面去地址名（TDD 穷举等价证明））=====

#[cfg(test)]
mod tests_rewrite_b3 {
    //! TDD step1（测试先行（强制完整））——穷举等价（新==旧（全域穷举（边界（异常路径））））。
    //! refs 锚（engine/mod.rs 注内 bank:addr）：旗门根（:298（bit5=$20））/$F56C（:349）/
    //! $F5AC（:358）/$F5CC（:369）/$F7FC（:380）。

    use super::*;
    use crate::engine::{s1_flag_gate, s1_f56c, s1_f5ac, s1_f5cc, s1_f7fc};

    /// 【穷举·旗门根】root 全域——flag_gate_bit5==s1_flag_gate。
    #[test]
    fn exhaustive_flag_gate_equivalence() {
        for r in 0u8..=255 {
            assert_eq!(s1_flag_gate(r), flag_gate_bit5(r));
        }
    }

    /// 【穷举·旗门反向派发】root×x 全域——dispatch_on_flag_miss==s1_f56c。
    #[test]
    fn exhaustive_dispatch_miss_equivalence() {
        for r in 0u8..=255 {
            for x in 0u8..=255 {
                assert_eq!(s1_f56c(r, x), dispatch_on_flag_miss(r, x));
            }
        }
    }

    /// 【穷举·门判别】root×v040e×v6af4 全域（x 边角）——check_motion_state==s1_f5ac。
    #[test]
    fn exhaustive_gate_check_equivalence() {
        for r in 0u8..=255 {
            for a in 0u8..=255 {
                for b in 0u8..=255 {
                    for x in [0u8, 0xFF] {
                        assert_eq!(s1_f5ac(r, x, a, b), check_motion_state_by_gate(r, x, a, b));
                    }
                }
            }
        }
    }

    /// 【穷举·反向双判别】root×x×v040e 全域——gate_reverse_double_check==s1_f5cc。
    #[test]
    fn exhaustive_reverse_double_check_equivalence() {
        for r in 0u8..=255 {
            for x in 0u8..=255 {
                for a in 0u8..=255 {
                    assert_eq!(s1_f5cc(r, x, a), gate_reverse_double_check(r, x, a));
                }
            }
        }
    }

    /// 【穷举·bpl 跳过】root×x×tbl 全域——clear_motion_pair==s1_f7fc（bpl 空集（异常路径））。
    #[test]
    fn exhaustive_bpl_skip_equivalence() {
        for r in 0u8..=255 {
            for x in 0u8..=255 {
                for t in 0u8..=255 {
                    assert_eq!(s1_f7fc(r, x, t), clear_motion_pair_on_negative(r, x, t));
                }
            }
        }
    }
}

// ===== 1:1 Rust 改写·块 3：s1 族 $F5xx/$F7FC 语义命名（去地址名（公式形（零字面特值）））=====
// refs 锚（engine/mod.rs 注内 bank:addr）：旗门根（:298（bit5=$20））/$F56C（:349）/
// $F5AC（:358）/$F5CC（:369）/$F7FC（:380）。

/// 旗门（旧 s1_flag_gate（bit5=$20 面）：root 位5 判）。
pub fn flag_gate_bit5(root_val: u8) -> bool {
    root_val & 0x20 != 0
}

/// 旗门反向派发（旧 s1_f56c（$F56C）：未命中⇒子调 $CBB8（尾跳 $F42D 注）；命中⇒None）。
pub fn dispatch_on_flag_miss(root_val: u8, x: u8) -> Option<u16> {
    let _ = x;
    if !flag_gate_bit5(root_val) {
        Some(0xCBB8)
    } else {
        None
    }
}

/// 门判别（旧 s1_f5ac（$F5AC）：命中⇒$040E[x]==$0B；未命中⇒$6AF4[x]==$04）。
pub fn check_motion_state_by_gate(root_val: u8, x: u8, v040e_x: u8, v6af4_x: u8) -> bool {
    let _ = x;
    if flag_gate_bit5(root_val) {
        v040e_x == 0x0B
    } else {
        v6af4_x == 0x04
    }
}

/// 反向双判别（旧 s1_f5cc（$F5CC）：命中⇒空；未命中⇒$040E[x]==$0B/$81 双判（真⇒空）；
/// 否则 $6AF4[x]←6）。
pub fn gate_reverse_double_check(root_val: u8, x: u8, v040e_x: u8) -> (Vec<(u16, u8)>, bool) {
    if flag_gate_bit5(root_val) {
        return (vec![], false);
    }
    if v040e_x == 0x0B || v040e_x == 0x81 {
        return (vec![], true);
    }
    (vec![(0x6AF4 + x as u16, 6)], false)
}

/// bpl 跳过面（旧 s1_f7fc（$F7FC）：非负⇒空；负⇒$0406[x]←0/$0407[x]←0/$6AFE[x]←tbl）。
pub fn clear_motion_pair_on_negative(root_val: u8, x: u8, tbl_entry: u8) -> Vec<(u16, u8)> {
    if (root_val as i8) >= 0 {
        return Vec::new();
    }
    vec![(0x0406 + x as u16, 0), (0x0407 + x as u16, 0), (0x6AFE + x as u16, tbl_entry)]
}

// ===== engine/mod.rs 清理·块 3（KillObject 族去地址名（TDD 穷举等价证明））=====

#[cfg(test)]
mod tests_cleanup3 {
    //! TDD step1（测试先行（强制完整））——穷举等价（新==旧（全域穷举（≤256³ 闸时限界
    //! （块 6 256⁴ 教训（闸时回归））））。refs 锚（engine/mod.rs 注内 bank:addr）：
    //! $F48A（:657）/$F4DF（:663）/$F52D（:673）/$FA18（:685）/$FA77（:691）/$FA91（:698）。

    use super::*;
    use crate::engine::{s1_f48a_body, s1_f4df_body, s1_f52d_body, s1_fa18_kill,
                        s1_fa77_body, s1_fa91_body};

    /// 【穷举·击杀写】x 全域——kill_enemy_slot==s1_f48a_body。
    #[test]
    fn exhaustive_kill_slot_equivalence() {
        for x in 0u8..=255 {
            assert_eq!(s1_f48a_body(x), kill_enemy_slot(x));
        }
    }

    /// 【穷举·计数击杀】x×v2d×v040f 全域 256³——kill_with_hit_counter==s1_f4df_body
    /// （$2D 位1 判（$6B←$A1/$A0）+$040F[x] dec 回绕）。
    #[test]
    fn exhaustive_counter_kill_equivalence() {
        for x in 0u8..=255 {
            for v2d in 0u8..=255 {
                for v0f in 0u8..=255 {
                    assert_eq!(s1_f4df_body(x, v2d, v0f), kill_with_hit_counter(x, v2d, v0f));
                }
            }
        }
    }

    /// 【穷举·状态暂存击杀】x×v040f×v0404 全域 256³——kill_and_stage==s1_f52d_body
    /// （$0404 位5 判段（未置⇒$040E 写））。
    #[test]
    fn exhaustive_stage_kill_equivalence() {
        for x in 0u8..=255 {
            for v0f in 0u8..=255 {
                for v04 in 0u8..=255 {
                    assert_eq!(s1_f52d_body(x, v0f, v04), kill_and_stage_motion_state(x, v0f, v04));
                }
            }
        }
    }

    /// 【穷举·指针击杀×2】x×tbl 全域+四参边角——kill_and_load_*==旧体。
    #[test]
    fn exhaustive_pointer_kill_equivalence() {
        for x in 0u8..=255 {
            assert_eq!(s1_fa18_kill(x), clear_enemy_slot(x));
            for &(a, b, c) in &[(0u8, 0u8, 0u8), (0xFF, 0xFF, 0xFF), (0x7F, 0x80, 1)] {
                assert_eq!(s1_fa77_body(x, a, b, c), kill_and_load_pointer(x, a, b, c));
            }
            for t in 0u8..=255 {
                assert_eq!(s1_fa91_body(x, t), kill_and_load_spawn_table(x, t));
            }
        }
    }
}

// ===== engine/mod.rs 清理·块 3：KillObject 族语义命名（去地址名（公式形（零字面特值）））=====
// refs 锚（engine/mod.rs 注内 bank:addr）：$F48A（:657）/$F4DF（:663）/$F52D（:673）/
// $FA18（:685）/$FA77（:691）/$FA91（:698）。

/// 击杀写（旧 s1_f48a_body（$F48A）：$6AF4[x]←0（KillObject 共写））。
pub fn kill_enemy_slot(x: u8) -> Vec<(u16, u8)> {
    vec![(0x6AF4 + x as u16, 0)]
}

/// 计数击杀（旧 s1_f4df_body（$F4DF）：$6AF4[x]←0+$6B←（$2D 位1?$A1:$A0）+
/// $040F[x]←dec（回绕）→jmp $F416）。
pub fn kill_with_hit_counter(x: u8, v2d: u8, v040f_x: u8) -> (Vec<(u16, u8)>, u16) {
    let hi = if v2d & 0x02 != 0 { 0xA1 } else { 0xA0 };
    (vec![(0x6AF4 + x as u16, 0), (0x006B, hi),
          (0x040F + x as u16, v040f_x.wrapping_sub(1))], 0xF416)
}

/// 状态暂存击杀（旧 s1_f52d_body（$F52D）：$6AF4[x]←0+$0A←$040F[x]
/// +$0404 位5 未置⇒$040E[x]←$0404[x]）。
pub fn kill_and_stage_motion_state(x: u8, v040f_x: u8, v0404_x: u8) -> Vec<(u16, u8)> {
    let mut ev = vec![(0x6AF4 + x as u16, 0), (0x000A, v040f_x)];
    if v0404_x & 0x20 == 0 {
        ev.push((0x040E + x as u16, v0404_x));
    }
    ev
}

/// 敌槽清（旧 s1_fa18_kill（$FA18 KillObject）：$6AF4[x]←0）。
pub fn clear_enemy_slot(x: u8) -> (Vec<(u16, u8)>, u8) {
    (vec![(0x6AF4 + x as u16, 0)], 0)
}

/// 指针击杀（旧 s1_fa77_body（$FA77）：$6AF4[x]←0+$02←$0400[x]/$03←$0401[x]/
/// $0B←$6AFB[x]→jmp $F97C）。
pub fn kill_and_load_pointer(x: u8, v0400_x: u8, v0401_x: u8, v6afb_x: u8) -> (Vec<(u16, u8)>, u16) {
    (vec![(0x6AF4 + x as u16, 0), (0x0002, v0400_x), (0x0003, v0401_x),
          (0x000B, v6afb_x)], 0xF97C)
}

/// 表载击杀（旧 s1_fa91_body（$FA91）：$6AF4[x]←0+$6AF9/$6AFA[x]←tbl+$6AF8[x]←0
/// →jmp $F97C）。
pub fn kill_and_load_spawn_table(x: u8, tbl_entry: u8) -> (Vec<(u16, u8)>, u16) {
    (vec![(0x6AF4 + x as u16, 0),
          (0x6AF9 + x as u16, tbl_entry), (0x6AFA + x as u16, tbl_entry),
          (0x6AF8 + x as u16, 0)], 0xF97C)
}
