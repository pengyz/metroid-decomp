//! Statue 门控族（T45-1——task-37 对表核心；全仪式关键件）。
//!
//! **asm 内容锚+结构互核注**（施工令②精化版）：
//! · `UpdateAllStatues: ; 07:D9D4`（状态机 0→1→2→3：lowered 瓦面→raised 瓦面→
//!   UpdateStatue×2→Bridge）+ `UpdateAllStatues_Bridge: ; 07:DADA`（门判①②③+
//!   **8×TileBlasts 生成**=Tourian 桥瓦面）。
//! · `UpdateStatue_Raise: ; 07:DA3D`（raise 状态机：1px/call 到 Y&$0F==0（16 界）→
//!   RaiseState bit7=Raised+**KraidStatueStatus bit7=up**）。
//! · **task-37 对表定谳**（docs/findings/t45-1-statue-gate-verification.md）：
//!   门判可与瓦爆域解耦（①②③=纯 RAM 位面）；可达性判据=双像 bit7 可置位∧桥旗 $54。
//! · **开门链对表**（docs/findings/door-open-chain.md §4）：isHit=4 写同源（雕像命中
//!   与门命中同一碰撞族——同例钩子）；房间 RAM 指针位 3 分流（Kraid=$0/$1/Ridley=$C/$D）。
//!
//! 地址账（考证自证）：KraidStatueStatus=$687B（bit0=闪/bit7=up）/RidleyStatueStatus=
//! $687C/StatuesBridgeIsSpawned=$54/Statue 槽=$0360（RaiseState $0364/$0365/IsHit
//! $0366/$0367/Y $036F/$0370）/TileBlasts=$0500（13×16B：routine+0/delay+7/type+10）。

use super::FrameMemory;

/// 址常量（考证自证——见模块注）。
const KRAID_STATUS: u16 = 0x687B;
const RIDLEY_STATUS: u16 = 0x687C;
const BRIDGE_SPAWNED: u16 = 0x0054;
const DOOR_ENTRY_STATUS: u16 = 0x0056;
const STATUE_SLOT: u16 = 0x0360;
const KRAID_RAISE: u16 = 0x0364;
const RIDLEY_RAISE: u16 = 0x0365;
const KRAID_Y: u16 = 0x036F;
const RIDLEY_Y: u16 = 0x0370;
const TILE_BLASTS: u16 = 0x0500;
const SLOT: u16 = 16;

/// UpdateAllStatues 单元（状态机+门判+桥生成）。
pub fn update_all_statues(mem: &mut FrameMemory) {
    let status = mem.read_ram(STATUE_SLOT);
    if status == 0 {
        return;
    }
    if status == 1 {
        // lowered 瓦面（双像 y=0/1）→UpdateStatueBGTiles（瓦写面——瓦爆域对表）
        crate::unported!("bank07:DAB0"); // UpdateStatueBGTiles
    }
    if status == 2 {
        if mem.read_ram(KRAID_STATUS) & 0x80 != 0 {
            crate::unported!("bank07:DAB0"); // Kraid raised 瓦面
        }
        if mem.read_ram(RIDLEY_STATUS) & 0x80 != 0 {
            crate::unported!("bank07:DAB0"); // Ridley raised 瓦面
        }
    }
    // UpdateStatue ×2（Raise+StartRaising+anim——单元 2）
    update_statue(mem, 0);
    update_statue(mem, 1);
    // Bridge
    update_statues_bridge(mem);
}

/// UpdateStatue_Raise raise 状态机（单元 2——1px/call 到 16 界）。
/// idx: 0=Kraid/1=Ridley。返回是否推进。
pub fn update_statue_raise(mem: &mut FrameMemory, idx: u8) {
    let (raise, y_addr, status) = if idx == 0 {
        (KRAID_RAISE, KRAID_Y, KRAID_STATUS)
    } else {
        (RIDLEY_RAISE, RIDLEY_Y, RIDLEY_STATUS)
    };
    let st = mem.read_ram(raise);
    if st & 0x80 != 0 {
        return; // 已升
    }
    mem.write_ram(raise, 0x01); // not raised（asm `lda #$01/sta`）
    let y = mem.read_ram(y_addr);
    if y & 0x0F == 0 {
        return; // 不动
    }
    mem.write_ram(raise, mem.read_ram(raise).wrapping_add(1)); // raising（asm `inc` 1→2）
    let y = mem.read_ram(y_addr).wrapping_sub(1); // 1px/call
    mem.write_ram(y_addr, y);
    if y & 0x0F != 0 {
        return; // 未完
    }
    // 完成：RaiseState|=0x80+Status|=0x80
    let done = mem.read_ram(raise) | 0x80;
    mem.write_ram(raise, done);
    mem.write_ram(status, done);
}

/// UpdateStatue（单元 2 壳——Raise+StartRaising+anim 三面）。
fn update_statue(mem: &mut FrameMemory, idx: u8) {
    update_statue_raise(mem, idx);
    crate::unported!("bank07:DA7C"); // UpdateStatue_StartRaising（触发时序——随件）
}

/// UpdateAllStatues_Bridge（**task-37 对表核心**——门判①②③+8×TileBlasts 生成）。
pub fn update_statues_bridge(mem: &mut FrameMemory) {
    if mem.read_ram(BRIDGE_SPAWNED) & 0x80 != 0 {
        return; // ① 已生成
    }
    if mem.read_ram(DOOR_ENTRY_STATUS) != 0 {
        return; // ② 在门内
    }
    let both = mem.read_ram(KRAID_STATUS) & mem.read_ram(RIDLEY_STATUS);
    if both & 0x80 == 0 {
        return; // ③ 双 AND 门（双 Boss 击败=双像 bit7）
    }
    mem.write_ram(BRIDGE_SPAWNED, 0x80); // 置位
    // 8×TileBlasts 生成（routine=3 待重生/delay=y*2（y=8..1）/type=4 generic shot）
    for j in 0..8u16 {
        let base = TILE_BLASTS + j * SLOT;
        mem.write_ram(base, 0x03);
        mem.write_ram(base + 7, ((8 - j) as u8).wrapping_mul(2));
        mem.write_ram(base + 10, 0x04);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【全仪式·桥门判】双 AND 门+8 瓦爆生成（task-37 核心面）。
    #[test]
    fn bridge_gate_and_tile_blasts() {
        let mut m = FrameMemory::default();
        m.write_ram(STATUE_SLOT, 2);
        // 双像未全 up→门不开
        m.write_ram(KRAID_STATUS, 0x80);
        update_statues_bridge(&mut m);
        assert_eq!(m.read_ram(BRIDGE_SPAWNED), 0);
        // 双 up→桥生成
        m.write_ram(RIDLEY_STATUS, 0x80);
        update_statues_bridge(&mut m);
        assert_eq!(m.read_ram(BRIDGE_SPAWNED), 0x80);
        // 8 瓦爆：slot0 delay=16/type=4/routine=3
        assert_eq!(m.read_ram(TILE_BLASTS), 0x03);
        assert_eq!(m.read_ram(TILE_BLASTS + 7), 16);
        assert_eq!(m.read_ram(TILE_BLASTS + 10), 0x04);
        assert_eq!(m.read_ram(TILE_BLASTS + 7 * SLOT + 7), 2); // slot7 delay=2
    }

    /// 【全仪式·raise 状态机】1px/call 到 16 界→bit7=up 双址置。
    #[test]
    fn statue_raise_state_machine() {
        let mut m = FrameMemory::default();
        m.write_ram(KRAID_Y, 0x1F); // 低半字节 F=15px 待升（asm 语义：Y&$0F=剩余偏移）
        for _ in 0..15 {
            update_statue_raise(&mut m, 0);
        }
        assert_eq!(m.read_ram(KRAID_RAISE) & 0x80, 0x80); // raised
        assert_eq!(m.read_ram(KRAID_STATUS) & 0x80, 0x80); // bit7=up
        assert_eq!(m.read_ram(KRAID_Y), 0x10); // 低半字节 0=对齐完成
    }
}
