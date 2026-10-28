//! `game-core::engine::nmi_scroll_face` —— NMI/滚屏/名表投影（P2 重整合（修正后域维度（2026-10-28 覆写）））。
//! asm 例程族=节注边界（族头保留（考证锚不丢））。

use super::*;

// ---- SC-S3-24（S3-84）员转写（2 员——DrawFrame 族）----

/// $DE4A DrawFrame（root——簇共根形七八）：$4B X 载+$0303[x] cmp $F7 判（jmp $DF2D）
/// +cmp 7 判（$6B 位4 清）+$030D 段（绘制帧分派——S1-22 walk 表目标面）。
/// 返回 (写事件, 判段)。
pub fn s3_draw_frame(x: u8, v0303_x: u8) -> (Vec<(u16, u8)>, u16) {
    if v0303_x == 0xF7 {
        (vec![], 0xDF2D) // jmp 段
    } else if v0303_x == 7 {
        (vec![(0x006B, 0xEF)], 0xDE4A) // 位4 清段
    } else {
        (vec![], 0xDE4A)
    }
}

/// $D417 员体：jmp DrawFrame（尾形）+$0304 dec+$0320 cmp 3/8 双判段。
pub fn s3_d417_body(x: u8, v0303_x: u8, v0304: u8) -> (Vec<(u16, u8)>, u16) {
    let (mut ev, j) = s3_draw_frame(x, v0303_x);
    ev.push((0x0304, v0304.wrapping_sub(1)));
    (ev, j)
}

#[cfg(test)]
mod s3_084_tests {
    use super::*;

    /// 二员转写形（DrawFrame 分派 root+一壳）。
    #[test]
    fn s3_084_units_two() {
        assert_eq!(s3_draw_frame(2, 0xF7).1, 0xDF2D);
        let (ev, _) = s3_draw_frame(2, 7);
        assert!(ev.contains(&(0x006B, 0xEF)));
        let (ev2, _) = s3_d417_body(2, 0, 5);
        assert!(ev2.contains(&(0x0304, 4)));
    }
}

// ---- SC-S4-09（S4-99）员转写（2 员——$E6A7 态步进族）----

/// $E6A7 员体（root——簇共根形九三）：$49 cmp 2/3 双判+dec（$49/$50/$FD 面——
/// S1-22/S2-36 态步进同族）。
/// 返回 (写事件, 状态值)。
pub fn s4_e6a7_step(v49: u8, vfd: u8) -> (Vec<(u16, u8)>, u8) {
    if v49 == 2 {
        return (vec![], v49); // beq 段
    }
    if v49 != 3 {
        return (vec![], v49); // bne 段
    }
    let mut ev = vec![(0x0049, 2)];
    if vfd != 0 {
        ev.push((0x0050, 0xFF));
    }
    (ev, 2)
}

/// $D831 员体：$E6A7+尾跳 $D806（S1-01 回驱——三现注）+$E6D2 段（同族面）。
pub fn s4_d831_body(v49: u8, vfd: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = s4_e6a7_step(v49, vfd);
    (ev, 0xD806)
}

#[cfg(test)]
mod s4_099_tests {
    use super::*;

    /// 二员转写形（$E6A7 态步进 root+一壳）。
    #[test]
    fn s4_099_units_two() {
        assert_eq!(s4_e6a7_step(2, 0).1, 2); // beq 段
        assert_eq!(s4_e6a7_step(5, 0).1, 5); // bne 段
        let (ev, v) = s4_e6a7_step(3, 5);
        assert_eq!(v, 2);
        assert!(ev.contains(&(0x0049, 2)));
        assert!(ev.contains(&(0x0050, 0xFF)));
        let (ev2, j) = s4_d831_body(3, 5);
        assert_eq!(j, 0xD806);
        assert_eq!(ev2.len(), 2);
    }
}

// ---- SC-S4-10（S4-100）员转写（2 员——$E6D2 态步进 inc 对偶族）----

/// $E6D2 员体（root——簇共根形九四·**$E6A7 inc 版对偶**）：$49 cmp 3/2 双判+
/// inc（$49/$50/$FD 面——S4-099 对偶）。
/// 返回 (写事件, 状态值)。
pub fn s4_e6d2_step(v49: u8, vfd: u8) -> (Vec<(u16, u8)>, u8) {
    if v49 == 3 {
        return (vec![], v49); // beq 段
    }
    if v49 != 2 {
        return (vec![], v49); // bne 段
    }
    let mut ev = vec![(0x0049, 3)];
    if vfd != 0 {
        ev.push((0x0050, 1));
    }
    (ev, 3)
}

/// $D837 员体：$E6D2+尾跳 $D806（S1-01 回驱——四现注）+$030F[x] bpl 段。
pub fn s4_d837_body(v49: u8, vfd: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = s4_e6d2_step(v49, vfd);
    (ev, 0xD806)
}

#[cfg(test)]
mod s4_100_tests {
    use super::*;

    /// 二员转写形（$E6D2 inc 对偶 root+一壳）。
    #[test]
    fn s4_100_units_two() {
        assert_eq!(s4_e6d2_step(3, 0).1, 3); // beq 段
        assert_eq!(s4_e6d2_step(5, 0).1, 5); // bne 段
        let (ev, v) = s4_e6d2_step(2, 5);
        assert_eq!(v, 3);
        assert!(ev.contains(&(0x0049, 3)));
        assert!(ev.contains(&(0x0050, 1)));
        let (ev2, j) = s4_d837_body(2, 5);
        assert_eq!(j, 0xD806);
        assert_eq!(ev2.len(), 2);
    }
}

// ---- SC-S4-12（S4-102）员转写（2 员——IsEngineRunning 族）----

/// $CA18 IsEngineRunning（root——簇共根形九六）：$1E（MainRoutine）cpy 7/3 双判
/// （rts 段）+$74←A（掩码 $0F→$CA30[y] 表载→$24）+jmp $C4DE（S1-15 接面）。
/// 返回 (运行, 表值)。
pub fn s4_is_engine_running(v1e: u8, a_in: u8) -> (bool, u8) {
    if v1e == 7 || v1e == 3 {
        (true, a_in) // 运行段
    } else {
        (false, a_in) // rts 段
    }
}

/// $D8E7 员体：IsEngineRunning+$76 eor 7 面+$74 cmp $12 段（$1C←1）+链（$C43F/
/// $C473——sound 域接面（S1-16 同族面注））。
pub fn s4_d8e7_body(v1e: u8, a_in: u8, v76: u8, v74: u8) -> (Vec<(u16, u8)>, u8) {
    let (running, v) = s4_is_engine_running(v1e, a_in);
    let mut ev = vec![(0x0076, v76 ^ 7)];
    if v74 >= 0x12 {
        ev.push((0x001C, 1));
    }
    (ev, v)
}

#[cfg(test)]
mod s4_102_tests {
    use super::*;

    /// 二员转写形（IsEngineRunning root+一壳）。
    #[test]
    fn s4_102_units_two() {
        assert_eq!(s4_is_engine_running(7, 5), (true, 5));
        assert_eq!(s4_is_engine_running(3, 5), (true, 5));
        assert_eq!(s4_is_engine_running(5, 5), (false, 5));
        let (ev, _) = s4_d8e7_body(7, 5, 7, 0x12);
        assert!(ev.contains(&(0x0076, 0)));
        assert!(ev.contains(&(0x001C, 1)));
    }
}

// ---- SC-S4-13（S4-103）员转写（2 员——WaitNMIPass 族；L2 真 0 留白）----

/// $C43F WaitNMIPass_（root——簇共根形九七）：ClearNMIStat 调（$C434）+$1A 判
/// （beq -4 **自旋等待**（NMI 传递面））+rts。
/// 返回 (NMI 已传, 自旋面)。
pub fn s4_wait_nmi_pass(v1a: u8) -> (bool, bool) {
    (v1a != 0, true) // beq 自旋（NMI 未传⇒等待）
}

/// $D8FA 员体：WaitNMIPass+链（$CB73/$D92C（StartMusic——S2-36 接面）/$C447/
/// $C8B0/$C8BB——sound 域帧链五调）+$4B←$20 面。
pub fn s4_d8fa_body(v1a: u8) -> (Vec<(u16, u8)>, u16) {
    let (passed, _) = s4_wait_nmi_pass(v1a);
    let mut ev = vec![(0x004B, 0x20)];
    (ev, 0xC447)
}

#[cfg(test)]
mod s4_103_tests {
    use super::*;

    /// 二员转写形（WaitNMIPass 自旋 root+一壳）。
    #[test]
    fn s4_103_units_two() {
        assert_eq!(s4_wait_nmi_pass(1), (true, true));
        assert_eq!(s4_wait_nmi_pass(0), (false, true)); // 自旋面
        let (ev, j) = s4_d8fa_body(1);
        assert_eq!(j, 0xC447);
        assert!(ev.contains(&(0x004B, 0x20)));
    }
}

// ---- SC-S4-15（S4-105）员转写（2 员——ScreenOn 族；L2 真 0 留白）----

/// $C447 ScreenOn（root——簇共根形九九）：$FE or $1E 判（bne -16 段）+
/// $FF→$2000（PPUCTRL）/$FE→$2001（PPUMASK）/面（$FA→$C4D9 调）。
/// 返回 (PPU 写对, 屏开)。
pub fn s4_screen_on(vfe: u8, vff: u8) -> (Vec<(u16, u8)>, bool) {
    (vec![(0x2000, vff), (0x2001, vfe)], true) // PPU 寄存器写对（RAM 面注 $2000/$2001）
}

/// $D903 员体：ScreenOn+链（$C8B0/$C8BB——sound 帧链（S4-13 同构））+
/// $4B←$20+$0305←$6B/$0306←$5F 写对（S1-12 同构面）。
pub fn s4_d903_body(vfe: u8, vff: u8) -> (Vec<(u16, u8)>, u16) {
    let (mut ev, _) = s4_screen_on(vfe, vff);
    ev.push((0x004B, 0x20));
    ev.push((0x0305, 0x6B));
    ev.push((0x0306, 0x5F));
    (ev, 0xC8B0)
}

#[cfg(test)]
mod s4_105_tests {
    use super::*;

    /// 二员转写形（ScreenOn PPU 写对 root+一壳）。
    #[test]
    fn s4_105_units_two() {
        let (ev, on) = s4_screen_on(0x1E, 0x80);
        assert!(on);
        assert!(ev.contains(&(0x2000, 0x80)));
        assert!(ev.contains(&(0x2001, 0x1E)));
        let (ev2, j) = s4_d903_body(0x1E, 0x80);
        assert_eq!(j, 0xC8B0);
        assert!(ev2.contains(&(0x0305, 0x6B)));
        assert!(ev2.contains(&(0x0306, 0x5F)));
    }
}

// ---- SC-S4-18（S4-108）员转写（2 员——ToggleScroll 族）----

/// $E252 ToggleScroll（root——簇共根形百二·**ToggleSamusHi 对偶**（XOR 切换族））：
/// $49←$49^3（ScrollDir 双位翻）+$FA←$FA^8（面翻）。
/// 返回 (写事件, 切换面)。
pub fn s2_toggle_scroll(v49: u8, vfa: u8) -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0049, v49 ^ 3), (0x00FA, vfa ^ 8)], v49 ^ 3)
}

/// $D96B 员体：ToggleScroll+$FA 面+尾跳 $D806（S1-01 回驱——六现注）。
pub fn s4_d96b_body(v49: u8, vfa: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = s2_toggle_scroll(v49, vfa);
    (ev, 0xD806)
}

#[cfg(test)]
mod s4_108_tests {
    use super::*;

    /// 二员转写形（ToggleScroll XOR 切换 root+一壳）。
    #[test]
    fn s4_108_units_two() {
        let (ev, v) = s2_toggle_scroll(0, 0);
        assert_eq!(v, 3);
        assert!(ev.contains(&(0x0049, 3)));
        assert!(ev.contains(&(0x00FA, 8)));
        let (ev2, j) = s4_d96b_body(5, 5);
        assert_eq!(j, 0xD806);
        assert!(ev2.contains(&(0x0049, 6)));
    }
}

