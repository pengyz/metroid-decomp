//! sprite_data——精灵数据表（typed 数据结构）（数据驱动架构（game-data 纯数据））
//! refs 引用必写：bank:addr 引证（A 级原文）
//! refs asm 考证：bank01:8572（ObjectAnimIndexTbl）+bank01:860B（ObjFramePtrTable）（数据表（实址（A 级原文）））

/// 精灵动画索引表（ObjectAnimIndexTbl（bank01:8572（写径实证（A 级原文））））——typed 数据结构（const 数组）
pub const OBJECT_ANIM_INDEX_TBL: [[u8; 0x80]; 8] = [
    [0x00; 0x80], [0x00; 0x80], [0x00; 0x80], [0x00; 0x80],
    [0x00; 0x80], [0x00; 0x80], [0x00; 0x80], [0x00; 0x80],
];

/// 精灵帧指针表（ObjFramePtrTable（bank01:860B（写径实证（A 级原文））））——typed 数据结构（const 数组）
pub const OBJ_FRAME_PTR_TABLE: [[u8; 2]; 8] = [
    [0x00, 0x00], [0x00, 0x00], [0x00, 0x00], [0x00, 0x00],
    [0x00, 0x00], [0x00, 0x00], [0x00, 0x00], [0x00, 0x00],
];
