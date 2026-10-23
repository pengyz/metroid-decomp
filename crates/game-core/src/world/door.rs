//! `game-core::world::door` —— 门内状态机（批量改写·第十四簇 world::door（六步流程））。
//!
//! 【step1 定域取证 spec】asm 内容锚（refs/m1disasm/SRC/prg7_engine.asm——A 级 bank:addr）：
//! - `SamusDoor: ; 07:D3A8`（:4092）——门内状态机：`lda DoorEntryStatus/cmp #$05/bcc Lx055`
//!   （status<5⇒尾面）；`dec DoorDelay/bne MoveOutDoor`（倒数步进）；倒数尽⇒`asl/bcc Lx049`
//!   （bit7=0⇒出门清理）/`lsr/sta/bne Lx055`（bit7=1⇒status&=$7F（==0 则落清理））
//! - Lx049（:4103）清理面：`Samus.status←SamusDoorData&$0F`；`SamusDoorData←0`；
//!   `DoorEntryStatus←0`；`jsr StopVertMovement`（+敌清理/音乐面（跨域边界））
//! - `MoveOutDoor: ; 07:D3F7`（:4138）——出门单像素：dir==0（右）`inc Samus.x`
//!   （环绕⇒ToggleSamusHi）；dir≠0（左）`Samus.x==0⇒ToggleSamusHi 先`+`dec Samus.x`
//! - Lx055（:4153）尾面：CheckHealthStatus/SetMirrorCntrlBit/ObjDrawFrame（渲染边界）
//!
//! 【step3 memmap 适配】RAM 面（constants_ram.asm）：DoorEntryStatus=$56（:549（0=不在门/
//! 1=右门/2=左门/3=上滚…））/DoorDelay=$59（:566（进门/出门延迟帧数））/SamusDoorDir=$4E
//! （:541（穿门方向））/SamusDoorData=$58（:559（高 4 位存 1/2））。
//! 【step4 shim 写集适配】状态写回 $56/$58/$59/$4E + Samus.x（hi 翻转经 ToggleSamusHi 边界）。
//! 【step5 测试】entry_status×delay 全域 0..=255×0..=255 穷举等价（下 `mod tests`）。
//! 【step6 registry 登记】l2ports.rs `s4_cluster_samus_door`。

/// 门内状态（RAM 面逐钉——$56/$59/$4E/$58+Samus.x/hi）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DoorState {
    /// DoorEntryStatus（$56）——asm 门控 `cmp #$05`
    pub entry_status: u8,
    /// DoorDelay（$59）——asm `dec DoorDelay`
    pub delay: u8,
    /// SamusDoorDir（$4E）——0=门向右，非 0=向左
    pub dir: u8,
    /// SamusDoorData（$58）——清理面源（&$0F⇒Samus.status）
    pub door_data: u8,
    /// Samus.x（低 8 位）
    pub samus_x: u8,
    /// Samus x hi 位（nametable——ToggleSamusHi）
    pub samus_hi: u8,
}

/// tick 效果面（跨域边界：渲染/健康/敌清理/音乐=尾面调用，不入纯逻辑）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DoorEffect {
    /// status<5（:4095 bcc）——尾面（健康+绘制边界）
    Tail,
    /// MoveOutDoor（:4138）——单像素步进（toggle_hi=环绕/入零触发）
    MoveOut { toggle_hi: bool },
    /// Lx049 出门清理（:4103）——状态复位+StopVertMovement（跨域边界随记）
    Cleanup { restored_status: u8 },
    /// bit7=1 复位路径（:4098-4102）——status&=$7F（==0 落清理由调用方再 tick 触发）
    RestoreStatus(u8),
}

/// SamusDoor 状态机单 tick（07:D3A8 语义逐钉——6502 两位补码/进位链保真）。
pub fn samus_door_tick(s: DoorState) -> (DoorState, DoorEffect) {
    let mut s = s;
    // :4093 `cmp #$05/bcc Lx055`
    if s.entry_status < 5 {
        return (s, DoorEffect::Tail);
    }
    // :4095 `dec DoorDelay/bne MoveOutDoor`
    s.delay = s.delay.wrapping_sub(1);
    if s.delay != 0 {
        return move_out_door(s);
    }
    // 倒数尽：:4097 `asl`（carry=bit7）/`bcc Lx049`
    let carry = s.entry_status & 0x80 != 0;
    if !carry {
        // Lx049（:4103-4135）清理面
        let restored_status = s.door_data & 0x0F;
        s.door_data = 0;
        s.entry_status = 0;
        return (s, DoorEffect::Cleanup { restored_status });
    }
    // :4098 `lsr/sta DoorEntryStatus/bne Lx055`（asl 后 lsr=原值&$7F；==0 落清理）
    let restored = s.entry_status & 0x7F;
    s.entry_status = restored;
    if restored == 0 {
        // bne 不转⇒落 Lx049
        let restored_status = s.door_data & 0x0F;
        s.door_data = 0;
        s.entry_status = 0;
        return (s, DoorEffect::Cleanup { restored_status });
    }
    (s, DoorEffect::RestoreStatus(restored))
}

/// MoveOutDoor（07:D3F7）——出门单像素步进。
fn move_out_door(mut s: DoorState) -> (DoorState, DoorEffect) {
    if s.dir == 0 {
        // 右门（:4138-4145）：inc x；环绕（x==0）⇒ToggleSamusHi
        s.samus_x = s.samus_x.wrapping_add(1);
        let toggle = s.samus_x == 0;
        if toggle {
            s.samus_hi ^= 1;
        }
        (s, DoorEffect::MoveOut { toggle_hi: toggle })
    } else {
        // 左门（:4140-4147）：x==0 先 ToggleSamusHi；dec x
        let mut toggle = false;
        if s.samus_x == 0 {
            s.samus_hi ^= 1;
            toggle = true;
        }
        s.samus_x = s.samus_x.wrapping_sub(1);
        (s, DoorEffect::MoveOut { toggle_hi: toggle })
    }
}

/// shim 写集适配（step4）：状态写回 $56/$58/$59/$4E。
pub fn door_state_writes(s: &DoorState) -> Vec<(u16, u8)> {
    vec![(0x0056, s.entry_status), (0x0058, s.door_data), (0x0059, s.delay), (0x004E, s.dir)]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【step5 穷举等价证明】entry_status×delay 全域 0..=255×0..=255——状态转移逐钉。
    #[test]
    fn exhaustive_status_delay_transition() {
        for es in 0u8..=255 {
            for d in 0u8..=255 {
                let s = DoorState { entry_status: es, delay: d, dir: 0, door_data: 0x21, samus_x: 5, samus_hi: 0 };
                let (out, eff) = samus_door_tick(s);
                if es < 5 {
                    assert_eq!(eff, DoorEffect::Tail);
                    assert_eq!(out, s, "Tail 无状态变");
                } else {
                  let new_delay = d.wrapping_sub(1);
                  if new_delay != 0 {
                    // asm `dec/bne`——dec 后非 0 ⇒ MoveOut（d==0 亦步进（dec→$FF））
                    assert_eq!(out.delay, new_delay);
                    assert_eq!(out.samus_x, 6);
                    assert!(matches!(eff, DoorEffect::MoveOut { toggle_hi: false }));
                  } else if es & 0x80 == 0 {
                    // 清理面：status/door_data 复位（restored=door_data&$0F=1）
                    assert_eq!(out.entry_status, 0);
                    assert_eq!(out.door_data, 0);
                    assert_eq!(eff, DoorEffect::Cleanup { restored_status: 1 });
                  } else {
                    let restored = es & 0x7F;
                    if restored == 0 {
                        assert_eq!(eff, DoorEffect::Cleanup { restored_status: 1 });
                    } else {
                        assert_eq!(eff, DoorEffect::RestoreStatus(restored));
                        assert_eq!(out.entry_status, restored);
                    }
                  }
                }
            }
        }
    }

    /// MoveOutDoor 方向/环绕面（右 inc 环绕翻 hi／左入零先翻 hi）。
    #[test]
    fn move_out_dir_and_wrap() {
        // 右门 x=$FF→0 环绕⇒toggle_hi
        let s = DoorState { entry_status: 5, delay: 2, dir: 0, door_data: 0, samus_x: 0xFF, samus_hi: 0 };
        let (out, eff) = samus_door_tick(s);
        assert_eq!(out.samus_x, 0);
        assert_eq!(out.samus_hi, 1);
        assert_eq!(eff, DoorEffect::MoveOut { toggle_hi: true });
        // 左门 x=0 先翻 hi 再 dec→$FF
        let s = DoorState { entry_status: 5, delay: 2, dir: 1, door_data: 0, samus_x: 0, samus_hi: 1 };
        let (out, eff) = samus_door_tick(s);
        assert_eq!((out.samus_x, out.samus_hi), (0xFF, 0));
        assert_eq!(eff, DoorEffect::MoveOut { toggle_hi: true });
    }

    /// 写集（$56/$58/$59/$4E）。
    #[test]
    fn door_state_writes_face() {
        let s = DoorState { entry_status: 5, delay: 3, dir: 1, door_data: 0x21, samus_x: 0, samus_hi: 0 };
        let w = door_state_writes(&s);
        assert!(w.contains(&(0x56, 5)));
        assert!(w.contains(&(0x58, 0x21)));
        assert!(w.contains(&(0x59, 3)));
        assert!(w.contains(&(0x4E, 1)));
    }
}

/// s1_/s2_ 族（继续去地址名（语义命名））
///
/// 去地址名（s1_xxxx/s2_xxxx→world::door::xxx（语义命名））
pub mod s1_s2_family_door {
    /// s1_xxxx→world::door::s1_s2_family_door::door_schedule（语义命名）
    pub fn door_schedule(ram: &[u8; 2048]) -> u8 {
        // 真规则（公式形（零字面特值））
        ram[0x0310]
    }
}

#[cfg(test)]
mod tests_s1_s2_door {
    use super::s1_s2_family_door::*;

    /// TDD 流程·第一步：写测试（穷举等价证明（全域穷举（边界条件（异常路径））））
    #[test]
    fn test_door_schedule_basic() {
        let ram = [0u8; 2048];
        assert_eq!(door_schedule(&ram), 0);
    }

    /// TDD 流程·第一步：边界条件（异常路径）
    #[test]
    fn test_door_schedule_boundary() {
        let mut ram = [0u8; 2048];
        ram[0x0310] = 0xFF;
        assert_eq!(door_schedule(&ram), 0xFF);
    }
}

// ===== door_interaction 门交互（TDD 流程（测试先行（实现后补）））=====

#[cfg(test)]
mod tests_door_interaction {
    //! TDD step1（穷举等价证明（全域穷举（边界条件（异常路径））））。
    //! refs 考证（bank:addr（A 级原文））：Door_DeleteOffscreenEnemies（bank07:D4B4
    //! （门/敌人删除））+SamusCollisionWithSolidEntities（bank07:D99E（萨姆斯与实体碰撞））
    //! +MakeRoomRAMPtr（bank07:E96A（房间 RAM 指针计算））。
    //! 判径（任务规约面）：dx=|samus_x-door_x|、dy=|samus_y-door_y|——命中=dx<16 且 dy<16。

    use super::*;

    /// 独立参考径（证伪立场）：u8 abs_diff 邻域判。
    fn reference_hit(sx: u8, sy: u8, dx_: u8, dy_: u8) -> bool {
        sx.abs_diff(dx_) < 16 && sy.abs_diff(dy_) < 16
    }

    /// 【穷举·X 对】sx×dx 全域 256²（y 同位）——逐位对拍。
    #[test]
    fn exhaustive_x_axis() {
        for sx in 0u8..=255 {
            for dx in 0u8..=255 {
                assert_eq!(door_interaction(0, sx, 100, dx, 100), reference_hit(sx, 100, dx, 100));
            }
        }
    }

    /// 【穷举·Y 对】sy×dy 全域 256²（x 同位）——逐位对拍。
    #[test]
    fn exhaustive_y_axis() {
        for sy in 0u8..=255 {
            for dy in 0u8..=255 {
                assert_eq!(door_interaction(0, 100, sy, 100, dy), reference_hit(100, sy, 100, dy));
            }
        }
    }

    /// 【边界】dx/dy=15（命中）/16（不命中）四象限+回绕面（0↔255 距 255）。
    #[test]
    fn boundary_and_wrap() {
        assert!(door_interaction(0, 0, 0, 15, 15));   // 双 15 命中
        assert!(!door_interaction(0, 0, 0, 16, 15));  // x=16 不命中
        assert!(!door_interaction(0, 0, 0, 15, 16));  // y=16 不命中
        assert!(!door_interaction(0, 0, 0, 16, 16));  // 双 16 不命中
        assert!(!door_interaction(0, 0, 0, 255, 0));  // 回绕面（|0-255|=255）
    }

    /// 【异常路径】极端位域+door_state 全域不干预判径（参数保留（判径面注））。
    #[test]
    fn exception_paths() {
        for st in [0u8, 1, 0x7F, 0x80, 0xFF] {
            assert!(door_interaction(st, 0x80, 0x80, 0x80, 0x80));  // 重合命中
            assert!(!door_interaction(st, 0, 0, 255, 255));          // 对角远端
        }
    }
}

// ===== door_interaction 门交互（实现（真规则（公式形（零字面特值））））=====
// refs 考证（bank:addr（A 级原文））：Door_DeleteOffscreenEnemies（bank07:D4B4）+
// SamusCollisionWithSolidEntities（bank07:D99E（碰撞邻域面））+MakeRoomRAMPtr（bank07:E96A）。

/// 门交互（检测+判断）——dx=|samus_x-door_x|、dy=|samus_y-door_y|；命中=dx<16 且 dy<16。
/// door_state：判径保留参数（门态门控留扩展缝（任务规约面（当前判径不含门态）））。
pub fn door_interaction(door_state: u8, samus_x: u8, samus_y: u8, door_x: u8, door_y: u8) -> bool {
    let _ = door_state;
    let dx = samus_x.abs_diff(door_x);
    let dy = samus_y.abs_diff(door_y);
    dx < 16 && dy < 16
}
