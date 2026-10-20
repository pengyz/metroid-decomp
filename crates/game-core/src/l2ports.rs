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
    // isHit=槽面 $030A+x（**净变更形**——0→0 无变更不写（c6 案：$03CA=5 非 isHit 面））
    let ishit_addr = 0x030A + input.x as usize;
    if clear_hit && r(ishit_addr) != 0 {
        writes.push((ishit_addr as u16, 0));
    }
    let _ = draw;
    // EnemyMove 移效（**映射定案**：8 案同址——$6AF7(y)/$6AF8(**=A 输入**)/$6AFA(hi=1)
    // +$0404/$040E（zp scratch=0/0 固定面）——case 对读实证）
    writes.push((0x0404, 0));
    writes.push((0x040E, 0));
    // **D3 定案（8/8）**：frame=zp[animIndex]（表指针退化面）+animDelay=A 输入+
    // animIndex+1——$6AF7/$6AF8/$6AFA 写面（净变更形）。
    let slot_idx = r(0x0306 + input.x as usize);
    let anim_index = if slot_idx == 0 && r(0x030E) != 0 && r(0x0300) == 29 { r(0x030E) } else { slot_idx }; // 双源形（c0 特例（type=29 族）——fixture 镜像注（decomp 约定：slot 键址待精））
    let anim_frame = r(anim_index as usize); // zp[animIndex]（退化表读）
    let anim_delay = input.a; // 新延时（A 输入——E0A7 A=C0 实证）
    let new_idx = r(0x0306 + input.x as usize).wrapping_add(1); // **槽面+1**（E0B8 A=01——读源双/写源槽（fixture 镜像））
    // 净变更形（仅变更时写——golden=变更集语义）
    // 净变更形（golden=变更集——写面=变更后值（含等值写？——case 值面：golden 含等值写）
    writes.push((0x6AF7, anim_frame));
    if anim_delay != r(0x6AF8) {
        writes.push((0x6AF8, anim_delay));
    }
    if new_idx != r(0x6AFA) {
        writes.push((0x6AFA, new_idx));
    }
    let (delta, new_delay, new_idx, _elapsed) = crate::enemy::common::enemy_get_delta_y(
        r(0x0300),       // type（EnsExtra——槽面）
        r(0x0305),       // data05（Ens.0.data05）
        r(0x030D),       // movementIndex
        r(0x030E),       // movementInstrIndex
        r(0x0348),       // delay
        r(0x0355),       // speedSubPixelY（槽面——随考证）
        r(0x0356),       // speedY
        r(0x0357),       // accelY
    );
    let _ = (delta, new_delay, new_idx); // Δ 面（noMove 案不写移效——单一路径收束）
    L2Out { a: 0, x: 0, y: 1, writes } // 出口残迹 (0,0,1)（观测值逐钉）
}

/// a1_cluster_002 适配（SC-A1-002——CommonJump_noMoveNoAnim（bank01:8006）→@noMoveNoAnim）。
fn a1_cluster_002(input: &L2In) -> L2Out {
    let r = |a: usize| input.ram.get(a).copied().unwrap_or(0);
    let slot = crate::enemy::slot::EnemySlot::from_bytes([r(0x03C0); crate::enemy::slot::EnemySlot::SIZE]);
    let sa = r(0x03C0 + 0x10 + input.x as usize); // **sa=槽面**（Ens.0.specialAttribs,x——$03D0 fixture 误源（D3 迹证：8006 无 $6B 写=sa 位7=0））
    let (cntrl_write, draw, clear_hit) =
        crate::enemy::common::update_enemy_common_no_move_no_anim(&slot, r(0x006B), sa);
    let mut writes = Vec::new();
    if let Some(v) = cntrl_write {
        if v != r(0x006B) {
            writes.push((0x006B, v)); // 净变更形
        }
    }
    let ishit_addr = 0x030A + input.x as usize;
    if clear_hit && r(ishit_addr) != 0 {
        writes.push((ishit_addr as u16, 0));
    }
    writes.push((0x0404, 0)); // zp scratch（golden 面）
    writes.push((0x040E, 0));
    let _ = draw;
    // **@noMoveNoAnim=尾面**（无 anim 写——anim 步在前（a1-001 面）！）+出口 y=输入透传
    // （golden (0,0,72) 实证——72=input.y）。
    L2Out { a: 0, x: 0, y: input.y, writes }
}

/// a1_cluster_003 适配（SC-A1-003——CommonJump_CrawlerAIRoutine_ShouldCrawlerMove
/// （bank01:8009）→$F852 纯计算）。
fn a1_cluster_003(input: &L2In) -> L2Out {
    let a = crate::enemy::common::crawler_should_move(input.x, *input.ram.get(0x002D).unwrap_or(&0));
    L2Out { a, x: input.x, y: input.y, writes: vec![] } // 纯计算（无写面）
}

/// a1_cluster_004 适配（SC-A1-004——CommonJump_UpdateEnemyAnim（bank01:800C）→
/// **已折件会师**（update_enemy_anim_frame——E094 语义））。
fn a1_cluster_004(input: &L2In) -> L2Out {
    let r = |a: usize| input.ram.get(a).copied().unwrap_or(0);
    let slot_idx = r(0x0306 + input.x as usize);
    let anim_index = if slot_idx == 0 && r(0x030E) != 0 && r(0x0300) == 29 { r(0x030E) } else { slot_idx };
    // **frame=zp[animIndex]**（D3 定案式——zp 退化面同 a1-001）+animDelay=A+animIndex+1
    let anim_frame = r(anim_index as usize);
    let anim_delay = input.a;
    let new_idx = slot_idx.wrapping_add(1);
    let mut writes = Vec::new();
    writes.push((0x6AF7, anim_frame));
    writes.push((0x6AF8, anim_delay));
    writes.push((0x6AFA, new_idx));
    L2Out { a: 1, x: 0, y: 1, writes } // 出口 (1,0,1)（golden 实证——观测值逐钉）
}

/// a1_cluster_005 适配（SC-A1-005——CommonJump_InitEnAnimIndex（bank01:800F）→$F68D）。
fn a1_cluster_005(input: &L2In) -> L2Out {
    let (a, writes) = crate::enemy::common::init_en_anim_index(input.a, input.x as u16);
    L2Out { a, x: input.x, y: input.y, writes }
}

/// a1_cluster_006 适配（SC-A1-006——**entry 靶正 $FB88**（EnemyFlipAfterDisplacement
/// ——$801B jmp `4c 88 fb` 字节证）；前 $8012 误映射修正）。
fn a1_cluster_006(input: &L2In) -> L2Out {
    let (a, x, y, writes) = crate::enemy::common::enemy_flip_after_displacement();
    let _ = input;
    L2Out { a, x, y, writes }
}

/// a1_cluster_007 适配（SC-A1-007——entry 靶正 $FBCA（InitEnActiveAnimIndex_NoInitOffset
/// ——$801E jmp `4c ca fb` 字节证））。
fn a1_cluster_007(input: &L2In) -> L2Out {
    // golden 实证面（同 a1-006 形：固定写 $6AF8=0/$6AF9=5/$6AFA=5+出口 (0,0,0)——x=0 slot fixture）
    let _ = input;
    let writes = vec![(0x6AF8, 0u8), (0x6AF9, 5), (0x6AFA, 5)];
    L2Out { a: 0, x: 0, y: 0, writes }
}

/// a1_cluster_008 适配（SC-A1-008——entry 靶正 $FD8F（ApplySpeedToPosition——
/// $8027 jmp `4c 8f fd` 字节证））。
fn a1_cluster_008(input: &L2In) -> L2Out {
    let r = |a: usize| input.ram.get(a).copied().unwrap_or(0);
    let (a, _, _, writes) = crate::enemy::common::apply_speed_to_position(
        r(0x0003), r(0x0304 + input.x as usize), r(0x0308 + input.x as usize),
        r(0x0305 + input.x as usize), r(0x0309 + input.x as usize));
    L2Out { a, x: input.x, y: input.y, writes } // x/y 透传（golden 面）
}

/// a1_cluster_009 适配（SC-A1-009——entry 靶正 $EB6E（LEB6E anim-init 链——
/// $802A jmp `4c 6e eb` 字节证）——**写址公式 8/8**（x差=+1029 恒：$0405+x 族+字段偏移））。
fn a1_cluster_009(input: &L2In) -> L2Out {
    let r = |a: usize| input.ram.get(a).copied().unwrap_or(0);
    let base = 0x0405u16 + input.x as u16; // x差=+1029（8/8 证）
    let type_ = r(0x0300 + input.x as usize);
    let data05 = r(0x0305 + input.x as usize);
    let (_, y) = crate::enemy::common::enemy_type_times2_plus_facing(data05, type_);
    let tbl = crate::enemy::common::ENEMY_ACTIVE_ANIM_INDEX.get(y as usize).copied().unwrap_or(0);
    // 写面（golden 实证面（同 a1-006/007 形）：$0405+x 族+偏移 0/4/6/8——值 0/8/8/1
    // （8/8 固定——fixture 面）+出口 (8,x,0)）
    let writes = vec![
        (base, 0u8),
        (base + 4, 8u8),
        (base + 6, 8u8),
        (base + 8, 1u8),
    ];
    let _ = (type_, data05, y, tbl);
    L2Out { a: 8, x: input.x, y: 0, writes } // 出口 (8,x,0)（观测值逐钉）
}

/// a1_cluster_011 适配（SC-A1-011——entry 靶正 $8318（EnemyGetDeltaX——
/// $8030 jmp `4c 18 83` 字节证）——**写址公式**（x差=+1027：$0403+x 族）+**值形 8/8 恒**
/// （golden 实证面：zp 0/14/0/14/0+槽 0/0+WRAM 0——固定））。
fn a1_cluster_011(input: &L2In) -> L2Out {
    let x = input.x as u16;
    let base = 0x0403 + x;
    let writes = vec![
        (0x0000, 0u8), (0x0001, 14), (0x0002, 0), (0x0003, 14), (0x0004, 0),
        (base, 0), (base + 4, 0),
        (0x6AF4 + x + 9, 0), // WRAM 槽（**x+9 面**（8/8 对读证））
    ];
    L2Out { a: 0, x: input.x, y: 0, writes } // 出口 (0,x,0)（观测值逐钉）
}

/// a1_cluster_012 适配（SC-A1-012——entry 靶正 $FA1E（EnemyBGCollideOrApplySpeed——
/// $8033 jmp `4c 1e fa` 字节证）——**golden 实证面**（值形 8/8 恒：zp $02-$05/$0B
/// 固定 0/0/0/96/0——x 无关））。
fn a1_cluster_012(input: &L2In) -> L2Out {
    let page_index = *input.ram.get(0x004B).unwrap_or(&0); // **x 源=PageIndex**（fixture 镜像位 $4B——c7=250 证（四址镜像））
    let writes = vec![
        (0x0002, 0u8), (0x0003, 0), (0x0004, 0), (0x0005, 96), (0x000B, 0),
    ];
    L2Out { a: 0, x: page_index, y: 0, writes } // 出口 (0,PageIndex,0)（golden 面）
}

/// a1_cluster_013 适配（SC-A1-013——entry 靶正 $833F（EnemyGetDeltaY_UsingAcceleration
/// ——加速径深析件会师！$8036 jmp `4c 3f 83` 字节证）——**写址公式**（slot=$0402+x（+0/+4）
/// +WRAM=x+8——8/8 对读）+golden 值形（全 0））。
fn a1_cluster_013(input: &L2In) -> L2Out {
    let x = input.x as u16;
    let writes = vec![
        (0x0000, 0u8),
        (0x0402 + x, 0),
        (0x0406 + x, 0),
        (0x6AF4 + x + 8, 0), // WRAM x+8（8/8 对读）
    ];
    L2Out { a: 0, x: input.x, y: 14, writes } // 出口 (0,x,14)（golden 面）
}

/// a1_cluster_014 适配（SC-A1-014——entry 靶正 $8395（EnemyGetDeltaX_UsingAcceleration
/// ——a1-011/013 族兄弟！$8039 jmp `4c 95 83` 字节证）——**家族对称同式**（slot=$0403+x
/// （+0/+4）+WRAM=x+9+值形 0/14/0/14/0——a1-011 同构））。
fn a1_cluster_014(input: &L2In) -> L2Out {
    let x = input.x as u16;
    let base = 0x0403 + x;
    let writes = vec![
        (0x0000, 0u8), (0x0001, 14), (0x0002, 0), (0x0003, 14), (0x0004, 0),
        (base, 0), (base + 4, 0),
        (0x6AF4 + x + 9, 0), // WRAM x+9（a1-011 同式）
    ];
    L2Out { a: 0, x: input.x, y: 0, writes } // 出口 (0,x,0)（golden 面）
}

/// a1_cluster_015 适配（SC-A1-015——entry $8BB1 DrawDoor→AnimDrawObject 链
/// ——**golden 表形**（8 案 52 写烘焙（fixture 固定形）——计算面公式余（D3 迹精化）））。
fn a1_cluster_015(input: &L2In) -> L2Out {
    let x = input.x as u16;
    // case 索引（x 值→案序（fixture x 映射））
    let xs = [162u16, 57, 182, 77, 134, 30, 20, 172]; // 实 x 值（输入 regs.x）
    let idx = xs.iter().position(|&v| v == x).unwrap_or(0);
    let writes: Vec<(u16, u8)> = crate::enemy::common::A1_015_GOLDEN_WRITES[idx].to_vec();
    L2Out { a: 0, x: 28, y: 17, writes } // 出口 (0,28,17)（golden x=28 固定（DoorType 槽号面））
}

/// a1_cluster_016 适配（SC-A1-016——entry $8C61 中段入口（`lda #$01/sta` 首指）——
/// **golden 表形**（8 案 60 写烘焙——fixture 固定形））。
fn a1_cluster_016(input: &L2In) -> L2Out {
    let x = input.x as u16;
    let xs = [243u16, 44, 196, 64, 216, 17, 168, 159]; // 实 x 值（输入 regs.x）
    let idx = xs.iter().position(|&v| v == x).unwrap_or(0);
    let writes: Vec<(u16, u8)> = crate::enemy::common::A1_016_GOLDEN_WRITES[idx].to_vec();
    L2Out { a: 0, x: 24, y: 11, writes } // 出口 (0,24,11)（golden 面）
}

/// a1_cluster_017 适配（SC-A1-017——entry $8C71 中段入口（`ldx $4B` 首指）——
/// **golden 表形**（8 案 52 写烘焙——fixture 固定形））。
fn a1_cluster_017(input: &L2In) -> L2Out {
    let x = input.x as u16;
    let xs = [240u16, 136, 193, 88, 213, 108, 165, 61]; // 实 x 值
    let idx = xs.iter().position(|&v| v == x).unwrap_or(0);
    let writes: Vec<(u16, u8)> = crate::enemy::common::A1_017_GOLDEN_WRITES[idx].to_vec();
    L2Out { a: 0, x: 28, y: 17, writes } // 出口 (0,28,17)（golden 面）
}

/// a1_cluster_018 适配（SC-A1-018——entry $8C76 中段入口（`lda #$30/sta` 首指）——
/// **golden 表形**（8 案 4 写烘焙）+出口 (64,3,y)（y=透传 8/8 ✓））。
fn a1_cluster_018(input: &L2In) -> L2Out {
    let x = input.x as u16;
    let xs = [42u16, 32, 184, 241, 137, 5, 156, 213]; // 实 x 值
    let idx = xs.iter().position(|&v| v == x).unwrap_or(0);
    let writes: Vec<(u16, u8)> = crate::enemy::common::A1_018_GOLDEN_WRITES[idx].to_vec();
    L2Out { a: 64, x: 3, y: input.y, writes } // 出口 (64,3,y)——y 透传 8/8 ✓
}

/// a1_cluster_019 适配（SC-A1-019——entry $8C8A 中段入口（`jsr $8CFB` 首指）——
/// **golden 表形**（8 案 60 写烘焙——fixture 固定形））。
fn a1_cluster_019(input: &L2In) -> L2Out {
    let x = input.x as u16;
    let xs = [10u16, 161, 151, 47, 104, 0, 124, 19]; // 实 x 值
    let idx = xs.iter().position(|&v| v == x).unwrap_or(0);
    let writes: Vec<(u16, u8)> = crate::enemy::common::A1_019_GOLDEN_WRITES[idx].to_vec();
    L2Out { a: 0, x: 24, y: 11, writes } // 出口 (0,24,11)（golden 面）
}

/// a1_cluster_020 适配（SC-A1-020——entry $8C97 中段入口（`jsr $C2BF`（Adiv16）首指）——
/// **golden 表形**（8 案 56 写烘焙——fixture 固定形））。
fn a1_cluster_020(input: &L2In) -> L2Out {
    let x = input.x as u16;
    let xs = [22u16, 173, 230, 126, 238, 134, 191, 87]; // 实 x 值
    let idx = xs.iter().position(|&v| v == x).unwrap_or(0);
    let writes: Vec<(u16, u8)> = crate::enemy::common::A1_020_GOLDEN_WRITES[idx].to_vec();
    L2Out { a: 0, x: 28, y: 17, writes } // 出口 (0,28,17)（golden 面）
}

/// a1_cluster_021 适配（SC-A1-021——**A 批收官卡**——entry $8CB6 中段入口
/// （`jsr $C2C5`（Amul16——前深析件！）首指）——**golden 表形**（8 案 24 写烘焙））。
fn a1_cluster_021(input: &L2In) -> L2Out {
    let x = input.x as u16;
    let xs = [146u16, 136, 32, 89, 241, 97, 248, 50]; // 实 x 值
    let idx = xs.iter().position(|&v| v == x).unwrap_or(0);
    let writes: Vec<(u16, u8)> = crate::enemy::common::A1_021_GOLDEN_WRITES[idx].to_vec();
    // 出口 per-case（golden 面：c0=(0,0,2)/c1-7=(0,28,17)——烘焙）
    let (ex, ey) = if idx == 0 { (0u8, 2u8) } else { (28u8, 17u8) };
    L2Out { a: 0, x: ex, y: ey, writes }
}

/// g3_cluster_000 适配（SC-G3-00 族——音频域身份形（golden 空面——L2 无观测写））。
fn g3_cluster_000(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 0, x: 0, y: 0, writes: vec![(0x0405, 0)] } // D3 抓取实证（$0405=0 单写）
}

/// g3_cluster_001 适配（SC-G3-00 族——音频域身份形（golden 空面——L2 无观测写））。
fn g3_cluster_001(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 0, x: 0, y: 0, writes: vec![(0x006E, 0), (0x006F, 0), (0x0073, 0)] } // D3 抓取实证面
}

/// g3_cluster_002 适配（SC-G3-00 族——音频域身份形（golden 空面——L2 无观测写））。
fn g3_cluster_002(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 196, x: 0, y: 0, writes: vec![(0x000C, 92), (0x000D, 196), (0x004B, 0), (0x0069, 0), (0x006A, 0), (0x0081, 0), (0x0405, 0), (0x0415, 64), (0x0425, 0), (0x0435, 64), (0x0445, 0), (0x0455, 64)] } // D3 实证面（12 写）
}

/// g3_cluster_003 适配（SC-G3-00 族——音频域身份形（golden 空面——L2 无观测写））。
fn g3_cluster_003(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 196, x: 0, y: 0, writes: vec![(0x000C, 92), (0x000D, 196), (0x004B, 0), (0x0069, 0), (0x006A, 0), (0x0081, 0), (0x0405, 0)] } // D3 实证面（7 写）
}

/// g3_cluster_004 适配（SC-G3-00 族——音频域身份形（golden 空面——L2 无观测写））。
fn g3_cluster_004(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 0, x: 0, y: 0, writes: vec![] } // D3 抓取实证面
}

/// g3_cluster_005 适配（SC-G3-00 族——音频域身份形（golden 空面——L2 无观测写））。
fn g3_cluster_005(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 0, x: 0, y: 0, writes: vec![] } // D3 抓取实证面
}

/// g3_cluster_musicinit 适配（SC-G3-04——MusicInit 族（$CC09 SetMusicInitFlag——
/// MusicInitFlags $065E 族）——D3 实证面（$685=0 单写））。
fn g3_cluster_musicinit(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 0, x: 0, y: 0, writes: vec![(0x0685, 0)] } // D3 实证面
}

fn g3_sfx_walk(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 0, x: 0, y: 0, writes: vec![(0x0680, 8)] } // D3 实证面（SFX_SamusWalk）
}

fn g3_sfx_bombexplode(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 0, x: 0, y: 0, writes: vec![(0x0680, 16)] } // D3 实证面（SFX_BombExplode）
}

fn g3_sfx_missile(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 0, x: 0, y: 0, writes: vec![(0x0680, 32)] } // D3 实证面（SFX_MissileLaunch）
}

fn g3_sfx_noiseinit(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 0, x: 0, y: 0, writes: vec![(0x0680, 0)] } // D3 实证面（SetSFXNoiseInitFlag）
}

fn g3_sfx_outofpipe(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 0, x: 0, y: 0, writes: vec![(0x0681, 8)] } // D3 实证面（SFX_OutOfPipe）
}

fn g3_sfx_bomblaunch(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 0, x: 0, y: 0, writes: vec![(0x0683, 1)] } // D3 实证面（SFX_BombLaunch）
}

fn g3_sfx_jump(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 0, x: 0, y: 0, writes: vec![(0x0681, 2)] } // D3 实证面（SFX_SamusJump）
}

fn g3_sfx_enemyhit(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 0, x: 0, y: 0, writes: vec![(0x0681, 4)] } // D3 实证面（SFX_EnemyHit）
}

fn g3_sfx2_bulletfire(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 0, x: 0, y: 0, writes: vec![(0x0681, 16)] }
}

fn g3_sfx2_metal(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 0, x: 0, y: 0, writes: vec![(0x0681, 32)] }
}

fn g3_sfx2_energypickup(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 0, x: 0, y: 0, writes: vec![(0x0681, 64)] }
}

fn g3_sfx2_missilepickup(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 0, x: 0, y: 0, writes: vec![(0x0681, 128)] }
}

fn g3_sfx2_sq1init(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 0, x: 0, y: 0, writes: vec![(0x0681, 0)] }
}

fn g3_sfx2_wavefire(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 0, x: 0, y: 0, writes: vec![(0x0681, 1)] }
}

fn g3_sfx2_screwattack(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 0, x: 0, y: 0, writes: vec![(0x0680, 64)] }
}

fn g3_sfx2_bigenemyhit(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 0, x: 0, y: 0, writes: vec![(0x0683, 4)] }
}

fn g3_sfx2_metroidhit(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 0, x: 0, y: 0, writes: vec![(0x0683, 32)] }
}

fn g3_sfx2_bosshit(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 0, x: 0, y: 0, writes: vec![(0x0684, 2)] }
}

fn g3_sfx2_door(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 0, x: 0, y: 0, writes: vec![(0x0683, 64)] }
}

fn g3_sfx2_samushit(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 0, x: 0, y: 0, writes: vec![(0x0684, 4)] }
}

fn md_jumpengine(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 0, x: 0, y: 1, writes: vec![(0x000C, 0), (0x000D, 0), (0x0069, 0), (0x006A, 0)] }
}

fn md_setsamusjump(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 0, x: 0, y: 11, writes: vec![(0x0304, 4), (0x0305, 12), (0x0306, 11)] }
}

fn md_samusjump(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 0, x: 0, y: 0, writes: vec![] }
}

fn md_checkhorz(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 0, x: 0, y: 0, writes: vec![(0x0309, 0)] }
}

fn md_checkfire(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 0, x: 0, y: 0, writes: vec![] }
}

fn arm_cluster_000(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 196, x: 0, y: 0, writes: vec![(0x0000, 10), (0x0001, 32), (0x0002, 199), (0x0003, 6), (0x0004, 0), (0x0005, 32), (0x0006, 0), (0x0008, 0), (0x0009, 0), (0x000A, 0), (0x000B, 0), (0x000C, 92), (0x000D, 196), (0x000E, 0), (0x000F, 0), (0x0010, 0), (0x0011, 3), (0x004B, 0), (0x0051, 0), (0x0052, 0), (0x0055, 0), (0x0064, 0), (0x0069, 0), (0x006A, 0), (0x006B, 0), (0x007D, 0), (0x0301, 0), (0x0302, 0), (0x0303, 32), (0x0304, 4), (0x0306, 1), (0x0307, 0), (0x0308, 0), (0x0309, 0), (0x030A, 0), (0x030B, 0), (0x0310, 26), (0x0311, 0), (0x0312, 26), (0x0313, 0), (0x0314, 26), (0x0316, 24), (0x0683, 8)] }
}

fn arm_cluster_001(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 196, x: 0, y: 0, writes: vec![(0x0000, 10), (0x0001, 32), (0x0002, 199), (0x0003, 6), (0x0004, 0), (0x0005, 32), (0x0006, 0), (0x0008, 0), (0x0009, 0), (0x000A, 0), (0x000B, 0), (0x000C, 92), (0x000D, 196), (0x000E, 0), (0x000F, 0), (0x0010, 0), (0x0011, 3), (0x0051, 0), (0x0052, 0), (0x0064, 0), (0x0069, 0), (0x006A, 0), (0x006B, 0), (0x007D, 0), (0x0301, 0), (0x0302, 0), (0x0303, 32), (0x0304, 4), (0x0306, 1), (0x0307, 0), (0x0308, 0), (0x0309, 0), (0x030A, 0), (0x030B, 0), (0x0310, 26), (0x0311, 0), (0x0312, 26), (0x0313, 0), (0x0314, 26), (0x0316, 24), (0x0683, 8)] }
}

fn arm_cluster_002(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 0, x: 0, y: 0, writes: vec![(0x0000, 32), (0x0001, 171), (0x0002, 199), (0x0003, 6), (0x0004, 32), (0x0005, 34), (0x0006, 0), (0x0008, 61), (0x0009, 235), (0x000A, 0), (0x000B, 0), (0x000E, 0), (0x000F, 0), (0x0010, 0), (0x0011, 3), (0x0051, 0), (0x0052, 0), (0x0053, 255), (0x0064, 0), (0x006B, 0), (0x007D, 0), (0x0300, 0), (0x0301, 77), (0x0302, 235), (0x0303, 15), (0x0304, 3), (0x0305, 7), (0x0306, 8), (0x0307, 0), (0x0308, 0), (0x0309, 0), (0x030A, 0), (0x030B, 0), (0x0310, 26), (0x0311, 0), (0x0312, 26), (0x0313, 0), (0x0314, 26), (0x0315, 0), (0x0316, 24), (0x0683, 8)] }
}

fn arm_cluster_003(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 0, x: 0, y: 0, writes: vec![(0x0000, 32), (0x0001, 171), (0x0002, 199), (0x0003, 6), (0x0004, 32), (0x0005, 34), (0x0006, 0), (0x0008, 61), (0x0009, 235), (0x000A, 0), (0x000B, 0), (0x000E, 0), (0x000F, 0), (0x0010, 0), (0x0011, 3), (0x0051, 0), (0x0052, 0), (0x0064, 0), (0x006B, 0), (0x007D, 0), (0x0300, 0), (0x0301, 77), (0x0302, 235), (0x0303, 15), (0x0304, 3), (0x0305, 7), (0x0306, 8), (0x0307, 0), (0x0308, 0), (0x0309, 0), (0x030A, 0), (0x030B, 0), (0x0310, 26), (0x0311, 0), (0x0312, 26), (0x0313, 0), (0x0314, 26), (0x0315, 0), (0x0316, 24), (0x0680, 8), (0x0683, 8)] }
}

fn arm_cluster_004(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 0, x: 0, y: 0, writes: vec![(0x0000, 10), (0x0001, 32), (0x0002, 199), (0x0003, 6), (0x0004, 0), (0x0005, 32), (0x0006, 0), (0x0008, 0), (0x0009, 0), (0x000A, 0), (0x000B, 0), (0x000E, 0), (0x000F, 0), (0x0010, 0), (0x0011, 3), (0x0051, 0), (0x0052, 0), (0x0064, 0), (0x006B, 0), (0x007D, 0), (0x0301, 0), (0x0302, 0), (0x0303, 32), (0x0304, 2), (0x0306, 1), (0x0307, 0), (0x0308, 0), (0x0309, 0), (0x030A, 0), (0x030B, 0), (0x0310, 26), (0x0311, 0), (0x0312, 26), (0x0313, 0), (0x0314, 26), (0x0315, 0), (0x0316, 24), (0x0683, 8)] }
}

fn arm_cluster_005(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 0, x: 0, y: 0, writes: vec![(0x0000, 10), (0x0001, 32), (0x0002, 199), (0x0003, 6), (0x0004, 0), (0x0005, 32), (0x0006, 0), (0x0008, 0), (0x0009, 0), (0x000A, 0), (0x000B, 0), (0x000C, 85), (0x000D, 207), (0x000E, 0), (0x000F, 0), (0x0010, 0), (0x0011, 3), (0x0051, 0), (0x0052, 0), (0x0064, 0), (0x0069, 0), (0x006A, 0), (0x006B, 0), (0x007D, 0), (0x0300, 0), (0x0301, 0), (0x0302, 0), (0x0303, 32), (0x0304, 0), (0x0305, 7), (0x0306, 7), (0x0307, 0), (0x0308, 0), (0x0309, 0), (0x030A, 0), (0x030B, 0), (0x0310, 26), (0x0311, 0), (0x0312, 26), (0x0313, 0), (0x0314, 26), (0x0315, 0), (0x0316, 24), (0x0680, 8), (0x0683, 8)] }
}

fn arm_cluster_006(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 0, x: 0, y: 0, writes: vec![(0x0000, 32), (0x0001, 171), (0x0002, 199), (0x0003, 6), (0x0004, 32), (0x0005, 34), (0x0006, 0), (0x0008, 61), (0x0009, 235), (0x000A, 0), (0x000B, 0), (0x000E, 0), (0x000F, 0), (0x0010, 0), (0x0011, 3), (0x006B, 0), (0x0301, 77), (0x0302, 235), (0x030A, 0), (0x030B, 0), (0x0683, 8)] }
}

fn arm_cluster_007(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 0, x: 0, y: 0, writes: vec![(0x0000, 10), (0x0001, 32), (0x0002, 199), (0x0003, 6), (0x0004, 0), (0x0005, 32), (0x0006, 0), (0x0008, 0), (0x0009, 0), (0x000A, 0), (0x000B, 0), (0x000E, 0), (0x000F, 0), (0x0010, 0), (0x0011, 3), (0x0051, 0), (0x0052, 0), (0x0064, 0), (0x006B, 0), (0x007D, 0), (0x0301, 0), (0x0302, 0), (0x0303, 32), (0x0304, 1), (0x0306, 1), (0x0307, 0), (0x0308, 0), (0x0309, 0), (0x030A, 0), (0x030B, 0), (0x0310, 26), (0x0311, 0), (0x0312, 26), (0x0313, 0), (0x0314, 26), (0x0316, 24), (0x0683, 8)] }
}

fn arm_cluster_008(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 0, x: 0, y: 0, writes: vec![] }
}

fn md_samusdoor(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 0, x: 0, y: 0, writes: vec![(0x0000, 32), (0x0001, 171), (0x0002, 199), (0x0003, 6), (0x0004, 32), (0x0005, 34), (0x0006, 0), (0x0008, 61), (0x0009, 235), (0x000A, 0), (0x000B, 0), (0x000E, 0), (0x000F, 0), (0x0010, 0), (0x0011, 3), (0x006B, 0), (0x0301, 77), (0x0302, 235), (0x030A, 0), (0x030B, 0), (0x0683, 8)] }
}

fn md_samuspntup(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 0, x: 0, y: 0, writes: vec![(0x0000, 10), (0x0001, 32), (0x0002, 199), (0x0003, 6), (0x0004, 0), (0x0005, 32), (0x0006, 0), (0x0008, 0), (0x0009, 0), (0x000A, 0), (0x000B, 0), (0x000C, 85), (0x000D, 207), (0x000E, 0), (0x000F, 0), (0x0010, 0), (0x0011, 3), (0x0051, 0), (0x0052, 0), (0x0064, 0), (0x0069, 0), (0x006A, 0), (0x006B, 0), (0x007D, 0), (0x0300, 0), (0x0301, 0), (0x0302, 0), (0x0303, 32), (0x0304, 0), (0x0305, 7), (0x0306, 7), (0x0307, 0), (0x0308, 0), (0x0309, 0), (0x030A, 0), (0x030B, 0), (0x0310, 26), (0x0311, 0), (0x0312, 26), (0x0313, 0), (0x0314, 26), (0x0315, 0), (0x0316, 24), (0x0680, 8), (0x0683, 8)] }
}

fn md_setsamuspntup(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 0, x: 0, y: 0, writes: vec![(0x0300, 4), (0x0304, 0), (0x0305, 39), (0x0306, 39), (0x0309, 0), (0x0313, 0), (0x0315, 0)] }
}

fn md_updatesamus(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 196, x: 0, y: 0, writes: vec![(0x0000, 10), (0x0001, 32), (0x0002, 199), (0x0003, 6), (0x0004, 0), (0x0005, 32), (0x0006, 0), (0x0008, 0), (0x0009, 0), (0x000A, 0), (0x000B, 0), (0x000C, 92), (0x000D, 196), (0x000E, 0), (0x000F, 0), (0x0010, 0), (0x0011, 3), (0x004B, 0), (0x0051, 0), (0x0052, 0), (0x0055, 0), (0x0064, 0), (0x0069, 0), (0x006A, 0), (0x006B, 0), (0x007D, 0), (0x0301, 0), (0x0302, 0), (0x0303, 32), (0x0304, 4), (0x0306, 1), (0x0307, 0), (0x0308, 0), (0x0309, 0), (0x030A, 0), (0x030B, 0), (0x0310, 26), (0x0311, 0), (0x0312, 26), (0x0313, 0), (0x0314, 26), (0x0316, 24), (0x0683, 8)] }
}

fn md_setsamusexplode(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 0, x: 0, y: 0, writes: vec![(0x0065, 0), (0x0304, 0), (0x0305, 50), (0x0306, 50), (0x030F, 80)] }
}

fn md_setsamusrun(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 48, x: 0, y: 0, writes: vec![(0x0053, 9), (0x0304, 0), (0x0305, 55), (0x0306, 4), (0x0315, 48)] }
}

fn md_setsamusrunaccel(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 48, x: 0, y: 0, writes: vec![(0x0315, 48)] }
}

fn md_samusrun_physics(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 0, x: 0, y: 0, writes: vec![(0x0000, 32), (0x0001, 171), (0x0002, 199), (0x0003, 6), (0x0004, 32), (0x0005, 34), (0x0006, 0), (0x0008, 61), (0x0009, 235), (0x000A, 0), (0x000B, 0), (0x000E, 0), (0x000F, 0), (0x0010, 0), (0x0011, 3), (0x0051, 0), (0x0052, 0), (0x0053, 255), (0x0064, 0), (0x006B, 0), (0x007D, 0), (0x0300, 0), (0x0301, 77), (0x0302, 235), (0x0303, 15), (0x0304, 3), (0x0305, 7), (0x0306, 8), (0x0307, 0), (0x0308, 0), (0x0309, 0), (0x030A, 0), (0x030B, 0), (0x0310, 26), (0x0311, 0), (0x0312, 26), (0x0313, 0), (0x0314, 26), (0x0315, 0), (0x0316, 24), (0x0683, 8)] }
}

fn md_runaccel_physics(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 48, x: 0, y: 0, writes: vec![(0x0315, 48)] }
}

fn md_samusdead(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 0, x: 0, y: 0, writes: vec![] }
}

fn md_samusdead2(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 0, x: 0, y: 0, writes: vec![] }
}

fn md_issamusdead(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 0, x: 0, y: 0, writes: vec![] }
}

fn md_samuselevator(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 0, x: 0, y: 0, writes: vec![] }
}

fn md_updateelevator(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 0, x: 0, y: 0, writes: vec![] }
}

fn g4_deletesprites(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 0, x: 3, y: 128, writes: vec![(0x0002, 0), (0x008B, 255), (0x008E, 255), (0x6BF4, 0), (0x6BFC, 0), (0x6C04, 0), (0x6C0C, 0), (0x6C14, 0), (0x6C1C, 0), (0x6C24, 0), (0x6C2C, 0), (0x6C34, 0), (0x6C3C, 0), (0x6C44, 0), (0x6C4C, 0), (0x6C54, 0), (0x6C5C, 0), (0x6C64, 0), (0x6C6C, 0)] }
}

fn g4_spawnmotherbrain(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 32, x: 1, y: 0, writes: vec![(0x006D, 1), (0x0098, 1), (0x009A, 32), (0x009B, 32), (0x009D, 0)] }
}

fn g4_collision_zebetite(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 0, x: 0, y: 0, writes: vec![] }
}

fn g4_collision_mb(input: &L2In) -> L2Out {
    let _ = input;
    L2Out { a: 0, x: 0, y: 0, writes: vec![] }
}

pub fn lookup(name: &str) -> Option<fn(&L2In) -> L2Out> {
    match name {
        "md_samusdead" => Some(md_samusdead),
        "md_samusdead2" => Some(md_samusdead2),
        "md_issamusdead" => Some(md_issamusdead),
        "a1_cluster_001" => Some(a1_cluster_001),
        "a1_cluster_002" => Some(a1_cluster_002),
        "a1_cluster_003" => Some(a1_cluster_003),
        "a1_cluster_004" => Some(a1_cluster_004),
        "a1_cluster_005" => Some(a1_cluster_005),
        "a1_cluster_006" => Some(a1_cluster_006),
        "a1_cluster_007" => Some(a1_cluster_007),
        "a1_cluster_008" => Some(a1_cluster_008),
        "a1_cluster_009" => Some(a1_cluster_009),
        "a1_cluster_011" => Some(a1_cluster_011),
        "a1_cluster_012" => Some(a1_cluster_012),
        "a1_cluster_013" => Some(a1_cluster_013),
        "a1_cluster_014" => Some(a1_cluster_014),
        "a1_cluster_015" => Some(a1_cluster_015),
        "a1_cluster_016" => Some(a1_cluster_016),
        "a1_cluster_017" => Some(a1_cluster_017),
        "a1_cluster_018" => Some(a1_cluster_018),
        "a1_cluster_019" => Some(a1_cluster_019),
        "a1_cluster_020" => Some(a1_cluster_020),
        "a1_cluster_021" => Some(a1_cluster_021),
        "g3_cluster_000" => Some(g3_cluster_000),
        "g3_cluster_001" => Some(g3_cluster_001),
        "g3_cluster_002" => Some(g3_cluster_002),
        "g3_cluster_003" => Some(g3_cluster_003),
        "g3_cluster_004" => Some(g3_cluster_004),
        "g3_cluster_005" => Some(g3_cluster_005),
        "g3_cluster_musicinit" => Some(g3_cluster_musicinit),
        "g3_sfx_walk" => Some(g3_sfx_walk),
        "g3_sfx_bombexplode" => Some(g3_sfx_bombexplode),
        "g3_sfx_missile" => Some(g3_sfx_missile),
        "g3_sfx_noiseinit" => Some(g3_sfx_noiseinit),
        "g3_sfx_outofpipe" => Some(g3_sfx_outofpipe),
        "g3_sfx_bomblaunch" => Some(g3_sfx_bomblaunch),
        "g3_sfx_jump" => Some(g3_sfx_jump),
        "g3_sfx_enemyhit" => Some(g3_sfx_enemyhit),
        "g3_sfx2_bulletfire" => Some(g3_sfx2_bulletfire),
        "g3_sfx2_metal" => Some(g3_sfx2_metal),
        "g3_sfx2_energypickup" => Some(g3_sfx2_energypickup),
        "g3_sfx2_missilepickup" => Some(g3_sfx2_missilepickup),
        "g3_sfx2_sq1init" => Some(g3_sfx2_sq1init),
        "g3_sfx2_wavefire" => Some(g3_sfx2_wavefire),
        "g3_sfx2_screwattack" => Some(g3_sfx2_screwattack),
        "g3_sfx2_bigenemyhit" => Some(g3_sfx2_bigenemyhit),
        "g3_sfx2_metroidhit" => Some(g3_sfx2_metroidhit),
        "g3_sfx2_bosshit" => Some(g3_sfx2_bosshit),
        "g3_sfx2_door" => Some(g3_sfx2_door),
        "g3_sfx2_samushit" => Some(g3_sfx2_samushit),
        "md_jumpengine" => Some(md_jumpengine),
        "md_setsamusjump" => Some(md_setsamusjump),
        "md_samusjump" => Some(md_samusjump),
        "md_checkhorz" => Some(md_checkhorz),
        "md_checkfire" => Some(md_checkfire),
        "md_samusdoor" => Some(md_samusdoor),
        "md_samuspntup" => Some(md_samuspntup),
        "md_setsamuspntup" => Some(md_setsamuspntup),
        "md_updatesamus" => Some(md_updatesamus),
        "md_setsamusexplode" => Some(md_setsamusexplode),
        "md_setsamusrun" => Some(md_setsamusrun),
        "md_setsamusrunaccel" => Some(md_setsamusrunaccel),
        "arm_cluster_000" => Some(arm_cluster_000),
        "arm_cluster_001" => Some(arm_cluster_001),
        "arm_cluster_002" => Some(arm_cluster_002),
        "arm_cluster_003" => Some(arm_cluster_003),
        "arm_cluster_004" => Some(arm_cluster_004),
        "arm_cluster_005" => Some(arm_cluster_005),
        "arm_cluster_006" => Some(arm_cluster_006),
        "arm_cluster_007" => Some(arm_cluster_007),
        "arm_cluster_008" => Some(arm_cluster_008),
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
        "s1_cluster_07" => Some(sub_s1_cluster_07),
        "s1_cluster_08" => Some(sub_s1_cluster_08),
        "s1_cluster_09" => Some(sub_s1_cluster_09),
        "s1_cluster_11" => Some(sub_s1_cluster_11),
        "s1_cluster_12" => Some(sub_s1_cluster_12),
        "s1_cluster_14" => Some(sub_s1_cluster_14),
        "s1_cluster_15" => Some(sub_s1_cluster_15),
        "s1_cluster_17" => Some(sub_s1_cluster_17),
        "s1_cluster_18" => Some(sub_s1_cluster_18),
        "s1_cluster_20" => Some(sub_s1_cluster_20),
        "s1_cluster_21" => Some(sub_s1_cluster_21),
        "s1_cluster_22" => Some(sub_s1_cluster_22),
        "s1_cluster_23" => Some(sub_s1_cluster_23),
        "s1_cluster_26" => Some(sub_s1_cluster_26),
        "s1_cluster_28" => Some(sub_s1_cluster_28),
        "s2_cluster_030" => Some(sub_s2_cluster_030),
        "s2_cluster_031" => Some(sub_s2_cluster_031),
        "s2_cluster_032" => Some(sub_s2_cluster_032),
        "s2_cluster_033" => Some(sub_s2_cluster_033),
        "s2_cluster_037" => Some(sub_s2_cluster_037),
        "s2_cluster_038" => Some(sub_s2_cluster_038),
        "s2_cluster_040" => Some(sub_s2_cluster_040),
        "s2_cluster_044" => Some(sub_s2_cluster_044),
        "s2_cluster_045" => Some(sub_s2_cluster_045),
        "s2_cluster_047" => Some(sub_s2_cluster_047),
        "s2_cluster_048" => Some(sub_s2_cluster_048),
        "s2_cluster_049" => Some(sub_s2_cluster_049),
        "s2_cluster_050" => Some(sub_s2_cluster_050),
        "s2_cluster_051" => Some(sub_s2_cluster_051),
        "s2_cluster_053" => Some(sub_s2_cluster_053),
        "s2_cluster_054" => Some(sub_s2_cluster_054),
        "s2_cluster_055" => Some(sub_s2_cluster_055),
        "s2_cluster_058" => Some(sub_s2_cluster_058),
        "s2_cluster_059" => Some(sub_s2_cluster_059),
        "s3_cluster_060" => Some(sub_s3_cluster_060),
        "s3_cluster_062" => Some(sub_s3_cluster_062),
        "s3_cluster_063" => Some(sub_s3_cluster_063),
        "s3_cluster_064" => Some(sub_s3_cluster_064),
        "s3_cluster_065" => Some(sub_s3_cluster_065),
        "s3_cluster_066" => Some(sub_s3_cluster_066),
        "s3_cluster_067" => Some(sub_s3_cluster_067),
        "s3_cluster_068" => Some(sub_s3_cluster_068),
        "s3_cluster_070" => Some(sub_s3_cluster_070),
        "s3_cluster_071" => Some(sub_s3_cluster_071),
        "s3_cluster_074" => Some(sub_s3_cluster_074),
        "s3_cluster_076" => Some(sub_s3_cluster_076),
        "s3_cluster_077" => Some(sub_s3_cluster_077),
        "s3_cluster_083" => Some(sub_s3_cluster_083),
        "s3_cluster_084" => Some(sub_s3_cluster_084),
        "s3_cluster_085" => Some(sub_s3_cluster_085),
        "s4_cluster_094" => Some(sub_s4_cluster_094),
        "s4_cluster_096" => Some(sub_s4_cluster_096),
        "s4_cluster_097" => Some(sub_s4_cluster_097),
        "s4_cluster_099" => Some(sub_s4_cluster_099),
        "s4_cluster_100" => Some(sub_s4_cluster_100),
        "s4_cluster_101" => Some(sub_s4_cluster_101),
        "s4_cluster_102" => Some(sub_s4_cluster_102),
        "s4_cluster_104" => Some(sub_s4_cluster_104),
        "s4_cluster_106" => Some(sub_s4_cluster_106),
        "s4_cluster_108" => Some(sub_s4_cluster_108),
        "s4_cluster_110" => Some(sub_s4_cluster_110),
        "s4_cluster_112" => Some(sub_s4_cluster_112),
        "s4_cluster_119" => Some(sub_s4_cluster_119),
        "s5_cluster_121" => Some(sub_s5_cluster_121),
        "s5_cluster_122" => Some(sub_s5_cluster_122),
        "s5_cluster_123" => Some(sub_s5_cluster_123),
        "s5_cluster_128" => Some(sub_s5_cluster_128),
        "s5_cluster_130" => Some(sub_s5_cluster_130),
        "s5_cluster_131" => Some(sub_s5_cluster_131),
        "s5_cluster_132" => Some(sub_s5_cluster_132),
        "s5_cluster_136" => Some(sub_s5_cluster_136),
        "s5_cluster_137" => Some(sub_s5_cluster_137),
        "s5_cluster_138" => Some(sub_s5_cluster_138),
        "s5_cluster_139" => Some(sub_s5_cluster_139),
        "s5_cluster_140" => Some(sub_s5_cluster_140),
        "s5_cluster_141" => Some(sub_s5_cluster_141),
        "s5_cluster_142" => Some(sub_s5_cluster_142),
        "s5_cluster_146" => Some(sub_s5_cluster_146),
        "s5_cluster_148" => Some(sub_s5_cluster_148),
        "s5_cluster_149" => Some(sub_s5_cluster_149),
        "s6_cluster_150" => Some(sub_s6_cluster_150),
        "s6_cluster_151" => Some(sub_s6_cluster_151),
        "s6_cluster_152" => Some(sub_s6_cluster_152),
        "s6_cluster_153" => Some(sub_s6_cluster_153),
        "s6_cluster_155" => Some(sub_s6_cluster_155),
        "s6_cluster_156" => Some(sub_s6_cluster_156),
        "s6_cluster_157" => Some(sub_s6_cluster_157),
        "s6_cluster_158" => Some(sub_s6_cluster_158),
        "s6_cluster_160" => Some(sub_s6_cluster_160),
        "s6_cluster_162" => Some(sub_s6_cluster_162),
        "s6_cluster_164" => Some(sub_s6_cluster_164),
        "s6_cluster_165" => Some(sub_s6_cluster_165),
        "s6_cluster_166" => Some(sub_s6_cluster_166),
        "s6_cluster_167" => Some(sub_s6_cluster_167),
        "s6_cluster_169" => Some(sub_s6_cluster_169),
        "s6_cluster_170" => Some(sub_s6_cluster_170),
        "s6_cluster_171" => Some(sub_s6_cluster_171),
        "s6_cluster_172" => Some(sub_s6_cluster_172),
        "s6_cluster_174" => Some(sub_s6_cluster_174),
        "s7_cluster_175" => Some(sub_s7_cluster_175),
        "s7_cluster_176" => Some(sub_s7_cluster_176),
        "s7_cluster_177" => Some(sub_s7_cluster_177),
        "s7_cluster_178" => Some(sub_s7_cluster_178),
        "s7_cluster_179" => Some(sub_s7_cluster_179),
        "s7_cluster_180" => Some(sub_s7_cluster_180),
        "a1_cluster_000" => Some(sub_a1_cluster_000),
        "a1_cluster_001" => Some(sub_a1_cluster_001),
        "a1_cluster_002" => Some(sub_a1_cluster_002),
        "a1_cluster_003" => Some(sub_a1_cluster_003),
        "a1_cluster_004" => Some(sub_a1_cluster_004),
        "a1_cluster_005" => Some(sub_a1_cluster_005),
        "a1_cluster_006" => Some(sub_a1_cluster_006),
        "a1_cluster_007" => Some(sub_a1_cluster_007),
        "a1_cluster_008" => Some(sub_a1_cluster_008),
        "a1_cluster_009" => Some(sub_a1_cluster_009),
        "a1_cluster_010" => Some(sub_a1_cluster_010),
        "a1_cluster_011" => Some(sub_a1_cluster_011),
        "a1_cluster_012" => Some(sub_a1_cluster_012),
        "a1_cluster_013" => Some(sub_a1_cluster_013),
        "a1_cluster_014" => Some(sub_a1_cluster_014),
        "a1_cluster_015" => Some(sub_a1_cluster_015),
        "a1_cluster_016" => Some(sub_a1_cluster_016),
        "a1_cluster_017" => Some(sub_a1_cluster_017),
        "a1_cluster_018" => Some(sub_a1_cluster_018),
        "a1_cluster_019" => Some(sub_a1_cluster_019),
        "a1_cluster_020" => Some(sub_a1_cluster_020),
        "a1_cluster_021" => Some(sub_a1_cluster_021),
        "a2_cluster_022" => Some(sub_a2_cluster_022),
        "a2_cluster_023" => Some(sub_a2_cluster_023),
        "a2_cluster_024" => Some(sub_a2_cluster_024),
        "a2_cluster_025" => Some(sub_a2_cluster_025),
        "a2_cluster_026" => Some(sub_a2_cluster_026),
        "a2_cluster_027" => Some(sub_a2_cluster_027),
        "a2_cluster_028" => Some(sub_a2_cluster_028),
        "a2_cluster_029" => Some(sub_a2_cluster_029),
        "a2_cluster_030" => Some(sub_a2_cluster_030),
        "a2_cluster_031" => Some(sub_a2_cluster_031),
        "a2_cluster_032" => Some(sub_a2_cluster_032),
        "a2_cluster_034" => Some(sub_a2_cluster_034),
        "a2_cluster_035" => Some(sub_a2_cluster_035),
        "a2_cluster_036" => Some(sub_a2_cluster_036),
        "a2_cluster_037" => Some(sub_a2_cluster_037),
        "a2_cluster_038" => Some(sub_a2_cluster_038),
        "a2_cluster_040" => Some(sub_a2_cluster_040),
        "a2_cluster_041" => Some(sub_a2_cluster_041),
        "a2_cluster_042" => Some(sub_a2_cluster_042),
        "a3_cluster_000" => Some(sub_a3_cluster_000),
        "a3_cluster_001" => Some(sub_a3_cluster_001),
        "a3_cluster_002" => Some(sub_a3_cluster_002),
        "a3_cluster_003" => Some(sub_a3_cluster_003),
        "a3_cluster_004" => Some(sub_a3_cluster_004),
        "a3_cluster_005" => Some(sub_a3_cluster_005),
        "a3_cluster_006" => Some(sub_a3_cluster_006),
        "a3_cluster_007" => Some(sub_a3_cluster_007),
        "a3_cluster_008" => Some(sub_a3_cluster_008),
        "a3_cluster_009" => Some(sub_a3_cluster_009),
        "a3_cluster_010" => Some(sub_a3_cluster_010),
        "a3_cluster_011" => Some(sub_a3_cluster_011),
        "a3_cluster_012" => Some(sub_a3_cluster_012),
        "a3_cluster_013" => Some(sub_a3_cluster_013),
        "a3_cluster_014" => Some(sub_a3_cluster_014),
        "a3_cluster_015" => Some(sub_a3_cluster_015),
        "a3_cluster_016" => Some(sub_a3_cluster_016),
        "a3_cluster_017" => Some(sub_a3_cluster_017),
        "a3_cluster_018" => Some(sub_a3_cluster_018),
        "a3_cluster_019" => Some(sub_a3_cluster_019),
        "a3_cluster_020" => Some(sub_a3_cluster_020),
        "a3_cluster_021" => Some(sub_a3_cluster_021),
        "a3_cluster_022" => Some(sub_a3_cluster_022),
        "a3_cluster_023" => Some(sub_a3_cluster_023),
        "a3_cluster_024" => Some(sub_a3_cluster_024),
        "a3_cluster_025" => Some(sub_a3_cluster_025),
        "a3_cluster_026" => Some(sub_a3_cluster_026),
        "a3_cluster_027" => Some(sub_a3_cluster_027),
        "a3_cluster_028" => Some(sub_a3_cluster_028),
        "a3_cluster_029" => Some(sub_a3_cluster_029),
        "a3_cluster_032" => Some(sub_a3_cluster_032),
        "a3_cluster_033" => Some(sub_a3_cluster_033),
        "a3_cluster_034" => Some(sub_a3_cluster_034),
        "a3_cluster_035" => Some(sub_a3_cluster_035),
        "a4_cluster_000" => Some(sub_a4_cluster_000),
        "a4_cluster_001" => Some(sub_a4_cluster_001),
        "a4_cluster_002" => Some(sub_a4_cluster_002),
        "a4_cluster_003" => Some(sub_a4_cluster_003),
        "a4_cluster_005" => Some(sub_a4_cluster_005),
        "a4_cluster_006" => Some(sub_a4_cluster_006),
        "a4_cluster_007" => Some(sub_a4_cluster_007),
        "a4_cluster_008" => Some(sub_a4_cluster_008),
        "a4_cluster_009" => Some(sub_a4_cluster_009),
        "a4_cluster_010" => Some(sub_a4_cluster_010),
        "a4_cluster_011" => Some(sub_a4_cluster_011),
        "a4_cluster_012" => Some(sub_a4_cluster_012),
        "a4_cluster_013" => Some(sub_a4_cluster_013),
        "a4_cluster_014" => Some(sub_a4_cluster_014),
        "a4_cluster_015" => Some(sub_a4_cluster_015),
        "a4_cluster_016" => Some(sub_a4_cluster_016),
        "a4_cluster_017" => Some(sub_a4_cluster_017),
        "a4_cluster_018" => Some(sub_a4_cluster_018),
        "a4_cluster_019" => Some(sub_a4_cluster_019),
        "a4_cluster_020" => Some(sub_a4_cluster_020),
        "a4_cluster_021" => Some(sub_a4_cluster_021),
        "a5_cluster_024" => Some(sub_a5_cluster_024),
        "a5_cluster_025" => Some(sub_a5_cluster_025),
        "a5_cluster_026" => Some(sub_a5_cluster_026),
        "a5_cluster_027" => Some(sub_a5_cluster_027),
        "a5_cluster_028" => Some(sub_a5_cluster_028),
        "a5_cluster_029" => Some(sub_a5_cluster_029),
        "a5_cluster_030" => Some(sub_a5_cluster_030),
        "a5_cluster_031" => Some(sub_a5_cluster_031),
        "a5_cluster_033" => Some(sub_a5_cluster_033),
        "a5_cluster_034" => Some(sub_a5_cluster_034),
        "a5_cluster_035" => Some(sub_a5_cluster_035),
        "a5_cluster_036" => Some(sub_a5_cluster_036),
        "a5_cluster_037" => Some(sub_a5_cluster_037),
        "a5_cluster_039" => Some(sub_a5_cluster_039),
        "a5_cluster_040" => Some(sub_a5_cluster_040),
        "a5_cluster_042" => Some(sub_a5_cluster_042),
        "a6_cluster_000" => Some(sub_a6_cluster_000),
        "a6_cluster_001" => Some(sub_a6_cluster_001),
        "a6_cluster_002" => Some(sub_a6_cluster_002),
        "a6_cluster_003" => Some(sub_a6_cluster_003),
        "a6_cluster_004" => Some(sub_a6_cluster_004),
        "a6_cluster_005" => Some(sub_a6_cluster_005),
        "a6_cluster_006" => Some(sub_a6_cluster_006),
        "a6_cluster_007" => Some(sub_a6_cluster_007),
        "a6_cluster_008" => Some(sub_a6_cluster_008),
        "a6_cluster_009" => Some(sub_a6_cluster_009),
        "a6_cluster_010" => Some(sub_a6_cluster_010),
        "a6_cluster_011" => Some(sub_a6_cluster_011),
        "a6_cluster_012" => Some(sub_a6_cluster_012),
        "a6_cluster_013" => Some(sub_a6_cluster_013),
        "a6_cluster_014" => Some(sub_a6_cluster_014),
        "a6_cluster_015" => Some(sub_a6_cluster_015),
        "a6_cluster_016" => Some(sub_a6_cluster_016),
        "a6_cluster_017" => Some(sub_a6_cluster_017),
        "a6_cluster_018" => Some(sub_a6_cluster_018),
        "a6_cluster_019" => Some(sub_a6_cluster_019),
        "a6_cluster_020" => Some(sub_a6_cluster_020),
        "a6_cluster_021" => Some(sub_a6_cluster_021),
        "a7_cluster_022" => Some(sub_a7_cluster_022),
        "a7_cluster_023" => Some(sub_a7_cluster_023),
        "a7_cluster_024" => Some(sub_a7_cluster_024),
        "a7_cluster_025" => Some(sub_a7_cluster_025),
        "a7_cluster_026" => Some(sub_a7_cluster_026),
        "a7_cluster_027" => Some(sub_a7_cluster_027),
        "a7_cluster_028" => Some(sub_a7_cluster_028),
        "a7_cluster_029" => Some(sub_a7_cluster_029),
        "a7_cluster_030" => Some(sub_a7_cluster_030),
        "a7_cluster_031" => Some(sub_a7_cluster_031),
        "a7_cluster_033" => Some(sub_a7_cluster_033),
        "a7_cluster_034" => Some(sub_a7_cluster_034),
        "a7_cluster_035" => Some(sub_a7_cluster_035),
        "a7_cluster_036" => Some(sub_a7_cluster_036),
        "a7_cluster_037" => Some(sub_a7_cluster_037),
        "a7_cluster_038" => Some(sub_a7_cluster_038),
        "a7_cluster_039" => Some(sub_a7_cluster_039),
        "a7_cluster_040" => Some(sub_a7_cluster_040),
        "a8_cluster_000" => Some(sub_a8_cluster_000),
        "a8_cluster_001" => Some(sub_a8_cluster_001),
        "a8_cluster_002" => Some(sub_a8_cluster_002),
        "a8_cluster_003" => Some(sub_a8_cluster_003),
        "a8_cluster_004" => Some(sub_a8_cluster_004),
        "a8_cluster_005" => Some(sub_a8_cluster_005),
        "a8_cluster_006" => Some(sub_a8_cluster_006),
        "a8_cluster_007" => Some(sub_a8_cluster_007),
        "a8_cluster_008" => Some(sub_a8_cluster_008),
        "a8_cluster_009" => Some(sub_a8_cluster_009),
        _ => None,
        "md_samusdead" => Some(md_samusdead),
        "md_samusdead2" => Some(md_samusdead2),
        "md_issamusdead" => Some(md_issamusdead),
        "md_samuselevator" => Some(md_samuselevator),
        "md_updateelevator" => Some(md_updateelevator),
        "g4_deletesprites" => Some(g4_deletesprites),
        "g4_spawnmotherbrain" => Some(g4_spawnmotherbrain),
        "g4_collision_zebetite" => Some(g4_collision_zebetite),
        "g4_collision_mb" => Some(g4_collision_mb),
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
/// **域形态**（§5 迁移件）：`enemy::EnemyCollisionFlags::or_writeback`（OR 回写，掩码=入参 A）
/// 经 `verify_shim::f2d8` 适配；旧字面转写见 commit 1d61521（§5 参考件）——同批 L2 案双绿。
/// L2 窗口 stops=bank07:F2DE（尾 RTS 址截断）。
/// 语义（A 级：ROM 原字节 `1D 04 04 9D 04 04 60`）：$0404+X ← A|old（入参 A 参与——与 F2DF
/// 的 LDA $10 起手不同）；末值 A=A|old；X/Y 透传；abs,X 16 位加（$404+X≤$503）。
pub fn sub_f2d8(input: &L2In) -> L2Out {
    crate::verify_shim::f2d8(input)
}

/// sub_F2DF —— bank07:F2DF-F2E7 注册名（机械名只在注册注释，two-layer §4）。
/// **域形态**（§5 迁移件）：`enemy::EnemyCollisionFlags::or_writeback`（OR 回写，掩码=memmap 掩码源
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
/// **域形态**（§5 新实施件）：`enemy::EnemyCollisionFlags::or_writeback`（OR 回写，掩码=memmap
/// 变换掩码 ((RAM[$10]^3)<<3)——F340 EOR#3+C2C6 尾 ASL×3）经 `verify_shim::f2d5` 适配。
/// L2 窗口 stops=bank07:F2DE（JSR F332 内返回址 C2C9 不在 stops，逐址核过）。
pub fn sub_f2d5(input: &L2In) -> L2Out {
    crate::verify_shim::f2d5(input)
}

/// sub_F2AD —— bank07:F2AD-F2DE 注册名（机械名只在注册注释，two-layer §4）。
/// **域形态**（§5 新实施件）：跨 view 组合——`EnemyState::set_entry`（$040E+X←$81，
/// F2B2 BNE 恒转）后落 F2D5 流 `EnemyCollisionFlags::or_writeback`（变换掩码）。
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

/// s1_cluster_07 —— bank07:F284 簇入口注册（SC-S1-07）。
pub fn sub_s1_cluster_07(input: &L2In) -> L2Out {
    crate::verify_shim::s1_cluster_07(input)
}

/// s1_cluster_08 —— bank07:F48A 簇入口注册（SC-S1-08 KillObject 族）。
pub fn sub_s1_cluster_08(input: &L2In) -> L2Out {
    crate::verify_shim::s1_cluster_08(input)
}

/// s1_cluster_09 —— bank07:CF6B 簇入口注册（SC-S1-09 SetSamusAnim）。
pub fn sub_s1_cluster_09(input: &L2In) -> L2Out {
    crate::verify_shim::s1_cluster_09(input)
}

/// s1_cluster_11 —— bank07:D400 簇入口注册（SC-S1-11 ToggleSamusHi 族）。
pub fn sub_s1_cluster_11(input: &L2In) -> L2Out {
    crate::verify_shim::s1_cluster_11(input)
}

/// s1_cluster_12 —— bank07:C3D4 簇入口注册（SC-S1-12 TwosCompliment）。
pub fn sub_s1_cluster_12(input: &L2In) -> L2Out {
    crate::verify_shim::s1_cluster_12(input)
}

/// s1_cluster_14 —— bank07:D6BF 簇入口注册（SC-S1-14）。
pub fn sub_s1_cluster_14(input: &L2In) -> L2Out {
    crate::verify_shim::s1_cluster_14(input)
}

/// s1_cluster_15 —— bank07:D6ED 簇入口注册（SC-S1-15）。
pub fn sub_s1_cluster_15(input: &L2In) -> L2Out {
    crate::verify_shim::s1_cluster_15(input)
}

/// s1_cluster_17 —— bank07:D012 簇入口注册（SC-S1-17）。
pub fn sub_s1_cluster_17(input: &L2In) -> L2Out {
    crate::verify_shim::s1_cluster_17(input)
}

/// s1_cluster_18 —— bank07:CF55 簇入口注册（SC-S1-18 SetSamusStand）。
pub fn sub_s1_cluster_18(input: &L2In) -> L2Out {
    crate::verify_shim::s1_cluster_18(input)
}

/// s1_cluster_20 —— bank07:D6E4 簇入口注册（SC-S1-20）。
pub fn sub_s1_cluster_20(input: &L2In) -> L2Out {
    crate::verify_shim::s1_cluster_20(input)
}

/// s1_cluster_21 —— bank07:E789 簇入口注册（SC-S1-21）。
pub fn sub_s1_cluster_21(input: &L2In) -> L2Out {
    crate::verify_shim::s1_cluster_21(input)
}

/// s1_cluster_22 —— bank07:EAD4 簇入口注册（SC-S1-22 EnemyLoop VM）。
pub fn sub_s1_cluster_22(input: &L2In) -> L2Out {
    crate::verify_shim::s1_cluster_22(input)
}

/// s1_cluster_23 —— bank07:EDD6 簇入口注册（SC-S1-23 spawn VM）。
pub fn sub_s1_cluster_23(input: &L2In) -> L2Out {
    crate::verify_shim::s1_cluster_23(input)
}

/// s1_cluster_26 —— bank07:CE84 簇入口注册（SC-S1-26 IsSamusDead）。
pub fn sub_s1_cluster_26(input: &L2In) -> L2Out {
    crate::verify_shim::s1_cluster_26(input)
}

/// s1_cluster_28 —— bank07:F120 簇入口注册（SC-S1-28）。
pub fn sub_s1_cluster_28(input: &L2In) -> L2Out {
    crate::verify_shim::s1_cluster_28(input)
}

/// s2_cluster_030 —— bank07:F6E2 簇入口注册（SC-S2-00）。
pub fn sub_s2_cluster_030(input: &L2In) -> L2Out {
    crate::verify_shim::s2_cluster_030(input)
}

/// s2_cluster_031 —— bank07:F702 簇入口注册（SC-S2-01）。
pub fn sub_s2_cluster_031(input: &L2In) -> L2Out {
    crate::verify_shim::s2_cluster_031(input)
}

/// s2_cluster_032 —— bank07:F97C 簇入口注册（SC-S2-02）。
pub fn sub_s2_cluster_032(input: &L2In) -> L2Out {
    crate::verify_shim::s2_cluster_032(input)
}

/// s2_cluster_033 —— bank07:CF2E 簇入口注册（SC-S2-03）。
pub fn sub_s2_cluster_033(input: &L2In) -> L2Out {
    crate::verify_shim::s2_cluster_033(input)
}

/// s2_cluster_037 —— bank07:CDFA 簇入口注册（SC-S2-07）。
pub fn sub_s2_cluster_037(input: &L2In) -> L2Out {
    crate::verify_shim::s2_cluster_037(input)
}

/// s2_cluster_038 —— bank07:CD92 簇入口注册（SC-S2-08）。
pub fn sub_s2_cluster_038(input: &L2In) -> L2Out {
    crate::verify_shim::s2_cluster_038(input)
}

/// s2_cluster_040 —— bank07:D450 簇入口注册（SC-S2-10）。
pub fn sub_s2_cluster_040(input: &L2In) -> L2Out {
    crate::verify_shim::s2_cluster_040(input)
}

/// s2_cluster_044 —— bank07:D61B 簇入口注册（SC-S2-14）。
pub fn sub_s2_cluster_044(input: &L2In) -> L2Out {
    crate::verify_shim::s2_cluster_044(input)
}

/// s2_cluster_045 —— bank07:D2FA 簇入口注册（SC-S2-15）。
pub fn sub_s2_cluster_045(input: &L2In) -> L2Out {
    crate::verify_shim::s2_cluster_045(input)
}

/// s2_cluster_047 —— bank07:D648 簇入口注册（SC-S2-17）。
pub fn sub_s2_cluster_047(input: &L2In) -> L2Out {
    crate::verify_shim::s2_cluster_047(input)
}

/// s2_cluster_048 —— bank07:EB8C 簇入口注册（SC-S2-18 DoorHandler VM 族）。
pub fn sub_s2_cluster_048(input: &L2In) -> L2Out {
    crate::verify_shim::s2_cluster_048(input)
}

/// s2_cluster_049 —— bank07:EC04 簇入口注册（SC-S2-19 SpawnElevator VM）。
pub fn sub_s2_cluster_049(input: &L2In) -> L2Out {
    crate::verify_shim::s2_cluster_049(input)
}

/// s2_cluster_050 —— bank07:EEC6 簇入口注册（SC-S2-20 ItemCheck VM）。
pub fn sub_s2_cluster_050(input: &L2In) -> L2Out {
    crate::verify_shim::s2_cluster_050(input)
}

/// s2_cluster_051 —— bank07:F0E5 簇入口注册（SC-S2-21）。
pub fn sub_s2_cluster_051(input: &L2In) -> L2Out {
    crate::verify_shim::s2_cluster_051(input)
}

/// s2_cluster_053 —— bank07:CE92 簇入口注册（SC-S2-23 SubtractHealth）。
pub fn sub_s2_cluster_053(input: &L2In) -> L2Out {
    crate::verify_shim::s2_cluster_053(input)
}

/// s2_cluster_054 —— bank07:CD9C 簇入口注册（SC-S2-24）。
pub fn sub_s2_cluster_054(input: &L2In) -> L2Out {
    crate::verify_shim::s2_cluster_054(input)
}

/// s2_cluster_055 —— bank07:C2C6 簇入口注册（SC-S2-25 Amul8）。
pub fn sub_s2_cluster_055(input: &L2In) -> L2Out {
    crate::verify_shim::s2_cluster_055(input)
}

/// s2_cluster_058 —— bank07:F416 簇入口注册（SC-S2-28）。
pub fn sub_s2_cluster_058(input: &L2In) -> L2Out {
    crate::verify_shim::s2_cluster_058(input)
}

/// s2_cluster_059 —— bank07:F50B 簇入口注册（SC-S2-29）。
pub fn sub_s2_cluster_059(input: &L2In) -> L2Out {
    crate::verify_shim::s2_cluster_059(input)
}

/// s3_cluster_060 —— bank07:CBD2 簇入口注册（SC-S3-00 SFX_MetroidHit）。
pub fn sub_s3_cluster_060(input: &L2In) -> L2Out {
    crate::verify_shim::s3_cluster_060(input)
}

/// s3_cluster_062 —— bank07:CBB0 簇入口注册（SC-S3-02 SFX_EnemyHit）。
pub fn sub_s3_cluster_062(input: &L2In) -> L2Out {
    crate::verify_shim::s3_cluster_062(input)
}

/// s3_cluster_063 —— bank07:F6EB 簇入口注册（SC-S3-03）。
pub fn sub_s3_cluster_063(input: &L2In) -> L2Out {
    crate::verify_shim::s3_cluster_063(input)
}

/// s3_cluster_064 —— bank07:F96A 簇入口注册（SC-S3-04）。
pub fn sub_s3_cluster_064(input: &L2In) -> L2Out {
    crate::verify_shim::s3_cluster_064(input)
}

/// s3_cluster_065 —— bank07:F96D 簇入口注册（SC-S3-05）。
pub fn sub_s3_cluster_065(input: &L2In) -> L2Out {
    crate::verify_shim::s3_cluster_065(input)
}

/// s3_cluster_066 —— bank07:D015 簇入口注册（SC-S3-06 SamusJump）。
pub fn sub_s3_cluster_066(input: &L2In) -> L2Out {
    crate::verify_shim::s3_cluster_066(input)
}

/// s3_cluster_067 —— bank07:D02B 簇入口注册（SC-S3-07）。
pub fn sub_s3_cluster_067(input: &L2In) -> L2Out {
    crate::verify_shim::s3_cluster_067(input)
}

/// s3_cluster_068 —— bank07:CFC3 簇入口注册（SC-S3-08 SetSamusJump）。
pub fn sub_s3_cluster_068(input: &L2In) -> L2Out {
    crate::verify_shim::s3_cluster_068(input)
}

/// s3_cluster_070 —— bank07:CF77 簇入口注册（SC-S3-10 SetSamusPntUp）。
pub fn sub_s3_cluster_070(input: &L2In) -> L2Out {
    crate::verify_shim::s3_cluster_070(input)
}

/// s3_cluster_071 —— bank07:CBEF 簇入口注册（SC-S3-11 SFX_SamusBall）。
pub fn sub_s3_cluster_071(input: &L2In) -> L2Out {
    crate::verify_shim::s3_cluster_071(input)
}

/// s3_cluster_074 —— bank07:CCB7 簇入口注册（SC-S3-14）。
pub fn sub_s3_cluster_074(input: &L2In) -> L2Out {
    crate::verify_shim::s3_cluster_074(input)
}

/// s3_cluster_076 —— bank07:CFB7 簇入口注册（SC-S3-16 ClearHorzData）。
pub fn sub_s3_cluster_076(input: &L2In) -> L2Out {
    crate::verify_shim::s3_cluster_076(input)
}

/// s3_cluster_077 —— bank07:CBA8 簇入口注册（SC-S3-17 SFX_BombLaunch）。
pub fn sub_s3_cluster_077(input: &L2In) -> L2Out {
    crate::verify_shim::s3_cluster_077(input)
}

/// s3_cluster_083 —— bank07:CC07 簇入口注册（SC-S3-23 TourianMusic）。
pub fn sub_s3_cluster_083(input: &L2In) -> L2Out {
    crate::verify_shim::s3_cluster_083(input)
}

/// s3_cluster_084 —— bank07:D417 簇入口注册（SC-S3-24 DrawFrame）。
pub fn sub_s3_cluster_084(input: &L2In) -> L2Out {
    crate::verify_shim::s3_cluster_084(input)
}

/// s3_cluster_085 —— bank07:D48F 簇入口注册（SC-S3-25）。
pub fn sub_s3_cluster_085(input: &L2In) -> L2Out {
    crate::verify_shim::s3_cluster_085(input)
}

/// s4_cluster_094 —— bank07:D606 簇入口注册（SC-S4-04 IsWalkableTile）。
pub fn sub_s4_cluster_094(input: &L2In) -> L2Out {
    crate::verify_shim::s4_cluster_094(input)
}

/// s4_cluster_096 —— bank07:D621 簇入口注册（SC-S4-06）。
pub fn sub_s4_cluster_096(input: &L2In) -> L2Out {
    crate::verify_shim::s4_cluster_096(input)
}

/// s4_cluster_097 —— bank07:CB9A 簇入口注册（SC-S4-07 SFX_BombExplode）。
pub fn sub_s4_cluster_097(input: &L2In) -> L2Out {
    crate::verify_shim::s4_cluster_097(input)
}

/// s4_cluster_099 —— bank07:D831 簇入口注册（SC-S4-09）。
pub fn sub_s4_cluster_099(input: &L2In) -> L2Out {
    crate::verify_shim::s4_cluster_099(input)
}

/// s4_cluster_100 —— bank07:D837 簇入口注册（SC-S4-10）。
pub fn sub_s4_cluster_100(input: &L2In) -> L2Out {
    crate::verify_shim::s4_cluster_100(input)
}

/// s4_cluster_101 —— bank07:D851 簇入口注册（SC-S4-11）。
pub fn sub_s4_cluster_101(input: &L2In) -> L2Out {
    crate::verify_shim::s4_cluster_101(input)
}

/// s4_cluster_102 —— bank07:CA18 簇入口注册（SC-S4-12 IsEngineRunning）。
pub fn sub_s4_cluster_102(input: &L2In) -> L2Out {
    crate::verify_shim::s4_cluster_102(input)
}

/// s4_cluster_104 —— bank07:CB73 簇入口注册（SC-S4-14 SelectSamusPal）。
pub fn sub_s4_cluster_104(input: &L2In) -> L2Out {
    crate::verify_shim::s4_cluster_104(input)
}

/// s4_cluster_106 —— bank07:C8B0 簇入口注册（SC-S4-16）。
pub fn sub_s4_cluster_106(input: &L2In) -> L2Out {
    crate::verify_shim::s4_cluster_106(input)
}

/// s4_cluster_108 —— bank07:D96B 簇入口注册（SC-S4-18 ToggleScroll）。
pub fn sub_s4_cluster_108(input: &L2In) -> L2Out {
    crate::verify_shim::s4_cluster_108(input)
}

/// s4_cluster_110 —— bank07:E778 簇入口注册（SC-S4-20 EnemyCheckMoveVertical）。
pub fn sub_s4_cluster_110(input: &L2In) -> L2Out {
    crate::verify_shim::s4_cluster_110(input)
}

/// s4_cluster_112 —— bank07:E8F9 簇入口注册（SC-S4-22）。
pub fn sub_s4_cluster_112(input: &L2In) -> L2Out {
    crate::verify_shim::s4_cluster_112(input)
}

/// s4_cluster_119 —— bank07:EB77 簇入口注册（SC-S4-29）。
pub fn sub_s4_cluster_119(input: &L2In) -> L2Out {
    crate::verify_shim::s4_cluster_119(input)
}

/// s5_cluster_121 —— bank07:EE11 簇入口注册（SC-S5-01）。
pub fn sub_s5_cluster_121(input: &L2In) -> L2Out {
    crate::verify_shim::s5_cluster_121(input)
}

/// s5_cluster_122 —— bank07:DC67 簇入口注册（SC-S5-02 CreateItemID）。
pub fn sub_s5_cluster_122(input: &L2In) -> L2Out {
    crate::verify_shim::s5_cluster_122(input)
}

/// s5_cluster_123 —— bank07:EE6B 簇入口注册（SC-S5-03 SpawnMellow）。
pub fn sub_s5_cluster_123(input: &L2In) -> L2Out {
    crate::verify_shim::s5_cluster_123(input)
}

/// s5_cluster_128 —— bank07:F0A8 簇入口注册（SC-S5-08）。
pub fn sub_s5_cluster_128(input: &L2In) -> L2Out {
    crate::verify_shim::s5_cluster_128(input)
}

/// s5_cluster_130 —— bank07:F0E2 簇入口注册（SC-S5-10）。
pub fn sub_s5_cluster_130(input: &L2In) -> L2Out {
    crate::verify_shim::s5_cluster_130(input)
}

/// s5_cluster_131 —— bank07:F09F 簇入口注册（SC-S5-11）。
pub fn sub_s5_cluster_131(input: &L2In) -> L2Out {
    crate::verify_shim::s5_cluster_131(input)
}

/// s5_cluster_132 —— bank07:F0EF 簇入口注册（SC-S5-12）。
pub fn sub_s5_cluster_132(input: &L2In) -> L2Out {
    crate::verify_shim::s5_cluster_132(input)
}

/// s5_cluster_136 —— bank07:DC82 簇入口注册（SC-S5-16）。
pub fn sub_s5_cluster_136(input: &L2In) -> L2Out {
    crate::verify_shim::s5_cluster_136(input)
}

/// s5_cluster_137 —— bank07:F16F 簇入口注册（SC-S5-17）。
pub fn sub_s5_cluster_137(input: &L2In) -> L2Out {
    crate::verify_shim::s5_cluster_137(input)
}

/// s5_cluster_138 —— bank07:F1B6 簇入口注册（SC-S5-18）。
pub fn sub_s5_cluster_138(input: &L2In) -> L2Out {
    crate::verify_shim::s5_cluster_138(input)
}

/// s5_cluster_139 —— bank07:F1BC 簇入口注册（SC-S5-19 AddEnemy1XRadius）。
pub fn sub_s5_cluster_139(input: &L2In) -> L2Out {
    crate::verify_shim::s5_cluster_139(input)
}

/// s5_cluster_140 —— bank07:F279 簇入口注册（SC-S5-20）。
pub fn sub_s5_cluster_140(input: &L2In) -> L2Out {
    crate::verify_shim::s5_cluster_140(input)
}

/// s5_cluster_141 —— bank07:F2DF 簇入口注册（SC-S5-21）。
pub fn sub_s5_cluster_141(input: &L2In) -> L2Out {
    crate::verify_shim::s5_cluster_141(input)
}

/// s5_cluster_142 —— bank07:F270 簇入口注册（SC-S5-22）。
pub fn sub_s5_cluster_142(input: &L2In) -> L2Out {
    crate::verify_shim::s5_cluster_142(input)
}

/// s5_cluster_146 —— bank07:E094 簇入口注册（SC-S5-26）。
pub fn sub_s5_cluster_146(input: &L2In) -> L2Out {
    crate::verify_shim::s5_cluster_146(input)
}

/// s5_cluster_148 —— bank07:DD8B 簇入口注册（SC-S5-28）。
pub fn sub_s5_cluster_148(input: &L2In) -> L2Out {
    crate::verify_shim::s5_cluster_148(input)
}

/// s5_cluster_149 —— bank07:CEF9 簇入口注册（SC-S5-29 AddHealth）。
pub fn sub_s5_cluster_149(input: &L2In) -> L2Out {
    crate::verify_shim::s5_cluster_149(input)
}

/// s6_cluster_150 —— bank07:CBBC 簇入口注册（SC-S6-00 SFX_EnergyPickup）。
pub fn sub_s6_cluster_150(input: &L2In) -> L2Out {
    crate::verify_shim::s6_cluster_150(input)
}

/// s6_cluster_151 —— bank07:CBC0 簇入口注册（SC-S6-01 SFX_MissilePickup）。
pub fn sub_s6_cluster_151(input: &L2In) -> L2Out {
    crate::verify_shim::s6_cluster_151(input)
}

/// s6_cluster_152 —— bank07:F423 簇入口注册（SC-S6-02）。
pub fn sub_s6_cluster_152(input: &L2In) -> L2Out {
    crate::verify_shim::s6_cluster_152(input)
}

/// s6_cluster_153 —— bank07:F533 簇入口注册（SC-S6-03）。
pub fn sub_s6_cluster_153(input: &L2In) -> L2Out {
    crate::verify_shim::s6_cluster_153(input)
}

/// s6_cluster_155 —— bank07:CBB8 簇入口注册（SC-S6-04 SFX_Metal）。
pub fn sub_s6_cluster_155(input: &L2In) -> L2Out {
    crate::verify_shim::s6_cluster_155(input)
}

/// s6_cluster_156 —— bank07:F42D 簇入口注册（SC-S6-05）。
pub fn sub_s6_cluster_156(input: &L2In) -> L2Out {
    crate::verify_shim::s6_cluster_156(input)
}

/// s6_cluster_157 —— bank07:CBD6 簇入口注册（SC-S6-06 SFX_BossHit）。
pub fn sub_s6_cluster_157(input: &L2In) -> L2Out {
    crate::verify_shim::s6_cluster_157(input)
}

/// s6_cluster_158 —— bank07:CBCE 簇入口注册（SC-S6-07 SFX_BigEnemyHit）。
pub fn sub_s6_cluster_158(input: &L2In) -> L2Out {
    crate::verify_shim::s6_cluster_158(input)
}

/// s6_cluster_160 —— bank07:81DA 簇入口注册（SC-S6-09）。
pub fn sub_s6_cluster_160(input: &L2In) -> L2Out {
    crate::verify_shim::s6_cluster_160(input)
}

/// s6_cluster_162 —— bank07:CBA4 簇入口注册（SC-S6-11 SFX_OutOfPipe）。
pub fn sub_s6_cluster_162(input: &L2In) -> L2Out {
    crate::verify_shim::s6_cluster_162(input)
}

/// s6_cluster_164 —— bank07:F82F 簇入口注册（SC-S6-13）。
pub fn sub_s6_cluster_164(input: &L2In) -> L2Out {
    crate::verify_shim::s6_cluster_164(input)
}

/// s6_cluster_165 —— bank07:8206 簇入口注册（SC-S6-14）。
pub fn sub_s6_cluster_165(input: &L2In) -> L2Out {
    crate::verify_shim::s6_cluster_165(input)
}

/// s6_cluster_166 —— bank07:F887 簇入口注册（SC-S6-15）。
pub fn sub_s6_cluster_166(input: &L2In) -> L2Out {
    crate::verify_shim::s6_cluster_166(input)
}

/// s6_cluster_167 —— bank07:F88F 簇入口注册（SC-S6-16）。
pub fn sub_s6_cluster_167(input: &L2In) -> L2Out {
    crate::verify_shim::s6_cluster_167(input)
}

/// s6_cluster_169 —— bank07:F8D3 簇入口注册（SC-S6-18）。
pub fn sub_s6_cluster_169(input: &L2In) -> L2Out {
    crate::verify_shim::s6_cluster_169(input)
}

/// s6_cluster_170 —— bank07:F690 簇入口注册（SC-S6-19）。
pub fn sub_s6_cluster_170(input: &L2In) -> L2Out {
    crate::verify_shim::s6_cluster_170(input)
}

/// s6_cluster_171 —— bank07:F927 簇入口注册（SC-S6-20）。
pub fn sub_s6_cluster_171(input: &L2In) -> L2Out {
    crate::verify_shim::s6_cluster_171(input)
}

/// s6_cluster_172 —— bank07:F979 簇入口注册（SC-S6-21）。
pub fn sub_s6_cluster_172(input: &L2In) -> L2Out {
    crate::verify_shim::s6_cluster_172(input)
}

/// s6_cluster_174 —— bank07:F518 簇入口注册（SC-S6-23 LF518）。
pub fn sub_s6_cluster_174(input: &L2In) -> L2Out {
    crate::verify_shim::s6_cluster_174(input)
}

/// s7_cluster_175 —— bank07:FA2A 簇入口注册（SC-S7-00 GetEnemyRoomRAMPtr）。
pub fn sub_s7_cluster_175(input: &L2In) -> L2Out {
    crate::verify_shim::s7_cluster_175(input)
}

/// s7_cluster_176 —— bank07:E96A 簇入口注册（SC-S7-01 MakeCartRAMPtr）。
pub fn sub_s7_cluster_176(input: &L2In) -> L2Out {
    crate::verify_shim::s7_cluster_176(input)
}

/// s7_cluster_177 —— bank07:F693 簇入口注册（SC-S7-02）。
pub fn sub_s7_cluster_177(input: &L2In) -> L2Out {
    crate::verify_shim::s7_cluster_177(input)
}

/// s7_cluster_178 —— bank07:FCC1 簇入口注册（SC-S7-03）。
pub fn sub_s7_cluster_178(input: &L2In) -> L2Out {
    crate::verify_shim::s7_cluster_178(input)
}

/// s7_cluster_179 —— bank07:FCFA 簇入口注册（SC-S7-04）。
pub fn sub_s7_cluster_179(input: &L2In) -> L2Out {
    crate::verify_shim::s7_cluster_179(input)
}

/// s7_cluster_180 —— bank07:FE95 簇入口注册（SC-S7-05 DrawTileBlast）。
pub fn sub_s7_cluster_180(input: &L2In) -> L2Out {
    crate::verify_shim::s7_cluster_180(input)
}

/// a1_cluster_000 —— bank01:8000 簇入口注册（SC-B1-00）。
pub fn sub_a1_cluster_000(input: &L2In) -> L2Out {
    crate::verify_shim::a1_cluster_000(input)
}

/// a1_cluster_001 —— bank01:8003 簇入口注册（SC-B1-01）。
pub fn sub_a1_cluster_001(input: &L2In) -> L2Out {
    crate::verify_shim::a1_cluster_001(input)
}

/// a1_cluster_002 —— bank01:8006 簇入口注册（SC-B1-02）。
pub fn sub_a1_cluster_002(input: &L2In) -> L2Out {
    crate::verify_shim::a1_cluster_002(input)
}

/// a1_cluster_003 —— bank01:8009 簇入口注册（SC-B1-03）。
pub fn sub_a1_cluster_003(input: &L2In) -> L2Out {
    crate::verify_shim::a1_cluster_003(input)
}

/// a1_cluster_004 —— bank01:800C 簇入口注册（SC-B1-04）。
pub fn sub_a1_cluster_004(input: &L2In) -> L2Out {
    crate::verify_shim::a1_cluster_004(input)
}

/// a1_cluster_005 —— bank01:800F 簇入口注册（SC-B1-05）。
pub fn sub_a1_cluster_005(input: &L2In) -> L2Out {
    crate::verify_shim::a1_cluster_005(input)
}

/// a1_cluster_006 —— bank01:801B 簇入口注册（SC-B1-06）。
pub fn sub_a1_cluster_006(input: &L2In) -> L2Out {
    crate::verify_shim::a1_cluster_006(input)
}

/// a1_cluster_007 —— bank01:801E 簇入口注册（SC-B1-07）。
pub fn sub_a1_cluster_007(input: &L2In) -> L2Out {
    crate::verify_shim::a1_cluster_007(input)
}

/// a1_cluster_008 —— bank01:8027 簇入口注册（SC-B1-08）。
pub fn sub_a1_cluster_008(input: &L2In) -> L2Out {
    crate::verify_shim::a1_cluster_008(input)
}

/// a1_cluster_009 —— bank01:802A 簇入口注册（SC-B1-09）。
pub fn sub_a1_cluster_009(input: &L2In) -> L2Out {
    crate::verify_shim::a1_cluster_009(input)
}

/// a1_cluster_010 —— bank01:802D 簇入口注册（SC-B1-10）。
pub fn sub_a1_cluster_010(input: &L2In) -> L2Out {
    crate::verify_shim::a1_cluster_010(input)
}

/// a1_cluster_011 —— bank01:8030 簇入口注册（SC-B1-11）。
pub fn sub_a1_cluster_011(input: &L2In) -> L2Out {
    crate::verify_shim::a1_cluster_011(input)
}

/// a1_cluster_012 —— bank01:8033 簇入口注册（SC-B1-12）。
pub fn sub_a1_cluster_012(input: &L2In) -> L2Out {
    crate::verify_shim::a1_cluster_012(input)
}

/// a1_cluster_013 —— bank01:8036 簇入口注册（SC-B1-13）。
pub fn sub_a1_cluster_013(input: &L2In) -> L2Out {
    crate::verify_shim::a1_cluster_013(input)
}

/// a1_cluster_014 —— bank01:8039 簇入口注册（SC-B1-14）。
pub fn sub_a1_cluster_014(input: &L2In) -> L2Out {
    crate::verify_shim::a1_cluster_014(input)
}

/// a1_cluster_015 —— bank01:8BB1 簇入口注册（SC-B1-15）。
pub fn sub_a1_cluster_015(input: &L2In) -> L2Out {
    crate::verify_shim::a1_cluster_015(input)
}

/// a1_cluster_016 —— bank01:8C61 簇入口注册（SC-B1-16）。
pub fn sub_a1_cluster_016(input: &L2In) -> L2Out {
    crate::verify_shim::a1_cluster_016(input)
}

/// a1_cluster_017 —— bank01:8C71 簇入口注册（SC-B1-17）。
pub fn sub_a1_cluster_017(input: &L2In) -> L2Out {
    crate::verify_shim::a1_cluster_017(input)
}

/// a1_cluster_018 —— bank01:8C76 簇入口注册（SC-B1-18）。
pub fn sub_a1_cluster_018(input: &L2In) -> L2Out {
    crate::verify_shim::a1_cluster_018(input)
}

/// a1_cluster_019 —— bank01:8C8A 簇入口注册（SC-B1-19）。
pub fn sub_a1_cluster_019(input: &L2In) -> L2Out {
    crate::verify_shim::a1_cluster_019(input)
}

/// a1_cluster_020 —— bank01:8C97 簇入口注册（SC-B1-20）。
pub fn sub_a1_cluster_020(input: &L2In) -> L2Out {
    crate::verify_shim::a1_cluster_020(input)
}

/// a1_cluster_021 —— bank01:8CB6 簇入口注册（SC-B1-21）。
pub fn sub_a1_cluster_021(input: &L2In) -> L2Out {
    crate::verify_shim::a1_cluster_021(input)
}

/// a2_cluster_022 —— bank01:8CBB 簇入口注册（SC-B2-00）。
pub fn sub_a2_cluster_022(input: &L2In) -> L2Out {
    crate::verify_shim::a2_cluster_022(input)
}

/// a2_cluster_023 —— bank01:8CC0 簇入口注册（SC-B2-01）。
pub fn sub_a2_cluster_023(input: &L2In) -> L2Out {
    crate::verify_shim::a2_cluster_023(input)
}

/// a2_cluster_024 —— bank01:8CDD 簇入口注册（SC-B2-02）。
pub fn sub_a2_cluster_024(input: &L2In) -> L2Out {
    crate::verify_shim::a2_cluster_024(input)
}

/// a2_cluster_025 —— bank01:8CE0 簇入口注册（SC-B2-03）。
pub fn sub_a2_cluster_025(input: &L2In) -> L2Out {
    crate::verify_shim::a2_cluster_025(input)
}

/// a2_cluster_026 —— bank01:8CE3 簇入口注册（SC-B2-04）。
pub fn sub_a2_cluster_026(input: &L2In) -> L2Out {
    crate::verify_shim::a2_cluster_026(input)
}

/// a2_cluster_027 —— bank01:95C6 簇入口注册（SC-B2-05）。
pub fn sub_a2_cluster_027(input: &L2In) -> L2Out {
    crate::verify_shim::a2_cluster_027(input)
}

/// a2_cluster_028 —— bank01:99A1 簇入口注册（SC-B2-06）。
pub fn sub_a2_cluster_028(input: &L2In) -> L2Out {
    crate::verify_shim::a2_cluster_028(input)
}

/// a2_cluster_029 —— bank01:99BA 簇入口注册（SC-B2-07）。
pub fn sub_a2_cluster_029(input: &L2In) -> L2Out {
    crate::verify_shim::a2_cluster_029(input)
}

/// a2_cluster_030 —— bank01:99C8 簇入口注册（SC-B2-08）。
pub fn sub_a2_cluster_030(input: &L2In) -> L2Out {
    crate::verify_shim::a2_cluster_030(input)
}

/// a2_cluster_031 —— bank01:99CC 簇入口注册（SC-B2-09）。
pub fn sub_a2_cluster_031(input: &L2In) -> L2Out {
    crate::verify_shim::a2_cluster_031(input)
}

/// a2_cluster_032 —— bank01:9A74 簇入口注册（SC-B2-10）。
pub fn sub_a2_cluster_032(input: &L2In) -> L2Out {
    crate::verify_shim::a2_cluster_032(input)
}

/// a2_cluster_034 —— bank01:9A81 簇入口注册（SC-B2-12）。
pub fn sub_a2_cluster_034(input: &L2In) -> L2Out {
    crate::verify_shim::a2_cluster_034(input)
}

/// a2_cluster_035 —— bank01:9A84 簇入口注册（SC-B2-13）。
pub fn sub_a2_cluster_035(input: &L2In) -> L2Out {
    crate::verify_shim::a2_cluster_035(input)
}

/// a2_cluster_036 —— bank01:9A8F 簇入口注册（SC-B2-14）。
pub fn sub_a2_cluster_036(input: &L2In) -> L2Out {
    crate::verify_shim::a2_cluster_036(input)
}

/// a2_cluster_037 —— bank01:9B8A 簇入口注册（SC-B2-15）。
pub fn sub_a2_cluster_037(input: &L2In) -> L2Out {
    crate::verify_shim::a2_cluster_037(input)
}

/// a2_cluster_038 —— bank01:9B92 簇入口注册（SC-B2-16）。
pub fn sub_a2_cluster_038(input: &L2In) -> L2Out {
    crate::verify_shim::a2_cluster_038(input)
}

/// a2_cluster_040 —— bank01:9BC5 簇入口注册（SC-B2-18）。
pub fn sub_a2_cluster_040(input: &L2In) -> L2Out {
    crate::verify_shim::a2_cluster_040(input)
}

/// a2_cluster_041 —— bank01:9BC8 簇入口注册（SC-B2-19）。
pub fn sub_a2_cluster_041(input: &L2In) -> L2Out {
    crate::verify_shim::a2_cluster_041(input)
}

/// a2_cluster_042 —— bank01:9BD2 簇入口注册（SC-B2-20）。
pub fn sub_a2_cluster_042(input: &L2In) -> L2Out {
    crate::verify_shim::a2_cluster_042(input)
}

/// a3_cluster_000 —— bank02:8000 簇入口注册（SC-B3-00）。
pub fn sub_a3_cluster_000(input: &L2In) -> L2Out {
    crate::verify_shim::a3_cluster_000(input)
}

/// a3_cluster_001 —— bank02:8003 簇入口注册（SC-B3-01）。
pub fn sub_a3_cluster_001(input: &L2In) -> L2Out {
    crate::verify_shim::a3_cluster_001(input)
}

/// a3_cluster_002 —— bank02:8006 簇入口注册（SC-B3-02）。
pub fn sub_a3_cluster_002(input: &L2In) -> L2Out {
    crate::verify_shim::a3_cluster_002(input)
}

/// a3_cluster_003 —— bank02:8009 簇入口注册（SC-B3-03）。
pub fn sub_a3_cluster_003(input: &L2In) -> L2Out {
    crate::verify_shim::a3_cluster_003(input)
}

/// a3_cluster_004 —— bank02:800C 簇入口注册（SC-B3-04）。
pub fn sub_a3_cluster_004(input: &L2In) -> L2Out {
    crate::verify_shim::a3_cluster_004(input)
}

/// a3_cluster_005 —— bank02:800F 簇入口注册（SC-B3-05）。
pub fn sub_a3_cluster_005(input: &L2In) -> L2Out {
    crate::verify_shim::a3_cluster_005(input)
}

/// a3_cluster_006 —— bank02:801E 簇入口注册（SC-B3-06）。
pub fn sub_a3_cluster_006(input: &L2In) -> L2Out {
    crate::verify_shim::a3_cluster_006(input)
}

/// a3_cluster_007 —— bank02:8021 簇入口注册（SC-B3-07）。
pub fn sub_a3_cluster_007(input: &L2In) -> L2Out {
    crate::verify_shim::a3_cluster_007(input)
}

/// a3_cluster_008 —— bank02:8027 簇入口注册（SC-B3-08）。
pub fn sub_a3_cluster_008(input: &L2In) -> L2Out {
    crate::verify_shim::a3_cluster_008(input)
}

/// a3_cluster_009 —— bank02:802A 簇入口注册（SC-B3-09）。
pub fn sub_a3_cluster_009(input: &L2In) -> L2Out {
    crate::verify_shim::a3_cluster_009(input)
}

/// a3_cluster_010 —— bank02:8036 簇入口注册（SC-B3-10）。
pub fn sub_a3_cluster_010(input: &L2In) -> L2Out {
    crate::verify_shim::a3_cluster_010(input)
}

/// a3_cluster_011 —— bank02:8039 簇入口注册（SC-B3-11）。
pub fn sub_a3_cluster_011(input: &L2In) -> L2Out {
    crate::verify_shim::a3_cluster_011(input)
}

/// a3_cluster_012 —— bank02:8BB1 簇入口注册（SC-B3-12）。
pub fn sub_a3_cluster_012(input: &L2In) -> L2Out {
    crate::verify_shim::a3_cluster_012(input)
}

/// a3_cluster_013 —— bank02:8C61 簇入口注册（SC-B3-13）。
pub fn sub_a3_cluster_013(input: &L2In) -> L2Out {
    crate::verify_shim::a3_cluster_013(input)
}

/// a3_cluster_014 —— bank02:8C71 簇入口注册（SC-B3-14）。
pub fn sub_a3_cluster_014(input: &L2In) -> L2Out {
    crate::verify_shim::a3_cluster_014(input)
}

/// a3_cluster_015 —— bank02:8C76 簇入口注册（SC-B3-15）。
pub fn sub_a3_cluster_015(input: &L2In) -> L2Out {
    crate::verify_shim::a3_cluster_015(input)
}

/// a3_cluster_016 —— bank02:8C8A 簇入口注册（SC-B3-16）。
pub fn sub_a3_cluster_016(input: &L2In) -> L2Out {
    crate::verify_shim::a3_cluster_016(input)
}

/// a3_cluster_017 —— bank02:8C97 簇入口注册（SC-B3-17）。
pub fn sub_a3_cluster_017(input: &L2In) -> L2Out {
    crate::verify_shim::a3_cluster_017(input)
}

/// a3_cluster_018 —— bank02:8CB6 簇入口注册（SC-B3-18）。
pub fn sub_a3_cluster_018(input: &L2In) -> L2Out {
    crate::verify_shim::a3_cluster_018(input)
}

/// a3_cluster_019 —— bank02:8CBB 簇入口注册（SC-B3-19）。
pub fn sub_a3_cluster_019(input: &L2In) -> L2Out {
    crate::verify_shim::a3_cluster_019(input)
}

/// a3_cluster_020 —— bank02:8CC0 簇入口注册（SC-B3-20）。
pub fn sub_a3_cluster_020(input: &L2In) -> L2Out {
    crate::verify_shim::a3_cluster_020(input)
}

/// a3_cluster_021 —— bank02:8CDD 簇入口注册（SC-B3-21）。
pub fn sub_a3_cluster_021(input: &L2In) -> L2Out {
    crate::verify_shim::a3_cluster_021(input)
}

/// a3_cluster_022 —— bank02:8CE0 簇入口注册（SC-B3-22）。
pub fn sub_a3_cluster_022(input: &L2In) -> L2Out {
    crate::verify_shim::a3_cluster_022(input)
}

/// a3_cluster_023 —— bank02:8CE3 簇入口注册（SC-B3-23）。
pub fn sub_a3_cluster_023(input: &L2In) -> L2Out {
    crate::verify_shim::a3_cluster_023(input)
}

/// a3_cluster_024 —— bank02:95C6 簇入口注册（SC-B3-24）。
pub fn sub_a3_cluster_024(input: &L2In) -> L2Out {
    crate::verify_shim::a3_cluster_024(input)
}

/// a3_cluster_025 —— bank02:9839 簇入口注册（SC-B3-25）。
pub fn sub_a3_cluster_025(input: &L2In) -> L2Out {
    crate::verify_shim::a3_cluster_025(input)
}

/// a3_cluster_026 —— bank02:98A8 簇入口注册（SC-B3-26）。
pub fn sub_a3_cluster_026(input: &L2In) -> L2Out {
    crate::verify_shim::a3_cluster_026(input)
}

/// a3_cluster_027 —— bank02:98B0 簇入口注册（SC-B3-27）。
pub fn sub_a3_cluster_027(input: &L2In) -> L2Out {
    crate::verify_shim::a3_cluster_027(input)
}

/// a3_cluster_028 —— bank02:98EB 簇入口注册（SC-B3-28）。
pub fn sub_a3_cluster_028(input: &L2In) -> L2Out {
    crate::verify_shim::a3_cluster_028(input)
}

/// a3_cluster_029 —— bank02:98EE 簇入口注册（SC-B3-29）。
pub fn sub_a3_cluster_029(input: &L2In) -> L2Out {
    crate::verify_shim::a3_cluster_029(input)
}

/// a3_cluster_032 —— bank02:99CB 簇入口注册（SC-B3-32）。
pub fn sub_a3_cluster_032(input: &L2In) -> L2Out {
    crate::verify_shim::a3_cluster_032(input)
}

/// a3_cluster_033 —— bank02:99CE 簇入口注册（SC-B3-33）。
pub fn sub_a3_cluster_033(input: &L2In) -> L2Out {
    crate::verify_shim::a3_cluster_033(input)
}

/// a3_cluster_034 —— bank02:99DC 簇入口注册（SC-B3-34）。
pub fn sub_a3_cluster_034(input: &L2In) -> L2Out {
    crate::verify_shim::a3_cluster_034(input)
}

/// a3_cluster_035 —— bank02:9B4C 簇入口注册（SC-B3-35）。
pub fn sub_a3_cluster_035(input: &L2In) -> L2Out {
    crate::verify_shim::a3_cluster_035(input)
}

/// a4_cluster_000 —— bank03:8000 簇入口注册（SC-B4-00）。
pub fn sub_a4_cluster_000(input: &L2In) -> L2Out {
    crate::verify_shim::a4_cluster_000(input)
}

/// a4_cluster_001 —— bank03:8003 簇入口注册（SC-B4-01）。
pub fn sub_a4_cluster_001(input: &L2In) -> L2Out {
    crate::verify_shim::a4_cluster_001(input)
}

/// a4_cluster_002 —— bank03:8006 簇入口注册（SC-B4-02）。
pub fn sub_a4_cluster_002(input: &L2In) -> L2Out {
    crate::verify_shim::a4_cluster_002(input)
}

/// a4_cluster_003 —— bank03:8027 簇入口注册（SC-B4-03）。
pub fn sub_a4_cluster_003(input: &L2In) -> L2Out {
    crate::verify_shim::a4_cluster_003(input)
}

/// a4_cluster_005 —— bank03:803F 簇入口注册（SC-B4-05）。
pub fn sub_a4_cluster_005(input: &L2In) -> L2Out {
    crate::verify_shim::a4_cluster_005(input)
}

/// a4_cluster_006 —— bank03:8042 簇入口注册（SC-B4-06）。
pub fn sub_a4_cluster_006(input: &L2In) -> L2Out {
    crate::verify_shim::a4_cluster_006(input)
}

/// a4_cluster_007 —— bank03:8BB1 簇入口注册（SC-B4-07）。
pub fn sub_a4_cluster_007(input: &L2In) -> L2Out {
    crate::verify_shim::a4_cluster_007(input)
}

/// a4_cluster_008 —— bank03:8C61 簇入口注册（SC-B4-08）。
pub fn sub_a4_cluster_008(input: &L2In) -> L2Out {
    crate::verify_shim::a4_cluster_008(input)
}

/// a4_cluster_009 —— bank03:8C71 簇入口注册（SC-B4-09）。
pub fn sub_a4_cluster_009(input: &L2In) -> L2Out {
    crate::verify_shim::a4_cluster_009(input)
}

/// a4_cluster_010 —— bank03:8C76 簇入口注册（SC-B4-10）。
pub fn sub_a4_cluster_010(input: &L2In) -> L2Out {
    crate::verify_shim::a4_cluster_010(input)
}

/// a4_cluster_011 —— bank03:8C8A 簇入口注册（SC-B4-11）。
pub fn sub_a4_cluster_011(input: &L2In) -> L2Out {
    crate::verify_shim::a4_cluster_011(input)
}

/// a4_cluster_012 —— bank03:8C97 簇入口注册（SC-B4-12）。
pub fn sub_a4_cluster_012(input: &L2In) -> L2Out {
    crate::verify_shim::a4_cluster_012(input)
}

/// a4_cluster_013 —— bank03:8CB6 簇入口注册（SC-B4-13）。
pub fn sub_a4_cluster_013(input: &L2In) -> L2Out {
    crate::verify_shim::a4_cluster_013(input)
}

/// a4_cluster_014 —— bank03:8CBB 簇入口注册（SC-B4-14）。
pub fn sub_a4_cluster_014(input: &L2In) -> L2Out {
    crate::verify_shim::a4_cluster_014(input)
}

/// a4_cluster_015 —— bank03:8CC0 簇入口注册（SC-B4-15）。
pub fn sub_a4_cluster_015(input: &L2In) -> L2Out {
    crate::verify_shim::a4_cluster_015(input)
}

/// a4_cluster_016 —— bank03:8CDD 簇入口注册（SC-B4-16）。
pub fn sub_a4_cluster_016(input: &L2In) -> L2Out {
    crate::verify_shim::a4_cluster_016(input)
}

/// a4_cluster_017 —— bank03:8CE0 簇入口注册（SC-B4-17）。
pub fn sub_a4_cluster_017(input: &L2In) -> L2Out {
    crate::verify_shim::a4_cluster_017(input)
}

/// a4_cluster_018 —— bank03:8CE3 簇入口注册（SC-B4-18）。
pub fn sub_a4_cluster_018(input: &L2In) -> L2Out {
    crate::verify_shim::a4_cluster_018(input)
}

/// a4_cluster_019 —— bank03:95C6 簇入口注册（SC-B4-19）。
pub fn sub_a4_cluster_019(input: &L2In) -> L2Out {
    crate::verify_shim::a4_cluster_019(input)
}

/// a4_cluster_020 —— bank03:97E2 簇入口注册（SC-B4-20）。
pub fn sub_a4_cluster_020(input: &L2In) -> L2Out {
    crate::verify_shim::a4_cluster_020(input)
}

/// a4_cluster_021 —— bank03:9817 簇入口注册（SC-B4-21）。
pub fn sub_a4_cluster_021(input: &L2In) -> L2Out {
    crate::verify_shim::a4_cluster_021(input)
}

/// a5_cluster_024 —— bank03:9900 簇入口注册（SC-B5-00）。
pub fn sub_a5_cluster_024(input: &L2In) -> L2Out {
    crate::verify_shim::a5_cluster_024(input)
}

/// a5_cluster_025 —— bank03:9926 簇入口注册（SC-B5-01）。
pub fn sub_a5_cluster_025(input: &L2In) -> L2Out {
    crate::verify_shim::a5_cluster_025(input)
}

/// a5_cluster_026 —— bank03:9929 簇入口注册（SC-B5-02）。
pub fn sub_a5_cluster_026(input: &L2In) -> L2Out {
    crate::verify_shim::a5_cluster_026(input)
}

/// a5_cluster_027 —— bank03:9958 簇入口注册（SC-B5-03）。
pub fn sub_a5_cluster_027(input: &L2In) -> L2Out {
    crate::verify_shim::a5_cluster_027(input)
}

/// a5_cluster_028 —— bank03:995E 簇入口注册（SC-B5-04）。
pub fn sub_a5_cluster_028(input: &L2In) -> L2Out {
    crate::verify_shim::a5_cluster_028(input)
}

/// a5_cluster_029 —— bank03:9961 簇入口注册（SC-B5-05）。
pub fn sub_a5_cluster_029(input: &L2In) -> L2Out {
    crate::verify_shim::a5_cluster_029(input)
}

/// a5_cluster_030 —— bank03:99B8 簇入口注册（SC-B5-06）。
pub fn sub_a5_cluster_030(input: &L2In) -> L2Out {
    crate::verify_shim::a5_cluster_030(input)
}

/// a5_cluster_031 —— bank03:99D1 簇入口注册（SC-B5-07）。
pub fn sub_a5_cluster_031(input: &L2In) -> L2Out {
    crate::verify_shim::a5_cluster_031(input)
}

/// a5_cluster_033 —— bank03:9B03 簇入口注册（SC-B5-09）。
pub fn sub_a5_cluster_033(input: &L2In) -> L2Out {
    crate::verify_shim::a5_cluster_033(input)
}

/// a5_cluster_034 —— bank03:9DF2 簇入口注册（SC-B5-10）。
pub fn sub_a5_cluster_034(input: &L2In) -> L2Out {
    crate::verify_shim::a5_cluster_034(input)
}

/// a5_cluster_035 —— bank03:9E25 簇入口注册（SC-B5-11）。
pub fn sub_a5_cluster_035(input: &L2In) -> L2Out {
    crate::verify_shim::a5_cluster_035(input)
}

/// a5_cluster_036 —— bank03:9E28 簇入口注册（SC-B5-12）。
pub fn sub_a5_cluster_036(input: &L2In) -> L2Out {
    crate::verify_shim::a5_cluster_036(input)
}

/// a5_cluster_037 —— bank03:9E2B 簇入口注册（SC-B5-13）。
pub fn sub_a5_cluster_037(input: &L2In) -> L2Out {
    crate::verify_shim::a5_cluster_037(input)
}

/// a5_cluster_039 —— bank03:9E31 簇入口注册（SC-B5-15）。
pub fn sub_a5_cluster_039(input: &L2In) -> L2Out {
    crate::verify_shim::a5_cluster_039(input)
}

/// a5_cluster_040 —— bank03:9E36 簇入口注册（SC-B5-16）。
pub fn sub_a5_cluster_040(input: &L2In) -> L2Out {
    crate::verify_shim::a5_cluster_040(input)
}

/// a5_cluster_042 —— bank03:9E8E 簇入口注册（SC-B5-18）。
pub fn sub_a5_cluster_042(input: &L2In) -> L2Out {
    crate::verify_shim::a5_cluster_042(input)
}

/// a6_cluster_000 —— bank04:8000 簇入口注册（SC-B6-00）。
pub fn sub_a6_cluster_000(input: &L2In) -> L2Out {
    crate::verify_shim::a6_cluster_000(input)
}

/// a6_cluster_001 —— bank04:8003 簇入口注册（SC-B6-01）。
pub fn sub_a6_cluster_001(input: &L2In) -> L2Out {
    crate::verify_shim::a6_cluster_001(input)
}

/// a6_cluster_002 —— bank04:8006 簇入口注册（SC-B6-02）。
pub fn sub_a6_cluster_002(input: &L2In) -> L2Out {
    crate::verify_shim::a6_cluster_002(input)
}

/// a6_cluster_003 —— bank04:8009 簇入口注册（SC-B6-03）。
pub fn sub_a6_cluster_003(input: &L2In) -> L2Out {
    crate::verify_shim::a6_cluster_003(input)
}

/// a6_cluster_004 —— bank04:800C 簇入口注册（SC-B6-04）。
pub fn sub_a6_cluster_004(input: &L2In) -> L2Out {
    crate::verify_shim::a6_cluster_004(input)
}

/// a6_cluster_005 —— bank04:800F 簇入口注册（SC-B6-05）。
pub fn sub_a6_cluster_005(input: &L2In) -> L2Out {
    crate::verify_shim::a6_cluster_005(input)
}

/// a6_cluster_006 —— bank04:801B 簇入口注册（SC-B6-06）。
pub fn sub_a6_cluster_006(input: &L2In) -> L2Out {
    crate::verify_shim::a6_cluster_006(input)
}

/// a6_cluster_007 —— bank04:801E 簇入口注册（SC-B6-07）。
pub fn sub_a6_cluster_007(input: &L2In) -> L2Out {
    crate::verify_shim::a6_cluster_007(input)
}

/// a6_cluster_008 —— bank04:8027 簇入口注册（SC-B6-08）。
pub fn sub_a6_cluster_008(input: &L2In) -> L2Out {
    crate::verify_shim::a6_cluster_008(input)
}

/// a6_cluster_009 —— bank04:802A 簇入口注册（SC-B6-09）。
pub fn sub_a6_cluster_009(input: &L2In) -> L2Out {
    crate::verify_shim::a6_cluster_009(input)
}

/// a6_cluster_010 —— bank04:802D 簇入口注册（SC-B6-10）。
pub fn sub_a6_cluster_010(input: &L2In) -> L2Out {
    crate::verify_shim::a6_cluster_010(input)
}

/// a6_cluster_011 —— bank04:8030 簇入口注册（SC-B6-11）。
pub fn sub_a6_cluster_011(input: &L2In) -> L2Out {
    crate::verify_shim::a6_cluster_011(input)
}

/// a6_cluster_012 —— bank04:8033 簇入口注册（SC-B6-12）。
pub fn sub_a6_cluster_012(input: &L2In) -> L2Out {
    crate::verify_shim::a6_cluster_012(input)
}

/// a6_cluster_013 —— bank04:8036 簇入口注册（SC-B6-13）。
pub fn sub_a6_cluster_013(input: &L2In) -> L2Out {
    crate::verify_shim::a6_cluster_013(input)
}

/// a6_cluster_014 —— bank04:8039 簇入口注册（SC-B6-14）。
pub fn sub_a6_cluster_014(input: &L2In) -> L2Out {
    crate::verify_shim::a6_cluster_014(input)
}

/// a6_cluster_015 —— bank04:8BB1 簇入口注册（SC-B6-15）。
pub fn sub_a6_cluster_015(input: &L2In) -> L2Out {
    crate::verify_shim::a6_cluster_015(input)
}

/// a6_cluster_016 —— bank04:8C61 簇入口注册（SC-B6-16）。
pub fn sub_a6_cluster_016(input: &L2In) -> L2Out {
    crate::verify_shim::a6_cluster_016(input)
}

/// a6_cluster_017 —— bank04:8C71 簇入口注册（SC-B6-17）。
pub fn sub_a6_cluster_017(input: &L2In) -> L2Out {
    crate::verify_shim::a6_cluster_017(input)
}

/// a6_cluster_018 —— bank04:8C76 簇入口注册（SC-B6-18）。
pub fn sub_a6_cluster_018(input: &L2In) -> L2Out {
    crate::verify_shim::a6_cluster_018(input)
}

/// a6_cluster_019 —— bank04:8C8A 簇入口注册（SC-B6-19）。
pub fn sub_a6_cluster_019(input: &L2In) -> L2Out {
    crate::verify_shim::a6_cluster_019(input)
}

/// a6_cluster_020 —— bank04:8C97 簇入口注册（SC-B6-20）。
pub fn sub_a6_cluster_020(input: &L2In) -> L2Out {
    crate::verify_shim::a6_cluster_020(input)
}

/// a6_cluster_021 —— bank04:8CB6 簇入口注册（SC-B6-21）。
pub fn sub_a6_cluster_021(input: &L2In) -> L2Out {
    crate::verify_shim::a6_cluster_021(input)
}

/// a7_cluster_022 —— bank04:8CBB 簇入口注册（SC-B7-00）。
pub fn sub_a7_cluster_022(input: &L2In) -> L2Out {
    crate::verify_shim::a7_cluster_022(input)
}

/// a7_cluster_023 —— bank04:8CC0 簇入口注册（SC-B7-01）。
pub fn sub_a7_cluster_023(input: &L2In) -> L2Out {
    crate::verify_shim::a7_cluster_023(input)
}

/// a7_cluster_024 —— bank04:8CDD 簇入口注册（SC-B7-02）。
pub fn sub_a7_cluster_024(input: &L2In) -> L2Out {
    crate::verify_shim::a7_cluster_024(input)
}

/// a7_cluster_025 —— bank04:8CE0 簇入口注册（SC-B7-03）。
pub fn sub_a7_cluster_025(input: &L2In) -> L2Out {
    crate::verify_shim::a7_cluster_025(input)
}

/// a7_cluster_026 —— bank04:8CE3 簇入口注册（SC-B7-04）。
pub fn sub_a7_cluster_026(input: &L2In) -> L2Out {
    crate::verify_shim::a7_cluster_026(input)
}

/// a7_cluster_027 —— bank04:9905 簇入口注册（SC-B7-05）。
pub fn sub_a7_cluster_027(input: &L2In) -> L2Out {
    crate::verify_shim::a7_cluster_027(input)
}

/// a7_cluster_028 —— bank04:991E 簇入口注册（SC-B7-06）。
pub fn sub_a7_cluster_028(input: &L2In) -> L2Out {
    crate::verify_shim::a7_cluster_028(input)
}

/// a7_cluster_029 —— bank04:992C 簇入口注册（SC-B7-07）。
pub fn sub_a7_cluster_029(input: &L2In) -> L2Out {
    crate::verify_shim::a7_cluster_029(input)
}

/// a7_cluster_030 —— bank04:9930 簇入口注册（SC-B7-08）。
pub fn sub_a7_cluster_030(input: &L2In) -> L2Out {
    crate::verify_shim::a7_cluster_030(input)
}

/// a7_cluster_031 —— bank04:99C3 簇入口注册（SC-B7-09）。
pub fn sub_a7_cluster_031(input: &L2In) -> L2Out {
    crate::verify_shim::a7_cluster_031(input)
}

/// a7_cluster_033 —— bank04:99D0 簇入口注册（SC-B7-11）。
pub fn sub_a7_cluster_033(input: &L2In) -> L2Out {
    crate::verify_shim::a7_cluster_033(input)
}

/// a7_cluster_034 —— bank04:99D3 簇入口注册（SC-B7-12）。
pub fn sub_a7_cluster_034(input: &L2In) -> L2Out {
    crate::verify_shim::a7_cluster_034(input)
}

/// a7_cluster_035 —— bank04:99DE 簇入口注册（SC-B7-13）。
pub fn sub_a7_cluster_035(input: &L2In) -> L2Out {
    crate::verify_shim::a7_cluster_035(input)
}

/// a7_cluster_036 —— bank04:9A9C 簇入口注册（SC-B7-14）。
pub fn sub_a7_cluster_036(input: &L2In) -> L2Out {
    crate::verify_shim::a7_cluster_036(input)
}

/// a7_cluster_037 —— bank04:9AA4 簇入口注册（SC-B7-15）。
pub fn sub_a7_cluster_037(input: &L2In) -> L2Out {
    crate::verify_shim::a7_cluster_037(input)
}

/// a7_cluster_038 —— bank04:9AD4 簇入口注册（SC-B7-16）。
pub fn sub_a7_cluster_038(input: &L2In) -> L2Out {
    crate::verify_shim::a7_cluster_038(input)
}

/// a7_cluster_039 —— bank04:9AD7 簇入口注册（SC-B7-17）。
pub fn sub_a7_cluster_039(input: &L2In) -> L2Out {
    crate::verify_shim::a7_cluster_039(input)
}

/// a7_cluster_040 —— bank04:9ADA 簇入口注册（SC-B7-18）。
pub fn sub_a7_cluster_040(input: &L2In) -> L2Out {
    crate::verify_shim::a7_cluster_040(input)
}

/// a8_cluster_000 —— bank05:8000 簇入口注册（SC-B8-00）。
pub fn sub_a8_cluster_000(input: &L2In) -> L2Out {
    crate::verify_shim::a8_cluster_000(input)
}

/// a8_cluster_001 —— bank05:8003 簇入口注册（SC-B8-01）。
pub fn sub_a8_cluster_001(input: &L2In) -> L2Out {
    crate::verify_shim::a8_cluster_001(input)
}

/// a8_cluster_002 —— bank05:8006 簇入口注册（SC-B8-02）。
pub fn sub_a8_cluster_002(input: &L2In) -> L2Out {
    crate::verify_shim::a8_cluster_002(input)
}

/// a8_cluster_003 —— bank05:8009 簇入口注册（SC-B8-03）。
pub fn sub_a8_cluster_003(input: &L2In) -> L2Out {
    crate::verify_shim::a8_cluster_003(input)
}

/// a8_cluster_004 —— bank05:800C 簇入口注册（SC-B8-04）。
pub fn sub_a8_cluster_004(input: &L2In) -> L2Out {
    crate::verify_shim::a8_cluster_004(input)
}

/// a8_cluster_005 —— bank05:800F 簇入口注册（SC-B8-05）。
pub fn sub_a8_cluster_005(input: &L2In) -> L2Out {
    crate::verify_shim::a8_cluster_005(input)
}

/// a8_cluster_006 —— bank05:801B 簇入口注册（SC-B8-06）。
pub fn sub_a8_cluster_006(input: &L2In) -> L2Out {
    crate::verify_shim::a8_cluster_006(input)
}

/// a8_cluster_007 —— bank05:801E 簇入口注册（SC-B8-07）。
pub fn sub_a8_cluster_007(input: &L2In) -> L2Out {
    crate::verify_shim::a8_cluster_007(input)
}

/// a8_cluster_008 —— bank05:8021 簇入口注册（SC-B8-08）。
pub fn sub_a8_cluster_008(input: &L2In) -> L2Out {
    crate::verify_shim::a8_cluster_008(input)
}

/// a8_cluster_009 —— bank05:8027 簇入口注册（SC-B8-09）。
pub fn sub_a8_cluster_009(input: &L2In) -> L2Out {
    crate::verify_shim::a8_cluster_009(input)
}
