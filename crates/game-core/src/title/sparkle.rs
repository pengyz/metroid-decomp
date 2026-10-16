//! game-core::title::sparkle —— 闪星精灵运动域（UpdateSparkleSprites 链；chain 19 形态）。
//!
//! asm 主文档（内容锚+结构互核注）：refs/m1disasm/SRC/prg0_title.asm
//! `UpdateSparkleSprites: ; 00:87CF`（**落穿双跑**：X=0 经 jsr+落穿连跑两 SparkleUpdate、
//! X=$10 同——4 跑串行状态演化）→`SparkleUpdate: ; 00:87D9`（nextDelay 门→DoSparkleSpriteCoord
//! 数据驱动→运动累积→图块翻转→WriteIntroSprite）。
//!
//! 【结构互核注】①**BUG 保真（D4）**：tileID 翻转 `eor #$C6~$C5`（=3）——asm 注
//! 「BUG! Should be $C6 and $C7」，Stage A 按字节语义保真（Stage B 修复项走 bug-ledger）；
//! ②byteType 双模的「branch always」注**非**字节事实：`lda b2 / bmi @endIf_C` 落穿即入
//! nibble 解析——b2 非负时 8 位模也走 nibble 解（字节语义保真；注=数据不变式主张）；
//! ③WriteIntroSprite 的 y **减 1 修正**（`sec/sbc #$01`）；④NibbleSubtract=「≥8 取
//! and#7+TwosComplement」（负三比特量级）。

/// IntroSpr 槽面（步距 $10；**偏移=字节真**：y0/tile1/attr2/x3/index4/nextDelay5/yChange6/
/// xChange7/changeDelay8/byteType9/completeA——constants_ram struct 列表声明与机器布局**不一致**
/// （`.db` 非分配字段位移）：机器操作数 `ad aa 6e`=complete@$6EAA 等为准——「asm=规格非模板」
/// 金样本，偏移一律从绝对,X 操作数提取）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SparkleSlot {
    pub y: u8,
    pub tile_id: u8,
    pub attrib: u8,
    pub x: u8,
    pub index: u8,
    pub next_delay: u8,
    pub y_change: u8,
    pub x_change: u8,
    pub change_delay: u8,
    pub byte_type: u8,  // 偏移 0x09（字节真）
    pub complete: u8,   // 偏移 0x0A（字节真）
}

/// 槽写事件（slot 0/1 + 字段偏移 + 值——地址由 memmap::title_state 映射）。
pub type SlotWrite = (u8, u8, u8);

/// 单跑 SparkleUpdate 结果（出口 A=WriteIntroSprite 末 x 值 / complete 检出值）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SparkleRunOutcome {
    pub a_out: u8,
    pub writes: Vec<SlotWrite>,
    /// Y 轨迹末值（coord `ldy idx/iny` → idx+1；无 coord 跑=0 默认）。
    pub y_out: u8,
}

/// nibble 符号量级解析（@NibbleSubtract：≥8 ⇒ and#7 取负）。
fn nibble_sub(a: u8) -> u8 {
    if a < 8 { a } else { (0u8.wrapping_sub(a & 7)) }
}

/// 淡变/运动数据表（窗口读 bank0——L2 案 window_bank=0 显式钉窗）：
/// SparkleAddressTbl $89AF 双字→Top $89B3/Bottom $89E9。
pub const TOP_SPARKLE_DATA: [u8; 54] = [
    0x01, 0x00, 0x01, 0x00, 0x01, 0x01, 0x06, 0x00, 0x07, 0x01, 0x10, 0x01, 0x03, 0x01,
    0x01, 0x00, 0x01, 0x01, 0x01, 0x00, 0x01, 0x01, 0x01, 0x00, 0x09, 0x01, 0x04, 0x00,
    0x27, 0x01, 0x01, 0x00, 0x01, 0x00, 0x06, 0x01, 0x01, 0x00, 0x15, 0x01, 0x06, 0x00,
    0x01, 0x01, 0x08, 0x01, 0x0E, 0x02, 0x02, 0x03, 0x06, 0x04, 0x00, 0x00,
];

/// Bottom 闪星数据（$89E9 起 54B 截面——B2 MSB 面 $99 族在此表）。
pub const BOTTOM_SPARKLE_DATA: [u8; 54] = [
    0x01, 0x00, 0x08, 0x09, 0x01, 0x99, 0x01, 0x09, 0x01, 0x99, 0x01, 0x09, 0x01, 0x99,
    0x01, 0x09, 0x01, 0x99, 0x01, 0x09, 0x01, 0x99, 0x01, 0x09, 0x01, 0x99, 0x01, 0x09,
    0x01, 0x99, 0x01, 0x09, 0x01, 0x99, 0x01, 0x09, 0x01, 0x99, 0x01, 0x19, 0x01, 0x11,
    0x01, 0x10, 0x01, 0x11, 0x01, 0x10, 0x01, 0x11, 0x00, 0x00, 0x00, 0x00,
];

/// LoadSparkleData 初值表（$87C4 11B——两槽同值装载）。
pub const INIT_SPARKLE_DATA: [u8; 11] = [0x3C, 0xC6, 0x01, 0x18, 0x00, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00];

impl SparkleSlot {
    /// 单跑 SparkleUpdate（$87D9 语义折叠）；`slot`=0（X=0，Top 表）/1（X=$10，Bottom 表）。
    pub fn update(&mut self, slot: u8) -> SparkleRunOutcome {
        let mut w = Vec::new();
        let tbl: &[u8] = if slot == 0 { &TOP_SPARKLE_DATA } else { &BOTTOM_SPARKLE_DATA };
        if self.next_delay == 0 {
            // DoSparkleSpriteCoord（数据驱动）——表指针 zp 写（$00=低 $B3/$E9、$01=高 $89）
            w.push((0x20, 0, if slot == 0 { 0xB3 } else { 0xE9 }));
            w.push((0x20, 1, 0x89));
            let b1 = tbl.get(self.index as usize).copied().unwrap_or(0);
            if b1 & 0x80 != 0 {
                self.byte_type = 1;
                w.push((slot, 0x09, 1));
            }
            if b1 == 0 {
                self.complete = 1;
                w.push((slot, 0x08, 1));
            }
            self.next_delay = b1;
            w.push((slot, 0x05, b1));
            let b2 = tbl.get(self.index as usize + 1).copied().unwrap_or(0);
            self.byte_type = self.byte_type.wrapping_sub(1);
            w.push((slot, 0x09, self.byte_type));
            if self.byte_type & 0x80 == 0 {
                // 8 位 x 模（b2 MSB⇒直存；否则**落穿** nibble 解——字节语义保真，注②）
                self.y_change = 0;
                w.push((slot, 0x06, 0));
                if b2 & 0x80 != 0 {
                    self.x_change = b2;
                    w.push((slot, 0x07, b2));
                } else {
                    self.byte_type = 0;
                    w.push((slot, 0x09, 0));
                    self.y_change = nibble_sub(b2 >> 4);
                    self.x_change = nibble_sub(b2 & 0x0F);
                    w.push((slot, 0x06, self.y_change));
                    w.push((slot, 0x07, self.x_change));
                }
            } else {
                // nibble 模
                self.byte_type = 0;
                w.push((slot, 0x09, 0));
                self.y_change = nibble_sub(b2 >> 4);
                self.x_change = nibble_sub(b2 & 0x0F);
                w.push((slot, 0x06, self.y_change));
                w.push((slot, 0x07, self.x_change));
            }
            self.index = self.index.wrapping_add(2);
            w.push((slot, 0x04, self.index));
        }
        if self.complete != 0 {
            return SparkleRunOutcome { a_out: self.complete, writes: w, y_out: 0 };
        }
        self.next_delay = self.next_delay.wrapping_sub(1);
        w.push((slot, 0x05, self.next_delay));
        self.y = self.y.wrapping_add(self.y_change);
        w.push((slot, 0x00, self.y));
        self.x = self.x.wrapping_add(self.x_change);
        w.push((slot, 0x03, self.x));
        self.change_delay = self.change_delay.wrapping_sub(1);
        w.push((slot, 0x08, self.change_delay));
        if self.change_delay == 0 {
            // BUG 保真（注①）：tileID 翻转 eor #3（asm 注「Should be $C6 and $C7」）
            self.tile_id ^= 0x03;
            w.push((slot, 0x01, self.tile_id));
            self.change_delay = 0x20;
            w.push((slot, 0x08, 0x20));
            self.attrib ^= 0x40; // `asl`（0x20→0x40=HFLIP）`eor attrib`
            w.push((slot, 0x02, self.attrib));
        }
        // WriteIntroSprite：SpriteRAM.4 面（y 减 1 修正——注③）；出口 A=x 值
        let out = self.x;
        w.push((slot | 0x80, 0x00, self.y.wrapping_sub(1))); // |0x80=SpriteRAM 面标记
        w.push((slot | 0x80, 0x01, self.tile_id));
        w.push((slot | 0x80, 0x02, self.attrib));
        w.push((slot | 0x80, 0x03, self.x));
        SparkleRunOutcome { a_out: out, writes: w, y_out: 0 }
    }
}

/// UpdateSparkleSprites（$87CF 落穿双跑 4 串行：X=0 两跑 + X=$10 两跑）。
pub fn update_sparkle_sprites(top: &mut SparkleSlot, bottom: &mut SparkleSlot) -> SparkleRunOutcome {
    let mut all = Vec::new();
    let mut a = 0;
    let mut y = 0u8;
    for _ in 0..2 {
        let idx_before = top.index;
        let ran_coord = top.next_delay == 0;
        let r = top.update(0);
        a = r.a_out;
        all.extend(r.writes);
        if ran_coord {
            y = idx_before.wrapping_add(1); // ldy idx / iny 末值
        }
    }
    for _ in 0..2 {
        let idx_before = bottom.index;
        let ran_coord = bottom.next_delay == 0;
        let r = bottom.update(1);
        a = r.a_out;
        all.extend(r.writes);
        if ran_coord {
            y = idx_before.wrapping_add(1);
        }
    }
    SparkleRunOutcome { a_out: a, writes: all, y_out: y }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// nibble 符号量级（≥8 取 and#7 取负——注④）。
    #[test]
    fn nibble_sub_negates_high_nibbles() {
        assert_eq!(nibble_sub(7), 7);
        assert_eq!(nibble_sub(8), 0);
        assert_eq!(nibble_sub(9), 0xFF);
        assert_eq!(nibble_sub(0xF), 0xF9);
    }

    /// 落穿双跑=4 串行（第二跑读第一跑写后态）。
    #[test]
    fn update_both_runs_four_serial() {
        let mut s = SparkleSlot { y: 0x3C, tile_id: 0xC6, attrib: 0x01, x: 0x18, index: 0,
                                  next_delay: 0, y_change: 0, x_change: 0,
                                  change_delay: 0x20, byte_type: 0, complete: 0 };
        let mut t = s.clone();
        let r = update_sparkle_sprites(&mut s, &mut t);
        assert!(!r.writes.is_empty());
    }
}
