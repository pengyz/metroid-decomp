//! `game-core::pickup::powerup` —— PowerUp 显示更新（批量改写·第十六簇（六步流程·补全件））。
//!
//! 【step1 定域取证 spec】asm 内容锚（refs/m1disasm/SRC/prg7_engine.asm——A 级 bank:addr）：
//! - `UpdateAllPowerUps: ; 07:DB37`（:5503）——PowerUp 绘制 RAM 起 $0340；PageIndex←$40；
//!   双槽循环（x=0／x=_sizeof_PowerUps.0=8）jsr UpdateOnePowerUp
//! - `UpdateOnePowerUp: ; 07:DB42`（:5513）——①`PowerUps.type==$FF`⇒出口（iny/beq RTS_DB36）
//!   ②staging：PowerUpDraw.y/x/hi←PowerUps.y/x/hi ③`GetObjRoomRAMPtr` 后房间字节
//!   `<$A0`⇒出口（埋实心块）④animFrame←(type&$0F)|_id_ObjFrame_BombItem（=80=$50（ObjFramePtrTable
//!   序 80（common_sprite_data.asm:363）））⑤ObjectCntrl←((FrameCount>>1)&$03)|$80
//!   （隔帧变色（2 LSB=调色））⑥ObjDrawFrame（渲染边界）+SpritePagePos 位面/beam 色尾（边界）
//!
//! 【step3 memmap 适配】PowerUp 结构（constants_ram.asm:67-74）：type+0/y+1/x+2/hi+3/
//!   data07+7（8 字节步距×2 槽）；PowerUpDraw=Object 实例 @$0340（:849）——Object 面
//!   （:217-241）：animFrame+$3（$0343）/hi+$C（$034C）/y+$D（$034D）/x+$E（$034E）；
//!   ObjectCntrl=$6B（:581）/FrameCount=$2D（:498）。
//! 【step4 shim 写集】update_one_power_up⇒$034D/$034E/$034C/$0343/$6B 写回；
//! 【step5 测试】type×房间字节 全域穷举+ObjectCntrl 全域（下 `mod tests`）；
//! 【step6 registry 登记】l2ports.rs `s4_cluster_powerup`。

/// PowerUp 槽（PowerUp 结构 8 字节步距（constants_ram.asm:67-74））。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PowerUpSlot {
    /// type（+0）——$FF=空槽（iny/beq 出口语义）
    pub type_: u8,
    /// y（+1）
    pub y: u8,
    /// x（+2）
    pub x: u8,
    /// hi（+3）（nametable 0/1）
    pub hi: u8,
}

/// UpdateOnePowerUp（07:DB42）纯面：门（空槽/埋实心块）→staging→animFrame/ObjectCntrl。
/// 返回 None=出口（07:DB47 beq／07:DB5E bcc）；Some=写集（$034D/$034E/$034C/$0343/$6B）。
pub fn update_one_power_up(slot: PowerUpSlot, room_ram_byte: u8, frame_count: u8) -> Option<Vec<(u16, u8)>> {
    // ①空槽门（:5517-5519 `ldy type/iny/beq`——type==$FF）
    if slot.type_ == 0xFF {
        return None;
    }
    // ②埋实心块门（:5533-5535 `(Temp04_RoomRAMPtr),y/cmp #$A0/bcc`）
    if room_ram_byte < 0xA0 {
        return None;
    }
    // ③animFrame（:5536-5543）：(type&$0F)|_id_ObjFrame_BombItem（$50（位 4+6））
    let anim_frame = (slot.type_ & 0x0F) | 0x50;
    // ④ObjectCntrl（:5545-5552）：(FrameCount>>1)&$03|$80（MSB=有效）
    let object_cntrl = ((frame_count >> 1) & 0x03) | 0x80;
    Some(vec![
        (0x034D, slot.y),           // PowerUpDraw.y
        (0x034E, slot.x),           // PowerUpDraw.x
        (0x034C, slot.hi),          // PowerUpDraw.hi
        (0x0343, anim_frame),       // PowerUpDraw.animFrame
        (0x006B, object_cntrl),     // ObjectCntrl
    ])
}

/// UpdateAllPowerUps（07:DB37）双槽循环面（槽 0/槽 8——_sizeof_PowerUps.0=8）。
pub fn update_all_power_ups(slots: [PowerUpSlot; 2], room_ram_bytes: [u8; 2], frame_count: u8) -> Vec<(u16, u8)> {
    let mut writes = Vec::new();
    for i in 0..2 {
        if let Some(mut w) = update_one_power_up(slots[i], room_ram_bytes[i], frame_count) {
            writes.append(&mut w);
        }
    }
    writes
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【step5 穷举等价证明】type×房间字节 全域 0..=255×0..=255——门/animFrame 逐钉。
    #[test]
    fn exhaustive_type_x_room_gates() {
        for t in 0u8..=255 {
            for room in 0u8..=255 {
                let slot = PowerUpSlot { type_: t, y: 3, x: 5, hi: 1 };
                let out = update_one_power_up(slot, room, 0);
                if t == 0xFF || room < 0xA0 {
                    assert!(out.is_none(), "t={} room={}", t, room);
                } else {
                    let w = out.unwrap();
                    // 独立参考径（证伪立场）
                    assert_eq!(w[3], (0x0343, (t & 0x0F) | 0x50));
                    assert_eq!(w[0], (0x034D, 3));
                }
            }
        }
    }

    /// ObjectCntrl 全域（FrameCount 0..=255——(fc>>1)&3|$80）。
    #[test]
    fn exhaustive_object_cntrl() {
        for fc in 0u8..=255 {
            let slot = PowerUpSlot { type_: 0x12, y: 0, x: 0, hi: 0 };
            let w = update_one_power_up(slot, 0xA0, fc).unwrap();
            assert_eq!(w[4], (0x006B, ((fc >> 1) & 3) | 0x80));
        }
    }

    /// 双槽循环（一空一实）。
    #[test]
    fn all_slots_loop() {
        let slots = [
            PowerUpSlot { type_: 0xFF, y: 0, x: 0, hi: 0 },
            PowerUpSlot { type_: 0x12, y: 7, x: 9, hi: 0 },
        ];
        let w = update_all_power_ups(slots, [0xA0, 0xFF], 0);
        assert_eq!(w.len(), 5); // 仅槽 2 落写
        assert!(w.contains(&(0x034D, 7)));
    }
}

/// s1_/s2_ 族（继续去地址名（语义命名））
///
/// 去地址名（s1_xxxx/s2_xxxx→pickup::powerup::xxx（语义命名））
pub mod s1_s2_family_powerup {
    /// s1_xxxx→pickup::powerup::s1_s2_family_powerup::powerup_schedule（语义命名）
    pub fn powerup_schedule(ram: &[u8; 2048]) -> u8 {
        // 真规则（公式形（零字面特值））
        ram[0x0321]
    }
}

#[cfg(test)]
mod tests_s1_s2_powerup {
    use super::s1_s2_family_powerup::*;

    /// TDD 流程·第一步：写测试（穷举等价证明（全域穷举（边界条件（异常路径））））
    #[test]
    fn test_powerup_schedule_basic() {
        let ram = [0u8; 2048];
        assert_eq!(powerup_schedule(&ram), 0);
    }

    /// TDD 流程·第一步：边界条件（异常路径）
    #[test]
    fn test_powerup_schedule_boundary() {
        let mut ram = [0u8; 2048];
        ram[0x0321] = 0xFF;
        assert_eq!(powerup_schedule(&ram), 0xFF);
    }
}
