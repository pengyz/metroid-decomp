//! 敌人 AI 派发表——**按区 bank**（每个区 bank 各有一张 `ChooseEnemyAIRoutine` 跳表）。
//!
//! # 为什么必须分区
//!
//! asm 里 `ChooseEnemyAIRoutine` 是**每区一份**，同一个 `type` 号在不同区的含义不同：
//!
//! | bank | 地址 | type 00 | 02 | 04 | 06 | 0B |
//! |---|---|---|---|---|---|---|
//! | Brinstar `01` | `01:95E5` | Sidehopper floor | Waver | Skree | Rio | Null（空指针跳 `$0000`：行为未定，C 级） |
//! | Norfair `02` | `02:95E5` | Swooper(未见萨) | Ripper II | **RemoveEnemy** | Crawler | **Squeept** |
//! | Tourian `03` | `03:95E5` | Metroid(红) | `L9A27` | **Rinka** | RemoveEnemy | RemoveEnemy |
//! | Kraid `04` | `04:95E5` | Sidehopper floor | `RTS_95CB` | Skree | **`RTS_95CB`** | `RTS_95CB` |
//! | Ridley `05` | `05:95E5` | Swooper(未见萨) | **Sidehopper floor** | RemoveEnemy | Crawler | RemoveEnemy |
//!
//! 之前 Rust 侧用**单张全局表**（`match t { 0|1 => sidehopper, 2 => waver, 3 => ripper,
//! 4 => skree, 5 => crawler }`）——在 Brinstar 恰好正确，在其余四区是错的
//! （例：Norfair type 4 应 `RemoveEnemy`，旧实现会跑 Skree AI）。
//!
//! # 三种「无事可做」
//!
//! - `RemoveEnemy ; 07:FA18` = `lda #enemyStatus_NoEnemy / sta EnsExtra.0.status,x / rts`
//!   （清槽，本件已实现）；
//! - `RTS_95CB` = 直接 `rts`（Kraid bank4 的占位项，**什么都不做**）；
//! - `$0000`（Brinstar 0B–0F）= 跳表里是空指针 ⇒ 原版硬崩；本件按 fail-loud 记
//!   `Null` 并走占位臂，保留「原版会崩」这一事实的可观测性。

/// 一张 16 项跳表里的单项。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Routine {
    SidehopperFloor,
    SidehopperCeiling,
    Waver,
    /// `RipperAIRoutine`（bank1/4：复用 Sidehopper 尾件）。
    RipperI,
    /// `RipperAIRoutine`（bank2 = Ripper II：喷焰，帧步进 3）。
    RipperII,
    Skree,
    Crawler,
    /// `RioAIRoutine`（bank1 `01:9AF5`，仅 Brinstar 生成）。
    Rio,
    /// `PipeBugAIRoutine`（bank1/2/4/5 各一份，速度表按区不同）。
    PipeBug,
    /// `SqueeptAIRoutine`（bank2 `02:9A64`，仅 Norfair）。
    Squeept,
    /// `SwooperAIRoutine00`（bank2/5 共件 `02:95E5` 0 号；本卡只接 Norfair）。
    Swooper00,
    /// `SwooperAIRoutine01`（同件 1 号；本卡只接 Norfair）。
    Swooper01,
    /// `MultiviolaAIRoutine`（bank2 `02:9A??`；bank5 同件，本卡只接 Norfair）。
    Multiviola,
    /// `DragonAIRoutine`（bank2 `02:9AE9`，**仅 Norfair include**——`prg2_norfair.asm:683`）。
    Dragon,
    /// `PolypAIRoutine`（Norfair `02:9B64` / Ridley `05:9B16`，`prg2_norfair.asm:687` +
    /// `prg5_ridley.asm:658` 各 include 一份；两支行为不同，本卡只接 Norfair）。
    Polyp,
    /// `KraidAIRoutine`（bank4 `04:9AB4`，`prg4_kraid.asm:757` include）。
    Kraid,
    /// `KraidLintAIRoutine`（bank4 `04:9AE4`）——Kraid 的手部弹体（槽 1..5）。
    KraidLint,
    /// `KraidNailAIRoutine`（bank4 `04:9B2C`）——asm 正文只有 `jmp KraidLintAIRoutine`。
    KraidNail,
    /// `RinkaAIRoutine`（bank3 `03:9A2C`，仅 Tourian include；`SetRinkaSpeed_{AREA}` 按区）。
    Rinka,
    /// `RemoveEnemy ; 07:FA18`——清槽。
    RemoveEnemy,
    /// `RTS_95CB`——直接返回，什么都不做（Kraid bank4 占位）。
    Rts95CB,
    /// 跳表里的空指针（Brinstar 0B–0F）——原版跳 `$0000`（零页，行为未定；C 级）。
    Null,
    /// 尚未端口的例程（保留 asm label 名，用于 fail-loud 记账）。
    Unported(&'static str),
}

use Routine::*;

/// `prg1_brinstar.asm:157-173`（`ChooseEnemyAIRoutine ; 01:95E5`）。
const BRINSTAR: [Routine; 16] = [
    SidehopperFloor,
    SidehopperCeiling,
    Waver,
    RipperI,
    Skree,
    Crawler,
    Rio,
    PipeBug,
    Kraid,
    KraidLint,
    KraidNail,
    Null,
    Null,
    Null,
    Null,
    Null,
];

/// `prg2_norfair.asm:157-172`（`02:95E5`）。
const NORFAIR: [Routine; 16] = [
    Swooper00,
    Swooper01,
    RipperII,
    RemoveEnemy,
    RemoveEnemy,
    RemoveEnemy,
    Crawler,
    PipeBug,
    RemoveEnemy,
    RemoveEnemy,
    RemoveEnemy,
    Squeept,
    Multiviola,
    // Dragon/Polyp 射弹径依赖 EnProjectile 子系统（SpawnEnProjectile `07:F870` +
    // UpdateAllEnProjectiles `07:F93B`）——该子系统已端口（`enemy::en_projectile`），
    // 故本区这两格由 `Unported` 换成实变体。
    // 注意：0x0D/0x0E 在 **Ridley 表**里是另外的语义（bank5 另有 Polyp 一份、行为不同）——
    // 本卡只改 Norfair 这张表。
    Dragon,
    Polyp,
    RemoveEnemy,
];

/// `prg3_tourian.asm:166-181`（`03:95E5`）。
const TOURIAN: [Routine; 16] = [
    Unported("MetroidAIRoutine"),
    Unported("MetroidAIRoutine"),
    Unported("L9A27"),
    RemoveEnemy,
    Rinka,
    RemoveEnemy,
    RemoveEnemy,
    RemoveEnemy,
    RemoveEnemy,
    RemoveEnemy,
    RemoveEnemy,
    RemoveEnemy,
    RemoveEnemy,
    RemoveEnemy,
    RemoveEnemy,
    RemoveEnemy,
];

/// `prg4_kraid.asm:163-178`（`04:95E5`）——未用项是 `RTS_95CB`（直接 rts）。
const KRAID: [Routine; 16] = [
    SidehopperFloor,
    SidehopperCeiling,
    Rts95CB,
    RipperI,
    Skree,
    Crawler,
    Rts95CB,
    PipeBug,
    Kraid,
    KraidLint,
    KraidNail,
    Rts95CB,
    Rts95CB,
    Rts95CB,
    Rts95CB,
    Rts95CB,
];

/// `prg5_ridley.asm:164-179`（`05:95E5`）。
const RIDLEY: [Routine; 16] = [
    Unported("SwooperAIRoutine00"),
    Unported("SwooperAIRoutine01"),
    SidehopperFloor,
    SidehopperCeiling,
    RemoveEnemy,
    RemoveEnemy,
    Crawler,
    PipeBug,
    RemoveEnemy,
    Unported("RidleyAIRoutine"),
    Unported("RidleyFireballAIRoutine"),
    RemoveEnemy,
    Unported("MultiviolaAIRoutine"),
    RemoveEnemy,
    Unported("PolypAIRoutine"),
    RemoveEnemy,
];

/// 区序号（`InArea & $0F`：0=Brinstar 1=Norfair 2=Kraid 3=Tourian 4=Ridley）
/// → 该区 bank 的跳表。
///
/// **未知区 fail-loud**（2026-11-16 按复核意见改）：原版没有「未知区回落 Brinstar」
/// 这一语义，静默回落会把真实缺口洗成「行为正确」；这里记 `unported!` 后仍返回
/// Brinstar 表只为不在战斗帧里 panic（调用方已在 `frame::cur_area` 侧 fail-loud）。
pub(crate) fn table(area: u8) -> &'static [Routine; 16] {
    match area & 0x0F {
        0 => &BRINSTAR,
        1 => &NORFAIR,
        2 => &KRAID,
        3 => &TOURIAN,
        4 => &RIDLEY,
        _ => {
            crate::unported!("bank07:CA30-choose_enemy_ai");
            &BRINSTAR
        }
    }
}

/// `ChooseEnemyAIRoutine` = `lda EnsExtra.0.type,x / jsr JumpEngine`（16 项，越界读
/// 未定义 ⇒ 本件按 `Null` 处理，由调用方 fail-loud）。
pub(crate) fn routine_for(area: u8, type_: u8) -> Routine {
    let t = table(area);
    match t.get(type_ as usize) {
        Some(r) => *r,
        None => Null,
    }
}

#[cfg(test)]
mod tests_ai_table {
    use super::*;

    /// 五张表逐项对 asm（转写本身的锚——防手抄漂移）。
    #[test]
    fn tables_match_asm_listings() {
        // 区序：0=Brinstar 1=Norfair 2=Kraid 3=Tourian 4=Ridley
        assert_eq!(routine_for(0, 0), SidehopperFloor);
        assert_eq!(routine_for(0, 2), Waver);
        assert_eq!(routine_for(0, 4), Skree);
        assert_eq!(routine_for(0, 0x0B), Null); // Brinstar 0B–0F 是空指针
        // Ripper I / II 必须是**两个表项**（同名 asm label、bank1/4 与 bank2 语义不同）
        assert_eq!(routine_for(0, 3), RipperI);
        assert_eq!(routine_for(2, 3), RipperI);
        assert_eq!(routine_for(1, 2), RipperII);

        // Norfair：02=Ripper II、04=RemoveEnemy（旧实现会误跑 Skree）、0B=Squeept
        assert_eq!(routine_for(1, 2), RipperII);
        assert_eq!(routine_for(1, 4), RemoveEnemy);
        assert_eq!(routine_for(1, 0x0B), Squeept);
        assert_eq!(routine_for(1, 0x0C), Multiviola);
        assert_eq!(routine_for(1, 0x00), Swooper00);
        assert_eq!(routine_for(1, 0x01), Swooper01);
        // Dragon/Polyp：EnProjectile 子系统已端口 ⇒ Norfair 这两格换成实变体。
        // **同一 type 号在 Ridley 表里语义不同**：bank5 的 0x0E 是另 include 的一份 Polyp
        // （门算式不同，见 `enemy::polyp` 模块注释），故 Ridley 侧仍是 `Unported`。
        assert_eq!(routine_for(1, 0x0D), Dragon);
        assert_eq!(routine_for(1, 0x0E), Polyp);
        assert_eq!(
            routine_for(4, 0x0E),
            Unported("PolypAIRoutine"),
            "Ridley 的 0x0E 是 `prg5_ridley.asm:178` 那份 Polyp（asm 注 unused），本卡不接线"
        );
        assert_eq!(
            routine_for(4, 0x0D),
            RemoveEnemy,
            "Ridley 的 0x0D 是 RemoveEnemy（`prg5_ridley.asm:177`）——与 Norfair 的 Dragon 不同语义"
        );

        // Tourian：00=Metroid、04=Rinka
        assert_eq!(routine_for(3, 0), Unported("MetroidAIRoutine"));
        assert_eq!(routine_for(3, 4), Rinka, "Tourian 0x04 = RinkaAIRoutine（本卡端口）");
        assert_eq!(routine_for(3, 5), RemoveEnemy);

        // Kraid：02/06/0B–0F 是 RTS_95CB（什么都不做），08=Kraid
        assert_eq!(routine_for(2, 2), Rts95CB);
        assert_eq!(routine_for(2, 6), Rts95CB);
        assert_eq!(routine_for(2, 8), Kraid);
        assert_eq!(routine_for(2, 9), KraidLint);
        assert_eq!(routine_for(2, 10), KraidNail);

        // Ridley：02/03=Sidehopper 家族（Dessgeega）、09=Ridley
        assert_eq!(routine_for(4, 2), SidehopperFloor);
        assert_eq!(routine_for(4, 3), SidehopperCeiling);
        assert_eq!(routine_for(4, 9), Unported("RidleyAIRoutine"));
    }

    /// 越界 type（原版 `JumpEngine` 读表外）⇒ `Null`，由调用方 fail-loud 而非静默。
    #[test]
    fn out_of_range_type_is_null() {
        assert_eq!(routine_for(0, 0x10), Null);
        assert_eq!(routine_for(4, 0x20), Null);
    }

    /// **派发按区**的真实链条证据：Norfair 的 type 4 是 `RemoveEnemy`，
    /// 同号在 Brinstar 是 Skree——旧全局表会把 Norfair 这只跑成 Skree。
    #[test]
    fn dispatch_uses_area_specific_table() {
        use crate::enemy::prep::{EN_F_DATA05, ENS_F_STATUS, ENS_F_TYPE};
        use crate::enemy::tables::{en_addr, ens_addr};
        use crate::frame::Manager;

        let run = |area: u8| {
            let mut m = Manager::default();
            game_data::pack::init_from_env().ok();
            let x = 0x20u8;
            m.write_ram(crate::ram_map::IN_AREA, area);
            m.write_ram(en_addr(x, 0), 0x40);
            m.write_ram(en_addr(x, 1), 0x50);
            m.write_ram(en_addr(x, EN_F_DATA05), 0x02); // bit1 闩 ⇒ 免 fresh 可见检
            m.write_wram(ens_addr(x, ENS_F_STATUS), 0x02); // Active
            m.write_wram(ens_addr(x, ENS_F_TYPE), 4); // type 4
            crate::enemy::slot_cycle::update_all_enemies(&mut m);
            m.read_wram(ens_addr(x, ENS_F_STATUS))
        };
        assert_eq!(run(1), 0, "Norfair type4 应 RemoveEnemy（清槽）");
        assert_eq!(run(0), 2, "Brinstar type4 是 Skree，应保持 Active");
    }

    /// 未知区序号回落到 Brinstar 表（调用方另记 DIVERGENCES）。
    #[test]
    fn unknown_area_is_fail_loud() {
        // 原版没有「未知区回落 Brinstar」语义（2026-11-16 按复核意见改）：
        // 未知区必须 fail-loud（`unported!`），不能静默给出一张表。
        let r = std::panic::catch_unwind(|| routine_for(0x0F, 2));
        assert!(r.is_err(), "未知区必须 panic（fail-loud），实测未 panic");
    }
}
