//! 区域图形域（r25——三件套烘焙面）。
pub mod graphics;
pub mod sprite_data; // r25 烘焙（sprite blob——AnimDrawObject 面）
pub mod items; // 特项表烘焙（SpecItmsTbl——ScanForItems/吃能力面）
pub mod norfair_graphics; // r28 烘焙（Norfair 房流五件套——bake_area_graphics.py 生成）

/// 房间图形五件套按区分发（**asm 形**：SetupRoom `lda (AreaPointers_RAM.RoomPtrTable),y`
/// 经 $3B/$3C 间接、DrawObject `lda (AreaPointers_RAM.StructPtrTable),y` 经 $3D/$3E——
/// 基址拷自各区 `$959A`（AreaPointers，14B 七指针）；本层把「间接查 ROM bank」折叠成
/// 「按 InArea&$0F 选烘焙表」，表值与 ROM 逐字节等价（提取自证见各件头注）。
///
/// 区序=BankTable（07:CA30）自证：0=Brinstar(bank1)/1=Norfair(bank2)/2=Kraid(bank4)/
/// 3=Tourian(bank3)/4=Ridley(bank5)——与 pickup::spawn::items_table 同源。
/// 未烘焙区（Kraid/Tourian/Ridley）返回 None，调用方 `unported!` fail-loud（D7）。
pub fn room_ptrs(area: u8) -> Option<&'static [u16]> {
    match area & 0x0F {
        0 => Some(&graphics::ROOM_PTRS),
        1 => Some(&norfair_graphics::ROOM_PTRS),
        _ => None,
    }
}

pub fn struct_ptrs(area: u8) -> Option<&'static [u16]> {
    match area & 0x0F {
        0 => Some(&graphics::STRUCT_PTRS),
        1 => Some(&norfair_graphics::STRUCT_PTRS),
        _ => None,
    }
}

pub fn metatile_defs(area: u8) -> Option<&'static [u8]> {
    match area & 0x0F {
        0 => Some(&graphics::METATILE_DEFS),
        1 => Some(&norfair_graphics::METATILE_DEFS),
        _ => None,
    }
}

pub fn room_streams(area: u8) -> Option<&'static [&'static [u8]]> {
    match area & 0x0F {
        0 => Some(&graphics::ROOM_STREAMS),
        1 => Some(&norfair_graphics::ROOM_STREAMS),
        _ => None,
    }
}

pub fn structures(area: u8) -> Option<&'static [&'static [u8]]> {
    match area & 0x0F {
        0 => Some(&graphics::STRUCTURES),
        1 => Some(&norfair_graphics::STRUCTURES),
        _ => None,
    }
}
