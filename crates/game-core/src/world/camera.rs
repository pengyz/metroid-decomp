//! `game-core::world::camera` —— 相机移动&地形刷新（行为等价）。
//!
//! refs 考证（bank:addr（A 级原文））：SamusMoveHorizontally（bank07:E0C2（萨姆斯水平移动））
//! +ScrollDir（bank07:0028（滚动方向））+MakeRoomRAMPtr（bank07:E96A（房间 RAM 指针计算））。

#[cfg(test)]
mod tests_camera {
    //! TDD step1（穷举等价证明（全域穷举（边界条件（异常路径））））。
    //! 判径（任务规约面）：camera（samus_x>camera_x⇒+1/<⇒-1/=⇒不变）；
    //! terrain（0x20 行写（(0x2000+i, camera_x+i)（wrapping）））。

    use super::*;

    /// 独立参考径（证伪立场）：双面公式直序。
    fn ref_camera(samus_x: u8, camera_x: u8) -> u8 {
        if samus_x > camera_x {
            camera_x.wrapping_add(1)
        } else if samus_x < camera_x {
            camera_x.wrapping_sub(1)
        } else {
            camera_x
        }
    }
    fn ref_terrain(camera_x: u8) -> Vec<(u16, u8)> {
        (0..0x20u16).map(|i| (0x2000 + i, camera_x.wrapping_add(i as u8))).collect()
    }

    /// 【穷举·相机】samus_x×camera_x 全域 256² 逐位对拍。
    #[test]
    fn exhaustive_camera_chase() {
        for s in 0u8..=255 {
            for c in 0u8..=255 {
                assert_eq!(camera_move(s, c), ref_camera(s, c), "s={} c={}", s, c);
            }
        }
    }

    /// 【边界·相机】回绕面（samus=0/camera=255（追+1⇒0）/samus=255/camera=0（退-1⇒255））
    /// +相等面（不变）。
    #[test]
    fn camera_boundary() {
        // 【证伪自纠】u8 序判（非环绕序）：(0,255)⇒0<255⇒退 254/(255,0)⇒255>0⇒追 1
        assert_eq!(camera_move(0, 255), 254, "u8 序退径（255-1=254）");
        assert_eq!(camera_move(255, 0), 1, "u8 序追径（0+1=1）");
        assert_eq!(camera_move(0, 0), 0, "零位相等");
        assert_eq!(camera_move(255, 255), 255, "极位相等");
        assert_eq!(camera_move(42, 42), 42, "相等⇒不变");
    }

    /// 【穷举·地形】camera_x 全域 256——0x20 行写全域对拍（址/值/序）+回绕面
    /// （camera_x=255（值 255,0,1,…））。
    #[test]
    fn exhaustive_terrain_refresh() {
        for c in 0u8..=255 {
            let w = terrain_refresh(c);
            let r = ref_terrain(c);
            assert_eq!(w, r, "camera_x={}", c);
            assert_eq!(w.len(), 0x20, "行写长恒 0x20");
            assert_eq!(w[0], (0x2000, c), "首址（$2000）");
            assert_eq!(w[0x1F], (0x201F, c.wrapping_add(0x1F)), "末址（$201F）");
        }
    }

    /// 【异常路径】terrain 回绕值序（camera_x=255⇒255,0,1,…（wrapping 加））。
    #[test]
    fn terrain_wrap_sequence() {
        let w = terrain_refresh(255);
        assert_eq!(w[0].1, 255);
        assert_eq!(w[1].1, 0);
        assert_eq!(w[2].1, 1);
    }
}

// ===== 相机移动&地形刷新（实现（真规则（公式形（零字面特值））））=====
// refs 考证（bank:addr（A 级原文））：SamusMoveHorizontally（bank07:E0C2）+ScrollDir
// （bank07:0028（滚动方向））+MakeRoomRAMPtr（bank07:E96A）。

/// 相机移动（行为等价）——samus_x>camera_x⇒camera+1；<⇒camera-1；=⇒不变（wrapping
/// （追随面（ScrollDir 07:0028 滚动方向对偶）））。
pub fn camera_move(samus_x: u8, camera_x: u8) -> u8 {
    if samus_x > camera_x {
        camera_x.wrapping_add(1)
    } else if samus_x < camera_x {
        camera_x.wrapping_sub(1)
    } else {
        camera_x
    }
}

/// 地形刷新（行为等价）——0x20 行写（(0x2000+i, camera_x+i)（wrapping）
/// （MakeRoomRAMPtr 07:E96A 指针算面））。
pub fn terrain_refresh(camera_x: u8) -> Vec<(u16, u8)> {
    let mut writes = Vec::with_capacity(0x20);
    for i in 0..0x20u16 {
        writes.push((0x2000 + i, camera_x.wrapping_add(i as u8)));
    }
    writes
}
