//! 标题状态面 ↔ zp 字节域（地址只活在本层——two-layer §1）。
//!
//! 地址证据（A 级）：bank00:$806E `e6 1f` INC zp $1F（TitleRoutine）；bank00:$8283
//! `e6 1f a9 10 85 2c`（Timer3←$10）；bank00:$8068 `a9 00 85 cb 85 c9`（SpareMemCB/C9）。
//! 常量内容锚：constants_ram.asm:479 `TitleRoutine db ;$1F`、:496 `Timer3 db ;$2C
//! ;Decremented every 10 frames after set`、:744-745 `SpareMemC9 dw ;$C9`（**写面只低字节**
//! ——`85 c9` zp 单址）/`SpareMemCB db ;$CB`。
//! TitleRoutine 地址唯一源=crate::ram_map::TITLE_ROUTINE（引擎级 zp 图既有件，不另立）。
//! 命名台账 N-0066（第三方名采纳自证）；批1 助手簇（title::HelperOutcome 语义面）映射。

use crate::ram_map::TITLE_ROUTINE;

/// Timer3=$2C（「Decremented every 10 frames after set」——0F DelayIntroReplay 写 $10
/// =160 帧延迟的单位面）。
pub const TIMER3: u16 = 0x002C;

/// SpareMemC9 低字节=$C9（「Written to in title routine, but never accessed」）。
pub const SPARE_C9: u16 = 0x00C9;

/// SpareMemCB=$CB（同上注）。
pub const SPARE_CB: u16 = 0x00CB;

/// TitleRoutine 读（相位面）。
pub fn read_phase(ram: &[u8]) -> u8 {
    ram[TITLE_ROUTINE as usize & 0x7FF]
}

/// 写事件：TitleRoutine（INC zp 后值）。
pub fn write_event_phase(v: u8) -> (u16, u8) {
    (TITLE_ROUTINE, v)
}

/// 写事件：Timer3。
pub fn write_event_timer3(v: u8) -> (u16, u8) {
    (TIMER3, v)
}

/// 写事件：SpareMemC9 低字节。
pub fn write_event_spare_c9(v: u8) -> (u16, u8) {
    (SPARE_C9, v)
}

/// 写事件：SpareMemCB。
pub fn write_event_spare_cb(v: u8) -> (u16, u8) {
    (SPARE_CB, v)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 地址唯一源+写事件面（$1F 复用 ram_map::TITLE_ROUTINE 不另立）。
    #[test]
    fn title_face_addresses_and_events() {
        let mut ram = vec![0u8; 0x800];
        ram[TITLE_ROUTINE as usize] = 0x0A;
        assert_eq!(read_phase(&ram), 0x0A);
        assert_eq!(write_event_phase(0x0B), (0x001F, 0x0B));
        assert_eq!(write_event_timer3(0x10), (TIMER3, 0x10));
        assert_eq!(write_event_spare_c9(0), (SPARE_C9, 0));
        assert_eq!(write_event_spare_cb(0), (SPARE_CB, 0));
    }
}

// ---- 批2 编排簇地址面（A 级内容锚：constants_ram.asm 逐址）----

/// IntroMusicRestart=$D8（:755「After all title routines run twice, restarts intro music」）。
pub const INTRO_MUSIC_RESTART: u16 = 0x00D8;
/// IsSamus=$55（:548）。
pub const IS_SAMUS: u16 = 0x0055;
/// Joy1Change/Status/Retrig=$12/$14/$16（:465-469）。
pub const JOY1_CHANGE: u16 = 0x0012;
/// Joy1Status。
pub const JOY1_STATUS: u16 = 0x0014;
/// Joy1Retrig。
pub const JOY1_RETRIG: u16 = 0x0016;
/// PaletteDataIndex=$C1（:735）。
pub const PALETTE_INDEX: u16 = 0x00C1;
/// ScreenFlashPaletteIndex=$C2（:736）。
pub const FLASH_INDEX: u16 = 0x00C2;
/// IntroStarOffset=$C3（:737；ram_map INTRO_STAR_OFFSET 同址——唯一源=ram_map 复用面）。
pub const INTRO_STAR_OFFSET: u16 = 0x00C3;
/// FadeDataIndex=$C4（:738）。
pub const FADE_INDEX: u16 = 0x00C4;
/// SpriteLoadPending=$C8（:743「Set to #$00 after sprite RAM load complete」）。
pub const SPRITE_LOAD_PENDING: u16 = 0x00C8;
/// CrossExplodeLengthIndex=$C6（:741）。
pub const CROSS_EXPLODE_INDEX: u16 = 0x00C6;
/// IsUpdatingCrossExplode=$C7（:742）。
pub const IS_UPDATING_CROSS: u16 = 0x00C7;
/// CrossMsl0to3SlowDelay=$BC（:727）。
pub const CROSS_SLOW_DELAY: u16 = 0x00BC;
/// SecondCrosshairSprites=$BF（:732）。
pub const SECOND_CROSS: u16 = 0x00BF;
/// SpareMem 族（:723-754「Written to in title routine, but never accessed」）。
pub const SPARE_B7: u16 = 0x00B7;
/// SpareMemB8（ds 3 区首——写面只 $B8 单址）。
pub const SPARE_B8: u16 = 0x00B8;
/// SpareMemBB。
pub const SPARE_BB: u16 = 0x00BB;
/// SpareMemC5。
pub const SPARE_C5: u16 = 0x00C5;
/// SpareMemCC..D3/D7 族。
pub const SPARE_CC: u16 = 0x00CC;
/// SpareMemCD。
pub const SPARE_CD: u16 = 0x00CD;
/// SpareMemCE。
pub const SPARE_CE: u16 = 0x00CE;
/// SpareMemCF。
pub const SPARE_CF: u16 = 0x00CF;
/// SpareMemD0。
pub const SPARE_D0: u16 = 0x00D0;
/// SpareMemD1。
pub const SPARE_D1: u16 = 0x00D1;
/// SpareMemD2。
pub const SPARE_D2: u16 = 0x00D2;
/// SpareMemD3（ds 4 区首——写面只 $D3 单址）。
pub const SPARE_D3: u16 = 0x00D3;
/// SpareMemD7。
pub const SPARE_D7: u16 = 0x00D7;
/// NARPASSWORD=$69B2（WRAM；:1263「0=invinsible Samus not active」）。
pub const NARPASSWORD: u16 = 0x69B2;
/// SFXMultiInitFlags=$0684（RAM 声引擎标志；:1127「Initialization Flags for SFX and some
/// music」——**非 APU 寄存器**：写面 L2 可钉，APU 效果经声引擎延迟施加）。
pub const SFX_MULTI_INIT: u16 = 0x0684;
/// Objects 块=$0300-$031F（PrepIntroRestart 清面）。
pub const OBJECTS_CLEAR_BASE: u16 = 0x0300;
/// 星精灵源 WRAM=$6E00-$6E9F（IntroStarSprite；LoadStarSprites 源面）。
pub const STAR_SPRITE_SRC: u16 = 0x6E00;
/// 星精灵目标 RAM=$0260-$02FF（SpriteRAM.24 起）。
pub const STAR_SPRITE_DST: u16 = 0x0260;

/// 批2 写事件映射（语义标签 → 事件；标签集合=OrchestrationOutcome.writes 的键域）。
pub fn write_event_tag(tag: &str, v: u8) -> (u16, u8) {
    let addr = match tag {
        "PPUCTRL_ZP" => crate::ram_map::PPUCTRL_ZP,
        "TIMER3" => TIMER3,
        "SPARE_C9" => SPARE_C9,
        "SPARE_CB" => SPARE_CB,
        "INTRO_MUSIC_RESTART" => INTRO_MUSIC_RESTART,
        "IS_SAMUS" => IS_SAMUS,
        "JOY1_CHANGE" => JOY1_CHANGE,
        "JOY1_STATUS" => JOY1_STATUS,
        "JOY1_RETRIG" => JOY1_RETRIG,
        "PALETTE_INDEX" => PALETTE_INDEX,
        "FLASH_INDEX" => FLASH_INDEX,
        "INTRO_STAR_OFFSET" => INTRO_STAR_OFFSET,
        "FADE_INDEX" => FADE_INDEX,
        "SPRITE_LOAD_PENDING" => SPRITE_LOAD_PENDING,
        "SPARE_B7" => SPARE_B7,
        "SPARE_B8" => SPARE_B8,
        "SPARE_BB" => SPARE_BB,
        "SPARE_C5" => SPARE_C5,
        "SPARE_CC" => SPARE_CC,
        "SPARE_CD" => SPARE_CD,
        "SPARE_CE" => SPARE_CE,
        "SPARE_CF" => SPARE_CF,
        "SPARE_D0" => SPARE_D0,
        "SPARE_D1" => SPARE_D1,
        "SPARE_D2" => SPARE_D2,
        "SPARE_D3" => SPARE_D3,
        "SPARE_D7" => SPARE_D7,
        "NARPASSWORD" => NARPASSWORD,
        "SFX_MULTI_INIT" => SFX_MULTI_INIT,
        "PTR_LO" => 0x0000,
        "PTR_HI" => 0x0001,
        "PTR_LO2" => 0x0002,
        other => panic!("title_state 未知写标签 {other}——fail-loud"),
    };
    (addr, v)
}

/// Objects 清区展开（$0300-$031F，32 事件）。
pub fn objects_clear_events() -> Vec<(u16, u8)> {
    (0..0x20u16).map(|i| (OBJECTS_CLEAR_BASE + i, 0)).collect()
}

#[cfg(test)]
mod tests_batch2 {
    use super::*;

    /// 批2 标签映射面（PPUCTRL_ZP 复用 ram_map 唯一源）。
    #[test]
    fn batch2_tag_mapping() {
        assert_eq!(write_event_tag("PPUCTRL_ZP", 0xF9), (0x00FF, 0xF9));
        assert_eq!(write_event_tag("SFX_MULTI_INIT", 0x10), (0x0684, 0x10));
        assert_eq!(write_event_tag("NARPASSWORD", 0), (0x69B2, 0));
        assert_eq!(objects_clear_events().len(), 32);
        assert_eq!(objects_clear_events()[0], (0x0300, 0));
        assert_eq!(objects_clear_events()[31], (0x031F, 0));
    }
}

// ---- 批4a 特效簇映射面（IntroSpr 槽 $6EA0/$6EB0 步距 $10 + SpriteRAM.4 面 $0210/$0220）----

/// IntroSpr 槽 0 基址（LoadSparkleData `sta IntroSprs.0.y,x` 落 $6EA0 实证）。
pub const INTRO_SPR_BASE: u16 = 0x6EA0;
/// 槽步距（X=0/$10）。
pub const INTRO_SPR_STRIDE: u16 = 0x10;
/// SpriteRAM.4 面基址（WriteIntroSprite 目标；SpriteRAM=$0200，.4=+16）。
pub const SPRITE_RAM4_BASE: u16 = 0x0210;

/// 特效写标签映射（slot|0x80=SpriteRAM 面；slot 0x40=调色板面（off 0=C1 clamp/1=C1/2=1C））。
pub fn effect_event(tag: (u8, u8, u8)) -> Vec<(u16, u8)> {
    let (slot, off, v) = tag;
    if slot & 0x80 != 0 {
        return vec![(SPRITE_RAM4_BASE + (slot & 1) as u16 * INTRO_SPR_STRIDE + off as u16, v)];
    }
    if slot == 0x20 {
        return vec![(off as u16, v)]; // zp 面（off=址）——精确判（0x30 族槽标记不得落此）
    }
    if slot == 0x10 {
        return vec![(INTRO_SPR_BASE + off as u16, v)]; // INTRO_SPR_BASE 相对原始偏移
    }
    if slot == 0x08 {
        return vec![(0x0200 + off as u16, v)]; // SpriteRAM $0200 相对（UpdateCrossExplode 面）
    }
    if (0x30..0x38).contains(&slot) {
        return vec![(INTRO_SPR_BASE + (slot - 0x30) as u16 * INTRO_SPR_STRIDE + off as u16, v)];
    }
    if (0x38..0x40).contains(&slot) {
        return vec![(SPRITE_RAM4_BASE + (slot - 0x38) as u16 * INTRO_SPR_STRIDE + off as u16, v)];
    }
    if slot == 0x02 {
        return vec![(0x0300 + off as u16, v)]; // Objects $0300 块面（09 死码残迹）
    }
    if slot & 0x40 != 0 {
        return vec![match off {
            0 => (PALETTE_INDEX, v),
            1 => (PALETTE_INDEX, v),
            2 => (crate::ram_map::PALETTE_PENDING, v),
            3 => (TIMER3, v),
            _ => panic!("effect 未知调色板偏移 {off}——fail-loud"),
        }];
    }
    vec![(INTRO_SPR_BASE + slot as u16 * INTRO_SPR_STRIDE + off as u16, v)]
}

// ==== 会师契约应用件（② outcome 应用面）====

/// 应用 TitleStep 写事件到内存面（桥侧一步到位；或直读 step.events 自行投影）。
/// RAM/WRAM 归一：addr<$2000 → 镜像 &0x7FF；$6000-$7FFF → wram；其他域 fail-loud。
pub fn apply_title_step(ram: &mut [u8], wram: &mut [u8], events: &[(u16, u8)]) {
    for &(addr, v) in events {
        if addr < 0x2000 {
            ram[(addr as usize) & 0x7FF] = v;
        } else if (0x6000..0x8000).contains(&addr) {
            wram[addr as usize - 0x6000] = v;
        } else {
            panic!("apply_title_step 域外写 ${addr:04X}——fail-loud");
        }
    }
}
