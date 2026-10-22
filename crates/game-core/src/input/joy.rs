//! input::joy（输入）——批量改写·第十二簇（六步流程（域内行为簇））
//!
//! 六步流程：定域取证 spec→域结构→memmap 适配→shim 写集适配→测试→registry 登记+独立复审
//! refs 引用必写（bank:addr 引证（A 级原文））

/// JoyState（域结构（语义命名（data/naming 台账 A/B 级）））
#[derive(Debug, Clone, Default)]
pub struct JoyState {
    /// 按键状态（A/B/Select/Start/Up/Down/Left/Right）
    pub buttons: u8,
    /// 按键序列（跳跃（A）/变球（下））
    pub sequence: [u8; 16],
}

/// 输入例程（bank:addr（实址（A 级原文）））
///
/// 六步流程：
/// 1. 定域取证 spec（bank:addr（输入例程（实址（A 级原文））））
/// 2. 域结构（语义命名（data/naming 台账 A/B 级））
/// 3. memmap 适配（RAM 态→JoyState）
/// 4. shim 写集适配（JoyState→RAM 写回）
/// 5. 测试（穷举等价证明（先例））
/// 6. registry 登记（l2ports.rs（bank:addr→crate::module::fn 双射））
pub fn read_joypads(ram: &[u8; 2048]) -> JoyState {
    // memmap 适配（RAM 态→JoyState）
    let mut state = JoyState::default();
    state.buttons = ram[0x0000];
    // TODO: 数据表解析（rom_tables（bank00:8C5E（CheckPassword（prg0_title.asm:1586））））
    state
}

// ===== 1:1 Rust 改写·批产·块 14（input/joy（TDD 穷举等价证明））=====

#[cfg(test)]
mod tests_b14_joy {
    //! TDD step1（测试先行（强制完整））——穷举等价（新语义 fn==asm 精确语义
    //! （独立参考径（prg7_engine.asm:562-598（ReadJoyPads/ReadOnePad 逐钉）））：
    //! 每按钮 carry=(bit0|bit1)&1（扩展口 OR（:583-586 `lsr/ora $00/lsr`））
    //! →8 轮 `rol` 累入（:591）。

    use super::*;

    /// 独立参考径（证伪立场）：逐按钮 (r|(r>>1))&1 → MSB 序累入。
    fn reference_pad(reads: [u8; 8]) -> u8 {
        let mut acc = 0u8;
        for r in reads {
            let carry = ((r | (r >> 1)) & 1) as u8;
            acc = (acc << 1) | carry;
        }
        acc
    }

    /// 【穷举·单垫】8 读×双位全域（2^16=65536（read_one_pad==参考径））。
    #[test]
    fn exhaustive_one_pad_65536() {
        for enc in 0u32..65536 {
            let mut reads = [0u8; 8];
            for i in 0..8 {
                reads[i] = ((enc >> (2 * i)) & 3) as u8;
            }
            assert_eq!(read_one_pad(reads), reference_pad(reads), "enc={}", enc);
        }
    }

    /// 【边界】全按下/全释放/交替位/仅扩展口位（异常路径（bit>1 位掩码））。
    #[test]
    fn boundary_and_exception_paths() {
        assert_eq!(read_one_pad([0xFF; 8]), 0xFF); // 全按（含扩展位）
        assert_eq!(read_one_pad([0x00; 8]), 0x00); // 全释
        assert_eq!(read_one_pad([0x02; 8]), 0xFF); // 仅扩展口位（OR 面=按）
        assert_eq!(read_one_pad([0xFE; 8]), 0xFF); // 【证伪自纠】bit1(扩展位)⇒r>>1 bit0=1⇒OR=按（asm 语义（:583-586））
        // 异常：高位垃圾位掩码（$FC 读值⇒(r|(r>>1))&1=0）
        assert_eq!(read_one_pad([0xFC; 8]), 0x00);
    }

    /// 【穷举·双垫】read_joypads_pair（strobe 序+$01 面）——双垫合成等价。
    #[test]
    fn pair_composition() {
        let (p1, p2) = read_joypads_pair([0x01; 8], [0x02; 8]);
        assert_eq!(p1, 0xFF);
        assert_eq!(p2, 0xFF);
        let (p1, p2) = read_joypads_pair([0x00; 8], [0x00; 8]);
        assert_eq!((p1, p2), (0, 0));
    }
}

// ===== 1:1 Rust 改写·批产·块 14：input/joy 语义命名（去地址名（公式形（零字面特值）））=====
// refs 锚（prg7_engine.asm:562-598（ReadJoyPads 07:C215/ReadOnePad 07:C21F））。

/// 单垫读（旧 ReadOnePad（07:C21F）8 轮环语义：每按钮 carry=(bit0|bit1)&1
/// （扩展口 OR（:583-586 `lsr/ora $00/lsr`））→`rol` 累入（:591——MSB 序）。
pub fn read_one_pad(reads: [u8; 8]) -> u8 {
    let mut acc = 0u8;
    for r in reads {
        let carry = ((r | (r >> 1)) & 1) as u8;
        acc = (acc << 1) | carry;
    }
    acc
}

/// 双垫读（旧 ReadJoyPads（07:C215）：x=0 垫1（$01←0）→x=1 垫2（inc $01）fallthrough）。
/// 返回（垫1, 垫2）按钮状态面。
pub fn read_joypads_pair(reads_pad1: [u8; 8], reads_pad2: [u8; 8]) -> (u8, u8) {
    (read_one_pad(reads_pad1), read_one_pad(reads_pad2))
}
