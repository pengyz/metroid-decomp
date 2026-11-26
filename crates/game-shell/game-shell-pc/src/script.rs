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
    /// 坐标落笔帧（`--start-pos-frame`）。`None` = 旧行为：`frame > s.frame` 且任务机空闲的第一帧。
    /// 用于「先靠滚动把目标房载进来、载完再把 Samus 摆到门台」的场景。
    pub pos_frame: Option<u64>,
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
        "frame,joy,game_mode,main_routine,room,map_x,map_y,scroll_x,scroll_y,door_entry,door_delay,samus_xy,samus_status,load_phase,door_data,door_scroll_status,door_slots,probe_tile,door_tiles,in_area"
    )?;
    if shot_dir.is_some() {
        std::fs::create_dir_all(shot_dir.unwrap())?;
    }
    let mut report = RunReport { frames: 0, rooms: Vec::new(), states: Vec::new(), start_room: None };
    let mut last = (u8::MAX, u8::MAX, u8::MAX);
    let mut start_seen = false;
    let mut pos_applied = start.map(|s| s.pos.is_none()).unwrap_or(true);
    for frame in 1..=frames {
        let pad = script.pad_at(frame);
        for p in pokes.iter().filter(|p| p.frame == frame) {
            driver.engine.mem.write_ram(p.addr, p.value);
        }
        // 启动定位：把目标房号摆进 RAM，让引擎按自己的载房判据重新装载（见 StartRoom 注释）。
        // 坐标要等**载房完成之后**再摆：装载会把 Samus 放回该房的默认出生点（实测 f20 写入
        // 028/06F，f27 装载完成后被改成 080/0B0）。所以房号在 `frame` 写一次，坐标在
        // 「房号已就位 + 任务机空闲 + 主例程在游戏内」后的第一帧写一次。
        if let Some(s) = start {
            if s.frame == frame {
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
                if let Some(r) = s.room {
                    mem.write_ram(game_core::ram_map::ROOM_NUMBER, r);
                }
                if let Some((mx, my)) = s.map {
                    mem.write_ram(game_core::ram_map::MAP_POS_X, mx);
                    mem.write_ram(game_core::ram_map::MAP_POS_Y, my);
                }
            }
            if !pos_applied && frame > s.frame {
                let mem = &mut driver.engine.mem;
                let ready = match s.pos_frame {
                    Some(pf) => frame >= pf && mem.room_load_phase == 0,
                    None => {
                        s.room
                            .map(|r| mem.read_ram(game_core::ram_map::ROOM_NUMBER) == r)
                            .unwrap_or(true)
                            && mem.room_load_phase == 0
                    }
                };
                if ready {
                    if let Some((x, y)) = s.pos {
                        mem.write_ram(game_core::ram_map::SAMUS_X, x);
                        mem.write_ram(game_core::ram_map::SAMUS_Y, y);
                    }
                    pos_applied = true;
                }
            }
        }
        driver.step(pad.bits, 0);
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
