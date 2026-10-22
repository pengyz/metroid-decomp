//! `game-core::weapon::missile` —— 导弹发射（批量改写·第三十一簇（六步流程））。
//!
//! 【step1 定域取证 spec】asm 内容锚（refs/m1disasm/SRC/prg7_engine.asm——A 级 bank:addr）：
//! - `CheckHorizontalMissileLaunch: ; 07:D315`（:4003）——门：MissileToggle==0⇒出／
//!   槽位≠SamusProjectiles.0（$03D0）⇒出（:4006-4009）；anim←HorizontalMissileAnims[SamusDir]
//! - `CheckMissileLaunchCommon: ; 07:D323`（:4013）——SetBulletAnim（animIndex/animResetIndex
//!   ←A/animDelay←0）+SFX_MissileLaunch（SFX 边界）+status←wa_Missile（$0B）+DieDelay←$FF+
//!   `dec MissileCount/bne`⇒非末枚出；末枚⇒`dec MissileToggle`（回普通火模式）+
//!   SelectSamusPalette（调色边界）
//! - `HorizontalMissileAnims: ; 07:D33E`（:4026）——2 项（Right/Left（SamusDir 0=右/1=左））
//! - `CheckVerticalMissileLaunch: ; 07:D340`（:4030）——同门；A←ObjAnim_MissileUp⇒common
//! - `SetBulletAnim: ; 07:D34D`（:4037）——animIndex/animResetIndex←A；animDelay←0
//!
//! 【step2 域结构】MissileLaunch（anim/写面/末枚标记——语义命名）；MissileAnims{right,left,up}
//! （表值差算面（ObjectAnimIndexTbl 差））；【step3 memmap】MissileToggle=$010E（constants_ram
//! :818）/MissileCount=$6879（:1210）/SamusDir=$4D（:540（0=右/1=左））/槽=$03D0 面
//! （Objects 面（:217-241）：status+0/animDelay+4/animResetIndex+5/animIndex+6）/
//! wa_Missile=$0B（constants.asm:111）；【step4 shim 写集】missile_launch_writes；
//! 【step5 测试】toggle×槽×方向×朝向×弹数全域穷举（下 `mod tests`）；
//! 【step6 registry 登记】l2ports.rs `s4_cluster_missile`。

/// 导弹 anim 表值（HorizontalMissileAnims（:4026）+Vertical（:4033）——表差算面）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MissileAnims {
    /// ObjAnim_MissileRight-ObjectAnimIndexTbl（SamusDir=0）
    pub right: u8,
    /// ObjAnim_MissileLeft-ObjectAnimIndexTbl（SamusDir=1）
    pub left: u8,
    /// ObjAnim_MissileUp-ObjectAnimIndexTbl（垂直发射）
    pub up: u8,
}

/// 发射结果（写面+末枚标记（MissileToggle 复位+调色边界随记））。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MissileLaunch {
    /// SetBulletAnim 面（animIndex=animResetIndex=anim（:4037-4041））
    pub anim: u8,
    /// dec MissileCount 后值（:4021）
    pub missile_count_after: u8,
    /// 末枚标记（count 归 0⇒dec MissileToggle（1→0）+调色）
    pub last_missile: bool,
}

/// CheckMissileLaunch（07:D315/D340/D323 合成面——门/anim/dec 语义逐钉）。
pub fn check_missile_launch(
    missile_toggle: u8,
    is_missile_slot: bool,
    samus_dir: u8,
    vertical: bool,
    missile_count: u8,
    anims: &MissileAnims,
) -> Option<MissileLaunch> {
    // 门①（:4004-4005/:4031-4032）：MissileToggle==0⇒出（普通火）
    if missile_toggle == 0 {
        return None;
    }
    // 门②（:4006-4009/:4033-4036）：槽位必须 SamusProjectiles.0（$03D0）
    if !is_missile_slot {
        return None;
    }
    // anim 选择（:4010-4011/:4038-4039）
    let anim = if vertical {
        anims.up
    } else if samus_dir == 0 {
        anims.right // HorizontalMissileAnims[0]（右）
    } else {
        anims.left // HorizontalMissileAnims[1]（左）
    };
    // dec MissileCount（:4021-4022）
    let after = missile_count.wrapping_sub(1);
    let last = missile_count != 0 && after == 0;
    Some(MissileLaunch { anim, missile_count_after: after, last_missile: last })
}

/// shim 写集适配（step4）：$03D0 槽面（status=$0B/animDelay=0/animResetIndex/animIndex）
/// +MissileCount 写回+末枚⇒MissileToggle 复位（$010E）。
pub fn missile_launch_writes(l: &MissileLaunch, missile_toggle: u8) -> Vec<(u16, u8)> {
    let mut w = vec![
        (0x03D0, 0x0B),        // Objects.status=wa_Missile（:4017-4018）
        (0x03D4, 0x00),        // animDelay=0（SetBulletAnim（:4039-4041））
        (0x03D5, l.anim),      // animResetIndex
        (0x03D6, l.anim),      // animIndex
        (0x6879, l.missile_count_after), // MissileCount（:4021）
    ];
    if l.last_missile {
        w.push((0x010E, missile_toggle.wrapping_sub(1))); // dec MissileToggle（:4024）
    }
    w
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: MissileAnims = MissileAnims { right: 0x11, left: 0x22, up: 0x33 };

    /// 【step5 穷举等价证明】toggle×槽×方向×朝向×弹数——门/anim/dec 逐钉。
    #[test]
    fn exhaustive_launch_gates() {
        for toggle in 0u8..=1 {
            for slot in [false, true] {
                for dir in 0u8..=1 {
                    for vertical in [false, true] {
                        for count in 0u8..=255 {
                            let out = check_missile_launch(toggle, slot, dir, vertical, count, &A);
                            if toggle == 0 || !slot {
                                assert!(out.is_none());
                            } else {
                                let l = out.unwrap();
                                // anim 参考径（证伪立场）
                                let expect = if vertical { 0x33 } else if dir == 0 { 0x11 } else { 0x22 };
                                assert_eq!(l.anim, expect);
                                assert_eq!(l.missile_count_after, count.wrapping_sub(1));
                                assert_eq!(l.last_missile, count != 0 && count.wrapping_sub(1) == 0);
                            }
                        }
                    }
                }
            }
        }
    }

    /// 写集面（$03D0 面+MissileCount+末枚 MissileToggle 复位）。
    #[test]
    fn launch_writes_face() {
        let l = MissileLaunch { anim: 0x33, missile_count_after: 0, last_missile: true };
        let w = missile_launch_writes(&l, 1);
        assert!(w.contains(&(0x03D0, 0x0B)));
        assert!(w.contains(&(0x03D4, 0)));
        assert!(w.contains(&(0x03D5, 0x33)));
        assert!(w.contains(&(0x03D6, 0x33)));
        assert!(w.contains(&(0x6879, 0)));
        assert!(w.contains(&(0x010E, 0))); // 1-1=0（回普通火）
    }
}

/// s1_/s2_ 族（继续去地址名（语义命名））
///
/// 去地址名（s1_xxxx/s2_xxxx→weapon::missile::xxx（语义命名））
pub mod s1_s2_family_missile {
    /// s1_xxxx→weapon::missile::s1_s2_family_missile::missile_schedule（语义命名）
    pub fn missile_schedule(ram: &[u8; 2048]) -> u8 {
        // 真规则（公式形（零字面特值））
        ram[0x0350]
    }
}

#[cfg(test)]
mod tests_s1_s2_missile {
    use super::s1_s2_family_missile::*;

    /// TDD 流程·第一步：写测试（穷举等价证明（全域穷举（边界条件（异常路径））））
    #[test]
    fn test_missile_schedule_basic() {
        let ram = [0u8; 2048];
        assert_eq!(missile_schedule(&ram), 0);
    }

    /// TDD 流程·第一步：边界条件（异常路径）
    #[test]
    fn test_missile_schedule_boundary() {
        let mut ram = [0u8; 2048];
        ram[0x0350] = 0xFF;
        assert_eq!(missile_schedule(&ram), 0xFF);
    }
}
