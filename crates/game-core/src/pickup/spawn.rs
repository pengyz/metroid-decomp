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

use crate::state::Manager;

/// PowerUps 基（2 槽×8B（constants_ram:1154-1158 `$0748-$0757`））。
const POWERUPS_BASE: u16 = 0x0748;
/// PowerUpDraw=Object 实例 @$0340（constants:849——PageIndex=$40 面）。
const POWERUP_DRAW_BASE: u16 = 0x0340;
/// PowerUpDelayFlag=$0109（constants:810——拾取音乐/延迟旗）。
const POWER_UP_DELAY_FLAG: u16 = 0x0109;

/// 区序→特项表（InArea&$0F：0=Brinstar/1=Norfair/2=Kraid/3=Tourian/4=Ridley——
/// BankTable（$CA30）序自证（is_engine_running 同源））。
fn items_table(area: u8) -> &'static [game_data::area::items::SpecialItem] {
    game_data::assets::items(area)
}

/// ScanForItems（ED98——SetupRoom 首件）：当前图格特项扫描→分派装载。
/// 【语义等价形】Y 序链表=按 (map_y,map_x) 排序扁表；扫描匹配当前图格 ✓。
pub fn scan_for_items(mem: &mut Manager) {
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
fn spawn_item(mem: &mut Manager, item: &game_data::area::items::SpecialItem) {
    match item.item_type & 0x0F {
        0x02 => spawn_power_up(mem, item),
        0x07 => spawn_zebetite(mem, item), // SpawnZebetite（07:EECA）
        0x08 => {
            // SpawnRinkaSpawner（07:EEEE）→ SpawnRinkaSpawnerRoutine（03:9D6C）
            crate::enemy::rinka::spawn_rinka_spawner(mem, item.item_type);
        }
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
                    0x09 => 0xEEF4, // SpawnDoor
                    _ => 0xEEFA,     // SpawnPalette
                })
            });
        }
    }
}

#[cfg(test)]
pub(crate) fn spawn_zebetite_for_test(mem: &mut Manager, item: &game_data::area::items::SpecialItem) {
    spawn_zebetite(mem, item);
}

/// `SpawnZebetite ; 07:EECA` + `SpawnZebetiteRoutine ; 03:9D3D`（Tourian）：
/// 特项字节高 nibble 定槽（`and #$F0 / lsr` ⇒ `x = (byte&$F0)>>1` = Zebetites 字节偏移），
/// `x*2 & $10 ^ $10 | $84` 是屏障左上瓦的 RoomRAM 低字节（`$84`/`$94` 交替），高字节由
/// `GetRoomRAMPtrHi ; 03:9D64`（`GetNameTableAtScrollDir_<<2 | (>RoomRAMA)+1`）给出 = `$61`/`$65`。
/// 之后 `CheckForItem(ui_ZEBETITE1 + 槽号)`：已收集（上次来过打过）⇒ 直接置"已死"形态
/// `status=$81 / isHit=1 / qtyHits=7`（看起来只剩一层）。
fn spawn_zebetite(mem: &mut Manager, item: &game_data::area::items::SpecialItem) {
    let byte = item.item_type;
    let slot = ((byte & 0xF0) >> 1) / 8;
    if slot >= crate::enemy::zebetite::COUNT {
        crate::unported!("bank03:9D3D-zebetite-slot");
    }
    // `asl / and #$10 / eor #$10 / ora #$84`
    let lo = (((byte & 0xF0) as u16) & 0x10 ^ 0x10) as u8 | 0x84;
    // `GetRoomRAMPtrHi`：名表号 = `(PPUCTRL_ZP ^ ScrollDir) & 1`
    let nt = (mem.read_ram(crate::ram_map::PPUCTRL_ZP) ^ mem.read_ram(crate::ram_map::SCROLL_DIR)) & 1;
    let hi = (nt << 2) | 0x61; // (>RoomRAMA)+1 = $61
    mem.set_zeb_ptr(slot, (hi as u16) << 8 | lo as u16);
    mem.set_zeb_status(slot, 1);
    mem.set_zeb_qty_hits(slot, 0);
    mem.set_zeb_healing_delay(slot, 0);
    mem.set_zeb_is_hit(slot, 0);
    // `txa / lsr / adc #>ui_ZEBETITE1` ⇒ id = ($00, $3C + slot*4)
    if check_for_item(mem, (0x00, 0x3C + slot * 4)) {
        mem.set_zeb_status(slot, 0x81);
        mem.set_zeb_is_hit(slot, 1);
        mem.set_zeb_qty_hits(slot, 7);
    }
}

/// CreateItemID（DC67）：唯一物 ID=（(MapX&7)<<5|MapY / (type<<2)|(MapX>>3)）。
fn create_item_id(map_x: u8, map_y: u8, item_type: u8) -> (u8, u8) {
    let b0 = ((map_x & 0x07) << 5) | (map_y & 0x1F);
    let b1 = ((item_type << 2) & 0xFC) | (map_x >> 3);
    (b0, b1)
}

/// CheckForItem（EE4A）：UniqueItemHistory（$6887 起 2B/件，**NumberOfUniqueItems=字节计数**）
/// 反向比对。返回 true=已收集。
///
/// asm 逐行（prg7_engine.asm:9593-9612）：
/// ```text
/// ldy NumberOfUniqueItems      ; 字节计数（0=史空）
/// beq @samusDoesNotHaveThisItem
/// @loop: lda Temp06_ItemID+1
///        cmp UniqueItemHistory-1,y   ; 高字节 = 末字节
///        bne @noMatch
///        lda Temp06_ItemID
///        cmp UniqueItemHistory-2,y   ; 低字节 = 首字节
///        beq @samusHasThisItem
/// @noMatch: dey / dey / bne @loop    ; 从尾部向前每 2B 一件
/// ```
/// 旧形把 $6886 当「件数」（`i*2` 寻址 + `0..n` 环）⇒ 1 件时只查 1 件且地址正确、
/// 2 件时查 2 件但第 2 件地址错位 ⇒ 已收集物重复入史（A 级：$6886 语义见
/// `AddItemToHistory ; 07:DC54` 的 `iny/iny/sty`）。
fn check_for_item(mem: &Manager, id: (u8, u8)) -> bool {
    // `CheckForItem ; 07:EE4A`：从尾部按 2 B 步长反向扫（视图内收口，语义见
    // `Manager::unique_item_find`）。
    mem.unique_item_find(id)
}

fn record_item(mem: &mut Manager, id: (u8, u8)) -> bool {
    // `AddItemToHistory ; 07:DC54`：字节游标追加（视图内收口）。
    mem.unique_item_push(id)
}

fn get_item_xy_pos(mem: &Manager, item_hi: u8) -> (u8, u8) {
    // `GetItemXYPos ; 07:DC1C`（= MapScrollRoutine 07:DC1E）逐行；输入面走语义视图
    // `item_spawn_inputs`（MapPosX/MapPosY/ScrollDir/ScrollX/ScrollY/PPUCTRL_ZP）。
    let (map_x, map_y, scroll_dir, scroll_x, scroll_y, ppuctrl) = mem.item_spawn_inputs();
    let mut item_x = map_x;
    let mut item_y = map_y;
    let vertical = (scroll_dir >> 1) == 0; // lsr 后 Z 置位
    let plus = (scroll_dir & 0x01) != 0; // 进位 = bit0（1=Down/Right）
    if vertical {
        if plus && scroll_y != 0 {
            item_y = item_y.wrapping_sub(1); // 下滚且偏移≠0 ⇒ y-=1
        }
    } else if plus && scroll_x != 0 {
        item_x = item_x.wrapping_sub(1); // 右滚且偏移≠0 ⇒ x-=1
    }
    // 名表位不同（(PPUCTRL_ZP ^ ItemHi) & 1）⇒ 该轴 +1
    if (ppuctrl ^ item_hi) & 0x01 != 0 {
        if vertical {
            item_y = item_y.wrapping_add(1);
        } else {
            item_x = item_x.wrapping_add(1);
        }
    }
    (item_x, item_y)
}

/// SpawnPowerUp（EDFE）：槽分配→PrepareItemID+CheckForItem→位置解码→槽写。
fn spawn_power_up(mem: &mut Manager, item: &game_data::area::items::SpecialItem) {
    let slot = if mem.read_ram(POWERUPS_BASE) == 0xFF {
        0u16
    } else if mem.read_ram(POWERUPS_BASE + 8) == 0xFF {
        8
    } else {
        return; // 双槽满（EE0F 后 @exit）
    };
    let item_type = item.args[0]; // pu_XXX
    // PrepareItemID（07:EE3D）：`sta Temp09_ItemType` + `lda MapPosX / sta Temp07_ItemX`
    // + `lda MapPosY / sta Temp06_ItemY` ⇒ **ID 取当前图格（MapPosX/MapPosY），
    // 不是特项表里的逐项 map_x/map_y**（后者只用于屏幕位置解码）。
    let (map_pos_x, map_pos_y, _, _, _, _) = mem.item_spawn_inputs();
    let id = create_item_id(
        map_pos_x,
        map_pos_y,
        item_type,
    );
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
pub fn update_all_power_ups(mem: &mut Manager) -> Option<u8> {
    // 【卡 E D2-1 端口】asm UpdateAllPowerUps 循环头（prg7_engine.asm:5502-5504）：
    // `lda #$40 / sta PageIndex`——双槽显示面共用 $340 槽（r14 dossier §3.3「漏端口」补）。
    mem.set_page_index(0x40);
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
        mem.set_obj_cntrl(((fc >> 1) & 0x03) | 0x80); // ObjectCntrl
        // 【卡 E D2-1】asm :5555-5561 `jsr ObjDrawFrame`（DE4A **纯绘**）——旧形
        // anim_draw_object（DE47=走表+绘）⇒**幻影走表**（r14 dossier §2.1 双 item 槽
        // a_in=0 走表实录）+走表错落 Samus 槽（$4B 漏设时代）。
        // 【:5555-5566 块随件勘误】`lda SpritePagePos/pha…pla/cmp SpritePagePos`=
        // **$5B**（constants_ram:568——OAM 填充计数）非 $4B（PageIndex）——旧形误读 $4B
        // 致「未绘出」恒真⇒拾取授予径永不可达。
        let sp0 = mem.sprite_page_pos(); // SpritePagePos
        crate::render::oam::obj_draw_frame(mem, 0x40);
        if mem.sprite_page_pos() == sp0 {
            continue; // 未绘出（:5563-5566 cmp/beq Exit9）
        }
        // AreObjectsTouching（DC7F——X=0（Samus）/Y=$40（物）；同名表近距判）
        if !are_objects_touching(mem) {
            continue; // 未触碰（:5604-5606 bcs Exit9）
        }
        // ==== 拾取授予（:5608-）====
        // asm（:5614-5628）：非 $FF 件先 `lda PowerUps.0.hi,x → Temp08_ItemHi` +
        // `lda PowerUps.0.type,x → Temp09_ItemType`，再 `jsr GetItemXYPos`（07:DC1C：
        // MapPosX/MapPosY + 滚动/名表位修正）→ CreateItemID → AddItemToHistory。
        {
            let (item_x, item_y) = get_item_xy_pos(mem, hi);
            record_item(mem, create_item_id(item_x, item_y, type_));
        }
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
fn are_objects_touching(mem: &Manager) -> bool {
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
        let mut m = Manager::default();
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

    /// 【卡 E D2-1（animi-divergence-collide-f800.md §11）】UpdateAllPowerUps 显示面=
    /// **纯绘**（asm :5555-5561 `jsr ObjDrawFrame`（DE4A））+循环头 **`lda #$40/sta
    /// PageIndex`**（prg7_engine.asm:5502-5504）。旧形 anim_draw_object（DE47=走表+绘）
    /// +$4B 漏设⇒**幻影走表落 Samus 槽**（r14 dossier §2.1 双 item 槽 a_in=0 走表
    /// 实录——每帧 2 条）——走表预算 3×⇒anim 过速/相位偏。emu probe_mem 写点对照：
    /// 该径零 anim 写（cardi-f798probe.probe.jsonl caller 集无 spawn 站）。
    #[test]
    fn powerup_display_is_pure_draw_no_walk() {
        let mut m = Manager::default();
        // Samus 槽走表哨兵（幻影走表会 dec animDelay）
        m.write_ram(0x0304, 5);
        m.write_ram(0x0306, 0x0D);
        // 一槽有效 powerup：type≠FF + 房块≥0xA0（埋实心门），远距不触（y/x 远离原点）
        let (y, x, hi) = (0x40u8, 0x20u8, 0u8);
        m.write_ram(POWERUPS_BASE, 0x01);
        m.write_ram(POWERUPS_BASE + 1, y);
        m.write_ram(POWERUPS_BASE + 2, x);
        m.write_ram(POWERUPS_BASE + 3, hi);
        let idx = (((y & 0xF8) as u16) << 2) | ((x >> 3) as u16);
        m.write_wram(0x6000 | (idx & 0x3FF), 0xA0);
        let _ = update_all_power_ups(&mut m);
        assert_eq!(m.read_ram(0x0304), 5, "纯绘=无走表（Samus 槽 animDelay 不动）");
        assert_eq!(m.read_ram(0x0306), 0x0D, "animIndex 不动（幻影走表会推进）");
        assert_eq!(m.read_ram(0x004B), 0x40, "PageIndex←$40（:5502-5504 端口）");
    }

    /// 【卡 E D2-1b】「未绘出」判据=**SpritePagePos($5B) 推进**（:5555-5566 pha/pla/cmp）
    /// ——旧形误读 $4B（PageIndex）⇒恒等⇒拾取授予径永不可达（原版行为缺失）。触碰+可绘
    /// ⇒授予（能力位+槽清+PowerUpDelayFlag）。
    #[test]
    fn powerup_touch_grants_when_drawn() {
        let mut m = Manager::default();
        // item 与 Samus 同位（触碰）且屏内（可绘）
        let (y, x, hi) = (0x40u8, 0x40u8, 0u8);
        m.write_ram(0x0300 + 0x0D, y); // Samus.y
        m.write_ram(0x0300 + 0x0E, x); // Samus.x
        m.write_ram(POWERUPS_BASE, 0x00); // type=0（能力件——bit0 授予）
        m.write_ram(POWERUPS_BASE + 1, y);
        m.write_ram(POWERUPS_BASE + 2, x);
        m.write_ram(POWERUPS_BASE + 3, hi);
        let idx = (((y & 0xF8) as u16) << 2) | ((x >> 3) as u16);
        m.write_wram(0x6000 | (idx & 0x3FF), 0xA0);
        let got = update_all_power_ups(&mut m);
        assert_eq!(got, Some(0x00), "触碰+可绘⇒拾取授予可达（$5B 判据）");
        assert_eq!(m.read_wram(crate::ram_map::SAMUS_GEAR) & 1, 1, "能力位 bit0 授予");
        assert_eq!(m.read_ram(POWERUPS_BASE), 0xFF, "槽清（:5646-5658）");
        assert_eq!(m.read_ram(POWER_UP_DELAY_FLAG), 0xFF, "PowerUpDelayFlag←FF");
    }
}

#[cfg(test)]
mod tests_spawn_zebetite {
    use crate::frame::Manager;

    fn item(byte: u8) -> game_data::area::items::SpecialItem {
        // 只用到 item_type（asm 的高 nibble 定槽），坐标/其它字段置 0。
        game_data::area::items::SpecialItem { map_y: 0, map_x: 0, item_type: byte, args: [0; 3] }
    }

    /// `SpawnZebetiteRoutine`：槽 = 高 nibble/8；ptr.lo 在 `$84/$94` 间交替；
    /// ptr.hi = `$61`（名表 0）或 `$65`（名表 3）。
    #[test]
    fn spawn_sets_pointer_by_nibble() {
        let mut m = Manager::default();
        m.write_ram(crate::ram_map::IN_AREA, 0x13);
        crate::pickup::spawn::spawn_zebetite_for_test(&mut m, &item(0x00));
        assert_eq!(m.zeb_status(0), 1, "槽 0 活");
        assert_eq!(m.zeb_ptr(0) & 0xFF, 0x94, "高 nibble 0 ⇒ lo=$94");
        assert_eq!((m.zeb_ptr(0) >> 8) as u8, 0x61, "名表 0 ⇒ $61");
        crate::pickup::spawn::spawn_zebetite_for_test(&mut m, &item(0x10));
        assert_eq!(m.zeb_ptr(1) & 0xFF, 0x84, "高 nibble 1 ⇒ lo=$84");
        // 名表位翻转 ⇒ hi=$65
        let mut m2 = Manager::default();
        m2.write_ram(crate::ram_map::IN_AREA, 0x13);
        m2.write_ram(crate::ram_map::PPUCTRL_ZP, 0x01);
        m2.write_ram(crate::ram_map::SCROLL_DIR, 0x00);
        crate::pickup::spawn::spawn_zebetite_for_test(&mut m2, &item(0x20));
        assert_eq!((m2.zeb_ptr(2) >> 8) as u8, 0x65, "名表 3 ⇒ $65");
    }

    /// 已收集（历史里有 `ui_ZEBETITE1+槽*4`）⇒ 生成即"已死"形态 `$81 / qty7 / isHit1`。
    #[test]
    fn already_collected_zebetite_spawns_dead() {
        let mut m = Manager::default();
        m.write_ram(crate::ram_map::IN_AREA, 0x13);
        m.unique_item_push((0x00, 0x3C + 4 * 4)); // 槽 4
        crate::pickup::spawn::spawn_zebetite_for_test(&mut m, &item(0x40));
        assert_eq!(m.zeb_status(4), 0x81);
        assert_eq!(m.zeb_qty_hits(4), 7);
        assert_eq!(m.zeb_is_hit(4), 1);
    }
}
