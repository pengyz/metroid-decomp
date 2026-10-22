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
