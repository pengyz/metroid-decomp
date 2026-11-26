//! 零页敌 AI / 结局链 union 面 `$007A-$0090`（23 B）的 typed 声明层（卡 #131）。
//!
//! 【这一面是什么】`constants_ram.asm:643-672` 的 **`.union` 段**：同一段地址在正常游戏里是
//! 敌 AI 的运行变量（Kraid/Ridley 开火计数、敌移动指针、喷弹参数、Rinka 生成器），
//! 在结局链里被**整段复用**为结局脚本变量（`EndMsgWrite` … `WaveSpriteCounter`）。
//! 它长期挂在 `nes_map` 的 `Undeclared` 上：账面上「有人按地址读写，但没人认领语义」。
//!
//! 【本面只做两件事】①把 asm 的 union 布局写成 typed 结构（每字节给**两个**名字与出处）；
//! ②把这 23 B 登记成 `Owner`。**行为零变化**：`enemy/`、`world/`、`ending/` 的读写路径
//! 一行不动，本面不接管任何访问。
//!
//! 【为什么能升 Owner（证据级 A）】
//! - 左值（敌 AI 面）：`constants_ram.asm:643-672` 逐字节具名；运行期写点如
//!   `prg7_engine.asm:9628 sta MellowRandomNumber`、`:5122 sta OnFrozenEnemy`、
//!   `:11328 sta Enemy82`、`:11812 lda SpawnEnProjectile_AnimIndex,y`、
//!   `:11735 lda SpawnEnProjectile_ExpectedStatus`；`prg3_tourian.asm:846/848`
//!   以 `RinkaSpawners.0/.1` 形式取槽偏移。
//! - 右值（结局面）：`constants_ram.asm:676-687` 的第二个 union 分支逐字节具名。
//! - 整段初始化写：`prg7_engine.asm:1862 sta SpareMem7A,x`（MoreInit 清 `$7A-$DE`，
//!   含 asm 自述的 off-by-one：应为 `$7A-$DF`）。
//!
//! 【没收录的相邻字节（有意）】
//! - `$0091-$00FB`：仍留 `Undeclared`，按「一次一片、可实测下降」的规矩留给后续批次；
//!   `$00FE-$00FD` 已由 `world/room_ram.rs` 等标 `Unmodeled`。
//!
//! 【命名纪律】`spare_*` 类字节（`$7A/$7B-$7C/$89`）只锁地址与 asm 名，不编造语义；
//! Kraid/Ridley 三字节在 `prg7` 无直接具名读写点（引擎按 `$7E,x` 间接访问），
//! 故其语义只按 asm 注释登记，不当作行为断言。

use crate::memmap::nesmap::{NesMapped, NesSpan, SpanKey};
use crate::memmap::storage::NesImage;

/// 段基址 `$007A`（union 首字节）。
pub const BASE: u16 = 0x007A;
/// 段长 23 B（`$007A-$0090`，含 `RinkaSpawners.1` 的 3 字节）。
pub const FACE_LEN: usize = 0x17;

/// 逐字节命名表：`(地址, 敌 AI 面名, 结局面名, 证据)`。
///
/// 结局面名在 `$85` 之后为空（第二个 union 分支到 `WaveSpriteCounter`=`$84` 结束）。
pub const SYMBOLS: [(u16, &str, &str, &str); FACE_LEN] = [
    (0x007A, "SpareMem7A", "EndMsgWrite", "constants_ram.asm:643,676"),
    (0x007B, "SpareMem7B", "IsCredits", "constants_ram.asm:645,677"),
    (0x007C, "SpareMem7B+1", "SpriteByteCounter", "constants_ram.asm:645,678"),
    (0x007D, "OnFrozenEnemy", "SpritePointerIndex", "constants_ram.asm:647,679"),
    (0x007E, "KraidLintCounter", "SpriteAttribByte", "constants_ram.asm:649,680"),
    (0x007F, "KraidNailCounter", "ColorCntIndex", "constants_ram.asm:650,681"),
    (0x0080, "RidleyFireballCounter", "CreditPageNumber", "constants_ram.asm:651,682"),
    (0x0081, "EnemyMovementPtr", "HideShowEndMsg", "constants_ram.asm:653,683"),
    (0x0082, "EnemyMovementPtr+1", "ClrChangeCounter", "constants_ram.asm:653,684"),
    (0x0083, "SpawnEnProjectile_AnimIndex", "WaveSpritePointer", "constants_ram.asm:658,685"),
    (0x0084, "SpawnEnProjectile_AnimIndex+1", "WaveSpriteCounter", "constants_ram.asm:658,686"),
    (0x0085, "SpawnEnProjectile_AnimTableIndex", "", "constants_ram.asm:660"),
    (0x0086, "EnemyFlipAfterDisplacementAnimIndex+1", "", "constants_ram.asm:661"),
    (0x0087, "SpawnEnProjectile_ExpectedStatus", "", "constants_ram.asm:663"),
    (0x0088, "SpawnEnProjectile_EnData0A", "", "constants_ram.asm:665"),
    (0x0089, "SpareMem89", "", "constants_ram.asm:667"),
    (0x008A, "MellowRandomNumber", "", "constants_ram.asm:669"),
    (0x008B, "RinkaSpawners.0.status", "", "constants_ram.asm:672"),
    (0x008C, "RinkaSpawners.0.hi", "", "constants_ram.asm:672"),
    (0x008D, "RinkaSpawners.0.posIndex", "", "constants_ram.asm:672"),
    (0x008E, "RinkaSpawners.1.status", "", "constants_ram.asm:672"),
    (0x008F, "RinkaSpawners.1.hi", "", "constants_ram.asm:672"),
    (0x0090, "RinkaSpawners.1.posIndex", "", "constants_ram.asm:672"),
];

/// 零页敌 AI / 结局链 union 面的声明层。
///
/// 字段即字节：`decode`/`encode` 与 `get_byte`/`set_byte` 互为镜像，均以 `&0x7FF`
/// 归一地址访问（与 `snapshot_nes()` 同源同值）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EnemyAiZeroPageFace {
    /// `$7A`：敌 AI 面为 spare（MoreInit 清零，asm 自述 never used）；结局面是 `EndMsgWrite`。
    pub spare_7a: u8,
    /// `$7B-$7C`：敌 AI 面 `SpareMem7B`（word）；结局面 `IsCredits` + `SpriteByteCounter`。
    pub spare_7b: u16,
    /// `$7D`：`OnFrozenEnemy`（`prg7:5122` 写 0/1、`:5143` inc）。
    pub on_frozen_enemy: u8,
    /// `$7E`：Kraid 开火计数（引擎按 `$7E,x` 间接访问，读点未具名 ⇒ 只锁地址）。
    pub kraid_lint_counter: u8,
    /// `$7F`：Kraid 钉刺计数（同上）。
    pub kraid_nail_counter: u8,
    /// `$80`：Ridley 火球计数（同上）。
    pub ridley_fireball_counter: u8,
    /// `$81-$82`：敌移动指针（`.dw`）；`$81` 亦作 `EnemyStatusPreAI`、`$82` 作 `Enemy82`。
    pub enemy_movement_ptr: u16,
    /// `$83-$84`：喷弹动画索引（`.dw`）。
    pub spawn_en_projectile_anim_index: u16,
    /// `$85`：`SpawnEnProjectile_AnimTableIndex`（`prg7:11759` 读）；同一字节亦作
    /// `EnemyFlipAfterDisplacementAnimIndex` 低字节。
    pub spawn_en_projectile_anim_table_index: u8,
    /// `$86`：`EnemyFlipAfterDisplacementAnimIndex` 高字节（`prg7:12306` 按 `,y` 读）。
    pub enemy_flip_after_displacement_anim_index_hi: u8,
    /// `$87`：喷弹期望状态（`prg7:11735` 读；asm 注 bit7 恒 0 ⇒ 该功能未使用）。
    pub spawn_en_projectile_expected_status: u8,
    /// `$88`：喷弹 `EnData0A`（`prg7:11838` 读）。
    pub spawn_en_projectile_en_data0a: u8,
    /// `$89`：spare。
    pub spare_89: u8,
    /// `$8A`：Mellow 随机数（`prg7:9628` 写、`:9655` 加、`:12607-12609` 再算）。
    pub mellow_random_number: u8,
    /// `$8B-$90`：2 槽 × 3 B 的 Rinka 生成器（`prg3:846/848` 取槽偏移）。
    pub rinka_spawners: [u8; 6],
}

impl EnemyAiZeroPageFace {
    /// 该地址是否落本面（`$007A-$0090`）。
    pub fn contains(addr: u16) -> bool {
        addr >= BASE && (addr as usize) < BASE as usize + FACE_LEN
    }

    /// 字节 → 字段。
    pub fn decode(bytes: &[u8; FACE_LEN]) -> Self {
        Self {
            spare_7a: bytes[0x00],
            spare_7b: u16::from_le_bytes([bytes[0x01], bytes[0x02]]),
            on_frozen_enemy: bytes[0x03],
            kraid_lint_counter: bytes[0x04],
            kraid_nail_counter: bytes[0x05],
            ridley_fireball_counter: bytes[0x06],
            enemy_movement_ptr: u16::from_le_bytes([bytes[0x07], bytes[0x08]]),
            spawn_en_projectile_anim_index: u16::from_le_bytes([bytes[0x09], bytes[0x0A]]),
            spawn_en_projectile_anim_table_index: bytes[0x0B],
            enemy_flip_after_displacement_anim_index_hi: bytes[0x0C],
            spawn_en_projectile_expected_status: bytes[0x0D],
            spawn_en_projectile_en_data0a: bytes[0x0E],
            spare_89: bytes[0x0F],
            mellow_random_number: bytes[0x10],
            rinka_spawners: [
                bytes[0x11],
                bytes[0x12],
                bytes[0x13],
                bytes[0x14],
                bytes[0x15],
                bytes[0x16],
            ],
        }
    }

    /// 字段 → 字节（与 `decode` 互为逆）。
    pub fn encode(self) -> [u8; FACE_LEN] {
        let spare_7b = self.spare_7b.to_le_bytes();
        let enemy_movement_ptr = self.enemy_movement_ptr.to_le_bytes();
        let spawn_anim = self.spawn_en_projectile_anim_index.to_le_bytes();
        [
            self.spare_7a,
            spare_7b[0],
            spare_7b[1],
            self.on_frozen_enemy,
            self.kraid_lint_counter,
            self.kraid_nail_counter,
            self.ridley_fireball_counter,
            enemy_movement_ptr[0],
            enemy_movement_ptr[1],
            spawn_anim[0],
            spawn_anim[1],
            self.spawn_en_projectile_anim_table_index,
            self.enemy_flip_after_displacement_anim_index_hi,
            self.spawn_en_projectile_expected_status,
            self.spawn_en_projectile_en_data0a,
            self.spare_89,
            self.mellow_random_number,
            self.rinka_spawners[0],
            self.rinka_spawners[1],
            self.rinka_spawners[2],
            self.rinka_spawners[3],
            self.rinka_spawners[4],
            self.rinka_spawners[5],
        ]
    }

    /// 按绝对地址取一个字节（面外返回 `None`）。
    pub fn get_byte(self, addr: u16) -> Option<u8> {
        let idx = addr.checked_sub(BASE)? as usize;
        (idx < FACE_LEN).then(|| self.encode()[idx])
    }

    /// 按绝对地址写一个字节（面外返回 `false`）。
    pub fn set_byte(&mut self, addr: u16, value: u8) -> bool {
        let Some(idx) = addr.checked_sub(BASE) else {
            return false;
        };
        if idx as usize >= FACE_LEN {
            return false;
        }
        let mut bytes = self.encode();
        bytes[idx as usize] = value;
        *self = Self::decode(&bytes);
        true
    }
}

impl NesMapped for EnemyAiZeroPageFace {
    /// 本段是 union 的「左值」与「右值」共用字节，没有单一判别源 ⇒ 无判别字段。
    const DISCRIMINATOR: Option<u16> = None;

    /// 整段连续 23 B 登记为 `Owner`：段内每字节都有 asm 具名（含 `spare_*` 与 union 右值）。
    const SPANS: &'static [(NesSpan, SpanKey)] =
        &[(NesSpan::value(BASE, FACE_LEN as u16), SpanKey::Owner)];

    fn project(&self, out: &mut NesImage) {
        let ram_len = out.ram.len();
        for (offset, byte) in self.encode().iter().enumerate() {
            let index = (BASE as usize + offset) & (ram_len - 1);
            out.ram[index] = *byte;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 几何与声明：只有一段 23 B `Owner`，且 `contains` 边界正确。
    #[test]
    fn geometry_and_span() {
        assert_eq!(FACE_LEN, 0x17);
        assert_eq!(EnemyAiZeroPageFace::SPANS.len(), 1);
        let (span, key) = &EnemyAiZeroPageFace::SPANS[0];
        assert_eq!(*span, NesSpan::value(0x007A, 23));
        assert!(matches!(key, SpanKey::Owner));
        assert!(EnemyAiZeroPageFace::contains(0x007A));
        assert!(EnemyAiZeroPageFace::contains(0x0090));
        assert!(!EnemyAiZeroPageFace::contains(0x0079));
        assert!(!EnemyAiZeroPageFace::contains(0x0091));
    }

    /// 命名表覆盖全部 23 B、地址连续、证据列非空；union 右值只到 `$84`。
    #[test]
    fn symbols_are_complete_and_ordered() {
        assert_eq!(SYMBOLS.len(), FACE_LEN);
        for (i, (addr, name, union, evidence)) in SYMBOLS.iter().enumerate() {
            assert_eq!(*addr, BASE + i as u16, "第 {i} 项地址不连续");
            assert!(!name.is_empty(), "${addr:04X} 缺敌 AI 面名");
            assert!(!evidence.is_empty(), "${addr:04X} 缺证据指针");
            if *addr > 0x0084 {
                assert!(union.is_empty(), "${addr:04X} 不应有结局面名");
            } else {
                assert!(!union.is_empty(), "${addr:04X} 缺结局面名");
            }
        }
    }

    /// 证据行号可复核：每个 `SYMBOLS` 项的证据所引 asm 行必须真的含该符号名。
    ///
    /// 这条是 2026-11-27 对影复核后补的：此前结局面行号整体偏 19 行（把第一分支的
    /// `SpawnEnProjectile_*` 区写成结局面），「证据非空」抓不到，必须逐行读文件。
    /// `refs/` 是 submodule，缺失时显式跳过（不静默通过：打印 SKIP 原因）。
    #[test]
    fn symbols_evidence_lines_name_their_symbol() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../refs/m1disasm/SRC/constants_ram.asm"
        );
        let asm = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("SKIP symbols_evidence_lines_name_their_symbol：读不到 {path}（{e}），refs 子模块未初始化");
                return;
            }
        };
        let lines: Vec<&str> = asm.lines().collect();
        for (addr, ai_name, end_name, evidence) in SYMBOLS.iter() {
            let nums: Vec<usize> = evidence
                .split_once("constants_ram.asm:")
                .and_then(|(_, rest)| {
                    rest.split(',')
                        .map(|n| n.trim().parse::<usize>().ok())
                        .collect::<Option<Vec<_>>>()
                })
                .unwrap_or_else(|| panic!("${addr:04X} 证据格式应为 constants_ram.asm:A[,B]：{evidence}"));
            let line = |n: usize| -> &str {
                lines
                    .get(n - 1)
                    .unwrap_or_else(|| panic!("${addr:04X} 证据行号 {n} 超出文件（{} 行）", lines.len()))
            };
            let base = |s: &str| s.split(['+', '.']).next().unwrap_or(s).to_string();
            assert!(
                line(nums[0]).contains(&base(ai_name)),
                "${addr:04X} 证据行 {} 不含敌 AI 名 {}：{:?}（行号可能偏移）",
                nums[0], ai_name, line(nums[0])
            );
            if !end_name.is_empty() {
                let n = *nums.get(1).unwrap_or(&nums[0]);
                assert!(
                    line(n).contains(&base(end_name)),
                    "${addr:04X} 证据行 {} 不含结局面名 {}：{:?}（行号可能偏移）",
                    n, end_name, line(n)
                );
            }
        }
    }

    /// decode/encode 往返：随机字节面必须逐字节还原。
    #[test]
    fn encode_decode_roundtrip() {
        let mut bytes = [0u8; FACE_LEN];
        for (i, b) in bytes.iter_mut().enumerate() {
            *b = (i as u8).wrapping_mul(37).wrapping_add(11);
        }
        let face = EnemyAiZeroPageFace::decode(&bytes);
        assert_eq!(face.encode(), bytes);
        // 两字节字段按小端拼装。
        assert_eq!(face.spare_7b, u16::from_le_bytes([bytes[1], bytes[2]]));
        assert_eq!(face.on_frozen_enemy, bytes[3]);
        assert_eq!(face.spawn_en_projectile_anim_index, u16::from_le_bytes([bytes[9], bytes[10]]));
    }

    /// `get_byte`/`set_byte` 与面域边界。
    #[test]
    fn byte_accessors_are_bounded() {
        let mut face = EnemyAiZeroPageFace::default();
        assert!(face.set_byte(0x008A, 0x5A));
        assert_eq!(face.get_byte(0x008A), Some(0x5A));
        assert_eq!(face.mellow_random_number, 0x5A);
        assert!(face.set_byte(0x008B + 5, 0x01));
        assert_eq!(face.rinka_spawners[5], 0x01);
        assert!(!face.set_byte(0x0091, 1));
        assert_eq!(face.get_byte(0x0079), None);
    }
}
