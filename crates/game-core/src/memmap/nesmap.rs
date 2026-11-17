//! `NesMapped` —— 实体自报「我的字段落在哪些 NES 字节」（P2 声明层）。
//!
//! 定位（`docs/spec/entity-ownership-and-nesmap.md` §3/§4）：
//! - 本模块只做**声明**与**核算**，不驱动任何读写；`read_ram/write_ram/snapshot_nes/
//!   restore_mss` 的语义一字不改（P2 的硬边界：零行为变化）。
//! - 声明是**现存事实的投影**：地址全部取自各域既有常量（`ram_map` / `ens*` / `obj` /
//!   `room_ram` / 相机常量 / 爆炸池常量），本文件零地址字面量（`no-raw-addr` 门禁）。
//! - 判据（§3 D-03）：`$0000-$07FF ∪ $6AF4-$6BD3` 每字节**恰一个 owner**；其他解释
//!   （`View`/`Case`/`Mode`/`Ephemeral`/`Unmodeled`）不得抢 owner 位。

use crate::memmap::storage::NesImage;

/// RAM 面长度（`$0000-$07FF`；`$0800-$1FFF` 是它的镜像别名，归一后同格）。
pub const RAM_LEN: usize = 0x800;
/// WRAM 面长度（`$6000-$7FFF`）。
pub const WRAM_LEN: usize = 0x2000;
/// WRAM 面基址。
pub const WRAM_BASE: u16 = 0x6000;

/// 一段连续 NES 字节 + 它的**存储形态**（§4 七形态里的"值"面）。
///
/// 形态只描述"这段字节怎么读"，不描述"谁拥有它"——归属由 [`SpanKey`] 表达。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NesSpan {
    /// 段首地址（NES 地址空间口径：RAM 面 `$0000` 起，WRAM 面 `$6000` 起）。
    pub start: u16,
    /// 段长（字节）。`0` 是非法声明，[`coverage`] 会报错。
    pub len: u16,
    /// 存储形态。
    pub kind: SpanKind,
}

impl NesSpan {
    /// 连续字节段（`SpanKind::Value`）——绝大多数声明的形态。
    pub const fn value(start: u16, len: u16) -> Self {
        Self {
            start,
            len,
            kind: SpanKind::Value,
        }
    }

    /// 逐字节地址（`start..start+len`）。
    pub fn addrs(self) -> impl Iterator<Item = u16> {
        (0..self.len).map(move |i| self.start.wrapping_add(i))
    }
}

/// 存储形态（§4 七形态的"值"面）：一段字节**怎么读**。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpanKind {
    /// 普通值（一个或多个连续字节）。
    Value,
    /// 位域（按位解释；括号内是判据说明，例 `"& $41 == $40"`）。
    Bits(&'static str),
    /// BCD 编码（密码系统一类）。
    Bcd(&'static str),
    /// 由其他字节推导（不独立存储；括号内是推导式说明）。
    Derived(&'static str),
}

/// 认领键（§4 七形态的"归属"面）：这段字节**归谁解释**。
///
/// 只有 [`SpanKey::Owner`] 参与覆盖度计位——其余键都是"同一批字节的第二种解释"，
/// 抢 owner 位会让覆盖度门禁在合法改动时报红（§3 D-03：每字节恰一个 owner）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpanKey {
    /// **真相面**：这段字节的权威表示就是本实体的 typed 字段。
    Owner,
    /// **只读视图**：真相在别处（括号内是持有真相的类型名，如 `"SamusState"`）。
    View(&'static [&'static str]),
    /// 同址按**状态**重解释（括号内是判别值；判别字段见 [`NesMapped::DISCRIMINATOR`]）。
    Case(u8),
    /// 同址按**模式**重解释（括号内是模式值）。
    Mode(u8),
    /// 例程临时量（跨调用不保义；括号内是用途说明）。
    Ephemeral(&'static str),
    /// **未 typed 化**（真相暂时只在字节面；括号内是缺口说明）。
    Unmodeled(&'static str),
}

/// 实体自报 NES 字节映射的契约。
///
/// 实现者必须满足：`SPANS` 是**现存事实的投影**（地址取本域既有常量），
/// `project` 与 `snapshot_nes()` 同源同值。P2 只登记与核算，不驱动任何读写。
pub trait NesMapped {
    /// 状态判别字段（同址按状态重解释时的判别源）；无判别字段或判别地址随槽变化时为 `None`。
    const DISCRIMINATOR: Option<u16>;
    /// 声明：字节段 + 认领键。
    const SPANS: &'static [(NesSpan, SpanKey)];
    /// 单向投影：只把本实体**拥有（`Owner`）**的字节写进 `out`。
    ///
    /// 未 typed 的域无事可做（空实现）——它们的真相暂时只在 `storage` 字节面里。
    fn project(&self, out: &mut NesImage);
}

/// 可认领面（两块）。`$0800-$1FFF` 归一进 RAM，`$6000-$7FFF` 归一进 WRAM。
#[derive(Clone, Copy)]
enum Face {
    Ram,
    Wram,
}

/// 地址 → (面, 面内下标)；不在可认领面内返回 `None`（声明错误，不静默归一）。
fn locate(addr: u16) -> Option<(Face, usize)> {
    if addr < (RAM_LEN as u16) * 4 {
        Some((Face::Ram, usize::from(addr) & (RAM_LEN - 1)))
    } else if (WRAM_BASE..WRAM_BASE + WRAM_LEN as u16).contains(&addr) {
        Some((Face::Wram, usize::from(addr - WRAM_BASE)))
    } else {
        None
    }
}

/// 逐字节扫描的三张位图 + 错误行。
struct Maps {
    /// 声明面：有任何键覆盖（`gaps` 用）。
    any_ram: [bool; RAM_LEN],
    any_wram: [bool; WRAM_LEN],
    /// 真相面：被 `Owner` 认领（`coverage`/`unowned_bytes` 用）。
    own_ram: [bool; RAM_LEN],
    own_wram: [bool; WRAM_LEN],
    own_count: [u32; 2],
    errs: Vec<String>,
}

fn scan(spans: &[(NesSpan, SpanKey)]) -> Maps {
    let mut m = Maps {
        any_ram: [false; RAM_LEN],
        any_wram: [false; WRAM_LEN],
        own_ram: [false; RAM_LEN],
        own_wram: [false; WRAM_LEN],
        own_count: [0; 2],
        errs: Vec::new(),
    };
    for (span, key) in spans {
        if span.len == 0 {
            m.errs.push(format!(
                "空段：start={:#06X} len=0（声明不得为空）",
                span.start
            ));
            continue;
        }
        for a in span.addrs() {
            let Some((face, i)) = locate(a) else {
                m.errs.push(format!(
                    "越界认领：{a:#06X} 不在 RAM($0000-$1FFF)/WRAM($6000-$7FFF) 可认领面内"
                ));
                continue;
            };
            match face {
                Face::Ram => m.any_ram[i] = true,
                Face::Wram => m.any_wram[i] = true,
            }
            if !matches!(key, SpanKey::Owner) {
                continue;
            }
            let (bit, slot) = match face {
                Face::Ram => (&mut m.own_ram[i], 0),
                Face::Wram => (&mut m.own_wram[i], 1),
            };
            if *bit {
                m.errs.push(format!("重叠：{a:#06X} 被多个 Owner 认领"));
            } else {
                *bit = true;
                m.own_count[slot] += 1;
            }
        }
    }
    m
}

/// **覆盖度核算**：`[RAM 面 owner 字节数, WRAM 面 owner 字节数]`。
///
/// 只 `Owner` 计位（brief 口径）：`View`/`Ephemeral`/`Unmodeled` 是"同一批字节的
/// 第二种解释"，不计位；`Case`/`Mode` 是**并列**解释（单比特图装不下判别维度），
/// 留给 P3 的判别感知腿（`DISCRIMINATOR`）。
///
/// 返回 `Err` 的情形（逐行，不合并、不静默）：空段、越界、同一字节被两个 `Owner`
/// 认领、计数超 `u8`（brief 签名 `[u8; 2]`——聚合面请分域核算后相加）。
pub fn coverage(spans: &[(NesSpan, SpanKey)]) -> Result<[u8; 2], Vec<String>> {
    let m = scan(spans);
    if !m.errs.is_empty() {
        return Err(m.errs);
    }
    let mut errs = Vec::new();
    for (slot, name) in [(0usize, "RAM"), (1usize, "WRAM")] {
        if m.own_count[slot] > u32::from(u8::MAX) {
            errs.push(format!(
                "{name} 面 owner 字节数 {} 超出 u8（brief 签名 [u8; 2]）——聚合核算请分域跑",
                m.own_count[slot]
            ));
        }
    }
    if !errs.is_empty() {
        return Err(errs);
    }
    Ok([m.own_count[0] as u8, m.own_count[1] as u8])
}

/// **声明空洞**：`domain` 内没有任何 span（任何键）覆盖的字节地址（升序）。
///
/// 判据是"声明面"而非"真相面"：未 typed 的字节也必须以 `Unmodeled` 显式登记，
/// 否则它是**没人解释的字节**——这正是覆盖度门要拦的东西。
pub fn gaps(spans: &[(NesSpan, SpanKey)], domain: &[NesSpan]) -> Vec<u16> {
    let m = scan(spans);
    let mut out = Vec::new();
    for d in domain {
        for a in d.addrs() {
            let covered = match locate(a) {
                Some((Face::Ram, i)) => m.any_ram[i],
                Some((Face::Wram, i)) => m.any_wram[i],
                None => true,
            };
            if !covered {
                out.push(a);
            }
        }
    }
    out
}

/// **typed 债**：`domain` 内没有 `Owner` 认领的字节数，`[RAM 面, WRAM 面]`。
///
/// 与 [`gaps`] 互补：`gaps` 为空 + 本函数非零 = "已声明但未 typed"的确定性债务清单。
pub fn unowned_bytes(spans: &[(NesSpan, SpanKey)], domain: &[NesSpan]) -> [usize; 2] {
    let m = scan(spans);
    let mut n = [0usize; 2];
    for d in domain {
        for a in d.addrs() {
            match locate(a) {
                Some((Face::Ram, i)) => {
                    if !m.own_ram[i] {
                        n[0] += 1;
                    }
                }
                Some((Face::Wram, i)) => {
                    if !m.own_wram[i] {
                        n[1] += 1;
                    }
                }
                None => {}
            }
        }
    }
    n
}

/// P2 比对面（spec §3）：`$0000-$07FF`（RAM 2KB 镜像）+ `$6AF4-$6BD3`（额外面 224 B）。
///
/// 尾界不在这里写死：取 `observe::dump::EXTRA_RANGE`（与 oracle `ram.hex` 同源，
/// 由敌/爆炸池实体常量推导）。这样"哪些字节必须被解释"的真相来自**对拍投影**，
/// 不是本模块自己的声明，更不是 `data/status/nes_map.json` 账本。
pub fn spec_face() -> [NesSpan; 2] {
    let (lo, hi) = crate::memmap::observe::dump::EXTRA_RANGE;
    [
        NesSpan::value(0, RAM_LEN as u16),
        NesSpan::value(lo, hi - lo + 1),
    ]
}

/// 静态腿出口：把六份 `NesMapped::SPANS` + 比对面机械导出成单行 JSON。
///
/// 消费端 = `tools/nes_map_check.py`（判定权在门禁，不在本函数）。本函数只遍历
/// **声明**与两个既有常量，不读任何 NES 字节。
pub fn dump_json() -> Result<String, Vec<String>> {
    use std::fmt::Write as _;

    fn json_str(s: &str) -> String {
        let mut out = String::with_capacity(s.len() + 2);
        out.push('"');
        for c in s.chars() {
            match c {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                c if (c as u32) < 0x20 => {
                    write!(out, "\\u{:04x}", c as u32).expect("写 String 不会失败");
                }
                c => out.push(c),
            }
        }
        out.push('"');
        out
    }

    fn key_json(key: &SpanKey) -> (&'static str, String) {
        match key {
            SpanKey::Owner => ("owner", "null".to_string()),
            SpanKey::View(holders) => ("view", json_str(&holders.join(","))),
            SpanKey::Case(v) => ("case", v.to_string()),
            SpanKey::Mode(v) => ("mode", v.to_string()),
            SpanKey::Ephemeral(s) => ("ephemeral", json_str(s)),
            SpanKey::Unmodeled(s) => ("unmodeled", json_str(s)),
        }
    }

    fn push_entity<T: NesMapped>(
        out: &mut Vec<String>,
        entity: &str,
        all: &mut Vec<(NesSpan, SpanKey)>,
    ) {
        for (span, key) in T::SPANS {
            let (k, note) = key_json(key);
            out.push(format!(
                "{{\"entity\":{},\"start\":{},\"len\":{},\"key\":{},\"note\":{}}}",
                json_str(entity),
                span.start,
                span.len,
                json_str(k),
                note
            ));
            all.push((*span, *key));
        }
    }

    let mut spans: Vec<String> = Vec::new();
    let mut all: Vec<(NesSpan, SpanKey)> = Vec::new();
    push_entity::<crate::samus::state::SamusState>(&mut spans, "SamusState", &mut all);
    push_entity::<crate::enemy::state::Enemy<'static>>(&mut spans, "Enemy", &mut all);
    push_entity::<crate::world::object::Object<'static>>(&mut spans, "Object", &mut all);
    push_entity::<crate::world::room_ram::Room<'static>>(&mut spans, "Room", &mut all);
    push_entity::<crate::world::camera::Camera<'static>>(&mut spans, "Camera", &mut all);
    push_entity::<crate::enemy::explosion::Explosion<'static>>(&mut spans, "Explosion", &mut all);

    let coverage = coverage(&all)?;
    let face = spec_face();
    let face_json: Vec<String> = face
        .iter()
        .map(|s| format!("{{\"start\":{},\"len\":{}}}", s.start, s.len))
        .collect();
    Ok(format!(
        "{{\"schema\":\"nesmap-dump-v1\",\"face\":[{}],\"spans\":[{}],\"coverage\":[{},{}]}}",
        face_json.join(","),
        spans.join(","),
        coverage[0],
        coverage[1]
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enemy::explosion;
    use crate::enemy::state::{
        ensx, Enemy, ENEMY_STRIDE, ENS_BASE, ENS_EXTRA_BASE, SLOT_COUNT as ENS_SLOTS,
    };
    use crate::frame::memory::TypedRestoreManifest;
    use crate::frame::Manager;
    use crate::memmap::storage::NesImage;
    use crate::ram_map::{
        MAP_POS_X, MAP_POS_Y, SAMUS_SCR_X, SAMUS_SCR_Y, SAMUS_STATUS, SCROLL_DIR, SCROLL_X,
        SCROLL_Y,
    };
    use crate::samus::state::SamusState;
    use crate::world::camera::Camera;
    use crate::world::object::{OBJECTS_BASE, SLOT_COUNT as OBJ_SLOTS, SLOT_STRIDE};
    use crate::world::room_ram::{
        Room, ROOM_NUMBER, ROOM_PTR, ROOM_RAM_BASE, ROOM_RAM_LEN, WORK_PTR,
    };

    /// 静态腿的比对面必须与 oracle `ram.hex` 同源——**独立于 `nes_map.json` 账本**。
    /// 若哪天有人把门逼绿的方式改成"缩小比对面"，这里先红。
    #[test]
    fn spec_face_matches_oracle_dump() {
        let face = spec_face();
        assert_eq!((face[0].start, face[0].len), (0x0000, 0x0800), "RAM 2KB 镜像面");
        assert_eq!(
            (face[1].start, face[1].len),
            (
                crate::enemy::state::ENS_EXTRA_BASE,
                0x00E0
            ),
            "额外面 224 B（起点取实体常量，长度用字面量独立锚定）"
        );
        assert_eq!(
            usize::from(face[0].len) + usize::from(face[1].len),
            0x8E0,
            "比对面 = 2272 B（与 observe::dump::DUMP_LEN 同数）"
        );
        let (lo, hi) = crate::memmap::observe::dump::EXTRA_RANGE;
        assert_eq!(
            (face[1].start, face[1].start + face[1].len - 1),
            (lo, hi),
            "尾界必须等于 oracle dump 的额外面闭区间"
        );
        assert_eq!(
            crate::enemy::state::ENS_EXTRA_BASE + 0xC0,
            crate::enemy::explosion::EXTRA_BASE,
            "敌额外面尾与爆炸额外面首首尾相接"
        );

        let json = dump_json().expect("六份声明不应有 Owner 重叠");
        assert!(
            json.starts_with("{\"schema\":\"nesmap-dump-v1\""),
            "dump 必须带 schema 头：{}",
            &json[..json.len().min(80)]
        );
        assert!(json.contains("\"coverage\":[23,84]"), "owner 字节数锚点");
    }

    /// 每条 Unmodeled 说明都必须带可 grep 的代码指针（`*.rs`），否则
    /// "整段未建模" 会变成免检通道——门禁侧 `nes_map_check.py` 对账本 reason
    /// 做同样的 `CODE_POINTER_RE` 校验，这里在声明源头先红。
    #[test]
    fn unmodeled_notes_name_a_code_location() {
        let groups: [&[(NesSpan, SpanKey)]; 6] = [
            spans_of::<SamusState>(),
            spans_of::<Enemy<'static>>(),
            spans_of::<crate::world::object::Object<'static>>(),
            spans_of::<Room<'static>>(),
            spans_of::<Camera<'static>>(),
            spans_of::<explosion::Explosion<'static>>(),
        ];
        for spans in groups {
            for (span, key) in spans {
                if let SpanKey::Unmodeled(note) = key {
                    assert!(
                        note.contains(".rs"),
                        "Unmodeled {:#06X}+{} 缺代码指针：{note}",
                        span.start,
                        span.len
                    );
                }
            }
        }
    }

    /// 带生命周期的实体类型取关联常量：`T::SPANS` 在 `impl<'a>` 上需要显式实例化。
    fn spans_of<T: NesMapped>() -> &'static [(NesSpan, SpanKey)] {
        T::SPANS
    }

    /// 敌域两面（`$0400-$04BF` 主面 + `$6AF4-$6BB3` 额外面）。
    fn enemy_domain() -> Vec<NesSpan> {
        vec![
            NesSpan::value(ENS_BASE, u16::from(ENS_SLOTS) * ENEMY_STRIDE),
            NesSpan::value(ENS_EXTRA_BASE, u16::from(ENS_SLOTS) * ENEMY_STRIDE),
        ]
    }

    /// 对象池整脸（`$0300-$03FF`，16 槽 × 16 B）。
    fn object_domain() -> Vec<NesSpan> {
        vec![NesSpan::value(
            OBJECTS_BASE,
            u16::from(OBJ_SLOTS) * SLOT_STRIDE,
        )]
    }

    /// 房间域：WRAM 房间缓冲 + 三个 `RoomPtr` 游标。
    fn room_domain() -> Vec<NesSpan> {
        vec![
            NesSpan::value(ROOM_RAM_BASE, ROOM_RAM_LEN),
            NesSpan::value(ROOM_NUMBER, 1),
            NesSpan::value(WORK_PTR, 2),
            NesSpan::value(ROOM_PTR, 2),
        ]
    }

    /// 相机域八个字节（`Camera` 的访问器全集：7 个滚动/位置 + `block_nt3`）。
    fn camera_domain() -> Vec<NesSpan> {
        vec![
            NesSpan::value(SCROLL_X, 1),
            NesSpan::value(SCROLL_Y, 1),
            NesSpan::value(SCROLL_DIR, 1),
            NesSpan::value(MAP_POS_X, 1),
            NesSpan::value(MAP_POS_Y, 1),
            NesSpan::value(SAMUS_SCR_X, 1),
            NesSpan::value(SAMUS_SCR_Y, 1),
            NesSpan::value(crate::ram_map::SCROLL_BLOCK_NT3, 1),
        ]
    }

    /// 爆炸域两面（`$04C0-$04DF` 主面 + `$6BB4-$6BD3` 额外面）。
    fn explosion_domain() -> Vec<NesSpan> {
        vec![
            NesSpan::value(
                explosion::BASE,
                u16::from(explosion::SLOT_COUNT) * explosion::STRIDE,
            ),
            NesSpan::value(
                explosion::EXTRA_BASE,
                u16::from(explosion::SLOT_COUNT) * explosion::EXTRA_STRIDE,
            ),
        ]
    }

    /// brief 的验收断言（逐字）：Samus 的 typed 面 = 23 个 RAM 字节，且无重叠。
    #[test]
    fn samus_declares_every_typed_byte_as_owner() {
        let c = coverage(crate::samus::state::SamusState::SPANS).expect("无重叠");
        assert_eq!(c[0], 23, "Samus 应认领 23 个 RAM 字节");
    }

    /// 声明集合必须与 `TYPED_ADDRS` **逐地址相等**——否则声明就是第二真相源。
    #[test]
    fn samus_declaration_agrees_with_typed_addrs() {
        let mut declared: Vec<u16> = Vec::new();
        for (span, key) in SamusState::SPANS {
            match key {
                SpanKey::Owner => declared.extend(span.addrs()),
                SpanKey::Unmodeled(_) => {
                    assert_eq!(
                        span.start,
                        crate::samus::state::HEALTH_LO,
                        "唯一未 typed 段=血量"
                    );
                    assert_eq!(span.len, 2, "血量两字节（低/高）");
                }
                other => panic!("Samus 声明里不该出现 {other:?}（只有 Owner + 血量 Unmodeled）"),
            }
        }
        // 漏一格的直接证据（先点名再对数）：typed 面内出现"没人声明"的字节。
        let face = [
            NesSpan::value(SAMUS_STATUS, SamusState::TYPED_LEN),
            NesSpan::value(crate::samus::state::HEALTH_LO, 2),
        ];
        assert!(
            gaps(SamusState::SPANS, &face).is_empty(),
            "Samus 全域有声明空洞：{:?}",
            gaps(SamusState::SPANS, &face)
        );
        assert_eq!(
            unowned_bytes(SamusState::SPANS, &face),
            [2, 0],
            "Samus 全域只有血量两字节未 typed"
        );
        assert_eq!(
            declared.len(),
            23,
            "声明条数 = typed 字节数，不允许重复地址"
        );
        declared.sort_unstable();
        let mut typed = SamusState::TYPED_ADDRS.to_vec();
        typed.sort_unstable();
        assert_eq!(declared, typed, "声明集合 ≠ TYPED_ADDRS（声明漂移）");
        for (i, a) in declared.iter().enumerate() {
            assert_eq!(
                *a,
                SAMUS_STATUS + i as u16,
                "typed 面必须从 $0300 起连续（对象池 slot0 的 overlay 长度靠它）"
            );
        }
    }

    /// `project` 与 `snapshot_nes()` 必须逐字节一致（同源投影，且只写 owner 字节）。
    #[test]
    fn samus_projection_matches_snapshot_projection() {
        let mut mem = Manager::default();
        mem.samus().set_speed_y(-2);
        mem.samus().set_speed_x(3);
        mem.samus().set_accel_y(-1);
        mem.samus().set_accel_x(7);
        mem.samus().set_health(0x0123);
        let img = mem.snapshot_nes();

        let mut out = NesImage {
            ram: [0xAA; 0x800],
            wram: [0xAA; 0x2000],
        };
        mem.samus.project(&mut out);

        for (span, key) in SamusState::SPANS {
            if !matches!(key, SpanKey::Owner) {
                continue;
            }
            for a in span.addrs() {
                assert_eq!(
                    out.ram[(a as usize) & 0x7FF],
                    img.ram[(a as usize) & 0x7FF],
                    "{a:#06X} 的投影与 snapshot_nes 不一致"
                );
            }
        }
        let written = SamusState::SPANS
            .iter()
            .filter(|(_, k)| matches!(k, SpanKey::Owner))
            .map(|(s, _)| s.len as usize)
            .sum::<usize>();
        let touched = out.ram.iter().filter(|b| **b != 0xAA).count();
        assert_eq!(
            touched, written,
            "project 只许写 owner 字节（哨兵外泄=越权投影）"
        );
        assert!(out.wram.iter().all(|b| *b == 0xAA), "Samus 不碰 WRAM 面");
    }

    /// 敌域两面都被声明覆盖（无空洞），owner 只在 `EnsExtra` 前 7 B × 12 槽。
    #[test]
    fn enemy_declares_both_faces_without_holes() {
        let c = coverage(Enemy::SPANS).expect("无重叠");
        assert_eq!(c, [0, 84], "敌域 owner = 12 槽 × 7 B，全在 WRAM 面");
        let dom = enemy_domain();
        assert!(
            gaps(Enemy::SPANS, &dom).is_empty(),
            "敌域两面必须被声明全覆盖，空洞：{:?}",
            gaps(Enemy::SPANS, &dom)
        );
        assert_eq!(
            unowned_bytes(Enemy::SPANS, &dom),
            [192, 108],
            "未 typed 债：主面 192 B（RAM）+ 额外尾 9 B × 12 槽（WRAM）"
        );
    }

    /// 声明与回灌清单（`TypedRestoreManifest`）逐地址互查——两份独立枚举必须一致。
    #[test]
    fn enemy_owner_bytes_agree_with_restore_manifest() {
        let declared: Vec<u16> = Enemy::SPANS
            .iter()
            .filter(|(_, k)| matches!(k, SpanKey::Owner))
            .flat_map(|(s, _)| s.addrs())
            .collect();
        let manifest: Vec<u16> = TypedRestoreManifest::wram_addrs().collect();
        assert_eq!(
            declared, manifest,
            "声明的 owner 字节 ≠ 回灌清单（声明或清单有一方漂移）"
        );
        assert_eq!(
            declared.len(),
            usize::from(ENS_SLOTS) * (usize::from(ensx::TYPED_MAX) + 1),
            "owner 字节数 = 槽数 × 每槽 typed 字节数"
        );
    }

    /// 对象池：`$0300` 起的 slot0 overlay 真相在 `SamusState` ⇒ 声明为 `View`（只读）。
    #[test]
    fn object_pool_declares_slot0_overlay_as_view() {
        let spans = spans_of::<crate::world::object::Object<'static>>();
        let dom = object_domain();
        assert!(
            gaps(spans, &dom).is_empty(),
            "对象池整脸必须无空洞：{:?}",
            gaps(spans, &dom)
        );
        // 本池自己**没有** typed 字节：23 B overlay 的 owner 是 `SamusState`（`View` 不计位），
        // 所以对象池的 typed 债是整脸 256 B——不是 233 B（233 是"本池未建模"的字节数）。
        assert_eq!(
            unowned_bytes(spans, &dom),
            [256, 0],
            "对象池自身 typed 债 = 整脸 256 B（overlay 归 Samus，不归本池）"
        );
        let views: Vec<NesSpan> = spans
            .iter()
            .filter(|(_, k)| matches!(k, SpanKey::View(_)))
            .map(|(s, _)| *s)
            .collect();
        assert_eq!(views.len(), 1, "overlay 只有一段 View");
        assert_eq!(views[0].start, OBJECTS_BASE, "View 起点 = 池基址");
        assert_eq!(
            views[0].len,
            SamusState::TYPED_LEN,
            "View 长度 = Samus typed 面长度（不另立数字）"
        );
    }

    /// 房间域与相机域：声明无空洞；全部未 typed（`Unmodeled`），owner 为 0。
    #[test]
    fn room_and_camera_declarations_have_no_holes() {
        let room = spans_of::<Room<'static>>();
        let rdom = room_domain();
        assert!(
            gaps(room, &rdom).is_empty(),
            "房间域空洞：{:?}",
            gaps(room, &rdom)
        );
        assert_eq!(
            unowned_bytes(room, &rdom),
            [5, 2048],
            "游标 5 B + 缓冲 2048 B 全未 typed"
        );
        assert_eq!(coverage(room).expect("无 Owner 段 ⇒ 无冲突"), [0, 0]);

        let cam = spans_of::<Camera<'static>>();
        let cdom = camera_domain();
        assert!(
            gaps(cam, &cdom).is_empty(),
            "相机域空洞：{:?}",
            gaps(cam, &cdom)
        );
        assert_eq!(
            unowned_bytes(cam, &cdom),
            [8, 0],
            "相机 8 B 全未 typed（含 ScrollBlockOnNameTable3）"
        );
        assert_eq!(coverage(cam).expect("无 Owner 段 ⇒ 无冲突"), [0, 0]);
    }

    /// 爆炸域两面：无空洞、全未 typed。
    #[test]
    fn explosion_declares_both_faces_without_holes() {
        let spans = spans_of::<explosion::Explosion<'static>>();
        let dom = explosion_domain();
        assert!(
            gaps(spans, &dom).is_empty(),
            "爆炸域空洞：{:?}",
            gaps(spans, &dom)
        );
        assert_eq!(
            unowned_bytes(spans, &dom),
            [32, 32],
            "主面 32 B + 额外面 32 B 全未 typed"
        );
        assert_eq!(coverage(spans).expect("无 Owner 段 ⇒ 无冲突"), [0, 0]);
    }

    /// 六份声明合并：owner 集合无冲突，总数 = Samus 23 + 敌 84。
    #[test]
    fn all_entity_declarations_have_no_owner_conflict() {
        let mut all: Vec<(NesSpan, SpanKey)> = Vec::new();
        all.extend_from_slice(SamusState::SPANS);
        all.extend_from_slice(spans_of::<Enemy<'static>>());
        all.extend_from_slice(spans_of::<crate::world::object::Object<'static>>());
        all.extend_from_slice(spans_of::<Room<'static>>());
        all.extend_from_slice(spans_of::<Camera<'static>>());
        all.extend_from_slice(spans_of::<explosion::Explosion<'static>>());
        assert_eq!(
            coverage(&all).expect("六份声明之间不得有 owner 重叠"),
            [23, 84]
        );
    }

    /// **注入实验 A**：故意让两个 Owner 段重叠 ⇒ 每个重叠字节各报一行。
    #[test]
    fn owner_overlap_is_reported() {
        let spans = [
            (NesSpan::value(0x0010, 4), SpanKey::Owner),
            (NesSpan::value(0x0012, 4), SpanKey::Owner),
        ];
        let err = coverage(&spans).expect_err("重叠必须报错");
        assert_eq!(err.len(), 2, "重叠 0x0012/0x0013 各一行：{err:?}");
        assert!(err.iter().all(|e| e.contains("重叠")), "{err:?}");
    }

    /// **注入实验 B**：删掉一格 ⇒ `gaps` 必须点名那一格（声明空洞不得静默）。
    #[test]
    fn declaration_hole_is_named() {
        let dom = [NesSpan::value(0x0030, 8)];
        let spans = [
            (NesSpan::value(0x0030, 4), SpanKey::Owner),
            (NesSpan::value(0x0035, 3), SpanKey::Unmodeled("尾段")),
        ];
        assert_eq!(gaps(&spans, &dom), vec![0x0034], "漏掉的 0x0034 必须被点名");
        assert_eq!(
            unowned_bytes(&spans, &dom),
            [4, 0],
            "未 typed 字节 = 域内无 Owner 的格"
        );
    }

    /// 非 `Owner` 键永不标 owner 位（否则覆盖度门会阻止合法改动）。
    #[test]
    fn non_owner_keys_never_claim_bits() {
        let spans = [
            (NesSpan::value(0x0020, 4), SpanKey::Owner),
            (NesSpan::value(0x0020, 4), SpanKey::View(&["Other"])),
            (NesSpan::value(0x0020, 4), SpanKey::Ephemeral("例程临时量")),
            (NesSpan::value(0x0020, 4), SpanKey::Unmodeled("未建模")),
            (NesSpan::value(0x0020, 4), SpanKey::Case(1)),
            (NesSpan::value(0x0020, 4), SpanKey::Mode(2)),
        ];
        assert_eq!(
            coverage(&spans).expect("非 Owner 键不参与冲突判定"),
            [4, 0],
            "只有 Owner 计位；View/Ephemeral/Unmodeled/Case/Mode 都不计"
        );
    }

    /// 越界认领（不在 RAM/WRAM 可认领面）必须报错，不许静默丢弃。
    #[test]
    fn out_of_face_claim_is_reported() {
        let spans = [(NesSpan::value(0x2000, 1), SpanKey::Owner)];
        let err = coverage(&spans).expect_err("$2000 不是可认领面");
        assert!(err[0].contains("越界"), "{err:?}");
    }

    /// 敌句柄的 `project` 只写本槽 7 B（域级声明 ≠ 句柄级投影）。
    #[test]
    fn enemy_projection_writes_only_this_slots_typed_bytes() {
        let mut mem = Manager::default();
        mem.write_wram(ENS_EXTRA_BASE + 3 * ENEMY_STRIDE + ensx::STATUS, 0x5A);
        let img = mem.snapshot_nes();

        let mut out = NesImage {
            ram: [0xAA; 0x800],
            wram: [0xAA; 0x2000],
        };
        mem.enemy(3).project(&mut out);

        let base = ENS_EXTRA_BASE + 3 * ENEMY_STRIDE;
        for off in 0..=ensx::TYPED_MAX {
            let a = base + off;
            let i = (a - crate::memmap::nesmap::WRAM_BASE) as usize;
            assert_eq!(
                out.wram[i], img.wram[i],
                "{a:#06X} 投影与 snapshot_nes 不一致"
            );
        }
        let touched = out.wram.iter().filter(|b| **b != 0xAA).count();
        assert_eq!(
            touched,
            usize::from(ensx::TYPED_MAX) + 1,
            "只许写本槽 7 B（其余 11 槽与主面都不许动）"
        );
        assert!(out.ram.iter().all(|b| *b == 0xAA), "敌句柄不碰 RAM 面");
    }
}
