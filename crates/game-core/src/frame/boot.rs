//! boot 执行序端口（task-10④）：bank07:$C01A 主初始化单元（首引爆点移交件）。
//!
//! 解码证据（A 级，精修 6502 表逐指令；task-12 调用图输入素材——指针留本 doc）：
//! `bank07:C01A` 起 `A9 00 8D 00 A0 ×5 8D 00 C0 ×5 20 FA C4 CA 9A A0 07 … 4C BC C0`。
//! 结构：①MMC1 复位写（CHR0/CHR1 各 5 次+折叠 C4FA 串行 PRG 写——C4FA 零 RAM 写、X 不动，
//! 折叠先例=F1FA/E449）②DEX/TXS 栈指针 ③**RAM 低区清零环**（pages $07..$02，(zp),Y
//! 值=偏移低字节；$00/$01 为指针，环不触 ZP/栈页——帧 1 实测 $0300-$07FF 全零=后续单元
//! 覆写后态，本环为中间态）④**WRAM 清零环**（$7F00 降至 $6000，同型填充）⑤zp 初始化序
//! （$25/$28/$FD/$FC/$1D/…；其中 JSR $C158/$C1A3/$C4B2/$C4DE 为嵌套单元——桩阵逐个引爆）。
//! ⑥主循环（C0BC）+ $1A 握手（NMI 置位）——tick_frame 语义：boot 一次 + 每帧 NMI 单元
//! （bank07:$C0D9，桩）+ 主循环一迭代。
//!
//! 执行序队列指针（task-12/da90f66 交接）：本单元序列与 tools/call_graph.py 执行序队列
//! （seq|bank:addr|调用深度|域|首执行|命中）boot 相 43 入口 f0 reset 链 11 节点对表——
//! 嵌套桩序（C158/C1A3/C4B2/C4DE/C266/C114/C000）即该链的未端口段，逐个引爆=按队列 seq 排。

use super::memory::FrameMemory;

/// MMC1 寄存器面（memmap 纪律：非 RAM 地址只活在此；bank07:$FFB0/$C01A 序引用）。
pub mod io {
    /// MMC1 CHR0（bank07:C01C `8D 00 A0` STA $A000）。
    pub const MMC1_CHR0: u16 = 0xA000;
    /// MMC1 CHR1（bank07:C02B `8D 00 C0` STA $C000）。
    pub const MMC1_CHR1: u16 = 0xC000;
    /// MMC1 PRG（bank07:C4FA 序 `8D 00 E0` STA $E000 ×5 串行）。
    pub const MMC1_PRG: u16 = 0xE000;
    /// PPU 滚动（bank07:C07B `8C 05 20` STY $2005 ×2）。
    pub const PPU_SCROLL: u16 = 0x2005;
    /// PPU 控制（bank07:C08C `8D 00 20` STA $2000，值 $90=NMI 使能）。
    pub const PPU_CTRL: u16 = 0x2000;
    /// APU $4011（bank07:C09E）。
    pub const APU_4011: u16 = 0x4011;
    /// APU $4015（bank07:C0A3）。
    pub const APU_4015: u16 = 0x4015;
    /// MMC1 control（bank07:C4C5 `8D 00 80` STA $8000 ×5 串行）。
    pub const MMC1_CTRL: u16 = 0x8000;
}

/// zp 指针对（清零环用；bank07:C041 `84 01`/C045 `84 00`）。
const PTR_LO: u16 = 0x0000;
const PTR_HI: u16 = 0x0001;

/// bank07:$C01A 主初始化（执行序一步一单元；未端口步=unported! 引爆）。
pub fn boot_init(mem: &mut FrameMemory) {
    // ① MMC1 复位写（A=0：CHR0/CHR1 串行清零）
    mem.regs.a = 0x00;
    mmc1_serial_write(mem, io::MMC1_CHR0);
    mmc1_serial_write(mem, io::MMC1_CHR1);
    // 折叠 C4FA（bank07:C4FA `8D 00 E0/4A ×4 … A5 00/60`）：PRG 串行写 A>>n，末值 A=RAM[$00]
    mmc1_serial_write(mem, io::MMC1_PRG);
    mem.regs.a = mem.read_ram(PTR_LO);
    // ② DEX/TXS（X=0（reset 序 LDX #$00）→ $FF）
    mem.regs.x = mem.regs.x.wrapping_sub(1);
    mem.regs.sp = mem.regs.x;
    // ③ RAM 低区清零环（pages $07..$02，值=偏移低字节；不触 ZP/栈页）
    clear_pages(mem, 7, 2);
    // ④ WRAM 清零环（$7F00 降至 $6000）
    clear_pages_wram(mem, 0x7F, 0x60);
    // ⑤ zp 初始化序（bank07:C06D-；$25/$28/$FD/$FC/$1D 后即嵌套单元）
    mem.write_ram(0x0025, 0x0E); // LDA #$0E/STA $25
    mem.write_ram(0x0028, 0x00);
    mem.write_ram(0x00FD, 0x00);
    mem.write_ram(0x00FC, 0x00);
    mem.write_io(io::PPU_SCROLL, 0x00); // STY $2005 ×2（Y=0）
    mem.write_io(io::PPU_SCROLL, 0x00);
    mem.write_ram(0x001D, 0x01); // INY/STY $1D
    // C158 单元（VRAM 清屏双遍——已端口）
    c158_unit(mem);
    // C1A3 单元（OAM 影子清屏——已端口）
    c1a3_unit(mem);
    #[allow(unreachable_code)]
    {
        // C158 后序（桩移除后可达；现为执行序地图占位）
        mem.write_io(io::PPU_CTRL, 0x90); // STA $2000（NMI 使能）
        mem.write_ram(0x00FF, 0x90);
        mem.write_ram(0x00FE, 0x02);
        mem.write_ram(0x00FA, 0x47);
        // C4B2 单元（MMC1 control 串行写——已端口）
        c4b2_unit(mem);
        #[allow(unreachable_code)]
        {
            mem.write_io(io::APU_4011, 0x00);
            mem.write_io(io::APU_4015, 0x0F);
            mem.write_ram(0x001F, 0x00);
            mem.write_ram(0x001E, 0x00);
            mem.write_ram(0x002E, 0x11);
            mem.write_ram(0x002F, 0xFF);
            mem.write_ram(0x0024, 0x01);
            // C4DE 单元（PRG 换 bank+分派——已端口）
            c4de_unit(mem);
        }
    }
}

/// L2 组件 → 内存态桥（组件库反哺：**既有验证件**经写集契约挂入帧引擎——
/// 「L2 管线按需补给」反向流；写址为 L2 归一面（&0x7FF 同源））。
fn l2_apply(mem: &mut FrameMemory, out: crate::L2Out) {
    mem.regs.a = out.a;
    mem.regs.x = out.x;
    mem.regs.y = out.y;
    for (addr, v) in out.writes {
        mem.write_ram(addr, v);
    }
}

/// C158 单元——VRAM 清屏（bank07:C158-C19E）：双遍填充 PPU nametable $2000/$2400/$2800
/// （各 1280B=$FF；**PPU 面非 RAM watch**——渲染面保真待 task-13）+ ptr 态。
///
/// 结构（A 级精修表）：`20 6D C1`（JSR $C16D——**复用既有验证件 sub_c16d**：$01=1/$00=FF）
/// → `$1D/$1F` 分派（**`C9 1D` CMP #$1D=立即数陷阱同族**）→ 尾共段（BNE 汇入 C16F
/// `85 01` STA $01/$00=FF）→ PPU 面（`AD 02 20` 读 status/`$FF` 清 bit2/`STA $2000`/
/// 表[$01-1]→$2006/`STA $2007` ×1280）→ C19E RTS（尾共段=次遍的出口）。
/// 结束态（RAM 面）：$00=FF、$01=2|3、$FF&=!4、A=FF/X=0/Y=0。
pub fn c158_unit(mem: &mut FrameMemory) {
    // 首遍：L2 反哺复用（sub_c16d 契约= $01=1/$00=FF/A=FF，44 案时代验证件）
    l2_apply(mem, crate::l2ports::sub_c16d(&crate::L2In::default()));
    // 次遍：$1D/$1F 分派（C15B-C16B）→ 尾共段 $01∈{2,3}
    let hi = if mem.read_ram(0x001D) == 0 || mem.read_ram(0x001F) != 0x1D { 2 } else { 3 };
    mem.write_ram(0x0001, hi);
    mem.write_ram(0x0000, 0xFF);
    // $FF 清 bit2（AND #$FB/STA $FF/STA $2000——PPU ctrl NMI 位清）
    let f = mem.read_ram(0x00FF) & 0xFB;
    mem.write_ram(0x00FF, f);
    // PPU 面（非 RAM watch）：LDX $2002（status 读）、表读 $C19F[$01-1]→$2006、$FF 填 ×1280
    // 结束态：LDA $00 尾次 A=FF；循环计数 X/Y 归零
    mem.regs.a = 0xFF;
    mem.regs.x = 0;
    mem.regs.y = 0;
}

/// C1A3 单元——OAM 影子清屏（bank07:C1A3-C1BB）：$0200-$02FF = $F0 填充（**帧 1 实测面**
/// ——oracle 帧 1 $0200 页 F0 填充 256/256 命中）+ ptr 态（$00=0/$01=2）。
///
/// $1D≠0 ⇒ 尾跳窗口 $988A（bank00——boot 态 MMC1 自证：FFB0 序 control=$FF（mode3）+
/// C01A 序 PRG reg=0 ⇒ $8000=bank0）。$988A 双检（`C9 1D` CMP 立即数同族陷阱/$C8/$2D）：
/// boot 态全短路→$98AD RTS 直回；**游戏态路径未端口**（$989B DEC 循环+后续）——fail-loud。
pub fn c1a3_unit(mem: &mut FrameMemory) {
    for y in 0..=0xFFu8 {
        mem.write_ram(0x0200 + y as u16, 0xF0);
    }
    mem.write_ram(0x0000, 0);
    mem.write_ram(0x0001, 2);
    if mem.read_ram(0x001D) != 0 {
        // $988A 双检：$1F>=$1D 或 $C8==0 或 $2D bit0=1 ⇒ 出口 $98AD（RTS）
        let in_game_path = mem.read_ram(0x001F) < 0x1D
            && mem.read_ram(0x00C8) != 0
            && mem.read_ram(0x002D) & 1 == 0;
        if in_game_path {
            crate::unported!("bank00:988A");
        }
    }
}

/// C4B2 单元——MMC1 control 串行写（bank07:C4B2-C4D8）：`A9 47/4A ×3/29 01` 取 bit
/// （A=$47 入口自载；**C4D9 变体**=`A5 FA/4C B6 C4` 尾跳 $C4B6——A=$FA，未入 boot 序）→
/// `$25=($25&!1)|bit`（RAM 面）→ `STA $8000` ×5 串行（寄存器面）。
/// boot 态：$25=0x0E（C06D 置）→ $00=0/$25=0x0E/A=0x0E；control=0x0E（mode3 ✓ 与
/// $988A bank0 归属自证一致）。
pub fn c4b2_unit(mem: &mut FrameMemory) {
    let a_in = 0x47; // C4B4 `A9 47` 自载（C4D9 变体=$FA——尾跳入口另立）
    let bit = (a_in >> 3) & 1;
    mem.write_ram(0x0000, bit);
    let v = (mem.read_ram(0x0025) & 0xFE) | bit;
    mem.write_ram(0x0025, v);
    mmc1_serial_write(mem, io::MMC1_CTRL);
    mem.regs.a = v;
}

/// C4DE 单元——PRG 换 bank + 跳转表分派（**asm 引证主体**，链规 17）：
/// refs/m1disasm/SRC/prg7_engine.asm:1296 `CheckBankSwitch: ; 07:C4DE`
/// （注：「Exit if zero(no bank switch issued). else Y contains bank#+1」）→
/// :1305 `CheckBankSwitch_OK: ; 07:C4E8`（「clear BankSwitchPending…dey sty CurrentBank」）
/// → `BankSwitch: ; 07:C4EF`（「tya sta Temp00_Bank / lda SwitchUpperBits and #$18」）→
/// :1318 族 `MMCWritePrgBank: ; 07:C4FA`（「Write bit 0..4 of ROM bank #」×5 串行）
/// → :1351 `GoBankInit: ; 07:C510`（「asl tay / jmp (Temp0A_BankInitPtr)」）+ 表 :1363
/// `BankInitTable: ; 07:C51F`（.word InitBank0..5/ExitSub×3）。
/// ROM 抽查佐证：`C4DE: A4 24 F0 2D 20 E8 C4 4C 10 C5`。
/// boot 态 $24=1（C0B4 置）→ 索引 0 → 表项 InitBank0（$C531）。其他表项未端口——fail-loud。
pub fn c4de_unit(mem: &mut FrameMemory) {
    let y_in = mem.read_ram(0x0024);
    if y_in == 0 {
        return; // 早出（A 不动——C4E0 BEQ $C50F）
    }
    // C4E8 体（含 C4FA 共享段 PRG 串行写——与 C01A 折叠同源）
    mem.write_ram(0x0024, 0);
    let idx = y_in.wrapping_sub(1);
    mem.write_ram(0x0023, idx);
    mem.write_ram(0x0000, idx);
    let v = (mem.read_ram(0x0028) & 0x18) | idx;
    mem.write_ram(0x0028, v);
    mem.regs.a = v;
    mmc1_serial_write(mem, io::MMC1_PRG);
    mem.regs.a = mem.read_ram(0x0000); // C50D LDA $00（分派索引）
    // C510 分派（表 bank0:C51F `31 C5 52 C5 …`）
    match mem.regs.a {
        0 => c531_unit(mem),
        _ => crate::unported!("bank00:C51F"), // 非 boot 表项未端口
    }
}

/// C531 单元（boot 分派目标；**asm 引证主体**，链规 17）——
/// refs/m1disasm/SRC/prg7_engine.asm:1376 `InitBank0: ; 07:C531`（「Title screen memory
/// page」注）：`sty GamePaused`（$31=0）/`sty GameMode`（$1D=1）→ :1381 `jsr ScreenNmiOff
/// ($C45D)`（「Waits for NMI to end then turns it off」）→ `jsr CopyMap ($A93E)`（「Copy game
/// map from ROM to cartridge RAM $7000-$73FF」）→ `jsr ClearNameTables ($C158)`（**复用已
/// 端口件**）→ 星星拷贝（「Loads sprite info for stars into RAM $6E00 thru 6E9F」，
/// IntroStarsData→IntroStarSprite，$A0 字节）→ `jsr InitTitleGFX ($C5D7)` → `jmp NMIOn
/// ($C487)`（「Turn on VBlank interrupts」）。ROM 抽查：`C531: A0 00 84 31 C8 84 1D`。
/// 链首桩=ScreenNmiOff（C45D）——逐段端口。
fn c531_unit(mem: &mut FrameMemory) {
    mem.write_ram(0x0031, 0);
    mem.write_ram(0x001D, 1);
    crate::unported!("bank07:C45D"); // 链首桩（每引爆一个端一个）
    #[allow(unreachable_code)]
    {
        // 后续链段：JSR $A93E（窗口件）/JSR $C158（c158_unit 复用）/ROM→WRAM 拷贝
        // （memmap 烘焙源）/JSR $C5D7/JMP $C487——逐段端口
    }
}

/// MMC1 串行写（5 次 STA，值 LSR 递移——bank07:C01C/C02B/C4FA 形态；非 RAM 面）。
fn mmc1_serial_write(mem: &mut FrameMemory, _reg: u16) {
    let mut v = mem.regs.a;
    for _ in 0..5 {
        let _ = v; // STA reg（MMC1 移位面——RAM watch 不可见）
        v >>= 1; // LSR A（C4FA 形态；C01C/C02B 恒写 0 形态同效）
    }
}

/// RAM 低区清零环（bank07:C03F-C055）：pages hi 降至 end_excl（CPX #$01 等值退出），
/// 每页 RAM[base+Y]=Y（值=偏移低字节）；$00/$01 指针环内不被触碰（页≥$02）。
/// 帧界不可见性实测：帧 1 $0300-$07FF 全零=后续单元覆写后态（本环为中间态）。
fn clear_pages(mem: &mut FrameMemory, mut page: u8, end_excl: u8) {
    loop {
        let base = (page as u16) << 8;
        for y in 0..=0xFFu8 {
            mem.write_ram(base + y as u16, y);
        }
        page = page.wrapping_sub(1);
        if page == 0xFF || page < end_excl {
            break;
        }
    }
}

/// WRAM 清零环（bank07:C057-C06B）：pages hi 降至 lo（CPX #$60/BCS 含等值页）。
fn clear_pages_wram(mem: &mut FrameMemory, mut page: u8, lo: u8) {
    loop {
        let base = (page as u16) << 8;
        for y in 0..=0xFFu8 {
            mem.write_wram(base + y as u16, y);
        }
        if page == lo {
            break;
        }
        page = page.wrapping_sub(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// panic payload 双型兜底（String / &str——工具链形态差异）。
    fn panic_message(err: &Box<dyn std::any::Any + Send>) -> String {
        err.downcast_ref::<String>()
            .cloned()
            .or_else(|| err.downcast_ref::<&'static str>().map(|s| (*s).to_string()))
            .unwrap_or_default()
    }

    /// 清零环语义（bank07:C03F-C055）：pages $07..$02 值=偏移低字节；ZP/栈页不触。
    #[test]
    fn clear_ram_lo_fills_pages_with_low_byte() {
        let mut m = FrameMemory::default();
        clear_pages(&mut m, 7, 2);
        assert_eq!(m.read_ram(0x0700), 0x00);
        assert_eq!(m.read_ram(0x02FF), 0xFF);
        assert_eq!(m.read_ram(0x0355), 0x55); // 值=偏移低字节
        assert_eq!(m.read_ram(0x0000), 0x00); // ZP 不触
        assert_eq!(m.read_ram(0x0180), 0x00); // 栈页不触
    }

    /// C158 分派双态（bank07:C15B-C16B）：$1D==0∨$1F≠$1D→$01=2；否则 3。
    /// 首遍经 sub_c16d 反哺（$00=FF/A=FF）——组件库复用溯源。
    #[test]
    fn c158_dispatch_two_states_and_c16d_reuse() {
        let mut m = FrameMemory::default();
        c158_unit(&mut m); // $1D=0 → $01=2
        assert_eq!(m.read_ram(0x0001), 2);
        assert_eq!(m.read_ram(0x0000), 0xFF); // sub_c16d 反哺 + 尾共段
        assert_eq!(m.regs.a, 0xFF); // C16D 契约 A=FF
        let mut m2 = FrameMemory::default();
        m2.write_ram(0x001D, 5);
        m2.write_ram(0x001F, 0x1D);
        c158_unit(&mut m2); // $1D≠0 ∧ $1F==$1D（立即数）→ $01=3
        assert_eq!(m2.read_ram(0x0001), 3);
    }

    /// C1A3 OAM 影子清屏（帧 1 实测面）+ boot 态双检短路。
    #[test]
    fn c1a3_fills_oam_shadow_and_boot_short_circuits() {
        let mut m = FrameMemory::default();
        m.write_ram(0x001D, 1); // boot 态（C082 置位）
        c1a3_unit(&mut m);
        assert_eq!(m.read_ram(0x0200), 0xF0);
        assert_eq!(m.read_ram(0x02FF), 0xF0);
        assert_eq!(m.read_ram(0x01FF), 0x00); // 栈页不触
        assert_eq!(m.read_ram(0x0001), 2);
    }

    /// C1A3 游戏态路径 fail-loud（$1F<$1D ∧ $C8≠0 ∧ $2D 偶——$989B DEC 循环未端口）。
    #[test]
    fn c1a3_ingame_path_detonates() {
        let mut m = FrameMemory::default();
        m.write_ram(0x001D, 1);
        m.write_ram(0x00C8, 1); // $C8≠0
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| c1a3_unit(&mut m)));
        let msg = panic_message(&r.unwrap_err());
        assert!(msg.contains("unported @ bank00:988A"), "引爆点失准: {msg}");
    }

    /// C4B2 MMC1 control 状态面（boot 态 $25=0x0E→$00=0/$25=0x0E/A=0x0E）。
    #[test]
    fn c4b2_writes_control_state() {
        let mut m = FrameMemory::default();
        m.write_ram(0x0025, 0x0E);
        c4b2_unit(&mut m);
        assert_eq!(m.read_ram(0x0000), 0x00); // ($47>>3)&1=0
        assert_eq!(m.read_ram(0x0025), 0x0E);
        assert_eq!(m.regs.a, 0x0E);
    }

    /// C4DE 早出（$24==0：A 不动）与状态面（$24=0/$23=idx/$00=idx/$28 组合）。
    #[test]
    fn c4de_early_exit_and_state_update() {
        let mut m = FrameMemory::default();
        m.regs.a = 0x77;
        c4de_unit(&mut m); // $24==0 → 早出
        assert_eq!(m.regs.a, 0x77);
        m.write_ram(0x0024, 1); // boot 态
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| c4de_unit(&mut m)));
        let msg = panic_message(&r.unwrap_err());
        assert!(msg.contains("unported @ bank07:C45D"), "引爆点失准: {msg}");
        assert_eq!(m.read_ram(0x0024), 0); // C4E8 已跑
        assert_eq!(m.read_ram(0x0023), 0);
        assert_eq!(m.read_ram(0x0028), 0); // ($28&0x18)|0
    }

    /// WRAM 环（bank07:C057-C06B）：$6000-$7FFF 同型填充。
    #[test]
    fn clear_wram_fills_with_low_byte() {
        let mut m = FrameMemory::default();
        clear_pages_wram(&mut m, 0x7F, 0x60);
        assert_eq!(m.read_wram(0x6000), 0x00);
        assert_eq!(m.read_wram(0x7FFF), 0xFF);
        assert_eq!(m.read_wram(0x72AB), 0xAB);
    }
}
