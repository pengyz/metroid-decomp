//! 段 0 的结局背景 VRAM 结构（`VRAMStruct_EndBackground ; 00:A052`）与 `PreparePPUProcess`
//! 的记录解包（规格 §5.1/§5.2）。
//!
//! 数据本体已由 `game-data` 从 ROM 导出（`assets::end_background()`，`hi==0` 自界 360 B）；
//! 但 `game-core` 里没有一层把「记录序列」解成可写的 `(PPU 地址, 数据)`，`PreparePPUProcess`
//! 的消费端目前只在 shell 侧。本模块补这一层 typed 解包，供段 0 接线时直接复用。
//!
//! 记录布局（`macros.asm:32-73`）：
//!
//! ```text
//! [ppuAddrHi][ppuAddrLo][count|$40 若重复][数据...] ... $00
//! ```
//!
//! `count|$40` 表示**重复 1 个字节 `count` 次**，否则写 `count` 个数据字节。

use game_data::{parse_vram_struct, VramRecord};

/// 结局背景名表（`VRAMStruct_EndBackground ; 00:A052`，360 B，`hi==0` 自带终止）。
pub fn background_bytes() -> &'static [u8] {
    game_data::assets::end_background()
}

/// 把一条 VRAMStruct 解成记录表（到 `$00` 终止）；结构非法返回 `None`。
pub fn decode_struct(bytes: &[u8]) -> Option<Vec<VramRecord>> {
    parse_vram_struct(bytes).map(|(recs, _)| recs)
}

/// 段 0 结局背景的记录表（`00:9AD5` 的 `PreparePPUProcess(VRAMStruct_EndBackground)`）。
///
/// **前提**：调用方已完成资产包初始化（`game_data::pack::init_from_env()`）——
/// 与 `enemy/ai_table.rs` 的测试同一约定。
pub fn background_records() -> Option<Vec<VramRecord>> {
    decode_struct(background_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 数据记录：`$2300` 写 32 B 的 `$30/$31` 交替（`00:A052` 头 10 B 的原样形状）。
    #[test]
    fn decodes_data_record_with_ppu_address() {
        let mut raw = vec![0x23, 0x00, 0x20];
        for i in 0..32 {
            raw.push(if i % 2 == 0 { 0x30 } else { 0x31 });
        }
        raw.push(0x00); // VRAMStructEnd
        let recs = decode_struct(&raw).expect("合法结构");
        assert_eq!(recs.len(), 1);
        match &recs[0] {
            VramRecord::Data { ppu_addr, data } => {
                assert_eq!(*ppu_addr, 0x2300);
                assert_eq!(data.len(), 32);
                assert_eq!((data[0], data[1], data[31]), (0x30, 0x31, 0x31));
            }
            other => panic!("应为 Data 记录，实得 {other:?}"),
        }
    }

    /// 重复记录：`$23F0` 重复 `$10` 次 `$FF`（属性表全满，`00:A0xx` 的写形状）。
    #[test]
    fn decodes_repeat_record() {
        let raw = [0x23, 0xF0, 0x40 | 0x10, 0xFF, 0x00];
        let recs = decode_struct(&raw).expect("合法结构");
        assert_eq!(recs.len(), 1);
        match &recs[0] {
            VramRecord::Repeat {
                ppu_addr,
                reps,
                value,
            } => {
                assert_eq!((*ppu_addr, *reps, *value), (0x23F0, 0x10, 0xFF));
            }
            other => panic!("应为 Repeat 记录，实得 {other:?}"),
        }
    }

    /// 缺终止符 / 被截断的结构必须返回 `None`（不静默当成功）。
    #[test]
    fn rejects_truncated_or_unterminated() {
        assert_eq!(decode_struct(&[0x23, 0x00]), None); // 只有地址、无控制字节
        assert_eq!(decode_struct(&[0x23, 0x00, 0x02, 0x30]), None); // 声明 2 B 只给 1 B
        assert_eq!(decode_struct(&[0x23]), None); // 连地址都不全
    }

    /// 资产包就绪时：`A052` 必须解成 17 条记录且首条是 `$2300` 的 32 B 地面砖。
    #[test]
    fn rom_background_struct_shape() {
        game_data::pack::init_from_env().ok();
        if !game_data::pack::is_ready() {
            return; // 无资产包的环境跳过（与仓库既有测试同一约定）
        }
        let bytes = background_bytes();
        assert_eq!(bytes.len(), 360, "A052 段应为 360 B");
        let recs = decode_struct(bytes).expect("A052 应可解");
        assert_eq!(recs.len(), 17, "17 条记录 + 终止符");
        match &recs[0] {
            VramRecord::Data { ppu_addr, data } => {
                assert_eq!(*ppu_addr, 0x2300);
                assert_eq!(data.len(), 32);
                assert_eq!((data[0], data[1]), (0x30, 0x31));
            }
            other => panic!("首条应为 $2300 Data，实得 {other:?}"),
        }
    }
}
