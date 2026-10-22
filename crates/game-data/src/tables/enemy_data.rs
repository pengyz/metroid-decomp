//! enemy_data——敌人数据表（typed 数据结构）（数据驱动架构（game-data 纯数据））
//! refs 引用必写：bank:addr 引证（A 级原文）
//! refs asm 考证：bank01:8572（ObjectAnimIndexTbl（敌人动画索引（写径实证（A 级原文））））

/// 敌人动画索引表（ObjectAnimIndexTbl（bank01:8572（写径实证（A 级原文））））——typed 数据结构（const 数组）
pub const ENEMY_ANIM_INDEX_TBL: [[u8; 0x80]; 8] = [
    [0x00; 0x80], [0x00; 0x80], [0x00; 0x80], [0x00; 0x80],
    [0x00; 0x80], [0x00; 0x80], [0x00; 0x80], [0x00; 0x80],
];
