//! `game-data::area::items` —— 区特殊物品表（SpecItmsTbl）烘焙（包内 `area.*.items` 段——6 B/条记录，`extract::extract_items` 生成）。
//!
//! 【A 级 ROM 取证】refs/m1disasm 各区 items.asm `SpecItmsTbl_{AREA}` 表体直读
//! （BRINSTAR bank1:$A3D6 / NORFAIR bank2:$A2D9 / KRAID bank4:$A26D /
//! TOURIAN bank3:$A83B / RIDLEY bank5:$A20D）：
//! Y 组链表（MapY+下一组 word）+X 条目链（MapX+相对偏移（$FF 末条））+条目数据
//! （it_Type 低半字节=ChooseSpawningRoutine 分派（prg7:9322）；PowerUp=pu_XXX+位置
//! 字节；$00 终）。扁平 SpecialItem 表=Y 序链表语义等价（ScanForItems（ED98）扫描径）。

/// 特项条目（map_y/map_x=图格坐标；item_type=it_ 分派号（1=Squeept/2=PowerUp/3=Mellow/
/// 4=Elevator/5=MB 炮/6=MB/7=Zebetite/8=Rinka/9=Door/A=调色）；args=类型参数。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpecialItem {
    pub map_y: u8,
    pub map_x: u8,
    pub item_type: u8,
    pub args: [u8; 3],
}
