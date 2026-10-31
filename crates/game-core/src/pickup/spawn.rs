//! `game-core::pickup::spawn` —— 特项扫描/PowerUp 装载/拾取授予全链。
//!
//! asm 内容锚（refs/m1disasm/SRC/prg7_engine.asm——A 级 bank:addr）：
//! - `ScanForItems: ; 07:ED98`（:9255）——**SetupRoom（EA2B）首件**（:8588）：当前图格
//!   （MapPosX/MapPosY）特项扫描→`ChooseSpawningRoutine: ; 07:EDD6`（it_Type 低半字节
//!   JumpEngine 分派：2=PowerUp/3=Mellow/4=Elevator/…）。
//! - `SpawnPowerUp: ; 07:EDFE`（:9349）——槽分配（PowerUps.type==$FF 空——2 槽×8B
//!   @$0748）→PrepareItemID（EE3D——Temp09_ItemType/Temp06/07 填）→**CheckForItem
//!   （EE4A——已收集⇒跳过）**→位置解码（y=(pos&$F0)|8/x=((pos<<4)&$F0)|8/hi=名表位）。
//! - `UpdateAllPowerUps: ; 07:DB37`（:5503）+`UpdateOnePowerUp: ; 07:DB42`（:5513）——
//!   埋实心块门（房间字节<$A0）→staging→ObjDrawFrame 显示→**AreObjectsTouching
//!   （DC7F——X 槽 0（Samus）/Y 槽 $40（物））**→拾取授予（MakeBitMask（DB2F：1<<type）
//!   →SamusGear（$6878）OR；弹药/能量罐（DBF7/DC00）；PowerUpDelayFlag（$0109）←$FF；
//!   槽清；SelectSamusPalette）。
//! - `CreateItemID: ; 07:DC67`（:5788）/`CheckForItem: ; 07:EE4A`（:9423）——唯一物 ID
//!   （(MapX&7)<<5|MapY / (type<<2)|(MapX>>3)）×UniqueItemHistory（$6887 起 2B/件）
//!   历史（NumberOfUniqueItems=$6886）。
//!
//! 数据面：game_data::area::items（tools/bake_items.py ROM 烘焙——五区 SpecItmsTbl）。

use crate::state::FrameMemory;

/// PowerUps 基（2 槽×8B（constants_ram:1154-1158 `$0748-$0757`））。
const POWERUPS_BASE: u16 = 0x0748;
/// PowerUpDraw=Object 实例 @$0340（constants:849——PageIndex=$40 面）。
const POWERUP_DRAW_BASE: u16 = 0x0340;
/// NumberOfUniqueItems=$6886（constants:1228）/UniqueItemHistory=$6887 起（2B/件）。
const NUM_UNIQUE_ITEMS: u16 = 0x6886;
const UNIQUE_ITEM_HISTORY: u16 = 0x6887;
/// PowerUpDelayFlag=$0109（constants:810——拾取音乐/延迟旗）。
const POWER_UP_DELAY_FLAG: u16 = 0x0109;

/// 区序→特项表（InArea&$0F：0=Brinstar/1=Norfair/2=Kraid/3=Tourian/4=Ridley——
/// BankTable（$CA30）序自证（is_engine_running 同源））。
fn items_table(area: u8) -> &'static [game_data::area::items::SpecialItem] {
    use game_data::area::items as it;
    match area & 0x0F {
        0 => it::ITEMS_BRINSTAR,
        1 => it::ITEMS_NORFAIR,
        2 => it::ITEMS_KRAID,
        3 => it::ITEMS_TOURIAN,
        4 => it::ITEMS_RIDLEY,
        _ => &[],
    }
}

/// ScanForItems（ED98——SetupRoom 首件）：当前图格特项扫描→分派装载。
/// 【语义等价形】Y 序链表=按 (map_y,map_x) 排序扁表；扫描匹配当前图格 ✓。
pub fn scan_for_items(mem: &mut FrameMemory) {
    let map_x = mem.read_ram(crate::ram_map::MAP_POS_X);
    let map_y = mem.read_ram(crate::ram_map::MAP_POS_Y);
    let area = mem.read_ram(crate::ram_map::IN_AREA);
    for item in items_table(area) {
        if item.map_y == map_y && item.map_x == map_x {
            spawn_item(mem, item);
        }
    }
}

/// ChooseSpawningRoutine（EDD6）分派：it_Type 低半字节。
fn spawn_item(mem: &mut FrameMemory, item: &game_data::area::items::SpecialItem) {
    match item.item_type & 0x0F {
        0x02 => spawn_power_up(mem, item),
        // 【占位臂记账】Squeept/Mellow/Elevator/MBCannon/MB/Zebetite/Rinka/Door/Palette
        // 装载件（打敌人/门/电梯包随件）——记账不炸（房间装载径保持可跑）。
        t => {
            crate::frame::DIVERGENCES.with(|d| {
                d.borrow_mut().push(match t {
                    0x01 => 0xEDF8, // SpawnMapEnemy
                    0x03 => 0xEE63, // SpawnMellows
                    0x04 => 0xEEA1, // SpawnElevator
                    0x05 => 0xEEA6, // SpawnCannon
                    0x06 => 0xEEAE, // SpawnMotherBrain
                    0x07 => 0xEECA, // SpawnZebetite
                    0x08 => 0xEEEE, // SpawnRinkaSpawner
                    0x09 => 0xEEF4, // SpawnDoor
                    _ => 0xEEFA,     // SpawnPalette
                })
            });
        }
    }
}

/// CreateItemID（DC67）：唯一物 ID=（(MapX&7)<<5|MapY / (type<<2)|(MapX>>3)）。
fn create_item_id(map_x: u8, map_y: u8, item_type: u8) -> (u8, u8) {
    let b0 = ((map_x & 0x07) << 5) | (map_y & 0x1F);
    let b1 = ((item_type << 2) & 0xFC) | (map_x >> 3);
    (b0, b1)
}

/// CheckForItem（EE4A）：UniqueItemHistory（$6887 起 2B/件）比对。返回 true=已收集。
fn check_for_item(mem: &FrameMemory, id: (u8, u8)) -> bool {
    let n = mem.read_wram(NUM_UNIQUE_ITEMS) as usize;
    for i in 0..n {
        let lo = mem.read_wram(UNIQUE_ITEM_HISTORY + (i * 2) as u16);
        let hi = mem.read_wram(UNIQUE_ITEM_HISTORY + (i * 2 + 1) as u16);
        if lo == id.0 && hi == id.1 {
            return true;
        }
    }
    false
}

/// GetItemXYPos 尾径（DC1C→史录——非 beam 件入史）。返回 false=史满（$6886 上界）。
fn record_item(mem: &mut FrameMemory, id: (u8, u8)) -> bool {
    let n = mem.read_wram(NUM_UNIQUE_ITEMS) as usize;
    if n >= 0x5B {
        return false; // 史满（$6887+2n<$68FC 界）
    }
    mem.write_wram(UNIQUE_ITEM_HISTORY + (n * 2) as u16, id.0);
    mem.write_wram(UNIQUE_ITEM_HISTORY + (n * 2 + 1) as u16, id.1);
    mem.write_wram(NUM_UNIQUE_ITEMS, (n + 1) as u8);
    true
}

/// SpawnPowerUp（EDFE）：槽分配→PrepareItemID+CheckForItem→位置解码→槽写。
fn spawn_power_up(mem: &mut FrameMemory, item: &game_data::area::items::SpecialItem) {
    let slot = if mem.read_ram(POWERUPS_BASE) == 0xFF {
        0u16
    } else if mem.read_ram(POWERUPS_BASE + 8) == 0xFF {
        8
    } else {
        return; // 双槽满（EE0F 后 @exit）
    };
    let item_type = item.args[0]; // pu_XXX
    let id = create_item_id(item.map_x, item.map_y, item_type);
    if check_for_item(mem, id) {
        return; // 已收集（EE4A carry 置——bcs @exit）
    }
    // 位置解码（EDF4-）：y=(pos&$F0)|8；x=((pos<<4)&$F0)|8；hi=GetNameTableAtScrollDir（EB85）
    let pos = item.args[1];
    let y = (pos & 0xF0) | 0x08;
    let x = ((pos << 4) & 0xF0) | 0x08;
    let hi = (mem.read_ram(crate::ram_map::PPUCTRL_ZP) ^ mem.read_ram(crate::ram_map::SCROLL_DIR)) & 1;
    mem.write_ram(POWERUPS_BASE + slot, item_type);
    mem.write_ram(POWERUPS_BASE + slot + 1, y);
    mem.write_ram(POWERUPS_BASE + slot + 2, x);
    mem.write_ram(POWERUPS_BASE + slot + 3, hi);
}

/// UpdateAllPowerUps（DB37）帧径（uw_all_power_ups 会师）：双槽→埋实心块门→显示→
/// 触碰→拾取授予。返回拾取的 pu_XXX（Some=本帧拾取）。
pub fn update_all_power_ups(mem: &mut FrameMemory) -> Option<u8> {
    for slot in [0u16, 8u16] {
        let type_ = mem.read_ram(POWERUPS_BASE + slot);
        if type_ == 0xFF {
            continue; // 空槽（:5517-5519）
        }
        let y = mem.read_ram(POWERUPS_BASE + slot + 1);
        let x = mem.read_ram(POWERUPS_BASE + slot + 2);
        let hi = mem.read_ram(POWERUPS_BASE + slot + 3);
        // 埋实心块门（:5529-5537 GetObjRoomRAMPtr→房间字节<$A0⇒出口）
        let room_byte = {
            let idx = (((y & 0xF8) as u16) << 2) | ((x >> 3) as u16);
            let page = if hi & 1 != 0 { 0x0400u16 } else { 0x0000 };
            mem.read_wram(0x6000 | page | (idx & 0x3FF))
        };
        if room_byte < 0xA0 {
            continue;
        }
        // staging（:5521-5527）→animFrame（:5539-5545：(type&$0F)|$50）/ObjectCntrl
        // （:5547-5554：(FC>>1)&3|$80）
        mem.write_ram(POWERUP_DRAW_BASE + 0x0D, y);
        mem.write_ram(POWERUP_DRAW_BASE + 0x0E, x);
        mem.write_ram(POWERUP_DRAW_BASE + 0x0C, hi);
        let anim_frame = (type_ & 0x0F) | 0x50; // _id_ObjFrame_BombItem=$50（位 4+6）
        mem.write_ram(POWERUP_DRAW_BASE + 0x03, anim_frame);
        let fc = mem.read_ram(crate::ram_map::FRAME_COUNT);
        mem.write_ram(0x006B, ((fc >> 1) & 0x03) | 0x80); // ObjectCntrl
        // ObjDrawFrame（DE4A——显示（sprite 管线：PageIndex=$40 面））
        let sp0 = mem.read_ram(0x004B); // SpritePagePos 位面
        crate::frame::sprite::anim_draw_object(mem, 0x40);
        if mem.read_ram(0x004B) == sp0 {
            continue; // 未绘出（:5563-5566 cmp/beq Exit9）
        }
        // AreObjectsTouching（DC7F——X=0（Samus）/Y=$40（物）；同名表近距判）
        if !are_objects_touching(mem) {
            continue; // 未触碰（:5604-5606 bcs Exit9）
        }
        // ==== 拾取授予（:5608-）====
        record_item(mem, create_item_id(
            mem.read_ram(crate::ram_map::MAP_POS_X),
            mem.read_ram(crate::ram_map::MAP_POS_Y),
            type_,
        ));
        let gear = mem.read_wram(crate::ram_map::SAMUS_GEAR);
        if type_ == 0x06 || type_ == 0x07 {
            // beam 件（wave/ice）：清旧 beam 位（:5633-5639）
            mem.write_wram(crate::ram_map::SAMUS_GEAR, gear & !0xC0);
        }
        if type_ >= 0x08 {
            // 能量罐/弹药（MissileEnergyTank DBF7）
            if type_ == 0x09 {
                // 弹药 +5（AddToMaxMissiles——弹药面）
                let m = mem.read_ram(0x6877).wrapping_add(5); // MaxMissiles（constants:101）
                mem.write_ram(0x6877, m);
            } else {
                // 能量罐（≤6）+满血（:5674-5691）
                let n = mem.read_ram(0x6874); // EnergyTankCount
                if n < 6 {
                    mem.write_ram(0x6874, n.wrapping_add(1));
                }
                let tanks = mem.read_ram(0x6874);
                mem.write_ram(0x0107, (tanks << 4) | 0x09); // Health+1（上 nibble 罐数+9）
                mem.write_ram(0x0106, 0x99); // Health（满）
            }
        } else {
            // 能力件：MakeBitMask（DB2F：1<<type）→SamusGear OR（:5640-5645）（$6878=WRAM！）
            let mask = 1u8 << type_;
            let gear = mem.read_wram(crate::ram_map::SAMUS_GEAR);
            mem.write_wram(crate::ram_map::SAMUS_GEAR, gear | mask);
        }
        // PowerUpDelayFlag←$FF/槽清/ItemRoomMusicStatus（:5646-5658）
        mem.write_ram(POWER_UP_DELAY_FLAG, 0xFF);
        mem.write_ram(POWERUPS_BASE + slot, 0xFF);
        if mem.read_ram(crate::ram_map::ITEM_ROOM_MUSIC_STATUS) != 0 {
            mem.write_ram(crate::ram_map::ITEM_ROOM_MUSIC_STATUS, 0x01);
        }
        return Some(type_);
    }
    None
}

/// AreObjectsTouching（DC7F——X 槽 0（Samus）/Y 槽 $40（PowerUpDraw)）：
/// **屏距径**（渲染同构：屏位=x−ScrollX（hi 只进可见门不进位距））：
/// |Δy|<radiusY 和∧|Δx|<radiusX 和（F1A7 半径和（Objects[x].radius+Objects[y].radius）
/// +F1FA 判盒）。
/// > 【2026-10-31 bug 报修更正】旧形 hi 差折 ±256⇒dx 恒 256+⇒**触碰永不触发**
/// > （用户报「碰撞变形球物品后无法吃到」根因）——渲染径屏位=纯 x 差（hi 仅可见门），
/// > 触碰同径（物 hi=1/人 hi=0 但同屏可视=屏距直比）。
fn are_objects_touching(mem: &FrameMemory) -> bool {
    let sy = mem.read_ram(0x030D);
    let sx = mem.read_ram(0x030E);
    let iy = mem.read_ram(POWERUP_DRAW_BASE + 0x0D);
    let ix = mem.read_ram(POWERUP_DRAW_BASE + 0x0E);
    let ry = mem.read_ram(0x0301) as u16 + mem.read_ram(POWERUP_DRAW_BASE + 0x01) as u16;
    let rx = mem.read_ram(0x0302) as u16 + mem.read_ram(POWERUP_DRAW_BASE + 0x02) as u16;
    let dy = (sy as i16 - iy as i16).unsigned_abs() as u16;
    let dx = (sx as i16 - ix as i16).unsigned_abs() as u16;
    dy < ry && dx < rx
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 触碰判（屏距径——bug 报修回归锚（hi 差不折距））：同屏近距⇒触；hi 差仍触（同屏）。
    #[test]
    fn touching_fires_regardless_of_nametable_hi() {
        let mut m = FrameMemory::default();
        m.write_ram(0x030D, 152); // Samus y
        m.write_ram(0x030E, 100); // x
        m.write_ram(0x0301, 15); // radiusY
        m.write_ram(0x0302, 4);
        m.write_ram(POWERUP_DRAW_BASE + 0x0D, 152); // item y（同高）
        m.write_ram(POWERUP_DRAW_BASE + 0x0E, 102); // x（2px 距）
        m.write_ram(POWERUP_DRAW_BASE + 0x01, 3);
        m.write_ram(POWERUP_DRAW_BASE + 0x02, 3);
        // hi 差（物=1/人=0）——旧形折 ±256⇒不触；正解=同屏直比
        m.write_ram(0x030C, 0);
        m.write_ram(POWERUP_DRAW_BASE + 0x0C, 1);
        assert!(are_objects_touching(&m), "同屏近距应触（hi 差不折距）");
        // 远距不触
        m.write_ram(POWERUP_DRAW_BASE + 0x0E, 200);
        assert!(!are_objects_touching(&m), "远距不触");
    }
}
