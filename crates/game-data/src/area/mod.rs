//! 区域图形域（r25——三件套烘焙面）。
pub mod graphics; // Brinstar 敌四表仍在此（房流五件套已迁包）
// sprite_data 已迁资产包（`assets::sprite_data`） // r25 烘焙（sprite blob——AnimDrawObject 面）
pub mod items; // 特项表烘焙（SpecItmsTbl——ScanForItems/吃能力面）

// 房流/结构/元tile 三件套**已迁资产包**：asm 形是 `lda (AreaPointers_RAM.RoomPtrTable),y`
// 经 `$3B/$3C` 间接、`lda (AreaPointers_RAM.StructPtrTable),y` 经 `$3D/$3E`——基址拷自各区
// `$959A`（AreaPointers 14B 七指针）。折叠等价：访问器按 `InArea&$0F` 选段（见 `crate::assets`）。
// 区序=BankTable（07:CA30）自证：0=Brinstar(bank1)/1=Norfair(bank2)/2=Kraid(bank4)/
// 3=Tourian(bank3)/4=Ridley(bank5)——未迁区返回 None，调用方 `unported!` fail-loud（D7）。
// 房流/结构/元tile 的五件套**已迁资产包**：访问器在 `crate::assets`（`room_ptr`/
// `room_streams`/`structures`/`metatile_defs`）。本模块只剩 ROM 派生的类型定义与旧注释。
