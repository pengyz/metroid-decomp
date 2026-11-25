//! arith —— 6502 算术/移位原语（纯函数，零内存访问）。
//!
//! 语义证据（A 级）：`refs/m1disasm/SRC/prg7_engine.asm` 的例程体，且逐字节对上 ROM
//! （bank7 文件偏移 = `16 + 7*16384 + (addr - $C000)`）：
//!
//! ```text
//! $C2A8 AddYToPtr00  98 18 65 00 85 00 90 02 E6 01 60   ; tya/clc/adc $00/sta $00/bcc +2/inc $01/rts
//! $C2BE Adiv32       4A 4A 4A 4A 4A 60                  ; lsr ×5
//! $C2BF Adiv16       （Adiv32+1：从第 2 条 lsr 进入 ⇒ 共 ×4）
//! $C2C0 Adiv8        （Adiv32+2：共 ×3）
//! $C2C4 Amul32       0A 0A 0A 0A 0A 60                  ; asl ×5
//! $C2C5 Amul16       （Amul32+1 ⇒ 共 ×4）
//! $C2C6 Amul8        （Amul32+2 ⇒ 共 ×3）
//! $C3D4 TwosComplement 49 FF 18 69 01 60               ; eor #$FF/clc/adc #$01/rts
//! ```
//!
//! 派生关系：六条移位例程共用同一串 `asl`/`lsr`，`Amul32 $C2C4` / `Adiv32 $C2BE` 是入口，
//! `*16` = 入口 +1 条、`*8` = 入口 +2 条。进位的位号因此是 `8-N`（乘法）与 `N-1`（除法），
//! **不是** `N`——`amul32` 的进位是入参 bit3（不是 bit5），`adiv32` 是 bit4。
//!
//! 标志位：本模块所有原语只写 C/N/Z，不写 V（`ASL`/`LSR` 不碰 V；`TwosComplement` 的
//! `adc #$01` 永不出带符号溢出）。用 [`finish_flags`] 把原语返回的 A 与 C 合成 [`Flags`]。
//!
//! **口径更正（对照 #41 卡面的措辞）**：`Amul16`/`Adiv16` 不是「8×16 位乘、返回 16 位」，
//! 而是**对 A 这个 8 位累加器连续移位**（4 条 `asl`/`lsr`）——结果是 `(a<<4) & 0xFF`、
//! 进位 = a 的 bit4。ROM 侧 16 位结果由调用者自己拼（低半字节先用 `and #$0F` 取、再
//! `jsr Amul16` 移位后 `ora` 回去，例 `SetMirrorCntrlBit` 07:CD92：`lda SamusDir / jsr
//! Amul16 / ora ObjectCntrl / sta ObjectCntrl` 把方向 bit0 挪到 bit4）。这里按 ROM 实现，
//! 并**同时返回进位**，因为调用者确实消费它（`room_draw.rs:280` 记的「Amul16 CF=bit4」）。
//!
//! 约定（三条例程共有）：A 进 A 出；X/Y 不变；除 `AddYToPtr00` 读写 `$00/$01` 外不碰内存。
//! 标志位：每个原语都返回 `(A, C)`，N/Z 用 [`finish_flags`] 补（`N=A.bit7`、`Z=(A==0)`）；
//! V 不受影响也不建模（见上）。
//!
//! 除零不存在：`Adiv8`/`Adiv16` 的除数是常量移位，没有运行期除数，故没有除零分支需要定义
//! （与 `Amul*` 对称）。

/// `asl` 的常量平移：返回 `(A, C)`。
///
/// `asl` 每移一位把移出的 bit7 送进位，所以 n 次后进位 = 入参的 bit(8-n)、
/// 结果 = `(a << n) & 0xFF`。
#[inline]
fn asl_n(a: u8, n: u32) -> (u8, bool) {
    debug_assert!(n >= 1 && n <= 8);
    let carry = (a >> (8 - n)) & 1 != 0;
    (a.wrapping_shl(n), carry)
}

/// `lsr` 的常量平移：返回 `(A, C)`。
///
/// `lsr` 每移一位把移出的 bit0 送进位，所以 n 次后进位 = 入参的 bit(n-1)、
/// 结果 = `a >> n`。
#[inline]
fn lsr_n(a: u8, n: u32) -> (u8, bool) {
    debug_assert!(n >= 1 && n <= 8);
    let carry = (a >> (n - 1)) & 1 != 0;
    (a >> n, carry)
}

/// `Amul32` —— bank7 `$C2C4`：`asl` ×5 ⇒ **A = (a<<5) & 0xFF，C = a.bit3**。
///
/// 派生关系：`Amul32` 是整段例程的入口，`Amul16`(`$C2C5`)=+1 条、`Amul8`(`$C2C6`)=+2 条，
/// 三者共用同一串 `asl`，只是从不同 PC 进入 ⇒ 本模块三条函数都是同一原语的「少移 n 位」特例。
/// 进位的位号随之递推：×32 取 bit3、×16 取 bit4、×8 取 bit5（**注意：是 `8-n`，不是 `n`**）。
///
/// 调用者例：xref 少于 Amul16，主要在把 5 位量搬到高位的场合（如槽号 → 对象步长 ×32）。
#[inline]
pub fn amul32(a: u8) -> (u8, bool) {
    asl_n(a, 5)
}

/// `Amul8` —— bank7 `$C2C6`（`Amul32+2`）：`asl` ×3 ⇒ **A = (a<<3) & 0xFF，C = a.bit5**。
///
/// 调用者例：`prg7_engine.asm:8259` 敌门槽号 `txa / jsr Amul8 / ora #$80`（槽号 ×8 即对象
/// 结构步长）；`:10647` 尾调用 `jmp Amul8`（把命中标志 ×8 交给调用者，返回即带 C）。
#[inline]
pub fn amul8(a: u8) -> (u8, bool) {
    asl_n(a, 3)
}

/// `Amul16` —— bank7 `$C2C5`（`Amul32+1`）：`asl` ×4 ⇒ **A = (a<<4) & 0xFF，C = a.bit4**。
///
/// 调用者例：`SetMirrorCntrlBit` 07:CD92 把朝向 bit0 挪到 bit4；`HorzAccelerate` 07:E3E5
/// 用 `jsr Amul16` 取最大速度的低半字节（亚像素）存 `Temp00_SpeedMaxSubPixel`。
#[inline]
pub fn amul16(a: u8) -> (u8, bool) {
    asl_n(a, 4)
}

/// `Adiv32` —— bank7 `$C2BE`：`lsr` ×5 ⇒ **A = a>>5，C = a.bit4**。
///
/// 派生关系与乘法侧对称：`Adiv32` 是入口，`Adiv16`(`$C2BF`)=+1 条、`Adiv8`(`$C2C0`)=+2 条，
/// 同一串 `lsr` 的不同进入点。进位的位号 = `n-1`：÷32 取 bit4、÷16 取 bit3、÷8 取 bit2。
#[inline]
pub fn adiv32(a: u8) -> (u8, bool) {
    lsr_n(a, 5)
}

/// `Adiv8` —— bank7 `$C2C0`（`Adiv32+2`）：`lsr` ×3 ⇒ **A = a>>3，C = a.bit2**。
///
/// 调用者例：`prg7_engine.asm:8036` 列流送地址 `lda ScrollX / and #$F8 / jsr Adiv8 / sta
/// Temp00_RoomRAMPtr`（像素→瓦片号；`and #$F8` 让低三位清零，故 C 恒 0）。
#[inline]
pub fn adiv8(a: u8) -> (u8, bool) {
    lsr_n(a, 3)
}

/// `Adiv16` —— bank7 `$C2BF`（`Adiv32+1`）：`lsr` ×4 ⇒ **A = a>>4，C = a.bit3**。
///
/// 调用者例：`prg7_engine.asm:898` `lda (ptr),y / jsr Adiv16 / sta Temp04_VRAMStringHeight`
/// （字节高半字节=行数）；`HorzAccelerate` 07:E3E5 取最大速度的像素部分。
#[inline]
pub fn adiv16(a: u8) -> (u8, bool) {
    lsr_n(a, 4)
}

/// `TwosComplement` —— bank7 `$C3D4`（`49 FF 18 69 01 60`）：**A = (~a + 1) & 0xFF，C = (a == 0)**。
///
/// 进位只在入参为 0 时置位：`eor #$FF` 得 `$FF`，`clc/adc #$01` 进位产生；其余入参
/// `~a + 1 ≤ $FF` 不再进位。等价于 `a.wrapping_neg()`（`$80` → `$80`，`$00` → `$00`）。
///
/// 调用者例：`prg7_engine.asm:3145/3346/4390` 把速度/位移取负。
#[inline]
pub fn twos_complement(a: u8) -> (u8, bool) {
    (a.wrapping_neg(), a == 0)
}

/// `AddYToPtr00` —— bank7 `$C2A8`：把 Y 加到 `$00/$01` 的 16 位指针上。
///
/// ROM 序：`tya / clc / adc $00 / sta $00 / bcc @RTS / inc $01`——低字节相加产生进位时
/// 高字节 +1；高字节 `$FF` 再 +1 会回绕到 `$00`（ROM 无越界检查，这里同样回绕）。
/// 返回 `(新指针, 低字节进位)`；进位在 `bcc @RTS` 之后仍可被调用者消费。
/// RTS 时 A=低字节和（`adc $00` 的结果），故 Z/N 由低字节决定——要复刻这两个标志用
/// `finish_flags(new_ptr as u8, carry)`（`as u8` 取的就是低字节）。
///
/// 调用者例：`prg7_engine.asm:852`（VRAM 结构串下一段）、`:925`（名表换行 `ldy #$20`）、
/// `:9837`（元瓦片推进 `ldy #$02`）。
#[inline]
pub fn add_y_to_ptr00(ptr: u16, y: u8) -> (u16, bool) {
    let lo = (ptr & 0x00FF) as u8;
    let (sum, carry) = lo.overflowing_add(y);
    let hi = (ptr >> 8) as u8;
    let hi = if carry { hi.wrapping_add(1) } else { hi };
    (u16::from_le_bytes([sum, hi]), carry)
}

/// 例程返回时的 6502 标志位（**V 不在此列**，见下）。
///
/// 这些原语都是「移位」或「`eor`+`adc`」，共同点是：只写 C/N/Z，**不写 V**——6502 的
/// `ASL`/`LSR` 不碰 V，`ADC` 才会按 (`A7==M7` && `r7!=A7`) 置 V，而 `TwosComplement` 的
/// `adc #$01` 永不出带符号溢出（`-a` 只有 `a==$80` 会「溢出」，但 `$80` 取负仍是 `$80`，
/// 两边同为负 ⇒ V=0）。故本模块不提供 V 字段；调用者若需要 V，只能来自其它加法/比较路径。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Flags {
    /// 进位：由具体原语决定（×N 取 bit(8-N)、÷N 取 bit(N-1)、取补取 a==0、指针加取低字节进位）。
    pub carry: bool,
    /// Z：结果 A == 0。
    pub zero: bool,
    /// N：结果 A 的 bit7。
    pub negative: bool,
}

/// 按 6502 规则补齐 N/Z 标志：`A` 进 `A/C` 出，`Z = (A==0)`、`N = A.bit7`、C 原样透传。
///
/// 用法（先做原语、再补标志）：
/// ```ignore
/// let (a, c) = amul16(x);
/// let f = finish_flags(a, c);   // f.carry / f.zero / f.negative
/// ```
///
/// 与卡面建议的 `finish_flags(a) -> (a, carry, zero, negative)` 的差别：**进位必须由调用方
/// 把原语返回的 C 传进来**——只给 A 无法还原进位（`0x80` 与 `0x00` 都可能来自不同的 C）。
/// 返回类型用具名字段而非四元组，避免调用点 `let (_, _, z, n) = ...` 读串位置。
///
/// # ROM 里确实消费这些标志的调用点
///
/// 本模块自身不接业务代码；下表是对 `refs/m1disasm/SRC/prg7_engine.asm` 逐处核对的结果
/// （判据：call 之后到分支之间只有不写标志的指令，如 `sta`/`lda`），移植对应调用点时必须
/// 还原这些标志：
///
/// | 例程 | 清单行 | 消费者 | 依赖 |
/// |---|---|---|---|
/// | `Adiv32` | :4062（`07:D375` 一带）| `lda #$00` / `bcs Lx048` / `lda #$0C` → `WeaponWaveInstrID` | **C**（入参 bit4 决定武器波形的 0/0x0C）|
/// | `Amul8` | :7053 | `bcs LE23D`（门房是否切换滚动）| **C** |
/// | `Amul16` | :9062 | `bcc @endIf_A`（左门 Y 调整）| **C** |
/// | `Adiv16` | :6834 | `sta Temp01_FullEnergyTanks` 后 `bne @endIf_emptyEnergyTanks` | **Z**（满能量罐数是否为 0）|
///
/// 反例（**不**消费本模块标志，别被相邻分支误导）：`:3132` `Adiv16` 后是 `cmp #$03`+`bcs`
/// （C 来自 `cmp`）；`:7189`/`:7292` `TwosComplement` 后先 `ldy SamusInLava` 再 `beq`（Z 来自
/// `ldy`）；`:12630` 的 `bcc` 前有 `cpy #$80`（C 来自 `cpy`）。全仓逐处扫描未发现这些例程后
/// 紧跟 `bmi/bpl` 的调用点，故 N 当前无消费者——但仍按 6502 语义提供。
#[inline]
pub fn finish_flags(a: u8, carry: bool) -> Flags {
    Flags {
        carry,
        zero: a == 0,
        negative: a & 0x80 != 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Amul8 / Amul16：闭式 (a<<n) & 0xFF、C = a.bit(8-n) ----

    #[test]
    fn amul8_rom_vectors() {
        // 0x11<<3 = 0x88，bit5(0x11)=0 ⇒ C=0
        assert_eq!(amul8(0x11), (0x88, false));
        // 0x20<<3 溢出到 0x00，bit5(0x20)=1 ⇒ C=1（进位可见）
        assert_eq!(amul8(0x20), (0x00, true));
        assert_eq!(amul8(0x00), (0x00, false));
        assert_eq!(amul8(0x01), (0x08, false));
        assert_eq!(amul8(0x80), (0x00, false));
        assert_eq!(amul8(0xFF), (0xF8, true));
        // 敌门槽号现场：txa / jsr Amul8；槽 3 ⇒ 24
        assert_eq!(amul8(0x03).0, 0x18);
    }

    #[test]
    fn amul16_rom_vectors() {
        // SetMirrorCntrlBit 07:CD92：SamusDir bit0 → ObjectCntrl bit4
        assert_eq!(amul16(0x01), (0x10, false));
        assert_eq!(amul16(0x00), (0x00, false));
        assert_eq!(amul16(0x0F), (0xF0, false)); // bit4(0x0F)=0
        assert_eq!(amul16(0x10), (0x00, true)); // bit4(0x10)=1
        assert_eq!(amul16(0x80), (0x00, false));
        assert_eq!(amul16(0xFF), (0xF0, true));
    }

    // ---- Adiv8 / Adiv16：闭式 a>>n、C = a.bit(n-1) ----

    #[test]
    fn adiv8_rom_vectors() {
        // 列流送瓦片号：ScrollX=0xF8 时 and #$F8 后 /8 = 0x1F
        assert_eq!(adiv8(0xF8), (0x1F, false));
        assert_eq!(adiv8(0xFF), (0x1F, true)); // bit2(0xFF)=1
        assert_eq!(adiv8(0x07), (0x00, true)); // bit2(0x07)=1
        assert_eq!(adiv8(0x08), (0x01, false));
        assert_eq!(adiv8(0x00), (0x00, false));
    }

    #[test]
    fn adiv16_rom_vectors() {
        // VRAM 串高半字节=行数：0x35 ⇒ 3 行
        assert_eq!(adiv16(0x35), (0x03, false));
        assert_eq!(adiv16(0x0F), (0x00, true)); // bit3(0x0F)=1
        assert_eq!(adiv16(0x10), (0x01, false));
        assert_eq!(adiv16(0x80), (0x08, false));
        assert_eq!(adiv16(0xFF), (0x0F, true));
        assert_eq!(adiv16(0x00), (0x00, false));
    }

    // ---- TwosComplement：A = wrapping_neg，C = (a == 0) ----

    #[test]
    fn twos_complement_rom_vectors() {
        // $C3D4：eor #$FF / clc / adc #$01
        assert_eq!(twos_complement(0x00), (0x00, true)); // $FF+1 进位
        assert_eq!(twos_complement(0x01), (0xFF, false));
        assert_eq!(twos_complement(0x80), (0x80, false)); // -128 取负仍是 -128
        assert_eq!(twos_complement(0x7F), (0x81, false));
        assert_eq!(twos_complement(0xFF), (0x01, false));
    }

    // ---- AddYToPtr00：16 位加 + 低字节进位；高位回绕 ----

    #[test]
    fn add_y_to_ptr00_rom_vectors() {
        // VRAM 结构串推进：$07A1 + 0x20
        assert_eq!(add_y_to_ptr00(0x07A1, 0x20), (0x07C1, false));
        // 名表换行：$00F0 + 0x10 跨页 ⇒ 高字节 +1、进位置位
        assert_eq!(add_y_to_ptr00(0x00F0, 0x10), (0x0100, true));
        assert_eq!(add_y_to_ptr00(0x07FF, 0x01), (0x0800, true));
        // 高字节 $FF 再 +1 回绕（ROM 无检查）
        assert_eq!(add_y_to_ptr00(0xFFFF, 0x01), (0x0000, true));
        assert_eq!(add_y_to_ptr00(0x00FF, 0x00), (0x00FF, false));
        assert_eq!(add_y_to_ptr00(0x0000, 0xFF), (0x00FF, false));
    }

    // ---- 全值域闭式校验：把 ROM 的「n 次移位」定义与实现逐值对齐 ----

    #[test]
    fn shift_family_matches_closed_form_over_full_domain() {
        for a in 0u16..=0xFF {
            let a = a as u8;
            assert_eq!(amul8(a), ((a << 3) & 0xFF, a & 0x20 != 0), "amul8({a:02X})");
            assert_eq!(amul16(a), ((a << 4) & 0xFF, a & 0x10 != 0), "amul16({a:02X})");
            assert_eq!(amul32(a), ((a << 5) & 0xFF, a & 0x08 != 0), "amul32({a:02X})");
            assert_eq!(adiv8(a), (a >> 3, a & 0x04 != 0), "adiv8({a:02X})");
            assert_eq!(adiv16(a), (a >> 4, a & 0x08 != 0), "adiv16({a:02X})");
            assert_eq!(adiv32(a), (a >> 5, a & 0x10 != 0), "adiv32({a:02X})");
            assert_eq!(twos_complement(a), (a.wrapping_neg(), a == 0), "twos({a:02X})");
        }
    }

    // ---- Amul32 / Adiv32：入口例程，比 *16 多一条移位（进位位号 bit3 / bit4） ----

    #[test]
    fn amul32_rom_vectors() {
        // $C2C4：asl ×5。0x01<<5=0x20，bit3(0x01)=0
        assert_eq!(amul32(0x01), (0x20, false));
        // 0x08<<5 溢出到 0x00，bit3(0x08)=1 ⇒ 进位可见（若按卡面写成 bit5 这里会得 false）
        assert_eq!(amul32(0x08), (0x00, true));
        assert_eq!(amul32(0x07), (0xE0, false)); // bit3(0x07)=0
        assert_eq!(amul32(0x80), (0x00, false)); // bit3(0x80)=0，结果清零但不进位
        assert_eq!(amul32(0xFF), (0xE0, true));
        assert_eq!(amul32(0x00), (0x00, false));
    }

    #[test]
    fn adiv32_rom_vectors() {
        // $C2BE：lsr ×5。进位位号 = n-1 = bit4
        assert_eq!(adiv32(0xFF), (0x07, true)); // bit4(0xFF)=1
        assert_eq!(adiv32(0x10), (0x00, true)); // bit4(0x10)=1，结果清零
        assert_eq!(adiv32(0x1F), (0x00, true));
        assert_eq!(adiv32(0x20), (0x01, false)); // bit4(0x20)=0
        assert_eq!(adiv32(0x80), (0x04, false));
        assert_eq!(adiv32(0x00), (0x00, false));
    }

    // ---- 派生关系：*32 = *16 再多一条移位（同一例程前缀的机械后果） ----

    #[test]
    fn amul32_is_one_asl_beyond_amul16_over_full_domain() {
        for a in 0u16..=0xFF {
            let a = a as u8;
            let (r16, _) = amul16(a);
            let (r32, c32) = amul32(a);
            assert_eq!(r32, r16 << 1 & 0xFF, "amul32({a:02X}) 结果不是 amul16 的再移一位");
            assert_eq!(c32, r16 & 0x80 != 0, "amul32({a:02X}) 进位不是 amul16 结果的 bit7");
        }
    }

    #[test]
    fn adiv32_is_one_lsr_beyond_adiv16_over_full_domain() {
        for a in 0u16..=0xFF {
            let a = a as u8;
            let (r16, _) = adiv16(a);
            let (r32, c32) = adiv32(a);
            assert_eq!(r32, r16 >> 1, "adiv32({a:02X}) 结果不是 adiv16 的再移一位");
            assert_eq!(c32, r16 & 0x01 != 0, "adiv32({a:02X}) 进位不是 adiv16 结果的 bit0");
        }
    }

    // ---- finish_flags：C 透传、Z = (A==0)、N = A.bit7、V 不建模 ----

    #[test]
    fn finish_flags_zero_negative_carry_vectors() {
        // ROM 侧取样：$00 结果必置 Z、$80 结果必置 N；C 与 A 无因果关系，只能透传。
        assert_eq!(
            finish_flags(0x00, false),
            Flags { carry: false, zero: true, negative: false }
        );
        assert_eq!(
            finish_flags(0x00, true),
            Flags { carry: true, zero: true, negative: false }
        );
        assert_eq!(
            finish_flags(0x80, false),
            Flags { carry: false, zero: false, negative: true }
        );
        assert_eq!(
            finish_flags(0x7F, true),
            Flags { carry: true, zero: false, negative: false }
        );
        assert_eq!(
            finish_flags(0xFF, false),
            Flags { carry: false, zero: false, negative: true }
        );
    }

    #[test]
    fn finish_flags_follows_every_primitive() {
        // 每个原语的 (A, C) 都能直接喂 finish_flags；这里挑「结果落在 0 / 0x80 / 非零」的输入，
        // 覆盖六个例程各自改变 A 的方式（移位、取补、指针加的低字节）。
        let cases: [(u8, bool, Flags); 8] = [
            // amul16(0x10) ⇒ (0x00, true)：结果清零 ⇒ Z=1
            (amul16(0x10).0, amul16(0x10).1, Flags { carry: true, zero: true, negative: false }),
            // amul32(0x04) ⇒ (0x80, false)：bit7 置位 ⇒ N=1
            (amul32(0x04).0, amul32(0x04).1, Flags { carry: false, zero: false, negative: true }),
            // adiv8(0x07) ⇒ (0x00, true)
            (adiv8(0x07).0, adiv8(0x07).1, Flags { carry: true, zero: true, negative: false }),
            // adiv32(0xFF) ⇒ (0x07, true)
            (adiv32(0xFF).0, adiv32(0xFF).1, Flags { carry: true, zero: false, negative: false }),
            // twos_complement(0x00) ⇒ (0x00, true)
            (twos_complement(0x00).0, twos_complement(0x00).1, Flags { carry: true, zero: true, negative: false }),
            // twos_complement(0x7F) ⇒ (0x81, false)：N=1
            (twos_complement(0x7F).0, twos_complement(0x7F).1, Flags { carry: false, zero: false, negative: true }),
            // add_y_to_ptr00($00FF,1) ⇒ ($0100,true)，A=RTS 时低字节 0x00 ⇒ Z=1
            ((add_y_to_ptr00(0x00FF, 0x01).0 & 0xFF) as u8, add_y_to_ptr00(0x00FF, 0x01).1,
             Flags { carry: true, zero: true, negative: false }),
            // add_y_to_ptr00($0000,0x80) ⇒ ($0080,false)，低字节 bit7 ⇒ N=1
            ((add_y_to_ptr00(0x0000, 0x80).0 & 0xFF) as u8, add_y_to_ptr00(0x0000, 0x80).1,
             Flags { carry: false, zero: false, negative: true }),
        ];
        for (a, c, want) in cases {
            assert_eq!(finish_flags(a, c), want, "finish_flags({a:02X}, {c})");
        }
    }

    #[test]
    fn finish_flags_is_consistent_with_6502_nz_over_full_domain() {
        // Z 与 N 只能由 A 决定、C 只能由入参透传：全值域双向验证，防止日后把 C 也算进 Z/N。
        for a in 0u16..=0xFF {
            let a = a as u8;
            for c in [false, true] {
                let f = finish_flags(a, c);
                assert_eq!(f.carry, c);
                assert_eq!(f.zero, a == 0);
                assert_eq!(f.negative, a >= 0x80);
                // Z 与 N 互斥（A 不可能同时为 0 且 bit7=1）——6502 上没有「zero 且 negative」态
                assert!(!(f.zero && f.negative));
            }
        }
    }

    #[test]
    fn add_y_to_ptr00_matches_u16_wrapping_sum_over_samples() {
        // ROM 语义=低字节相加进位、高字节 wrapping_inc ⇒ 等价于 u16 wrapping_add。
        for lo in [0x00u8, 0x01, 0x7F, 0x80, 0xF0, 0xFE, 0xFF] {
            for hi in [0x00u8, 0x07, 0x80, 0xFF] {
                for y in [0x00u8, 0x01, 0x02, 0x20, 0x7F, 0x80, 0xFF] {
                    let p = u16::from_le_bytes([lo, hi]);
                    let (got, c) = add_y_to_ptr00(p, y);
                    assert_eq!(got, p.wrapping_add(y as u16), "ptr={p:04X} y={y:02X}");
                    assert_eq!(c, u16::from(lo) + u16::from(y) > 0xFF);
                }
            }
        }
    }
}
