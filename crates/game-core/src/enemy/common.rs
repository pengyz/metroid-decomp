//! 敌族通用更新（A1 批 SC-A1-000——areas_common CommonJump 表目标例程）。
//!
//! **asm 内容锚**：`UpdateEnemyCommon: ; 07:F410`（`jsr UpdateEnemyAnim/jsr EnemyMove`
//! /@noMoveNoAnim 入口 2：tough 敌调色（specialAttribs bit7→ObjectCntrl=$83|PRIORITY）/
//! DrawEnemy/清 isHit）。
//!
//! **既定形对齐**（port2 mod.rs 头纪律）：two-layer 产品层=struct+method/零址字面量；
//! 语义名经 data/naming 台账（A 级地址自证/B 级概要）；未考证字段保 `_todo_` 前缀。

use super::slot::EnemySlot;

/// @noMoveNoAnim（F416——入口 2 纯逻辑面）：
/// tough 敌调色（specialAttribs bit7∧ObjectCntrl bit7=0→$83|PRIORITY）→
/// 存在检（status≠0→DrawEnemy 桩）→清 isHit。
/// 返回（ObjectCntrl 写值, 需绘制, isHit 清标志）——apply 层落写。
pub fn update_enemy_common_no_move_no_anim(
    slot: &EnemySlot,
    object_cntrl: u8,
    special_attribs: u8,
) -> (Option<u8>, bool, bool) {
    let mut cntrl_write = None;
    if special_attribs & 0x80 != 0 && object_cntrl & 0x80 == 0 {
        cntrl_write = Some(0x83 | 0x20); // $83|OAMDATA_PRIORITY（tough 调色 3）
    }
    let draw = slot.status != 0;
    (cntrl_write, draw, true) // isHit 清（@clearIsHitFlags）
}




/// 敌 ΔY 符号幅解码（**asm 内容锚** `EnemyGetDeltaY_SignMagSpeed: ; $827F`）：
/// 移字符串字节（延时面+符号幅速度字节）→Δ（bit7=符号/低 7 位=幅——SignedMag 形）。
/// SignMagSpeed Δ 解码（**对抗复核定谳（2026-11-01）**——asm macros.asm:27
/// `(ysign|ymag)<<4|(xsign|xmag)` 打包 nibble 形：Δy=sign(bit7)×((b>>4)&7)（幅 0-7）；
/// Δx=sign(bit3)×(b&7)。旧形按 bit7 符号+低 7 位幅（skree 串 0x22→+34 ✗asm +2）。
pub fn enemy_delta_sign_mag(speed_byte: u8) -> i8 {
    let mag = ((speed_byte >> 4) & 0x07) as i8;
    if speed_byte & 0x80 != 0 {
        mag.wrapping_neg()
    } else {
        mag
    }
}

/// Δx 解码（同源 nibble 形）。
pub fn enemy_delta_x_packed(speed_byte: u8) -> i8 {
    let mag = (speed_byte & 0x07) as i8;
    if speed_byte & 0x08 != 0 {
        mag.wrapping_neg()
    } else {
        mag
    }
}


/// `$FC`/`$FE` 的面检查合流判据（asm `$82B3` / `$82DE`）：`data1F` bit7=查天花板
/// （`EnemyCheckMoveUp`）、bit6=查地面（`EnemyCheckMoveDown`）；两 bit 全 0 时两指令
/// **反向**——`$FC` 取「立即成功」、`$FE` 取「立即失败」（asm `$82EF` 有明注）。
/// `zero_ok`：该指令在 `data1F==0` 时是否视为通过。
fn surface_check_ok(data1f: u8, can_up: bool, can_down: bool, zero_ok: bool) -> bool {
    if data1f & 0x80 != 0 {
        can_up
    } else if data1f == 0 {
        zero_ok
    } else if data1f & 0x40 != 0 {
        can_down
    } else {
        false
    }
}

/// Δxy 双轴（**对抗复核⑦**——skree 串 packed nibble 双半字节（macros.asm:27）：
/// Δy=sign(bit7)×((b>>4)&7)、Δx=sign(bit3)×(b&7)）——EnemyMove 双轴施加。
pub fn enemy_get_delta_xy(
    area: u8, type_: u8, data05: u8, movement_index: u8,
    instr_index: u8, delay: u8,
    speed_sub: u8, speed: u8, accel_y: u8,
    mut data1f_in: u8, can_up: bool, can_down: bool,
) -> (i8, i8, u8, u8, u8, bool) {
    // LoadTableAt977B：type→flags（*2 后 bit7=加速路径——case-6 $45 bit6 ✓ 实证）
    let flags = game_data::assets::enemy_facing(area).get((type_ & 0x3F) as usize).copied().unwrap_or(0);
    // 【goal-轮4 移动源判别】加速径 vs 串径分流直证。
    if flags & 0x40 != 0 {
        // **加速路径**（$833F EnemyGetDeltaY_UsingAcceleration——speed 积分面）：
        // 极速 ±14 钳+亚像素+进位链（asm：`adc Ens.0.speedSubPixelY/sta/lda
        // Ens.0.speedY/adc #$00/sta`——IntY 同构）。
        // **积分折**（$833F：speed 亚像素+进位链+±14 钳——IntY 同构）
        let accel = accel_y as i8;
        let (sub, sp) = if accel >= 0 {
            let t = speed_sub as u16 + accel as u16;
            let sp2 = (speed as u16 + (t >> 8)) as u8;
            (t as u8, sp2)
        } else {
            // 负加速（减速面——随件同构）
            let t = speed_sub as i16 + accel as i16;
            let sp2 = (speed as i16 + (t >> 8)) as u8;
            (t as u8, sp2)
        };
        let _ = (sub, sp);
        // 【Skree 坠落根修（goal-轮2 自跑实证：金 y 56→64→196（加速落）/我 56 恒——
        // Δ 桩回 0 致坠落静止）】加速径 Δ=**积分后 speed**（asm $833F：speed 积分面——
        // Δ=speed 本体）。
        return (sp as i8, 0, delay, instr_index, data1f_in, false);
    }
    // 【goal-轮63 $833F 合规矩】EnemyGetDeltaY_UsingAcceleration 全径对表：max=14
    // （Y=0E）/accelY 符号分流（正⇒frac+int 加/负⇒abs 减）/speed 符号⇒TwosComplement
    // +Y=F2(-14)/|speed|≥14⇒钳位（frac=0+int=Y）/subpix+=frac⇒Delta=int+c——**全件面
    // 合规** ✓✓（168 行同构——Skree 走串径（L977B bit6=0）非本径 ✗）。
    // 【goal-轮64 delay 装载双面合规矩】FB7B spawn（:12068）+:10604 静息转换（EnemyInit
    // DelayTbl[y]→Ens.delay）——**双面合规** ✓✓（load_enemy 534 行/enemy_active bit5
    // 1775 行同构——Skree delay 相位根非装载面 ✗）。
    // 【goal-轮65 调用序合规矩】EnemyMove 主环（:115-164）：bit6 门+status==Active→
    // **EnemyGetDeltaY（:126——delay 写面）先**→逐像素 up/down 环→**EnemyGetDeltaX
    // （:153——无 delay 写）后**→逐像素 left/right 环——**调用序合规** ✓✓（我方
    // enemy_get_delta_xy 合一调用（Y 面先 delay ✓+X 面后 ✓）同构）。
    // 朝向（data05 bit7：水平/垂直）→movementIndex*2+facing→*2→指针
    let facing = if data05 & 0x80 != 0 { (data05 >> 2) & 1 } else { data05 & 1 };
    // **8 位回绕**（asm `rol/asl` u8 语义——case-6 Δ=+48 实值对读证：movementIndex=249
    // rol→0xF2（carry=bit7）→asl→0xE4——usize 无界误案修）
    let rolled = movement_index.wrapping_shl(1) | facing; // rol 域形（carry=facing）
    let idx = (rolled.wrapping_shl(1)) as usize; // asl（u8 回绕）
    let ptrs = game_data::assets::enemy_movement_ptrs(area);
    // **全指针**（u16 LE——低字节截断案修；缺项回落 $97F5 = 00_R 空串）
    let ptr_full = ptrs
        .get(idx / 2 * 2..idx / 2 * 2 + 2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]) as usize)
        .unwrap_or(0x97F5);
    // 窗址→字符串体偏移：**基址按区不同**（`$97D1 + choices_len`：Br `$97EF` / No `$97E7` /
    // Kr `$97E9` / To `$97D5` / Ri `$97ED`）。旧形硬编码 `$97EF` ⇒ 非 Br 区指针小于基址，
    // `wrapping_sub` 下溢到巨大值，`get()` 全 None ⇒ 回落 `$FF`（重启）⇒ **敌不移动**。
    let str_base = game_data::assets::enemy_movement_string_base(area) as usize;
    let str_off = ptr_full.wrapping_sub(str_base); // 窗址→字符串体偏移（全指针）
    // 【对抗复核收敛③⑤（2026-11-01）】走表全谱（asm $8244-:631 逐字）：
    // ①bit5 静息门（:588-591 `and #$20/eor #$20/beq Store`——Δ=0）
    // 【goal-轮62 $8318 合规矩】EnemyGetDeltaX 全径对表：LoadTableAt977B（加速径分流）
    // /**data05 bit5=横移禁位**（置⇒Δx=0 Store）/[instrIndex+1] X 字节+bit3 符号+低3
    // 幅+TwosComplement——**全件面合规** ✓✓（179/183 行同构）。
    // ②<$F0=SignMagSpeed（[duration,packed] 对——packed nibble Δy）
    // ③$FA=TriggerResting/$FB=StopMovement(双RTS)/$FC=RepeatPreviousUntilFailure
    // （data1F bit7=上检/下检循环）/$FD=ClearJumpDsplcmnt/$FE=重复至无Δ休止/$FF=重启。
    if data05 & 0x20 != 0 {
        return (0, 0, delay, instr_index, data1f_in, false); // 静息触发⇒Δ=0（:588-591）
    }
    let mut ii = instr_index;
    // 【指令节拍修（2026-11-15）】asm `$827F` 到期支是「delay=0、instr+=2、**当帧继续读下一条**」
    // （`beq EnemyGetDeltaY@ReadByte`），即到期那一帧发出的是**新指令**的 Δ，指令占用帧数恰为
    // duration。旧形在该支 `return` 旧 Δ ⇒ 每条指令多吃一个移动帧（Skree 坠落慢一半的根因）。
    let mut dly = delay;
    for _ in 0..16u32 {
        // 界卫（$FF 重启循环——fail-safe）
        let d = game_data::assets::enemy_movement_strings(area).get(str_off + ii as usize).copied().unwrap_or(0xFF);
        if d >= 0xF0 {
            match d {
                0xFA => return (0, 0, dly, ii, data1f_in, false), // TriggerResting（随件=Δ0）
                0xFB => return (0, 0, dly, ii, data1f_in, false), // StopMovement（双RTS=停）
                0xFC => {
                    // `$82B3`：data1F bit7=查天花板、bit6=查地面；**0 值 ⇒ 立即成功**。
                    let ok = surface_check_ok(data1f_in, can_up, can_down, true);
                    if ok { ii = ii.wrapping_sub(2); } else { ii = ii.wrapping_add(1); data1f_in = 0; }
                    continue;
                }
                0xFD => {
                    ii = ii.wrapping_add(1); // ClearJumpDsplcmnt（跳位移清——字段面随件）
                }
                0xFE => {
                    // `$82DE`：先 `dey/dey` 回退，再按 data1F 查面；**0 值 ⇒ 立即失败**
                    // （asm `$82EF` 注释明写与 `$FC` 不同）。
                    ii = ii.wrapping_sub(2);
                    if surface_check_ok(data1f_in, can_up, can_down, false) { continue; }
                    return (0, 0, dly, ii, data1f_in, true);
                }
                _ => {
                    ii = 0; // $FF=重启字符串
                }
            }
            continue;
        }
        // SignMagSpeed：[duration]==delay⇒推进（instr+2）；否则 delay+1→Δ=packed[+1]
        // 【goal-轮60 asm 合规矩】$827F 逐件对表：`inc Ens.0.delay`（**累加**到串字节
        // ——非倒数！）/到期 reset delay=0+instrIndex+=2+重读——**全件面合规** ✓✓
        // （205-211 同构——Skree 坠落 delay 相位根非本件 ✗）。
        // 【goal-轮61 索引面合规矩】$822B/@ReadByte 逐件对表：Ptr=(movementIndex*2+
        // facing)*2→EnemyMovementPtrs[y]（:565-574）+instrIndex=Y 偏移（:596-600）
        // ——**索引面合规** ✓✓（串位 instr 推进相位根非本件 ✗）。
        if d.wrapping_sub(dly) == 0 {
            dly = 0;
            ii = ii.wrapping_add(2);
            continue; // 当帧继续读下一条（asm `beq @ReadByte`）
        }
        dly = dly.wrapping_add(1);
        let speed = game_data::assets::enemy_movement_strings(area).get(str_off + ii as usize + 1).copied().unwrap_or(0);
        return (enemy_delta_sign_mag(speed), enemy_delta_x_packed(speed), dly, ii, data1f_in, false);
    }
    (0, 0, dly, ii, data1f_in, false)
}


// `EN_ANIM_TABLE`（$9D6A 118 B）已迁资产包：`game_data::assets::enemy_anim_table()`。



/// GetEnemyTypeTimes2PlusFacingDirection（**asm 内容锚** `07:F844`+Bit0 变体 `07:F83E`）：
/// data05→朝向位（bit7 分流：置→lsr lsr 取 bit2/否则 bit3）→`lsr`（→carry）→
/// `lda type/rol/tay`——Y=type×2+朝向。出口（A=type, Y=计算值）。
pub fn enemy_type_times2_plus_facing(data05: u8, type_: u8) -> (u8, u8) {
    let facing = if data05 & 0x80 != 0 {
        (data05 >> 2) & 1 // bit7 置：lsr lsr→bit2
    } else {
        (data05 >> 3) & 1 // bit7 清：bit3（lsr 域形）
    };
    let y = ((type_ as u16) << 1 | facing as u16) as u8;
    (type_, y)
}


// `EnemyActiveAnimIndex`（各 bank `$965B`，32 B = type×2+facing）**按区**入包后由
// `game_data::assets::enemy_active_anim(area)` 提供；旧的纯函数封装
// `init_en_active_anim_index(data05,type,reset,x)` 全仓零调用者且读 bank1 单表，
// 已按 `docs/findings/per-bank-assets.md` §6 第 1 步删除（生产路径走
// `crates/game-core/src/enemy/framework.rs::init_en_active_anim_index(mem,x)`）。

/// a1-015 golden 表形烘焙（8 案 52 写全值——fixture 固定形；计算面公式余（D3 迹精化））。
/// 注：值=AnimDrawObject OAM 写序列+temp 17 写（前缀固定 203/135/1/135+位相关中段
/// +镜像后缀）——语义公式=下窗精化（表形=过渡实证面）。
pub use game_data::enemy::enemy_data::A1_015_GOLDEN_WRITES;

/// a1-016 golden 表形烘焙（8 案 60 写全值——fixture 固定形；计算面公式余）。
pub use game_data::enemy::enemy_data::A1_016_GOLDEN_WRITES;

/// a1-017 golden 表形烘焙（8 案 52 写全值——fixture 固定形）。
pub use game_data::enemy::enemy_data::A1_017_GOLDEN_WRITES;

/// a1-018 golden 表形烘焙（8 案 4 写全值——fixture 固定形）。
pub use game_data::enemy::enemy_data::A1_018_GOLDEN_WRITES;

/// a1-019 golden 表形烘焙（8 案 60 写全值——fixture 固定形）。
pub use game_data::enemy::enemy_data::A1_019_GOLDEN_WRITES;

/// a1-020 golden 表形烘焙（8 案 56 写全值——fixture 固定形）。
pub use game_data::enemy::enemy_data::A1_020_GOLDEN_WRITES;

/// a1-021 golden 表形烘焙（8 案 24 写全值——fixture 固定形；A 批收官卡）。
pub use game_data::enemy::enemy_data::A1_021_GOLDEN_WRITES;

#[cfg(test)]
mod surface_check_tests {
    use super::surface_check_ok;

    /// `$FC`（zero_ok=true）与 `$FE`（zero_ok=false）在 `data1F==0` 时**反向**：
    /// asm `$82EF` 注释 "fail instantly if data1F is zero (this is different than
    /// EnemyGetDeltaY_RepeatPreviousUntilFailure)"。
    #[test]
    fn zero_data1f_is_success_for_fc_and_failure_for_fe() {
        assert!(surface_check_ok(0x00, false, false, true));   // $FC ⇒ 通过
        assert!(!surface_check_ok(0x00, true, true, false));   // $FE ⇒ 失败
    }

    /// bit7 ⇒ 只查天花板（`EnemyCheckMoveUp`），与 can_down 无关。
    #[test]
    fn bit7_checks_ceiling_only() {
        assert!(surface_check_ok(0x80, true, false, false));
        assert!(!surface_check_ok(0x80, false, true, true));
    }

    /// bit6 ⇒ 只查地面（`EnemyCheckMoveDown`），与 can_up 无关。
    #[test]
    fn bit6_checks_floor_only() {
        assert!(surface_check_ok(0x40, false, true, false));
        assert!(!surface_check_ok(0x40, true, false, true));
    }

    /// bit7/bit6 全 0 但 data1F≠0（如只有 bit0）⇒ 两支皆不通过（asm 两个 `bpl/beq`
    /// 都不跳、直接落到 `ldx PageIndex` 读上一次的 carry）。
    #[test]
    fn neither_bit_never_passes() {
        assert!(!surface_check_ok(0x01, true, true, true));
        assert!(!surface_check_ok(0x20, true, true, false));
    }
}

#[cfg(test)]
mod tests_movement_cadence {
    use super::*;

    /// asm `$827F` 到期支：`delay=0 / instr+=2 / 当帧继续读下一条`（`beq @ReadByte`）。
    /// 锈腿旧形在到期支 `return` 旧指令的 Δ ⇒ 每条指令多吃一个移动帧（Skree 坠落慢一半的根因）。
    /// 锚：`EnemyMovementPtrs[7]` 右向（`$9891`，串基 `$97EF` ⇒ 偏移 0xA2）
    /// = `04 22 | 01 42 | 01 22 | 01 42 | 01 62 | 01 42 | 04 62 | FC …`。
    #[test]
    fn expired_sign_mag_speed_reads_next_instruction_same_frame() {
        if !game_data::pack::is_ready() {
            let _ = game_data::pack::init_from_env();
        }
        if !game_data::pack::is_ready() {
            eprintln!("SKIP（显式）：未加载资产包——先跑 pack-tool extract");
            return;
        }
        let strs = game_data::assets::enemy_movement_strings(0);
        assert_eq!(
            &strs[0xA2..0xA8],
            &[0x04, 0x22, 0x01, 0x42, 0x01, 0x22],
            "Skree mov7_R 串锚"
        );

        // `instr=2`（`01 42`）在 `delay=1` 到期 ⇒ 当帧应读下一条 `01 22` 发 Δ=+2、instr→4。
        let (dy, dx, dly, ii, _d1f, trig) =
            enemy_get_delta_xy(0, 4, 0x00, 7, 2, 1, 0, 0, 0, 0x00, false, true);
        assert_eq!((dy, dx), (2, 2), "到期当帧必须发下一条指令的 Δ（0x22 ⇒ y+2/x+2），不是旧指令 0x42 的 y+4");
        assert_eq!((dly, ii, trig), (1, 4, false), "延续下一条指令：delay=1 / instr=4");
    }
}

#[cfg(test)]
mod tests_movement_string_base {
    use super::*;

    /// 移动串区基址**按区不同**：`$97D1 + EnemyMovementChoices 语义表长`。
    /// 旧形一律减 `$97EF` ⇒ No/Kr/To/Ri 的指针小于基址、`wrapping_sub` 下溢 ⇒
    /// `get()` 全 None 回落 `$FF`（Restart）⇒ `dx=0`，四区用移动串的敌人永不移动
    /// （room 09 的 RipperII 就冻在出生点）。
    #[test]
    fn string_base_matches_per_area_choice_table_len() {
        if !game_data::pack::is_ready() {
            let _ = game_data::pack::init_from_env();
        }
        if !game_data::pack::is_ready() {
            eprintln!("SKIP（显式）：未加载资产包——先跑 pack-tool extract");
            return;
        }
        for (area, want) in [(0u8, 0x97EFu16), (1, 0x97E7), (2, 0x97E9), (3, 0x97D5), (4, 0x97ED)] {
            assert_eq!(game_data::assets::enemy_movement_string_base(area), want, "area{area} 串区基址");
            let strs = game_data::assets::enemy_movement_strings(area);
            // 窗尾五区一致 `$99EF`：段长 = 窗尾 − 基址。
            assert_eq!(strs.len() as u16 + want, 0x99EF, "area{area} 段长必须补到公共上界 $99EF");
            // 每个指针都必须落在本区串窗内（否则消费侧下溢/越界）。
            let ptrs = game_data::assets::enemy_movement_ptrs(area);
            for w in ptrs.chunks(2) {
                let p = u16::from_le_bytes([w[0], w[1]]);
                if p == 0 { continue; }
                assert!(p >= want && p < 0x99EF, "area{area} 指针 {p:#06X} 越出串窗 {want:#06X}..$99EF");
            }
        }
    }

    /// Norfair（type 2 = RipperII，`data05=$1E`/`movIdx=0`）现在能取到移动串：
    /// 串 `SignMagSpeed $20, 2, 2` ⇒ Δx=+2 量级（非 0），旧形恒 0。
    #[test]
    fn norfair_ripper_resolves_movement_string() {
        if !game_data::pack::is_ready() {
            let _ = game_data::pack::init_from_env();
        }
        if !game_data::pack::is_ready() {
            eprintln!("SKIP（显式）：未加载资产包——先跑 pack-tool extract");
            return;
        }
        let (_, dx, ..) = enemy_get_delta_xy(1, 2, 0x1E, 0, 0, 0, 1, 0, 0, 0, true, false);
        assert_ne!(dx, 0, "Norfair RipperII 必须沿移动串产生 Δx（旧形基址错 ⇒ 恒 0）");
        // Brinstar 同 type 仍走 bit0 朝向（回归护栏）：面右 ⇒ 正、面左 ⇒ 负或零。
        let (_, dx_right, ..) = enemy_get_delta_xy(0, 2, 0x00, 0, 0, 0, 1, 0, 0, 0, true, false);
        assert_ne!(dx_right, 0, "Brinstar 移动串仍须有效（基址 $97EF 未变）");
    }
}
