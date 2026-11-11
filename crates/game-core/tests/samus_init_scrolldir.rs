//! 卡 C：SamusInit 非 Brinstar 径的 `lsr ScrollDir` 端口（TDD 锚）。
//!
//! asm 内容锚 `refs/m1disasm/SRC/prg7_engine.asm:1996-2005`
//! `SamusInit: ; 07:C8D1`：
//!   ldy #$27 / lda InArea / and #$0F / beq Lx002(07:C908)
//!   lsr ScrollDir          ; 07:C902  ROM 逐字节 = 46 49（lsr $49 zp）
//!   ldy #$2F               ; 置 PPU 横向镜像
//!   Lx002: sty MirrorCntrl / sty MissilePickupQtyMax / sty EnergyPickupQtyMax
//! 常量锚 `refs/m1disasm/SRC/constants_ram.asm:530` `ScrollDir db ;$49`。
//!
//! 旧实现把该径留作 `unported!("bank07:C902-ScrollDir")` fail-loud 桩
//! （注释「ScrollAddr 未定谳——不猜写」）——ROM 直读后地址已定谳，按 asm 真形端口。

use game_core::frame::mainloop::samus_init;
use game_core::frame::FrameMemory;
use game_core::ram_map::{IN_AREA, MIRROR_CNTRL, SCROLL_DIR};

#[test]
fn samus_init_non_brinstar_lsr_scrolldir_and_ldy_2f() {
    // 非 Brinstar（InArea=0x11 Norfair，JUSTIN BAILEY 出生区）：
    // ScrollDir=2(Left) --lsr--> 1(Down)；MirrorCntrl/MissilePickupQtyMax/EnergyPickupQtyMax ← $2F
    let mut m = FrameMemory::default();
    m.write_ram(IN_AREA, 0x11);
    m.write_ram(SCROLL_DIR, 0x02);
    samus_init(&mut m);
    assert_eq!(
        m.read_ram(SCROLL_DIR),
        0x01,
        "asm 07:C902 `lsr ScrollDir($49)`：2(Left)→1(Down)，carry 后续未用"
    );
    assert_eq!(m.read_ram(MIRROR_CNTRL), 0x2F, "asm `ldy #$2F`→sty MirrorCntrl($FA)");
    assert_eq!(m.read_ram(0x0093), 0x2F, "sty MissilePickupQtyMax($93)");
    assert_eq!(m.read_ram(0x0094), 0x2F, "sty EnergyPickupQtyMax($94)");
}

#[test]
fn samus_init_brinstar_leaves_scrolldir_and_ldy_27() {
    // Brinstar（InArea=0）：beq Lx002 直接跳到 C908，不执行 lsr；Y=$27
    let mut m = FrameMemory::default();
    m.write_ram(IN_AREA, 0x00);
    m.write_ram(SCROLL_DIR, 0x02);
    samus_init(&mut m);
    assert_eq!(m.read_ram(SCROLL_DIR), 0x02, "Brinstar 不 lsr ScrollDir（beq Lx002 跳过）");
    assert_eq!(m.read_ram(MIRROR_CNTRL), 0x27, "asm `ldy #$27`→sty MirrorCntrl($FA)");
}
