//! 区域初值表（r25 烘焙——SamusInit `lda AreaSamusY` 面）。
//!
//! ROM 锚（A 级）：各区域 bank `$95D9`（prg1_brinstar.asm:135 `AreaSamusY: ; 01:95D9`——
//! prg2/3/4/5 同址）——字节自证：
//! `python3 -c "p=open('rom/Metroid (USA).nes','rb').read()[0x10:]; print([hex(p[b*0x4000+0x15D9]) for b in range(1,6)])"`

/// AreaSamusY（区域序=InArea&$0F：0=Brinstar/1=Norfair/2=Kraid/3=Tourian/4=Ridley）。
pub const AREA_SAMUS_Y: [u8; 5] = [0xB0, 0x6E, 0x6E, 0x6E, 0x6E];

#[cfg(test)]
mod tests {
    use super::*;

    /// ROM 锚测试（五值逐字——上面一行自证命令）。
    #[test]
    fn area_samus_y_rom_anchor() {
        assert_eq!(AREA_SAMUS_Y, [0xB0, 0x6E, 0x6E, 0x6E, 0x6E]);
    }
}
