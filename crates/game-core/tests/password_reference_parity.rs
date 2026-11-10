//! 密码族 ROM 执行参考径穷举对拍（卡 B G2 硬验收）。
//!
//! 参考径=analysis/password_verify pwlib.py（mini-6502 **ROM 字节执行** $8F60
//! ConsolidatePassword + $8DDE ValidatePassword——非再实现，铁证=ROM 字节）。
//! 1014 组向量（JB 锚例/全零/全 FF/全 3F/NAR 5+10/canonical 450（含 shift
//! 0x00/01/7F/80/FF 极值）/原始随机 500/坏 checksum 45），逐字节对拍。

mod common;
use common::{VECTORS, N};

use game_core::ui::password::check_password;

/// 硬验收：1014 组向量逐字节对拍 rust check_password vs ROM 字节执行
/// （$8F60 ConsolidatePassword+$8E4E Unscramble+$8E21 Checksum cmp，pwlib 实跑）。
#[test]
fn reference_parity_1014_vectors() {
    let mut mism = 0usize;
    for (i, &(chars, nar, pb_exp, valid_exp)) in VECTORS.iter().enumerate() {
        let (pb, valid, _) = check_password(&chars, nar);
        if pb != pb_exp || valid != valid_exp {
            if mism < 5 {
                eprintln!(
                    "vector #{}/{} MISMATCH\n  chars={:02X?}\n  nar={}\n  rust pb={:02X?} valid={}\n  rom  pb={:02X?} valid={}",
                    i + 1,
                    N,
                    chars,
                    nar,
                    pb,
                    valid,
                    pb_exp,
                    valid_exp
                );
            }
            mism += 1;
        }
    }
    assert_eq!(mism, 0, "参考径对拍分歧 {}/{}", mism, N);
}
