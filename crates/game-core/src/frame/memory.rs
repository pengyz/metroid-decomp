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
                self.render.ppuctrl = v; // 全寄存器面（图案表选 bit3/4——渲染腿）
            }
            // $2001 PPUMASK 真寄存器面（asm `WritePPUCtrl: ; 07:C44D` 的
            // `lda PPUMASK_ZP / sta PPUMASK`（prg7_engine.asm:517）落点——NMI 内每帧落一次；
            // ScreenOff/ScreenNmiOff 只改影子 $FE，「渲染关」由本写进 RenderState 生效。
            0x2001 => self.render.ppumask = v,
            0x2005 => self.render.write_reg_2005(v), // PPUSCROLL（V1 摄像机——双写锁存）
            0x2006 => self.render.write_reg_2006(v), // PPUADDR flip-flop（S6-10 接线）
            0x2007 => {
                // PPUDATA→RenderState 喂入（S6-10 ①——840ad00 建模件运行时喂入面）
                // 增量位=PPUCTRL bit2（1/32）——从 ram $FF 读
                let inc = if self.ram()[0x00FF] & 0x04 != 0 {
                    32u16
                } else {
                    1u16
                };
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
    // ---- 密码屏 / 特项历史 语义视图 ----------------------------------------
    // 族内尚未 typed 化（asm 临时量与 WRAM 状态位），按 `raw.rs` 的迁移纪律在**本兼容
    // 辖**收口（`data/status/raw_surface.json` 只减不增：域件不得直接摸裸面）。
    // asm 根：`AddItemToHistory ; 07:DC54`、`CheckForItem ; 07:EE4A`、
    // `UniqueItemSearch ; 00:8B9C`、`CalculatePassword ; 00:8C7A`、
    // `LoadPasswordCharToVRAMStringRAM ; 00:93F9`。

    /// `NumberOfUniqueItems`（$6886）——**字节计数**（每件 +2；asm 07:DC54 `iny/iny`）。
    pub fn unique_item_count(&self) -> u8 {
        self.read_wram(crate::ram_map::NUMBER_OF_UNIQUE_ITEMS)
    }

    /// `UniqueItemHistory`（$6887+off）的 2 字节项（lo,hi；`CreateItemID` 字节序）。
    pub fn unique_item_word_at(&self, off: u16) -> (u8, u8) {
        let base = crate::ram_map::UNIQUE_ITEM_HISTORY + off;
        (self.read_wram(base), self.read_wram(base + 1))
    }

    /// `AddItemToHistory ; 07:DC54`：按**字节游标**追加（写址=count，count+=2）；
    /// 越过 256 字节预算 ⇒ false（容纳不下，asm 无越界保护，调用点保持原样）。
    pub fn unique_item_push(&mut self, id: (u8, u8)) -> bool {
        let n = self.unique_item_count() as u16;
        if n + 2 > 0x100 {
            return false;
        }
        let base = crate::ram_map::UNIQUE_ITEM_HISTORY + n;
        self.write_wram(base, id.0);
        self.write_wram(base + 1, id.1);
        self.write_wram(crate::ram_map::NUMBER_OF_UNIQUE_ITEMS, (n + 2) as u8);
        true
    }

    /// `CheckForItem ; 07:EE4A`：从尾部按 2 B 步长反向扫（先比 hi、再比 lo）。
    pub fn unique_item_find(&self, id: (u8, u8)) -> bool {
        let mut y = self.unique_item_count();
        while y >= 2 {
            let (lo, hi) = self.unique_item_word_at(y as u16 - 2);
            if hi == id.1 && lo == id.0 {
                return true;
            }
            y -= 2;
        }
        false
    }

    /// `PasswordByte`（$6988）第 i 字节。
    pub fn password_byte(&self, i: u16) -> u8 {
        self.read_ram(crate::ram_map::PASSWORD_BYTE + i)
    }

    /// 写回 `PasswordByte`（`CalculatePassword` 尾的 18 B 落盘面）。
    pub fn password_set_byte(&mut self, i: u16, v: u8) {
        self.write_ram(crate::ram_map::PASSWORD_BYTE + i, v);
    }

    /// `PasswordChar`（$699A）第 i 字节（=名表 tile 号，`LoadPasswordChar` 产物）。
    pub fn password_char(&self, i: u16) -> u8 {
        self.read_ram(crate::ram_map::PASSWORD_CHAR + i)
    }

    /// 整块落盘（`CalculatePassword` 尾：18 B PasswordByte + 24 B PasswordChar）。
    pub fn password_store(&mut self, bytes: &[u8; 18], chars: &[u8; 24]) {
        for (i, b) in bytes.iter().enumerate() {
            self.password_set_byte(i as u16, *b);
        }
        for (i, c) in chars.iter().enumerate() {
            self.write_ram(crate::ram_map::PASSWORD_CHAR + i as u16, *c);
        }
    }

    /// `CalculatePassword ; 00:8C7A` 的进度输入面（游戏状态字段，随机字节单独取）：
    /// (InArea&$3F, suitless, SamusGear, MissileCount, KraidStatueStatus,
    ///  RidleyStatueStatus, SamusAge[0..4])。
    pub fn password_inputs(&self) -> [u8; 10] {
        let r = |a: u16| self.read_ram(a);
        let mut age = [0u8; 4];
        for (i, v) in age.iter_mut().enumerate() {
            *v = r(crate::ram_map::SAMUS_AGE + i as u16);
        }
        [
            r(crate::ram_map::IN_AREA) & 0x3F,
            u8::from(r(crate::ram_map::JUST_IN_BAILEY) != 0),
            r(crate::ram_map::SAMUS_GEAR),
            r(crate::ram_map::MISSILE_COUNT),
            r(crate::ram_map::KRAID_STATUE_STATUS),
            r(crate::ram_map::RIDLEY_STATUE_STATUS),
            age[0],
            age[1],
            age[2],
            age[3],
        ]
    }

    /// `RandomNumber1`（$2E）——`CalculatePassword` 的 `$0F` 掩码环取值面。
    pub fn random_number1(&self) -> u8 {
        self.read_ram(crate::ram_map::RANDOM1)
    }

    /// `PrepareItemID ; 07:EE3D` + `GetItemXYPos ; 07:DC1C` 的输入面：
    /// (MapPosX, MapPosY, ScrollDir, ScrollX, ScrollY, PPUCTRL_ZP)。
    pub fn item_spawn_inputs(&self) -> (u8, u8, u8, u8, u8, u8) {
        let r = |a: u16| self.read_ram(a);
        (
            r(crate::ram_map::MAP_POS_X),
            r(crate::ram_map::MAP_POS_Y),
            r(crate::ram_map::SCROLL_DIR),
            r(crate::ram_map::SCROLL_X),
            r(crate::ram_map::SCROLL_Y),
            r(crate::ram_map::PPUCTRL_ZP),
        )
    }

    /// PPUMASK 影子（$FE；渲染权威面——`nmi.rs` 每帧据此刷真 $2001）。
    pub fn ppumask_shadow(&self) -> u8 {
        self.read_ram(0x00FE)
    }

    /// 写 PPUMASK 影子（`ScreenOff/ScreenOn ; 07:C439/C447` 的 `sta PPUMASK_ZP` 面）。
    pub fn set_ppumask_shadow(&mut self, v: u8) {
        self.write_ram(0x00FE, v);
    }

    /// `ClearNMIStat ; 07:C434`（`lda #0 / sta NMIStatus`——自旋入口）。
    pub fn clear_nmi_status(&mut self) {
        self.write_ram(crate::ram_map::NMI_STATUS, 0);
    }

    pub fn title_continuation(&self) -> bool {
        let c = self.title_cursor;
        (c > 0
            && c != 0xFF
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
            (
                "zp",
                crate::memmap::observe::frame_face::encode_zp(&img.ram),
            ),
            (
                "iram",
                crate::memmap::observe::frame_face::encode_iram(&img.ram),
            ),
            (
                "wram",
                crate::memmap::observe::frame_face::encode_wram(&img.wram),
            ),
            (
                "pw",
                crate::memmap::observe::frame_face::encode_pw(&img.wram),
            ),
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
        assert_eq!(
            m.read_ram(0x0077),
            m.ram()[0x077],
            "ram[0x077] 不受影响（镜像归一保持）"
        );
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
        assert!(snap[0]
            .1
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()));
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
    ///
    /// 枚举源=各域的地址表（`SamusState::TYPED_ADDRS` / `HealthChange::ADDRS` / …）：
    /// 加 typed 域只在 chain 上接一段，**禁止第二份手写地址表**。
    pub(crate) fn ram_addrs() -> impl Iterator<Item = u16> {
        crate::samus::state::SamusState::TYPED_ADDRS
            .iter()
            .copied()
            .chain(crate::samus::health_change::HealthChange::ADDRS)
            // 血量两字节（`$0106/$0107`）：B1 起由 `SamusVitals` typed 认领，
            // MSS 回灌/清零必须一并覆盖（漏了它 = 存档丢血）。
            .chain(crate::memmap::samus_vitals_face::SamusVitals::ADDRS)
            .chain(crate::sound::state::SoundInitFlags::ADDRS)
            .chain(crate::world::object_globals::ObjectGlobals::ADDRS)
            .chain(crate::world::scroll_blocks::ScrollBlocks::ADDRS)
            .chain(crate::render::oam_shadow::OamShadow::addrs())
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
        // 【多帧载房任务机（2026-11-19 A-lite）】任务机相位不在 RAM 面（非 RAM 相位字段），
        // 但可以从 `$005A RoomNumber` 的语义**重建**：窗内 RAM 里就是**要载的房号**
        // （`< $F0 && != $FF`）⇒ 恢复后按「窗已开」重建，避免把 `RoomNumber` 当成新请求重装
        // 第二遍（重复掉帧窗 + `delete_offscreen_room_sprites`/`scan_for_items` 二次副作用）。
        // 精度边界：剩余窗长（`iter_gates`）本身也不在 MSS 面，故只能恢复相位、不能恢复剩余拍数。
        {
            // 判据（对抗复核 P1 修正）：`$5A < $F0` **且** `$33/$34` 已落在 WRAM（`$6000-$7FFF`）
            // 才算「窗内」。只看 `$5A` 会把**上电初值 `$5A=$00`**（标题屏/密码屏 bundle 里就是它，
            // `analysis/bundles/title-recon.json` 等 4 件）误判成「正在装 0 号房」⇒ 开局幻影尾段
            // （`$5A` 被改成 `F0→F1→F3→FF`）与 `attrib_table_write` 在 `room_ptr=$0000` 上下溢。
            let rn = mem.ram()[crate::ram_map::ROOM_NUMBER as usize & 0x7FF];
            let ptr = mem.room().room_ptr();
            if rn != 0xFF && rn < 0xF0 && (0x6000..0x8000).contains(&ptr) {
                mem.room_load_phase = 1;
                mem.room_load_room = rn;
                mem.room_load_tail = 0;
            }
        }
        mem.wram_mut().copy_from_slice(&wram[..0x2000]);
        // typed 域辖字段从字节面回灌（MSS 是外部真相面，恢复后必须重建 typed 状态）。
        // 枚举源=`TypedRestoreManifest`——下面清零用的是同一份清单（回灌/清零禁各写一份）。
        for a in crate::samus::state::SamusState::TYPED_ADDRS {
            mem.samus.set_byte(a, ram[a as usize & 0x7FF]);
        }
        for a in crate::samus::health_change::HealthChange::ADDRS {
            assert!(
                mem.health_change.set_byte(a, ram[a as usize & 0x7FF]),
                "`HealthChange` 地址必须在 typed 域内"
            );
        }
        for a in crate::memmap::samus_vitals_face::SamusVitals::ADDRS {
            assert!(
                mem.samus.vitals.set_byte(a, ram[a as usize & 0x7FF]),
                "`SamusVitals` 地址必须在 typed 域内"
            );
        }
        for a in crate::world::scroll_blocks::ScrollBlocks::ADDRS {
            assert!(
                mem.scroll_blocks.set_byte(a, ram[a as usize & 0x7FF]),
                "`ScrollBlocks` 地址必须在 typed 域内"
            );
        }
        for a in crate::sound::state::SoundInitFlags::ADDRS {
            assert!(
                mem.sound_init.set_byte(a, ram[a as usize & 0x7FF]),
                "`SoundInitFlags` 地址必须在 typed 域内"
            );
        }
        for a in crate::world::object_globals::ObjectGlobals::ADDRS {
            assert!(
                mem.object_globals.set_byte(a, ram[a as usize & 0x7FF]),
                "`ObjectGlobals` 地址必须在 typed 域内"
            );
        }
        // OAM 可整页拷贝（`OamShadow::from_page`）——比逐址 set_byte 少 255 次分派。
        let mut oam_page = [0u8; crate::render::oam_shadow::OAM_LEN];
        oam_page.copy_from_slice(&ram[0x0200..0x0300]);
        mem.oam = crate::render::oam_shadow::OamShadow::from_page(oam_page);
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
        for a in TypedRestoreManifest::ram_addrs() {
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
            TypedRestoreManifest::ram_addrs().all(|a| {
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
        assert_eq!(
            (ram[0x0300], ram[0x0308], ram[0x030E]),
            (9, 0xFE, 0xD8),
            "投影必须含 typed 字段"
        );

        let mut back = Manager::restore_mss(&ram, &wram, regs);
        assert_eq!(
            back.samus().status_raw(),
            9,
            "restore 后 status 必须回灌 typed"
        );
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
            assert_eq!(
                back.enemies[slot].get_byte(0),
                Some(slot as u8 + 1),
                "槽 {slot} status"
            );
            assert_eq!(
                back.enemies[slot].get_byte(3),
                Some(slot as u8 + 0x20),
                "槽 {slot} animFrame"
            );
            assert_eq!(
                back.enemies[slot].get_byte(6),
                Some(slot as u8 + 0x40),
                "槽 {slot} animIndex"
            );
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
        let regs = Registers {
            a: 1,
            x: 2,
            y: 3,
            sp: 4,
            pc: 0x8000,
            p: 0x24,
        }; // MSS ps↔p（status 同义）
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

    /// `EnemyStatusPreAI`（`$0081`，`constants_ram.asm`）——**AI 例程读的就是这一格**。
    ///
    /// `UpdateEnemy`（asm `prg7:10511 lda EnsExtra.0.status / sta EnemyStatusPreAI`）在
    /// `JumpEngine` 分派前把当时 status 快照进 `$0081`；其后 `try_becoming_active`
    /// （1→2）、`dec_status`（2→1）、`react_to_projectile`（→3/4/6）都会改写现役
    /// `EnsExtra.status`，**但 `$0081` 保持分派前的值**。因此 AI 例程若读现役 status 就会
    /// 在三类可达帧上与 asm 分叉（激活帧 / Active→Resting 帧 / 中弹冻结帧）——
    /// 这正是 `crawler.rs`、`skree.rs`、`update_enemy_common_decide` 都读本格的原因。
    ///
    /// 由 `frame/memory.rs`（`raw-surface` 豁免的兼容辖）收口裸读，新 AI 模块直接调用即可。
    /// `MetroidLatch0400+slot*$10`（`$77F8+slot`，六槽）——Metroid 吸附 Samus 的
    /// 锁存字节：bits0-3 = 吸附进度帧计数（到 `$0C` 满）、bits4-6 = 炸弹命中计数
    /// （到 5 次脱离）、bit7 = 吸附时 x 速度符号。
    ///
    /// asm 锚：`LoadEnemySlotIDIntoY ; 03:99B7`（`txa / jsr Adiv16_ / tay`）⇒ 下标是**槽号**
    /// 而不是字节偏移；`InitBank3 ; 07:C59A` 把 `MetroidData` 的 6 个 `$00` 拷到
    /// `$77F8-$77FD`。槽号越界（≥6）在原版是读 `$77FE` 之后的 `MetroidAccel` 表 —— 这里
    /// fail-loud，不静默回绕。
    /// `EndTimer+1`（`$010B`）——逃逸计时器高字节。`MetroidAIRoutine ; 03:97F9` 用它判断
    /// 「逃逸序列是否进行中」（`ldy EndTimer+1 / iny / beq`：`$FF` 表示未启动）。
    // ---- MotherBrain 面（`$0098-$009F`，`constants_ram.asm:708-733`）----
    /// `MotherBrainStatus`（`$0098`）。
    pub fn mb_status(&self) -> u8 {
        self.read_ram(crate::ram_map::MOTHER_BRAIN_STATUS)
    }
    pub fn set_mb_status(&mut self, v: u8) {
        self.write_ram(crate::ram_map::MOTHER_BRAIN_STATUS, v);
    }
    /// `MotherBrainQtyHits`（`$0099`）——union：消融期 `DeathStringID`、已死期 `TimeBombCounter`。
    pub fn mb_qty_hits(&self) -> u8 {
        self.read_ram(crate::ram_map::MOTHER_BRAIN_QTY_HITS)
    }
    pub fn set_mb_qty_hits(&mut self, v: u8) {
        self.write_ram(crate::ram_map::MOTHER_BRAIN_QTY_HITS, v);
    }
    /// `MotherBrainAnimBrainDelay`（`$009A`）——union：消融期 `DeathInstrID`。
    pub fn mb_brain_delay(&self) -> u8 {
        self.read_ram(crate::ram_map::MOTHER_BRAIN_BRAIN_DELAY)
    }
    pub fn set_mb_brain_delay(&mut self, v: u8) {
        self.write_ram(crate::ram_map::MOTHER_BRAIN_BRAIN_DELAY, v);
    }
    /// `MotherBrainAnimEyeDelay`（`$009B`，bit7=眼闭）。
    pub fn mb_eye_delay(&self) -> u8 {
        self.read_ram(crate::ram_map::MOTHER_BRAIN_EYE_DELAY)
    }
    pub fn set_mb_eye_delay(&mut self, v: u8) {
        self.write_ram(crate::ram_map::MOTHER_BRAIN_EYE_DELAY, v);
    }
    /// `MotherBrainAnimFrameTableID`（`$009C`）。
    pub fn mb_frame_id(&self) -> u8 {
        self.read_ram(crate::ram_map::MOTHER_BRAIN_FRAME_ID)
    }
    pub fn set_mb_frame_id(&mut self, v: u8) {
        self.write_ram(crate::ram_map::MOTHER_BRAIN_FRAME_ID, v);
    }
    /// `MotherBrainHi`（`$009D`）。
    pub fn mb_hi(&self) -> u8 {
        self.read_ram(crate::ram_map::MOTHER_BRAIN_HI)
    }
    pub fn set_mb_hi(&mut self, v: u8) {
        self.write_ram(crate::ram_map::MOTHER_BRAIN_HI, v);
    }
    /// `MotherBrainIsHit`（`$009E`）。
    pub fn mb_is_hit(&self) -> u8 {
        self.read_ram(crate::ram_map::MOTHER_BRAIN_IS_HIT)
    }
    pub fn set_mb_is_hit(&mut self, v: u8) {
        self.write_ram(crate::ram_map::MOTHER_BRAIN_IS_HIT, v);
    }
    /// `MotherBrainFlashDelay`（`$009F`）。
    pub fn mb_flash_delay(&self) -> u8 {
        self.read_ram(crate::ram_map::MOTHER_BRAIN_FLASH_DELAY)
    }
    pub fn set_mb_flash_delay(&mut self, v: u8) {
        self.write_ram(crate::ram_map::MOTHER_BRAIN_FLASH_DELAY, v);
    }
    /// `EndTimerEnemyHi`（`$010C`）/`EndTimerEnemyIsEnabled`（`$010D`）。
    pub fn end_timer_enemy_hi(&self) -> u8 {
        self.read_ram(crate::ram_map::END_TIMER_ENEMY_HI)
    }
    pub fn set_end_timer_enemy_hi(&mut self, v: u8) {
        self.write_ram(crate::ram_map::END_TIMER_ENEMY_HI, v);
    }
    pub fn end_timer_enemy_enabled(&self) -> u8 {
        self.read_ram(crate::ram_map::END_TIMER_ENEMY_ENABLED)
    }
    pub fn set_end_timer_enemy_enabled(&mut self, v: u8) {
        self.write_ram(crate::ram_map::END_TIMER_ENEMY_ENABLED, v);
    }
    /// `SFXMultiInitFlags`（`$0684`）的或写（asm 里各处 `ora #bit / sta`）。
    pub fn or_sfx_multi_init(&mut self, bit: u8) {
        let v = self.read_ram(crate::ram_map::SFX_MULTI_INIT) | bit;
        self.write_ram(crate::ram_map::SFX_MULTI_INIT, v);
    }
    /// `MusicInitFlags`（`$0692`）的或写（`music_Escape` 等）。
    pub fn or_music_init_flags(&mut self, bit: u8) {
        let v = self.read_ram(crate::sound::addrs::MUSIC_INIT_FLAGS) | bit;
        self.write_ram(crate::sound::addrs::MUSIC_INIT_FLAGS, v);
    }
    /// `EndTimer`（`$010A-$010B`，BCD 四位 `9999`）双字节写。
    pub fn set_end_timer_pair(&mut self, v: u8) {
        self.write_ram(crate::ram_map::END_TIMER, v);
        self.write_ram(crate::ram_map::END_TIMER + 1, v);
    }
    pub fn end_timer(&self) -> (u8, u8) {
        (
            self.read_ram(crate::ram_map::END_TIMER),
            self.read_ram(crate::ram_map::END_TIMER + 1),
        )
    }
    /// `MapPosX/MapPosY`（`$004F/$0050`）——母脑造门选槽与特项扫描共用。
    pub fn map_pos(&self) -> (u8, u8) {
        (
            self.read_ram(crate::ram_map::MAP_POS_X),
            self.read_ram(crate::ram_map::MAP_POS_Y),
        )
    }
    pub fn set_map_pos(&mut self, x: u8, y: u8) {
        self.write_ram(crate::ram_map::MAP_POS_X, x);
        self.write_ram(crate::ram_map::MAP_POS_Y, y);
    }
    /// `EndTimer` 双字节写（`UpdateEndTimer` 的 BCD 递减落盘）。
    pub fn store_end_timer(&mut self, lo: u8, hi: u8) {
        self.write_ram(crate::ram_map::END_TIMER, lo);
        self.write_ram(crate::ram_map::END_TIMER + 1, hi);
    }
    pub fn set_timer3(&mut self, v: u8) {
        self.write_ram(crate::ram_map::TIMER3, v);
    }

    /// `Timer3`（`$002C`，每 10 帧减 1）。
    pub fn timer3(&self) -> u8 {
        self.read_ram(crate::ram_map::TIMER3)
    }

    /// `PPUCTRL_ZP`（`$00FF`）——NMI 影子 PPUCTRL（bit0=名表基、bit2=VRAM 增量）。
    pub fn ppuctrl_zp(&self) -> u8 {
        self.read_ram(crate::ram_map::PPUCTRL_ZP)
    }
    pub fn set_ppuctrl_zp(&mut self, v: u8) {
        self.write_ram(crate::ram_map::PPUCTRL_ZP, v);
    }

    /// `EnemyFlipAfterDisplacementAnimIndex`（`$0085/$0086`）——位移后翻转用的动画索引。
    pub fn flip_anim_index(&self, left: bool) -> u8 {
        self.read_ram(0x0085 + left as u16)
    }
    pub fn set_flip_anim_index(&mut self, left: bool, v: u8) {
        self.write_ram(0x0085 + left as u16, v);
    }
    /// `RidleyFireballCounter`（`$0080`）——Ridley 吐火球的帧计数器。
    pub fn ridley_fireball_counter(&self) -> u8 {
        self.read_ram(crate::ram_map::RIDLEY_FIREBALL_COUNTER)
    }
    pub fn set_ridley_fireball_counter(&mut self, v: u8) {
        self.write_ram(crate::ram_map::RIDLEY_FIREBALL_COUNTER, v);
    }

    /// `Cannons`（`$6BF4-$6C73`，16 槽 ×8 B：status/y/x/hi/instrListID/angle/instrDelay/instrID）。
    /// 字段偏移由 `enemy::cannon::f` 定义——这里只提供按偏移的字节面，避免新文件接入裸面。
    pub fn cannon_byte(&self, index: u8, off: u16) -> u8 {
        self.read_wram(0x6BF4 + (index as u16) * 8 + off)
    }
    pub fn set_cannon_byte(&mut self, index: u8, off: u16, v: u8) {
        self.write_wram(0x6BF4 + (index as u16) * 8 + off, v);
    }

    /// `RinkaSpawners`（`$008B-$0090`，2 槽 ×3 B：status/hi/posIndex）——Tourian 母脑房
    /// 的 Rinka 生成器（`.struct RinkaSpawner`，`constants_ram.asm:10-16`）。
    fn rinka_spawner_addr(slot: u8, off: u8) -> u16 {
        0x008B + (slot as u16) * 3 + off as u16
    }
    pub fn rinka_spawner_status(&self, slot: u8) -> u8 {
        self.read_ram(Self::rinka_spawner_addr(slot, 0))
    }
    pub fn set_rinka_spawner_status(&mut self, slot: u8, v: u8) {
        self.write_ram(Self::rinka_spawner_addr(slot, 0), v);
    }
    pub fn rinka_spawner_hi(&self, slot: u8) -> u8 {
        self.read_ram(Self::rinka_spawner_addr(slot, 1))
    }
    pub fn set_rinka_spawner_hi(&mut self, slot: u8, v: u8) {
        self.write_ram(Self::rinka_spawner_addr(slot, 1), v);
    }
    pub fn rinka_spawner_pos_index(&self, slot: u8) -> u8 {
        self.read_ram(Self::rinka_spawner_addr(slot, 2))
    }
    pub fn set_rinka_spawner_pos_index(&mut self, slot: u8, v: u8) {
        self.write_ram(Self::rinka_spawner_addr(slot, 2), v);
    }

    /// 精灵 RAM（`$0200-$02FF`）单字节写——`DrawEndTimerEnemy` 直接改已画精灵的 tile 字节。
    /// `WeaponWaveDir=$0502`/`WeaponWaveInstrTimer=$0501`/`WeaponWaveInstrID=$0500`（按弹槽 y 索引）。
    pub fn weapon_wave_dir(&self, y: u8) -> u8 {
        self.read_wram(0x0502 + y as u16)
    }
    pub fn set_weapon_wave_dir(&mut self, y: u8, v: u8) {
        self.write_wram(0x0502 + y as u16, v);
    }
    pub fn set_weapon_wave_instr(&mut self, y: u8, timer: u8, id: u8) {
        self.write_wram(0x0501 + y as u16, timer);
        self.write_wram(0x0500 + y as u16, id);
    }
    /// `SamusDir=$004D`——0=右 1=左（大地图方向位）。
    pub fn samus_dir(&self) -> u8 {
        self.read_ram(crate::ram_map::SAMUS_DIR)
    }

    /// `MissileCount=$6879`（WRAM）。
    pub fn missile_count(&self) -> u8 {
        self.read_ram(crate::ram_map::MISSILE_COUNT)
    }
    pub fn set_missile_count(&mut self, v: u8) {
        self.write_ram(crate::ram_map::MISSILE_COUNT, v);
    }
    /// `MissileToggle=$010E`：0=普通弹 1=导弹（`constants_ram.asm:840`）。
    pub fn missile_toggle(&self) -> u8 {
        self.read_ram(crate::ram_map::MISSILE_TOGGLE)
    }
    pub fn set_missile_toggle(&mut self, v: u8) {
        self.write_ram(crate::ram_map::MISSILE_TOGGLE, v);
    }

    /// `PasswordByte` 18 B（`$6988` 起，`constants_ram.asm`）——`InitializeGame` 的输入面。
    pub fn password_bytes(&self) -> [u8; 18] {
        let mut pw = [0u8; 18];
        for (i, b) in pw.iter_mut().enumerate() {
            *b = self.read_ram(crate::ram_map::PASSWORD_BYTE + i as u16);
        }
        pw
    }

    /// `$69B2`：NARPASSWORD 判定字节（非 0 ⇒ `InitializeGame` 跳过 LoadPasswordData）。
    pub fn nar_password_flag(&self) -> bool {
        self.read_ram(0x69B2) != 0
    }

    pub fn set_oam_byte(&mut self, off: u8, v: u8) {
        self.write_ram(0x0200 + off as u16, v);
    }
    /// `SFXSQ1InitFlags`（`$0681`）的或写——`SetSoundInitFlag ; 07:CBE8`（`ora / sta`），
    /// 被 `SFX_*` 宏族调用（如 `SFX_OutOfPipe ; 07:CBA4` 的 `lda #$08`）。
    pub fn or_sfx_sq1_init(&mut self, bit: u8) {
        let v = self.read_ram(crate::sound::addrs::SQ1_INIT) | bit;
        self.write_ram(crate::sound::addrs::SQ1_INIT, v);
    }
    /// `PaletteDataPending`（`$001C`）——NMI 期按它（值 = 调色板 id + 1）挑 VRAM 结构。
    pub fn set_palette_pending(&mut self, v: u8) {
        self.write_ram(crate::ram_map::PALETTE_PENDING, v);
    }
    pub fn palette_pending(&self) -> u8 {
        self.read_ram(crate::ram_map::PALETTE_PENDING)
    }

    pub fn end_timer_hi(&mut self) -> u8 {
        self.read_ram(crate::ram_map::END_TIMER + 1)
    }

    /// `MetroidOnSamus`（`$92`）——有 Metroid 完全吸附并正在吸血。
    pub fn metroid_on_samus(&mut self) -> u8 {
        self.read_ram(crate::ram_map::METROID_ON_SAMUS)
    }

    pub fn set_metroid_on_samus(&mut self, v: u8) {
        self.write_ram(crate::ram_map::METROID_ON_SAMUS, v);
    }

    pub fn metroid_latch(&mut self, slot: u8) -> u8 {
        assert!(
            slot < 6,
            "MetroidLatch 只有 6 槽（$77F8-$77FD），越界是原版表尾读：{slot}"
        );
        self.read_ram(0x77F8 + slot as u16)
    }

    pub fn set_metroid_latch(&mut self, slot: u8, v: u8) {
        assert!(
            slot < 6,
            "MetroidLatch 只有 6 槽（$77F8-$77FD），越界是原版表尾读：{slot}"
        );
        self.write_ram(0x77F8 + slot as u16, v);
    }

    /// 写 `$0081 EnemyStatusPreAI`（`UpdateEnemy ; 07:F351` 每槽进入 AI 前快照，
    /// `crawler.asm:8` 等「按 AI 前状态选出口」的分支读它）。具名 setter 只为测试/工具面，
    /// 生产路径上它恒由 `UpdateEnemy` 写。
    pub fn set_enemy_status_pre_ai(&mut self, v: u8) {
        self.write_ram(crate::ram_map::ENEMY_STATUS_PRE_AI, v);
    }

    pub fn enemy_status_pre_ai(&mut self) -> u8 {
        self.read_ram(crate::ram_map::ENEMY_STATUS_PRE_AI)
    }

    /// `FrameCount`（`$002D`，`constants_ram.asm`）——**全局帧计数器**，asm AI 例程把它当
    /// 免费的周期计数器用（Dragon `lda FrameCount / and #$1F`＝每 32 帧一次、
    /// Polyp `adc FrameCount` 做 8/64 帧相序）。
    ///
    /// 【为什么要有这个入口】`$002D` 不属于任何 typed 域（它既不是敌槽也不是萨姆斯），
    /// 但新 AI 模块（`enemy/dragon.rs`、`enemy/polyp.rs`）按 `raw-surface` 纪律**不许**
    /// 新接入裸字节面——不带这个收集口，移植方只能二选一：破门禁或伪造计时。
    /// 与 `random_number1()` 同族（同一类 asm 全局量），收口在本豁免文件。
    pub fn frame_count(&self) -> u8 {
        self.read_ram(crate::ram_map::FRAME_COUNT)
    }

    // ---- TileBlasts 池（`$0500`，13 槽 × 16 B；`constants_ram.asm:986`）----
    // 【为什么收口在本文件】与 [`Self::frame_count`]/[`Self::kraid_lint_counter`] 同族：
    // `$0500` 不属于任何 typed 域（既不是敌槽也不是 RoomRAM），`world/tile_blast.rs`
    // 按 `raw-surface` 纪律不许新接入裸字节面，故池内字节读写统一走这里。
    // 字段偏移由 `world/tile_blast.rs` 的 asm 结构化访问（`TileBlasts.0.<field>,x`）钉住。

    /// TileBlasts 槽基址。
    fn tb_addr(slot: u8, off: u16) -> u16 {
        debug_assert!(slot < 13, "TileBlasts 槽号越界：{slot}");
        0x0500 + slot as u16 * 16 + off
    }
    //-------------------------------------------------- Statue（`$0360` 槽 + 双像状态字）
    // `UpdateAllStatues ; 07:D9D4` 族的字节面收口（`constants_ram.asm:922-933` 别名字段）：
    // Object 布局 status+0/hi+0C/y+0D/x+0E/data0F+0F ⇒ RaiseState `$0364/$0365`、
    // IsHit `$0366/$0367`、KraidY `$036F`、RidleyY `$0370`；状态字 `$687B/$687C`。

    /// `Statue.status`（`$0360`）。
    pub fn statue_status(&self) -> u8 {
        self.read_ram(0x0360)
    }
    /// 设 `Statue.status`。
    pub fn set_statue_status(&mut self, v: u8) {
        self.write_ram(0x0360, v);
    }
    /// `Statue.hi`（`$036C`）。
    pub fn statue_hi(&self) -> u8 {
        self.read_ram(0x036C)
    }
    /// 设 `Statue.hi`。
    pub fn set_statue_hi(&mut self, v: u8) {
        self.write_ram(0x036C, v);
    }
    /// `Statue.y`（`$036D`）。
    pub fn statue_y(&self) -> u8 {
        self.read_ram(0x036D)
    }
    /// 设 `Statue.y`。
    pub fn set_statue_y(&mut self, v: u8) {
        self.write_ram(0x036D, v);
    }
    /// `Statue.x`（`$036E`）。
    pub fn statue_x(&self) -> u8 {
        self.read_ram(0x036E)
    }
    /// 设 `Statue.x`。
    pub fn set_statue_x(&mut self, v: u8) {
        self.write_ram(0x036E, v);
    }
    /// `Statue.animFrame`（`$0363`）。
    pub fn set_statue_anim_frame(&mut self, v: u8) {
        self.write_ram(0x0363, v);
    }
    /// 像的 RaiseState（idx 0=Kraid `$0364` / 1=Ridley `$0365`）。
    pub fn statue_raise_state(&self, idx: u8) -> u8 {
        self.read_ram(0x0364 + (idx & 1) as u16)
    }
    /// 设 RaiseState。
    pub fn set_statue_raise_state(&mut self, idx: u8, v: u8) {
        self.write_ram(0x0364 + (idx & 1) as u16, v);
    }
    /// 像的 IsHit（`$0366/$0367`）。
    pub fn statue_is_hit(&self, idx: u8) -> u8 {
        self.read_ram(0x0366 + (idx & 1) as u16)
    }
    /// 设 IsHit。
    pub fn set_statue_is_hit(&mut self, idx: u8, v: u8) {
        self.write_ram(0x0366 + (idx & 1) as u16, v);
    }
    /// 像的当前 Y（Kraid `$036F` / Ridley `$0370`）。
    pub fn statue_raise_y(&self, idx: u8) -> u8 {
        self.read_ram(if idx == 0 { 0x036F } else { 0x0370 })
    }
    /// 设像的 Y。
    pub fn set_statue_raise_y(&mut self, idx: u8, v: u8) {
        self.write_ram(if idx == 0 { 0x036F } else { 0x0370 }, v);
    }
    /// 像的状态字（`KraidStatueStatus=$687B` / `RidleyStatueStatus=$687C`，bit7=up）。
    pub fn statue_status_word(&self, idx: u8) -> u8 {
        self.read_ram(if idx == 0 {
            crate::ram_map::KRAID_STATUE_STATUS
        } else {
            crate::ram_map::RIDLEY_STATUE_STATUS
        })
    }
    /// 设像的状态字。
    pub fn set_statue_status_word(&mut self, idx: u8, v: u8) {
        let a = if idx == 0 {
            crate::ram_map::KRAID_STATUE_STATUS
        } else {
            crate::ram_map::RIDLEY_STATUE_STATUS
        };
        self.write_ram(a, v);
    }
    /// `DoorEntryStatus`（`$0056`，非 0=正在门转场）。
    pub fn door_entry_status(&self) -> u8 {
        self.read_ram(crate::ram_map::DOOR_ENTRY_STATUS)
    }

    /// `SamusDoorData`（`$0058`，踩到门格时由 `SamusDoorData` 分派写入：`$A0` 门 +1、`$A1` 门 +2）。
    pub fn samus_door_data(&self) -> u8 {
        self.read_ram(crate::ram_map::SAMUS_DOOR_DATA)
    }


    /// `StatuesBridgeIsSpawned`（`$0054`）。
    pub fn statues_bridge_spawned(&self) -> u8 {
        self.read_ram(crate::ram_map::STATUES_BRIDGE_SPAWNED)
    }
    /// 设 `StatuesBridgeIsSpawned`。
    pub fn set_statues_bridge_spawned(&mut self, v: u8) {
        self.write_ram(crate::ram_map::STATUES_BRIDGE_SPAWNED, v);
    }

    /// 槽内 `routine`。
    pub fn tb_routine(&self, slot: u8) -> u8 {
        self.read_ram(Self::tb_addr(slot, 0))
    }
    /// 设 `routine`。
    pub fn set_tb_routine(&mut self, slot: u8, v: u8) {
        self.write_ram(Self::tb_addr(slot, 0), v);
    }
    /// 槽内 `delay`。
    pub fn tb_delay(&self, slot: u8) -> u8 {
        self.read_ram(Self::tb_addr(slot, 7))
    }
    /// 设 `delay`。
    pub fn set_tb_delay(&mut self, slot: u8, v: u8) {
        self.write_ram(Self::tb_addr(slot, 7), v);
    }
    /// 槽内 `roomRAMPtr`（两个字节合成 u16）。
    pub fn tb_ptr(&self, slot: u8) -> u16 {
        (self.read_ram(Self::tb_addr(slot, 9)) as u16) << 8
            | self.read_ram(Self::tb_addr(slot, 8)) as u16
    }
    /// 设 `roomRAMPtr`。
    pub fn set_tb_ptr(&mut self, slot: u8, v: u16) {
        self.write_ram(Self::tb_addr(slot, 8), v as u8);
        self.write_ram(Self::tb_addr(slot, 9), (v >> 8) as u8);
    }
    /// 槽内 `type`（碎块类型 —— 4×4 小泡/1×1/2×2 分组）。
    pub fn tb_type(&self, slot: u8) -> u8 {
        self.read_ram(Self::tb_addr(slot, 10))
    }
    /// 设 `type`。
    pub fn set_tb_type(&mut self, slot: u8, v: u8) {
        self.write_ram(Self::tb_addr(slot, 10), v);
    }
    /// 槽内 `animFrame`（= VRAM 字符串 id）。
    pub fn tb_anim_frame(&self, slot: u8) -> u8 {
        self.read_ram(Self::tb_addr(slot, 3))
    }
    /// 设 `animFrame`。
    pub fn set_tb_anim_frame(&mut self, slot: u8, v: u8) {
        self.write_ram(Self::tb_addr(slot, 3), v);
    }
    /// 槽内 `animIndex`。
    pub fn tb_anim_index(&self, slot: u8) -> u8 {
        self.read_ram(Self::tb_addr(slot, 6))
    }
    /// 设 `animIndex`。
    pub fn set_tb_anim_index(&mut self, slot: u8, v: u8) {
        self.write_ram(Self::tb_addr(slot, 6), v);
    }
    /// 槽内 `animDelay`。
    pub fn tb_anim_delay(&self, slot: u8) -> u8 {
        self.read_ram(Self::tb_addr(slot, 4))
    }
    /// 设 `animDelay`。
    pub fn set_tb_anim_delay(&mut self, slot: u8, v: u8) {
        self.write_ram(Self::tb_addr(slot, 4), v);
    }
    /// 槽内 `spare05`（原版把 `animIndex` 镜一份进来，`UpdateTileBlastAnim` 回滚用）。
    pub fn tb_spare05(&self, slot: u8) -> u8 {
        self.read_ram(Self::tb_addr(slot, 5))
    }
    /// 设 `spare05`。
    pub fn set_tb_spare05(&mut self, slot: u8, v: u8) {
        self.write_ram(Self::tb_addr(slot, 5), v);
    }

    // ---- Zebetites（`$0758`，5 槽 × 8 B）----
    //
    // `constants_ram.asm:76-83` 的 `.struct Zebetite`：status0 / roomRAMPtr1-2 /
    // qtyHits3 / healingDelay4 / isHit5 / unused6-7。Tourian 专用（`UpdateZebetite`）。

    fn zeb_addr(slot: u8, off: u16) -> u16 {
        debug_assert!(slot < 5, "Zebetites 槽号越界：{slot}");
        0x0758 + slot as u16 * 8 + off
    }
    /// 槽内 `status`（bit0=活、bit1=已死停更新、bit7=重绘失败标记）。
    pub fn zeb_status(&self, slot: u8) -> u8 {
        self.read_ram(Self::zeb_addr(slot, 0))
    }
    pub fn set_zeb_status(&mut self, slot: u8, v: u8) {
        self.write_ram(Self::zeb_addr(slot, 0), v);
    }
    /// 槽内 `roomRAMPtr`（左上角瓦的 RoomRAM 地址）。
    pub fn zeb_ptr(&self, slot: u8) -> u16 {
        (self.read_ram(Self::zeb_addr(slot, 2)) as u16) << 8
            | self.read_ram(Self::zeb_addr(slot, 1)) as u16
    }
    pub fn set_zeb_ptr(&mut self, slot: u8, v: u16) {
        self.write_ram(Self::zeb_addr(slot, 1), v as u8);
        self.write_ram(Self::zeb_addr(slot, 2), (v >> 8) as u8);
    }
    /// 槽内 `qtyHits`（8 击死）。
    pub fn zeb_qty_hits(&self, slot: u8) -> u8 {
        self.read_ram(Self::zeb_addr(slot, 3))
    }
    pub fn set_zeb_qty_hits(&mut self, slot: u8, v: u8) {
        self.write_ram(Self::zeb_addr(slot, 3), v);
    }
    /// 槽内 `healingDelay`（`$40`→0 自愈 1 击）。
    pub fn zeb_healing_delay(&self, slot: u8) -> u8 {
        self.read_ram(Self::zeb_addr(slot, 4))
    }
    pub fn set_zeb_healing_delay(&mut self, slot: u8, v: u8) {
        self.write_ram(Self::zeb_addr(slot, 4), v);
    }
    /// `UpdatingSamusProjectile`（`$0071`）——非 0 表示当前在跑萨姆斯弹件环
    /// （`IsBlastTile ; 07:E9BE` 的门；Tourian 弹撞瓦钩也用它）。
    pub fn updating_samus_projectile(&self) -> u8 {
        self.read_ram(crate::ram_map::UPDATING_SAMUS_PROJECTILE)
    }

    /// 槽内 `isHit`（本帧被导弹命中）。
    pub fn zeb_is_hit(&self, slot: u8) -> u8 {
        self.read_ram(Self::zeb_addr(slot, 5))
    }
    pub fn set_zeb_is_hit(&mut self, slot: u8, v: u8) {
        self.write_ram(Self::zeb_addr(slot, 5), v);
    }

    // ---- PipeBugHoles（`$0728`，4 槽 × 8 B）----
    //
    // `constants_ram.asm:56-63` 的 `.struct PipeBugHole`：status0（bit0-5=要生出的敌型
    // （生敌处掩 `&$3F`；原 asm 第三方注只写 bit0-3）、
    // bit7=strong、`$FF`=无洞）/ enemySlot1 / y2 / x3 / hi4 / unused5-7
    // （`:1177 PipeBugHoles instanceof PipeBugHole 4 startfrom 0`）。
    // 装载 `LoadPipeBugHole ; 07:EC57`、更新 `UpdatePipeBugHole ; 07:FAFF`。

    fn pbh_addr(slot: u8, off: u16) -> u16 {
        debug_assert!(slot < 4, "PipeBugHoles 槽号越界：{slot}");
        0x0728 + slot as u16 * 8 + off
    }
    /// 槽内 `status`（`$FF`=空槽；否则低半字节=要生出的敌型、bit7=strong）。
    pub fn pbh_status(&self, slot: u8) -> u8 {
        self.read_ram(Self::pbh_addr(slot, 0))
    }
    pub fn set_pbh_status(&mut self, slot: u8, v: u8) {
        self.write_ram(Self::pbh_addr(slot, 0), v);
    }
    /// 槽内 `enemySlot`（`$0400+x` 的对象槽偏移，`b0 & $F0`）。
    pub fn pbh_enemy_slot(&self, slot: u8) -> u8 {
        self.read_ram(Self::pbh_addr(slot, 1))
    }
    pub fn set_pbh_enemy_slot(&mut self, slot: u8, v: u8) {
        self.write_ram(Self::pbh_addr(slot, 1), v);
    }
    /// 槽内 `y`（`(b2 & $F0) | $08`）。
    pub fn pbh_y(&self, slot: u8) -> u8 {
        self.read_ram(Self::pbh_addr(slot, 2))
    }
    pub fn set_pbh_y(&mut self, slot: u8, v: u8) {
        self.write_ram(Self::pbh_addr(slot, 2), v);
    }
    /// 槽内 `x`（`b2 * 16`）。
    pub fn pbh_x(&self, slot: u8) -> u8 {
        self.read_ram(Self::pbh_addr(slot, 3))
    }
    pub fn set_pbh_x(&mut self, slot: u8, v: u8) {
        self.write_ram(Self::pbh_addr(slot, 3), v);
    }
    /// 槽内 `hi`（名表位，`GetNameTableAtScrollDir`）。
    pub fn pbh_hi(&self, slot: u8) -> u8 {
        self.read_ram(Self::pbh_addr(slot, 4))
    }
    pub fn set_pbh_hi(&mut self, slot: u8, v: u8) {
        self.write_ram(Self::pbh_addr(slot, 4), v);
    }

    /// 设 `InArea`（`$0074`，`$10`=Brinstar … `$14`=Ridley）。
    /// `MirrorCntrl($FA)` bit3 → `RenderState.mirror`（`constants_ram.asm:811-813`：
    /// 置=横镜（竖滚房 `$2000≡$2400`）、清=竖镜（横滚房 `$2000≡$2800`））。渲染层
    /// 与 ppu.hex 出口都按此选物理页——见 `render::nt_offset_with`。
    pub fn sync_mirroring(&mut self) {
        self.render.mirror = self.read_ram(crate::ram_map::MIRROR_CNTRL) & 0x08 != 0;
    }

    pub fn set_in_area(&mut self, v: u8) { self.write_ram(crate::ram_map::IN_AREA, v); }
    /// 设 `ScrollDir`（`$0049`）。
    pub fn set_scroll_dir(&mut self, v: u8) { self.write_ram(crate::ram_map::SCROLL_DIR, v); }
    /// 测试/诊断用：写 `ScrollX($00FD)`（`no-raw-addr` 门要求经访问器而非裸地址）。
    pub fn set_scroll_x(&mut self, v: u8) { self.write_ram(crate::ram_map::SCROLL_X, v); }

    /// `ScrollDir`（`$0049`，0=Up/1=Down/2=Left/3=Right）。
    /// `ScrollX($00FD)`——垂直/水平滚动的列像素（敌换名表判据要读它）。
    pub fn scroll_x(&self) -> u8 { self.read_ram(crate::ram_map::SCROLL_X) }

    pub fn scroll_dir(&self) -> u8 {
        self.read_ram(crate::ram_map::SCROLL_DIR)
    }

    /// `VRAMStructBufferIndex`（`$07A0`）——PPU 写缓冲游标（>= `$1F` 视为满）。
    pub fn vram_struct_index(&self) -> u8 {
        self.read_ram(crate::ram_map::VRAM_STRUCT_INDEX)
    }

    /// `Temp00/Temp01`（`$0000/$0001`）这对零页临时量——`DrawTileBlast` 用它们
    /// 把「RoomRAM 源地址 / 名表目标高字节」传给 `write_vram_string`。
    pub fn set_ptr00_lo_hi(&mut self, lo: u8, hi: u8) {
        self.write_ram(0x0000, lo);
        self.write_ram(0x0001, hi);
    }

    /// 设 `SamusGear`（测试/装配面）。
    pub fn set_samus_gear(&mut self, v: u8) {
        self.write_wram(crate::ram_map::SAMUS_GEAR, v);
    }

    /// 设 `FrameCount`（`$002D`）——测试里摆帧相序用。
    pub fn set_frame_count(&mut self, v: u8) {
        self.write_ram(crate::ram_map::FRAME_COUNT, v);
    }

    /// 设 `Joy1Change`（`$0012`）——测试里模拟「本帧刚按下」。
    pub fn set_joy1_change(&mut self, v: u8) {
        self.write_ram(crate::ram_map::JOY1_CHANGE, v);
    }

    /// `SamusGear`（`$6878`，WRAM 装备位图）——`CheckBombLaunch ; 07:D150` 读 bit1
    /// （有没有炸弹）。它不在 16 字节 Samus 结构内，按同族收口在本豁免文件。
    pub fn samus_gear(&self) -> u8 {
        self.read_wram(crate::ram_map::SAMUS_GEAR)
    }

    /// `Joy1Change`（`$0012`）——「本帧刚按下」的按键位图（`CheckBombLaunch` 读它）。
    /// 与 [`Self::frame_count`] 同族收口（非任何 typed 域的字段）。
    pub fn joy1_change(&self) -> u8 {
        self.read_ram(crate::ram_map::JOY1_CHANGE)
    }

    /// `Joy1Retrig`（`$0016`）——连发重触发位图（与 `Joy1Change` 同判 FIRE）。
    pub fn joy1_retrig(&self) -> u8 {
        self.read_ram(crate::title::state::JOY1_RETRIG)
    }

    /// `KraidLintCounter`（`$007E`，`constants_ram.asm:649`）——Kraid 的 Lint 发射周期
    /// 计数器（`KraidTryToLaunchLint ; 04:9BE0` 每 2 帧减 1）。asm 注释：Ridley 也误用它。
    ///
    /// 【为什么收口在本文件】与 [`Self::frame_count`] 同族：`$7E` 既不是敌槽也不是
    /// Samus 字段，`enemy/` 下的新 AI 模块按 `raw-surface` 纪律不许新接入裸字节面。
    pub fn kraid_lint_counter(&self) -> u8 {
        self.read_ram(crate::ram_map::KRAID_LINT_COUNTER)
    }

    pub fn set_kraid_lint_counter(&mut self, v: u8) {
        self.write_ram(crate::ram_map::KRAID_LINT_COUNTER, v);
    }

    /// `KraidNailCounter`（`$007F`，`constants_ram.asm:650`）——Nail 发射周期计数器
    /// （`KraidTryToLaunchNail ; 04:9C19`）。见 [`Self::kraid_lint_counter`] 的收口理由。
    pub fn kraid_nail_counter(&self) -> u8 {
        self.read_ram(crate::ram_map::KRAID_NAIL_COUNTER)
    }

    pub fn set_kraid_nail_counter(&mut self, v: u8) {
        self.write_ram(crate::ram_map::KRAID_NAIL_COUNTER, v);
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
        src.samus()
            .set_status(crate::samus::state::SamusStatus::Roll);
        src.enemy(3)
            .set_status(crate::enemy::state::EnemyStatus::Active);
        let (ram, wram, regs) = src.capture_mss();
        let mut restored = crate::state::Manager::restore_mss(&ram, &wram, regs);
        assert_eq!(restored.ram()[0x0300], 0, "storage 不得保留 typed 副本");
        assert_eq!(restored.read_ram(0x0300), 3, "读径仍走 typed");
        assert_eq!(
            restored.enemy(3).status(),
            crate::enemy::state::EnemyStatus::Active
        );
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

        for a in TypedRestoreManifest::ram_addrs() {
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
        let mut manifest: Vec<u16> = TypedRestoreManifest::ram_addrs().collect();
        bypass.sort_unstable();
        manifest.sort_unstable();
        assert_eq!(
            bypass, manifest,
            "RAM 面：读径绕过 storage 的地址集合 != typed 清单"
        );

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
            .flat_map(|slot| {
                (0..EXTRA_STRIDE).map(move |off| EXTRA_BASE + slot * EXTRA_STRIDE + off)
            })
            .collect();

        let manifest: Vec<u16> = TypedRestoreManifest::wram_addrs().collect();
        let overlap: Vec<u16> = face
            .iter()
            .copied()
            .filter(|a| manifest.contains(a))
            .collect();
        assert!(
            overlap.is_empty(),
            "爆炸额外面与敌槽 typed 清单重叠：{overlap:?}"
        );

        let ens_end =
            crate::enemy::state::ENS_EXTRA_BASE + crate::enemy::state::SLOT_COUNT as u16 * 16 - 1;
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
            assert_eq!(
                mem.wram()[i],
                wram[i],
                "{a:#06X}：爆炸额外面是未迁族，必须留在字节面"
            );
            assert_eq!(
                mem.read_wram(a),
                wram[i],
                "{a:#06X}：爆炸额外面读径不得绕过 storage"
            );
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
        let regs = Registers {
            a: 7,
            x: 9,
            y: 11,
            sp: 0xFD,
            pc: 0xC0DE,
            p: 0x31,
        };
        let mem = Manager::restore_mss(&ram, &wram, regs);

        let typed_ram: Vec<u16> = TypedRestoreManifest::ram_addrs().collect();
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
            assert_eq!(
                mem.wram()[i],
                wram[i],
                "{a:#06X}：WRAM 未迁族必须留在字节面"
            );
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
impl crate::state::Manager {
    /// `MaxMissiles`（`$687A`）——导弹上限。消费端 `UpdateEnemy_Pickup ; 07:F4C3`
    /// （`cmp MaxMissiles / bcc @dontCapMissiles`）需要它做封顶。
    pub fn max_missiles(&self) -> u8 {
        self.read_ram(crate::ram_map::MAX_MISSILES)
    }
}
