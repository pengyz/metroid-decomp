//! **动态执行证据探针（P3 动态腿）**——记录字节面实际被读/写过的地址。
//!
//! 【定位】本模块只在 `#[cfg(test)]` 下编译（`memmap/mod.rs` 以 `#[cfg(test)] mod verify_shim;`
//! 声明）。生产构建里不存在、也不可达：探针不改变任何读写语义，只做位图记录。
//! 之所以不做成 `crates/game-core/tests/*.rs` 集成测试，是因为集成测试链接的 lib 不带
//! `cfg(test)`，探针就得进生产可见面——那违反「插桩仅测试面」。
//!
//! 【挂点】`raw.rs` 的 `read_ram`/`write_ram`/`read_wram`/`write_wram` 四个入口 +
//! `samus/state.rs` 的 `Samus::get`/`Samus::set` 未命中 typed 时的字节面回退。
//! 口径：**字节面访问请求**（含 typed 域经 compat 面读到的请求），不是物理存储写。
//!
//! 【盲区】（报告必须写明，探针不猜）：
//! - `frame/memory.rs` 写 PPUDATA 时 `self.ram()[0x00FF]` 直读——不经 `read_ram`；
//! - `restore_mss` / typed 清零直接 `ram_mut()`/`wram_mut()`——测试装配径，非游戏循环；
//! - boot 的 `clear_pages` 走 `write_ram`，属无条件零填充；动态腿录制窗口从 tick2 起，
//!   显式排除（否则 $0200-$07FF、$6000-$7EFF 会全被记成"写过"）。
//!
//! 【正名】与 Phase-0 历史 `verify_shim/`（L2 写集投影，已删除）无关：本模块只记
//! 「字节面被谁读过/写过」的位图，不构造状态、不做投影。
//!
//! 【纪律】探针内部不调用 `read_ram`/`write_ram`（不递归），不读环境变量，不写文件。

use std::cell::RefCell;

use crate::memmap::nesmap::{NesSpan, SpanKey};

pub(crate) const RAM_LEN: usize = 0x800;
pub(crate) const WRAM_LEN: usize = 0x2000;
const RAM_WORDS: usize = RAM_LEN / 64; // 2048 / 64 = 32
const WRAM_WORDS: usize = WRAM_LEN / 64; // 16384 / 64 = 256

/// 读/写两个位面（0=未触碰，1=触碰）；未 `arm` 时一律不记。
struct Recorder {
    armed: bool,
    ram_read: [u64; RAM_WORDS],
    ram_write: [u64; RAM_WORDS],
    wram_read: [u64; WRAM_WORDS],
    wram_write: [u64; WRAM_WORDS],
}

impl Recorder {
    const fn new() -> Self {
        Self {
            armed: false,
            ram_read: [0; RAM_WORDS],
            ram_write: [0; RAM_WORDS],
            wram_read: [0; WRAM_WORDS],
            wram_write: [0; WRAM_WORDS],
        }
    }

    fn clear(&mut self) {
        self.ram_read = [0; RAM_WORDS];
        self.ram_write = [0; RAM_WORDS];
        self.wram_read = [0; WRAM_WORDS];
        self.wram_write = [0; WRAM_WORDS];
    }
}

thread_local! {
    static REC: RefCell<Recorder> = RefCell::new(Recorder::new());
}

#[inline]
fn mark(words: &mut [u64], idx: usize) {
    words[idx / 64] |= 1u64 << (idx % 64);
}

/// 字节面读请求（`raw.rs` 四入口 + Samus 回退）。
#[inline]
pub(crate) fn note_read(addr: u16) {
    note(addr, true);
}

/// 字节面写请求。
#[inline]
pub(crate) fn note_write(addr: u16) {
    note(addr, false);
}

#[inline]
fn note(addr: u16, is_read: bool) {
    REC.with(|cell| {
        let mut rec = cell.borrow_mut();
        if !rec.armed {
            return;
        }
        // 归一必须与 `read_ram`/`write_ram` 的分派一致：$6000-$7FFF 是 WRAM，
        // 其余按 2KB 镜像 `& 0x7FF`（$0B00 与 $0300 同格）。
        if (0x6000..0x8000).contains(&addr) {
            let idx = (usize::from(addr) - 0x6000) & 0x1FFF;
            let words = if is_read {
                &mut rec.wram_read
            } else {
                &mut rec.wram_write
            };
            mark(words, idx);
        } else {
            let idx = usize::from(addr) & 0x7FF;
            let words = if is_read {
                &mut rec.ram_read
            } else {
                &mut rec.ram_write
            };
            mark(words, idx);
        }
    });
}

/// 开录（清空旧位图）。
pub(crate) fn arm() {
    REC.with(|cell| {
        let mut rec = cell.borrow_mut();
        rec.clear();
        rec.armed = true;
    });
}

/// 停录并取快照。
pub(crate) fn disarm() -> Snapshot {
    REC.with(|cell| {
        let mut rec = cell.borrow_mut();
        rec.armed = false;
        Snapshot {
            ram_read: rec.ram_read,
            ram_write: rec.ram_write,
            wram_read: rec.wram_read,
            wram_write: rec.wram_write,
        }
    })
}

pub(crate) fn is_armed() -> bool {
    REC.with(|cell| cell.borrow().armed)
}

/// 位图快照（值语义；`Clone`/`Eq` 供确定性断言）。
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Snapshot {
    pub(crate) ram_read: [u64; RAM_WORDS],
    pub(crate) ram_write: [u64; RAM_WORDS],
    pub(crate) wram_read: [u64; WRAM_WORDS],
    pub(crate) wram_write: [u64; WRAM_WORDS],
}

/// 位面（ram/wram × 读/写）。
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Plane {
    RamRead,
    RamWrite,
    WramRead,
    WramWrite,
}

impl Snapshot {
    /// 单格读/写位：bit0=读过，bit1=写过。
    pub(crate) fn bits_ram(&self, addr: u16) -> u8 {
        let idx = usize::from(addr) & 0x7FF;
        let w = idx / 64;
        let m = 1u64 << (idx % 64);
        u8::from(self.ram_read[w] & m != 0) | (u8::from(self.ram_write[w] & m != 0) << 1)
    }

    /// WRAM（$6000-$7FFF）单格读/写位。
    pub(crate) fn bits_wram(&self, addr: u16) -> u8 {
        let idx = (usize::from(addr) - 0x6000) & 0x1FFF;
        let w = idx / 64;
        let m = 1u64 << (idx % 64);
        u8::from(self.wram_read[w] & m != 0) | (u8::from(self.wram_write[w] & m != 0) << 1)
    }

    /// 某位面里被触碰的**连续区间**（含端点，升序）。
    pub(crate) fn ranges(&self, plane: Plane) -> Vec<(u16, u16)> {
        let (words, base, len, read): (&[u64], u16, usize, bool) = match plane {
            Plane::RamRead => (&self.ram_read, 0x0000, RAM_LEN, true),
            Plane::RamWrite => (&self.ram_write, 0x0000, RAM_LEN, false),
            Plane::WramRead => (&self.wram_read, 0x6000, WRAM_LEN, true),
            Plane::WramWrite => (&self.wram_write, 0x6000, WRAM_LEN, false),
        };
        let _ = read;
        let mut out: Vec<(u16, u16)> = Vec::new();
        let mut cur: Option<(u16, u16)> = None;
        for idx in 0..len {
            let hit = words[idx / 64] & (1u64 << (idx % 64)) != 0;
            let addr = base + idx as u16;
            match (hit, cur) {
                (true, None) => cur = Some((addr, addr)),
                (true, Some((lo, _))) => cur = Some((lo, addr)),
                (false, Some((lo, hi))) => {
                    out.push((lo, hi));
                    cur = None;
                }
                (false, None) => {}
            }
        }
        if let Some(r) = cur {
            out.push(r);
        }
        out
    }

    /// 全部被触碰的字节（RAM 2KB + WRAM 16KB），`(addr, bits)`。
    pub(crate) fn touched(&self) -> Vec<(u16, u8)> {
        let mut out = Vec::new();
        for idx in 0..RAM_LEN {
            let w = idx / 64;
            let m = 1u64 << (idx % 64);
            let bits =
                u8::from(self.ram_read[w] & m != 0) | (u8::from(self.ram_write[w] & m != 0) << 1);
            if bits != 0 {
                out.push((idx as u16, bits));
            }
        }
        for idx in 0..WRAM_LEN {
            let w = idx / 64;
            let m = 1u64 << (idx % 64);
            let bits =
                u8::from(self.wram_read[w] & m != 0) | (u8::from(self.wram_write[w] & m != 0) << 1);
            if bits != 0 {
                out.push((0x6000 + idx as u16, bits));
            }
        }
        out
    }
}

// ---------------- 静态声明口（与账本/门禁同一份 SPANS） ----------------

/// 单条声明：`(实体名, 段, key)`。
pub(crate) type Decl = (&'static str, NesSpan, SpanKey);

/// 全部实体声明（顺序=实体注册序；供动态腿分类，也是 owner 唯一性判据）。
pub(crate) fn decls() -> Vec<Decl> {
    crate::memmap::nesmap::entity_spans()
        .iter()
        .flat_map(|(name, spans)| spans.iter().map(move |&(span, key)| (*name, span, key)))
        .collect()
}

/// 覆盖 `addr` 的 Owner 实体名（0/1/2+ 个）。**只统计 Owner**——View/Case/Mode 不是归属。
pub(crate) fn owner_entities_at(decls: &[Decl], addr: u16) -> Vec<&'static str> {
    decls
        .iter()
        .filter(|(_, span, key)| {
            matches!(key, SpanKey::Owner)
                && span.start <= addr
                && addr < span.start.saturating_add(span.len)
        })
        .map(|(name, _, _)| *name)
        .collect()
}

/// 断言 `addr` 至多一个 Owner；冲突即 panic（文本含 `0xNNNN` 地址与冲突方）。
///
/// 计划里写作 `assert_owner_unique(mem, addr, writer)`；`mem` 不参与判定（SPANS 是静态
/// 声明），故去掉该参数——签名缩小，语义不变。
pub(crate) fn assert_owner_unique(decls: &[Decl], addr: u16, writer: &'static str) {
    let holders = owner_entities_at(decls, addr);
    assert!(
        holders.len() <= 1,
        "owner 冲突 {addr:#06X}：{} 同时声明 owner（写入者 {writer}）——\
         D-03 要求每字节有且仅有一个 owner（见 docs/spec/entity-ownership-and-nesmap.md）",
        holders.join(", ")
    );
}

/// 四态（+ case/mode/ephemeral）分类：`D-03` 的账本口径。
///
/// 优先级：Owner > Unmodeled > View > Case > Mode > Ephemeral > Undeclared。
/// 之所以 Unmodeled 压过 View/Case：Unmodeled 是"已声明、字节面是真相"的强声明，
/// 报告要按它统计 typed 债；View 只是只读别名。
pub(crate) fn class_of(decls: &[Decl], addr: u16) -> &'static str {
    let mut seen_unmodeled = false;
    let mut seen_view = false;
    let mut seen_case = false;
    let mut seen_mode = false;
    let mut seen_ephemeral = false;
    for (_, span, key) in decls {
        if span.start <= addr && addr < span.start.saturating_add(span.len) {
            match key {
                SpanKey::Owner => return "owner",
                SpanKey::Unmodeled(_) => seen_unmodeled = true,
                SpanKey::View(_) => seen_view = true,
                SpanKey::Case(_) => seen_case = true,
                SpanKey::Mode(_) => seen_mode = true,
                SpanKey::Ephemeral(_) => seen_ephemeral = true,
            }
        }
    }
    if seen_unmodeled {
        "unmodeled"
    } else if seen_view {
        "view"
    } else if seen_case {
        "case"
    } else if seen_mode {
        "mode"
    } else if seen_ephemeral {
        "ephemeral"
    } else {
        "undeclared"
    }
}

/// `addr` 是否在 `spec_face()` 内（动态腿断言"没有面外触碰"）。
pub(crate) fn in_face(addr: u16) -> bool {
    crate::memmap::nesmap::spec_face()
        .iter()
        .any(|s| s.start <= addr && addr < s.start.saturating_add(s.len))
}

// ---------------- 测试 ----------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::FrameEngine;

    const BTN_A: u8 = 0x80;
    const BTN_B: u8 = 0x40;
    const BTN_START: u8 = 0x10;
    const BTN_LEFT: u8 = 0x02;
    const BTN_RIGHT: u8 = 0x01;

    // `cases/case-001-game-start-samus-interaction.txt` 的输入回放（同帧号）。
    fn walk_joy(f: u32) -> u8 {
        match f {
            310 | 350 => BTN_START,
            910..=960 => BTN_RIGHT,
            1010..=1060 => BTN_LEFT,
            1110 => BTN_A,
            1210 => BTN_B,
            _ => 0,
        }
    }

    // `cases/case-002-door-blue-interaction.txt` 的输入回放（同帧号，截到 1500）。
    fn door_joy(f: u32) -> u8 {
        match f {
            310 | 350 => BTN_START,
            910..=1080 => BTN_RIGHT,
            1090 | 1100 => BTN_B,
            1180..=1240 => BTN_RIGHT,
            _ => 0,
        }
    }

    fn title_joy(_f: u32) -> u8 {
        0
    }

    struct Run {
        name: &'static str,
        last_frame: u32,
        snap: Snapshot,
    }

    /// 单场景：tick1（boot）不录，tick2..=last_frame 录制。
    fn run_scenario(name: &'static str, last_frame: u32, joy: fn(u32) -> u8) -> Run {
        let mut engine = FrameEngine::new();
        engine.tick_frame(); // tick1：boot（copy_map/清零/…）——不计入证据
        arm();
        assert!(is_armed());
        for f in 2..=last_frame {
            engine.mem.joy1 = joy(f);
            engine.mem.joy2 = 0;
            engine.tick_frame();
        }
        let snap = disarm();
        assert!(!is_armed(), "disarm 后不得再记录");
        Run {
            name,
            last_frame,
            snap,
        }
    }

    fn ranges_json(ranges: &[(u16, u16)]) -> String {
        let parts: Vec<String> = ranges
            .iter()
            .map(|(lo, hi)| format!("[{lo},{hi}]"))
            .collect();
        format!("[{}]", parts.join(","))
    }

    /// 场景矩阵结果 → 单行 `RAWCOV {...}`（`--nocapture` 捕获；不写文件、不读 env）。
    #[test]
    fn dynamic_execution_evidence() {
        let decls = decls();
        // 静态腿的 owner 唯一性在测试内再断言（门禁 `coverage()` 也查，但动态腿不依赖门禁）。
        for (_, span, _) in &decls {
            for addr in span.addrs() {
                assert_owner_unique(&decls, addr, "dynamic_execution_evidence");
            }
        }

        let runs = [
            run_scenario("title", 400, title_joy),
            run_scenario("walk", 1300, walk_joy),
            run_scenario("door", 1500, door_joy),
        ];

        // ① 探针归一自检：RAM 面（$0000-$07FF）内不得出现面外地址；
        //    WRAM 面（$6000-$7FFF）只有 $6AF4-$6BD3 进账本 face——其余 WRAM 被触碰是
        //    **账本未建模**（不是探针错误），单独计数进报告，不算 FAIL。
        let mut outside_face: Vec<(u16, u8)> = Vec::new();
        for run in &runs {
            for (addr, bits) in run.snap.touched() {
                if !in_face(addr) {
                    assert!(
                        addr >= 0x6000,
                        "场景 {} 触碰 RAM 面外地址 {addr:#06X}（bits={bits}）——探针归一错误",
                        run.name
                    );
                    match outside_face.iter_mut().find(|(a, _)| *a == addr) {
                        Some((_, b)) => *b |= bits,
                        None => outside_face.push((addr, bits)),
                    }
                }
            }
        }
        outside_face.sort_unstable();

        // ② 关键锚必须被真实执行触达（防"探针没挂上"的自证陷阱）。
        let ram: Vec<u8> = runs
            .iter()
            .flat_map(|r| (0..=0x7FFu16).map(move |a| r.snap.bits_ram(a)))
            .collect();
        let wram_touched: Vec<(u16, u8)> = runs
            .iter()
            .flat_map(|r| r.snap.touched())
            .filter(|(a, _)| *a >= 0x6000)
            .collect();
        for anchor in [
            0x0000u16,
            0x00FF,
            0x0106,
            0x0107,
            0x0200,
            crate::ram_map::SAMUS_STATUS,
        ] {
            let bits = runs
                .iter()
                .map(|r| r.snap.bits_ram(anchor))
                .fold(0, |a, b| a | b);
            assert_ne!(bits, 0, "关键锚 {anchor:#06X} 未被触达——探针可能没挂上");
        }
        assert!(
            !wram_touched.is_empty(),
            "WRAM 面零触碰——敌槽/房间缓冲探针未生效"
        );
        let _ = ram;

        // ③ 确定性：同场景跑两遍位图逐位相等。
        let again = run_scenario("walk", 1300, walk_joy);
        assert_eq!(
            again.snap.touched(),
            runs[1].snap.touched(),
            "同场景两次执行触碰集必须逐位相同（确定性）"
        );

        // ④ 输出机器事实：聚合触碰（含分类）+ 分场景区间。
        let mut touched_parts: Vec<String> = Vec::new();
        let mut agg: Vec<(u16, u8)> = Vec::new();
        for run in &runs {
            for (addr, bits) in run.snap.touched() {
                if !in_face(addr) {
                    continue; // 面外（WRAM 未建模区）单独计入 outside_face
                }
                match agg.iter_mut().find(|(a, _)| *a == addr) {
                    Some((_, b)) => *b |= bits,
                    None => agg.push((addr, bits)),
                }
            }
        }
        agg.sort_unstable();
        for (addr, bits) in &agg {
            let b = match bits {
                1 => "r",
                2 => "w",
                _ => "rw",
            };
            touched_parts.push(format!(
                "[\"{addr:#06X}\",\"{b}\",\"{}\"]",
                class_of(&decls, *addr)
            ));
        }
        let scenario_parts: Vec<String> = runs
            .iter()
            .map(|run| {
                format!(
                    "{{\"name\":\"{}\",\"frames\":[2,{}],\"ram_read\":{},\"ram_write\":{},\
                     \"wram_read\":{},\"wram_write\":{}}}",
                    run.name,
                    run.last_frame,
                    ranges_json(&run.snap.ranges(Plane::RamRead)),
                    ranges_json(&run.snap.ranges(Plane::RamWrite)),
                    ranges_json(&run.snap.ranges(Plane::WramRead)),
                    ranges_json(&run.snap.ranges(Plane::WramWrite)),
                )
            })
            .collect();
        // 面部按类计数（分母=声明面，不是触碰集）——报告表格的 total 列。
        let mut face_class: Vec<(&'static str, usize)> = Vec::new();
        for span in crate::memmap::nesmap::spec_face() {
            for addr in span.addrs() {
                let c = class_of(&decls, addr);
                match face_class.iter_mut().find(|(k, _)| *k == c) {
                    Some((_, n)) => *n += 1,
                    None => face_class.push((c, 1)),
                }
            }
        }
        face_class.sort_unstable();
        let face_class_json: Vec<String> = face_class
            .iter()
            .map(|(k, n)| format!("\"{k}\":{n}"))
            .collect();

        let outside_read = outside_face.iter().filter(|(_, b)| b & 1 != 0).count();
        let outside_write = outside_face.iter().filter(|(_, b)| b & 2 != 0).count();
        println!(
            "RAWCOV {{\"schema\":\"raw-execution-coverage-v1\",\"boot\":\"excluded\",\
             \"outside_face\":{{\"read\":{},\"write\":{}}},\
             \"face_classes\":{{{}}},\
             \"scenarios\":[{}],\"touched\":[{}]}}",
            outside_read,
            outside_write,
            face_class_json.join(","),
            scenario_parts.join(","),
            touched_parts.join(",")
        );
    }

    #[test]
    fn probe_normalizes_mirror_and_respects_disarm() {
        // 用未受监控的家庭（$0500 段）验证归一：避免在受监控族（$0300-$0316 等）新增裸字面量。
        let mut m = crate::frame::Manager::default();
        arm();
        let _ = m.read_ram(0x0D00); // $0D00 是 $0500 的 2KB 镜像
        let _ = m.write_ram(0x0501, 0xAA);
        let snap = disarm();
        assert_eq!(snap.bits_ram(0x0500), 1, "读镜像必须归一到 $0500");
        assert_eq!(snap.bits_ram(0x0501), 2, "写 $0501 只记写位");
        let _ = m.read_ram(0x0502);
        assert_eq!(disarm().bits_ram(0x0502), 0, "disarm 后不得再记录");
    }

    #[test]
    fn probe_sees_samus_untyped_fallback() {
        // $0106/$0107 是 SamusState 的 Unmodeled 段（血量低字节），不经 typed 域，
        // 必须由 `Samus::get/set` 回退处的探针记到——否则动态腿会漏掉它。
        let mut engine = FrameEngine::new();
        engine.tick_frame();
        arm();
        {
            let mut s = engine.mem.samus();
            s.set_health(0x0121);
            assert_eq!(s.health(), 0x0121);
        }
        let snap = disarm();
        assert_eq!(snap.bits_ram(0x0106), 3, "$0106 应被读+写");
    }

    #[test]
    fn synthetic_duplicate_owner_panics_with_address() {
        // 合成双 Owner：同一字节（真实受监控族首字节 $0300，用 ram_map 常量取，避免新增裸地址）
        // 同时被 SamusState 与 ObjectPool 声明（ObjectPool 为虚构名，仅演练 panic 文本，
        // 不进入任何真实声明表）。
        let addr = crate::ram_map::SAMUS_STATUS;
        let span = NesSpan::value(addr, 1);
        let decisions: Vec<Decl> = vec![
            ("SamusState", span, SpanKey::Owner),
            ("ObjectPool", span, SpanKey::Owner),
        ];
        let err = std::panic::catch_unwind(|| {
            assert_owner_unique(&decisions, addr, "synthetic_duplicate_owner")
        })
        .expect_err("双 Owner 必须 panic");
        let msg = err
            .downcast_ref::<String>()
            .cloned()
            .unwrap_or_else(|| "panic payload 非 String".to_string());
        assert!(
            msg.contains(&format!("{addr:#06X}")),
            "panic 文本必须含冲突地址 {addr:#06X}：{msg}"
        );
    }
}
