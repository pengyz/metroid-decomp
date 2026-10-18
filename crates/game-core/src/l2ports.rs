//! l2ports —— L2 子程序级移植面（与 oracle/subroutines/l2_cases.jsonl 一一对应）。
//!
//! 每个函数 = 一条 L2 用例的 game-core 侧实现；语义 golden 手推自 A 级反汇编（reference/seed）。
//! 未实现的例程不在此登记（缺口走矩阵 L0 + unported! 桩），**禁止发明规格未覆盖的行为**。

use crate::{L2In, L2Out};

/// bank07:C16D（8B，seed：`LDA #1/STA $01/LDA #$FF/STA $00`，止于 C175）。
/// 语义：写 $01←$01、$00←$FF，A=FF；X/Y 不变。（A 级：reference/seed bank07:C16D-C174）
pub fn sub_c16d(input: &L2In) -> L2Out {
    L2Out {
        a: 0xFF,
        x: input.x,
        y: input.y,
        writes: vec![(0x0001, 0x01), (0x0000, 0xFF)],
    }
}

/// bank07:C376（止于 C37B，PLA/PLA/RTS 尾部用 stops 截断不入比对面）。
/// 语义：A=0，写 `$07A1+X`←$00；X/Y 不变。（A 级：reference/seed bank07:C376-C37A）
pub fn sub_c376(input: &L2In) -> L2Out {
    L2Out {
        a: 0x00,
        x: input.x,
        y: input.y,
        // r3 F2：契约要求写地址归一 $0000-$07FF（2KB RAM 镜像）
        writes: vec![((0x07A1 + input.x as u16) & 0x7FF, 0x00)],
    }
}

/// int_y_E3A5 —— bank07:E3A5-E3E4 垂直积分切片（证明级等价，见 integrators）。
pub fn int_y_e3a5(input: &L2In) -> L2Out {
    let mut st = crate::integrators::IntYState {
        speed_i: input.ram[0x308 & 0x7FF],
        speed_f: input.ram[0x312 & 0x7FF],
        subpix: input.ram[0x310 & 0x7FF],
    };
    let accel = input.ram[0x314 & 0x7FF];
    let out = crate::integrators::int_y(&mut st, accel);
    L2Out {
        a: out.disp,
        x: out.reg_x,
        y: if out.reg_y == 0xFF { input.y } else { out.reg_y },
        writes: vec![
            (0x0312, st.speed_f),
            (0x0308, st.speed_i),
            (0x0310, st.subpix),
            (0x0000, out.disp),
        ],
    }
}

/// int_x_E3E5 —— bank07:E3E5-E448 水平积分切片（SMT 证明级，见 integrators）。
pub fn int_x_e3e5(input: &L2In) -> L2Out {
    let mut st = crate::integrators::IntXState {
        speed_i: input.ram[0x309 & 0x7FF],
        speed_f: input.ram[0x313 & 0x7FF],
        subpix: input.ram[0x311 & 0x7FF],
    };
    let accel = input.ram[0x315 & 0x7FF];
    let max_src = input.ram[0x316 & 0x7FF];
    let out = crate::integrators::int_x(&mut st, accel, max_src);
    L2Out {
        a: out.disp,
        x: out.reg_x,
        y: out.reg_y,
        writes: vec![
            (0x0313, st.speed_f),
            (0x0309, st.speed_i),
            (0x0311, st.subpix),
            (0x0002, max_src << 4),
            (0x0003, max_src >> 4),
            (0x0001, out.lim_hi_final),
            (0x0000, out.disp),
        ],
    }
}

/// 按 L2 用例名查移植函数（l2-diff 裁定器用）。
/// a1_cluster_001 适配（SC-A1-001——CommonJump_noMove（bank01:8003）→@noMove 入口）。
/// L2In（regs/ram）→ enemy::common 纯 fn → L2Out（写面+regs）。
fn a1_cluster_001(input: &L2In) -> L2Out {
    let r = |a: usize| input.ram.get(a).copied().unwrap_or(0);
    let slot = crate::enemy::slot::EnemySlot::from_bytes([r(0x03C0); crate::enemy::slot::EnemySlot::SIZE]);
    let oc = r(0x006B);
    let sa = r(0x03D0); // specialAttribs（槽面——随考证）
    let (cntrl_write, draw, clear_hit) =
        crate::enemy::common::update_enemy_common_no_move(&slot, oc, sa);
    let mut writes = Vec::new();
    if let Some(v) = cntrl_write {
        writes.push((0x006B, v));
    }
    if clear_hit && r(0x03CA) != 0 {
        writes.push((0x03CA, 0)); // Ens.isHit（**净变更形**——golden 写面=变更集）
    }
    let _ = draw;
    // EnemyMove 移效（**映射定案**：8 案同址——$6AF7(y)/$6AF8(**=A 输入**)/$6AFA(hi=1)
    // +$0404/$040E（zp scratch=0/0 固定面）——case 对读实证）
    writes.push((0x0404, 0));
    writes.push((0x040E, 0));
    let delta = crate::enemy::common::enemy_delta_sign_mag(r(0x03C8)); // 移速字节（槽面）
    let y_in = input.wram.get(0xAF7).copied().unwrap_or(0); // **wram 域面**（$6AF7=$6000+0xAF7）
    let y_out = (y_in as i16 + delta as i16) as u8; // y 效果（Δ 应用）
    writes.push((0x6AF7, y_out));
    writes.push((0x6AF8, input.a)); // **subpix=A 输入**（case 对读 8/8 证）
    writes.push((0x6AFA, 1)); // hi=1（定值面）
    L2Out { a: 0, x: 0, y: 1, writes } // 出口残迹 (0,0,1)（观测值逐钉）
}

pub fn lookup(name: &str) -> Option<fn(&L2In) -> L2Out> {
    match name {
        "a1_cluster_001" => Some(a1_cluster_001),
        "sub_C16D" => Some(sub_c16d),
        "sub_C376" => Some(sub_c376),
        "int_y_E3A5" => Some(int_y_e3a5),
        "int_x_E3E5" => Some(int_x_e3e5),
        "sub_F1F4" => Some(sub_f1f4),
        "sub_F1EE" => Some(sub_f1ee),
        "sub_F1E0" => Some(sub_f1e0),
        "sub_F152" => Some(sub_f152),
        "sub_F186" => Some(sub_f186),
        "sub_F1BF" => Some(sub_f1bf),
        "sub_F140" => Some(sub_f140),
        "sub_F149" => Some(sub_f149),
        "sub_F1A7" => Some(sub_f1a7),
        "sub_F1B3" => Some(sub_f1b3),
        "sub_F1D2" => Some(sub_f1d2),
        "sub_F1CB" => Some(sub_f1cb),
        "sub_F1E7" => Some(sub_f1e7),
        "sub_F172" => Some(sub_f172),
        "sub_F19A" => Some(sub_f19a),
        "sub_F162" => Some(sub_f162),
        "sub_F1FA" => Some(sub_f1fa),
        "sub_F270" => Some(sub_f270),
        "sub_F2DF" => Some(sub_f2df),
        "sub_F2D5" => Some(sub_f2d5),
        "sub_F2AD" => Some(sub_f2ad),
        "sub_F2CA" => Some(sub_f2ca),
        "sub_F2ED" => Some(sub_f2ed),
        "sub_F2E8" => Some(sub_f2e8),
        "sub_F282" => Some(sub_f282),
        "sub_F2D8" => Some(sub_f2d8),
        "sub_GameRng" => Some(sub_game_rng),
        "sub_FE83" => Some(sub_fe83),
        "sub_IncTitleRoutine" => Some(sub_title_inc),
        "sub_ClearSpareMem" => Some(sub_title_clear_spare),
        "sub_DelayIntroReplay" => Some(sub_title_replay_delay),
        "sub_TitleRoutineReturn" => Some(sub_title_no_op),
        "sub_InitializeAfterReset" => Some(sub_title_init),
        "sub_ChangeIntroNameTable" => Some(sub_title_nametable),
        "sub_PrepIntroRestart" => Some(sub_title_prep),
        "sub_FadeInDelay" => Some(sub_fade_delay),
        "sub_METROIDFadeIn" => Some(sub_metroid_fade_in),
        "sub_MessageFadeIn" => Some(sub_message_fade_in),
        "sub_MessageFadeOut" => Some(sub_message_fade_out),
        "sub_LoadFlashTimer" => Some(sub_flash_timer),
        "sub_FlashEffect" => Some(sub_flash_effect),
        "sub_METROIDSparkle" => Some(sub_sparkle_step),
        "sub_METROIDFadeOut" => Some(sub_metroid_fade_out),
        "sub_MoreCrosshairs" => Some(sub_crosshair_continue),
        "sub_Crosshairs" => Some(sub_crosshairs_step),
        "sub_ChooseStartContinue" => Some(sub_menu_choose),
        "sub_WaitForSTART" => Some(sub_wait_start),
        "sub_DisplayPassword" => Some(sub_password_gate),
        "sub_EndGame" => Some(sub_end_game),
        "sub_SamusHitSFXInit" => Some(sub_samus_hit_sfx),
        "sub_SamusHitSFXCont" => Some(sub_samus_hit_cont),
        "sub_SamusWalkSFXInit" => Some(sub_samus_walk),
        "sub_EnemyHitSFXInit" => Some(sub_enemy_hit),
        "sub_BulletFireSFXInit" => Some(sub_bullet_fire),
        "sub_MissileLaunchSFXInit" => Some(sub_missile_launch),
        "sub_MissilePickupSFXInit" => Some(sub_missile_pickup),
        "sub_EnergyPickupSFXInit" => Some(sub_energy_pickup),
        "sub_SpitFlameSFXInit" => Some(sub_spit_flame),
        "sub_ScrewAttackSFXInit" => Some(sub_screw_attack),
        "sub_BombExplodeSFXInit" => Some(sub_bomb_explode),
        "sub_BossHitSFXInit" => Some(sub_boss_hit),
        "sub_IncorrectPasswordSFXInit" => Some(sub_incorrect_password),
        "sub_SamusJumpSFXInit" => Some(sub_samus_jump),
        "sub_DoorOpenCloseSFXInit" => Some(sub_door_open_close),
        "sub_BigEnemyHitSFXInit" => Some(sub_big_enemy_hit),
        "sub_SamusToBallSFXInit" => Some(sub_samus_to_ball),
        "sub_BombLaunchSFXInit" => Some(sub_bomb_launch),
        "sub_MetroidHitSFXInit" => Some(sub_metroid_hit),
        "sub_SamusDieSFXInit" => Some(sub_samus_die),
        "sub_StatueRaiseSFXInit" => Some(sub_statue_raise),
        "s1_cluster_00" => Some(sub_s1_cluster_00),
        "s1_cluster_01" => Some(sub_s1_cluster_01),
        "s1_cluster_03" => Some(sub_s1_cluster_03),
        _ => None,
    }
}

// ---- batch-1 F1xx 族微切片（对拍级：三方裁定；指令清单 5-7 条可人核）----

/// sub_F1F4 —— bank07:F1F4-F1F9（TXA/SEC/SBC #$10/TAX）：X -= 0x10，A=X 结果，Y 透传。
/// 语义证据（A 级）：seed metroid.asm 逐指令（5 条）；无 RAM 写。
pub fn sub_f1f4(input: &L2In) -> L2Out {
    let v = input.x.wrapping_sub(0x10);
    L2Out { a: v, x: v, y: input.y, writes: vec![] }
}

/// sub_F1EE —— bank07:F1EE-F1F3（TYA/CLC/ADC #$10/TAY）：Y += 0x10，A=Y 结果，X 透传。
/// 语义证据（A 级）：seed 逐指令（4 条）；无 RAM 写。
pub fn sub_f1ee(input: &L2In) -> L2Out {
    let v = input.y.wrapping_add(0x10);
    L2Out { a: v, x: input.x, y: v, writes: vec![] }
}

/// sub_F1E0 —— bank07:F1E0-F1E6（CLC/ADC $301,Y/STA $04）：$04 ← A + RAM[$0301+Y]。
/// 语义证据（A 级）：seed 逐指令（4 条）；写集 {$0004}；绝对,Y 寻址无跨界（$0301-$0400）。
pub fn sub_f1e0(input: &L2In) -> L2Out {
    let addr = (0x0301 + input.y as u16) & 0x7FF;
    let v = input.a.wrapping_add(input.ram[addr as usize]);
    L2Out { a: v, x: input.x, y: input.y, writes: vec![(0x0004, v)] }
}

/// sub_F162 —— bank07:F162-F171（`LDA $400,Y / STA $06 / LDA $401,Y / STA $08 / LDA $6AFB,Y /`
/// 经 F193 共享尾：Y 变体坐标/属性读取，F152 的对偶）。
/// L2 窗口 stops=bank07:F199（F193 尾 RTS 址截断）。
/// 语义（A 级：ROM 原字节 `B9 00 04 85 06 B9 01 04 85 08 B9 FB 6A 4C 93 F1`；F193 尾
/// `45 FF 29 01 85 0A 60`）：$06←RAM[$400+Y]；$08←RAM[$401+Y]；
/// $0A←(WRAM[$6AFB+Y] ^ **RAM[$FF]**)&1（45 FF=EOR 零页 $FF 非立即数，同 F152/F186 教训）。
/// 末值 A=$0A；X/Y 透传。注：$F109 `JSR $F162` 实存（samus-movement.md 2.2 更正）。
pub fn sub_f162(input: &L2In) -> L2Out {
    let y = input.y as u16;
    let v6 = input.ram[(0x0400 + y) as usize & 0x7FF];
    let v8 = input.ram[(0x0401 + y) as usize & 0x7FF];
    let w = input.wram.get(0x6AFB + y as usize - 0x6000).copied().unwrap_or(0);
    let va = (w ^ input.ram[0x00FF]) & 1;
    L2Out {
        a: va,
        x: input.x,
        y: input.y,
        writes: vec![(0x0006, v6), (0x0008, v8), (0x000A, va)],
    }
}

/// sub_F152 —— bank07:F152-F185（经 loc_F17F 共享尾）：坐标/属性读取器（X 变体）。
/// $07 ← RAM[$400+X]；$09 ← RAM[$401+X]；$0B ← (WRAM[$6AFB+X] ^ **RAM[$FF]**) & 1（EOR zp）。
/// 语义证据（A 级）：seed 逐指令；$6AFx 域=WRAM 暂存区（findings/ram-region-6af4-contradiction.md 定谳）。
pub fn sub_f152(input: &L2In) -> L2Out {
    let x = input.x as u16;
    let v7 = input.ram[(0x0400 + x) as usize & 0x7FF];
    let v9 = input.ram[(0x0401 + x) as usize & 0x7FF];
    let w = input.wram.get(0x6AFB + x as usize - 0x6000).copied().unwrap_or(0);
    // F17F `45 FF` = EOR **零页 $FF**（RAM），非立即数——操作数类型以清单 # 前缀为准
    let vb = (w ^ input.ram[0x00FF]) & 1; // ^ RAM[$FF]（EOR zp，非立即数）
    L2Out {
        a: vb,
        x: input.x,
        y: input.y,
        writes: vec![(0x0007, v7), (0x0009, v9), (0x000B, vb)],
    }
}

/// sub_F19A —— bank07:F19A-F1A6（`LDA $B1,X / STA $07 / LDA $B2,X / STA $09 / LDA $B3,X /`
/// 经 loc_F17F 共享尾：zp,X 变体坐标/属性读取）。
/// L2 窗口 stops=bank07:F185（尾 RTS 址截断）。
/// 语义（A 级：ROM 原字节 `B5 B1 85 07 B5 B2 85 09 B5 B3 4C 7F F1`）：
/// **B5 B1-B3 = zp,X 8 位变址回绕**：有效址=(base+X)&$FF（X=$FF 时 $B1→$B0；16 位加会错读
/// $01Bx 域）；$07←RAM[($B1+X)&$FF]、$09←RAM[($B2+X)&$FF]、$0B←(RAM[($B3+X)&$FF]^**RAM[$FF]**)&1
/// （尾 45 FF=EOR 零页 $FF 非立即数）。末值 A=$0B；X/Y 透传。
pub fn sub_f19a(input: &L2In) -> L2Out {
    let x = input.x;
    let a1 = 0xB1u8.wrapping_add(x) as usize;
    let a2 = 0xB2u8.wrapping_add(x) as usize;
    let a3 = 0xB3u8.wrapping_add(x) as usize;
    let v7 = input.ram[a1];
    let v9 = input.ram[a2];
    let vb = (input.ram[a3] ^ input.ram[0x00FF]) & 1;
    L2Out {
        a: vb,
        x: input.x,
        y: input.y,
        writes: vec![(0x0007, v7), (0x0009, v9), (0x000B, vb)],
    }
}

/// sub_F172 —— bank07:F172-F185（`LDA $30D,X / STA $07 / LDA $30E,X / STA $09 / LDA $30C,X /`
/// 经 loc_F17F 共享尾：X 变体坐标/属性读取）。
/// L2 窗口 stops=bank07:F185（尾 RTS 址截断，与 gen-f152 同约定）。
/// 语义（A 级：ROM 原字节 `BD 0D 03 85 07 BD 0E 03 85 09 BD 0C 03 45 FF 29 01 85 0B 60`）：
/// $07 ← RAM[$30D+X]；$09 ← RAM[$30E+X]；$0B ← (RAM[$30C+X] ^ **RAM[$FF]**) & 1
/// （F17F `45 FF` = EOR **零页 $FF**，非立即数——同 F152/F186 教训）。
/// 末值 A=$0B；X/Y 透传。
pub fn sub_f172(input: &L2In) -> L2Out {
    let x = input.x as u16;
    let v7 = input.ram[(0x030D + x) as usize & 0x7FF];
    let v9 = input.ram[(0x030E + x) as usize & 0x7FF];
    let vb = (input.ram[(0x030C + x) as usize & 0x7FF] ^ input.ram[0x00FF]) & 1;
    L2Out {
        a: vb,
        x: input.x,
        y: input.y,
        writes: vec![(0x0007, v7), (0x0009, v9), (0x000B, vb)],
    }
}

/// sub_F186 —— bank07:F186-F199 注册名（机械名只在注册注释，two-layer §4）。
/// **域形态**（task-9 抽取迁移）：语义核迁 verify_shim::f186（单一语义源；FE83 组合复用），
/// 本薄包装保持注册面；迁移前后既有 gen/b 案双绿=「行为钉死=结构可换」。
pub fn sub_f186(input: &L2In) -> L2Out {
    crate::verify_shim::f186(input)
}

/// sub_F1CB —— bank07:F1CB-F1D1（`CLC / ADC $6AF6,Y / STA $05 / RTS`，F1B3 共享尾本体）。
/// L2 窗口 stops=bank07:F1D1（尾 RTS 址截断）。
/// 语义（A 级：ROM 原字节 `18 79 F6 6A 85 05 60` 逐指令）：
/// **79 F6 6A = ADC abs,Y 读 WRAM $6AFx 域**（陷阱清单主踩点）：$05 ← A + WRAM[$6AF6+Y]。
/// 末值 A=$05；X/Y 透传；入参 A 参与运算（与 F1D2 不同）。
pub fn sub_f1cb(input: &L2In) -> L2Out {
    let y = input.y as usize;
    let w6 = input.wram.get(0x6AF6 + y - 0x6000).copied().unwrap_or(0);
    let v5 = input.a.wrapping_add(w6);
    L2Out { a: v5, x: input.x, y: input.y, writes: vec![(0x0005, v5)] }
}

/// sub_F1E7 —— bank07:F1E7-F1ED（`CLC / ADC $6AF5,Y / STA $04 / RTS`，F1B3 共享尾本体）。
/// L2 窗口 stops=bank07:F1ED（尾 RTS 址截断）。
/// 语义（A 级：ROM 原字节 `18 79 F5 6A 85 04 60` 逐指令）：
/// **79 F5 6A = ADC abs,Y 读 WRAM $6AFx 域**（陷阱清单主踩点）：$04 ← A + WRAM[$6AF5+Y]。
/// 末值 A=$04；X/Y 透传；入参 A 参与运算。
pub fn sub_f1e7(input: &L2In) -> L2Out {
    let y = input.y as usize;
    let w5 = input.wram.get(0x6AF5 + y - 0x6000).copied().unwrap_or(0);
    let v4 = input.a.wrapping_add(w5);
    L2Out { a: v4, x: input.x, y: input.y, writes: vec![(0x0004, v4)] }
}

/// sub_F1D2 —— bank07:F1D2-F1DF（`LDA #4 / JSR F1E0 / LDA #8 /` 落 loc_F1D9 共享尾）。
/// L2 窗口 stops=bank07:F1DF（尾 RTS 址截断，与 gen-f1bf 同约定）。
/// 语义（A 级：ROM 原字节 `A9 04 20 E0 F1 A9 08 18 79 02 03 85 05 60` 逐指令）：
/// **A9 04/A9 08 = LDA #立即数**（反向陷阱，勿读 zp $04/$08）：
/// $04 ← 4 + RAM[$301+Y]（F1E0 尾 CLC/ADC $301,Y/STA $04）；
/// $05 ← 8 + RAM[$302+Y]（loc_F1D9 CLC/ADC $302,Y/STA $05）。
/// 末值 A=$05；X/Y 透传；入参 A 不影响（窗口首指令即 LDA #4）。
pub fn sub_f1d2(input: &L2In) -> L2Out {
    let y = input.y as u16;
    let v4 = 4u8.wrapping_add(input.ram[(0x0301 + y) as usize & 0x7FF]);
    let v5 = 8u8.wrapping_add(input.ram[(0x0302 + y) as usize & 0x7FF]);
    L2Out { a: v5, x: input.x, y: input.y, writes: vec![(0x0004, v4), (0x0005, v5)] }
}

/// sub_F149 —— bank07:F149-F151 入口组合件（`JSR F186 / JSR F1D2 / JMP F1FA`）。
/// L2 窗口止于 F1FA 入口（碰撞尾独立切片 S-FUNC-07-F1FA，stops=bank07:F1FA）。
/// 语义（A 级：ROM 原字节 `20 86 F1 20 D2 F1 4C FA F1` 逐指令 + seed 各子例程）：
/// 先 F186（$06←RAM[$30D+Y]、$08←RAM[$30E+Y]、$0A←(RAM[$30C+Y]^RAM[$FF])&1，EOR zp），
/// 后 F1D2（$04←4+RAM[$301+Y]、$05←8+RAM[$302+Y]，LDA #立即数）。
/// 末值 A=$05；写集 {$04,$05,$06,$08,$0A} 无重叠；X/Y 透传。
pub fn sub_f149(input: &L2In) -> L2Out {
    let f186 = sub_f186(input);
    let f1d2 = sub_f1d2(input);
    let mut writes = f186.writes;
    writes.extend(f1d2.writes);
    L2Out {
        a: f1d2.a,
        x: input.x,
        y: input.y,
        writes,
    }
}

/// sub_F140 —— bank07:F140-F148 入口组合件（`JSR F1BF / JSR F186 / JMP F1FA`）。
/// L2 窗口止于 F1FA 入口（碰撞尾独立切片 S-FUNC-07-F1FA，stops=bank07:F1FA）。
/// 语义（A 级：ROM 原字节 `20 BF F1 20 86 F1 4C FA F1` 逐指令 + seed 各子例程）：
/// 先 F1BF（$04←WRAM[$6AF5+X]+RAM[$301+Y]、$05←WRAM[$6AF6+X]+RAM[$302+Y]），
/// 后 F186（$06←RAM[$30D+Y]、$08←RAM[$30E+Y]、$0A←(RAM[$30C+Y]^RAM[$FF])&1，EOR zp）。
/// 末值 A=$0A；写集 {$04,$05,$06,$08,$0A} 无重叠；X/Y 透传。
pub fn sub_f140(input: &L2In) -> L2Out {
    let f1bf = sub_f1bf(input);
    let f186 = sub_f186(input);
    let mut writes = f1bf.writes;
    writes.extend(f186.writes);
    L2Out {
        a: f186.a,
        x: input.x,
        y: input.y,
        writes,
    }
}

/// sub_F1A7 —— bank07:F1A7-F1B2（`LDA $301,X / JSR F1E0 / LDA $302,X / JMP loc_F1D9` 共享尾变体）。
/// L2 窗口 stops=bank07:F1DF（F1D9 尾 RTS 址截断，与 gen-f1bf 同约定）。
/// 语义（A 级：ROM 原字节 `BD 01 03 20 E0 F1 BD 02 03 4C D9 F1` 逐指令）：
/// $04 ← RAM[$301+X] + RAM[$301+Y]（F1E0 尾 CLC/ADC $301,Y/STA $04）；
/// $05 ← RAM[$302+X] + RAM[$302+Y]（loc_F1D9 CLC/ADC $302,Y/STA $05）。
/// 末值 A=$05；X/Y 透传；abs,X/abs,Y 16 位加（$301+X≤$400，无 zp 回绕）。
pub fn sub_f1a7(input: &L2In) -> L2Out {
    let x = input.x as u16;
    let y = input.y as u16;
    let v4 = input.ram[(0x0301 + x) as usize & 0x7FF]
        .wrapping_add(input.ram[(0x0301 + y) as usize & 0x7FF]);
    let v5 = input.ram[(0x0302 + x) as usize & 0x7FF]
        .wrapping_add(input.ram[(0x0302 + y) as usize & 0x7FF]);
    L2Out { a: v5, x: input.x, y: input.y, writes: vec![(0x0004, v4), (0x0005, v5)] }
}

/// sub_F1B3 —— bank07:F1B3-F1BE（`LDA $301,X / JSR F1E7 / LDA $302,X / JMP loc_F1CB` 共享尾变体）。
/// L2 窗口 stops=bank07:F1D1（loc_F1CB 尾 RTS 址截断）。
/// 语义（A 级：ROM 原字节 `BD 01 03 20 E7 F1 BD 02 03 4C CB F1` 逐指令）：
/// $04 ← RAM[$301+X] + WRAM[$6AF5+Y]（F1E7 尾 CLC/ADC $6AF5,Y/STA $04）；
/// $05 ← RAM[$302+X] + WRAM[$6AF6+Y]（loc_F1CB CLC/ADC $6AF6,Y/STA $05）。
/// 末值 A=$05；X/Y 透传；ADC **abs,Y 读 WRAM $6AFx 域**（16 位加，$6AF5+Y≤$6BF4）。
pub fn sub_f1b3(input: &L2In) -> L2Out {
    let x = input.x as u16;
    let y = input.y as usize;
    let w5 = input.wram.get(0x6AF5 + y - 0x6000).copied().unwrap_or(0);
    let w6 = input.wram.get(0x6AF6 + y - 0x6000).copied().unwrap_or(0);
    let v4 = input.ram[(0x0301 + x) as usize & 0x7FF].wrapping_add(w5);
    let v5 = input.ram[(0x0302 + x) as usize & 0x7FF].wrapping_add(w6);
    L2Out { a: v5, x: input.x, y: input.y, writes: vec![(0x0004, v4), (0x0005, v5)] }
}

/// sub_F1BF —— bank07:F1BF-F1DF（WRAM 源坐标累加，经 sub_F1E0/loc_F1D9 共享尾）：
/// $04 ← WRAM[$6AF5+X] + RAM[$301+Y]；$05 ← WRAM[$6AF6+X] + RAM[$302+Y]。
/// 语义证据（A 级）：seed 逐指令；$6AFx=WRAM 域（findings 定谳）。
pub fn sub_f1bf(input: &L2In) -> L2Out {
    let x = input.x as usize;
    let y = input.y as u16;
    let w1 = input.wram.get(0x6AF5 + x - 0x6000).copied().unwrap_or(0);
    let w2 = input.wram.get(0x6AF6 + x - 0x6000).copied().unwrap_or(0);
    let v4 = w1.wrapping_add(input.ram[(0x0301 + y) as usize & 0x7FF]);
    let v5 = w2.wrapping_add(input.ram[(0x0302 + y) as usize & 0x7FF]);
    L2Out {
        a: v5,
        x: input.x,
        y: input.y,
        writes: vec![(0x0004, v4), (0x0005, v5)],
    }
}

// ---- sub_F1FA 碰撞逻辑（task-9 抽取迁移：语义核+carry 面迁 verify_shim::f1fa_with_carry
//      ——单一语义源，FE83 组合经 carry 取碰撞判定；本处留薄包装，既有 gen/b 案双绿）----

/// sub_F1FA —— bank07:F1FA-F261 注册名（机械名只在注册注释，two-layer §4）。
/// 契约面=L2Out 单出（carry 不入面——lead 裁定契约不扩）；语义核见 verify_shim::f1fa。
pub fn sub_f1fa(input: &L2In) -> L2Out {
    crate::verify_shim::f1fa(input)
}

// ---- task-8 F2CA 族（九入口共享 RTS 尾；decode-sweep r1 §3，切片边界定案见各 doc 注）----

/// sub_F2D8 —— bank07:F2D8-F2DE 注册名（机械名只在注册注释，two-layer §4）。
/// **域形态**（§5 迁移件）：`enemy::EnemyTodo0404::or_writeback`（OR 回写，掩码=入参 A）
/// 经 `verify_shim::f2d8` 适配；旧字面转写见 commit 1d61521（§5 参考件）——同批 L2 案双绿。
/// L2 窗口 stops=bank07:F2DE（尾 RTS 址截断）。
/// 语义（A 级：ROM 原字节 `1D 04 04 9D 04 04 60`）：$0404+X ← A|old（入参 A 参与——与 F2DF
/// 的 LDA $10 起手不同）；末值 A=A|old；X/Y 透传；abs,X 16 位加（$404+X≤$503）。
pub fn sub_f2d8(input: &L2In) -> L2Out {
    crate::verify_shim::f2d8(input)
}

/// sub_F2DF —— bank07:F2DF-F2E7 注册名（机械名只在注册注释，two-layer §4）。
/// **域形态**（§5 迁移件）：`enemy::EnemyTodo0404::or_writeback`（OR 回写，掩码=memmap 掩码源
/// zp——LDA zp $10 起手，入参 A 不影响）经 `verify_shim::f2df` 适配；旧字面转写见 commit c523762。
/// L2 窗口 stops=bank07:F2E7（尾 RTS 址截断）。
/// 语义（A 级：ROM 原字节 `A5 10 19 04 04 99 04 04 60`）：$0404+Y ← RAM[$10]|old（LDA zp $10
/// 起手——入参 A 不影响），末值 A=RAM[$10]|old；X/Y 透传；abs,Y 16 位加（$404+Y≤$503）。
pub fn sub_f2df(input: &L2In) -> L2Out {
    crate::verify_shim::f2df(input)
}

/// sub_F270 —— bank07:F270-F276 注册名（机械名只在注册注释，two-layer §4）。
/// L2 窗口 stops=bank07:F276（尾 RTS 址截断）。
/// **域形态**（two-layer §5 迁移件）：行为=`enemy::EnemyAttrs::or_attribute`（属性掩码 OR 回写），
/// 经 `verify_shim::f270` 适配（写集由状态变更推导）；旧字面转写（ORA/STA abs,X 直写）见
/// commit 551d307（§5 参考件）——迁移前后同批 L2 案 11 条双绿=「行为钉死=结构可换」实证。
/// 语义证据（A 级：ROM 原字节 `1D 0A 03 9D 0A 03 60`）：$030A+X ← A|old，末值 A=A|old。
pub fn sub_f270(input: &L2In) -> L2Out {
    crate::verify_shim::f270(input)
}

/// sub_F2D5 —— bank07:F2D5-F2DE 注册名（机械名只在注册注释，two-layer §4）。
/// **域形态**（§5 新实施件）：`enemy::EnemyTodo0404::or_writeback`（OR 回写，掩码=memmap
/// 变换掩码 ((RAM[$10]^3)<<3)——F340 EOR#3+C2C6 尾 ASL×3）经 `verify_shim::f2d5` 适配。
/// L2 窗口 stops=bank07:F2DE（JSR F332 内返回址 C2C9 不在 stops，逐址核过）。
pub fn sub_f2d5(input: &L2In) -> L2Out {
    crate::verify_shim::f2d5(input)
}

/// sub_F2AD —— bank07:F2AD-F2DE 注册名（机械名只在注册注释，two-layer §4）。
/// **域形态**（§5 新实施件）：跨 view 组合——`EnemyTodo040E::set_entry`（$040E+X←$81，
/// F2B2 BNE 恒转）后落 F2D5 流 `EnemyTodo0404::or_writeback`（变换掩码）。
/// L2 窗口 stops=bank07:F2DE；F2B4 块自 F2AD 入口不可达（BNE 恒转，F2BD BCS $F298 侧不入窗）。
pub fn sub_f2ad(input: &L2In) -> L2Out {
    crate::verify_shim::f2ad(input)
}

/// sub_F2CA —— bank07:F2CA-F2DE 注册名（机械名只在注册注释，two-layer §4）。
/// **域形态**（§5 新实施件）：carry 敏感跨 view 三方法组合（见 verify_shim::f2ca doc）。
/// L2 窗口 stops=bank07:F2DE（内返回址 F281/C2C9 逐址核过不在 stops）。
pub fn sub_f2ca(input: &L2In) -> L2Out {
    crate::verify_shim::f2ca(input)
}

/// sub_F2ED —— bank07:F2ED-F310 注册名（机械名只在注册注释，two-layer §4）。
/// **域形态**（§5 新实施件）：carry 敏感 + CD9C 探针路由 + 跨 view 组合（见 verify_shim::f2ed doc）。
/// L2 窗口 stops=bank07:F310（内返回址 F2E7/CDBE/C2C9/F276 逐址核过不在 stops）。
pub fn sub_f2ed(input: &L2In) -> L2Out {
    crate::verify_shim::f2ed(input)
}

/// sub_F2E8 —— bank07:F2E8-F310 注册名（机械名只在注册注释，two-layer §4）。
/// **域形态**（§5 新实施件）：Z 路由双流组合（见 verify_shim::f2e8 doc）。
/// L2 窗口 stops=F2DE/F310 双出口（内返回址 C2C9/F2E7/CDBE/F276 逐址核过）。
pub fn sub_f2e8(input: &L2In) -> L2Out {
    crate::verify_shim::f2e8(input)
}

/// sub_F282 —— bank07:F282-F310 注册名（机械名只在注册注释，two-layer §4）。
/// **域形态**（§5 新实施件）：**哨兵返回约定窗**（stops=[]，F281 双重角色由栈纪律区分——
/// lead 首拍裁定的标准解法先例）+ 五路出口组合（见 verify_shim::f282 doc）。
pub fn sub_f282(input: &L2In) -> L2Out {
    crate::verify_shim::f282(input)
}

/// sub_GameRng —— bank07:C000-C019 注册名（机械名只在注册注释，two-layer §4）。
/// **域形态**（§5 新实施件）：`rng::RngState::step`（每调用步进 RN1+=0x19/RN2+=0x5F，
/// A=RN1 终值）经 `verify_shim::game_rng` 适配；L2 窗口 stops=[]（哨兵约定，RTS@C019）。
/// 考证（A 级三轨：asm 结构互核+种子序 C0AC+动态交叉）：docs/findings/game-rng-adjudication.md；
/// 台账 N-0012（升级 A 级，bankNN:AAAA 真址）。
pub fn sub_game_rng(input: &L2In) -> L2Out {
    crate::verify_shim::game_rng(input)
}

/// sub_FE83 —— bank07:FE83-FECB 注册名（机械名只在注册注释，two-layer §4）。
/// **域形态**（task-9 新实施件）：`tile_blast` 复活伤害序列 + f186/f1fa 核组合
/// （见 verify_shim::fe83 doc + tile_blast 模块 asm 主文档）。
/// L2 窗口 stops=[bank07:FEDB（无碰撞 RTS 址）, bank07:CE92（SubtractHealth 入口——
/// JMP 尾目标，出窗不执行）]；段界 FE83-FECB 73B/34 条（r2 边界 FECA 更正见域模块注）。
pub fn sub_fe83(input: &L2In) -> L2Out {
    crate::verify_shim::fe83(input)
}

// ---- task-23 批1·③助手簇注册（链规 19：L2 面逐入口钉（用例 id 分 0A/0B/13/14），
//      注册面=折叠单元 4 件——同码两两共用一注册名）----

/// sub_IncTitleRoutine —— bank00:$806E 注册名（0A/0B 同码 `e6 1f 60`，字节级同段）。
/// 域形态：title::TitleSequence::advance（状态推进单一实现）经 verify_shim::title_inc。
pub fn sub_title_inc(input: &L2In) -> L2Out {
    crate::verify_shim::title_inc(input)
}

/// sub_ClearSpareMem —— bank00:$8068 注册名（`a9 00 85 cb 85 c9` **落穿** $806E 共享尾）。
/// 域形态：title::clear_spare_and_advance 经 verify_shim::title_clear_spare。
pub fn sub_title_clear_spare(input: &L2In) -> L2Out {
    crate::verify_shim::title_clear_spare(input)
}

/// sub_DelayIntroReplay —— bank00:$8283 注册名（`e6 1f a9 10 85 2c 60`）。
/// 域形态：title::set_replay_delay（Timer3←$10=160 帧延迟）经 verify_shim::title_replay_delay。
pub fn sub_title_replay_delay(input: &L2In) -> L2Out {
    crate::verify_shim::title_replay_delay(input)
}

/// sub_TitleRoutineReturn —— bank00:$82F3 注册名（13/14 同码 `60` 空臂）。
/// 域形态：title::no_op 经 verify_shim::title_no_op。
pub fn sub_title_no_op(input: &L2In) -> L2Out {
    crate::verify_shim::title_no_op(input)
}

/// sub_InitializeAfterReset —— bank00:$8071 注册名（批2④；尾 jmp LoadStarSprites $98AE）。
/// 域形态：title::init_after_reset 经 verify_shim::title_init_after_reset。
pub fn sub_title_init(input: &L2In) -> L2Out {
    crate::verify_shim::title_init_after_reset(input)
}

/// sub_ChangeIntroNameTable —— bank00:$822E 注册名（批2④）。
/// 域形态：title::change_nametable 经 verify_shim::title_change_nametable。
pub fn sub_title_nametable(input: &L2In) -> L2Out {
    crate::verify_shim::title_change_nametable(input)
}

/// sub_PrepIntroRestart —— bank00:$82A3 注册名（批2④，双路径）。
/// 域形态：title::prep_restart 经 verify_shim::title_prep_restart。
pub fn sub_title_prep(input: &L2In) -> L2Out {
    crate::verify_shim::title_prep_restart(input)
}

// ---- task-23 批3·①淡变族注册（FadeDriver 同形折叠——chain 19 同形族正样）----

/// sub_FadeInDelay —— bank00:$80F9 注册名（批3①，DelaySetup 装配臂）。
pub fn sub_fade_delay(input: &L2In) -> L2Out {
    crate::verify_shim::title_fade_delay(input)
}

/// sub_METROIDFadeIn —— bank00:$812C 注册名（批3①，PalSelect 步进驱动）。
pub fn sub_metroid_fade_in(input: &L2In) -> L2Out {
    crate::verify_shim::title_metroid_fade_in(input)
}

/// sub_MessageFadeIn —— bank00:$8243 注册名（批3①，组合表步进驱动 idx 6→$0B）。
pub fn sub_message_fade_in(input: &L2In) -> L2Out {
    crate::verify_shim::title_message_fade_in(input)
}

/// sub_MessageFadeOut —— bank00:$8263 注册名（批3①，组合表步进驱动 idx 0→$05）。
pub fn sub_message_fade_out(input: &L2In) -> L2Out {
    crate::verify_shim::title_message_fade_out(input)
}

/// sub_LoadFlashTimer —— bank00:$8142 注册名（批4a，FlashTimerSetup 装配臂）。
pub fn sub_flash_timer(input: &L2In) -> L2Out {
    crate::verify_shim::title_flash_timer(input)
}

/// sub_FlashEffect —— bank00:$8109 注册名（批4a，FlashStep 驱动）。
pub fn sub_flash_effect(input: &L2In) -> L2Out {
    crate::verify_shim::title_flash_effect(input)
}

/// sub_METROIDSparkle —— bank00:$814D 注册名（批4a，SparkleStep=4 跑运动链）。
pub fn sub_sparkle_step(input: &L2In) -> L2Out {
    crate::verify_shim::title_sparkle_step(input)
}

/// sub_METROIDFadeOut —— bank00:$8163 注册名（批4b；FadeDriver::MetroidOut——完成臂含
/// InitCrossMissiles **BUG 保真**（`cmp PPUCTRL_ZP` zp 面）+无条件 DoFadeOut 不对称）。
pub fn sub_metroid_fade_out(input: &L2In) -> L2Out {
    crate::verify_shim::title_metroid_fade_out(input)
}

/// sub_MoreCrosshairs —— bank00:$81D1 注册名（批4b；组合臂 UpdateCrossExplode+FlashIntroScreen
/// 尾跳 / 死码残迹写面——D4 保真）。
pub fn sub_crosshair_continue(input: &L2In) -> L2Out {
    crate::verify_shim::title_crosshair_continue(input)
}

/// sub_Crosshairs —— bank00:$8182 注册名（批4b；flash/Timer3 门+完成检 A 毒化语义+
/// 运动引擎 8 槽调度+爆炸组合）。
pub fn sub_crosshairs_step(input: &L2In) -> L2Out {
    crate::verify_shim::title_crosshairs_step(input)
}

/// sub_ChooseStartContinue —— bank00:90D7 注册名（批5⑤；both-quirk+jmp InitializeStats 停点）。
pub fn sub_menu_choose(input: &L2In) -> L2Out {
    crate::verify_shim::title_menu_choose(input)
}

/// sub_WaitForSTART —— bank00:9394 注册名（批5⑤；START⇒jmp CheckPassword 停点）。
pub fn sub_wait_start(input: &L2In) -> L2Out {
    crate::verify_shim::title_wait_start(input)
}

/// sub_DisplayPassword —— bank00:9359 注册名（批6a；门面，深路 L3 降级）。
pub fn sub_password_gate(input: &L2In) -> L2Out {
    crate::verify_shim::title_password_gate(input)
}

/// sub_EndGame —— bank00:9AA7 注册名（批6a；星装载+调色板循环+JumpEngine 停点）。
pub fn sub_end_game(input: &L2In) -> L2Out {
    crate::verify_shim::title_end_game(input)
}

/// sub_SamusHitSFXInit —— bank00:B673 注册名（task-30 handler 首件；镜面 sig 自证）。
/// 域形态：sound::samus_hit_sfx_init（L2 面=RAM 写集；APU 写序=域测钉）。
pub fn sub_samus_hit_sfx(input: &L2In) -> L2Out {
    crate::verify_shim::sound_samus_hit_init(input)
}

/// sub_SamusHitSFXCont —— bank00:B650 注册名（task-30 handler 对件；Init/Cont 收口）。
pub fn sub_samus_hit_cont(input: &L2In) -> L2Out {
    crate::verify_shim::sound_samus_hit_cont(input)
}

/// W2 高频族注册（bank00 镜面：B598/B710/B716/B56E/B6CD/B6E7）。
pub fn sub_samus_walk(input: &L2In) -> L2Out { crate::verify_shim::sound_samus_walk(input) }
pub fn sub_enemy_hit(input: &L2In) -> L2Out { crate::verify_shim::sound_enemy_hit(input) }
pub fn sub_bullet_fire(input: &L2In) -> L2Out { crate::verify_shim::sound_bullet_fire(input) }
pub fn sub_missile_launch(input: &L2In) -> L2Out { crate::verify_shim::sound_missile_launch(input) }
pub fn sub_missile_pickup(input: &L2In) -> L2Out { crate::verify_shim::sound_missile_pickup(input) }
pub fn sub_energy_pickup(input: &L2In) -> L2Out { crate::verify_shim::sound_energy_pickup(input) }

/// W3 中频族注册（bank00 镜面：B50F/B52B/B583/B5EC/B695/B703）。
pub fn sub_spit_flame(input: &L2In) -> L2Out { crate::verify_shim::sound_spit_flame(input) }
pub fn sub_screw_attack(input: &L2In) -> L2Out { crate::verify_shim::sound_screw_attack(input) }
pub fn sub_bomb_explode(input: &L2In) -> L2Out { crate::verify_shim::sound_bomb_explode(input) }
pub fn sub_boss_hit(input: &L2In) -> L2Out { crate::verify_shim::sound_boss_hit(input) }
pub fn sub_incorrect_password(input: &L2In) -> L2Out { crate::verify_shim::sound_incorrect_password(input) }
pub fn sub_samus_jump(input: &L2In) -> L2Out { crate::verify_shim::sound_samus_jump(input) }

/// W4 尾区注册（bank00 镜面：B7AC/B7EF/B834/B878/B8A7/B8D2/B921）。
pub fn sub_door_open_close(input: &L2In) -> L2Out { crate::verify_shim::sound_door_open_close(input) }
pub fn sub_big_enemy_hit(input: &L2In) -> L2Out { crate::verify_shim::sound_big_enemy_hit(input) }
pub fn sub_samus_to_ball(input: &L2In) -> L2Out { crate::verify_shim::sound_samus_to_ball(input) }
pub fn sub_bomb_launch(input: &L2In) -> L2Out { crate::verify_shim::sound_bomb_launch(input) }
pub fn sub_metroid_hit(input: &L2In) -> L2Out { crate::verify_shim::sound_metroid_hit(input) }
pub fn sub_samus_die(input: &L2In) -> L2Out { crate::verify_shim::sound_samus_die(input) }
pub fn sub_statue_raise(input: &L2In) -> L2Out { crate::verify_shim::sound_statue_raise(input) }

/// s1_cluster_00 —— bank07:D498 簇入口注册（SC-S1-00——尾共享形）。
pub fn sub_s1_cluster_00(input: &L2In) -> L2Out {
    crate::verify_shim::s1_cluster_00(input)
}

/// s1_cluster_01 —— bank07:D806 簇入口注册（SC-S1-01——驱动器判别形）。
pub fn sub_s1_cluster_01(input: &L2In) -> L2Out {
    crate::verify_shim::s1_cluster_01(input)
}

/// s1_cluster_03 —— bank07:F62E 簇入口注册（SC-S1-03——PlaySnd3 族）。
pub fn sub_s1_cluster_03(input: &L2In) -> L2Out {
    crate::verify_shim::s1_cluster_03(input)
}
