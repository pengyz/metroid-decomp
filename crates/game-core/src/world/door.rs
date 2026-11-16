//! 门域：萨姆斯穿门位移、离屏门清除、门打开时清屏内敌人。
//!
//! 从 `samus/handler.rs` 迁出（原位置是「萨姆斯处理链顺手写了门」的产物）。
//! asm 归属：`SamusInDoor`/`MoveOutDoor ; 07:D3F7`、`Doors_RemoveIfOffScreen ; 07:ED65`、
//! `Door_DeleteOffscreenEnemies ; 07:D48C`——三者读写的是**对象槽 8-11**
//! （`$0380-$03AF`），与萨姆斯实体互不相干。

use crate::frame::Manager;

/// `DoorRoomRAMPtr`：门的四列房间指针数组（`$5C/$5D`、`$5E/$5F`、`$60/$61`、`$62/$63`），
/// **lo/hi 交织**、索引步长 2（asm `sta DoorRoomRAMPtr,x` / `sta DoorRoomRAMPtr+1,x`，x∈{0,2,4,6}）。
pub const DOOR_ROOM_PTR_LO: u16 = 0x005C;
pub const DOOR_ROOM_PTR_HI: u16 = 0x005D;

/// 写第 `idx`（0..4）个门的房间指针。
pub(crate) fn set_door_room_ptr(mem: &mut Manager, idx: u8, ptr: u16) {
    debug_assert!(idx < 4, "DoorRoomRAMPtr 只有 4 列");
    let o = (idx as u16) * 2;
    mem.write_ram(DOOR_ROOM_PTR_LO + o, ptr as u8);
    mem.write_ram(DOOR_ROOM_PTR_HI + o, (ptr >> 8) as u8);
}

/// 读第 `idx`（0..4）个门的房间指针（`DoorRoomRAMPtr`）。
pub(crate) fn door_room_ptr(mem: &Manager, idx: u8) -> u16 {
    debug_assert!(idx < 4, "DoorRoomRAMPtr 只有 4 列");
    let o = (idx as u16) * 2;
    u16::from_le_bytes([
        mem.read_ram(DOOR_ROOM_PTR_LO + o),
        mem.read_ram(DOOR_ROOM_PTR_HI + o),
    ])
}

/// 管道虫洞表（`PipeBugHoles`）。
pub(crate) const PIPE_BUG_HOLES: u16 = 0x0728;

/// `MoveOutDoor ; 07:D3F7`——`lda SamusDoorDir/beq Lx054`：
/// 门通向左（dir≠0）⇒ `ldy Samus.x / bne / jsr ToggleSamusHi` 后 `dec Samus.x`；
/// 门通向右（dir=0）⇒ `inc Samus.x / bne / jsr ToggleSamusHi`（进位到 0 才翻第 9 位）。
pub(crate) fn move_out_door(mem: &mut Manager) {
    if mem.read_ram(crate::ram_map::SAMUS_DOOR_DIR) != 0 {
        if mem.samus().x() == 0 {
            crate::samus::physics::toggle_samus_hi(mem); // Lx053 前件
        }
        let x = mem.samus().x().wrapping_sub(1);
        mem.samus().set_x(x);
    } else {
        let x = mem.samus().x().wrapping_add(1);
        mem.samus().set_x(x);
        if x == 0 {
            crate::samus::physics::toggle_samus_hi(mem); // Lx054 尾件
        }
    }
}

/// `Door_DeleteOffscreenEnemies ; 07:D48C`（ROM 字节 `A2 60 38 20 B4 D4 8A E9 20 AA
/// 10 F7` 逐字节自证）：
/// ① 敌环 `ldx #$60`/`sbc #$20`/`bpl` ⇒ 偏移 **$60/$40/$20/$00**（asm 自带 BUG 注：
///    应为 `#$50`/`#$10`——当前形会碰到槽 6（敌弹区）并漏掉槽 1/3/5；D4 保真保留）。
///    每槽：`lda Ens.data05,x / and #$02 / bne 跳` ⇒ 不可见（bit1 清）时
///    `EnsExtra.status,x ← 0`（A 被 and 清零）。
/// ② 管道虫洞环 `ldx #$18`/`sbc #$08`/`bne`（ROM `A2 18 20 A8 D4 8A 38 E9 08 AA
///    D0 F6`）⇒ 偏移 **$18/$10/$08**，**槽 0 不被访问**（`bne` 在 0 处退出——同样是
///    原版行为，非笔误）。y=GetNameTableAtScrollDir，`cmp PipeBugHoles.hi,x / bne`
///    ⇒ hi 与当前名表**相等**才 `status ← $FF`（删洞）。
pub(crate) fn door_delete_offscreen_enemies(mem: &mut Manager) {
    let mut x: i16 = 0x60;
    while x >= 0 {
        let slot = (x as u16 / 16) as u8;
        if mem.enemy(slot).data05() & 0x02 == 0 {
            mem.enemy(slot).set_status(crate::enemy::state::EnemyStatus::NoEnemy);
        }
        x -= 0x20;
    }
    let y = (mem.read_ram(crate::ram_map::PPUCTRL_ZP) ^ mem.read_ram(crate::ram_map::SCROLL_DIR))
        & 0x01;
    let mut x: u16 = 0x18;
    loop {
        if mem.read_ram(PIPE_BUG_HOLES + x + 4) == y {
            mem.write_ram(PIPE_BUG_HOLES + x, 0xFF);
        }
        x = x.wrapping_sub(8);
        if x == 0 {
            break; // `bne`——到 0 即退出（槽 0 不处理）
        }
    }
}

/// `Doors_RemoveIfOffScreen ; 07:ED65`（ROM `A2 B0 BD 00 03 F0 08 BD 0B 03 D0 03
/// 9D 00 03 20 F4 F1`）：x 自 $B0 起每轮 −$10（`Xminus16`+`bmi` 环）⇒ 4 门
/// $B0/$A0/$90/$80；`status≠0` 且 `onScreen==0` ⇒ `status ← 0`（A=0 直接回存）。
pub(crate) fn doors_remove_if_offscreen(mem: &mut Manager) {
    // `ldx #Doors-Objects+_sizeof_Doors-_sizeof_Doors.0` = $B0；循环尾
    // `jsr Xminus16 / bmi @loop` —— **bmi 是回跳条件**：减后仍为负才继续（i8 符号位）。
    // 序列 $B0→$A0→$90→$80→$70（$70 为正 ⇒ 退出）⇒ 恰 4 槽，$03B0..$0380。
    let mut x: i16 = 0xB0;
    loop {
        let slot = (x / 16) as u8;
        if mem.object(slot).status() != 0 && mem.object(slot).on_screen() == 0 {
            mem.object(slot).set_status(0);
        }
        x -= 0x10;
        if (x as i8) >= 0 {
            break;
        }
    }
}
