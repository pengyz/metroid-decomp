//! cpu6502 —— 无 PPU 的纯 6502 核（L2 用；documented 指令集，未知操作码 fail-loud）。
//!
//! 用途唯一：子程序级直跑 ROM 原字节做出参/写集裁定（D7 机制①）。不模拟周期时序。
//! 内存模型见 l2.rs 模块注释；6502 无十进制（2A03），ADC/SBC 忽略 D 标志。

use game_core::L2In;

pub const FLAG_C: u8 = 0x01;
pub const FLAG_Z: u8 = 0x02;
pub const FLAG_I: u8 = 0x04;
pub const FLAG_D: u8 = 0x08;
pub const FLAG_B: u8 = 0x10;
pub const FLAG_U: u8 = 0x20;
pub const FLAG_V: u8 = 0x40;
pub const FLAG_N: u8 = 0x80;

pub struct Cpu {
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub sp: u8,
    pub p: u8,
    pub pc: u16,
    ram: Vec<u8>,
    prg: Vec<u8>,
    /// $6000-$7FFF WRAM 8KB（敌槽/暂存区；实机证据见 findings/ram-region-6af4-contradiction.md）
    wram: Vec<u8>,
    window_bank: usize,
    writes: Vec<(u16, u8)>,
}

impl Cpu {
    /// prg = 128KB PRG（8×16KB）；window_bank = $8000 窗口 bank 号（0..8）。
    pub fn new(prg: &[u8], window_bank: usize, input: &L2In) -> Self {
        assert_eq!(prg.len(), 0x20000, "PRG 须为 128KB");
        assert!(window_bank < 8, "window_bank 须 < 8");
        Self {
            a: input.a,
            x: input.x,
            y: input.y,
            sp: 0xF8,
            p: input.p | FLAG_U,
            pc: 0,
            ram: input.ram.clone(),
            prg: prg.to_vec(),
            wram: { let mut w = input.wram.clone(); w.resize(0x2000, 0); w },
            window_bank,
            writes: Vec::new(),
        }
    }

    pub fn set_pc(&mut self, pc: u16) {
        self.pc = pc;
    }

    fn read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1FFF => self.ram[(addr as usize) & 0x7FF],
            0x6000..=0x7FFF => self.wram[addr as usize - 0x6000],
            0x8000..=0xBFFF => self.prg[self.window_bank * 0x4000 + (addr as usize - 0x8000)],
            0xC000..=0xFFFF => self.prg[0x1C000 + (addr as usize - 0xC000)],
            _ => 0, // PPU/APU/IO/未映射 stub
        }
    }

    fn write(&mut self, addr: u16, val: u8) {
        if addr < 0x2000 {
            let norm = (addr as usize) & 0x7FF;
            self.ram[norm] = val;
            self.writes.push((norm as u16, val));
        } else if (0x6000..0x8000).contains(&addr) {
            // WRAM 写：记完整地址（$6AFx 域；2KB 镜像归一仅适用 $0000-$07FF）
            self.wram[addr as usize - 0x6000] = val;
            self.writes.push((addr, val));
        }
        // PPU/APU/mapper 寄存器写：L2 面忽略（有副作用的例程走 L3，pipeline R3）
    }

    fn set_zn(&mut self, v: u8) {
        self.set_flag(FLAG_Z, v == 0);
        self.set_flag(FLAG_N, v & 0x80 != 0);
    }

    fn set_flag(&mut self, f: u8, on: bool) {
        if on {
            self.p |= f;
        } else {
            self.p &= !f;
        }
    }

    fn get_flag(&self, f: u8) -> bool {
        self.p & f != 0
    }

    fn fetch(&mut self) -> u8 {
        let v = self.read(self.pc);
        self.pc = self.pc.wrapping_add(1);
        v
    }

    fn fetch16(&mut self) -> u16 {
        let lo = self.fetch() as u16;
        let hi = self.fetch() as u16;
        lo | (hi << 8)
    }

    /// 机械栈写（JSR/PHA/哨兵压栈）：**不入写集**（契约：栈压入/弹出恒不入面）。
    fn push(&mut self, v: u8) {
        let addr = 0x0100 | self.sp as u16;
        self.ram[(addr as usize) & 0x7FF] = v;
        self.sp = self.sp.wrapping_sub(1);
    }

    fn pop(&mut self) -> u8 {
        self.sp = self.sp.wrapping_add(1);
        self.read(0x0100 | self.sp as u16)
    }

    fn addr_zp(&mut self) -> u16 {
        self.fetch() as u16
    }

    fn addr_zpx(&mut self) -> u16 {
        (self.fetch().wrapping_add(self.x)) as u16
    }

    fn addr_zpy(&mut self) -> u16 {
        (self.fetch().wrapping_add(self.y)) as u16
    }

    fn addr_abs(&mut self) -> u16 {
        self.fetch16()
    }

    fn addr_absx(&mut self) -> u16 {
        self.fetch16().wrapping_add(self.x as u16)
    }

    fn addr_absy(&mut self) -> u16 {
        self.fetch16().wrapping_add(self.y as u16)
    }

    fn addr_izx(&mut self) -> u16 {
        let zp = self.fetch().wrapping_add(self.x);
        let lo = self.read(zp as u16) as u16;
        let hi = self.read(zp.wrapping_add(1) as u16) as u16;
        lo | (hi << 8)
    }

    fn addr_izy(&mut self) -> u16 {
        let zp = self.fetch();
        let lo = self.read(zp as u16) as u16;
        let hi = self.read(zp.wrapping_add(1) as u16) as u16;
        (lo | (hi << 8)).wrapping_add(self.y as u16)
    }

    fn branch(&mut self, take: bool) {
        let off = self.fetch() as i8;
        if take {
            self.pc = self.pc.wrapping_add(off as u16);
        }
    }

    fn compare(&mut self, reg: u8, v: u8) {
        let r = reg.wrapping_sub(v);
        self.set_flag(FLAG_C, reg >= v);
        self.set_zn(r);
    }

    fn adc(&mut self, v: u8) {
        let a = self.a as u16;
        let c = if self.get_flag(FLAG_C) { 1u16 } else { 0 };
        let sum = a + v as u16 + c;
        self.set_flag(FLAG_C, sum > 0xFF);
        let r = (sum & 0xFF) as u8;
        self.set_flag(FLAG_V, (!(self.a ^ v) & (self.a ^ r) & 0x80) != 0);
        self.a = r;
        self.set_zn(r);
    }

    fn sbc(&mut self, v: u8) {
        self.adc(!v);
    }

    fn asl_val(&mut self, v: u8) -> u8 {
        self.set_flag(FLAG_C, v & 0x80 != 0);
        let r = v << 1;
        self.set_zn(r);
        r
    }

    fn lsr_val(&mut self, v: u8) -> u8 {
        self.set_flag(FLAG_C, v & 0x01 != 0);
        let r = v >> 1;
        self.set_zn(r);
        r
    }

    fn rol_val(&mut self, v: u8) -> u8 {
        let carry_in = if self.get_flag(FLAG_C) { 1u8 } else { 0 };
        self.set_flag(FLAG_C, v & 0x80 != 0);
        let r = (v << 1) | carry_in;
        self.set_zn(r);
        r
    }

    fn ror_val(&mut self, v: u8) -> u8 {
        let carry_in = if self.get_flag(FLAG_C) { 0x80u8 } else { 0 };
        self.set_flag(FLAG_C, v & 0x01 != 0);
        let r = (v >> 1) | carry_in;
        self.set_zn(r);
        r
    }

    /// 执行一条指令。
    pub fn step(&mut self) -> Result<(), String> {
        let op = self.fetch();
        match op {
            // ADC
            0x69 => { let v = self.fetch(); self.adc(v); }
            0x65 => { let a = self.addr_zp(); let v = self.read(a); self.adc(v); }
            0x75 => { let a = self.addr_zpx(); let v = self.read(a); self.adc(v); }
            0x6D => { let a = self.addr_abs(); let v = self.read(a); self.adc(v); }
            0x7D => { let a = self.addr_absx(); let v = self.read(a); self.adc(v); }
            0x79 => { let a = self.addr_absy(); let v = self.read(a); self.adc(v); }
            0x61 => { let a = self.addr_izx(); let v = self.read(a); self.adc(v); }
            0x71 => { let a = self.addr_izy(); let v = self.read(a); self.adc(v); }
            // AND
            0x29 => { let v = self.fetch(); self.a &= v; self.set_zn(self.a); }
            0x25 => { let a = self.addr_zp(); self.a &= self.read(a); self.set_zn(self.a); }
            0x35 => { let a = self.addr_zpx(); self.a &= self.read(a); self.set_zn(self.a); }
            0x2D => { let a = self.addr_abs(); self.a &= self.read(a); self.set_zn(self.a); }
            0x3D => { let a = self.addr_absx(); self.a &= self.read(a); self.set_zn(self.a); }
            0x39 => { let a = self.addr_absy(); self.a &= self.read(a); self.set_zn(self.a); }
            0x21 => { let a = self.addr_izx(); self.a &= self.read(a); self.set_zn(self.a); }
            0x31 => { let a = self.addr_izy(); self.a &= self.read(a); self.set_zn(self.a); }
            // ASL
            0x0A => { self.a = self.asl_val(self.a); }
            0x06 => { let a = self.addr_zp(); let r = self.asl_val(self.read(a)); self.write(a, r); }
            0x16 => { let a = self.addr_zpx(); let r = self.asl_val(self.read(a)); self.write(a, r); }
            0x0E => { let a = self.addr_abs(); let r = self.asl_val(self.read(a)); self.write(a, r); }
            0x1E => { let a = self.addr_absx(); let r = self.asl_val(self.read(a)); self.write(a, r); }
            // 分支
            0x90 => { let c = !self.get_flag(FLAG_C); self.branch(c); }
            0xB0 => { let c = self.get_flag(FLAG_C); self.branch(c); }
            0xF0 => { let c = self.get_flag(FLAG_Z); self.branch(c); }
            0x30 => { let c = self.get_flag(FLAG_N); self.branch(c); }
            0xD0 => { let c = !self.get_flag(FLAG_Z); self.branch(c); }
            0x10 => { let c = !self.get_flag(FLAG_N); self.branch(c); }
            0x50 => { let c = !self.get_flag(FLAG_V); self.branch(c); }
            0x70 => { let c = self.get_flag(FLAG_V); self.branch(c); }
            // BIT
            0x24 => { let a = self.addr_zp(); let v = self.read(a); self.bit(v); }
            0x2C => { let a = self.addr_abs(); let v = self.read(a); self.bit(v); }
            // 标志
            0x18 => self.set_flag(FLAG_C, false),
            0xD8 => self.set_flag(FLAG_D, false),
            0x58 => self.set_flag(FLAG_I, false),
            0xB8 => self.set_flag(FLAG_V, false),
            0x38 => self.set_flag(FLAG_C, true),
            0xF8 => self.set_flag(FLAG_D, true),
            0x78 => self.set_flag(FLAG_I, true),
            // CMP
            0xC9 => { let v = self.fetch(); let a = self.a; self.compare(a, v); }
            0xC5 => { let ad = self.addr_zp(); let v = self.read(ad); let a = self.a; self.compare(a, v); }
            0xD5 => { let ad = self.addr_zpx(); let v = self.read(ad); let a = self.a; self.compare(a, v); }
            0xCD => { let ad = self.addr_abs(); let v = self.read(ad); let a = self.a; self.compare(a, v); }
            0xDD => { let ad = self.addr_absx(); let v = self.read(ad); let a = self.a; self.compare(a, v); }
            0xD9 => { let ad = self.addr_absy(); let v = self.read(ad); let a = self.a; self.compare(a, v); }
            0xC1 => { let ad = self.addr_izx(); let v = self.read(ad); let a = self.a; self.compare(a, v); }
            0xD1 => { let ad = self.addr_izy(); let v = self.read(ad); let a = self.a; self.compare(a, v); }
            // CPX/CPY
            0xE0 => { let v = self.fetch(); let x = self.x; self.compare(x, v); }
            0xE4 => { let ad = self.addr_zp(); let v = self.read(ad); let x = self.x; self.compare(x, v); }
            0xEC => { let ad = self.addr_abs(); let v = self.read(ad); let x = self.x; self.compare(x, v); }
            0xC0 => { let v = self.fetch(); let y = self.y; self.compare(y, v); }
            0xC4 => { let ad = self.addr_zp(); let v = self.read(ad); let y = self.y; self.compare(y, v); }
            0xCC => { let ad = self.addr_abs(); let v = self.read(ad); let y = self.y; self.compare(y, v); }
            // DEC/INC
            0xC6 => { let a = self.addr_zp(); let r = self.read(a).wrapping_sub(1); self.write(a, r); self.set_zn(r); }
            0xD6 => { let a = self.addr_zpx(); let r = self.read(a).wrapping_sub(1); self.write(a, r); self.set_zn(r); }
            0xCE => { let a = self.addr_abs(); let r = self.read(a).wrapping_sub(1); self.write(a, r); self.set_zn(r); }
            0xDE => { let a = self.addr_absx(); let r = self.read(a).wrapping_sub(1); self.write(a, r); self.set_zn(r); }
            0xE6 => { let a = self.addr_zp(); let r = self.read(a).wrapping_add(1); self.write(a, r); self.set_zn(r); }
            0xF6 => { let a = self.addr_zpx(); let r = self.read(a).wrapping_add(1); self.write(a, r); self.set_zn(r); }
            0xEE => { let a = self.addr_abs(); let r = self.read(a).wrapping_add(1); self.write(a, r); self.set_zn(r); }
            0xFE => { let a = self.addr_absx(); let r = self.read(a).wrapping_add(1); self.write(a, r); self.set_zn(r); }
            // 寄存器增减
            0xCA => { self.x = self.x.wrapping_sub(1); self.set_zn(self.x); }
            0x88 => { self.y = self.y.wrapping_sub(1); self.set_zn(self.y); }
            0xE8 => { self.x = self.x.wrapping_add(1); self.set_zn(self.x); }
            0xC8 => { self.y = self.y.wrapping_add(1); self.set_zn(self.y); }
            // EOR
            0x49 => { let v = self.fetch(); self.a ^= v; self.set_zn(self.a); }
            0x45 => { let a = self.addr_zp(); self.a ^= self.read(a); self.set_zn(self.a); }
            0x55 => { let a = self.addr_zpx(); self.a ^= self.read(a); self.set_zn(self.a); }
            0x4D => { let a = self.addr_abs(); self.a ^= self.read(a); self.set_zn(self.a); }
            0x5D => { let a = self.addr_absx(); self.a ^= self.read(a); self.set_zn(self.a); }
            0x59 => { let a = self.addr_absy(); self.a ^= self.read(a); self.set_zn(self.a); }
            0x41 => { let a = self.addr_izx(); self.a ^= self.read(a); self.set_zn(self.a); }
            0x51 => { let a = self.addr_izy(); self.a ^= self.read(a); self.set_zn(self.a); }
            // JMP/JSR/RTS/RTI/BRK
            0x4C => { let a = self.fetch16(); self.pc = a; }
            0x6C => {
                let a = self.fetch16();
                let lo = self.read(a) as u16;
                let hi_addr = (a & 0xFF00) | ((a as u8).wrapping_add(1) as u16); // 6502 页面 bug
                let hi = self.read(hi_addr) as u16;
                self.pc = lo | (hi << 8);
            }
            0x20 => {
                let target = self.fetch16();
                let ret = self.pc.wrapping_sub(1);
                self.push((ret >> 8) as u8);
                self.push((ret & 0xFF) as u8);
                self.pc = target;
            }
            0x60 => {
                let lo = self.pop() as u16;
                let hi = self.pop() as u16;
                self.pc = (lo | (hi << 8)).wrapping_add(1);
            }
            0x40 => {
                self.p = self.pop() | FLAG_U;
                let lo = self.pop() as u16;
                let hi = self.pop() as u16;
                self.pc = lo | (hi << 8);
            }
            0x00 => return Err(format!("BRK @ ${:04X}（L2 面不支持，走 L3）", self.pc.wrapping_sub(1))),
            // LDA
            0xA9 => { self.a = self.fetch(); self.set_zn(self.a); }
            0xA5 => { let a = self.addr_zp(); self.a = self.read(a); self.set_zn(self.a); }
            0xB5 => { let a = self.addr_zpx(); self.a = self.read(a); self.set_zn(self.a); }
            0xAD => { let a = self.addr_abs(); self.a = self.read(a); self.set_zn(self.a); }
            0xBD => { let a = self.addr_absx(); self.a = self.read(a); self.set_zn(self.a); }
            0xB9 => { let a = self.addr_absy(); self.a = self.read(a); self.set_zn(self.a); }
            0xA1 => { let a = self.addr_izx(); self.a = self.read(a); self.set_zn(self.a); }
            0xB1 => { let a = self.addr_izy(); self.a = self.read(a); self.set_zn(self.a); }
            // LDX
            0xA2 => { self.x = self.fetch(); self.set_zn(self.x); }
            0xA6 => { let a = self.addr_zp(); self.x = self.read(a); self.set_zn(self.x); }
            0xB6 => { let a = self.addr_zpy(); self.x = self.read(a); self.set_zn(self.x); }
            0xAE => { let a = self.addr_abs(); self.x = self.read(a); self.set_zn(self.x); }
            0xBE => { let a = self.addr_absy(); self.x = self.read(a); self.set_zn(self.x); }
            // LDY
            0xA0 => { self.y = self.fetch(); self.set_zn(self.y); }
            0xA4 => { let a = self.addr_zp(); self.y = self.read(a); self.set_zn(self.y); }
            0xB4 => { let a = self.addr_zpx(); self.y = self.read(a); self.set_zn(self.y); }
            0xAC => { let a = self.addr_abs(); self.y = self.read(a); self.set_zn(self.y); }
            0xBC => { let a = self.addr_absx(); self.y = self.read(a); self.set_zn(self.y); }
            // LSR
            0x4A => { self.a = self.lsr_val(self.a); }
            0x46 => { let a = self.addr_zp(); let r = self.lsr_val(self.read(a)); self.write(a, r); }
            0x56 => { let a = self.addr_zpx(); let r = self.lsr_val(self.read(a)); self.write(a, r); }
            0x4E => { let a = self.addr_abs(); let r = self.lsr_val(self.read(a)); self.write(a, r); }
            0x5E => { let a = self.addr_absx(); let r = self.lsr_val(self.read(a)); self.write(a, r); }
            // NOP
            0xEA => {}
            // ORA
            0x09 => { let v = self.fetch(); self.a |= v; self.set_zn(self.a); }
            0x05 => { let a = self.addr_zp(); self.a |= self.read(a); self.set_zn(self.a); }
            0x15 => { let a = self.addr_zpx(); self.a |= self.read(a); self.set_zn(self.a); }
            0x0D => { let a = self.addr_abs(); self.a |= self.read(a); self.set_zn(self.a); }
            0x1D => { let a = self.addr_absx(); self.a |= self.read(a); self.set_zn(self.a); }
            0x19 => { let a = self.addr_absy(); self.a |= self.read(a); self.set_zn(self.a); }
            0x01 => { let a = self.addr_izx(); self.a |= self.read(a); self.set_zn(self.a); }
            0x11 => { let a = self.addr_izy(); self.a |= self.read(a); self.set_zn(self.a); }
            // 栈
            0x48 => { let a = self.a; self.push(a); }
            0x08 => { let p = self.p | FLAG_B | FLAG_U; self.push(p); }
            0x68 => { self.a = self.pop(); self.set_zn(self.a); }
            0x28 => { self.p = self.pop() | FLAG_U; }
            // ROL/ROR
            0x2A => { self.a = self.rol_val(self.a); }
            0x26 => { let a = self.addr_zp(); let r = self.rol_val(self.read(a)); self.write(a, r); }
            0x36 => { let a = self.addr_zpx(); let r = self.rol_val(self.read(a)); self.write(a, r); }
            0x2E => { let a = self.addr_abs(); let r = self.rol_val(self.read(a)); self.write(a, r); }
            0x3E => { let a = self.addr_absx(); let r = self.rol_val(self.read(a)); self.write(a, r); }
            0x6A => { self.a = self.ror_val(self.a); }
            0x66 => { let a = self.addr_zp(); let r = self.ror_val(self.read(a)); self.write(a, r); }
            0x76 => { let a = self.addr_zpx(); let r = self.ror_val(self.read(a)); self.write(a, r); }
            0x6E => { let a = self.addr_abs(); let r = self.ror_val(self.read(a)); self.write(a, r); }
            0x7E => { let a = self.addr_absx(); let r = self.ror_val(self.read(a)); self.write(a, r); }
            // SBC
            0xE9 => { let v = self.fetch(); self.sbc(v); }
            0xE5 => { let a = self.addr_zp(); let v = self.read(a); self.sbc(v); }
            0xF5 => { let a = self.addr_zpx(); let v = self.read(a); self.sbc(v); }
            0xED => { let a = self.addr_abs(); let v = self.read(a); self.sbc(v); }
            0xFD => { let a = self.addr_absx(); let v = self.read(a); self.sbc(v); }
            0xF9 => { let a = self.addr_absy(); let v = self.read(a); self.sbc(v); }
            0xE1 => { let a = self.addr_izx(); let v = self.read(a); self.sbc(v); }
            0xF1 => { let a = self.addr_izy(); let v = self.read(a); self.sbc(v); }
            // STA
            0x85 => { let a = self.addr_zp(); let v = self.a; self.write(a, v); }
            0x95 => { let a = self.addr_zpx(); let v = self.a; self.write(a, v); }
            0x8D => { let a = self.addr_abs(); let v = self.a; self.write(a, v); }
            0x9D => { let a = self.addr_absx(); let v = self.a; self.write(a, v); }
            0x99 => { let a = self.addr_absy(); let v = self.a; self.write(a, v); }
            0x81 => { let a = self.addr_izx(); let v = self.a; self.write(a, v); }
            0x91 => { let a = self.addr_izy(); let v = self.a; self.write(a, v); }
            // STX/STY
            0x86 => { let a = self.addr_zp(); let v = self.x; self.write(a, v); }
            0x96 => { let a = self.addr_zpy(); let v = self.x; self.write(a, v); }
            0x8E => { let a = self.addr_abs(); let v = self.x; self.write(a, v); }
            0x84 => { let a = self.addr_zp(); let v = self.y; self.write(a, v); }
            0x94 => { let a = self.addr_zpx(); let v = self.y; self.write(a, v); }
            0x8C => { let a = self.addr_abs(); let v = self.y; self.write(a, v); }
            // 传输
            0xAA => { self.x = self.a; self.set_zn(self.x); }
            0xA8 => { self.y = self.a; self.set_zn(self.y); }
            0xBA => { self.x = self.sp; self.set_zn(self.x); }
            0x8A => { self.a = self.x; self.set_zn(self.a); }
            0x9A => { self.sp = self.x; }
            0x98 => { self.a = self.y; self.set_zn(self.a); }
            // **undocumented 6502 族**（D4 保真面——g3 音频域实证（$C2/$4B/$07 族））：
            // $C2=NOP imm（跳 1 字节）/$4B=ALR imm（AND+LSR）/$07=SLO zp（ASL+ORA）
            0xC2 => { self.fetch(); } // NOP imm（undocumented——D3 字节证）
            0x4B => { let v = self.fetch() & self.a; self.a = v >> 1; self.set_zn(self.a); } // ALR imm
            0x07 => { let a = self.addr_zp(); let v = self.read(a); let c = (v >> 7) & 1; let v2 = v << 1; self.write(a, v2); self.set_flag(FLAG_C, c != 0); self.a |= v2; self.set_zn(self.a); } // SLO zp
            _ => {
                return Err(format!(
                    "未知操作码 ${:02X} @ ${:04X}（L2 核 fail-loud，不静默跳过）",
                    op,
                    self.pc.wrapping_sub(1)
                ))
            }
        }
        Ok(())
    }

    fn bit(&mut self, v: u8) {
        self.set_flag(FLAG_Z, (self.a & v) == 0);
        self.set_flag(FLAG_N, v & 0x80 != 0);
        self.set_flag(FLAG_V, v & 0x40 != 0);
    }

    /// crate 内机械写（L2 引擎压哨兵栈用；不入写集）。
    pub(crate) fn store_ram(&mut self, addr: u16, val: u8) {
        self.ram[(addr as usize) & 0x7FF] = val;
    }

    /// 取出并清空 RAM 写集（地址归一至 $0000-$07FF）。
    pub fn take_writes(&mut self) -> Vec<(u16, u8)> {
        std::mem::take(&mut self.writes)
    }
}
