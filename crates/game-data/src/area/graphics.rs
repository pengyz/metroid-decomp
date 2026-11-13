//! 区域图形三件套（r25 烘焙——tools/bake_area_graphics.py 生成；**禁手抄**）。
//!
//! ROM 锚（A 级）：RoomPtrTable $A314（47 项）/StructPtrTable $A372（50 项）/
//! MetatileDefs $AEF0（4B 组）；房流/结构=$FF 自界。
//! 字节自证：`python3 -c "p=open('rom/Metroid (USA).nes','rb').read()[0x10:]; print(p[0x2314:0x2330].hex())"`




/// 房流（47 件——对象 3B 流至 $FD 后敌门流至 $FF；DrawRoom 消费）。
#[cfg(test)]
mod tests {
    // 本文件已不持有任何 ROM 派生数据：房流五件套 + 敌九表全部在资产包
    // （访问器 `crate::assets::*`）。机械判据=`data-boundary` 门禁的账本条数。
    #[test]
    fn area_graphics_module_holds_no_rom_data() {
        // 就地提醒：真正的机械判据是 data-boundary 门禁（账本条数只减不增）。
        // 搬迁完成时本文件应只剩模块注释与空壳；若再出现大数组，账本会立刻要求登记。
    }
}
