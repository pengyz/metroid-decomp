//! 资产包（`.pack`）——ROM 派生数据的**唯一**运行期载体。
//!
//! 设计见 [docs/spec/data-code-separation.md](../../../docs/spec/data-code-separation.md)。
//! 一句话：仓库里只有代码与「段清单」（本文件的 [`PackKey`]），ROM 字节由用户在本地用
//! `pack-tool extract` 提取成包，运行期从包读取。
//!
//! 格式（小端）：
//! ```text
//! Header 32B: magic "MTPK" | version u16 | flags u16 | rom_crc32 u32 | rom_size u32 |
//!              section_count u16 | reserved u16 | reserved2 u32 | reserved3 u32 |
//!              payload_off u32（偏移 28）
//! Entry 20B : key_id u16 | flags u16 | off u32 | len u32 | crc32 u32 | reserved u32
//! ```
//! `off` 相对 `payload_off`。

use std::sync::OnceLock;

/// 包 magic。
pub const MAGIC: [u8; 4] = *b"MTPK";
/// 包格式版本。**任何字段语义变化都必须 +1**（旧包直接拒绝，不许猜）。
pub const VERSION: u16 = 1;
/// 头长度。
pub const HEADER_LEN: usize = 32;
/// 索引项长度。
pub const ENTRY_LEN: usize = 20;

/// 段 id（**名字只在代码里**；包里只存数值——包里不含任何可读语义）。
///
/// 新增段：先在这里加枚举 + id，再在 `extract` feature 里加提取函数，
/// 最后在消费侧加访问器。三处同源，禁止只改两处。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum PackKey {
    /// 资产元信息（JSON 文本；**运行时从不读**，只由 `pack-tool list/verify` 消费）。
    Meta = 0,
    /// `GFXInfo`（ROM `07:C6E0`，29 项 × 7B）——图形装载描述子表。
    GfxInfo = 1,
    /// GFX 数据体（29 项数据按 info 顺序拼接，共 23 712 B）。
    GfxData = 2,

    // ---- 敌帧/位数据面（area/enemy_gfx；bank1 $9D6A-$9F0E 族）----
    /// `EnemyAnimIndexTbl`（bank1 `$9D6A`，128 B）。
    EnAnimTable = 3,
    /// `EnFramePtrTable1`（`$9DE0`，128 项 ×2 B）。
    EnFramePtrs1 = 4,
    /// `EnFramePtrTable2`（`$9EE0`，128 项 ×2 B）。
    EnFramePtrs2 = 5,
    /// `EnPlacePtrTable`（`$9F0E`，128 项 ×2 B）。
    EnPlacePtrs = 6,
    /// 帧数据体（帧流 `[ctrl][ry][rx][流…$FF]` 拼接）。
    EnFrames = 7,
    /// 位件数据体（y/x 对流拼接）。
    EnPlaces = 8,
    /// 帧项表（256 × `(off u32, len u32)` LE）——**烘焙侧算，运行侧只读**：
    /// 帧流自界规则只在提取器里实现一份，运行侧不再解析（避免第二处实现）。
    EnFrameOffs = 9,
    /// 位件项表（128 × `(off u32, len u32)` LE）。
    EnPlaceOffs = 10,
    /// 世界地图（`WorldMap $A53E`，32×32=1 KB；行=MapY 列=MapX，`$FF`=无房）。
    WorldMap = 11,
    // ---- 区域房流（Brinstar bank1 / Norfair bank2；指针表 + 4B 元tile 组 + 房流/结构体）----
    BrRoomPtrs = 12,
    BrStructPtrs = 13,
    BrMetatiles = 14,
    BrRoomStreams = 15,
    BrRoomOffs = 16,
    BrStructures = 17,
    BrStructOffs = 18,
    NoRoomPtrs = 19,
    NoStructPtrs = 20,
    NoMetatiles = 21,
    NoRoomStreams = 22,
    NoRoomOffs = 23,
    NoStructures = 24,
    NoStructOffs = 25,
    BrEnemyHealth = 26,
    BrEnemyRestingAnim = 27,
    BrEnemyActiveAnim = 28,
    BrEnemyActiveAnimInitOffset = 29,
    BrEnemyForceAttribs = 30,
    BrEnemyForceSpeedDelay = 31,
    BrEnemyDistanceThreshold = 32,
    BrEnemyInitDelay = 33,
    BrEnemyFacing = 34,
    RomTable968B = 35,
    RomVec95CE = 36,
    MusicNotes = 38,
    NoteLen4 = 39,
    NoteLen6 = 40,
    NoteLen7 = 41,
    SongStreams = 42,
    SongStreamOffs = 43,
    StreamBases = 44,
    EnvPtrs = 45,
    EnvBody = 46,
    EnvOffs = 47,
    SfxData = 48,
    PalPtrs = 49,
    PalData = 50,
    PalOffs = 51,
    SpriteData = 52,
    SndRegion = 53,
    BrItems = 54,
    NoItems = 55,
    KrItems = 56,
    ToItems = 57,
    RiItems = 58,
    NametableHi = 37,
}

impl PackKey {
    /// 包内数值 id。
    pub fn id(self) -> u16 {
        self as u16
    }

    /// 调试用名字（**只用于日志/错误信息，不参与匹配**）。
    pub fn name(self) -> &'static str {
        match self {
            PackKey::GfxInfo => "gfx.info",
            PackKey::GfxData => "gfx.data",
            PackKey::EnAnimTable => "en.anim",
            PackKey::EnFramePtrs1 => "en.fptr1",
            PackKey::EnFramePtrs2 => "en.fptr2",
            PackKey::EnPlacePtrs => "en.pptr",
            PackKey::EnFrames => "en.frames",
            PackKey::EnPlaces => "en.places",
            PackKey::EnFrameOffs => "en.foffs",
            PackKey::EnPlaceOffs => "en.poffs",
            PackKey::WorldMap => "world.map",
            PackKey::Meta => "meta.json",
            PackKey::BrRoomPtrs => "area.br.room_ptrs",
            PackKey::BrStructPtrs => "area.br.struct_ptrs",
            PackKey::BrMetatiles => "area.br.metatiles",
            PackKey::BrRoomStreams => "area.br.room_streams",
            PackKey::BrRoomOffs => "area.br.room_offs",
            PackKey::BrStructures => "area.br.structs",
            PackKey::BrStructOffs => "area.br.struct_offs",
            PackKey::NoRoomPtrs => "area.no.room_ptrs",
            PackKey::NoStructPtrs => "area.no.struct_ptrs",
            PackKey::NoMetatiles => "area.no.metatiles",
            PackKey::NoRoomStreams => "area.no.room_streams",
            PackKey::NoRoomOffs => "area.no.room_offs",
            PackKey::NoStructures => "area.no.structs",
            PackKey::NoStructOffs => "area.no.struct_offs",
            PackKey::BrEnemyHealth => "area.br.en_health",
            PackKey::BrEnemyRestingAnim => "area.br.en_rest_anim",
            PackKey::BrEnemyActiveAnim => "area.br.en_act_anim",
            PackKey::BrEnemyActiveAnimInitOffset => "area.br.en_act_init",
            PackKey::BrEnemyForceAttribs => "area.br.en_force_attr",
            PackKey::BrEnemyForceSpeedDelay => "area.br.en_force_speed",
            PackKey::BrEnemyDistanceThreshold => "area.br.en_dist_thr",
            PackKey::BrEnemyInitDelay => "area.br.en_init_delay",
            PackKey::BrEnemyFacing => "area.br.en_facing",
            PackKey::RomTable968B => "rom.tbl968b",
            PackKey::RomVec95CE => "rom.vec95ce",
            PackKey::MusicNotes => "sound.notes",
            PackKey::NoteLen4 => "sound.note_len4",
            PackKey::NoteLen6 => "sound.note_len6",
            PackKey::NoteLen7 => "sound.note_len7",
            PackKey::SongStreams => "sound.streams",
            PackKey::SongStreamOffs => "sound.stream_offs",
            PackKey::StreamBases => "sound.stream_bases",
            PackKey::EnvPtrs => "sound.env_ptrs",
            PackKey::EnvBody => "sound.env_body",
            PackKey::EnvOffs => "sound.env_offs",
            PackKey::SfxData => "sound.sfx_data",
            PackKey::PalPtrs => "pal.ptrs",
            PackKey::PalData => "pal.data",
            PackKey::PalOffs => "pal.offs",
            PackKey::SpriteData => "sprite.data",
            PackKey::SndRegion => "sound.region",
            PackKey::BrItems => "area.br.items",
            PackKey::NoItems => "area.no.items",
            PackKey::KrItems => "area.kr.items",
            PackKey::ToItems => "area.to.items",
            PackKey::RiItems => "area.ri.items",
            PackKey::NametableHi => "rom.nt_hi",
        }
    }

    /// 由数值 id 反查（未知 id ⇒ `None`——包里出现代码不认识的段不算错，只是用不到）。
    pub fn from_id(id: u16) -> Option<PackKey> {
        Some(match id {
            1 => PackKey::GfxInfo,
            2 => PackKey::GfxData,
            3 => PackKey::EnAnimTable,
            4 => PackKey::EnFramePtrs1,
            5 => PackKey::EnFramePtrs2,
            6 => PackKey::EnPlacePtrs,
            7 => PackKey::EnFrames,
            8 => PackKey::EnPlaces,
            9 => PackKey::EnFrameOffs,
            10 => PackKey::EnPlaceOffs,
            11 => PackKey::WorldMap,
            0 => PackKey::Meta,
            12 => PackKey::BrRoomPtrs,
            13 => PackKey::BrStructPtrs,
            14 => PackKey::BrMetatiles,
            15 => PackKey::BrRoomStreams,
            16 => PackKey::BrRoomOffs,
            17 => PackKey::BrStructures,
            18 => PackKey::BrStructOffs,
            19 => PackKey::NoRoomPtrs,
            20 => PackKey::NoStructPtrs,
            21 => PackKey::NoMetatiles,
            22 => PackKey::NoRoomStreams,
            23 => PackKey::NoRoomOffs,
            24 => PackKey::NoStructures,
            25 => PackKey::NoStructOffs,
            26 => PackKey::BrEnemyHealth,
            27 => PackKey::BrEnemyRestingAnim,
            28 => PackKey::BrEnemyActiveAnim,
            29 => PackKey::BrEnemyActiveAnimInitOffset,
            30 => PackKey::BrEnemyForceAttribs,
            31 => PackKey::BrEnemyForceSpeedDelay,
            32 => PackKey::BrEnemyDistanceThreshold,
            33 => PackKey::BrEnemyInitDelay,
            34 => PackKey::BrEnemyFacing,
            35 => PackKey::RomTable968B,
            36 => PackKey::RomVec95CE,
            38 => PackKey::MusicNotes,
            39 => PackKey::NoteLen4,
            40 => PackKey::NoteLen6,
            41 => PackKey::NoteLen7,
            42 => PackKey::SongStreams,
            43 => PackKey::SongStreamOffs,
            44 => PackKey::StreamBases,
            45 => PackKey::EnvPtrs,
            46 => PackKey::EnvBody,
            47 => PackKey::EnvOffs,
            48 => PackKey::SfxData,
            49 => PackKey::PalPtrs,
            50 => PackKey::PalData,
            51 => PackKey::PalOffs,
            52 => PackKey::SpriteData,
            53 => PackKey::SndRegion,
            54 => PackKey::BrItems,
            55 => PackKey::NoItems,
            56 => PackKey::KrItems,
            57 => PackKey::ToItems,
            58 => PackKey::RiItems,
            37 => PackKey::NametableHi,
            _ => return None,
        })
    }
}

/// 包错误（**全部 fail-loud**：不静默回落任何内置副本）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackError {
    /// 长度不足头+索引。
    Truncated { need: usize, got: usize },
    /// magic 不符（不是我们的包）。
    BadMagic([u8; 4]),
    /// 版本不符（旧包或新包）。
    BadVersion { expect: u16, got: u16 },
    /// ROM 身份不符（用户用了别的版本的 ROM 提取）。
    RomMismatch { expect_crc: u32, got_crc: u32 },
    /// 某个段的偏移/长度越界。
    BadEntry { key: u16, off: u32, len: u32, payload: usize },
    /// 某段 crc32 不符（包被截断/被拼接）。
    BadCrc { key: u16, expect: u32, got: u32 },
    /// 重复段 id。
    DuplicateKey(u16),
    /// 已初始化过（不同包）。
    AlreadyInitialized,
    /// 找不到包文件。
    NotFound { path: String },
}

impl std::fmt::Display for PackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PackError::Truncated { need, got } => write!(f, "包太短：需要 {need} B，实际 {got} B"),
            PackError::BadMagic(m) => write!(f, "不是资产包（magic={m:02X?}，应为 MTPK）"),
            PackError::BadVersion { expect, got } => write!(f, "包版本不符：期望 v{expect}，实际 v{got}"),
            PackError::RomMismatch { expect_crc, got_crc } => write!(
                f,
                "ROM 身份不符：包由 crc32={expect_crc:08X} 的 ROM 提取，代码期望 {got_crc:08X}"
            ),
            PackError::BadEntry { key, off, len, payload } => {
                write!(f, "段 {key} 越界：off={off} len={len} payload={payload}")
            }
            PackError::BadCrc { key, expect, got } => {
                write!(f, "段 {key} 校验失败：记录 crc32={expect:08X}，实算 {got:08X}")
            }
            PackError::DuplicateKey(k) => write!(f, "段 {k} 重复"),
            PackError::AlreadyInitialized => write!(f, "资产包已初始化（不同包被拒绝）"),
            PackError::NotFound { path } => write!(f, "找不到资产包：{path}"),
        }
    }
}

impl std::error::Error for PackError {}

/// 解析后的包视图（**纯结构**，不碰全局——单测可直接用）。
#[derive(Debug)]
pub struct PackView {
    bytes: &'static [u8],
    payload_off: usize,
    rom_crc32: u32,
    rom_size: u32,
    entries: Vec<(u16, u32, u32)>,
}

impl PackView {
    /// 解析包。`expect_rom_crc32` 非 `None` 时校验 ROM 身份。
    pub fn parse(bytes: &'static [u8], expect_rom_crc32: Option<u32>) -> Result<Self, PackError> {
        if bytes.len() < HEADER_LEN {
            return Err(PackError::Truncated { need: HEADER_LEN, got: bytes.len() });
        }
        let magic = [bytes[0], bytes[1], bytes[2], bytes[3]];
        if magic != MAGIC {
            return Err(PackError::BadMagic(magic));
        }
        let version = u16::from_le_bytes([bytes[4], bytes[5]]);
        if version != VERSION {
            return Err(PackError::BadVersion { expect: VERSION, got: version });
        }
        let rom_crc32 = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
        let rom_size = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
        if let Some(expect) = expect_rom_crc32 {
            if expect != rom_crc32 {
                return Err(PackError::RomMismatch { expect_crc: rom_crc32, got_crc: expect });
            }
        }
        let section_count = u16::from_le_bytes([bytes[16], bytes[17]]) as usize;
        let payload_off = u32::from_le_bytes([bytes[28], bytes[29], bytes[30], bytes[31]]) as usize;
        let idx_end = HEADER_LEN + section_count * ENTRY_LEN;
        if bytes.len() < idx_end {
            return Err(PackError::Truncated { need: idx_end, got: bytes.len() });
        }
        let mut entries = Vec::with_capacity(section_count);
        for i in 0..section_count {
            let base = HEADER_LEN + i * ENTRY_LEN;
            let key = u16::from_le_bytes([bytes[base], bytes[base + 1]]);
            let off = u32::from_le_bytes([bytes[base + 4], bytes[base + 5], bytes[base + 6], bytes[base + 7]]);
            let len = u32::from_le_bytes([bytes[base + 8], bytes[base + 9], bytes[base + 10], bytes[base + 11]]);
            let crc = u32::from_le_bytes([
                bytes[base + 12],
                bytes[base + 13],
                bytes[base + 14],
                bytes[base + 15],
            ]);
            let start = payload_off
                .checked_add(off as usize)
                .ok_or(PackError::BadEntry { key, off, len, payload: bytes.len() })?;
            let end = start
                .checked_add(len as usize)
                .ok_or(PackError::BadEntry { key, off, len, payload: bytes.len() })?;
            if end > bytes.len() {
                return Err(PackError::BadEntry { key, off, len, payload: bytes.len() });
            }
            let got = crc32(&bytes[start..end]);
            if got != crc {
                return Err(PackError::BadCrc { key, expect: crc, got });
            }
            if entries.iter().any(|(k, _, _)| *k == key) {
                return Err(PackError::DuplicateKey(key));
            }
            entries.push((key, off, len));
        }
        Ok(PackView { bytes, payload_off, rom_crc32, rom_size, entries })
    }

    /// 取段（未命中 ⇒ `None`）。
    pub fn get(&self, key: PackKey) -> Option<&'static [u8]> {
        let (_, off, len) = self.entries.iter().find(|(k, _, _)| *k == key.id())?;
        let start = self.payload_off + *off as usize;
        Some(&self.bytes[start..start + *len as usize])
    }

    /// 包声明的 ROM 身份（`(crc32, size)`）。
    pub fn rom_identity(&self) -> (u32, u32) {
        (self.rom_crc32, self.rom_size)
    }

    /// 已解析段数。
    pub fn section_count(&self) -> usize {
        self.entries.len()
    }

    /// 段清单（id/off/len）——`pack-tool list` 用。
    pub fn entries(&self) -> &[(u16, u32, u32)] {
        &self.entries
    }
}

static PACK: OnceLock<PackView> = OnceLock::new();

/// 进程级初始化（幂等：同包重复调用返回 `Ok`）。
///
/// 包字节 `Box::leak` 成 `&'static`——访问器才能返回 `&'static [u8]`，
/// 从而让 180+ 个数据访问点只改 `CONST` → `fn()`，不必把 `&Pack` 穿进每一层。
/// 代价：一次进程泄漏（包体量 MB 级，游戏单实例，可接受）。
pub fn init_from_bytes(bytes: Vec<u8>) -> Result<(), PackError> {
    let leaked: &'static [u8] = Box::leak(bytes.into_boxed_slice());
    let crc = u32::from_le_bytes([leaked[8], leaked[9], leaked[10], leaked[11]]);
    let view = PackView::parse(leaked, Some(crc))?;
    match PACK.set(view) {
        Ok(()) => Ok(()),
        Err(_) => Ok(()), // 已初始化：视为幂等成功（同包/异包都只保留首个）
    }
}

/// 从环境变量 `METROID_PACK` 或默认路径加载。
///
/// 默认路径顺序：`$METROID_PACK` → `./assets/metroid.pack` → 可执行文件同级 `metroid.pack`。
pub fn init_from_env() -> Result<(), PackError> {
    let mut cands: Vec<String> = Vec::new();
    if let Ok(p) = std::env::var("METROID_PACK") {
        cands.push(p);
    }
    cands.push("assets/metroid.pack".to_string());
    // 开发/测试便利：编译期记录的源目录（`cargo test` 的 CWD 是各 crate 根，不是工作区根）。
    // 安装后的二进制没有这个目录，命中不到就跳过——不影响发行形态。
    cands.push(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/metroid.pack").to_string());
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            cands.push(dir.join("metroid.pack").to_string_lossy().to_string());
            cands.push(dir.join("assets/metroid.pack").to_string_lossy().to_string());
        }
    }
    for c in &cands {
        if let Ok(bytes) = std::fs::read(c) {
            return init_from_bytes(bytes);
        }
    }
    Err(PackError::NotFound { path: cands.join(" | ") })
}

/// 是否已就绪。
pub fn is_ready() -> bool {
    PACK.get().is_some()
}

/// 惰性初始化：首次取段时若未就绪，尝试 [`init_from_env`]。
///
/// 二进制入口仍会**显式**调一次 `init_from_env`（fail-fast 退出码 3）；本函数只是
/// 保证单测与工具不必各自记得初始化，且行为与显式初始化完全一致（同一候选路径表）。
fn ensure_loaded() {
    if !is_ready() {
        let _ = init_from_env();
    }
}

/// 取段（未初始化/缺段 ⇒ panic，并打印**如何生成包**）。
pub fn get(key: PackKey) -> &'static [u8] {
    ensure_loaded();
    match try_get(key) {
        Some(s) => s,
        None if !is_ready() => panic!(
            "资产包未加载：缺少段 `{}`。\n\
             生成方法：cargo run -p pack-tool -- extract --rom \"rom/Metroid (USA).nes\" --out assets/metroid.pack\n\
             （或设置环境变量 METROID_PACK 指向已有包；设计见 docs/spec/data-code-separation.md）",
            key.name()
        ),
        None => panic!(
            "资产包缺少段 `{}`（包版本可能落后于代码）。\n\
             重新生成：cargo run -p pack-tool -- extract --rom \"rom/Metroid (USA).nes\" --out assets/metroid.pack",
            key.name()
        ),
    }
}

/// 取段（未命中 ⇒ `None`，测试用）。
pub fn try_get(key: PackKey) -> Option<&'static [u8]> {
    PACK.get()?.get(key)
}

/// crc32（IEEE，与 `gates/cli.py` 同多项式；零依赖实现）。
/// `GFX_INFO` 描述子 → 该项在 `gfx.data` 平面里的 `(偏移, 尺寸)`。
///
/// **格式知识在提取侧**：描述子 7 B = `[bank][src lo][src hi][dest lo][dest hi][size lo][size hi]`
/// （`GFXInfo ; 07:C6E0`），尺寸取 `+5..+7` LE，偏移按前序项尺寸累加——
/// 这两条约定只由 `extract.rs` 的 `extract_gfx_info`/`extract_gfx_data` 定义。
pub fn gfx_block_offs(id: usize) -> Option<(usize, usize)> {
    let info = get(PackKey::GfxInfo);
    if id >= info.len() / 7 {
        return None;
    }
    let mut off = 0usize;
    for i in 0..=id {
        let rec = info.get(i * 7..i * 7 + 7)?;
        let size = usize::from(u16::from_le_bytes([rec[5], rec[6]]));
        if i == id {
            return Some((off, size));
        }
        off += size;
    }
    None
}

/// 项表（`(off u32, len u32)` LE）第 `idx` 项——越界 ⇒ `None`。
pub fn offs_entry(key: PackKey, idx: usize) -> Option<(u32, u32)> {
    let t = get(key);
    let e = t.get(idx * 8..idx * 8 + 8)?;
    Some((
        u32::from_le_bytes([e[0], e[1], e[2], e[3]]),
        u32::from_le_bytes([e[4], e[5], e[6], e[7]]),
    ))
}

pub fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 合成包构造（与 `extract::write_pack` 同格式，但数据可任意——测的是格式不是 ROM）。
    fn build_pack(rom_crc32: u32, rom_size: u32, sections: &[(u16, Vec<u8>)]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&MAGIC);
        out.extend_from_slice(&VERSION.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&rom_crc32.to_le_bytes());
        out.extend_from_slice(&rom_size.to_le_bytes());
        out.extend_from_slice(&(sections.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes()); // reserved2
        out.extend_from_slice(&0u32.to_le_bytes()); // reserved3
        let payload_off = (HEADER_LEN + sections.len() * ENTRY_LEN) as u32;
        out.extend_from_slice(&payload_off.to_le_bytes()); // 0x20
        assert_eq!(out.len(), HEADER_LEN);
        let mut off = 0u32;
        for (key, data) in sections {
            out.extend_from_slice(&key.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&off.to_le_bytes());
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(&crc32(data).to_le_bytes());
            out.extend_from_slice(&0u32.to_le_bytes());
            off += data.len() as u32;
        }
        for (_, data) in sections {
            out.extend_from_slice(data);
        }
        out
    }

    fn leak(v: Vec<u8>) -> &'static [u8] {
        Box::leak(v.into_boxed_slice())
    }

    #[test]
    fn parses_and_returns_sections() {
        let a = vec![1u8, 2, 3, 4];
        let b = vec![9u8; 300];
        let bytes = leak(build_pack(0xDEAD_BEEF, 245_776, &[(1, a.clone()), (2, b.clone())]));
        let p = PackView::parse(bytes, Some(0xDEAD_BEEF)).expect("解析应成功");
        assert_eq!(p.get(PackKey::GfxInfo), Some(&a[..]));
        assert_eq!(p.get(PackKey::GfxData), Some(&b[..]));
        assert_eq!(p.rom_identity(), (0xDEAD_BEEF, 245_776));
        assert_eq!(p.section_count(), 2);
    }

    #[test]
    fn rejects_wrong_rom_identity() {
        let bytes = leak(build_pack(0x1111_2222, 1, &[(1, vec![7u8; 8])]));
        match PackView::parse(bytes, Some(0x3333_4444)) {
            Err(PackError::RomMismatch { expect_crc, got_crc }) => {
                assert_eq!((expect_crc, got_crc), (0x1111_2222, 0x3333_4444));
            }
            other => panic!("应报 RomMismatch，实际 {other:?}"),
        }
    }

    #[test]
    fn rejects_bad_magic_and_version() {
        let mut bytes = build_pack(0, 0, &[(1, vec![0u8; 4])]);
        bytes[0] = b'X';
        match PackView::parse(leak(bytes), None) {
            Err(PackError::BadMagic(_)) => {}
            other => panic!("应报 BadMagic，实际 {other:?}"),
        }
        let mut bytes = build_pack(0, 0, &[(1, vec![0u8; 4])]);
        bytes[4] = 0xFF;
        match PackView::parse(leak(bytes), None) {
            Err(PackError::BadVersion { .. }) => {}
            other => panic!("应报 BadVersion，实际 {other:?}"),
        }
    }

    #[test]
    fn rejects_corrupted_payload() {
        let mut bytes = build_pack(0, 0, &[(1, vec![1u8, 2, 3, 4])]);
        let last = bytes.len() - 1;
        bytes[last] ^= 0xFF; // 改一个字节 ⇒ crc 必须报
        match PackView::parse(leak(bytes), None) {
            Err(PackError::BadCrc { key, .. }) => assert_eq!(key, 1),
            other => panic!("应报 BadCrc，实际 {other:?}"),
        }
    }

    #[test]
    fn missing_section_is_none_not_panic() {
        let bytes = leak(build_pack(0, 0, &[(1, vec![5u8; 4])]));
        let p = PackView::parse(bytes, None).expect("解析应成功");
        assert!(p.get(PackKey::GfxData).is_none(), "缺段应为 None（访问器负责 panic）");
    }

    #[test]
    fn crc32_matches_known_vector() {
        // "123456789" 的 IEEE crc32 = 0xCBF43926（标准测试向量）
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }
}
