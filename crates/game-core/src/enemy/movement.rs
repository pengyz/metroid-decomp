//! `game-core::enemy::movement` —— 敌人移动检径（批量改写·第三簇 enemy::movement（六步流程））。
//!
//! 【step1 定域取证 spec】asm 内容锚（refs/m1disasm/SRC/prg7_engine.asm——A 级 bank:addr 引证）：
//! - `EnemyCheckMoveUp: ; 07:E770`（:7991）——A←radiusY+8（上方块检）
//! - `EnemyCheckMoveDown: ; 07:E77B`（:7999）——A←0−radiusY（下方块检，fallthrough）
//! - `EnemyCheckMoveVertical: ; 07:E783`（:8007）——$02←A／$04←8（redundant 保真）／
//!   StoreEnemyPositionToTemp／A←radiusX→CheckMoveVertical
//! - `StoreEnemyPositionToTemp: ; 07:E792`（:8016）——$09←Ens.x／$08←Ens.y／$0B←EnsExtra.hi
//! - `CheckMoveVertical: ; 07:E7BD`（:8044）——radiusX==0 ⇒ sec/rts（无碰撞）；else 瓦片环 kernel
//! - `EnemyCheckMoveLeft: ; 07:E8F1`（:8295）——A←radiusX+8（左方块检）
//! - `EnemyCheckMoveRight: ; 07:E8FC`（:8303）——A←0−radiusX（右方块检）
//! - `EnemyCheckMoveHorizontalBranch: ; 07:E904`（:8310）——$03←A／staging→CheckMoveHorizontal
//!
//! 【step2 域结构】MoveCheckStaging=Temp 面逐钉（$02/$03/$04/$08/$09/$0B）；
//! 【step3 memmap 适配】Ens/EnsExtra=WRAM $6Axx 敌槽（wram=addr−$6000），PageIndex=x；
//! 【step4 shim 写集适配】staging→zero-page 写集（staging_writes）；EnemyState→RAM 写回
//!   走 verify_shim 既有 WRAM 敌槽面（$6AEx 族——common.rs enemy_move_effects 同法）；
//! 【step5 测试】radius 全域穷举等价证明（下 `mod tests`）；
//! 【step6 registry 登记】l2ports.rs `s4_cluster_movement_check`（bank:addr→crate::module::fn 双射）。
//!
//! 命名纪律：语义命名经 data/naming 台账 A 级（bank:addr 注释自证）；未证面保留原 asm 名。

/// 移动检 staging（asm Temp 面逐钉——`EnemyCheckMoveVertical/HorizontalBranch`）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MoveCheckStaging {
    /// Temp02_DistToCenterY（07:E783 `sta Temp02`）
    pub dist_to_center_y: u8,
    /// Temp03_DistToCenterX（07:E904 `sta Temp03`）
    pub dist_to_center_x: u8,
    /// Temp04_NumBlocksToCheck（07:E783 `lda #$08/sta`（asm 标注 redundant——保真保留））
    pub num_blocks_to_check: u8,
    /// Temp08_PositionY（07:E792）
    pub pos_y: u8,
    /// Temp09_PositionX（07:E792）
    pub pos_x: u8,
    /// Temp0B_PositionHi（07:E792 EnsExtra.hi）
    pub pos_hi: u8,
    /// CheckMoveVertical 入参（07:E783 `lda EnsExtra.0.radiusX`）
    pub radius_x: u8,
    /// CheckMoveHorizontal 入参（07:E904 `ldy EnsExtra.0.radiusY`）
    pub radius_y: u8,
}

/// CheckMoveVertical/Horizontal 边界面（07:E7BD——radius==0 ⇒ 无碰撞；else 瓦片环碰撞查询）。
/// 【r17 消歧更名（M4 复核裁定）】NeedsBgKernel→NeedsBgCollision：旧词与帧内核
/// （frame/=kernel 义）一词二义；asm 本名=CheckMoveVertical（refs 无 kernel 词）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveCheckOutcome {
    /// radius==0 快路径（07:E7BE `bne` 不转⇒`sec/rts`——无碰撞，carry set）
    NoCollision,
    /// 需瓦片环碰撞查询（CheckMoveVertical/Horizontal——collision 簇，边界交出）
    NeedsBgCollision(MoveCheckStaging),
}

/// 上检（07:E770）：distY←radiusY+8，staging→垂直面（radiusX=E783 入参）。
pub fn enemy_check_move_up(radius_x: u8, radius_y: u8, x: u8, y: u8, hi: u8) -> MoveCheckStaging {
    enemy_check_move_vertical(radius_y.wrapping_add(8), radius_x, x, y, hi)
}

/// 下检（07:E77B）：distY←0−radiusY（sec/sbc 两位补码），staging→垂直面。
pub fn enemy_check_move_down(radius_x: u8, radius_y: u8, x: u8, y: u8, hi: u8) -> MoveCheckStaging {
    enemy_check_move_vertical(0u8.wrapping_sub(radius_y), radius_x, x, y, hi)
}

/// 垂直面（07:E783）：$02←distY／$04←8／位置 staging／radiusX 入参。
pub fn enemy_check_move_vertical(
    dist_to_center_y: u8, radius_x: u8, x: u8, y: u8, hi: u8,
) -> MoveCheckStaging {
    MoveCheckStaging {
        dist_to_center_y,
        dist_to_center_x: 0,
        num_blocks_to_check: 8,
        pos_y: y,
        pos_x: x,
        pos_hi: hi,
        radius_x,
        radius_y: 0,
    }
}

/// 左检（07:E8F1）：distX←radiusX+8，staging→水平面。
pub fn enemy_check_move_left(radius_x: u8, radius_y: u8, x: u8, y: u8, hi: u8) -> MoveCheckStaging {
    enemy_check_move_horizontal_branch(radius_x.wrapping_add(8), radius_y, x, y, hi)
}

/// 右检（07:E8FC）：distX←0−radiusX（sec/sbc），staging→水平面。
pub fn enemy_check_move_right(radius_x: u8, radius_y: u8, x: u8, y: u8, hi: u8) -> MoveCheckStaging {
    enemy_check_move_horizontal_branch(0u8.wrapping_sub(radius_x), radius_y, x, y, hi)
}

/// 水平面（07:E904）：$03←distX／位置 staging／radiusY 入参。
pub fn enemy_check_move_horizontal_branch(
    dist_to_center_x: u8, radius_y: u8, x: u8, y: u8, hi: u8,
) -> MoveCheckStaging {
    MoveCheckStaging {
        dist_to_center_y: 0,
        dist_to_center_x,
        num_blocks_to_check: 8,
        pos_y: y,
        pos_x: x,
        pos_hi: hi,
        radius_x: 0,
        radius_y,
    }
}

/// 边界裁定（07:E7BD）：radius==0 ⇒ 无碰撞；else 交瓦片环 kernel（collision 簇边界）。
pub fn check_move_boundary(staging: MoveCheckStaging, horizontal: bool) -> MoveCheckOutcome {
    let radius = if horizontal { staging.radius_y } else { staging.radius_x };
    if radius == 0 {
        MoveCheckOutcome::NoCollision
    } else {
        MoveCheckOutcome::NeedsBgCollision(staging)
    }
}

/// shim 写集适配（step4）：staging→zero-page 写集（Temp 面逐钉——golden 写面观测同构）。
pub fn staging_writes(s: &MoveCheckStaging, horizontal: bool) -> Vec<(u16, u8)> {
    let mut w = Vec::with_capacity(6);
    if horizontal {
        w.push((0x03, s.dist_to_center_x));
    } else {
        w.push((0x02, s.dist_to_center_y));
    }
    w.push((0x04, s.num_blocks_to_check));
    w.push((0x08, s.pos_y));
    w.push((0x09, s.pos_x));
    w.push((0x0B, s.pos_hi));
    w
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【step5 穷举等价证明】radius 全域 0..=255×角点位置——几何/staging 逐钉等价。
    #[test]
    fn exhaustive_geometry_up_down_left_right() {
        for r in 0u8..=255 {
            for &(x, y, hi) in &[(0u8, 0u8, 0u8), (0x5A, 0xEF, 1), (0xFF, 0x7F, 1)] {
                // Up：distY=radiusY+8（clc/adc #$08 语义=wrapping_add）
                let s = enemy_check_move_up(r, r.wrapping_add(3), x, y, hi);
                assert_eq!(s.dist_to_center_y, r.wrapping_add(3).wrapping_add(8), "up r={}", r);
                assert_eq!(s.num_blocks_to_check, 8);
                assert_eq!((s.pos_x, s.pos_y, s.pos_hi), (x, y, hi));
                // Down：distY=0−radiusY（lda #$00/sec/sbc 语义=wrapping_sub）
                let s = enemy_check_move_down(r, r, x, y, hi);
                assert_eq!(s.dist_to_center_y, 0u8.wrapping_sub(r), "down r={}", r);
                // Left：distX=radiusX+8
                let s = enemy_check_move_left(r, r, x, y, hi);
                assert_eq!(s.dist_to_center_x, r.wrapping_add(8), "left r={}", r);
                // Right：distX=0−radiusX
                let s = enemy_check_move_right(r, r, x, y, hi);
                assert_eq!(s.dist_to_center_x, 0u8.wrapping_sub(r), "right r={}", r);
            }
        }
    }

    /// 边界面穷举：radius==0 ⇒ 无碰撞（07:E7BE 快路径）；else 交 kernel。
    #[test]
    fn exhaustive_boundary_radius0_fastpath() {
        for r in 0u8..=255 {
            let s = enemy_check_move_up(r, 7, 1, 2, 3);
            match check_move_boundary(s, false) {
                MoveCheckOutcome::NoCollision => assert_eq!(r, 0),
                MoveCheckOutcome::NeedsBgCollision(t) => {
                    assert_ne!(r, 0);
                    assert_eq!(t, s);
                }
            }
            let s = enemy_check_move_left(7, r, 1, 2, 3);
            match check_move_boundary(s, true) {
                MoveCheckOutcome::NoCollision => assert_eq!(r, 0),
                MoveCheckOutcome::NeedsBgCollision(t) => assert_eq!(t, s),
            }
        }
    }

    /// shim 写集：Temp 面逐钉（$02/$03/$04/$08/$09/$0B）。
    #[test]
    fn staging_writes_temp_face() {
        let s = enemy_check_move_up(0x10, 0x10, 0x22, 0x33, 1);
        let w = staging_writes(&s, false);
        assert!(w.contains(&(0x02, 0x18))); // distY=radiusY(0x10)+8
        assert!(w.contains(&(0x04, 8)));
        assert!(w.contains(&(0x08, 0x33)));
        assert!(w.contains(&(0x09, 0x22)));
        assert!(w.contains(&(0x0B, 1)));
    }
}
