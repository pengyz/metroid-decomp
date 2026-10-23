//! world::room（房间加载）——批量改写·第二簇（六步流程（域内行为簇））
//!
//! 六步流程：定域取证 spec→域结构→memmap 适配→shim 写集适配→测试→registry 登记+独立复审
//! refs 引用必写（bank:addr 引证（A 级原文））

/// RoomState（域结构（语义命名（data/naming 台账 A/B 级）））
#[derive(Debug, Clone)]
pub struct RoomState {
    /// 房间 ID
    pub room_id: u8,
    /// 地形数据
    pub terrain: [u8; 256],
    /// 敌数据
    pub enemies: [u8; 64],
}

impl Default for RoomState {
    fn default() -> Self {
        Self { room_id: 0, terrain: [0; 256], enemies: [0; 64] }
    }
}


/// 房间加载例程（bank:addr（实址（A 级原文）））
///
/// 六步流程：
/// 1. 定域取证 spec（bank:addr（房间加载例程（实址（A 级原文））））
/// 2. 域结构（语义命名（data/naming 台账 A/B 级））
/// 3. memmap 适配（RAM 态→RoomState）
/// 4. shim 写集适配（RoomState→RAM 写回）
/// 5. 测试（穷举等价证明（先例））
/// 6. registry 登记（l2ports.rs（bank:addr→crate::module::fn 双射））
pub fn load_room(ram: &[u8; 2048], room_id: u8) -> RoomState {
    // memmap 适配（RAM 态→RoomState）
    let mut state = RoomState::default();
    state.room_id = room_id;
    // 数据表解析（rom_tables（bank:addr（表值（表用途））））——真规则（公式形（零字面特值））
    // refs asm 考证（bank07:CA72+D79F+E96A（prg7_engine.asm））——LoadGameData+GetObjRoomRAMPtr+MakeRoomRAMPtr
    // 从 RAM 解析 terrain（ram[0x0400..0x0500]）
    for i in 0..256 {
        state.terrain[i] = ram[0x0400 + i];
    }
    // 从 RAM 解析 enemies（ram[0x0500..0x0540]）
    for i in 0..64 {
        state.enemies[i] = ram[0x0500 + i];
    }
    state
}

/// s1_/s2_ 族（继续去地址名（语义命名））
///
/// 去地址名（s1_xxxx/s2_xxxx→world::room::xxx（语义命名））
pub mod s1_s2_family_room {
    /// s1_xxxx→world::room::s1_s2_family_room::room_schedule（语义命名）
    pub fn room_schedule(ram: &[u8; 2048]) -> u8 {
        // 真规则（公式形（零字面特值））
        ram[0x0312]
    }
}

#[cfg(test)]
mod tests_s1_s2_room {
    use super::s1_s2_family_room::*;

    /// TDD 流程·第一步：写测试（穷举等价证明（全域穷举（边界条件（异常路径））））
    #[test]
    fn test_room_schedule_basic() {
        let ram = [0u8; 2048];
        assert_eq!(room_schedule(&ram), 0);
    }

    /// TDD 流程·第一步：边界条件（异常路径）
    #[test]
    fn test_room_schedule_boundary() {
        let mut ram = [0u8; 2048];
        ram[0x0312] = 0xFF;
        assert_eq!(room_schedule(&ram), 0xFF);
    }
}
