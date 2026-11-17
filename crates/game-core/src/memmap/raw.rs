//! **裸字节读写面（兼容辖）**——`read_ram` / `write_ram` / `read_wram` / `write_wram`。
//!
//! 【定位】这是**唯一**允许按 NES 地址读写字节的地方；它是 typed 域迁完之前的过渡面，
//! 也是将来「业务层看不到地址」那天要删除的入口点。业务代码请优先用语义视图：
//! `mem.samus()` / `mem.enemy(slot)` / `mem.explosion(slot)` / `mem.object(slot)` /
//! `mem.camera()` / `mem.room()` / `mem.ens_face(x)`。
//!
//! 【为什么还不删】仍有约 630 处落在**尚未 typed 化**的地址族（asm 临时量 `$00-$0F`、
//! OAM 影子 `$0200-$02FF`、声音旗标 `$0600-$06FF`、`PageIndex/RoomNumber/SpritePagePos`
//! 等伪寄存器）。删接口=删这些族的唯一入口，会变成大规模盲改。
//!
//! 【怎么收】不靠一次性改写，靠三道渐进闸（都不要求立刻改代码）：
//! 1. `no-raw-addr`：**受监控族**（Samus 23 字节/敌槽/爆炸池/对象池）的裸地址只减不增；
//! 2. `raw-surface`：调用本模块的**文件清单**只减不增（新文件不许接裸面）；
//! 3. 逐族视图化（每族落地即从账本里消一行），归零后再把本模块降为 `pub(crate)` → 私有。
//!
//! 【纪律】本模块内不允许再长出新的便利方法（`read_u16` 之类）——每加一个，
//! 将来删除面就多一处语义要迁移。

use crate::state::Manager;

impl Manager {
    /// RAM 读（2KB 镜像归一）。
    pub fn read_ram(&self, addr: u16) -> u8 {
        // 【typed 域辖】Samus 的已 typed 化字段真相在 `Manager.samus`（`samus::state::SamusState`），
        // 字节数组不再持有它们；地址面保持不变（兼容辖），逐域迁完后此分支即"读取口"唯一实现。
        // 【read_ram 映射修（2026-11-06 全链根——$6877 读 255 垃圾破案）】$6000-$7FFF
        // =WRAM 分派（asm constants：EnergyTankCount $6877=$6000+0x877）——旧形
        // `& 0x7FF` 镜像归一把 WRAM 读混入 ram[0x077] 垃圾。同族修（分派非点修）。
        // **WRAM 必须先判**：否则 `0x6B00 & 0x7FF == 0x0300` 会误落 typed 面（本轮实证）。
        if (0x6000..0x8000).contains(&addr) {
            // 单一分派：WRAM 面只经 `read_wram`（敌槽 typed 域在同一入口判）。
            // 若这里直读 `storage`，`read_ram` 与 `read_wram` 会对同一格给两个答案。
            return self.read_wram(addr);
        }
        // 【镜像归一先于 typed 分派（2026-11-17）】`read_ram(0x0B00)` 与 `read_ram(0x0300)`
        // 是同一格（2KB 镜像），若先查 typed 会漏判 ⇒ 别名绕过 typed 面。
        let a = addr & 0x7FF;
        if let Some(v) = self.samus.get_byte(a) {
            return v;
        }
        // `HealthChange`（`$006E/$006F`）：typed 真源在 `Manager.health_change`。
        if let Some(v) = self.health_change.get_byte(a) {
            return v;
        }
        // 声引擎 init 旗（`$0683/$0684`）：typed 真源在 `Manager.sound_init`。
        if let Some(v) = self.sound_init.get_byte(a) {
            return v;
        }
        // 对象全局伪寄存器（`$4B/$5A/$5B`）：typed 真源在 `Manager.object_globals`。
        if let Some(v) = self.object_globals.get_byte(a) {
            return v;
        }
        // OAM 页（`$0200-$02FF`）：typed 真源在 `Manager.oam`。
        if let Some(v) = self.oam.get_byte(a) {
            return v;
        }
        self.ram()[a as usize]
    }

    /// RAM 写（2KB 镜像归一）。
    pub fn write_ram(&mut self, addr: u16, v: u8) {
        // 【typed 域辖】同 read_ram：已 typed 化地址落 typed 状态，不落字节数组。
        // 【read_ram 映射修同族（2026-11-06）】WRAM 分派（写径同读径）——必须先于 typed，
        // 理由同读径（`0x6B00` 的镜像别名等于 `$0300`）。
        if (0x6000..0x8000).contains(&addr) {
            // 单一分派：同 read_ram——写径也必须经过 `write_wram` 的 typed 判。
            self.write_wram(addr, v);
            return;
        }
        let a = addr & 0x7FF;
        if self.samus.set_byte(a, v) {
            return;
        }
        // `HealthChange`（`$006E/$006F`）：写径同读径，落 typed 真相。
        if self.health_change.set_byte(a, v) {
            return;
        }
        // 声引擎 init 旗（`$0683/$0684`）：写径同读径，落 typed 真相。
        if self.sound_init.set_byte(a, v) {
            return;
        }
        // 对象全局伪寄存器（`$4B/$5A/$5B`）：写径同读径，落 typed 真相。
        if self.object_globals.set_byte(a, v) {
            return;
        }
        // OAM 页（`$0200-$02FF`）：写径同读径，落 typed 真相。
        if self.oam.set_byte(a, v) {
            return;
        }
        // 【goal-轮80 Samus radiusY 写钩】0x301 写点（碰撞判径源）。
        self.ram_mut()[(addr as usize) & 0x7FF] = v;
    }

    /// WRAM 读（$6000-$7FFF 字节偏移面）。
    pub fn read_wram(&self, addr: u16) -> u8 {
        // 【typed 域辖】敌槽 `EnsExtra` 前 7 字节真相在 `Manager.enemies`。
        if let Some((slot, off)) = crate::enemy::state::split_ens_extra(addr) {
            if let Some(v) = self.enemies[slot].get_byte(off) {
                return v;
            }
        }
        self.wram()[(addr as usize - 0x6000) & 0x1FFF]
    }

    /// WRAM 写。
    pub fn write_wram(&mut self, addr: u16, v: u8) {
        if let Some((slot, off)) = crate::enemy::state::split_ens_extra(addr) {
            if self.enemies[slot].set_byte(off, v) {
                return;
            }
        }
        self.wram_mut()[(addr as usize - 0x6000) & 0x1FFF] = v;
    }

}
