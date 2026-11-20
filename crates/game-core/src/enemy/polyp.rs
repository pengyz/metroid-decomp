//! Polyp（迷你火山 / 岩石发射器）—— `refs/m1disasm/SRC/enemies/polyp.asm`（64 行逐行对读）。
//!
//! **本件被两个 bank include**（A 级，全仓 grep `polyp.asm` 只得这两处）：
//! - `refs/m1disasm/SRC/prg2_norfair.asm:687 .include "enemies/polyp.asm"` ⇒ Norfair，例程
//!   落在 `02:9B64`（ROM bank2 逐字节核对见下）。
//! - `refs/m1disasm/SRC/prg5_ridley.asm:658 .include "enemies/polyp.asm"` ⇒ Ridley，落在
//!   `05:9B16`；asm 注释亦言 "(there arent any in ridley though)"，AI 表里那格写
//!   `; 0E - polyp (unused)`（`prg5_ridley.asm:178`）。
//!
//! ⇒ 同一例程的 `.if BANK == 2 / .elif BANK == 5` 是**按区不同的两种行为**，必须按
//! `frame::area_bank(frame::cur_area(mem))` 分派，**不能**只实现调用方所在的那一支
//! （"别把别区同 type 号当同语义"）。本卡只接线 Norfair（`ai_table::RIDLEY` 的 0x0E 仍是
//! `Unported`）；bank5 那支由本模块门算式承载并有单帧判别锚测，接线留给 Ridley 卡。
//!
//! asm 原文（`02:9B64`，bank2/Norfair 形）：
//! ```text
//! PolypAIRoutine_{AREA}: ; 02:9B64
//!     ; set radius to 0
//!     lda #$00 / sta EnsExtra.0.radiusY,x / sta EnsExtra.0.radiusX,x
//!     ; set Ens.0.data05 to #$10
//!     ; the enemy is invisible and will remain resting
//!     lda #$10 / sta Ens.0.data05,x
//!
//!     .if BANK == 2
//!         ; put bit 4 of enemy slot offset in bit 6
//!         txa / asl / asl / sta $00
//!     .endif
//!     ; get enemy slot id
//!     txa / lsr / lsr / lsr / lsr
//!     ; add framecount
//!     adc FrameCount
//!     .if BANK == 2
//!         ; polyps in norfair will try shooting 8 times every 8 frames, then stop trying for 64 frames
//!         ; when polyps of even enemy slot id stops trying, polyps of odd enemy slot id will start trying, and vice versa
//!         adc $00
//!         and #$47
//!     .elif BANK == 5
//!         ; polyps in ridley will always try to shoot every 8 frames
//!         and #$07
//!     .endif
//!     ; exit if polyp isn't trying to shoot
//!     bne RTS_Polyp_{AREA}
//!
//!     ; polyp is trying to shoot
//!     ; rotate horizontal facing flag into carry
//!     lsr Ens.0.data05,x
//!     ; set expected status to #$02|$01
//!     lda #enemyStatus_Resting | enemyStatus_Active / sta SpawnEnProjectile_ExpectedStatus
//!     ; set horizontal facing flag to a random bit
//!     lda RandomNumber1 / lsr / rol Ens.0.data05,x
//!     ; exit if misfired (25% random chance)
//!     and #$03 / beq RTS_Polyp_{AREA}
//!     ; store random number (1, 2 or 3) into SpawnEnProjectile_EnData0A
//!     ; this chooses between EnProjectileMovement1, EnProjectileMovement2 or EnProjectileMovement3
//!     sta SpawnEnProjectile_EnData0A
//!     ; set EnProjectile animation
//!     lda #$02 / sta SpawnEnProjectile_AnimTableIndex
//!     ; spawn projectile
//!     jmp CommonJump_SpawnEnProjectile
//!
//! RTS_Polyp_{AREA}:
//!     rts
//! ```
//!
//! 语义要点（易错处 / 反直觉处）：
//! - **本件没有 `UpdateEnemyCommon` 尾件**：两个出口都是 `rts`（`02:9B9D`），所以 Polyp 的
//!   动画/绘制/移动**全部**不推进——它自称是"隐形且恒休止"的火山（`data05=$10` 同时是
//!   `SpawnEnProjectile` 的第一道门 `07:F872`）。这是全敌 AI 里少见的"AI 即全部"形态，
//!   不许顺手补一个 `update_enemy_common_*`（那会凭空产生原版没有的绘制）。
//! - **`adc $00`（`02:9B7D`）会吃上一条 `adc FrameCount` 的进位**（6502 `adc` 带 carry-in）。
//!   slot id + FrameCount 溢出 $FF 时（如 slot 2 + `$FE`）A=$00 且 C=1 ⇒ 再 `+$00+1` ⇒
//!   掩码位与"忽略进位"的算法不同 ⇒ 会**少射一发**。`norfair_gate_consumes_adc_carry` 锚测
//!   单帧判别这件事；别把门算式"化简"成 `(slot + fc + (x<<2)) & $47`。
//! - **`sta $00` 的零页槽是纯中间量**：`(x<<2)&$FF` 只在紧跟的 `adc $00` 里被读一次。本件把
//!   它算成局部量（同 `framework::update_enemy_common_decide_with_beats` 的先例），不新增
//!   零页裸写——行为等价，且 `$00` 在那之间无人写。
//! - **`SpawnEnProjectile_AnimIndex` 本件完全不设**：asm 直接 `jmp`，那对立即数仍是上一个
//!   敌人留下的垃圾。但 `ExpectedStatus=#$03` 的 bit6 是 0 ⇒ 弹体**不读**这一对
//!   （`07:F80C` 的 `bit ExpectedStatus / bvc`，且 `prg7_engine.asm` 旁注"polyp sets this
//!   to #$02|$01"）。故传 `[0,0]` 是等价形，不是"假装"。
//! - **`and #$03 / beq` 之后存进 `SpawnEnProjectile_EnData0A` 的是 A**，即
//!   `(RandomNumber1 >> 1) & $03` ⇒ 只有 1/2/3 三种（0 已在此前 `beq` 掉）⇒
//!   `EnProjectileMovement1/2/3`，没有 Movement0。
//! - **`lsr data05,x` + `rol data05,x` 是一对**（`02:9B83`/`02:9B8D`）：先右移除 $10 剩 $08，
//!   再用随机 bit0 左滚回来 ⇒ 净效果 `data05 = $10 | (RandomNumber1 & $01)`，即**朝向位每
//!   次试图射击时被随机重掷**（这也是它会朝两边乱吐的原因）。
//!
//! **ROM 逐字节交叉核对（A 级，2026-11-20）**：直接读 `rom/Metroid (USA).nes` 的 bank2
//! 镜像（文件偏移 `0x10 + 2*0x4000 + (addr - $8000)`）：
//! - `02:9B64` `A9 00 9D F5 6A 9D F6 6A`（radiusY/radiusX←0；`$6AF5/$6AF6` =
//!   `EnsExtra.0.radiusY/radiusX`）、`A9 10 9D 05 04`（data05←$10）、
//!   `8A 0A 0A 85 00`（`txa/asl/asl/sta $00`）、`8A 4A 4A 4A 4A`（`txa` + 4×`lsr`）、
//!   `65 2D`（`adc FrameCount`）、`65 00 29 47 D0 1A`（`adc $00 / and #$47 / bne`）、
//!   `5E 05 04`（`lsr $0405,x`）、`A9 03 85 87`（ExpectedStatus=#$03）、
//!   `A5 2E 4A 3E 05 04`（`lda RandomNumber1 / lsr / rol $0405,x`）、`29 03 F0 09`、
//!   `85 88`（EnData0A）、`A9 02 85 85`（AnimTableIndex=#$02）、
//!   `4C 21 80`（`jmp CommonJump_SpawnEnProjectile`）、`9B9D 60`（`rts`）。
//! - bank5 形 `05:9B16`：同样的头，`8A 4A 4A 4A 4A 65 2D 29 07`（**无 `$00` 项、掩码 $07**）
//!   ⇒ 两支的字节级差异就是说明文档里那句 "polyps in ridley will always try to shoot every
//!   8 frames"。
//! - 上升动画表 `02:978B` 第 3 对实字节 `4D 4D` ⇒ `AnimTableIndex=#$02` 的弹体画成
//!   `EnAnim_PolypRock_{AREA}`（`02:9C27`）＝ `EnAnimTable_{AREA}`（`02:9BDA`）**+$4D**；
//!   `02:979B`/`02:97A3` 的位置偏移第 3 对都是 `0` ⇒ 弹体初始落点与 Polyp 本体**完全重合**。

use crate::enemy::en_projectile;
use crate::enemy::state::slot_of;
use crate::frame::Manager;

/// `Ens.0.data05 ← #$10`：隐形标记 + 恒休止；bit4 同时是 `SpawnEnProjectile` 第一道门。
const DATA05_INVISIBLE: u8 = 0x10;

/// 未命中概率（asm `and #$03 / beq`）⇒ 25% 空放。
const MISFIRE_MASK: u8 = 0x03;

/// Norfair 相位掩码（asm `.if BANK == 2` ⇒ `and #$47`）：
/// bit0-2＝每 8 帧一个机会窗，bit6＝64 帧一轮的"停火"位。
const GATE_MASK_NORFAIR: u8 = 0x47;

/// Ridley 相位掩码（asm `.elif BANK == 5` ⇒ `and #$07`）＝只保留每 8 帧窗口。
const GATE_MASK_RIDLEY: u8 = 0x07;

/// `enemyStatus_Resting | enemyStatus_Active = #$03`（asm `lda #enemyStatus_Resting | enemyStatus_Active`）。
const EXPECTED_STATUS: u8 = 0x03;

/// 弹体动画组（asm `lda #$02 / sta SpawnEnProjectile_AnimTableIndex`）⇒
/// `EnProjectileRisingAnimIndexTable`（`02:978B`）第 3 对 = `EnAnim_PolypRock_{AREA}`。
const EN_PROJECTILE_ANIM_TABLE_INDEX: u8 = 0x02;

/// `PolypAIRoutine_{AREA}`（`02:9B64`）。
///
/// **没有尾件**：两个出口都 `rts`——不动画、不绘制、不移动（见模块注释第一条）。
pub(crate) fn polyp_ai(mem: &mut Manager, x: u8) {
    let slot = slot_of(x);

    // `lda #$00 / sta EnsExtra.0.radiusY,x / sta EnsExtra.0.radiusX,x`——**每帧**都写，
    // 与门是否放行无关（这句在相位门之前）。
    mem.enemy(slot).set_radius_y(0);
    mem.enemy(slot).set_radius_x(0);
    // `lda #$10 / sta Ens.0.data05,x`——同理每帧都写：隐形 + 恒休止。
    mem.enemy(slot).set_data05(DATA05_INVISIBLE);

    // `… adc FrameCount / adc $00 / and #$47 / bne RTS`（bank2）或 `and #$07`（bank5）。
    if !phase_gate_open(mem, x) {
        return; // `bne RTS_Polyp_{AREA}`
    }

    // `lsr Ens.0.data05,x`——RMW：data05 >>= 1，C = 旧 bit0（原朝向）。
    let old = mem.enemy(slot).data05();
    mem.enemy(slot).set_data05(old >> 1);
    // `lda RandomNumber1 / lsr` ⇒ A = RNG>>1，C = RNG bit0。
    let rng = mem.random_number1();
    // `rol Ens.0.data05,x` ⇒ data05 = (data05 << 1) | C（6502 `rol` 把旧 bit7 丢进 carry，
    // 本处无人读那个 carry；Rust 的 `u8 << 1` 同样丢最高位，等价）。
    let facing = rng & 0x01;
    let rolled = (mem.enemy(slot).data05() << 1) | facing;
    mem.enemy(slot).set_data05(rolled);

    // `and #$03 / beq RTS_Polyp_{AREA}`——A 此时是 `RNG >> 1`，25% 空放。
    let movement = (rng >> 1) & MISFIRE_MASK;
    if movement == 0 {
        return;
    }

    // `sta SpawnEnProjectile_EnData0A`（1/2/3 ⇒ Movement1/2/3）+ `lda #$02 /
    // sta SpawnEnProjectile_AnimTableIndex` + `jmp CommonJump_SpawnEnProjectile`。
    // `AnimIndex` 那对 asm 不设（bit6 为 0 ⇒ 弹体不读，见模块注释）。尾调用 ⇒ 之后无动作。
    let _spawned = en_projectile::spawn_en_projectile(
        mem,
        x,
        EXPECTED_STATUS,
        EN_PROJECTILE_ANIM_TABLE_INDEX,
        movement,
        [0, 0],
    );
}

/// 相位门（`.if BANK == 2 … .elif BANK == 5 …`）：返回 `true` 表示本帧"试图射击"。
///
/// bank2 形（`02:9B71` 起）：`txa/asl/asl/sta $00` ⇒ `$00=(x<<2)&$FF`；`txa`+4×`lsr` ⇒
/// A=slot id（低 4 位必 0 ⇒ 第 4 次 `lsr` 的进位恒 0）；`adc FrameCount`；`adc $00`
/// （**带进位**）；`and #$47`。
/// bank5 形（`05:9B23` 起）：同序但**没有** `$00` 项，`and #$07`。
fn phase_gate_open(mem: &Manager, x: u8) -> bool {
    let bank = crate::frame::area_bank(crate::frame::cur_area(mem));
    // asm 只被 bank2/bank5 include（模块注释的两行）。走到这里说明有人把本件挂到了
    // 没有这份代码的区——fail-loud，不做"猜一个掩码"的静默回落。
    assert!(
        bank == 2 || bank == 5,
        "PolypAIRoutine 只存在于 bank2/bank5（当前 bank {bank}）"
    );

    // `txa` + 4×`lsr`：A = x>>4 = slot id，且移位带出的最后一位是 x 的 bit3 = 0（x=slot*16）。
    let slot_id = (x >> 4) as u16;
    // `adc FrameCount`：带进位入 0（上一步移出的 bit3 恒 0）⇒ 与普通加等价，但进位要传给下一条。
    let sum = slot_id + mem.frame_count() as u16;
    let carry = u16::from(sum > 0xFF);
    let a = sum & 0xFF;

    let masked = if bank == 2 {
        // `adc $00`——**吃上一条的进位**（这是本件最容易被"化简"掉的一位）。
        let zp00 = ((x as u16) << 2) & 0xFF;
        ((a + zp00 + carry) & 0xFF) as u8 & GATE_MASK_NORFAIR
    } else {
        a as u8 & GATE_MASK_RIDLEY
    };
    masked == 0 // `bne RTS` ⇒ 非 0 就退出
}

#[cfg(test)]
mod tests_polyp {
    use super::*;
    use crate::enemy::en_projectile::{SLOT_COUNT, base_slot};
    use crate::enemy::prep::ENS_F_TYPE;
    use crate::enemy::state::EnemyStatus;
    use crate::enemy::tables::ens_addr;
    use crate::ram_map;

    const X: u8 = 0x20; // slot 2 ⇒ `$00 = (0x20<<2)&$FF = $80`
    const SLOT: u8 = X / 16;

    /// 夹具：Norfair + Polyp(type 0x0E) + 指定 FrameCount/随机数。
    ///
    /// 不预置 `data05`：本件进门第一件事就是把它写成 `$10`（这也是被锚测的一条）。
    fn stage(m: &mut Manager, fc: u8, rng: u8) {
        game_data::pack::init_from_env().ok();
        m.write_ram(ram_map::IN_AREA, 1); // Norfair(bank2)
        m.write_wram(ens_addr(X, ENS_F_TYPE), 0x0E); // Polyp
        m.write_ram(ram_map::FRAME_COUNT, fc);
        m.write_ram(ram_map::RANDOM1, rng);
        m.enemy(SLOT).set_status(EnemyStatus::Resting); // 实机 Polyp 恒休止
        m.enemy(SLOT).set_y(0x50);
        m.enemy(SLOT).set_pos_x(0x70);
        m.enemy(SLOT).set_radius_y(9); // 进门必须被清 0
        m.enemy(SLOT).set_radius_x(9);
        m.enemy(SLOT).set_data05(0x00); // 进门必须被写 $10
        m.enemy(SLOT).set_reset_anim_index(0x49); // 谁都不该动它（无尾件）
        m.enemy(SLOT).set_anim_index(0x4E);
        m.enemy(SLOT).set_anim_delay(0x03);
    }

    /// 弹体池里"有弹"的槽号（`FindSlot` 从槽 6 起），无则 `None`。
    fn spawned_slot(m: &mut Manager) -> Option<u8> {
        (0..SLOT_COUNT).map(|i| base_slot(i)).find(|&s| m.enemy(s).status_raw() != 0)
    }

    /// **每帧无条件写的两句**：radius→0、`data05←$10`。相位门关着也照写
    /// （`02:9B64-9B70` 在 `bne RTS` 之前）。
    #[test]
    fn radius_and_data05_are_written_before_the_phase_gate() {
        let mut m = Manager::default();
        stage(&mut m, 0x00, 0x02); // slot2 在 fc=0 时掩码 $02 ⇒ 门关
        polyp_ai(&mut m, X);
        assert_eq!(m.enemy(SLOT).radius_y(), 0, "radiusY 每帧清 0");
        assert_eq!(m.enemy(SLOT).radius_x(), 0, "radiusX 每帧清 0");
        assert_eq!(m.enemy(SLOT).data05(), DATA05_INVISIBLE, "data05 每帧写 $10");
        assert_eq!(spawned_slot(&mut m), None, "门关 ⇒ 不吐弹（bne RTS_Polyp）");
    }

    /// **相位门（放行径）+ 生弹四参数**：Norfair slot2 在 `FrameCount=6` 放行；
    /// 弹体 `status←Active`、`data0A=(Movement*2+朝向)&7`、动画=`EnAnim_PolypRock`(`+$4D`)。
    #[test]
    fn norfair_phase_gate_opens_at_frame_6_and_spawns_rock() {
        let mut m = Manager::default();
        stage(&mut m, 6, 0x02); // RNG=2 ⇒ 朝向 0、Movement=(2>>1)&3=1
        let (y0, x0) = (m.enemy(SLOT).y(), m.enemy(SLOT).pos_x());
        polyp_ai(&mut m, X);
        let p = base_slot(0);
        assert_eq!(m.enemy(p).status_raw(), 2, "AnimTableIndex=2 ≥ 2 ⇒ 弹体被激活");
        assert_eq!(
            m.enemy(p).data0a(),
            (1 * 2) & 0x07,
            "EnData0A=1（Movement1）+ 朝向 0 ⇒ data0A=(1*2+0)&7"
        );
        assert_eq!(
            m.enemy(p).reset_anim_index(),
            0x4D,
            "AnimTableIndex=2 ⇒ EnAnim_PolypRock（02:978B 第 3 对；$02:9C27-$02:9BDA）"
        );
        assert_eq!(
            m.enemy(p).reset_anim_index(),
            game_data::assets::en_projectile_rising_anim(1)[4],
            "与资产投影的 rising 表第 3 对右项一致（防窗口错位）"
        );
        assert_eq!(m.enemy(p).y(), y0, "02:97A3 第 3 对 = 0 ⇒ 弹体 y 与本体重合");
        assert_eq!(m.enemy(p).pos_x(), x0, "02:979B 第 3 对 = 0,0 ⇒ 弹体 x 与本体重合");
    }

    /// **相位门（停火径）**：同一 slot 到 `FrameCount=0x46`（=6+64）时 bit6 置起 ⇒ 不射。
    /// 这就是 asm 注释"try shooting 8 times every 8 frames, then stop trying for 64 frames"
    /// 的 $47 掩码形。
    #[test]
    fn norfair_gate_rests_for_64_frames_after_the_burst() {
        let mut m = Manager::default();
        stage(&mut m, 0x46, 0x02); // 2 + 0x46 = 0x48；0x48+0x80 = 0xC8 ⇒ &$47 = $40
        polyp_ai(&mut m, X);
        assert_eq!(spawned_slot(&mut m), None, "bit6 置起 ⇒ 本轮停火");
    }

    /// **按区行为分叉（单帧判别）**：`FrameCount=0x3E` 时 Norfair(bank2) 停火而 Ridley(bank5)
    /// 放行——同一帧、同一 slot，只有掩码与 `$00` 项不同 ⇒ 证明两支都被实现。
    #[test]
    fn norfair_and_ridley_masks_diverge_on_the_same_frame() {
        let mut m = Manager::default();
        stage(&mut m, 0x3E, 0x02);
        polyp_ai(&mut m, X);
        assert_eq!(spawned_slot(&mut m), None, "bank2：2+0x3E=0x40,+0x80=$C0 ⇒ &$47=$40 ⇒ 停火");

        let mut m = Manager::default();
        stage(&mut m, 0x3E, 0x02);
        m.write_ram(ram_map::IN_AREA, 4); // Ridley ⇒ area_bank=5
        polyp_ai(&mut m, X);
        assert_eq!(
            spawned_slot(&mut m),
            Some(base_slot(0)),
            "bank5：(2+0x3E)&$07=0 ⇒ 放行（asm 说 ridley 每 8 帧都试）"
        );
    }

    /// **`adc $00` 吃进位**：slot2 + `FrameCount=$FE` ⇒ 第一条 `adc FrameCount` 溢出产生 C=1，
    /// 于是 A=0+$80+1=$81 ⇒ 掩码 $01 ⇒ 停火；若"化简"成无进位的加法会得 $80 ⇒ 掩码 0
    /// ⇒ 错射一发。本锚测单帧判别这个位。
    #[test]
    fn norfair_gate_consumes_adc_carry() {
        let mut m = Manager::default();
        stage(&mut m, 0xFE, 0x02);
        polyp_ai(&mut m, X);
        assert_eq!(
            spawned_slot(&mut m),
            None,
            "2+$FE=$100 ⇒ C=1 ⇒ A=$00+$80+1=$81 ⇒ &$47=$01 ⇒ 停火"
        );
    }

    /// **未命中径 + 朝向重掷**：`and #$03 / beq` 在 `rol data05,x` **之后** ⇒ 即便空放，
    /// 朝向位也已被随机改写（这里的 RNG=1 ⇒ bit0=1 ⇒ data05 从 $10 变 $11）。
    #[test]
    fn misfire_still_rerolls_facing_bit() {
        let mut m = Manager::default();
        stage(&mut m, 6, 0x01); // RNG=1 ⇒ A=(1>>1)=0 ⇒ &3=0 ⇒ 空放；bit0=1
        polyp_ai(&mut m, X);
        assert_eq!(spawned_slot(&mut m), None, "RNG&3 位为 0 ⇒ 25% 空放");
        assert_eq!(m.enemy(SLOT).data05(), 0x11, "朝向位已按 RNG 的 bit0 重掷为 1");
    }

    /// **`data05` bit0 位义**：随机朝向同时进弹体的 `data0A`（`(Movement*2+朝向)&7`），
    /// 因此同一 Movement 下左/右朝向给出不同 Movement 串。
    #[test]
    fn random_facing_bit_feeds_projectile_movement() {
        for (rng, facing) in [(0x02u8, 0u8), (0x03u8, 1u8)] {
            let mut m = Manager::default();
            stage(&mut m, 6, rng);
            polyp_ai(&mut m, X);
            let p = base_slot(0);
            assert_eq!(
                m.enemy(SLOT).data05() & 0x01,
                facing,
                "data05 bit0 ← RNG bit0（rng={rng:#04x}）"
            );
            let movement = (rng >> 1) & 0x03;
            assert_eq!(
                m.enemy(p).data0a(),
                (movement * 2 + facing) & 0x07,
                "弹体 data0A=(Movement*2+朝向)&7（rng={rng:#04x}）"
            );
        }
    }

    /// **"AI 即全部"**：本件不调 `UpdateEnemyCommon`（asm 两个出口都是 `rts`）⇒ 动画四字段
    /// 谁都不动。若哪天有人补上尾件，这条会立刻红。
    #[test]
    fn polyp_never_touches_anim_state() {
        let mut m = Manager::default();
        stage(&mut m, 6, 0x02); // 门放行 + 真生弹
        polyp_ai(&mut m, X);
        assert_eq!(spawned_slot(&mut m), Some(base_slot(0)), "前提：本帧确实生弹了");
        assert_eq!(m.enemy(SLOT).reset_anim_index(), 0x49, "resetAnimIndex 未被写");
        assert_eq!(m.enemy(SLOT).anim_index(), 0x4E, "animIndex 未被写");
        assert_eq!(m.enemy(SLOT).anim_delay(), 0x03, "animDelay 未被写");
    }

    /// **接线锚**：Norfair AI 表 type `0x0E` 经 `choose_enemy_ai` 真的到本件
    /// （`lda #$10 / sta data05,x` 这一句是全域唯一的入口签名，且它在相位门之前）。
    #[test]
    fn dispatch_wiring_reaches_polyp_for_type_0x0e() {
        let mut m = Manager::default();
        game_data::pack::init_from_env().ok();
        m.write_ram(ram_map::IN_AREA, 1);
        m.write_wram(ens_addr(X, ENS_F_TYPE), 0x0E);
        m.enemy(SLOT).set_status(EnemyStatus::Resting);
        m.enemy(SLOT).set_y(0x50);
        crate::enemy::slot_cycle::update_all_enemies(&mut m);
        assert_eq!(
            m.enemy(SLOT).data05() & 0x10,
            0x10,
            "经 choose_enemy_ai 到 Polyp 径（data05 被写成含 bit4 的 $10）"
        );
        assert_eq!(m.enemy(SLOT).radius_y(), 0, "radiusY 被清 0（同一入口签名）");
    }
}
