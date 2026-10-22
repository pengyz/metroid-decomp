//! room_ram——房间数据（RoomRAM（typed 数据结构））（数据驱动架构（game-data 纯数据））
//! refs 引用必写：bank:addr 引证（A 级原文）
//! refs asm 考证：brinstar/+kraid/+norfair/（房间数据（写径实证（A 级原文）））

/// 房间数据（RoomRAM（typed 数据结构（parse_room（写径实证（A 级原文）））））
pub const ROOM_PTRS: (u8, u16) = (1, 0xA314);
/// 房间数据（RoomRAM（typed 数据结构（parse_room（写径实证（A 级原文）））））
pub const ROOM00: (u8, u16) = (1, 0xA441);
/// 结构数据（RoomRAM（typed 数据结构（parse_structure（写径实证（A 级原文）））））
pub const STRUCT_PTRS: (u8, u16) = (1, 0xA372);
/// 结构数据（RoomRAM（typed 数据结构（parse_structure（写径实证（A 级原文）））））
pub const STRUCTURE00: (u8, u16) = (1, 0xAC84);
