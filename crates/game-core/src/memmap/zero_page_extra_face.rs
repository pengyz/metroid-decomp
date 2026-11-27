//! 零页补面 `$0034-$0036 / $003B-$0048 / $004A / $004C-$004E / $0053-$0059 /
//! $005C-$0064 / $0066-$006A / $0070-$0079`（合计 **52 B**）的 typed 声明层（卡 #125）。
//!
//! 【这一面是什么】`$0033` 之后、`$007A` 之前，`constants_ram.asm` 里**逐个具名**的零页变量。
//! 此前它们全部落在 `data/status/nes_map.json` 的 `Undeclared`（430 B）里：账面上「有人按地址
//! 读写，但没人认领语义」。本模块只做两件事——①把 asm 的具名布局写成 typed 结构；
//! ②把这些字节登记成 `Owner`。**行为零变化**：`world/`、`frame/`、`samus/` 的读写路径一行不动。
//!
//! 【为什么这些段能升 `Owner`（证据级 A）】每一字节都有 `refs/m1disasm/SRC/constants_ram.asm`
//! 的具名声明，且语义有 ROM 注释或既有 Rust 消费点佐证；见 [`SYMBOLS`] 的 `source` 列。
//! 反例纪律（`AGENTS.md` §3.2）：无名/纯 spare 的字节用 `spare_NN` 或 `*_unref` 命名，
//! 只锁地址不锁语义，不编造含义。
//!
//! 【敌/敌弹相关的那几格】本面的 `$0041-$0048`（`EnFramePtrTable1/2`、`EnPlacePtrTable`、
//! `EnAnimTable`）与 `$0066`（`EnemyMovePixelQty`）是**敌 AI 的指针/计数**；`$0071`
//! （`UpdatingSamusProjectile`）是**弹更新标志**。卡面要的「敌槽/敌弹字段」在对象页
//! `$0400-$04FF`（已由 `ObjectSlots` 面 100% Owner）与 `$0317-$03FF`（`Unmodeled`，
//! 不是 Undeclared），因此本卡收的是**零页里与敌/弹有关的那部分**＋同段具名变量。
//!
//! 【没收录的相邻字节（有意的）】
//! - `$0033` / `$0037-$003A` / `$0049` / `$004B` / `$004F-$0052` / `$00FC-$00FD`：已经是
//!   `Unmodeled`（有面写了原因，如 `world/room_ram.rs:162`、`world/camera/mod.rs:96`），
//!   本卡不动（棘轮只允许 Unmodeled 下降，重复声明会撞 `coverage` 重叠断言）。
//! - `$0110-$01FF`（240 B）：按 `stack_page_face` 的既有裁定，段 B 零写空档 + 段 C 真栈，
//!   **故意**留 `Undeclared`（见 `crates/game-core/src/memmap/stack_page_face.rs` 模块文档）。
//! - `$007A-$00FB` 里无名/纯 spare 的部分留给后续批次（本批只收具名连续段）。
//!
//! 【取证口径】逐字节对 `refs/m1disasm/SRC/constants_ram.asm:530-640` 与
//! `crates/game-core/src/ram_map.rs` 的同址常量；两处名字不一致时以 asm 名为准，
//! Rust 侧别名写进 [`SYMBOLS`] 的备注不再单独造字段。

use crate::memmap::nesmap::{NesMapped, NesSpan, SpanKey};
use crate::memmap::storage::NesImage;

/// 面内各段的基址（CPU 地址）——段间不连续，逐段给常量而不是一个 `BASE`。
pub const SPAN_ROOMPTR_HI: u16 = 0x0034;
pub const SPAN_STRUCTPTR: u16 = 0x0035;
pub const SPAN_AREA_POINTERS: u16 = 0x003B;
pub const SPAN_TEMP_SCROLL_DIR: u16 = 0x004A;
pub const SPAN_DIRS: u16 = 0x004C;
pub const SPAN_DOOR_STATUS: u16 = 0x0053;
pub const SPAN_DOOR_RAM_PTR: u16 = 0x005C;
pub const SPAN_ROOM_PALETTE: u16 = 0x0066;
pub const SPAN_SAMUS_FLAGS: u16 = 0x0070;

/// 本面声明总长（52 B）。
pub const FACE_LEN: usize = 52;

/// 具名地址表：`(地址, asm 名, 证据源, 本次是否单独成字段)`。
///
/// `地址` 是字段**起始**字节；`dw` 字段占 2 B，只登记起始地址。
/// `is_field = false` 表示该字节由同一结构里的数组/备用字段覆盖（如 `DoorRoomRAMPtr` 4 槽）。
pub const SYMBOLS: [(u16, &str, &str); 34] = [
    (0x0034, "RoomPtr+1", "constants_ram.asm:530（`RoomPtr dw ;$33`，本字节是高字节）"),
    (0x0035, "StructPtr", "constants_ram.asm:533"),
    (0x003B, "AreaPointers_RAM.RoomPtrTable", "constants_ram.asm:207"),
    (0x003D, "AreaPointers_RAM.StructPtrTable", "constants_ram.asm:208"),
    (0x003F, "AreaPointers_RAM.MetatileDefs", "constants_ram.asm:209"),
    (0x0041, "AreaPointers_RAM.EnFramePtrTable1", "constants_ram.asm:210"),
    (0x0043, "AreaPointers_RAM.EnFramePtrTable2", "constants_ram.asm:211"),
    (0x0045, "AreaPointers_RAM.EnPlacePtrTable", "constants_ram.asm:213"),
    (0x0047, "AreaPointers_RAM.EnAnimTable", "constants_ram.asm:214"),
    (0x004A, "TempScrollDir", "constants_ram.asm:554"),
    (0x004C, "ItemIndex", "constants_ram.asm:559"),
    (0x004D, "SamusDir", "constants_ram.asm:562"),
    (0x004E, "SamusDoorDir", "constants_ram.asm:563"),
    (0x0053, "WalkSoundDelay", "constants_ram.asm:568"),
    (0x0054, "StatuesBridgeIsSpawned", "constants_ram.asm:569"),
    (0x0055, "IsSamus", "constants_ram.asm:570"),
    (0x0056, "DoorEntryStatus", "constants_ram.asm:571（ram_map.rs 同址 DOOR_ENTRY_STATUS）"),
    (0x0057, "DoorScrollStatus", "constants_ram.asm:576"),
    (0x0058, "SamusDoorData", "constants_ram.asm:581"),
    (0x0059, "DoorDelay", "constants_ram.asm:588"),
    (0x005C, "DoorRoomRAMPtr+0", "constants_ram.asm:593（`dsw 4` 4 槽 × 2 B）"),
    (0x0064, "SamusInLava", "constants_ram.asm:596"),
    (0x0066, "EnemyMovePixelQty", "constants_ram.asm:598（敌 AI 单步次数）"),
    (0x0067, "ObjectPalette", "constants_ram.asm:599"),
    (0x0068, "RoomPalette", "constants_ram.asm:600"),
    (0x0069, "TempX", "constants_ram.asm:601"),
    (0x006A, "TempY", "constants_ram.asm:602"),
    (0x0070, "SamusInvincibleDelay", "constants_ram.asm:621（ram_map.rs 同址 SAMUS_INVINCIBLE_DELAY）"),
    (0x0071, "UpdatingSamusProjectile", "constants_ram.asm:622（弹更新标志）"),
    (0x0072, "SamusKnockbackDir", "constants_ram.asm:623"),
    (0x0073, "SamusKnockbackIsBomb", "constants_ram.asm:625"),
    (0x0074, "InArea", "constants_ram.asm:627（ram_map.rs 同址 IN_AREA）"),
    (0x0075, "SpareMem75", "constants_ram.asm:630（asm 自述 spare）"),
    (0x0076, "PaletteToggle", "constants_ram.asm:631"),
];

/// 零页补面的字段视图（字段顺序 = 地址顺序）。
///
/// 多字节字段一律小端；`spare_*` 是 asm 自述备用字节，只锁地址不锁语义。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ZeroPageExtraFace {
    /// `$0034` `RoomPtr` 高字节（`RoomPtr` 本体 `$0033` 属 `Unmodeled`，不在本面）。
    pub room_ptr_hi: u8,
    /// `$0035-$0036` `StructPtr`（房间结构表游标，小端）。
    pub struct_ptr: u16,
    /// `$003B` `RoomPtrTable`（当前区房间指针表基址）。
    pub room_ptr_table: u16,
    /// `$003D` `StructPtrTable`。
    pub struct_ptr_table: u16,
    /// `$003F` `MetatileDefs`（当前区 metatile 定义表）。
    pub metatile_defs: u16,
    /// `$0041` `EnFramePtrTable1`（敌动画帧表 1，敌 AI）。
    pub en_frame_ptr_table1: u16,
    /// `$0043` `EnFramePtrTable2`（敌动画帧表 2，敌 AI）。
    pub en_frame_ptr_table2: u16,
    /// `$0045` `EnPlacePtrTable`（敌放置表，敌 AI）。
    pub en_place_ptr_table: u16,
    /// `$0047` `EnAnimTable`（`EnemyAnimIndexTbl` 基址，敌 AI）。
    pub en_anim_table: u16,
    /// `$004A` `TempScrollDir`（载房时保存的 `ScrollDir`）。
    pub temp_scroll_dir: u8,
    /// `$004C` `ItemIndex`（`$00`/`$08`，道具双槽选择）。
    pub item_index: u8,
    /// `$004D` `SamusDir`（`0`=右，`1`=左）。
    pub samus_dir: u8,
    /// `$004E` `SamusDoorDir`（进门方向）。
    pub samus_door_dir: u8,
    /// `$0053` `WalkSoundDelay`。
    pub walk_sound_delay: u8,
    /// `$0054` `StatuesBridgeIsSpawned`（`0`=未生成，`1`=生成中）。
    pub statues_bridge_is_spawned: u8,
    /// `$0055` `IsSamus`（`1`=当前对象是 Samus）。
    pub is_samus: u8,
    /// `$0056` `DoorEntryStatus`（`0`=不在门，`1..5` 见 asm 注释，MSB=已进门）。
    pub door_entry_status: u8,
    /// `$0057` `DoorScrollStatus`（`1`=右门，`2`=左门，`3`/`4`=竖井门）。
    pub door_scroll_status: u8,
    /// `$0058` `SamusDoorData`（高 4 位滚动方向，低 4 位进门动作）。
    pub samus_door_data: u8,
    /// `$0059` `DoorDelay`（进/出门每帧倒计时）。
    pub door_delay: u8,
    /// `$005C-$0063` `DoorRoomRAMPtr`（4 槽 × 2 B，小端）。
    pub door_room_ram_ptr: [u16; 4],
    /// `$0064` `SamusInLava`（`1`=在岩浆里）。
    pub samus_in_lava: u8,
    /// `$0066` `EnemyMovePixelQty`（敌 AI 单步次数）。
    pub enemy_move_pixel_qty: u8,
    /// `$0067` `ObjectPalette`（`$00-$03`）。
    pub object_palette: u8,
    /// `$0068` `RoomPalette`。
    pub room_palette: u8,
    /// `$0069` `TempX`。
    pub temp_x: u8,
    /// `$006A` `TempY`。
    pub temp_y: u8,
    /// `$0070` `SamusInvincibleDelay`（无敌倒计时）。
    pub samus_invincible_delay: u8,
    /// `$0071` `UpdatingSamusProjectile`（弹更新标志）。
    pub updating_samus_projectile: u8,
    /// `$0072` `SamusKnockbackDir`。
    pub samus_knockback_dir: u8,
    /// `$0073` `SamusKnockbackIsBomb`。
    pub samus_knockback_is_bomb: u8,
    /// `$0074` `InArea`（低 4 位区号：`0`=Brinstar…`4`=Ridley）。
    pub in_area: u8,
    /// `$0075` `SpareMem75`（asm 自述 spare）。
    pub spare_75: u8,
    /// `$0076` `PaletteToggle`。
    pub palette_toggle: u8,
    /// `$0077` `SamusKnockbackIsBomb77`（asm 原名即带地址后缀，保留以避免混淆）。
    pub samus_knockback_is_bomb77: u8,
    /// `$0078` `SpareMem78`（asm 自述 spare）。
    pub spare_78: u8,
    /// `$0079` `ItemRoomMusicStatus`。
    pub item_room_music_status: u8,
}

impl ZeroPageExtraFace {
    /// 从 RAM 面解码（`ram` 是 `$0000-$07FF` 全页，长度 ≥ `0x800`）。
    ///
    /// 只读本面声明的 52 B，其余字节不碰。
    pub fn decode(ram: &[u8]) -> Self {
        assert!(ram.len() >= 0x800, "零页补面需要 ≥ 0x800 B 的 RAM 切片");
        let mut f = Self::default();
        f.room_ptr_hi = ram[0x0034];
        f.struct_ptr = le16(ram, 0x0035);
        f.room_ptr_table = le16(ram, 0x003B);
        f.struct_ptr_table = le16(ram, 0x003D);
        f.metatile_defs = le16(ram, 0x003F);
        f.en_frame_ptr_table1 = le16(ram, 0x0041);
        f.en_frame_ptr_table2 = le16(ram, 0x0043);
        f.en_place_ptr_table = le16(ram, 0x0045);
        f.en_anim_table = le16(ram, 0x0047);
        f.temp_scroll_dir = ram[0x004A];
        f.item_index = ram[0x004C];
        f.samus_dir = ram[0x004D];
        f.samus_door_dir = ram[0x004E];
        f.walk_sound_delay = ram[0x0053];
        f.statues_bridge_is_spawned = ram[0x0054];
        f.is_samus = ram[0x0055];
        f.door_entry_status = ram[0x0056];
        f.door_scroll_status = ram[0x0057];
        f.samus_door_data = ram[0x0058];
        f.door_delay = ram[0x0059];
        for (i, p) in f.door_room_ram_ptr.iter_mut().enumerate() {
            *p = le16(ram, 0x005C + 2 * i);
        }
        f.samus_in_lava = ram[0x0064];
        f.enemy_move_pixel_qty = ram[0x0066];
        f.object_palette = ram[0x0067];
        f.room_palette = ram[0x0068];
        f.temp_x = ram[0x0069];
        f.temp_y = ram[0x006A];
        f.samus_invincible_delay = ram[0x0070];
        f.updating_samus_projectile = ram[0x0071];
        f.samus_knockback_dir = ram[0x0072];
        f.samus_knockback_is_bomb = ram[0x0073];
        f.in_area = ram[0x0074];
        f.spare_75 = ram[0x0075];
        f.palette_toggle = ram[0x0076];
        f.samus_knockback_is_bomb77 = ram[0x0077];
        f.spare_78 = ram[0x0078];
        f.item_room_music_status = ram[0x0079];
        f
    }

    /// 编码回 RAM 面（只写本面声明的 52 B）。
    pub fn encode(&self, ram: &mut [u8]) {
        assert!(ram.len() >= 0x800, "零页补面需要 ≥ 0x800 B 的 RAM 切片");
        ram[0x0034] = self.room_ptr_hi;
        put_le16(ram, 0x0035, self.struct_ptr);
        put_le16(ram, 0x003B, self.room_ptr_table);
        put_le16(ram, 0x003D, self.struct_ptr_table);
        put_le16(ram, 0x003F, self.metatile_defs);
        put_le16(ram, 0x0041, self.en_frame_ptr_table1);
        put_le16(ram, 0x0043, self.en_frame_ptr_table2);
        put_le16(ram, 0x0045, self.en_place_ptr_table);
        put_le16(ram, 0x0047, self.en_anim_table);
        ram[0x004A] = self.temp_scroll_dir;
        ram[0x004C] = self.item_index;
        ram[0x004D] = self.samus_dir;
        ram[0x004E] = self.samus_door_dir;
        ram[0x0053] = self.walk_sound_delay;
        ram[0x0054] = self.statues_bridge_is_spawned;
        ram[0x0055] = self.is_samus;
        ram[0x0056] = self.door_entry_status;
        ram[0x0057] = self.door_scroll_status;
        ram[0x0058] = self.samus_door_data;
        ram[0x0059] = self.door_delay;
        for (i, p) in self.door_room_ram_ptr.iter().enumerate() {
            put_le16(ram, 0x005C + 2 * i, *p);
        }
        ram[0x0064] = self.samus_in_lava;
        ram[0x0066] = self.enemy_move_pixel_qty;
        ram[0x0067] = self.object_palette;
        ram[0x0068] = self.room_palette;
        ram[0x0069] = self.temp_x;
        ram[0x006A] = self.temp_y;
        ram[0x0070] = self.samus_invincible_delay;
        ram[0x0071] = self.updating_samus_projectile;
        ram[0x0072] = self.samus_knockback_dir;
        ram[0x0073] = self.samus_knockback_is_bomb;
        ram[0x0074] = self.in_area;
        ram[0x0075] = self.spare_75;
        ram[0x0076] = self.palette_toggle;
        ram[0x0077] = self.samus_knockback_is_bomb77;
        ram[0x0078] = self.spare_78;
        ram[0x0079] = self.item_room_music_status;
    }

    /// 本面 9 段的 `(起始地址, 长度)`——测试与文档用，与 [`NesMapped::SPANS`] 同源。
    pub const SPAN_TABLE: [(u16, u16); 9] = [
        (SPAN_ROOMPTR_HI, 1),
        (SPAN_STRUCTPTR, 2),
        (SPAN_AREA_POINTERS, 14),
        (SPAN_TEMP_SCROLL_DIR, 1),
        (SPAN_DIRS, 3),
        (SPAN_DOOR_STATUS, 7),
        (SPAN_DOOR_RAM_PTR, 9),
        (SPAN_ROOM_PALETTE, 5),
        (SPAN_SAMUS_FLAGS, 10),
    ];
}

fn le16(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([b[at], b[at + 1]])
}

fn put_le16(b: &mut [u8], at: usize, v: u16) {
    let [lo, hi] = v.to_le_bytes();
    b[at] = lo;
    b[at + 1] = hi;
}

impl NesMapped for ZeroPageExtraFace {
    const DISCRIMINATOR: Option<u16> = None;

    /// 9 段 `Owner`，合计 52 B；段间不连续（中间那些字节各有别的面或另有裁定）。
    const SPANS: &'static [(NesSpan, SpanKey)] = &[
        (NesSpan::value(SPAN_ROOMPTR_HI, 1), SpanKey::Owner),
        (NesSpan::value(SPAN_STRUCTPTR, 2), SpanKey::Owner),
        (NesSpan::value(SPAN_AREA_POINTERS, 14), SpanKey::Owner),
        (NesSpan::value(SPAN_TEMP_SCROLL_DIR, 1), SpanKey::Owner),
        (NesSpan::value(SPAN_DIRS, 3), SpanKey::Owner),
        (NesSpan::value(SPAN_DOOR_STATUS, 7), SpanKey::Owner),
        (NesSpan::value(SPAN_DOOR_RAM_PTR, 9), SpanKey::Owner),
        (NesSpan::value(SPAN_ROOM_PALETTE, 5), SpanKey::Owner),
        (NesSpan::value(SPAN_SAMUS_FLAGS, 10), SpanKey::Owner),
    ];

    fn project(&self, out: &mut NesImage) {
        self.encode(&mut out.ram);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ram_with_pattern() -> [u8; 0x800] {
        let mut ram = [0u8; 0x800];
        for (a, b) in ram.iter_mut().enumerate() {
            *b = (a as u8).wrapping_mul(31).wrapping_add(7);
        }
        ram
    }

    /// 段表与 `SPANS` 必须逐项一致、合计 52 B、且两两不重叠。
    #[test]
    fn spans_match_table_and_total_is_52() {
        assert_eq!(ZeroPageExtraFace::SPANS.len(), 9);
        let mut total = 0usize;
        let mut seen: Vec<(u16, u16)> = Vec::new();
        for (i, (span, key)) in ZeroPageExtraFace::SPANS.iter().enumerate() {
            assert!(matches!(key, SpanKey::Owner), "段 {i} 必须是 Owner");
            let (start, len) = ZeroPageExtraFace::SPAN_TABLE[i];
            assert_eq!(span.start, start, "段 {i} 起点");
            assert_eq!(span.len, len, "段 {i} 长度");
            total += usize::from(len);
            seen.push((start, start + len));
        }
        assert_eq!(total, FACE_LEN);
        seen.sort_unstable();
        for w in seen.windows(2) {
            assert!(w[0].1 <= w[1].0, "段重叠：{:04X?}", w);
        }
    }

    /// 每个具名符号都必须落在某一段内（不许出现「声明了但没字段」的字节）。
    #[test]
    fn every_symbol_falls_inside_a_declared_span() {
        for (addr, name, src) in SYMBOLS {
            let covered = ZeroPageExtraFace::SPAN_TABLE
                .iter()
                .any(|(s, l)| addr >= *s && addr < *s + *l);
            assert!(covered, "${addr:04X} {name}（{src}）不在任何已声明段内");
        }
    }

    /// 逐段读回：写进去的字节必须能按字段读出来（每段至少一条断言）。
    #[test]
    fn each_span_round_trips_a_written_byte() {
        let mut ram = [0u8; 0x800];
        ram[0x0034] = 0x61;
        ram[0x0035] = 0x34;
        ram[0x0036] = 0x12;
        ram[0x0041] = 0xCD;
        ram[0x0042] = 0xAB;
        ram[0x004A] = 0x02;
        ram[0x004C] = 0x08;
        ram[0x0056] = 0x81;
        ram[0x005E] = 0x40;
        ram[0x005F] = 0x60;
        ram[0x0066] = 0x03;
        ram[0x0071] = 0x01;

        let f = ZeroPageExtraFace::decode(&ram);
        assert_eq!(f.room_ptr_hi, 0x61, "$0034 RoomPtr+1");
        assert_eq!(f.struct_ptr, 0x1234, "$0035 StructPtr（小端）");
        assert_eq!(f.en_frame_ptr_table1, 0xABCD, "$0041 敌帧表 1");
        assert_eq!(f.temp_scroll_dir, 0x02, "$004A TempScrollDir");
        assert_eq!(f.item_index, 0x08, "$004C ItemIndex");
        assert_eq!(f.door_entry_status, 0x81, "$0056 DoorEntryStatus");
        assert_eq!(f.door_room_ram_ptr[1], 0x6040, "$005E DoorRoomRAMPtr[1]");
        assert_eq!(f.enemy_move_pixel_qty, 0x03, "$0066 EnemyMovePixelQty");
        assert_eq!(f.updating_samus_projectile, 0x01, "$0071");
    }

    /// 编码只写自己那 52 B：面外字节必须原样保留。
    #[test]
    fn encode_touches_only_owned_bytes() {
        let before = ram_with_pattern();
        let mut ram = before;
        let f = ZeroPageExtraFace {
            room_ptr_hi: 0xFF,
            door_delay: 0x12,
            item_room_music_status: 0x33,
            ..Default::default()
        };
        f.encode(&mut ram);
        for a in 0..0x800usize {
            let owned = ZeroPageExtraFace::SPAN_TABLE
                .iter()
                .any(|(s, l)| (a as u16) >= *s && (a as u16) < *s + *l);
            if !owned {
                assert_eq!(ram[a], before[a], "面外字节 ${a:04X} 被改写");
            }
        }
        assert_eq!(ram[0x0034], 0xFF);
        assert_eq!(ram[0x0059], 0x12);
        assert_eq!(ram[0x0079], 0x33);
    }

    /// 解码→编码往返：面上字节逐字节相等（全 52 B）。
    #[test]
    fn decode_encode_round_trip_is_identity_on_face_bytes() {
        let ram = ram_with_pattern();
        let f = ZeroPageExtraFace::decode(&ram);
        let mut out = [0u8; 0x800];
        f.encode(&mut out);
        for (s, l) in ZeroPageExtraFace::SPAN_TABLE {
            for a in s..s + l {
                assert_eq!(out[a as usize], ram[a as usize], "往返差在 ${a:04X}");
            }
        }
    }

    /// `project` 只写 RAM 面、不碰 WRAM。
    #[test]
    fn project_writes_ram_only() {
        let f = ZeroPageExtraFace {
            in_area: 0x03,
            ..Default::default()
        };
        let mut img = NesImage {
            ram: [0x11u8; 0x800],
            wram: [0x22u8; 0x2000],
        };
        f.project(&mut img);
        assert_eq!(img.ram[0x0074], 0x03);
        assert!(img.wram.iter().all(|b| *b == 0x22));
    }
}
