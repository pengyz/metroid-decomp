//! 脚本化输入 + 无窗口运行（可玩构建的自动化验收面，#102）。
//!
//! 目的：把「用键盘走进门」变成一条可复跑的确定性命令——`--script` 给逐帧按键，
//! `--headless` 不开窗，`--shot-frames` 在指定帧落 PPM，`--state-out` 落逐帧状态 CSV。
//! 只服务验收与回归，不参与游戏逻辑（判据/白名单不动）。

use std::io::{BufWriter, Write};
use std::path::Path;

use game_platform::{GameDriver, PadState, FRAME_H, FRAME_W};

use crate::CoreDriver;

/// 一段按键窗口（帧号 1-based，闭区间；`end = None` 表示到脚本结束）。
#[derive(Clone, Copy, Debug)]
pub struct KeyWindow {
    pub start: u64,
    pub end: Option<u64>,
    pub pad: PadState,
}

/// 整份脚本：按帧号取按键；窗口未覆盖的帧=无按键。
#[derive(Debug, Default)]
pub struct Script {
    windows: Vec<KeyWindow>,
}

/// 单帧 RAM 注入（`--poke 帧:地址=值`）。
///
/// 只用于「把某个已证实的输入条件摆到位」再观察下游链——例如直接写
/// `SamusDoorData($58)` 模拟「Samus 已踩在门格瓦上」，从而把门过渡链与
/// 「能不能走到门格」解耦。它不改游戏逻辑、不改判据，只做一次 `write_ram`。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Poke {
    pub frame: u64,
    pub addr: u16,
    pub value: u8,
}

/// 解析 `帧:地址=值`（帧 1-based；地址/值可写 `0x..` 或十进制）。
pub fn parse_poke(spec: &str) -> Result<Poke, String> {
    let (frame_s, rest) = spec
        .split_once(':')
        .ok_or_else(|| format!("poke 要写成 `帧:地址=值`：{spec:?}"))?;
    let (addr_s, val_s) = rest
        .split_once('=')
        .ok_or_else(|| format!("poke 要写成 `帧:地址=值`：{spec:?}"))?;
    let parse_num = |s: &str| -> Result<u32, String> {
        let t = s.trim();
        let (digits, radix) = if let Some(h) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
            (h, 16)
        } else {
            (t, 10)
        };
        u32::from_str_radix(digits, radix).map_err(|_| format!("不是整数：{s:?}"))
    };
    let frame = u64::from(parse_num(frame_s)?);
    if frame == 0 {
        return Err("帧号从 1 开始（1-based）".into());
    }
    let addr = parse_num(addr_s)?;
    let value = parse_num(val_s)?;
    if addr > 0xFFFF {
        return Err(format!("地址超过 $FFFF：{addr:#X}"));
    }
    if value > 0xFF {
        return Err(format!("值超过 $FF：{value:#X}"));
    }
    Ok(Poke { frame, addr: addr as u16, value: value as u8 })
}

/// 启动定位（`--start-room`）：**只在指定帧把「房号 / MapPos / Samus 坐标」摆到位**，
/// 让引擎自己的载房判据（`room_load::is_new_load_request`：房号 `< $F0` 且与在装房号不同）
/// 把目标房当成一次正常载房请求。由此把「走到门」从「先能走到那间房」里解耦——
/// 不是 `--poke`（不写门相关状态 `$56/$57/$58`），也不改游戏逻辑。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StartRoom {
    pub frame: u64,
    /// `Some` = 同时请求载房（`--start-room`）；`None` = 只摆坐标/MapPos（`--start-pos` 单用）。
    pub room: Option<u8>,
    pub map: Option<(u8, u8)>,
    pub pos: Option<(u8, u8)>,
    /// `Some(n)` = 同时把 `InArea`（`$0074`）低 4 位设为 n（0=Brinstar 1=Norfair 2=Kraid
    /// 3=Tourian 4=Ridley）。区号决定载房用哪张区表，所以必须与 `room` 同一帧写。
    pub area: Option<u8>,
    /// 坐标落笔帧（`--start-pos-frame`）。`None` = 旧行为：`frame > s.frame`、房号已就位且
    /// 载房机连续空闲 `POS_SETTLE_FRAMES` 帧的第一帧（`None` 时还要再等一拍站稳验证）。
    /// 用于「先靠滚动把目标房载进来、载完再把 Samus 摆到门台」的场景。
    pub pos_frame: Option<u64>,
    /// 与房号同帧写入的固化条件（`--start-preset` 用），空 = 无。
    ///
    /// 只写「已证实的世界状态」（雕像 bit7、桥未生成标志等），不改游戏逻辑；
    /// 目的同 `Poke`：把「已经通过正常流程能得到的状态」摆到位，再观察下游链。
    pub fixtures: &'static [(u16, u8)],
    /// 预设路径自带开机按键（START 两次进 GameEngine）：`--start-preset` 没有外部脚本，
    /// 不按键则引擎停在标题/初始例程，物理与滚屏都不跑（实测 main_routine=0 时写房号
    /// 只改了 RAM 数字，`check_update_name_table` 永不被调用）。普通 `--start-room`
    /// 由调用方脚本负责按 START，这里保持 false。
    pub auto_start: bool,
}

/// 坐标落笔需要连续观察到的「载房机空闲」帧数。
///
/// 实测（`--start-room 0x2C`）：房号在 f900 写入、引擎同一帧装载完（`room_load_phase` 仍 0），
/// 但别的房要走过 `SetupRoom` 的若干尾帧；只等「phase==0 的第一帧」会把坐标写进装载中途，
/// 随后被出生点覆盖（#116 §8.3 的 0xF1/0xF3/0xFF 链就是这样起的）。改为连续 3 帧空闲再落笔，
/// 并在落笔后再观察 2 帧确认没被覆盖（被覆盖则重写，最多 3 次）。
pub const POS_SETTLE_FRAMES: u64 = 3;
/// 起动帧之后强制按 RIGHT 的帧数：只为触发一次真正的房间装载（见 `run_scripted` 注释）。
pub const NUDGE_RIGHT_FRAMES: u64 = 60;

/// 找一个「右邻格仍是目标房」的世界图格：滚动 nudge 需要它，否则 `GetRoomNum`
/// 会按 #61 的 `inc MapPosX` 特径把邻房载进来。世界图里同一房常占多格
/// （例：0x2B 在 (y15,x24)，右邻 (y15,x25) 也是 0x2B）。
fn nudge_cell(room: u8) -> Option<(u8, u8)> {
    let map = game_core::memmap::world_map::world_map();
    for y in 0..32u8 {
        for x in 0..31u8 {
            let i = (y as usize) * 32 + x as usize;
            if map[i] == room && map[i + 1] == room {
                return Some((x, y));
            }
        }
    }
    None
}


/// 坐标落笔的等待上限（帧）：超过就判失败（exit 8），不再静默忽略。
pub const POS_DEADLINE_FRAMES: u64 = 300;

/// 固化起点预设（`--start-preset`）：一组「房间 + 世界状态」的可复跑起点。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StartPreset {
    pub name: &'static str,
    pub room: u8,
    pub map: (u8, u8),
    pub pos: (u8, u8),
    pub area: u8,
    /// 与房号同帧写的世界状态（`(地址, 值)`）。
    pub fixtures: &'static [(u16, u8)],
}

/// `bridge-tourian`：双 Boss 已死 + 两座石像已抬（`$687B/$687C` 的 bit7）+ 站在桥房。
///
/// 位定义来自 `docs/spec/statue-bridge-chain.md`（#76）：bit0=已击败、bit7=石像抬到位；
/// 桥的生成判据是两值 bit7 相与（`07:DADA`），`$0054` bit7=桥已生成（这里显式清 0，
/// 让桥在本帧重新生成，等同「刚抬完像走进桥房」）。房间=Brinstar `Room2B`（世界图 2,4），
/// 坐标取桥房左段（`$E0,$6F`，同 #116 §8.3 尝试 B）。
pub const PRESET_BRIDGE_TOURIAN: StartPreset = StartPreset {
    name: "bridge-tourian",
    room: 0x2B,
    map: (4, 2),
    pos: (0xE0, 0x6F),
    area: 0,
    fixtures: &[(0x687B, 0x81), (0x687C, 0x82), (0x0054, 0x00)],
};

/// 解析 `--start-preset` 的名字。
pub fn parse_preset(name: &str) -> Result<StartPreset, String> {
    match name.trim() {
        "bridge-tourian" => Ok(PRESET_BRIDGE_TOURIAN),
        other => Err(format!(
            "未知预设 {other:?}；可用：bridge-tourian（双 Boss 已死+石像已抬+桥房起点）"
        )),
    }
}

/// 只改 `InArea($0074)` 的低 4 位（区号），高 4 位保留。
///
/// 区号决定载房用哪张区表（`room_load::area_idx` 读低 4 位），所以换成别的区的房间时
/// 必须与房号同一帧写进去。
pub fn with_area(cur: u8, area: u8) -> u8 {
    (cur & 0xF0) | (area & 0x0F)
}

/// 解析 `--start-room` 的房号（十进制或 `0x..`）；接受范围与引擎载房判据一致。
pub fn parse_room(spec: &str) -> Result<u8, String> {
    let t = spec.trim();
    let (digits, radix) = if let Some(h) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        (h, 16)
    } else {
        (t, 10)
    };
    let v = u32::from_str_radix(digits, radix)
        .map_err(|_| format!("房间号不被接受：{spec:?} 不是整数（写十进制或 0x..）"))?;
    if v >= 0xF0 {
        return Err(format!(
            "房间号不被接受：{spec:?}={v:#X}；载房判据只认 < $F0（$F0..$FE 是载房尾段游标、$FF 是空房号）"
        ));
    }
    Ok(v as u8)
}

/// 解析 `X,Y`（各自十进制或 `0x..`）。
pub fn parse_pair(spec: &str) -> Result<(u8, u8), String> {
    let (a, b) = spec
        .split_once(',')
        .ok_or_else(|| format!("要写成 `X,Y`：{spec:?}"))?;
    let one = |s: &str| -> Result<u8, String> {
        let t = s.trim();
        let (digits, radix) = if let Some(h) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
            (h, 16)
        } else {
            (t, 10)
        };
        let v = u32::from_str_radix(digits, radix).map_err(|_| format!("不是整数：{s:?}"))?;
        if v > 0xFF {
            return Err(format!("坐标超过 $FF：{s:?}"));
        }
        Ok(v as u8)
    };
    Ok((one(a)?, one(b)?))
}

/// 解析一行 `起始-结束 KEYS`：
/// - 帧号 1-based，`20-80` 闭区间；`20-` 表示开到脚本结束；
/// - KEYS 用 `+` 分隔，如 `RIGHT+A`；`NONE` 表示显式无键（可省略）。
pub fn parse_line(line: &str) -> Result<Option<KeyWindow>, String> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return Ok(None);
    }
    let mut it = line.split_whitespace();
    let range = it.next().ok_or_else(|| format!("缺帧区间：{line:?}"))?;
    let keys = it.next().unwrap_or("NONE");
    if let Some(rest) = it.next() {
        return Err(format!("多余字段 {rest:?}（格式：`起始-结束 KEYS`）"));
    }
    let (a, b) = range
        .split_once('-')
        .ok_or_else(|| format!("帧区间要写成 `起始-结束`：{range:?}"))?;
    let start: u64 = a.parse().map_err(|_| format!("起始帧不是整数：{a:?}"))?;
    if start == 0 {
        return Err("帧号从 1 开始（1-based）".into());
    }
    let end = if b.trim().is_empty() {
        None
    } else {
        let e: u64 = b.parse().map_err(|_| format!("结束帧不是整数：{b:?}"))?;
        if e < start {
            return Err(format!("结束帧 {e} 小于起始帧 {start}"));
        }
        Some(e)
    };
    let pad = parse_keys(keys)?;
    Ok(Some(KeyWindow { start, end, pad }))
}

/// `RIGHT+A` → PadState（大小写不敏感；`NONE`/`-` = 空）。
pub fn parse_keys(spec: &str) -> Result<PadState, String> {
    let mut p = PadState::default();
    if spec.eq_ignore_ascii_case("NONE") || spec == "-" {
        return Ok(p);
    }
    for k in spec.split('+') {
        if k.is_empty() {
            return Err(format!("按键段为空：{spec:?}"));
        }
        let k = k.to_ascii_uppercase();
        p = match k.as_str() {
            "A" => p.with(PadState::A, true),
            "B" => p.with(PadState::B, true),
            "SELECT" => p.with(PadState::SELECT, true),
            "START" => p.with(PadState::START, true),
            "UP" => p.with(PadState::UP, true),
            "DOWN" => p.with(PadState::DOWN, true),
            "LEFT" => p.with(PadState::LEFT, true),
            "RIGHT" => p.with(PadState::RIGHT, true),
            other => return Err(format!("未知按键 {other:?}（可用 A/B/SELECT/START/UP/DOWN/LEFT/RIGHT）")),
        };
    }
    Ok(p)
}

impl Script {
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut windows = Vec::new();
        for (i, line) in text.lines().enumerate() {
            if let Some(w) = parse_line(line).map_err(|e| format!("第 {} 行：{e}", i + 1))? {
                windows.push(w);
            }
        }
        Ok(Self { windows })
    }

    /// 取该帧的按键形（多个窗口重叠时按位或；帧号 1-based）。
    pub fn pad_at(&self, frame: u64) -> PadState {
        let mut p = PadState::default();
        for w in &self.windows {
            let in_start = frame >= w.start;
            let in_end = w.end.map_or(true, |e| frame <= e);
            if in_start && in_end {
                p.bits |= w.pad.bits;
            }
        }
        p
    }

    pub fn windows(&self) -> &[KeyWindow] {
        &self.windows
    }
}

/// 逐帧状态行（观测面：房间/地图格/滚屏/门状态/萨姆斯位置——门→切房的最小证据集）。
fn state_row(driver: &CoreDriver, frame: u64, pad: PadState) -> String {
    let m = &driver.engine.mem;
    let r = |a: u16| m.read_ram(a);
    let (sx, sy) = (r(game_core::ram_map::SAMUS_X), r(game_core::ram_map::SAMUS_Y));
    format!(
        "{frame},{:02X},{},{},{},{},{},{},{},{},{},{}",
        pad.bits,
        r(game_core::ram_map::GAME_MODE),
        r(game_core::ram_map::MAIN_ROUTINE),
        r(game_core::ram_map::ROOM_NUMBER),
        r(game_core::ram_map::MAP_POS_X),
        r(game_core::ram_map::MAP_POS_Y),
        r(game_core::ram_map::SCROLL_X),
        r(game_core::ram_map::SCROLL_Y),
        r(game_core::ram_map::DOOR_ENTRY_STATUS),
        r(game_core::ram_map::DOOR_DELAY),
        format_args!("{sx:03X}/{sy:03X}"),
    ) + &format!(
        ",{},{},{},{},{},{},{},{}",
        r(game_core::ram_map::SAMUS_STATUS),
        m.room_load_phase,
        r(game_core::ram_map::SAMUS_DOOR_DATA),
        r(game_core::ram_map::DOOR_SCROLL_STATUS),
        door_slots(m),
        probe_tile(m),
        door_tiles(m),
        r(game_core::ram_map::IN_AREA),
    ) + &format!(
        ",{:02X},{:02X},{:02X}",
        r(0x0054),
        r(0x687B),
        r(0x687C)
    )
}

/// 四门槽 status（$B0/$A0/$90/$80——按槽序 8..11）拼成 `a/b/c/d`。
fn door_slots(m: &game_core::frame::Manager) -> String {
    (0x80u16..=0xB0).step_by(0x10).map(|o| m.read_ram(0x0300 + o).to_string()).collect::<Vec<_>>().join("/")
}

/// Samus 右侧判点所在 WRAM 瓦（页选=Samus.hi($030C)&1；越界回 `--`）。
fn probe_tile(m: &game_core::frame::Manager) -> String {
    let sx = m.read_ram(game_core::ram_map::SAMUS_X);
    let sy = m.read_ram(game_core::ram_map::SAMUS_Y);
    let hi = m.read_ram(0x030C) & 1;
    let x = sx.wrapping_add(8);
    let idx = (((sy & 0xF8) as u16) << 2) | ((x >> 3) as u16);
    let page = if hi != 0 { 0x0400u16 } else { 0 };
    format!("{:02X}", m.read_wram(0x6000 | page | (idx & 0x3FF)))
}

/// 房间碰撞图里 `$A0/$A1`（门瓦）的个数 `竖门/横门`——判「本房有没有门瓦」。
fn door_tiles(m: &game_core::frame::Manager) -> String {
    let mut a0 = 0u32;
    let mut a1 = 0u32;
    for i in 0..0x800u16 {
        match m.read_wram(0x6000 | i) {
            0xA0 => a0 += 1,
            0xA1 => a1 += 1,
            _ => {}
        }
    }
    format!("{a0}/{a1}")
}

/// PPM(P6) 落盘：零依赖、可用任意看图器/转换器打开。
pub fn write_ppm(path: &Path, rgb: &[u8]) -> std::io::Result<()> {
    let mut f = BufWriter::new(std::fs::File::create(path)?);
    write!(f, "P6\n{FRAME_W} {FRAME_H}\n255\n")?;
    f.write_all(rgb)?;
    f.flush()
}

/// 脚本运行结果（供调用方断言/打印）。
pub struct RunReport {
    pub frames: u64,
    pub rooms: Vec<(u64, u8)>,
    pub states: Vec<(u64, u8, u8, u8)>, // (frame, room, mapx, mapy) 每次变化
    /// 启动定位结果：(目标房号, 引擎是否装载成功)。`None` = 本次未用 `--start-room`。
    pub start_room: Option<(u8, bool)>,
    /// 坐标落笔结果：`None` = 本次未用 `--start-pos`；`Some(true)` = 落笔并站稳；
    /// `Some(false)` = 在 `POS_DEADLINE_FRAMES` 内没落成（调用方必须 fail-loud）。
    pub pos: Option<bool>,
}

/// 坐标落笔的小状态机（等空闲 → 落笔 → 站稳验证，被覆盖就重写）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PosPhase {
    /// 等「载房机连续空闲」。
    Wait,
    /// 已落笔，正在核对接下来两帧是否保持目标坐标。
    Verify { left: u8, retries: u8 },
    /// 落成（或无需落笔）。
    Done,
    /// 超时未落成。
    Failed,
}

/// 无窗口跑 `frames` 帧：逐帧喂脚本按键、落状态 CSV、在指定帧落 PPM。
///
/// 帧号 1-based：第 1 次 `step` 即 f=1。返回状态变化摘要（房间/地图格变化点）。
pub fn run_scripted(
    driver: &mut CoreDriver,
    script: &Script,
    frames: u64,
    state_out: Option<&Path>,
    shot_frames: &[u64],
    shot_dir: Option<&Path>,
    pokes: &[Poke],
    start: Option<&StartRoom>,
) -> std::io::Result<RunReport> {
    let mut sink: Box<dyn Write> = match state_out {
        Some(p) => Box::new(BufWriter::new(std::fs::File::create(p)?)),
        None => Box::new(std::io::stdout()),
    };
    writeln!(
        sink,
        "frame,joy,game_mode,main_routine,room,map_x,map_y,scroll_x,scroll_y,door_entry,door_delay,samus_xy,samus_status,load_phase,door_data,door_scroll_status,door_slots,probe_tile,door_tiles,in_area,bridge,statue_kraid,statue_ridley"
    )?;
    if shot_dir.is_some() {
        std::fs::create_dir_all(shot_dir.unwrap())?;
    }
    let mut report =
        RunReport { frames: 0, rooms: Vec::new(), states: Vec::new(), start_room: None, pos: None };
    let mut last = (u8::MAX, u8::MAX, u8::MAX);
    let mut start_seen = false;
    let mut pos_phase = match start {
        Some(s) if s.pos.is_some() => PosPhase::Wait,
        _ => PosPhase::Done,
    };
    let mut idle_streak = 0u64;
    let mut nudge_right_until = 0u64;
    let mut nudge_restore_map = false;
    let mut nudge_done = false;
    let mut nudge_started = false;
    let mut nudge_loaded = false;
    let mut start_applied: Option<u64> = None;
    for frame in 1..=frames {
        let mut pad = script.pad_at(frame);
        // 预设路径没有外部脚本：自己按两次 START（9/18 帧）进 GameEngine。否则引擎停在
        // 初始例程，写房号只改 RAM 数字（实测 main_routine=0，check_update_name_table 不跑）。
        if start.is_some_and(|s| s.auto_start)
            && start_applied.is_none()
            && (frame == 9 || frame == 18 || frame % 60 == 0)
        {
            pad.bits |= 0x10; // START：推进开场/标题，直到进 GameEngine
        }
        let scroll_x_before = driver.engine.mem.read_ram(game_core::ram_map::SCROLL_X);
        if frame <= nudge_right_until && !nudge_done {
            pad.bits |= 0x01; // RIGHT：触发 scroll_right → check_update_name_table 的真实装载径
        }
        for p in pokes.iter().filter(|p| p.frame == frame) {
            driver.engine.mem.write_ram(p.addr, p.value);
        }
        // 启动定位：把目标房号摆进 RAM，让引擎按自己的载房判据重新装载（见 StartRoom 注释）。
        // 坐标要等**载房完成之后**再摆：装载会把 Samus 放回该房的默认出生点（实测 f20 写入
        // 028/06F，f27 装载完成后被改成 080/0B0）。所以房号在 `frame` 写一次，坐标在
        // 「房号已就位 + 任务机空闲 + 主例程在游戏内」后的第一帧写一次。
        if let Some(s) = start {
            // 只进 GameEngine（MainRoutine=3）之后才落「载房起点」：标题/初始化例程里写房号
            // 不会走装载径。预设自带 START；普通 --start-room 由调用方脚本保证已进游戏。
            if start_applied.is_none()
                && frame >= s.frame
                && driver.engine.mem.read_ram(game_core::ram_map::MAIN_ROUTINE) == 3
            {
                start_applied = Some(frame);
                let mem = &mut driver.engine.mem;
                // 区号必须先写：载房用哪张区表由 InArea 低 4 位决定（room_load::area_idx），
                // 同一帧写进去，引擎在下一拍的载房判据里才会用新区的房间表。
                if let Some(a) = s.area {
                    let cur = mem.read_ram(game_core::ram_map::IN_AREA);
                    mem.write_ram(game_core::ram_map::IN_AREA, with_area(cur, a));
                    // 只改 InArea 不够：AreaPointers_RAM（$3B-$48）在 boot 时已按 Brinstar 拷好，
                    // 载房/绘制读的是那张表。原版在区切换（电梯）时调 CopyAreaPointers 重拷，
                    // 这里复用同一支已端口的函数，保持「换区 = 重拷区表」的语义。
                    game_core::frame::tick::copy_area_pointers(mem);
                }
                if let Some((mx, my)) = s.map {
                    mem.write_ram(game_core::ram_map::MAP_POS_X, mx);
                    mem.write_ram(game_core::ram_map::MAP_POS_Y, my);
                }
                if let Some(r) = s.room {
                    mem.write_ram(game_core::ram_map::ROOM_NUMBER, r);
                    // 只写房号**不会装载房间对象**：装载由 `check_update_name_table` 在
                    // **滚屏调用**里做（`world/camera/scroll.rs` 的 `is_new_load_request`
                    // 分支；`scroll_left/right` 是 pub(crate)，shell 调不到）。
                    // 实测 `--start-room 0x13` 后 `door_slots=0/0/0/0`、门瓦 0；
                    // 正常滚屏进同房时 `door_slots=2/0/5/2`、门瓦 36。
                    // 故这里只做「让引擎自己滚一格」的前置：先把 ScrollX 抬离 0，
                    // 避免 ScrollX==0 的 `inc MapPosX + GetRoomNum` 特径（#61）把
                    // MapPos 推到邻格载错房；随后 4 帧强制按 RIGHT，
                    // 由 `scroll_right` 触发的 `check_update_name_table` 走真正的装载径。
                    mem.write_ram(game_core::ram_map::SCROLL_X, 0x00);
                    // 只按 RIGHT 不会滚：滚屏条件是 SamusScrX≥$8F（physics::move_samus_right
                    // 的 `ScrX>=0x8F ⇒ scroll_right`）——站在房中间时 ScrX 远小于阈值，
                    // 之前「4 帧 RIGHT 触发装载」的假设因此不成立（复核实测 door_slots 全 0）。
                    // 故把 Samus 摆到右边界、ScrX 抬过阈值：下一帧 RIGHT 必然调 scroll_right
                    // → check_update_name_table → 真装载。y 选非门行（门恒 y=$68），避免撞门。
                    mem.write_ram(game_core::ram_map::SAMUS_X, 0x90);
                    mem.write_ram(game_core::ram_map::SAMUS_Y, 0xB0);
                    mem.write_ram(game_core::ram_map::SAMUS_SCR_X, 0x90);
                    // 滚动 nudge 会让 `scroll_right` 在 ScrollX==0 时先 `inc MapPosX` 再
                    // `GetRoomNum`（#61 特径）——若右邻格不是同一房，就会载成邻房。
                    // 故先把 MapPos 摆到**右邻格仍是同一房**的地图格（世界图里同一房可占
                    // 多格，如 0x2B 在 (15,24) 的右邻 (15,25) 仍是 0x2B），滚一格后
                    // GetRoomNum 得到同一房号 ⇒ 引擎照常装载目标房。载完后（pos 核对阶段）
                    // 再把 MapPos 恢复成调用方要求的格。
                    if let Some((nx, ny)) = nudge_cell(r) {
                        mem.write_ram(game_core::ram_map::MAP_POS_X, nx);
                        mem.write_ram(game_core::ram_map::MAP_POS_Y, ny);
                        nudge_right_until = frame + NUDGE_RIGHT_FRAMES;
                        nudge_restore_map = s.map.is_some();
                    }
                }
                // 预设固化状态：与房号同帧写，保证载房判据（雕像门/桥生成）看到的是这套世界状态。
                for &(addr, value) in s.fixtures {
                    mem.write_ram(addr, value);
                }
            }
            let applied_at = start_applied.unwrap_or(s.frame);
            match pos_phase {
                PosPhase::Wait if frame > applied_at => {
                    let mem = &mut driver.engine.mem;
                    let idle = mem.room_load_phase == 0;
                    // nudge 装载路径下，载房任务机会把 RoomNumber 走到 $F0..$FF（尾游标），
                    // 不能再用「房号==目标」判就位；改用「任务机空闲 + 已观测到载房窗开过」
                    // ——窗开过就证明引擎确实按目标房号执行了装载（MapPos 预置在右邻同房的格）。
                    let room_ok = s
                        .room
                        .map(|r| mem.read_ram(game_core::ram_map::ROOM_NUMBER) == r)
                        .unwrap_or(true)
                        || (nudge_loaded && mem.room_load_phase == 0);
                    // nudge 路径要等「载房窗确实开过又关上」才允许落位/恢复地图格：
                    // 窗还没开就恢复 MapPos，会让下一帧的 RIGHT 从 4/2 推到 5/2，载成邻房。
                    // 只要请求了房号，就等到「真的装载过」才允许落位：否则 pos 会写在
                    // 载房前，随后被房间出生点/物理顶掉（复核实测 B 路径 f904 就落笔、
                    // f919 已漂到 0x7A）。装载证据 = 滚屏触发过 nudge 或载房窗开过。
                    let load_ok = s.room.is_none() || nudge_loaded;
                    if idle && room_ok && load_ok {
                        if nudge_restore_map {
                            if let Some((mx, my)) = s.map {
                                mem.write_ram(game_core::ram_map::MAP_POS_X, mx);
                                mem.write_ram(game_core::ram_map::MAP_POS_Y, my);
                            }
                            nudge_restore_map = false;
                        }
                        idle_streak += 1;
                    } else {
                        idle_streak = 0;
                    }
                    if let Some(pf) = s.pos_frame {
                        // 显式落笔帧：用户已算好载房耗时，仍要求该帧载房机空闲。
                        if frame >= pf && idle {
                            if let Some((x, y)) = s.pos {
                                mem.write_ram(game_core::ram_map::SAMUS_X, x);
                                mem.write_ram(game_core::ram_map::SAMUS_Y, y);
                            }
                            pos_phase = PosPhase::Verify { left: 2, retries: 0 };
                        }
                    } else if idle_streak >= POS_SETTLE_FRAMES {
                        if let Some((x, y)) = s.pos {
                            mem.write_ram(game_core::ram_map::SAMUS_X, x);
                            mem.write_ram(game_core::ram_map::SAMUS_Y, y);
                        }
                        idle_streak = 0;
                        pos_phase = PosPhase::Verify { left: 2, retries: 0 };
                    }
                    // 起点还没落（`start_applied=None`）时不要计时：预设要先等引擎进
                    // GameEngine（实测到 f466），用 s.frame 计会在这之前就判超时。
                    if start_applied.is_some() && frame > applied_at + POS_DEADLINE_FRAMES {
                        pos_phase = PosPhase::Failed;
                    }
                }
                PosPhase::Verify { left, retries } => {
                    let mem = &mut driver.engine.mem;
                    let target = s.pos.unwrap_or((0, 0));
                    let cur = (
                        mem.read_ram(game_core::ram_map::SAMUS_X),
                        mem.read_ram(game_core::ram_map::SAMUS_Y),
                    );
                    if cur == target {
                        if nudge_restore_map {
                            if let Some((mx, my)) = s.map {
                                mem.write_ram(game_core::ram_map::MAP_POS_X, mx);
                                mem.write_ram(game_core::ram_map::MAP_POS_Y, my);
                            }
                            nudge_restore_map = false;
                        }
                        pos_phase = if left <= 1 {
                            PosPhase::Done
                        } else {
                            PosPhase::Verify { left: left - 1, retries }
                        };
                    } else if retries < 3 {
                        // 被出生点/物理回弹覆盖：再写一次，最多 3 次。
                        mem.write_ram(game_core::ram_map::SAMUS_X, target.0);
                        mem.write_ram(game_core::ram_map::SAMUS_Y, target.1);
                        pos_phase = PosPhase::Verify { left: 2, retries: retries + 1 };
                    } else {
                        pos_phase = PosPhase::Failed;
                    }
                }
                _ => {}
            }
        }
        driver.step(pad.bits, 0);
        // nudge 的退出条件：载房任务机开窗（`room_load_phase != 0`）**或**本帧真的滚了
        // （SCROLL_X 0→非 0）。后者是必须的：滚屏调用里 `check_update_name_table` 会当场
        // `setup_room`，载房窗可能同拍开完，壳层每帧采样看不到 `phase!=0`——只认 phase
        // 会让 `nudge_started` 永远为 false（复核阻断 A 的根因）。滚动发生即证明
        // `check_update_name_table` 跑过且 `is_load` 为真（启动帧写入的房号 <$F0）。
        let scroll_x_after = driver.engine.mem.read_ram(game_core::ram_map::SCROLL_X);
        let scrolled = scroll_x_before == 0 && scroll_x_after != 0;
        if !nudge_done && nudge_right_until > 0 && (driver.engine.mem.room_load_phase != 0 || scrolled) {
            nudge_done = true;
            nudge_started = true;
            nudge_loaded = true;
        }
        writeln!(sink, "{}", state_row(driver, frame, pad))?;
        let m = &driver.engine.mem;
        let now = (
            m.read_ram(game_core::ram_map::ROOM_NUMBER),
            m.read_ram(game_core::ram_map::MAP_POS_X),
            m.read_ram(game_core::ram_map::MAP_POS_Y),
        );
        if let Some(s) = start {
            // 「被接受」= 起动帧之后引擎确实把该房号装过（窗内 RoomNumber 就是目标房号）。
            match s.room {
                Some(r) if now.0 == r && frame >= s.frame => start_seen = true,
                None => start_seen = true,
                _ => {}
            }
        }
        if now != last {
            report.rooms.push((frame, now.0));
            report.states.push((frame, now.0, now.1, now.2));
            last = now;
        }
        if shot_frames.contains(&frame) {
            if let Some(dir) = shot_dir {
                let p = dir.join(format!("f{frame:05}.ppm"));
                write_ppm(&p, driver.video_frame())?;
            }
        }
        report.frames = frame;
    }
    sink.flush()?;
    report.start_room = start.and_then(|s| s.room.map(|r| (r, start_seen)));
    report.pos = start.and_then(|s| {
        s.pos.map(|_| match pos_phase {
            PosPhase::Done => true,
            _ => false,
        })
    });
    // fail-loud：帧数不够/一直没进 GameEngine 时，起点从未写过——不能报「坐标被覆盖」，
    // 那是另一类故障。这里给出可执行的诊断，调用方仍按 pos=Some(false) 退 8。
    if let Some(s) = start {
        if s.pos.is_some() && start_applied.is_none() {
            eprintln!(
                "[start] {} 帧内没有进入 GameEngine（MainRoutine=3），起点从未落笔；加大 --frames 或检查 START 是否被吃掉",
                report.frames
            );
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_windows_and_keys() {
        let s = Script::parse(
            "# 注释\n30-60 START\n120-199 RIGHT+A\n200-210 NONE\n300- RIGHT\n",
        )
        .unwrap();
        assert_eq!(s.pad_at(1).bits, 0);
        assert!(s.pad_at(30).bits & PadState::START != 0);
        assert!(s.pad_at(60).bits & PadState::START != 0);
        assert_eq!(s.pad_at(61).bits & PadState::START, 0);
        let p = s.pad_at(150);
        assert!(p.bits & PadState::RIGHT != 0 && p.bits & PadState::A != 0);
        assert_eq!(s.pad_at(205).bits, 0, "窗口之间的空档必须是空输入");
        assert!(s.pad_at(9999).bits & PadState::RIGHT != 0, "`300-` 开到结尾");
    }

    /// 重叠窗口是**并集**（任一窗口命中的键都算按下）——两个窗口交叠时不得只取先匹配者，
    /// 否则后一个窗口里的按键会被静默吞掉（排查门链时踩过一次：射击窗口落在长按 RIGHT
    /// 窗口内部，B 从未真正喂进引擎）。
    #[test]
    fn overlapping_windows_union_keys() {
        let s = Script::parse("10-20 RIGHT\n15-18 B\n").unwrap();
        let p = s.pad_at(16);
        assert!(p.bits & PadState::RIGHT != 0, "长窗口的 RIGHT 仍在");
        assert!(p.bits & PadState::B != 0, "重叠窗口的 B 不得被吞");
        assert_eq!(s.pad_at(19).bits & PadState::B, 0, "B 窗口结束即松开");
    }

    #[test]
    fn rejects_bad_scripts() {
        assert!(Script::parse("0-10 RIGHT").is_err());
        assert!(Script::parse("10 RIGHT").is_err());
        assert!(Script::parse("10-5 RIGHT").is_err());
        assert!(Script::parse("10-20 RIGHT+LEFTOVERS").is_err());
    }
}

#[cfg(test)]
mod tests_poke {
    use super::*;

    #[test]
    fn parse_poke_hex_and_decimal() {
        assert_eq!(parse_poke("700:0x58=2").unwrap(), Poke { frame: 700, addr: 0x58, value: 2 });
        assert_eq!(parse_poke("12:88=255").unwrap(), Poke { frame: 12, addr: 88, value: 255 });
    }

    #[test]
    fn parse_poke_rejects_bad_specs() {
        for bad in ["700:0x58", "0:0x58=2", "700:0x10000=2", "700:0x58=0x100", "abc:0x58=2"] {
            assert!(parse_poke(bad).is_err(), "{bad:?} 应被拒");
        }
    }
}

#[cfg(test)]
mod tests_start_room {
    use super::*;

    #[test]
    fn parse_room_accepts_decimal_and_hex() {
        assert_eq!(parse_room("30").unwrap(), 0x1E);
        assert_eq!(parse_room("0x1E").unwrap(), 0x1E);
        assert_eq!(parse_room("239").unwrap(), 0xEF);
    }

    /// 房号必须 < $F0：$F0..$FE 是载房尾段游标、$FF 是「空房号」，写进去只会被引擎当成
    /// 无需载入，`--start-room` 就静默不生效——这里必须报错，不能当 0xFF 混过去。
    #[test]
    fn parse_room_rejects_loader_sentinels() {
        for bad in ["0xF0", "255", "0xFF", "256", "-1", "abc"] {
            let err = parse_room(bad).unwrap_err();
            assert!(!err.is_empty(), "{bad:?} 的报错不能为空");
        }
        assert!(parse_room("0xF5").unwrap_err().contains("房间号不被接受"));
    }

    #[test]
    fn parse_pair_reads_two_bytes() {
        assert_eq!(parse_pair("0xD0,0x6F").unwrap(), (0xD0, 0x6F));
        assert_eq!(parse_pair("208,111").unwrap(), (208, 111));
        for bad in ["0xD0", "0xD0,", ",0x6F", "0xD0,0x6F,1", "zz,1"] {
            assert!(parse_pair(bad).is_err(), "{bad:?} 应被拒");
        }
    }

    /// 区号只写低 4 位：高 4 位是同一个字节里的别的标志，不能被顺手清掉。
    #[test]
    fn with_area_only_touches_low_nibble() {
        assert_eq!(with_area(0x30, 2), 0x32);
        assert_eq!(with_area(0x05, 1), 0x01);
        assert_eq!(with_area(0xA0, 0), 0xA0);
    }
}
