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

/// 水平积分器状态（= RAM 影子 $309/$313/$311）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntXState {
    /// $309 速度整数部（补码）
    pub speed_i: u8,
    /// $313 速度小数部
    pub speed_f: u8,
    /// $311 亚像素累加
    pub subpix: u8,
}

/// 水平积分器完整输出面（L2 面 = RAM 写集 + a/x/y）。
pub struct IntXOut {
    /// 位移（→$00 末值与 A 末值）
    pub disp: u8,
    /// X 末值：正路径=new frac、负路径=-frac（E403 TAX / E41C TAX）
    pub reg_x: u8,
    /// Y 末值：正路径=new int、负路径=neg int（E413 TAY / E422 TAY）
    pub reg_y: u8,
    /// $01 末值（限幅高字节；负路径经 sub_E449 取负）
    pub lim_hi_final: u8,
}

/// 水平积分一步（bank07:E3E5，止于 E448 RTS）。
///
/// 语义（逐条，seed 105440+）：
///   limit16 = 16 × max_src（$316：E3E8 sub_C2C5=<<4 → lo、E3F2 sub_C2BF=>>4 → hi）；
///   frac += accel（CLC/ADC）；int += carry + (accel<0 ? 0xFF : 0)（BIT $315 测 accel 符号）；
///   负路径：(X,Y)=|speed|（SEC/SBC 取负链）且 limit 取负（sub_E449）；
///   16 位无符号 |speed| ≥ limit ⇒ 速度重置为 ±limit（frac:=$00、int:=$01）；
///   subpix += frac（c1）；位移 = int + c1。
pub fn int_x(st: &mut IntXState, accel: u8, max_src: u8) -> IntXOut {
    let lim_lo = max_src << 4; // $02/$00 初值
    let lim_hi = max_src >> 4; // $03/$01 初值

    // E3F9-E400: frac += accel
    let (f, c) = st.speed_f.overflowing_add(accel);
    st.speed_f = f;
    // E404-E410: int += carry + (accel 符号扩展)
    let sign_ext = if accel & 0x80 != 0 { 0xFFu8 } else { 0u8 };
    let t = st.speed_i.wrapping_add(sign_ext);
    let (i, c2) = t.overflowing_add(u8::from(c));
    let _ = c2;
    st.speed_i = i;

    let reg_x;
    let reg_y;
    let mut lim0 = lim_lo;
    let mut lim1 = lim_hi;

    if st.speed_i & 0x80 == 0 {
        // 正路径：X=new frac、Y=new int
        reg_x = st.speed_f;
        reg_y = st.speed_i;
    } else {
        // 负路径：X=-frac（SEC/SBC），Y=-int-borrow；limit 取负（sub_E449）
        reg_x = 0u8.wrapping_sub(st.speed_f);
        let borrow = u8::from(st.speed_f == 0); // SBC 后 C=1 ⇔ frac==0
        reg_y = 0u8.wrapping_sub(st.speed_i).wrapping_sub(1 - borrow);
        // 16 位取负：(lim1:lim0) = -(lim_hi:lim_lo)
        let new_lo = 0u8.wrapping_sub(lim_lo);
        let b2 = u8::from(lim_lo == 0);
        lim0 = new_lo;
        lim1 = 0u8.wrapping_sub(lim_hi).wrapping_sub(1 - b2);
    }

    // E426-E42B: 16 位无符号 (reg_y:reg_x) vs ($03:$02)=(lim_hi:lim_lo)——CPX/SBC 借位链
    let c_lo = reg_x >= lim_lo;
    let sub_hi = reg_y.wrapping_sub(lim_hi).wrapping_sub(u8::from(!c_lo));
    let borrow = reg_y < lim_hi || (reg_y == lim_hi && !c_lo);
    let _ = sub_hi;
    if !borrow {
        // 钳位：frac := $00、int := $01（负路径已取负 ⇒ ±limit）
        st.speed_f = lim0;
        st.speed_i = lim1;
    }

    // E437-E43E: subpix += frac
    let (p, c1) = st.subpix.overflowing_add(st.speed_f);
    st.subpix = p;
    // E441-E446: 位移 = int + c1
    IntXOut {
        disp: st.speed_i.wrapping_add(u8::from(c1)),
        reg_x,
        reg_y,
        lim_hi_final: lim1,
    }
}

#[cfg(test)]
mod tests_x {
    use super::*;

    #[test]
    fn int_x_plain_accumulation() {
        let mut s = IntXState { speed_i: 0, speed_f: 0x10, subpix: 0xF0 };
        let out = int_x(&mut s, 0x20, 0x10); // limit=0x0100
        // frac=0x30 无进位；int=0+0+0=0；|0x0030|<0x0100 不钳；subpix=0xF0+0x30=0x20 进位
        assert_eq!(s, IntXState { speed_i: 0, speed_f: 0x30, subpix: 0x20 });
        assert_eq!(out.disp, 1);
    }

    #[test]
    fn int_x_positive_clamp_to_limit() {
        let mut s = IntXState { speed_i: 0x02, speed_f: 0x00, subpix: 0 };
        let out = int_x(&mut s, 0x00, 0x10); // limit=0x0100，speed=0x0200 ≥ limit
        assert_eq!(s, IntXState { speed_i: 0x01, speed_f: 0x00, subpix: 0 });
        assert_eq!(out.disp, 1);
        assert_eq!(out.reg_x, 0x00); // 钳位前 X=new frac=0
        assert_eq!(out.reg_y, 0x02); // 钳位前 Y=new int=2
    }

    #[test]
    fn int_x_negative_clamp_to_neg_limit() {
        let mut s = IntXState { speed_i: 0xFE, speed_f: 0x00, subpix: 0 };
        let out = int_x(&mut s, 0x00, 0x10);
        // int=0xFE(-2)；|0xFE00|=0x0200 ≥ 0x0100 ⇒ 钳位 -0x0100 → {0x00,0xFF}
        assert_eq!(s, IntXState { speed_i: 0xFF, speed_f: 0x00, subpix: 0 });
        assert_eq!(out.disp, 0xFF);
        assert_eq!(out.lim_hi_final, 0xFF); // limit 取负高字节
    }

    #[test]
    fn int_x_accel_sign_extension() {
        let mut s = IntXState { speed_i: 0x05, speed_f: 0xFF, subpix: 0 };
        let out = int_x(&mut s, 0x01, 0xFF); // frac=0x00 进位；accel≥0 ⇒ int=5+1=6
        assert_eq!(s.speed_i, 6);
        let mut s2 = IntXState { speed_i: 0x05, speed_f: 0x00, subpix: 0 };
        int_x(&mut s2, 0x80, 0xFF); // frac=0x80 无进位；accel<0 ⇒ int=5+0xFF=4
        assert_eq!(s2.speed_i, 4);
        let _ = out;
    }
}
