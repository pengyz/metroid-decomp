//! game-core::memmap::object_slot_face —— **对象槽面（敌对象 + 爆散 + 两个首领槽）的 typed 声明层**
//! （B1-c：把「已移植但零 typed 声明」的 256 B 升格为可审计的 `Owner` 段）。
//!
//! 【这一面是什么】`constants_ram.asm:963-978` 的 `.enum $0400 export`——敌对象/爆炸/首领的
//! 槽池。asm 注释把 16 个「16 字节槽位」串起来数，但**真实结构不是 16×16**：
//!
//! | 地址 | asm | 槽 × 长 | 语义 |
//! |---|---|---|---|
//! | `$0400-$045F` | `Ens instanceof En $6 startfrom 0` | 6 × 16 B | 普通敌（slot 0-5） |
//! | `$0460-$04BF` | `EnProjectiles instanceof En $6 startfrom 0` | 6 × 16 B | 敌弹（slot 6-B，同 `En` 布局） |
//! | `$04C0-$04DF` | `EnExplosions instanceof EnExplosion $4` | 4 × 8 B | 爆散（`EnExplosion` 只有 8 B） |
//! | `$04E0-$04EF` | `EnMotherBrain instanceof En` | 1 × 16 B | Mother Brain 本体 |
//! | `$04F0-$04FF` | `EnMellowHandler instanceof En` | 1 × 16 B | Mellow 处理器用于收尾的敌槽 |
//!
//! 地址锚（A 级）：`refs/m1disasm/out/M1_NES_NTSC.sym`——`Ens.0 = $0400`（:11111）、
//! `EnExplosions = $04C0`（:11340）、`EnMotherBrain = $04E0`（:11369）、
//! `EnMellowHandler = $04F0`（:11388）；ROM 侧直接以 `lda #EnMotherBrain - Ens`
//! （`refs/m1disasm/SRC/prg3_tourian.asm:761`）取「槽 E 的字节偏移 `$E0`」，与 16 字节步长同源。
//!
//! 【为什么这一面此前不是 `Owner`】行为**都已移植**（`crates/game-core/src/enemy/` 一整套），
//! 但字段布局只活在字段偏移常量里：`crates/game-core/src/enemy/state.rs:20-45` 的 `ens::*`
//! （`Ens` 的 16 个偏移，含 `$0406` 一字节三名）与 `crates/game-core/src/enemy/explosion.rs:40-48`
//! 的 `explx::*`（8 B 爆炸槽）。这两处是「域视图」，不是 typed 声明面：`Enemy::SPANS`
//! 把主面整脸登记成 `Unmodeled`（`enemy/state.rs:956-961`），`Explosion::SPANS` 同样
//! （`enemy/explosion.rs:307-311`）——审计因此只看到「有声明但未 typed」。
//!
//! 【本模块只做两件事】①把 asm 结构升格成 typed 表示（字段名/类型/偏移，逐字段带 asm 锚）；
//! ②把该面登记进 [`crate::memmap::nesmap::entity_spans`]。**行为零变化**：业务层仍是唯一
//! 读写路径；[`ObjectSlotFace::project`] 是同一批字节的编码出口，[`ObjectSlotFace::decode`]
//! 是解码入口，往返互检进单测。
//!
//! 【声明层为什么落在 `memmap/`】与 B1-a（`memmap/tile_blast_face.rs`）、B1-b
//! （`memmap/room_sprite_table_face.rs`）同因：`memmap/mod.rs` 的规定是**地址常量与 codec
//! 只活在本层**，本模块是该规定下的「槽语义视图 + 字节偏移契约面」。
//!
//! 【P4 迁移方向（`docs/design/fc-memory-projection-architecture.md` §4.1）】该族转为常驻
//! typed 真源时：`enemy::state::Enemy` / `enemy::explosion::Explosion` 的字节访问改委托本结构、
//! `storage` 不再持有这 256 B。在那之前本结构是**按需解码**，不持有跨帧状态——这也是
//! `Owner` 在本层的判据：这批字节的权威**解释**（字段布局）只有这里一份，字节真源仍是 `storage`。
//!
//! 【`$0406` 一字节三名的纪律】asm 在 `$0406` 叠了三个名字：`chunkyExplosionTimer`（爆散计时）/
//! `movementInstrIndex`（移动指令指针）/ `speedSubPixelY`（竖速小数），三处代码各按自己的语义用同一格。
//! 本模块**不**给它起一个「猜含义」的名字：字段名用中性的 `data06`，三个 asm 名并列写进
//! [`ens::DATA06_ALIASES`]，与 `enemy::state::ens::DATA06_ALIASES` 逐字相同。同理
//! `specialAttribs` 的 bit7/bit6 是「强化版/小 boss」标志，位掩码以常量给出、不引入状态机。
//!
//! 【未赋义字节的处理】asm 声明但本模块不解释的 `EnExplosion.unused ds 3` 按原字节 round-trip，
//! 不赋予猜测语义。
//!
//! 【与业务层常量的同步】本模块的偏移常量与 `enemy/state.rs::ens`、`enemy/explosion.rs::explx`
//! 是**两份**：那份是业务读径的常量，本模块是声明面的契约。单测
//! `addresses_match_business_layer_literals` / `field_offsets_match_asm_struct` 把两边钉在
//! 同一组数字上，任一处漂移都会红。

use crate::memmap::nesmap::{NesMapped, NesSpan, SpanKey};
use crate::memmap::storage::NesImage;

// ─────────────────────────────── 面几何 ───────────────────────────────

/// 面基址（`constants_ram.asm:963` 的 `.enum $0400`）。
pub const BASE: u16 = crate::enemy::state::ENS_BASE;
/// 面长（`$0400-$04FF` = 256 B）。
pub const FACE_LEN: u16 = 0x100;

/// `En` 槽长（16 B；`Ens`/`EnProjectiles`/`EnMotherBrain`/`EnMellowHandler` 同构）。
pub const EN_SLOT_LEN: u16 = 16;
/// 敌对象槽数（`Ens` 6 + `EnProjectiles` 6 = slot 0-B）。
pub const EN_SLOT_COUNT: usize = 12;
/// 敌对象槽组起点（`Ens.0 = $0400`，sym:11111）。
pub const EN_BASE: u16 = crate::enemy::state::ENS_BASE;

/// `EnExplosion` 槽长（8 B，**不是** 16）。
pub const EXPLOSION_SLOT_LEN: u16 = 8;
/// 爆散槽数（`instanceof EnExplosion $4`）。
pub const EXPLOSION_SLOT_COUNT: usize = 4;
/// 爆散槽组起点（`EnExplosions = $04C0`，sym:11340）。
pub const EXPLOSIONS_BASE: u16 = crate::enemy::explosion::BASE;

/// Mother Brain 槽起点（`EnMotherBrain = $04E0`，sym:11369；整槽 `En`）。
pub const MOTHER_BRAIN_BASE: u16 = 0x04E0;
/// Mellow handler 槽起点（`EnMellowHandler = $04F0`，sym:11388；整槽 `En`）。
pub const MELLOW_HANDLER_BASE: u16 = 0x04F0;

// ─────────────────────────── `En` 字段偏移 ───────────────────────────

/// `En` 结构体的字段偏移（`$0400+i*16+f`，`constants_ram.asm:85-121`）。
///
/// 与 `crate::enemy::state::ens` 逐字相同（单测 `field_offsets_match_asm_struct` 钉住）。
pub mod ens {
    pub const Y: u16 = 0;
    pub const X: u16 = 1;
    pub const SPEED_Y: u16 = 2;
    pub const SPEED_X: u16 = 3;
    pub const IS_HIT: u16 = 4;
    /// bit0 朝向 / bit1 可见 / bit2 朝向（上下）/ bit3 休息中可被 `EnDelay` 激活 /
    /// bit4 Samus 在阈值内 / bit5 触发休息 / bit6 每帧翻转（30FPS 分频）/ bit7 bit2 作朝向。
    pub const DATA05: u16 = 5;
    /// **`$0406` 一字节三名**：`chunkyExplosionTimer` / `movementInstrIndex` / `speedSubPixelY`。
    pub const DATA06: u16 = 6;
    pub const SPEED_SUBPIXEL_X: u16 = 7;
    pub const MOVEMENT_INDEX: u16 = 8;
    pub const DELAY: u16 = 9;
    pub const DATA0A: u16 = 10;
    pub const HEALTH: u16 = 11;
    /// 受伤前的状态字节（bit7/bit6 即 `EnSpecialAttribs` 的高两位）。
    pub const PREV_STATUS: u16 = 12;
    pub const DATA0D: u16 = 13;
    pub const WEAPON_ACTION: u16 = 14;
    /// bit7 = 强化版（tough），bit6 = 小 boss；受伤时也用作硬直计时。
    pub const SPECIAL_ATTRIBS: u16 = 15;

    /// `$0406` 的三个 asm 名——**只登记不合并**（同 `enemy::state::ens::DATA06_ALIASES`）。
    pub const DATA06_ALIASES: [&str; 3] =
        ["chunkyExplosionTimer", "movementInstrIndex", "speedSubPixelY"];

    /// `data05` 的位掩码（`constants_ram.asm:93-110` 逐条对应）。
    pub const DATA05_FACING_LEFT: u8 = 0x01;
    pub const DATA05_VISIBLE: u8 = 0x02;
    pub const DATA05_FACING_UP: u8 = 0x04;
    pub const DATA05_ACTIVATES_ON_DELAY: u8 = 0x08;
    pub const DATA05_SAMUS_CLOSE: u8 = 0x10;
    pub const DATA05_REST_TRIGGER: u8 = 0x20;
    pub const DATA05_FRAME_TOGGLE: u8 = 0x40;
    pub const DATA05_BIT2_AS_FACING: u8 = 0x80;

    /// `specialAttribs` 的位掩码（`constants_ram.asm:120-121`）。
    pub const SPECIAL_TOUGH: u8 = 0x80;
    pub const SPECIAL_MINIBOSS: u8 = 0x40;
}

/// `EnExplosion` 结构体的字段偏移（`$04C0+i*8+f`，`constants_ram.asm:152-159`）。
pub mod expl {
    pub const Y: u16 = 0;
    pub const X: u16 = 1;
    /// `unused ds 3`（3 B，原字节 round-trip）。
    pub const UNUSED: u16 = 2;
    pub const UNUSED_LEN: u16 = 3;
    /// bit1 = `IsObjectVisible`（与 `En.data05` 的 bit1 同义）。
    pub const DATA05: u16 = 5;
    pub const DELAY: u16 = 6;
    pub const QUANTITY: u16 = 7;

    /// `data05` 的位掩码（asm 只登记 bit1）。
    pub const DATA05_VISIBLE: u8 = 0x02;
}

// ─────────────────────────────── 槽结构 ───────────────────────────────

/// 一个 `En` 槽（16 B）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EnSlot {
    pub y: u8,
    pub x: u8,
    pub speed_y: u8,
    pub speed_x: u8,
    pub is_hit: u8,
    pub data05: u8,
    /// `$0406`——三名同格，见 [`ens::DATA06_ALIASES`]。
    pub data06: u8,
    pub speed_subpixel_x: u8,
    pub movement_index: u8,
    pub delay: u8,
    pub data0a: u8,
    pub health: u8,
    pub prev_status: u8,
    pub data0d: u8,
    pub weapon_action: u8,
    pub special_attribs: u8,
}

impl EnSlot {
    /// 从槽字节解码（16 B）。
    pub fn decode(bytes: &[u8]) -> Self {
        Self {
            y: bytes[usize::from(ens::Y)],
            x: bytes[usize::from(ens::X)],
            speed_y: bytes[usize::from(ens::SPEED_Y)],
            speed_x: bytes[usize::from(ens::SPEED_X)],
            is_hit: bytes[usize::from(ens::IS_HIT)],
            data05: bytes[usize::from(ens::DATA05)],
            data06: bytes[usize::from(ens::DATA06)],
            speed_subpixel_x: bytes[usize::from(ens::SPEED_SUBPIXEL_X)],
            movement_index: bytes[usize::from(ens::MOVEMENT_INDEX)],
            delay: bytes[usize::from(ens::DELAY)],
            data0a: bytes[usize::from(ens::DATA0A)],
            health: bytes[usize::from(ens::HEALTH)],
            prev_status: bytes[usize::from(ens::PREV_STATUS)],
            data0d: bytes[usize::from(ens::DATA0D)],
            weapon_action: bytes[usize::from(ens::WEAPON_ACTION)],
            special_attribs: bytes[usize::from(ens::SPECIAL_ATTRIBS)],
        }
    }

    /// 编码回槽字节（16 B）。
    pub fn encode(&self, bytes: &mut [u8]) {
        bytes[usize::from(ens::Y)] = self.y;
        bytes[usize::from(ens::X)] = self.x;
        bytes[usize::from(ens::SPEED_Y)] = self.speed_y;
        bytes[usize::from(ens::SPEED_X)] = self.speed_x;
        bytes[usize::from(ens::IS_HIT)] = self.is_hit;
        bytes[usize::from(ens::DATA05)] = self.data05;
        bytes[usize::from(ens::DATA06)] = self.data06;
        bytes[usize::from(ens::SPEED_SUBPIXEL_X)] = self.speed_subpixel_x;
        bytes[usize::from(ens::MOVEMENT_INDEX)] = self.movement_index;
        bytes[usize::from(ens::DELAY)] = self.delay;
        bytes[usize::from(ens::DATA0A)] = self.data0a;
        bytes[usize::from(ens::HEALTH)] = self.health;
        bytes[usize::from(ens::PREV_STATUS)] = self.prev_status;
        bytes[usize::from(ens::DATA0D)] = self.data0d;
        bytes[usize::from(ens::WEAPON_ACTION)] = self.weapon_action;
        bytes[usize::from(ens::SPECIAL_ATTRIBS)] = self.special_attribs;
    }

    /// `data05` 的某一位是否置位（掩码取 [`ens`] 的 `DATA05_*`）。
    pub fn data05_bit(&self, mask: u8) -> bool {
        self.data05 & mask != 0
    }

    /// 置/清 `data05` 的某一位，返回旧值。
    pub fn set_data05_bit(&mut self, mask: u8, on: bool) -> bool {
        let old = self.data05_bit(mask);
        if on {
            self.data05 |= mask;
        } else {
            self.data05 &= !mask;
        }
        old
    }

    /// `specialAttribs` 的某一位是否置位（[`ens::SPECIAL_TOUGH`] / [`ens::SPECIAL_MINIBOSS`]）。
    pub fn special_bit(&self, mask: u8) -> bool {
        self.special_attribs & mask != 0
    }

    /// 置/清 `specialAttribs` 的某一位，返回旧值。
    pub fn set_special_bit(&mut self, mask: u8, on: bool) -> bool {
        let old = self.special_bit(mask);
        if on {
            self.special_attribs |= mask;
        } else {
            self.special_attribs &= !mask;
        }
        old
    }
}

/// 一个 `EnExplosion` 槽（8 B）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ExplosionSlot {
    pub y: u8,
    pub x: u8,
    /// `unused ds 3`——原字节 round-trip，不赋义。
    pub unused: [u8; expl::UNUSED_LEN as usize],
    pub data05: u8,
    pub delay: u8,
    pub quantity: u8,
}

impl ExplosionSlot {
    /// 从槽字节解码（8 B）。
    pub fn decode(bytes: &[u8]) -> Self {
        let at = usize::from(expl::UNUSED);
        let mut unused = [0u8; expl::UNUSED_LEN as usize];
        unused.copy_from_slice(&bytes[at..at + expl::UNUSED_LEN as usize]);
        Self {
            y: bytes[usize::from(expl::Y)],
            x: bytes[usize::from(expl::X)],
            unused,
            data05: bytes[usize::from(expl::DATA05)],
            delay: bytes[usize::from(expl::DELAY)],
            quantity: bytes[usize::from(expl::QUANTITY)],
        }
    }

    /// 编码回槽字节（8 B）。
    pub fn encode(&self, bytes: &mut [u8]) {
        bytes[usize::from(expl::Y)] = self.y;
        bytes[usize::from(expl::X)] = self.x;
        let at = usize::from(expl::UNUSED);
        bytes[at..at + expl::UNUSED_LEN as usize].copy_from_slice(&self.unused);
        bytes[usize::from(expl::DATA05)] = self.data05;
        bytes[usize::from(expl::DELAY)] = self.delay;
        bytes[usize::from(expl::QUANTITY)] = self.quantity;
    }

    /// `data05` 的 bit1（`IsObjectVisible`）是否置位。
    pub fn is_visible(&self) -> bool {
        self.data05 & expl::DATA05_VISIBLE != 0
    }
}

// ─────────────────────────────── 整面 ───────────────────────────────

/// 对象槽整面（`$0400-$04FF`，256 B）的 typed 表示。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ObjectSlotFace {
    en: [EnSlot; EN_SLOT_COUNT],
    explosions: [ExplosionSlot; EXPLOSION_SLOT_COUNT],
    mother_brain: EnSlot,
    mellow_handler: EnSlot,
}

impl ObjectSlotFace {
    /// 从 2KB RAM 镜像面解码（地址按 `& 0x7FF` 归一）。
    pub fn decode(ram: &[u8]) -> Self {
        let at = usize::from(BASE & 0x7FF);
        let mut en = [EnSlot::default(); EN_SLOT_COUNT];
        for (i, slot) in en.iter_mut().enumerate() {
            let base = at + usize::from(EN_BASE - BASE) + i * usize::from(EN_SLOT_LEN);
            *slot = EnSlot::decode(&ram[base..base + usize::from(EN_SLOT_LEN)]);
        }
        let mut explosions = [ExplosionSlot::default(); EXPLOSION_SLOT_COUNT];
        for (i, slot) in explosions.iter_mut().enumerate() {
            let base =
                at + usize::from(EXPLOSIONS_BASE - BASE) + i * usize::from(EXPLOSION_SLOT_LEN);
            *slot = ExplosionSlot::decode(&ram[base..base + usize::from(EXPLOSION_SLOT_LEN)]);
        }
        let mb_at = at + usize::from(MOTHER_BRAIN_BASE - BASE);
        let mellow_at = at + usize::from(MELLOW_HANDLER_BASE - BASE);
        Self {
            en,
            explosions,
            mother_brain: EnSlot::decode(&ram[mb_at..mb_at + usize::from(EN_SLOT_LEN)]),
            mellow_handler: EnSlot::decode(&ram[mellow_at..mellow_at + usize::from(EN_SLOT_LEN)]),
        }
    }

    /// 编码回 2KB RAM 镜像面（只写 `$0400-$04FF`）。
    pub fn encode(&self, ram: &mut [u8]) {
        let at = usize::from(BASE & 0x7FF);
        for (i, slot) in self.en.iter().enumerate() {
            let base = at + usize::from(EN_BASE - BASE) + i * usize::from(EN_SLOT_LEN);
            slot.encode(&mut ram[base..base + usize::from(EN_SLOT_LEN)]);
        }
        for (i, slot) in self.explosions.iter().enumerate() {
            let base =
                at + usize::from(EXPLOSIONS_BASE - BASE) + i * usize::from(EXPLOSION_SLOT_LEN);
            slot.encode(&mut ram[base..base + usize::from(EXPLOSION_SLOT_LEN)]);
        }
        let mb_at = at + usize::from(MOTHER_BRAIN_BASE - BASE);
        self.mother_brain
            .encode(&mut ram[mb_at..mb_at + usize::from(EN_SLOT_LEN)]);
        let mellow_at = at + usize::from(MELLOW_HANDLER_BASE - BASE);
        self.mellow_handler
            .encode(&mut ram[mellow_at..mellow_at + usize::from(EN_SLOT_LEN)]);
    }

    /// 地址是否落在本面（`$0400-$04FF`）。
    pub fn contains(addr: u16) -> bool {
        (BASE..BASE + FACE_LEN).contains(&(addr & 0x7FF))
    }

    /// 敌对象槽视图（slot 0-B，越界返回 `None`）。
    pub fn en(&self, index: usize) -> Option<&EnSlot> {
        self.en.get(index)
    }

    /// 敌对象槽可变视图（越界返回 `None`）。
    pub fn en_mut(&mut self, index: usize) -> Option<&mut EnSlot> {
        self.en.get_mut(index)
    }

    /// 爆散槽视图（slot 0-3，越界返回 `None`）。
    pub fn explosion(&self, index: usize) -> Option<&ExplosionSlot> {
        self.explosions.get(index)
    }

    /// 爆散槽可变视图（越界返回 `None`）。
    pub fn explosion_mut(&mut self, index: usize) -> Option<&mut ExplosionSlot> {
        self.explosions.get_mut(index)
    }

    /// Mother Brain 槽视图（`$04E0`，整槽 `En`）。
    pub fn mother_brain(&self) -> &EnSlot {
        &self.mother_brain
    }

    /// Mellow handler 槽视图（`$04F0`，整槽 `En`）。
    pub fn mellow_handler(&self) -> &EnSlot {
        &self.mellow_handler
    }

    /// 敌对象槽的槽首地址（slot 0-B），越界返回 `None`。
    pub fn en_addr(index: usize) -> Option<u16> {
        if index < EN_SLOT_COUNT {
            Some(EN_BASE + index as u16 * EN_SLOT_LEN)
        } else {
            None
        }
    }

    /// 爆散槽的槽首地址（slot 0-3），越界返回 `None`。
    pub fn explosion_addr(index: usize) -> Option<u16> {
        if index < EXPLOSION_SLOT_COUNT {
            Some(EXPLOSIONS_BASE + index as u16 * EXPLOSION_SLOT_LEN)
        } else {
            None
        }
    }

    /// 地址 → 字节（不在本面返回 `None`）。
    pub fn get_byte(&self, addr: u16) -> Option<u8> {
        let a = addr & 0x7FF;
        if !Self::contains(a) {
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
        if !Self::contains(a) {
            return false;
        }
        let mut bytes = [0u8; 0x800];
        self.encode(&mut bytes);
        bytes[usize::from(BASE & 0x7FF) + usize::from(a - BASE)] = v;
        *self = Self::decode(&bytes);
        true
    }
}

// ─────────────────────────── P2 声明（域级） ───────────────────────────

impl NesMapped for ObjectSlotFace {
    const DISCRIMINATOR: Option<u16> = None;

    /// 18 段 `Owner`：12（`Ens`+`EnProjectiles` 同构 16 B 槽）+ 4（`EnExplosion` 8 B 槽）
    /// + 1（`EnMotherBrain` 16 B）+ 1（`EnMellowHandler` 16 B），连续覆盖 `$0400-$0500`。
    ///
    /// 逐槽一段（同 B1-a/B1-b）：槽是 asm 的数组元素，槽内字段不拆段——拆开只会让
    /// `owner_spans` 噪声化，不增加可审计信息。手写而非循环生成：`SPANS` 是 `const`，
    /// 且 `--write` 的「既有 Owner span 不得变形」判据按 `(start,len)` 精确比对。
    const SPANS: &'static [(NesSpan, SpanKey)] = &[
        (NesSpan::value(EN_BASE, EN_SLOT_LEN), SpanKey::Owner),
        (
            NesSpan::value(EN_BASE + EN_SLOT_LEN, EN_SLOT_LEN),
            SpanKey::Owner,
        ),
        (
            NesSpan::value(EN_BASE + 2 * EN_SLOT_LEN, EN_SLOT_LEN),
            SpanKey::Owner,
        ),
        (
            NesSpan::value(EN_BASE + 3 * EN_SLOT_LEN, EN_SLOT_LEN),
            SpanKey::Owner,
        ),
        (
            NesSpan::value(EN_BASE + 4 * EN_SLOT_LEN, EN_SLOT_LEN),
            SpanKey::Owner,
        ),
        (
            NesSpan::value(EN_BASE + 5 * EN_SLOT_LEN, EN_SLOT_LEN),
            SpanKey::Owner,
        ),
        (
            NesSpan::value(EN_BASE + 6 * EN_SLOT_LEN, EN_SLOT_LEN),
            SpanKey::Owner,
        ),
        (
            NesSpan::value(EN_BASE + 7 * EN_SLOT_LEN, EN_SLOT_LEN),
            SpanKey::Owner,
        ),
        (
            NesSpan::value(EN_BASE + 8 * EN_SLOT_LEN, EN_SLOT_LEN),
            SpanKey::Owner,
        ),
        (
            NesSpan::value(EN_BASE + 9 * EN_SLOT_LEN, EN_SLOT_LEN),
            SpanKey::Owner,
        ),
        (
            NesSpan::value(EN_BASE + 10 * EN_SLOT_LEN, EN_SLOT_LEN),
            SpanKey::Owner,
        ),
        (
            NesSpan::value(EN_BASE + 11 * EN_SLOT_LEN, EN_SLOT_LEN),
            SpanKey::Owner,
        ),
        (
            NesSpan::value(EXPLOSIONS_BASE, EXPLOSION_SLOT_LEN),
            SpanKey::Owner,
        ),
        (
            NesSpan::value(EXPLOSIONS_BASE + EXPLOSION_SLOT_LEN, EXPLOSION_SLOT_LEN),
            SpanKey::Owner,
        ),
        (
            NesSpan::value(EXPLOSIONS_BASE + 2 * EXPLOSION_SLOT_LEN, EXPLOSION_SLOT_LEN),
            SpanKey::Owner,
        ),
        (
            NesSpan::value(EXPLOSIONS_BASE + 3 * EXPLOSION_SLOT_LEN, EXPLOSION_SLOT_LEN),
            SpanKey::Owner,
        ),
        (NesSpan::value(MOTHER_BRAIN_BASE, EN_SLOT_LEN), SpanKey::Owner),
        (
            NesSpan::value(MELLOW_HANDLER_BASE, EN_SLOT_LEN),
            SpanKey::Owner,
        ),
    ];

    /// 只写本面 256 B。
    fn project(&self, out: &mut NesImage) {
        self.encode(&mut out.ram);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn face_bytes() -> [u8; FACE_LEN as usize] {
        let mut bytes = [0u8; FACE_LEN as usize];
        for (i, b) in bytes.iter_mut().enumerate() {
            *b = (i as u8).wrapping_mul(37).wrapping_add(11);
        }
        bytes
    }

    fn ram_with_face() -> [u8; 0x800] {
        let mut ram = [0x3Cu8; 0x800];
        let at = usize::from(BASE & 0x7FF);
        ram[at..at + FACE_LEN as usize].copy_from_slice(&face_bytes());
        ram
    }

    #[test]
    fn face_geometry_matches_asm_enum() {
        // `constants_ram.asm:963-978` + sym 锚：12×16 + 4×8 + 1×16 + 1×16 = 256。
        assert_eq!(
            EN_SLOT_COUNT as u16 * EN_SLOT_LEN
                + EXPLOSION_SLOT_COUNT as u16 * EXPLOSION_SLOT_LEN
                + 2 * EN_SLOT_LEN,
            FACE_LEN
        );
        assert_eq!(EN_BASE + EN_SLOT_COUNT as u16 * EN_SLOT_LEN, EXPLOSIONS_BASE);
        assert_eq!(
            EXPLOSIONS_BASE + EXPLOSION_SLOT_COUNT as u16 * EXPLOSION_SLOT_LEN,
            MOTHER_BRAIN_BASE
        );
        assert_eq!(MOTHER_BRAIN_BASE + EN_SLOT_LEN, MELLOW_HANDLER_BASE);
        assert_eq!(MELLOW_HANDLER_BASE + EN_SLOT_LEN, BASE + FACE_LEN);
    }

    #[test]
    fn spans_cover_the_face_contiguously() {
        assert_eq!(ObjectSlotFace::SPANS.len(), 18);
        let mut cursor = BASE;
        for (span, key) in ObjectSlotFace::SPANS {
            assert_eq!(span.start, cursor, "span 必须连续：起点 {}", span.start);
            assert!(matches!(key, SpanKey::Owner));
            cursor += span.len;
        }
        assert_eq!(cursor, BASE + FACE_LEN);
    }

    #[test]
    fn field_offsets_match_asm_struct() {
        // `constants_ram.asm:85-121`（`En`）逐字段对齐；`$0406` 一名三义。
        let expect_en: [u16; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
        let got_en = [
            ens::Y,
            ens::X,
            ens::SPEED_Y,
            ens::SPEED_X,
            ens::IS_HIT,
            ens::DATA05,
            ens::DATA06,
            ens::SPEED_SUBPIXEL_X,
            ens::MOVEMENT_INDEX,
            ens::DELAY,
            ens::DATA0A,
            ens::HEALTH,
            ens::PREV_STATUS,
            ens::DATA0D,
            ens::WEAPON_ACTION,
            ens::SPECIAL_ATTRIBS,
        ];
        assert_eq!(got_en, expect_en);
        assert_eq!(ens::DATA06_ALIASES.len(), 3);
        // `constants_ram.asm:152-159`（`EnExplosion`，8 B）。
        assert_eq!(
            [expl::Y, expl::X, expl::UNUSED, expl::DATA05, expl::DELAY, expl::QUANTITY],
            [0, 1, 2, 5, 6, 7]
        );
        assert_eq!(expl::UNUSED + expl::UNUSED_LEN, expl::DATA05);
        assert_eq!(expl::QUANTITY + 1, EXPLOSION_SLOT_LEN);
    }

    #[test]
    fn slot_boundaries_decode_independently() {
        let face = ObjectSlotFace::decode(&ram_with_face());
        // slot 0 与 slot 11 的 y 字节分别是面内偏移 0 与 11*16。
        let raw = face_bytes();
        assert_eq!(face.en(0).unwrap().y, raw[0]);
        assert_eq!(face.en(11).unwrap().y, raw[11 * 16]);
        assert_eq!(
            face.en(0).unwrap().special_attribs,
            raw[15],
            "slot0 末字节 = 偏移 15"
        );
        assert_eq!(
            face.en(11).unwrap().special_attribs,
            raw[11 * 16 + 15],
            "slot11 末字节 = 偏移 191"
        );
        assert_eq!(ObjectSlotFace::en_addr(0), Some(BASE));
        assert_eq!(ObjectSlotFace::en_addr(11), Some(0x04B0));
        assert_eq!(ObjectSlotFace::en_addr(12), None, "槽号 12 越界（只有 0-B）");
    }

    #[test]
    fn explosion_boundaries_decode_independently() {
        let face = ObjectSlotFace::decode(&ram_with_face());
        let raw = face_bytes();
        let e0 = usize::from(EXPLOSIONS_BASE - BASE);
        assert_eq!(face.explosion(0).unwrap().y, raw[e0]);
        assert_eq!(face.explosion(3).unwrap().quantity, raw[e0 + 3 * 8 + 7]);
        assert_eq!(ObjectSlotFace::explosion_addr(0), Some(EXPLOSIONS_BASE));
        assert_eq!(ObjectSlotFace::explosion_addr(3), Some(0x04D8));
        assert_eq!(
            ObjectSlotFace::explosion_addr(4),
            None,
            "爆散只有 4 槽（4×8B）"
        );
    }

    #[test]
    fn mother_brain_and_mellow_are_full_en_slots() {
        let face = ObjectSlotFace::decode(&ram_with_face());
        let raw = face_bytes();
        let mb = usize::from(MOTHER_BRAIN_BASE - BASE);
        let mw = usize::from(MELLOW_HANDLER_BASE - BASE);
        assert_eq!(face.mother_brain().y, raw[mb]);
        assert_eq!(face.mother_brain().special_attribs, raw[mb + 15]);
        assert_eq!(face.mellow_handler().y, raw[mw]);
        assert_eq!(face.mellow_handler().special_attribs, raw[mw + 15]);
    }

    #[test]
    fn data05_bit_round_trip() {
        let mut slot = EnSlot::default();
        for (mask, _name) in [
            (ens::DATA05_FACING_LEFT, "facing_left"),
            (ens::DATA05_VISIBLE, "visible"),
            (ens::DATA05_FACING_UP, "facing_up"),
            (ens::DATA05_ACTIVATES_ON_DELAY, "activates_on_delay"),
            (ens::DATA05_SAMUS_CLOSE, "samus_close"),
            (ens::DATA05_REST_TRIGGER, "rest_trigger"),
            (ens::DATA05_FRAME_TOGGLE, "frame_toggle"),
            (ens::DATA05_BIT2_AS_FACING, "bit2_as_facing"),
        ] {
            assert!(!slot.data05_bit(mask), "初值必须为 0：{mask:#04X}");
            assert!(!slot.set_data05_bit(mask, true), "置位返回旧值 false");
            assert!(slot.data05_bit(mask));
            assert!(slot.set_data05_bit(mask, false), "清位返回旧值 true");
            assert!(!slot.data05_bit(mask));
        }
        assert_eq!(slot.data05, 0, "全部清位后字节回 0");
    }

    #[test]
    fn special_attribs_bit_round_trip() {
        let mut slot = EnSlot::default();
        assert!(!slot.set_special_bit(ens::SPECIAL_TOUGH, true));
        assert_eq!(slot.special_attribs, 0x80);
        assert!(!slot.set_special_bit(ens::SPECIAL_MINIBOSS, true));
        assert_eq!(slot.special_attribs, 0xC0);
        assert!(slot.special_bit(ens::SPECIAL_TOUGH) && slot.special_bit(ens::SPECIAL_MINIBOSS));
        assert!(slot.set_special_bit(ens::SPECIAL_TOUGH, false));
        assert_eq!(slot.special_attribs, 0x40);
    }

    #[test]
    fn face_round_trip_is_byte_exact() {
        // 固定图案 + 全 0 + 全 FF + 4 组 LCG。
        let mut samples: Vec<[u8; FACE_LEN as usize]> = vec![
            face_bytes(),
            [0u8; FACE_LEN as usize],
            [0xFFu8; FACE_LEN as usize],
        ];
        for seed in [1u32, 7, 12345, 0xDEAD_BEEF] {
            let mut s = seed;
            let mut arr = [0u8; FACE_LEN as usize];
            for b in arr.iter_mut() {
                s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                *b = (s >> 24) as u8;
            }
            samples.push(arr);
        }
        for sample in samples {
            let mut ram = [0x3Cu8; 0x800];
            let at = usize::from(BASE & 0x7FF);
            ram[at..at + FACE_LEN as usize].copy_from_slice(&sample);
            let face = ObjectSlotFace::decode(&ram);
            let mut out = [0x3Cu8; 0x800];
            face.encode(&mut out);
            assert_eq!(
                &out[at..at + FACE_LEN as usize],
                &sample[..],
                "round-trip 必须逐字节相等"
            );
        }
    }

    #[test]
    fn encode_touches_only_the_face() {
        let mut ram = [0x3Cu8; 0x800];
        let face = ObjectSlotFace::decode(&ram_with_face());
        face.encode(&mut ram);
        let at = usize::from(BASE & 0x7FF);
        assert!(
            ram[..at].iter().all(|b| *b == 0x3C),
            "面之前字节不得被写"
        );
        assert!(
            ram[at + FACE_LEN as usize..].iter().all(|b| *b == 0x3C),
            "面之后字节不得被写"
        );
    }

    #[test]
    fn byte_accessors_cover_exactly_the_face() {
        let face = ObjectSlotFace::decode(&ram_with_face());
        for addr in BASE..BASE + FACE_LEN {
            assert!(face.get_byte(addr).is_some(), "{addr:#06X} 应可读");
        }
        assert!(face.get_byte(BASE - 1).is_none(), "$03FF 不属本面");
        assert!(face.get_byte(BASE + FACE_LEN).is_none(), "$0500 不属本面");

        let mut mutable = ObjectSlotFace::default();
        let mut touched = 0usize;
        for addr in BASE..BASE + FACE_LEN {
            assert!(mutable.set_byte(addr, 0xA5), "{addr:#06X} 应可写");
            touched += 1;
        }
        assert_eq!(touched, usize::from(FACE_LEN));
        assert!(!mutable.set_byte(BASE + FACE_LEN, 0xFF), "$0500 拒绝写入");
        // 全 A5 后按字段读回应一致。
        assert_eq!(mutable.en(0).unwrap().y, 0xA5);
        assert_eq!(mutable.en(11).unwrap().special_attribs, 0xA5);
        assert_eq!(mutable.explosion(3).unwrap().quantity, 0xA5);
        assert_eq!(mutable.mellow_handler().x, 0xA5);
    }

    #[test]
    fn addresses_match_business_layer_literals() {
        // 业务侧地址常量（`enemy/state.rs::ENS_BASE`、`slot*16`；
        // `enemy/explosion.rs::ens_offset` = 0xC0 + slot*8 ⇒ $04C0 起）。
        assert_eq!(ObjectSlotFace::en_addr(0), Some(BASE));
        assert_eq!(ObjectSlotFace::en_addr(1), Some(EN_BASE + 0x10));
        assert_eq!(ObjectSlotFace::explosion_addr(0), Some(BASE + 0xC0));
        assert_eq!(ObjectSlotFace::explosion_addr(1), Some(BASE + 0xC8));
        // sym 锚（refs/m1disasm/out/M1_NES_NTSC.sym）。
        assert_eq!(EN_BASE, BASE);
        assert_eq!(EXPLOSIONS_BASE, BASE + 0xC0);
        assert_eq!(MOTHER_BRAIN_BASE, 0x04E0);
        assert_eq!(MELLOW_HANDLER_BASE, 0x04F0);
    }

    #[test]
    fn project_writes_only_this_face() {
        let face = ObjectSlotFace::decode(&ram_with_face());
        let mut img = NesImage {
            ram: [0x3Cu8; 0x800],
            wram: [0u8; 0x2000],
        };
        face.project(&mut img);
        let at = usize::from(BASE & 0x7FF);
        assert_eq!(&img.ram[at..at + FACE_LEN as usize], &face_bytes()[..]);
        assert!(img.ram[..at].iter().all(|b| *b == 0x3C));
        assert!(img.ram[at + FACE_LEN as usize..].iter().all(|b| *b == 0x3C));
        assert!(img.wram.iter().all(|b| *b == 0));
    }
}
