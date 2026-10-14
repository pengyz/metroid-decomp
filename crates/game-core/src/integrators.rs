//! game-core::integrators —— 移动积分器（bank07:E3A5-E3E4 垂直切片）。
//!
//! 等价性等级：**证明级**（2^32 输入域穷举，见 game-oracle bin `int-y-proof`）。
//! 语义来源：bank07:E3A5-E3E4 + 内嵌 sub_D147（D147=LDY#0/STY $308/STY $312/RTS，
//! 即钳位重置），seed metroid.asm 逐指令（A 级）。

/// 垂直积分器状态（= RAM 影子 $308/$312/$310）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntYState {
    /// $308 速度整数部（补码）
    pub speed_i: u8,
    /// $312 速度小数部
    pub speed_f: u8,
    /// $310 亚像素累加
    pub subpix: u8,
}

/// 垂直积分一步（bank07:E3A5，止于 E3E4 RTS）。返回本帧整像素位移（→$00，补码）。
///
/// 指令语义（逐条，seed 105390+）：
///   frac += accel（CLC/ADC，进位入 int）；int += carry；
///   负向（int 符号位）：幅值=0-frac 借位-int，幅值≥6 ⇒ 速度重置 {int:-6,frac:0}
///   正向：int≥5 ⇒ 速度重置 {int:5,frac:0}（sub_D147 清零 frac/int 后 STX int）；
///   subpix += frac（进位 c1）；位移输出 = int + c1。
/// 完整输出面（L2 面 = RAM 写集 + a/x/y 寄存器）。
pub struct IntYOut {
    /// 位移（→$00 与 A 末值）
    pub disp: u8,
    /// X 末值：正路径=5、负路径=0xFA（E3A5 LDX#5 / E3C5 LDX#$FA）
    pub reg_x: u8,
    /// Y 末值：钳位触发（D147 LDY#0）⇒ 0，否则原值
    pub reg_y: u8,
}

pub fn int_y(st: &mut IntYState, accel: u8) -> IntYOut {
    // E3A7-E3AE: speed_f += accel
    let (f, c) = st.speed_f.overflowing_add(accel);
    st.speed_f = f;
    // E3B1-E3B6: speed_i += carry（flags 由 ADC #0 的结果决定 = 新 speed_i）
    let (i, _) = st.speed_i.overflowing_add(u8::from(c));
    st.speed_i = i;

    let mut reg_x = 5u8;
    let mut reg_y = 0u8; // 调用方用 input y 覆盖：reg_y 标记是否被 D147 清零
    let mut y_wiped = false;
    if st.speed_i & 0x80 == 0 {
        // E3C9 正向：CMP #5（无符号），C=1 ⇔ speed_i >= 5 ⇒ 钳位（reg_x 保持 5）
        if st.speed_i >= 5 {
            // E3CD JSR D147（LDY#0/STY×2 清零）+ E3D0 STX $308（X=5）
            st.speed_i = 5;
            st.speed_f = 0;
            y_wiped = true;
        }
    } else {
        reg_x = 0xFA; // E3C5 LDX #$FA（负路径必执行）
        // E3BB-E3C3 负向：A = 0 - speed_f 的借位 - speed_i（SBC 语义）
        let c_cmp = u8::from(st.speed_f == 0);
        let a = 0u8
            .wrapping_sub(st.speed_i)
            .wrapping_sub(1 - c_cmp);
        // E3C3 CMP #6：C=1 ⇔ a >= 6（无符号）⇒ 钳位
        if a >= 6 {
            st.speed_i = 0xFA;
            st.speed_f = 0;
            y_wiped = true;
        }
    }

    // E3D3-E3DA: subpix += frac（c1）
    let (p, c1) = st.subpix.overflowing_add(st.speed_f);
    st.subpix = p;
    // E3DD-E3E2: 位移输出 = 0 + speed_i + c1
    IntYOut {
        disp: st.speed_i.wrapping_add(u8::from(c1)),
        reg_x,
        reg_y: if y_wiped { 0 } else { 0xFF }, // 0xFF=哨兵表示「保持 input.y」
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn int_y_plain_accumulation() {
        let mut s = IntYState { speed_i: 1, speed_f: 0x80, subpix: 0x70 };
        let out = int_y(&mut s, 0x10);
        // frac=0x90 无进位；int 不变；subpix=0x70+0x90=0x00 进位 → 位移=1+1=2
        assert_eq!(s, IntYState { speed_i: 1, speed_f: 0x90, subpix: 0x00 });
        assert_eq!(out.disp, 2);
    }

    #[test]
    fn int_y_positive_clamp_resets_to_5() {
        let mut s = IntYState { speed_i: 4, speed_f: 0xFF, subpix: 0 };
        let out = int_y(&mut s, 0x02);
        // frac=0x01 进位 → int=5 ⇒ >=5 钳位 → {5,0}；subpix=0+0 → 位移=5
        assert_eq!(s, IntYState { speed_i: 5, speed_f: 0, subpix: 0 });
        assert_eq!(out.disp, 5);
    }

    #[test]
    fn int_y_negative_clamp_resets_to_minus6() {
        let mut s = IntYState { speed_i: 0xFB, speed_f: 0x80, subpix: 0 };
        let out = int_y(&mut s, 0x00);
        // frac 不进位 int=0xFB(-5)；负向幅值 a = 0-0xFB-(1-0) = 0x04 < 6 ⇒ 不钳
        assert_eq!(s.speed_i, 0xFB);
        let mut s2 = IntYState { speed_i: 0xFA, speed_f: 0, subpix: 0 };
        let out2 = int_y(&mut s2, 0x00);
        // a = 0-0xFA-(1-1) = 0x06 ≥ 6 ⇒ 钳位 {-6,0}；位移=-6
        assert_eq!(s2, IntYState { speed_i: 0xFA, speed_f: 0, subpix: 0 });
        assert_eq!(out2.disp, 0xFA);
        let _ = out;
    }
}
