//! SubtractHealth 死亡分支的**行为测**（放 `/tests/` 目录＝门禁 test 面，不计生产裸字节债）。
use crate::frame::Manager;

fn hc(m: &Manager) -> (u8, u8) {
    (m.read_ram(0x006E), m.read_ram(0x006F))
}

fn set_health(m: &mut Manager, lo: u8, hi: u8) {
    m.write_ram(0x0106, lo);
    m.write_ram(0x0107, hi);
}

#[test]
fn lethal_damage_kills_and_starts_explosion() {
    let mut m = Manager::default();
    set_health(&mut m, 0x60, 0x00);
    m.write_ram(0x006E, 0x60); // HealthChange = 0x0060
    crate::weapon::projectile::subtract_health(&mut m);
    // asm @dead：`Health←0 / Health+1←0 / status←sa_Dead / jmp SetSamusExplode`
    assert_eq!(m.read_ram(0x0106), 0x00, "Health 低位应清零");
    assert_eq!(m.read_ram(0x0107), 0x00, "Health 高位应清零");
    assert_eq!(m.samus().status_raw(), 7, "status 应为 sa_Dead");
    assert_eq!(m.samus().anim_reset_index(), 0x32, "爆炸动画 reset=$32");
    assert_eq!(m.samus().anim_index(), 0x32, "爆炸动画 index=$32");
    assert_eq!(m.samus().jump_displacement(), 0x50, "asm 写 jumpDsplcmnt=$50");
    assert_eq!(m.tri_init_flags() & 0x80, 0x80, "SFX_SamusDie 应置 TRI init $80");
    assert_eq!(hc(&m), (0x60, 0x00), "死径不得清 HealthChange（asm 跳过 cleanup）");
}

#[test]
fn nonlethal_damage_clears_health_change_only() {
    let mut m = Manager::default();
    set_health(&mut m, 0x99, 0x00);
    m.write_ram(0x006E, 0x05);
    crate::weapon::projectile::subtract_health(&mut m);
    assert_eq!(m.read_ram(0x0106), 0x94, "BCD 0x99-0x05=0x94");
    assert_eq!(m.samus().status_raw(), 0, "非致命不应改 status");
    assert_eq!(hc(&m), (0x00, 0x00), "非死径清 HealthChange");
}

#[test]
fn health_below_ten_counts_as_dead() {
    // asm `lda Health / and #$F0 / beq @dead` ⇒ 十位域为 0 即判死（Health<10 quirk）
    let mut m = Manager::default();
    set_health(&mut m, 0x08, 0x00);
    m.write_ram(0x006E, 0x01);
    crate::weapon::projectile::subtract_health(&mut m);
    assert_eq!(m.read_ram(0x0106), 0x00, "Health<10 ⇒ 判死并清零");
    assert_eq!(m.samus().status_raw(), 7);
}
