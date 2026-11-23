//! game-core::memmap::room_sprite_table_face —— **房间精灵表面（`$0700-$077F`）的 typed 声明层**
//! （B1-b 首批：把「已实现但零声明」的 128 B 升格为可审计的 `Owner` 段）。
//!
//! 【这一面是什么】`constants_ram.asm:1169-1206` 的 `.enum $0700 export`——房间里按名表
//! （name table）登记的静态精灵表，共 5 组：
//!
//! | 地址 | asm | 槽 × 长 | 语义 |
//! |---|---|---|---|
//! | `$0700-$0723` | `Mem0700 instanceof Struct0700 6` | 6 × 6 B | 六槽通用精灵槽（asm 注 unused） |
//! | `$0724-$0727` | `SpareMem0724 ds 4` | 4 B | 填充 |
//! | `$0728-$0747` | `PipeBugHoles instanceof PipeBugHole 4` | 4 × 8 B | 管道虫洞口 |
//! | `$0748-$0757` | `PowerUps instanceof PowerUp 2` | 2 × 8 B | 名表上的道具 |
//! | `$0758-$077F` | `Zebetites instanceof Zebetite 5` | 5 × 8 B | Tourian 的斑马壁（Zebetite） |
//!
//! 【为什么这一面此前是 Undeclared】行为**都已移植**，但读写全走裸地址：
//! `world::room_draw::delete_offscreen_room_sprites`（`Doors_RemoveIfOffScreen` 族的 ⑨/⑪/⑫
//! 三段循环：`MEM0700 = 0x0700`、`PIPE_BUG_HOLES = 0x0728`、`POWER_UPS = 0x0748`，
//! 见 `crates/game-core/src/world/room_draw.rs:375-382`、`:463-464`、`:492-493`）；
//! Zebetite 走 `crate::frame::memory::Manager` 的 `0x0758 + slot*8 + off` 访问器
//! （`crates/game-core/src/frame/memory.rs:1030`，调用点 `crates/game-core/src/enemy/zebetite.rs:204`）。
//! 字段布局只活在那些字面量偏移里 ⇒ `nes-map` 审计看不到这 128 B 的归属。
//!
//! 【本模块只做两件事】①把 asm 结构升格成 typed 表示（字段名/类型/偏移，逐字段带 asm 锚）；
//! ②把该面登记进 [`crate::memmap::nesmap::entity_spans`]。**行为零变化**：业务层仍是唯一
//! 读写路径；[`RoomSpriteTablesFace::project`] 是同一批字节的编码出口，
//! [`RoomSpriteTablesFace::decode`] 是解码入口，往返互检进单测。
//!
//! 【声明层为什么落在 `memmap/` 而不在 `world/`】与 B1-a（`memmap/tile_blast_face.rs`）同因：
//! `memmap/mod.rs` 的分层口径规定**地址常量与 codec 只活在本层**，本模块是该口径下的
//! 「槽语义视图 + 字节偏移契约面」。B1-b 的硬约束同样只动 `memmap/`。
//!
//! 【P4 迁移方向（`docs/design/fc-memory-projection-architecture.md` §4.1）】该族转为常驻
//! typed 真源时：`room_draw` 的三段裸地址循环改为委托本结构、`Manager` 的 Zebetite 访问器
//! 改为引用本模块、`storage` 不再持有这 128 B。在那之前本结构是**按需解码**，不持有跨帧
//! 状态——这也是 `Owner` 在本层的判据：这批字节的权威**解释**（字段布局）只有这里一份，
//! 字节真源仍是 `storage`。
//!
//! 【未赋义字节的处理】asm 声明但全仓无引用的 `spare01`/`spare05`/`unused`/`data07`
//! 按原字节 round-trip，不赋予猜测语义；`SpareMem0724`（4 B）作为独立 `Owner` 段保留，
//! 段名直接沿用 asm（它是被 `.enum $0700` 明确计入的地址，不是「未解释的洞」）。

use crate::memmap::nesmap::{NesMapped, NesSpan, SpanKey};
use crate::memmap::storage::NesImage;

/// 面基址（`constants_ram.asm:1169` 的 `.enum $0700`）。
pub const BASE: u16 = 0x0700;
/// 面长（`Mem0700` 起至 `Zebetites` 末 = `$0700-$077F`）。
pub const FACE_LEN: u16 = 0x80;

/// `Mem0700` 槽长（`.struct Struct0700` 6 B）。
pub const MEM0700_SLOT_LEN: u16 = 6;
/// `Mem0700` 槽数（`instanceof Struct0700 6`）。
pub const MEM0700_SLOTS: usize = 6;
/// `PipeBugHole` 槽长（8 B）。
pub const PIPE_BUG_HOLE_SLOT_LEN: u16 = 8;
/// `PipeBugHole` 槽数（`instanceof PipeBugHole 4`）。
pub const PIPE_BUG_HOLE_SLOTS: usize = 4;
/// `PowerUp` 槽长（8 B）。
pub const POWER_UP_SLOT_LEN: u16 = 8;
/// `PowerUp` 槽数（`instanceof PowerUp 2`）。
pub const POWER_UP_SLOTS: usize = 2;
/// `Zebetite` 槽长（8 B）。
pub const ZEBETITE_SLOT_LEN: u16 = 8;
/// `Zebetite` 槽数（`instanceof Zebetite 5`）。
pub const ZEBETITE_SLOTS: usize = 5;

// 组起始地址（`constants_ram.asm:1171-1183`，逐行对齐 asm 注释里的地址）。
/// `Mem0700` 组起点（`$0700`）。
pub const MEM0700_BASE: u16 = 0x0700;
/// `SpareMem0724` 起点（`$0724`，`ds 4`）。
pub const SPARE_0724_BASE: u16 = 0x0724;
/// `SpareMem0724` 长（4 B）。
pub const SPARE_0724_LEN: u16 = 4;
/// `PipeBugHoles` 组起点（`$0728`）。
pub const PIPE_BUG_HOLES_BASE: u16 = 0x0728;
/// `PowerUps` 组起点（`$0748`）。
pub const POWER_UPS_BASE: u16 = 0x0748;
/// `Zebetites` 组起点（`$0758`）。
pub const ZEBETITES_BASE: u16 = 0x0758;

// `Struct0700` 字段偏移（`.struct Struct0700 ; unused`，constants_ram.asm:49-54）。
const S0700_STATUS: u16 = 0;
const S0700_SPARE01: u16 = 1; // ds 3
const S0700_DATA04: u16 = 4;
const S0700_SPARE05: u16 = 5;

// `PipeBugHole` 字段偏移（constants_ram.asm:56-65）。
const PBH_STATUS: u16 = 0;
const PBH_ENEMY_SLOT: u16 = 1;
const PBH_Y: u16 = 2;
const PBH_X: u16 = 3;
const PBH_HI: u16 = 4;
const PBH_UNUSED: u16 = 5; // ds 3

// `PowerUp` 字段偏移（constants_ram.asm:67-74）。
const PU_TYPE: u16 = 0;
const PU_Y: u16 = 1;
const PU_X: u16 = 2;
const PU_HI: u16 = 3;
const PU_UNUSED: u16 = 4; // ds 3
const PU_DATA07: u16 = 7;

// `Zebetite` 字段偏移（constants_ram.asm:76-83）。
const ZB_STATUS: u16 = 0;
const ZB_ROOM_RAM_PTR: u16 = 1; // dw（小端）
const ZB_QTY_HITS: u16 = 3;
const ZB_HEALING_DELAY: u16 = 4;
const ZB_IS_HIT: u16 = 5;
const ZB_UNUSED: u16 = 6; // dw

/// 单槽 `Struct0700`（6 B）的 typed 表示。
///
/// asm 注释标为 unused（全仓无结构化访问点）；仍按原布局登记，因为
/// `delete_offscreen_room_sprites` 的 ⑨ 段读 `data04`、写 `status`（`$FF` = 空槽）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Mem0700Slot {
    /// +0 `status`：槽状态（`$FF` = 空槽，见 ⑨ 段 `status←$FF`）。
    pub status: u8,
    /// +1 `spare01`（`ds 3`，无结构化引用）：按原字节保留。
    pub spare01: [u8; 3],
    /// +4 `data04`：⑨ 段的删除判据（`data04 == 0` ⇒ 该槽在进入视野的名表里 ⇒ 清空）。
    pub data04: u8,
    /// +5 `spare05`（无引用）：按原字节保留。
    pub spare05: u8,
}

impl Mem0700Slot {
    /// 槽内字节解码（`bytes` 长度必须 ≥ [`MEM0700_SLOT_LEN`]）。
    pub fn decode(bytes: &[u8]) -> Self {
        debug_assert!(bytes.len() >= MEM0700_SLOT_LEN as usize, "Mem0700 槽解码需要 6 B");
        Self {
            status: bytes[usize::from(S0700_STATUS)],
            spare01: [
                bytes[usize::from(S0700_SPARE01)],
                bytes[usize::from(S0700_SPARE01) + 1],
                bytes[usize::from(S0700_SPARE01) + 2],
            ],
            data04: bytes[usize::from(S0700_DATA04)],
            spare05: bytes[usize::from(S0700_SPARE05)],
        }
    }

    /// 槽内字节编码（`bytes` 长度必须 ≥ [`MEM0700_SLOT_LEN`]）。
    pub fn encode(&self, bytes: &mut [u8]) {
        debug_assert!(bytes.len() >= MEM0700_SLOT_LEN as usize, "Mem0700 槽编码需要 6 B");
        bytes[usize::from(S0700_STATUS)] = self.status;
        bytes[usize::from(S0700_SPARE01)..usize::from(S0700_SPARE01) + 3]
            .copy_from_slice(&self.spare01);
        bytes[usize::from(S0700_DATA04)] = self.data04;
        bytes[usize::from(S0700_SPARE05)] = self.spare05;
    }
}

/// 单槽 `PipeBugHole`（8 B）：管道虫洞口。
///
/// ⑪ 段判据：`nt == hi` ⇒ `status←$FF`（洞口所在的位面正在进入视野 ⇒ 删）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PipeBugHoleSlot {
    /// +0 `status`：bit0-3 = 生成的敌人类型（常见 `$7` 管道虫）、bit7 = 强化变体、
    /// `$FF` = 无洞口。
    pub status: u8,
    /// +1 `enemySlot`：管道虫要占用的敌槽号。
    pub enemy_slot: u8,
    /// +2 `y`：洞口 y（房间坐标）。
    pub y: u8,
    /// +3 `x`：洞口 x（房间坐标）。
    pub x: u8,
    /// +4 `hi`：洞口所在名表。
    pub hi: u8,
    /// +5 `unused`（`ds 3`）：按原字节保留。
    pub unused: [u8; 3],
}

impl PipeBugHoleSlot {
    /// 槽内字节解码（`bytes` 长度必须 ≥ [`PIPE_BUG_HOLE_SLOT_LEN`]）。
    pub fn decode(bytes: &[u8]) -> Self {
        debug_assert!(
            bytes.len() >= PIPE_BUG_HOLE_SLOT_LEN as usize,
            "PipeBugHole 槽解码需要 8 B"
        );
        Self {
            status: bytes[usize::from(PBH_STATUS)],
            enemy_slot: bytes[usize::from(PBH_ENEMY_SLOT)],
            y: bytes[usize::from(PBH_Y)],
            x: bytes[usize::from(PBH_X)],
            hi: bytes[usize::from(PBH_HI)],
            unused: [
                bytes[usize::from(PBH_UNUSED)],
                bytes[usize::from(PBH_UNUSED) + 1],
                bytes[usize::from(PBH_UNUSED) + 2],
            ],
        }
    }

    /// 槽内字节编码（`bytes` 长度必须 ≥ [`PIPE_BUG_HOLE_SLOT_LEN`]）。
    pub fn encode(&self, bytes: &mut [u8]) {
        debug_assert!(
            bytes.len() >= PIPE_BUG_HOLE_SLOT_LEN as usize,
            "PipeBugHole 槽编码需要 8 B"
        );
        bytes[usize::from(PBH_STATUS)] = self.status;
        bytes[usize::from(PBH_ENEMY_SLOT)] = self.enemy_slot;
        bytes[usize::from(PBH_Y)] = self.y;
        bytes[usize::from(PBH_X)] = self.x;
        bytes[usize::from(PBH_HI)] = self.hi;
        bytes[usize::from(PBH_UNUSED)..usize::from(PBH_UNUSED) + 3]
            .copy_from_slice(&self.unused);
    }
}

/// 单槽 `PowerUp`（8 B）：名表上的道具。
///
/// ⑫ 段判据：`nt == hi` ⇒ `type←$FF`（道具所在位面正在进入视野 ⇒ 删）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PowerUpSlot {
    /// +0 `type`：描述名表上是什么道具的字节（⑫ 段删除时写 `$FF`）。
    pub kind: u8,
    /// +1 `y`：道具 y。
    pub y: u8,
    /// +2 `x`：道具 x。
    pub x: u8,
    /// +3 `hi`：`$00` = 名表 0，`$01` = 名表 3。
    pub hi: u8,
    /// +4 `unused`（`ds 3`）：按原字节保留。
    pub unused: [u8; 3],
    /// +7 `data07`：进 A 后被 `ObjDrawFrame` 立即覆写（asm 注）。
    pub data07: u8,
}

impl PowerUpSlot {
    /// 槽内字节解码（`bytes` 长度必须 ≥ [`POWER_UP_SLOT_LEN`]）。
    pub fn decode(bytes: &[u8]) -> Self {
        debug_assert!(bytes.len() >= POWER_UP_SLOT_LEN as usize, "PowerUp 槽解码需要 8 B");
        Self {
            kind: bytes[usize::from(PU_TYPE)],
            y: bytes[usize::from(PU_Y)],
            x: bytes[usize::from(PU_X)],
            hi: bytes[usize::from(PU_HI)],
            unused: [
                bytes[usize::from(PU_UNUSED)],
                bytes[usize::from(PU_UNUSED) + 1],
                bytes[usize::from(PU_UNUSED) + 2],
            ],
            data07: bytes[usize::from(PU_DATA07)],
        }
    }

    /// 槽内字节编码（`bytes` 长度必须 ≥ [`POWER_UP_SLOT_LEN`]）。
    pub fn encode(&self, bytes: &mut [u8]) {
        debug_assert!(bytes.len() >= POWER_UP_SLOT_LEN as usize, "PowerUp 槽编码需要 8 B");
        bytes[usize::from(PU_TYPE)] = self.kind;
        bytes[usize::from(PU_Y)] = self.y;
        bytes[usize::from(PU_X)] = self.x;
        bytes[usize::from(PU_HI)] = self.hi;
        bytes[usize::from(PU_UNUSED)..usize::from(PU_UNUSED) + 3]
            .copy_from_slice(&self.unused);
        bytes[usize::from(PU_DATA07)] = self.data07;
    }
}

/// 单槽 `Zebetite`（8 B）：Tourian 的斑马壁。
///
/// 访问器在 `crate::frame::memory::Manager`（`0x0758 + slot*8 + off`），调用点
/// `crate::enemy::zebetite`（导弹命中、8 击击破、`$40` 帧回血 1 击）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ZebetiteSlot {
    /// +0 `status`：槽状态（`$FF` = 空槽）。
    pub status: u8,
    /// +1 `roomRAMPtr`（`dw`，小端）：指向斑马壁左上 tile 的 RoomRAM 指针。
    pub room_ram_ptr: u16,
    /// +3 `qtyHits`：已吃导弹数（8 击击破；差分口径见 `enemy::zebetite`）。
    pub qty_hits: u8,
    /// +4 `healingDelay`：从 `$40` 倒数到 `$00` 时回复 1 击。
    pub healing_delay: u8,
    /// +5 `isHit`：本帧被导弹命中 = `$01`，否则 `$00`。
    pub is_hit: u8,
    /// +6 `unused`（`dw`）：按原字节保留。
    pub unused: [u8; 2],
}

impl ZebetiteSlot {
    /// 槽内字节解码（`bytes` 长度必须 ≥ [`ZEBETITE_SLOT_LEN`]）。
    pub fn decode(bytes: &[u8]) -> Self {
        debug_assert!(bytes.len() >= ZEBETITE_SLOT_LEN as usize, "Zebetite 槽解码需要 8 B");
        Self {
            status: bytes[usize::from(ZB_STATUS)],
            // 6502 小端：`lda Zebetites.0.roomRAMPtr,x` / `... + 1,x`。
            room_ram_ptr: u16::from_le_bytes([
                bytes[usize::from(ZB_ROOM_RAM_PTR)],
                bytes[usize::from(ZB_ROOM_RAM_PTR) + 1],
            ]),
            qty_hits: bytes[usize::from(ZB_QTY_HITS)],
            healing_delay: bytes[usize::from(ZB_HEALING_DELAY)],
            is_hit: bytes[usize::from(ZB_IS_HIT)],
            unused: [
                bytes[usize::from(ZB_UNUSED)],
                bytes[usize::from(ZB_UNUSED) + 1],
            ],
        }
    }

    /// 槽内字节编码（`bytes` 长度必须 ≥ [`ZEBETITE_SLOT_LEN`]）。
    pub fn encode(&self, bytes: &mut [u8]) {
        debug_assert!(bytes.len() >= ZEBETITE_SLOT_LEN as usize, "Zebetite 槽编码需要 8 B");
        bytes[usize::from(ZB_STATUS)] = self.status;
        let ptr = self.room_ram_ptr.to_le_bytes();
        bytes[usize::from(ZB_ROOM_RAM_PTR)] = ptr[0];
        bytes[usize::from(ZB_ROOM_RAM_PTR) + 1] = ptr[1];
        bytes[usize::from(ZB_QTY_HITS)] = self.qty_hits;
        bytes[usize::from(ZB_HEALING_DELAY)] = self.healing_delay;
        bytes[usize::from(ZB_IS_HIT)] = self.is_hit;
        bytes[usize::from(ZB_UNUSED)..usize::from(ZB_UNUSED) + 2]
            .copy_from_slice(&self.unused);
    }
}

/// 房间精灵表整面（`$0700-$077F`，128 B）的 typed 表示。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RoomSpriteTablesFace {
    mem0700: [Mem0700Slot; MEM0700_SLOTS],
    spare0724: [u8; SPARE_0724_LEN as usize],
    pipe_bug_holes: [PipeBugHoleSlot; PIPE_BUG_HOLE_SLOTS],
    power_ups: [PowerUpSlot; POWER_UP_SLOTS],
    zebetites: [ZebetiteSlot; ZEBETITE_SLOTS],
}

impl RoomSpriteTablesFace {
    /// 从 2KB RAM 镜像面解码（地址按 `& 0x7FF` 归一）。
    pub fn decode(ram: &[u8]) -> Self {
        let at = usize::from(BASE & 0x7FF);
        let mut mem0700 = [Mem0700Slot::default(); MEM0700_SLOTS];
        for (i, slot) in mem0700.iter_mut().enumerate() {
            let base = at + usize::from(MEM0700_BASE - BASE) + i * usize::from(MEM0700_SLOT_LEN);
            *slot = Mem0700Slot::decode(&ram[base..base + usize::from(MEM0700_SLOT_LEN)]);
        }
        let spare_at = at + usize::from(SPARE_0724_BASE - BASE);
        let mut spare0724 = [0u8; SPARE_0724_LEN as usize];
        spare0724.copy_from_slice(&ram[spare_at..spare_at + SPARE_0724_LEN as usize]);

        let mut pipe_bug_holes = [PipeBugHoleSlot::default(); PIPE_BUG_HOLE_SLOTS];
        for (i, slot) in pipe_bug_holes.iter_mut().enumerate() {
            let base = at + usize::from(PIPE_BUG_HOLES_BASE - BASE)
                + i * usize::from(PIPE_BUG_HOLE_SLOT_LEN);
            *slot = PipeBugHoleSlot::decode(&ram[base..base + usize::from(PIPE_BUG_HOLE_SLOT_LEN)]);
        }

        let mut power_ups = [PowerUpSlot::default(); POWER_UP_SLOTS];
        for (i, slot) in power_ups.iter_mut().enumerate() {
            let base =
                at + usize::from(POWER_UPS_BASE - BASE) + i * usize::from(POWER_UP_SLOT_LEN);
            *slot = PowerUpSlot::decode(&ram[base..base + usize::from(POWER_UP_SLOT_LEN)]);
        }

        let mut zebetites = [ZebetiteSlot::default(); ZEBETITE_SLOTS];
        for (i, slot) in zebetites.iter_mut().enumerate() {
            let base = at + usize::from(ZEBETITES_BASE - BASE) + i * usize::from(ZEBETITE_SLOT_LEN);
            *slot = ZebetiteSlot::decode(&ram[base..base + usize::from(ZEBETITE_SLOT_LEN)]);
        }

        Self {
            mem0700,
            spare0724,
            pipe_bug_holes,
            power_ups,
            zebetites,
        }
    }

    /// 编码回 2KB RAM 镜像面（只写 `$0700-$077F`）。
    pub fn encode(&self, ram: &mut [u8]) {
        let at = usize::from(BASE & 0x7FF);
        for (i, slot) in self.mem0700.iter().enumerate() {
            let base = at + usize::from(MEM0700_BASE - BASE) + i * usize::from(MEM0700_SLOT_LEN);
            slot.encode(&mut ram[base..base + usize::from(MEM0700_SLOT_LEN)]);
        }
        let spare_at = at + usize::from(SPARE_0724_BASE - BASE);
        ram[spare_at..spare_at + SPARE_0724_LEN as usize].copy_from_slice(&self.spare0724);
        for (i, slot) in self.pipe_bug_holes.iter().enumerate() {
            let base = at + usize::from(PIPE_BUG_HOLES_BASE - BASE)
                + i * usize::from(PIPE_BUG_HOLE_SLOT_LEN);
            slot.encode(&mut ram[base..base + usize::from(PIPE_BUG_HOLE_SLOT_LEN)]);
        }
        for (i, slot) in self.power_ups.iter().enumerate() {
            let base =
                at + usize::from(POWER_UPS_BASE - BASE) + i * usize::from(POWER_UP_SLOT_LEN);
            slot.encode(&mut ram[base..base + usize::from(POWER_UP_SLOT_LEN)]);
        }
        for (i, slot) in self.zebetites.iter().enumerate() {
            let base = at + usize::from(ZEBETITES_BASE - BASE) + i * usize::from(ZEBETITE_SLOT_LEN);
            slot.encode(&mut ram[base..base + usize::from(ZEBETITE_SLOT_LEN)]);
        }
    }

    /// `Mem0700` 槽视图（越界返回 `None`）。
    pub fn mem0700(&self, index: usize) -> Option<&Mem0700Slot> {
        self.mem0700.get(index)
    }

    /// `Mem0700` 槽可变视图（越界返回 `None`）。
    pub fn mem0700_mut(&mut self, index: usize) -> Option<&mut Mem0700Slot> {
        self.mem0700.get_mut(index)
    }

    /// `PipeBugHoles` 槽视图（越界返回 `None`）。
    pub fn pipe_bug_hole(&self, index: usize) -> Option<&PipeBugHoleSlot> {
        self.pipe_bug_holes.get(index)
    }

    /// `PipeBugHoles` 槽可变视图（越界返回 `None`）。
    pub fn pipe_bug_hole_mut(&mut self, index: usize) -> Option<&mut PipeBugHoleSlot> {
        self.pipe_bug_holes.get_mut(index)
    }

    /// `PowerUps` 槽视图（越界返回 `None`）。
    pub fn power_up(&self, index: usize) -> Option<&PowerUpSlot> {
        self.power_ups.get(index)
    }

    /// `PowerUps` 槽可变视图（越界返回 `None`）。
    pub fn power_up_mut(&mut self, index: usize) -> Option<&mut PowerUpSlot> {
        self.power_ups.get_mut(index)
    }

    /// `Zebetites` 槽视图（越界返回 `None`）。
    pub fn zebetite(&self, index: usize) -> Option<&ZebetiteSlot> {
        self.zebetites.get(index)
    }

    /// `Zebetites` 槽可变视图（越界返回 `None`）。
    pub fn zebetite_mut(&mut self, index: usize) -> Option<&mut ZebetiteSlot> {
        self.zebetites.get_mut(index)
    }

    /// `SpareMem0724` 原始字节（`ds 4`，无结构化语义）。
    pub fn spare_0724(&self) -> &[u8; SPARE_0724_LEN as usize] {
        &self.spare0724
    }

    /// 地址 → 字节（不在本面返回 `None`）；供将来接入 `Manager::read_ram` 分派用。
    pub fn get_byte(&self, addr: u16) -> Option<u8> {
        let a = addr & 0x7FF;
        if !(BASE..BASE + FACE_LEN).contains(&a) {
            return None;
        }
        // 临时缓冲用整块 2KB 镜像面：`encode` 以 `BASE & 0x7FF` 为索引基址。
        let mut bytes = [0u8; 0x800];
        self.encode(&mut bytes);
        Some(bytes[usize::from(BASE & 0x7FF) + usize::from(a - BASE)])
    }

    /// 字节 → 地址（不在本面返回 `false`，调用方继续往下一域分派）。
    pub fn set_byte(&mut self, addr: u16, v: u8) -> bool {
        let a = addr & 0x7FF;
        if !(BASE..BASE + FACE_LEN).contains(&a) {
            return false;
        }
        let mut bytes = [0u8; 0x800];
        self.encode(&mut bytes);
        bytes[usize::from(BASE & 0x7FF) + usize::from(a - BASE)] = v;
        *self = Self::decode(&bytes);
        true
    }

    /// `Mem0700` 第 `index` 槽的绝对地址（`$0700 + index*6`）。
    pub fn mem0700_addr(index: usize) -> u16 {
        MEM0700_BASE + (index as u16) * MEM0700_SLOT_LEN
    }

    /// `PipeBugHoles` 第 `index` 槽的绝对地址（`$0728 + index*8`）。
    pub fn pipe_bug_hole_addr(index: usize) -> u16 {
        PIPE_BUG_HOLES_BASE + (index as u16) * PIPE_BUG_HOLE_SLOT_LEN
    }

    /// `PowerUps` 第 `index` 槽的绝对地址（`$0748 + index*8`）。
    pub fn power_up_addr(index: usize) -> u16 {
        POWER_UPS_BASE + (index as u16) * POWER_UP_SLOT_LEN
    }

    /// `Zebetites` 第 `index` 槽的绝对地址（`$0758 + index*8`）。
    pub fn zebetite_addr(index: usize) -> u16 {
        ZEBETITES_BASE + (index as u16) * ZEBETITE_SLOT_LEN
    }
}

impl NesMapped for RoomSpriteTablesFace {
    const DISCRIMINATOR: Option<u16> = None;

    /// 18 段 `Owner`：6（`Mem0700`）+ 1（`SpareMem0724`）+ 4（`PipeBugHoles`）+
    /// 2（`PowerUps`）+ 5（`Zebetites`），连续覆盖 `$0700-$0780`。
    ///
    /// 逐槽一段（同 B1-a 的 `TileBlastFace`）：槽是 asm 的数组元素，槽内字段不拆段
    /// ——拆开只会让 `owner_spans` 噪声化，不增加可审计信息。手写而非循环生成：
    /// `SPANS` 是 `const`，且 `--write` 的「既有 Owner span 不得变形」判据按
    /// `(start,len)` 精确比对，显式几何更容易被复核。
    const SPANS: &'static [(NesSpan, SpanKey)] = &[
        (NesSpan::value(MEM0700_BASE, MEM0700_SLOT_LEN), SpanKey::Owner),
        (
            NesSpan::value(MEM0700_BASE + 1 * MEM0700_SLOT_LEN, MEM0700_SLOT_LEN),
            SpanKey::Owner,
        ),
        (
            NesSpan::value(MEM0700_BASE + 2 * MEM0700_SLOT_LEN, MEM0700_SLOT_LEN),
            SpanKey::Owner,
        ),
        (
            NesSpan::value(MEM0700_BASE + 3 * MEM0700_SLOT_LEN, MEM0700_SLOT_LEN),
            SpanKey::Owner,
        ),
        (
            NesSpan::value(MEM0700_BASE + 4 * MEM0700_SLOT_LEN, MEM0700_SLOT_LEN),
            SpanKey::Owner,
        ),
        (
            NesSpan::value(MEM0700_BASE + 5 * MEM0700_SLOT_LEN, MEM0700_SLOT_LEN),
            SpanKey::Owner,
        ),
        (NesSpan::value(SPARE_0724_BASE, SPARE_0724_LEN), SpanKey::Owner),
        (
            NesSpan::value(PIPE_BUG_HOLES_BASE, PIPE_BUG_HOLE_SLOT_LEN),
            SpanKey::Owner,
        ),
        (
            NesSpan::value(
                PIPE_BUG_HOLES_BASE + 1 * PIPE_BUG_HOLE_SLOT_LEN,
                PIPE_BUG_HOLE_SLOT_LEN,
            ),
            SpanKey::Owner,
        ),
        (
            NesSpan::value(
                PIPE_BUG_HOLES_BASE + 2 * PIPE_BUG_HOLE_SLOT_LEN,
                PIPE_BUG_HOLE_SLOT_LEN,
            ),
            SpanKey::Owner,
        ),
        (
            NesSpan::value(
                PIPE_BUG_HOLES_BASE + 3 * PIPE_BUG_HOLE_SLOT_LEN,
                PIPE_BUG_HOLE_SLOT_LEN,
            ),
            SpanKey::Owner,
        ),
        (NesSpan::value(POWER_UPS_BASE, POWER_UP_SLOT_LEN), SpanKey::Owner),
        (
            NesSpan::value(POWER_UPS_BASE + POWER_UP_SLOT_LEN, POWER_UP_SLOT_LEN),
            SpanKey::Owner,
        ),
        (NesSpan::value(ZEBETITES_BASE, ZEBETITE_SLOT_LEN), SpanKey::Owner),
        (
            NesSpan::value(ZEBETITES_BASE + 1 * ZEBETITE_SLOT_LEN, ZEBETITE_SLOT_LEN),
            SpanKey::Owner,
        ),
        (
            NesSpan::value(ZEBETITES_BASE + 2 * ZEBETITE_SLOT_LEN, ZEBETITE_SLOT_LEN),
            SpanKey::Owner,
        ),
        (
            NesSpan::value(ZEBETITES_BASE + 3 * ZEBETITE_SLOT_LEN, ZEBETITE_SLOT_LEN),
            SpanKey::Owner,
        ),
        (
            NesSpan::value(ZEBETITES_BASE + 4 * ZEBETITE_SLOT_LEN, ZEBETITE_SLOT_LEN),
            SpanKey::Owner,
        ),
    ];

    fn project(&self, out: &mut NesImage) {
        self.encode(&mut out.ram);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 组几何必须等于 asm 的 `.enum $0700` 逐项地址。
    #[test]
    fn face_geometry_matches_asm_enum() {
        assert_eq!(BASE, 0x0700, "`.enum $0700 export`");
        assert_eq!(MEM0700_BASE, 0x0700, "`Mem0700` 起点");
        assert_eq!(MEM0700_SLOTS, 6, "`instanceof Struct0700 6`");
        assert_eq!(MEM0700_SLOT_LEN, 6, "`.struct Struct0700` 6 B");
        assert_eq!(MEM0700_BASE + MEM0700_SLOTS as u16 * MEM0700_SLOT_LEN, 0x0724);
        assert_eq!(SPARE_0724_BASE, 0x0724, "`SpareMem0724` 紧跟 Mem0700");
        assert_eq!(SPARE_0724_LEN, 4, "`ds 4`");
        assert_eq!(PIPE_BUG_HOLES_BASE, 0x0728, "`PipeBugHoles` 起点");
        assert_eq!(PIPE_BUG_HOLE_SLOTS, 4, "`instanceof PipeBugHole 4`");
        assert_eq!(
            PIPE_BUG_HOLES_BASE + PIPE_BUG_HOLE_SLOTS as u16 * PIPE_BUG_HOLE_SLOT_LEN,
            0x0748
        );
        assert_eq!(POWER_UPS_BASE, 0x0748, "`PowerUps` 起点");
        assert_eq!(POWER_UP_SLOTS, 2, "`instanceof PowerUp 2`");
        assert_eq!(
            POWER_UPS_BASE + POWER_UP_SLOTS as u16 * POWER_UP_SLOT_LEN,
            0x0758
        );
        assert_eq!(ZEBETITES_BASE, 0x0758, "`Zebetites` 起点");
        assert_eq!(ZEBETITE_SLOTS, 5, "`instanceof Zebetite 5`");
        assert_eq!(
            ZEBETITES_BASE + ZEBETITE_SLOTS as u16 * ZEBETITE_SLOT_LEN,
            0x0780,
            "`Zebetites` 末 = `$077F`，下一字节是 `VRAMStringRAMDimensions`"
        );
        assert_eq!(FACE_LEN, 0x80, "`$0700-$077F` = 128 B");
    }

    /// 声明表必须恰好 18 段 `Owner`、连续覆盖 `$0700-$0780`、无重叠。
    #[test]
    fn spans_cover_the_face_contiguously() {
        let spans = <RoomSpriteTablesFace as NesMapped>::SPANS;
        assert_eq!(spans.len(), 18, "6 + 1 + 4 + 2 + 5");
        let mut expect = BASE;
        for (span, key) in spans {
            assert_eq!(span.start, expect, "段必须连续（无重叠/无洞）");
            assert!(matches!(key, SpanKey::Owner));
            expect += span.len;
        }
        assert_eq!(expect, BASE + FACE_LEN, "末段结束 = `$0780`");
    }

    /// 字段偏移必须与 asm 的四个 `.struct` 逐项一致（每字段唯一哨兵值再定位）。
    #[test]
    fn field_offsets_match_asm_structs() {
        let mut bytes = [0u8; 8];

        let m = Mem0700Slot {
            status: 0x11,
            spare01: [0x22, 0x23, 0x24],
            data04: 0x33,
            spare05: 0x44,
        };
        m.encode(&mut bytes);
        assert_eq!(&bytes[..6], &[0x11, 0x22, 0x23, 0x24, 0x33, 0x44]);
        assert_eq!(Mem0700Slot::decode(&bytes[..6]), m);

        let p = PipeBugHoleSlot {
            status: 0x07,
            enemy_slot: 0x03,
            y: 0x55,
            x: 0x66,
            hi: 0x01,
            unused: [0x77, 0x78, 0x79],
        };
        p.encode(&mut bytes);
        assert_eq!(bytes, [0x07, 0x03, 0x55, 0x66, 0x01, 0x77, 0x78, 0x79]);
        assert_eq!(PipeBugHoleSlot::decode(&bytes), p);

        let u = PowerUpSlot {
            kind: 0xA0,
            y: 0xA1,
            x: 0xA2,
            hi: 0xA3,
            unused: [0xA4, 0xA5, 0xA6],
            data07: 0xA7,
        };
        u.encode(&mut bytes);
        assert_eq!(bytes, [0xA0, 0xA1, 0xA2, 0xA3, 0xA4, 0xA5, 0xA6, 0xA7]);
        assert_eq!(PowerUpSlot::decode(&bytes), u);

        let z = ZebetiteSlot {
            status: 0x00,
            room_ram_ptr: 0xBD99, // 小端 ⇒ lo=0x99 @+1 / hi=0xBD @+2
            qty_hits: 0x08,
            healing_delay: 0x40,
            is_hit: 0x01,
            unused: [0xEE, 0xEF],
        };
        z.encode(&mut bytes);
        assert_eq!(bytes, [0x00, 0x99, 0xBD, 0x08, 0x40, 0x01, 0xEE, 0xEF]);
        assert_eq!(ZebetiteSlot::decode(&bytes), z);
    }

    /// 任意字节面往返：`encode(decode(ram)) == ram`（含 spare/unused 字节，不丢信息）。
    #[test]
    fn face_round_trip_is_byte_exact() {
        for case in 0..6u32 {
            let mut ram = [0u8; 0x800];
            let mut seed = 0x9E37_79B9u32 ^ case;
            for byte in ram.iter_mut() {
                *byte = if case == 0 {
                    0x00
                } else if case == 1 {
                    0xFF
                } else {
                    seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                    (seed >> 24) as u8
                };
            }
            let face = RoomSpriteTablesFace::decode(&ram);
            let mut out = ram;
            face.encode(&mut out);
            assert_eq!(out, ram, "case {case}：面内字节必须逐字节往返");
        }
    }

    /// 面外字节不受影响：`encode` 只写 `$0700-$077F`。
    #[test]
    fn encode_touches_only_the_face() {
        let mut ram = [0x5Au8; 0x800];
        let at = usize::from(BASE & 0x7FF);
        ram[at - 1] = 0x11; // $06FF
        ram[at + usize::from(FACE_LEN)] = 0x22; // $0780
        let (lo, hi) = (ram[at - 1], ram[at + usize::from(FACE_LEN)]);
        let face = RoomSpriteTablesFace::decode(&ram);
        face.encode(&mut ram);
        assert_eq!(ram[at - 1], lo, "$06FF 不得被写");
        assert_eq!(ram[at + usize::from(FACE_LEN)], hi, "$0780 不得被写");
        let after = RoomSpriteTablesFace::decode(&ram);
        assert_eq!(after.mem0700[5].status, 0x5A, "槽字节应原样往返");
        assert_eq!(after.zebetites[4].is_hit, 0x5A, "槽字节应原样往返");
    }

    /// `get_byte`/`set_byte` 覆盖整面且不越界（将来接入 `Manager::read_ram` 分派的契约）。
    #[test]
    fn byte_accessors_cover_exactly_the_face() {
        let mut face = RoomSpriteTablesFace::default();
        for addr in BASE..BASE + FACE_LEN {
            assert!(face.get_byte(addr).is_some(), "{addr:#06X} 应在面内");
            assert!(face.set_byte(addr, (addr & 0xFF) as u8), "{addr:#06X} 应可写");
        }
        for addr in BASE..BASE + FACE_LEN {
            assert_eq!(face.get_byte(addr), Some((addr & 0xFF) as u8), "{addr:#06X} 值回读");
        }
        assert_eq!(face.get_byte(BASE - 1), None, "$06FF 不在本面");
        assert_eq!(face.get_byte(BASE + FACE_LEN), None, "$0780 不在本面");
        assert!(!face.set_byte(BASE + FACE_LEN, 1), "面外写必须被拒");
        // 镜像归一：`$0F00` 与 `$0700` 是同一格（2KB 镜像面）。
        assert_eq!(face.get_byte(0x0F00), face.get_byte(BASE));
    }

    /// `project` 只写本面字节（`NesMapped` 契约）。
    #[test]
    fn project_writes_only_this_face() {
        let mut ram = [0xA5u8; 0x800];
        let face = RoomSpriteTablesFace::decode(&ram);
        let mut img = NesImage {
            ram: [0x3Cu8; 0x800],
            wram: [0u8; 0x2000],
        };
        face.project(&mut img);
        for addr in 0..0x800usize {
            let in_face = (usize::from(BASE)..usize::from(BASE + FACE_LEN)).contains(&addr);
            if in_face {
                assert_eq!(img.ram[addr], 0xA5, "${addr:#06X} 属于本面，应被投影");
            } else {
                assert_eq!(img.ram[addr], 0x3C, "${addr:#06X} 不属于本面，不得被写");
            }
        }
    }

    /// 业务层裸地址口径必须落到本面的同一格（防止声明与 `room_draw` 漂移）。
    #[test]
    fn slot_addresses_match_business_layer_literals() {
        // `world/room_draw.rs:379-382` 的三个常量。
        assert_eq!(MEM0700_BASE, 0x0700, "room_draw::MEM0700");
        assert_eq!(PIPE_BUG_HOLES_BASE, 0x0728, "room_draw::PIPE_BUG_HOLES");
        assert_eq!(POWER_UPS_BASE, 0x0748, "room_draw::POWER_UPS");
        // `frame/memory.rs:1030` 的 `0x0758 + slot*8 + off`。
        for slot in 0..ZEBETITE_SLOTS {
            assert_eq!(
                RoomSpriteTablesFace::zebetite_addr(slot),
                0x0758 + slot as u16 * 8,
                "Zebetite 槽 {slot}"
            );
        }
        assert_eq!(RoomSpriteTablesFace::power_up_addr(1) + 3, 0x0748 + 8 + 3);
        assert_eq!(RoomSpriteTablesFace::mem0700_addr(5) + 4, 0x0700 + 30 + 4);
        assert_eq!(
            RoomSpriteTablesFace::pipe_bug_hole_addr(3),
            0x0728 + 3 * 8
        );
    }
}
