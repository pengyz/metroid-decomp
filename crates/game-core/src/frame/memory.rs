//! 全内存模型（task-10②）：RAM 2KB（含栈页 $0100-$01FF）+ WRAM 8KB + 寄存器面。
//!
//! 地址归一口径同 L2 契约（2KB 镜像 `addr & 0x7FF`；WRAM $6000-$7FFF 字节偏移）；
//! watch 面=零页 $0000-$00FF + 栈/游戏变量区 $0100-$07FF（l3_manifest 显式基址区间，
//! gotcha-stack-page 教训：不用惯常分区假设）。

/// G11 类型下沉（r19）：定义归 `crate::state`（frame↔域件环消除）——本件保留 re-export
/// （兼容径）+方法实现（同 crate 跨模块 impl）。
pub use crate::state::{Manager, Registers};

impl Manager {
    /// IO/寄存器面写（PPU/APU/手柄——**不入 RAM**：write_ram 的 2KB 镜像归一会把
    /// $2000 等寄存器址漏进 RAM $0000——本方法是寄存器面唯一入口；渲染面建模挂点=task-13）。
    pub fn write_io(&mut self, addr: u16, v: u8) {
        match addr {
            0x2000 => {
                self.nmi_enabled = v & 0x80 != 0; // PPUCTRL bit7=NMI（hardware.asm:28）
                self.render.ppuctrl = v;          // 全寄存器面（图案表选 bit3/4——渲染腿）
            }
            // $2001 PPUMASK 真寄存器面（asm `WritePPUCtrl: ; 07:C44D` 的
            // `lda PPUMASK_ZP / sta PPUMASK`（prg7_engine.asm:517）落点——NMI 内每帧落一次；
            // ScreenOff/ScreenNmiOff 只改影子 $FE，「渲染关」由本写进 RenderState 生效。
            0x2001 => self.render.ppumask = v,
            0x2005 => self.render.write_reg_2005(v),   // PPUSCROLL（V1 摄像机——双写锁存）
            0x2006 => self.render.write_reg_2006(v),   // PPUADDR flip-flop（S6-10 接线）
            0x2007 => {
                // PPUDATA→RenderState 喂入（S6-10 ①——840ad00 建模件运行时喂入面）
                // 增量位=PPUCTRL bit2（1/32）——从 ram $FF 读
                let inc = if self.ram()[0x00FF] & 0x04 != 0 { 32u16 } else { 1u16 };
                self.render.write_ppudata(v, inc);
            }
            _ => {
                // APU 面（$4000-$4017）：壳层音频消费源（apu_log 捕获——驱动逐帧 drain）
                if (0x4000..=0x4017).contains(&addr) {
                    self.apu_log.push((addr, v));
                }
            } // 其余寄存器面 S5 骨架暂不建模
        }
        // 其余寄存器面 S5 骨架暂不建模（RAM watch 不可见）；渲染面挂 task-13
    }

    /// 迭代内帧等待（**WaitNMIPass/WriteAndWait 族**（asm prg7:1108-1140 `ClearNMIStat`
    /// +spin 等下一 NMI）——1 tick 等待+NMI 内块全跑（含 ReadJoyPads）。定谳件
    /// findings/move-drift-iteration-scheduling.md R2）。
    pub fn iter_wait(&mut self) {
        self.iter_gates.push(1);
    }

    /// 迭代重活 **bogged 帧**（NMIStatus 跳过门（asm prg7:251 `lda NMIStatus/bne`）
    /// ——内块跳过（无 ReadJoyPads/palette/VRAM）=原版掉帧窗（D4 保真项）。R3 件。
    pub fn iter_bogged(&mut self, n: u8) {
        for _ in 0..n {
            self.iter_gates.push(0);
        }
    }

    /// 臂内帧判定（**Step 3 谓词单源**——engine 缝抑制与 mainloop 续段共用同一实现，
    /// 判定用 tick 首态=上一拍分派遗留）：同相位续段（cursor>0 且归属相位==当前相位
    /// ——F4 孤儿语义）｜完成拍哨兵（0xFF，Step 4——显式排除出续段：完成拍无 prelude
    /// 无分发）｜spin 等待段（r23）。臂内帧=asm 例程体内自旋/续段/完成拍——无 C0D3
    /// 执行 ⇒ 无缝行、无 prelude（map-v2 §4-④/F10——与 span 三性质同源）。
    pub fn title_continuation(&self) -> bool {
        let c = self.title_cursor;
        (c > 0 && c != 0xFF
            && self.title_cursor_phase == self.read_ram(crate::ram_map::TITLE_ROUTINE))
            || c == 0xFF
            || self.title_spin > 0
    }

    /// watch 状态向量（zp/iram/wram/pw，大写 hex——与 oracle 帧 trace 同形）。
    /// **编码唯一源=memmap::observe::frame_face**（严重-6 对账闭合：废本层手写切片第二源）。
    /// 委托化=task-22 slot①（lead 例外授权：只动委托结构不动语义）；wram 全 8KB 钉死（严重-5）。
    /// pw=密码字节面（$6988-$698F——sc-item-pickup 系 manifest watch）。
    pub fn watch_snapshot(&self) -> Vec<(&'static str, String)> {
        let img = self.snapshot_nes();
        vec![
            ("zp", crate::memmap::observe::frame_face::encode_zp(&img.ram)),
            ("iram", crate::memmap::observe::frame_face::encode_iram(&img.ram)),
            ("wram", crate::memmap::observe::frame_face::encode_wram(&img.wram)),
            ("pw", crate::memmap::observe::frame_face::encode_pw(&img.wram)),
        ]
    }
}


    /// 【S6-10 接线】write_io PPU 面喂入 RenderState（$2006 flip-flop+$2007 分域）。
    #[test]
    fn write_io_feeds_render_state() {
        let mut m = Manager::default();
        m.write_io(0x2006, 0x10); // hi
        m.write_io(0x2006, 0x00); // lo → $1000
        m.write_io(0x2007, 0xAB); // CHR 写
        assert_eq!(m.render.chr[0x1000], 0xAB);
        m.write_io(0x2006, 0x20);
        m.write_io(0x2006, 0x00); // $2000
        m.ram_mut()[0x00FF] = 0x04; // 32 增量
        m.write_io(0x2007, 0xCD); // 名表写+32 步进
        assert_eq!(m.render.nametable[0], 0xCD);
        assert_eq!(m.render.ppuaddr, 0x2020);
    }
#[cfg(test)]
mod tests {
    /// 【read_ram 映射修 TDD（2026-11-06）】$6877（WRAM $6000+0x877）读=wram[0x877]
    /// ——旧形 & 0x7FF 误读 ram[0x077]（255 垃圾——§47 全链根）。
    #[test]
    fn read_ram_wram_dispatch_6877() {
        let mut m = crate::state::Manager::default();
        m.wram_mut()[0x877] = 3; // EnergyTankCount=3（asm 语义：小值 1-5）
        assert_eq!(m.read_ram(0x6877), 3, "WRAM 分派（$6877→wram[0x877]）");
        m.write_ram(0x6877, 5);
        assert_eq!(m.wram()[0x877], 5, "写径同分派");
        assert_eq!(m.read_ram(0x0077), m.ram()[0x077], "ram[0x077] 不受影响（镜像归一保持）");
    }

    use super::*;

    /// 寄存器面不漏 RAM 镜像（write_ram 的 &0x7FF 归一会把 $2000 漏进 $0000——
    /// tick_frame 测试实抓回归案；寄存器面唯一入口=write_io）。
    #[test]
    fn write_io_does_not_leak_into_ram_mirror() {
        let mut m = Manager::default();
        m.write_io(0x2000, 0x90);
        m.write_io(0x2005, 0x00);
        assert_eq!(m.read_ram(0x0000), 0x00);
        assert_eq!(m.read_ram(0x0005), 0x00);
    }

    /// 2KB 镜像归一 + watch 面编码（zp=512 hex、iram=3584 hex，大写）。
    #[test]
    fn ram_mirror_and_watch_face() {
        let mut m = Manager::default();
        m.write_ram(0x0800, 0xAB); // 镜像回 $0000
        assert_eq!(m.read_ram(0x0000), 0xAB);
        assert_eq!(m.read_ram(0x1000), 0xAB); // $1000 镜像面
        let snap = m.watch_snapshot();
        assert_eq!(snap[0].0, "zp");
        assert_eq!(snap[0].1.len(), 512);
        assert_eq!(snap[1].0, "iram");
        assert_eq!(snap[1].1.len(), 3584);
        assert!(snap[0].1.starts_with("AB"));
        assert!(snap[0].1.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()));
    }
}

/// **typed 回灌清单（唯一枚举源）**——`restore_mss` 只认这一处。
///
/// 两个面各给出「哪些地址由 typed 状态拥有」：
/// · RAM：`SamusState::TYPED_ADDRS`（$0300-$0316 的 23 个字节，投影/解码同源）
/// · WRAM：敌槽 `EnsExtra` 前 `0..=TYPED_MAX` 字节（基址 `$6AF4`、步长 16、`SLOT_COUNT` 槽）
///
/// 清单与「整段拷贝」是**互斥**的：整段拷贝先把 MSS 字节灌进 `storage`，随后清单里
/// 每个地址都被 typed 回灌并**在 storage 中清零**。判据由
/// `tests_mss::typed_manifest_exactly_covers_bypass_face_and_carves_out_storage`
/// 机械钉住（读径绕过 storage 的地址集合 == 本清单；清单地址的 storage 字节 == 0）。
/// 加 typed 字段只改 `TYPED_ADDRS`/`TYPED_MAX`，本清单自动跟随——**禁止第二份手写地址表**。
pub(crate) struct TypedRestoreManifest;

impl TypedRestoreManifest {
    /// RAM 面 typed 地址（$0000-$07FF 域内，未做 2KB 镜像归一）。
    pub(crate) fn ram_addrs() -> &'static [u16] {
        &crate::samus::state::SamusState::TYPED_ADDRS
    }

    /// WRAM 面 typed 地址迭代器（$6000-$7FFF，未做字节偏移变换）。
    pub(crate) fn wram_addrs() -> impl Iterator<Item = u16> {
        (0..crate::enemy::state::SLOT_COUNT as u16).flat_map(|slot| {
            (0..=crate::enemy::state::ensx::TYPED_MAX).map(move |off| 0x6AF4 + slot * 16 + off)
        })
    }
}

/// MSS↔Manager 映射（**spec §4 分工件**——docs/spec/state-bundle-schema.md §4
/// 接缝表；双腿任意起点收官缝）：
/// · RAM 区（$0000-$07FF）→ ram[0x800] **直映**（A 级）
/// · WRAM（$6000-$7FFF）→ wram[0x2000] **直映**
/// · CPU 寄存器（cpu.a/x/y/sp/pc/ps）→ regs 直映（ps↔p=status 同义名）
/// · mapper.prgReg/chrReg/prgMode=**memmap 扩展面**（task-14 A9 后遗同面——随接）
/// · ppu.*=S6 渲染腿（bundle v1 不含）；apu/clock=时钟/RNG 对表（task-10 定谳后接）。
impl Manager {
    /// MSS 恢复构造（直映三面）。
    pub fn restore_mss(ram: &[u8], wram: &[u8], regs: Registers) -> Self {
        assert!(ram.len() >= 0x800, "MSS RAM 面不足 $800");
        assert!(wram.len() >= 0x2000, "MSS WRAM 面不足 $2000");
        let mut mem = Self::default();
        mem.ram_mut().copy_from_slice(&ram[..0x800]);
        mem.wram_mut().copy_from_slice(&wram[..0x2000]);
        // typed 域辖字段从字节面回灌（MSS 是外部真相面，恢复后必须重建 typed 状态）。
        // 枚举源=`TypedRestoreManifest`——下面清零用的是同一份清单（回灌/清零禁各写一份）。
        for &a in TypedRestoreManifest::ram_addrs() {
            mem.samus.set_byte(a, ram[a as usize & 0x7FF]);
        }
        // 敌槽 `EnsExtra` 前 7 字节（$6AF4 + slot*16）同样从字节面回灌——否则恢复后
        // 这些字段读回的是 typed 默认零值（同族事故：整面 dump 曾漏 typed）。
        for a in TypedRestoreManifest::wram_addrs() {
            let (slot, off) = crate::enemy::state::split_ens_extra(a)
                .expect("typed 清单地址必须落在敌槽面 $6AF4-$6BD3");
            mem.enemies[slot].set_byte(off, wram[(a - 0x6000) as usize & 0x1FFF]);
        }
        // 【spec §2 R3 / P1】**清零 typed 格**：typed 是唯一真相，`storage` 只留未迁族。
        // 不清零 ⇒ 字节面冻结一份「恢复时刻的旧值」，此后任何 typed 写都不再同步它，
        // `ram()`/`wram()` 便静默说谎（Task 1 刚修好的同源 dump 跟着说谎）——而读径照走
        // typed、矩阵照绿，只有字节面错。故清零与回灌必须用同一份清单、成对执行。
        for &a in TypedRestoreManifest::ram_addrs() {
            mem.ram_mut()[(a & 0x7FF) as usize] = 0;
        }
        for a in TypedRestoreManifest::wram_addrs() {
            mem.wram_mut()[((a - 0x6000) & 0x1FFF) as usize] = 0;
        }
        mem.regs = regs;
        mem.nmi_enabled = mem.ram()[0x00FF] & 0x80 != 0;
        // 「typed 是唯一真相、storage 只留未迁族」——恢复完成后立刻钉住：回灌漏项或
        // 清零漏项都在这里 fail-loud，而不是等 dump 差分再回溯。
        debug_assert!(
            TypedRestoreManifest::ram_addrs().iter().all(|&a| {
                let i = (a & 0x7FF) as usize;
                mem.ram()[i] == 0 && mem.read_ram(a) == ram[i]
            }) && TypedRestoreManifest::wram_addrs().all(|a| {
                let i = ((a - 0x6000) & 0x1FFF) as usize;
                mem.wram()[i] == 0 && mem.read_wram(a) == wram[i]
            }),
            "restore_mss：typed 格必须在 storage 清零（typed 是唯一真相，storage 只留未迁族）"
        );
        mem
    }

    /// 导出 MSS 面（反向——三直映；bundle 生产侧用）。
    pub fn capture_mss(&self) -> (Vec<u8>, Vec<u8>, Registers) {
        let img = self.snapshot_nes();
        (img.ram.to_vec(), img.wram.to_vec(), self.regs)
    }
}

#[cfg(test)]
mod mss_map_tests {
    /// MSS 往返必须保住 typed 域：`capture_mss` 走投影、`restore_mss` 回灌 typed。
    /// （漏一个方向 = 快照恢复后 Samus 状态回零，而像素面可能很久才显形。）
    #[test]
    fn mss_roundtrip_preserves_typed_domains() {
        let mut mem = Manager::default();
        mem.write_ram(0x0300, 9); // status = Elevator
        mem.write_ram(0x0308, 0xFE); // speedY = -2
        mem.write_ram(0x030E, 0xD8); // x
        let (ram, wram, regs) = mem.capture_mss();
        assert_eq!((ram[0x0300], ram[0x0308], ram[0x030E]), (9, 0xFE, 0xD8), "投影必须含 typed 字段");

        let mut back = Manager::restore_mss(&ram, &wram, regs);
        assert_eq!(back.samus().status_raw(), 9, "restore 后 status 必须回灌 typed");
        assert_eq!(back.samus().speed_y(), -2);
        assert_eq!(back.samus().x(), 0xD8);
        assert_eq!(back.snapshot_nes().ram[0x0300], 9, "再次投影一致");
    }

    /// **敌槽回灌不能漏**：`restore_mss` 里那段 `$6AF4 + slot*16` 的循环若被删掉，
    /// 上面那条只覆盖 Samus 三字节的测仍会全绿——这里把 12 槽 × 7 字节钉住。
    #[test]
    fn mss_roundtrip_preserves_enemy_typed_slots() {
        let mut mem = Manager::default();
        for slot in 0..crate::enemy::state::SLOT_COUNT as usize {
            mem.enemies[slot].set_byte(0, slot as u8 + 1); // status
            mem.enemies[slot].set_byte(3, slot as u8 + 0x20); // animFrame
            mem.enemies[slot].set_byte(6, slot as u8 + 0x40); // animIndex
        }
        let (ram, wram, regs) = mem.capture_mss();
        let back = Manager::restore_mss(&ram, &wram, regs);
        for slot in 0..crate::enemy::state::SLOT_COUNT as usize {
            assert_eq!(back.enemies[slot].get_byte(0), Some(slot as u8 + 1), "槽 {slot} status");
            assert_eq!(back.enemies[slot].get_byte(3), Some(slot as u8 + 0x20), "槽 {slot} animFrame");
            assert_eq!(back.enemies[slot].get_byte(6), Some(slot as u8 + 0x40), "槽 {slot} animIndex");
        }
    }

    use super::*;

    /// MSS 映射保真（三直映往返+nmi 推导）。
    #[test]
    fn restore_mss_maps_all_faces() {
        let mut ram = vec![0u8; 0x800];
        ram[0x001A] = 1;
        ram[0x00FF] = 0x90;
        let mut wram = vec![0u8; 0x2000];
        wram[0x1234] = 0xAB;
        let regs = Registers { a: 1, x: 2, y: 3, sp: 4, pc: 0x8000, p: 0x24 }; // MSS ps↔p（status 同义）
        let mem = Manager::restore_mss(&ram, &wram, regs);
        assert_eq!(mem.read_ram(0x001A), 1);
        assert_eq!(mem.read_wram(0x7234), 0xAB);
        assert_eq!((mem.regs.a, mem.regs.pc), (1, 0x8000));
        assert!(mem.nmi_enabled);
        let (r2, w2, regs2) = mem.capture_mss();
        assert_eq!(&r2[..0x800], &ram[..]);
        assert_eq!(&w2[..0x2000], &wram[..]);
        assert_eq!(regs2, regs);
    }
}

impl Manager {
    /// 实体句柄入口：`mem.samus()` —— 三个实体目录的唯一构造点。
    ///
    /// 视图持 `&mut self.ram_mut()`，因此一次只能持一个域（`samus::state` 细则 3）；领域代码
    /// 经它读写 `$0300-$0315`，不再拼地址。
    pub fn samus(&mut self) -> crate::samus::state::Samus<'_> {
        crate::samus::state::Samus::new(self)
    }

    /// 敌槽语义句柄（`$0400+i*16` / `$6AF4+i*16`）——槽号而非字节偏移。
    pub fn enemy(&mut self, slot: u8) -> crate::enemy::state::Enemy<'_> {
        crate::enemy::state::Enemy::new(self, slot)
    }
}

/// **P1·R3 断言组**：`restore_mss` 之后 typed 格不得在 `storage` 里留副本。
///
/// 背景（spec §2 R3）：`restore_mss` 曾把 0x800 字节无条件拷回 `storage.ram`，于是
/// typed 字段在字节面留一份**恢复时刻的旧值**——读径走 typed 所以矩阵全绿，但字节面
/// 已不再描述状态；Task 1 刚让 dump 尾段与 RAM 同源，这份 stale 会直接让 dump 说谎。
#[cfg(test)]
mod tests_mss {
    use super::*;
    use crate::enemy::state::EnemyStatus;
    use crate::samus::state::SamusStatus;

    /// 用例2（brief Step 1·7）：restore 后 storage 无 typed 副本，读径仍走 typed。
    #[test]
    fn restore_leaves_no_typed_copy_in_storage() {
        let mut src = crate::state::Manager::default();
        src.samus().set_status(crate::samus::state::SamusStatus::Roll);
        src.enemy(3).set_status(crate::enemy::state::EnemyStatus::Active);
        let (ram, wram, regs) = src.capture_mss();
        let mut restored = crate::state::Manager::restore_mss(&ram, &wram, regs);
        assert_eq!(restored.ram()[0x0300], 0, "storage 不得保留 typed 副本");
        assert_eq!(restored.read_ram(0x0300), 3, "读径仍走 typed");
        assert_eq!(restored.enemy(3).status(), crate::enemy::state::EnemyStatus::Active);
    }

    /// 全非零字节面（`i%255+1` ∈ 1..=255）。**非零是判据的一部分**：typed 格若留在
    /// storage 且值相同，"读径 vs 存储"的差分就看不见它——用零字节会让漏清零假绿。
    fn nonzero_face(n: usize) -> Vec<u8> {
        (0..n).map(|i| (i % 255 + 1) as u8).collect()
    }

    /// 对抗问 2+3：typed 回灌清单与「整段拷贝」无交集，且清单**恰好**覆盖 typed 面。
    ///
    /// 判据不是"清单看起来对"，而是机械对账两遍：
    /// · 逐地址：读径 == 输入字节（回灌真发生）且 storage == 0（整段拷贝被抹掉）；
    /// · 全集：`{a | read(a) != storage[a]}` **等于**清单集合——多一个=漏清零，
    ///   少一个=漏回灌（两种情况都会让字节面变成第二真相/读回陈旧值）。
    #[test]
    fn typed_manifest_exactly_covers_bypass_face_and_carves_out_storage() {
        let ram = nonzero_face(0x800);
        let wram = nonzero_face(0x2000);
        let mem = Manager::restore_mss(&ram, &wram, Registers::default());

        for &a in TypedRestoreManifest::ram_addrs() {
            let i = (a & 0x7FF) as usize;
            assert_eq!(mem.read_ram(a), ram[i], "{a:#06X}：typed 回灌值");
            assert_eq!(mem.ram()[i], 0, "{a:#06X}：整段拷贝必须被清零");
        }
        for a in TypedRestoreManifest::wram_addrs() {
            let i = ((a - 0x6000) & 0x1FFF) as usize;
            assert_eq!(mem.read_wram(a), wram[i], "{a:#06X}：敌槽 typed 回灌值");
            assert_eq!(mem.wram()[i], 0, "{a:#06X}：敌槽整段拷贝必须被清零");
        }

        let mut bypass: Vec<u16> = (0..0x800u16)
            .filter(|&a| mem.read_ram(a) != mem.ram()[a as usize])
            .collect();
        let mut manifest: Vec<u16> = TypedRestoreManifest::ram_addrs().to_vec();
        bypass.sort_unstable();
        manifest.sort_unstable();
        assert_eq!(bypass, manifest, "RAM 面：读径绕过 storage 的地址集合 != typed 清单");

        // 【I-1】**扫全 WRAM 面**（$6000-$7FFF），不是只扫敌槽窗：
        // 旧窗右端 `$6AF4+SLOT_COUNT*16` 是左闭右开、恰好顶到爆炸额外面基址 `$6BB4` 之前，
        // 于是「$6BB4 被 typed 化但漏接清单」在该窗内看不见（identity 映射时 storage 与
        // read 同值），只能靠全脸往返测兜底——那正是本计划要消灭的静默漂移。
        let mut wbypass: Vec<u16> = (0x6000u16..0x8000)
            .filter(|&a| mem.read_wram(a) != mem.wram()[((a - 0x6000) & 0x1FFF) as usize])
            .collect();
        let mut wmanifest: Vec<u16> = TypedRestoreManifest::wram_addrs().collect();
        wbypass.sort_unstable();
        wmanifest.sort_unstable();
        assert_eq!(
            wbypass, wmanifest,
            "WRAM 面：读径绕过 storage 的地址集合 != typed 清单（全 $6000-$7FFF）"
        );
    }

    /// **I-1 回归**：爆炸额外面（`enemy::explosion::EXTRA_BASE + slot*EXTRA_STRIDE + off`）
    /// **不是** typed 域——它必须留在字节面，且不得落进「读径绕过 storage」集合。
    ///
    /// 区间一律从 `enemy::explosion` 常量推导（不手写 `$6BB4`），并断言它**紧跟**敌槽 typed 面
    /// 末格（`ENS_EXTRA_BASE + SLOT_COUNT*16 - 1`）：两域相邻，任何一侧步长/槽数改动都会红。
    /// 旧形把扫描窗右端截在 `$6BB4` 之前，故「$6BB4 被 typed 化但漏接清单」在窗内不可见。
    #[test]
    fn explosion_extra_face_stays_outside_typed_manifest() {
        use crate::enemy::explosion::{EXTRA_BASE, EXTRA_STRIDE, SLOT_COUNT as EX_SLOTS};
        let face: Vec<u16> = (0..EX_SLOTS as u16)
            .flat_map(|slot| (0..EXTRA_STRIDE).map(move |off| EXTRA_BASE + slot * EXTRA_STRIDE + off))
            .collect();

        let manifest: Vec<u16> = TypedRestoreManifest::wram_addrs().collect();
        let overlap: Vec<u16> = face.iter().copied().filter(|a| manifest.contains(a)).collect();
        assert!(overlap.is_empty(), "爆炸额外面与敌槽 typed 清单重叠：{overlap:?}");

        let ens_end = crate::enemy::state::ENS_EXTRA_BASE
            + crate::enemy::state::SLOT_COUNT as u16 * 16
            - 1;
        assert_eq!(face[0], ens_end + 1, "爆炸额外面必须紧接敌槽 typed 面末格");
        assert_eq!(
            *face.last().unwrap(),
            EXTRA_BASE + EX_SLOTS as u16 * EXTRA_STRIDE - 1,
            "额外面末格必须由 base+slots*stride 推出"
        );

        let ram = nonzero_face(0x800);
        let wram = nonzero_face(0x2000);
        let mem = Manager::restore_mss(&ram, &wram, Registers::default());
        for a in face {
            let i = ((a - 0x6000) & 0x1FFF) as usize;
            assert_eq!(mem.wram()[i], wram[i], "{a:#06X}：爆炸额外面是未迁族，必须留在字节面");
            assert_eq!(mem.read_wram(a), wram[i], "{a:#06X}：爆炸额外面读径不得绕过 storage");
        }
    }

    /// 对抗问 4：既有行为不得被改掉——非 typed 字节逐字节原样恢复，
    /// 且 `capture(restore(x))` 两面对 x 逐字节相等（typed 面按 typed 重建）。
    ///
    /// 这条挡的是"顺手把整段拷贝改成子集化"（漏恢复某段未 typed 字节）这类回归。
    #[test]
    fn restore_preserves_untyped_bytes_and_roundtrips_whole_face() {
        let ram = nonzero_face(0x800);
        let wram = nonzero_face(0x2000);
        let regs = Registers { a: 7, x: 9, y: 11, sp: 0xFD, pc: 0xC0DE, p: 0x31 };
        let mem = Manager::restore_mss(&ram, &wram, regs);

        let typed_ram: Vec<u16> = TypedRestoreManifest::ram_addrs().to_vec();
        for a in 0..0x800u16 {
            if typed_ram.contains(&a) {
                continue;
            }
            let i = a as usize;
            assert_eq!(mem.ram()[i], ram[i], "{a:#06X}：未迁族必须留在字节面");
            assert_eq!(mem.read_ram(a), ram[i], "{a:#06X}：未迁族读径同值");
        }
        let typed_wram: Vec<u16> = TypedRestoreManifest::wram_addrs().collect();
        for a in 0x6000u16..0x8000 {
            if typed_wram.contains(&a) {
                continue;
            }
            let i = (a - 0x6000) as usize;
            assert_eq!(mem.wram()[i], wram[i], "{a:#06X}：WRAM 未迁族必须留在字节面");
            assert_eq!(mem.read_wram(a), wram[i], "{a:#06X}：WRAM 未迁族读径同值");
        }

        let (r2, w2, regs2) = mem.capture_mss();
        assert_eq!(r2, ram, "capture(restore(x)) 的 RAM 面必须逐字节等于 x");
        assert_eq!(w2, wram, "capture(restore(x)) 的 WRAM 面必须逐字节等于 x");
        assert_eq!(regs2, regs);
    }

    /// 修复的**危害面**：restore 之后 typed 写不得让字节面复活旧值。
    /// 旧形留副本时，副本只冻结在"恢复时刻"——之后任何 typed 写都与它分叉，
    /// `ram()` 便悄悄说谎（Task 1 的同源 dump 跟着说谎）。
    #[test]
    fn typed_write_after_restore_cannot_revive_stale_storage() {
        let mut src = Manager::default();
        src.samus().set_status(SamusStatus::Roll); // 非零：旧形会把 3 冻结在 storage
        src.enemy(0).set_status(EnemyStatus::Active); // 同上（2）
        let (ram, wram, regs) = src.capture_mss();
        let mut mem = Manager::restore_mss(&ram, &wram, regs);
        mem.samus().set_status(SamusStatus::Elevator);
        mem.enemy(0).set_status(EnemyStatus::Frozen);
        assert_eq!(mem.ram()[0x0300], 0, "typed 写不得在字节面留旧值");
        assert_eq!(mem.wram()[(0x6AF4 - 0x6000) as usize], 0, "敌槽同病同查");
        assert_eq!(mem.snapshot_nes().ram[0x0300], 9, "投影才是字节面唯一出口");
        assert_eq!(mem.snapshot_nes().wram[(0x6AF4 - 0x6000) as usize], 4);
    }
}
