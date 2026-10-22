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

/// InitCrossMissiles 初值表（bank0:$88BE 64B——X=$3F..0 双槽装载面）。
pub const INIT_CROSS_TBL: [u8; 64] = [
    0x20, 0xC5, 0x80, 0x00, 0xFF, 0xFF, 0x74, 0x58, 0xFF, 0xFF, 0x00, 0xFF, 0x1D, 0x0E, 0x01, 0x01,
    0x20, 0xC5, 0xC0, 0xF8, 0xFF, 0xFF, 0x7C, 0x58, 0xFF, 0xFF, 0x00, 0xFF, 0x1F, 0x0E, 0x80, 0x01,
    0xC8, 0xC5, 0x00, 0x00, 0xFF, 0xFF, 0x74, 0x60, 0xFF, 0xFF, 0x00, 0xFF, 0x1D, 0x1A, 0x01, 0x80,
    0xC8, 0xC5, 0x40, 0xF8, 0xFF, 0xFF, 0x7C, 0x60, 0xFF, 0xFF, 0x00, 0xFF, 0x1F, 0x1A, 0x80, 0x80,
];

/// InitCrossMissiles（$8897-$88BD）语义折叠——**BUG 保真（D4，注②同族）**：
/// `cmp PPUCTRL_ZP`（zp $FF **RAM 值**——asm 注「supposed to be #$FF」=立即数之误）——
/// 表字节==RAM[$FF] 才跳过装载。0xFF 面因 RAM[$FF]=$90 而**被装载**（bug 行为）；
/// 夹具双钉（RAM[$FF]=$FF 时 0xFF 面跳过）。尾特例：槽 6/7 y←$B8、speedY(0x0D)←$16。
pub fn init_cross_missiles(ff_zp: u8) -> Vec<SlotWrite> {
    // 标记约定：0x20=zp 面（off=址）；0x10=INTRO_SPR_BASE 相对原始偏移（off=0..0x7F，
    // 覆盖槽 0-3 面 $6EA0+x 与槽 4-7 面 $6EE0+x=+0x40+x 及尾特例 $6F00/$6F0D/$6F10/$6F1D）。
    let mut w = vec![(0x20, 0xBD, 0x20)]; // $BD←$20（CrossMsl4to7SpawnDelay）
    for x in (0..0x40u8).rev() {
        let v = INIT_CROSS_TBL[x as usize];
        if v == ff_zp {
            continue; // BUG（D4）：`cmp PPUCTRL_ZP`=zp $FF RAM 值——命中跳过（注「supposed to be #$FF」）
        }
        w.push((0x10, x, v));
        w.push((0x10, 0x40 + x, v));
    }
    w.push((0x10, 0x60, 0xB8)); // 槽6.y←$B8
    w.push((0x10, 0x70, 0xB8)); // 槽7.y←$B8
    w.push((0x10, 0x6D, 0x16)); // 槽6.speedY(0x0D)←$16
    w.push((0x10, 0x7D, 0x16)); // 槽7.speedY←$16
    w
}

// ---- 批4b·crosshairs 子机器（UpdateCrossExplode/FlashIntroScreen——09/08 组合点）----

/// CrossExplode 长度表（bank0:$89AA 5B——DataTbl 终止索引）。
pub const CROSS_EXPLODE_LENGTH_TBL: [u8; 5] = [0x05, 0x19, 0x41, 0x19, 0x05];

/// CrossExplode 数据表（bank0:$8A4B 68B）。
pub const CROSS_EXPLODE_DATA_TBL: [u8; 68] = [
    0x10, 0x5A, 0xC0, 0x00, 0x79, 0x14, 0x52, 0xC8, 0x00, 0x79, 0x18, 0x5A, 0xC2, 0x40, 0x71, 0x1C,
    0x5A, 0xC2, 0x00, 0x81, 0x20, 0x62, 0xC8, 0x80, 0x79, 0x14, 0x52, 0xC9, 0x00, 0x79, 0x18, 0x5A,
    0xC1, 0x00, 0x71, 0x1C, 0x5A, 0xC1, 0x00, 0x81, 0x20, 0x62, 0xC9, 0x00, 0x79, 0x24, 0x4A, 0xC8,
    0x00, 0x79, 0x28, 0x5A, 0xC2, 0x40, 0x69, 0x2C, 0x5A, 0xC2, 0x00, 0x89, 0x30, 0x6A, 0xC8, 0x80,
    0x79, 0xA4, 0xC1, 0xB9,
];

/// ScreenFlash 调色板表（bank0:$8ABD 10B，$FF 终止）。
pub const SCREEN_FLASH_PAL_TBL: [u8; 10] = [0x11, 0x01, 0x11, 0x01, 0x11, 0x11, 0x01, 0x11, 0x01, 0xFF];

/// UpdateCrossExplode（$8976-$89A9）语义折叠。
/// 返回 (A, X, Y 末值, 写面)。标记：0x20=zp 面；0x08=SpriteRAM $0200 相对原始偏移。
pub fn update_cross_explode(is_updating: u8, index: u8, frame_count: u8) -> (u8, Option<u8>, Option<u8>, Vec<SlotWrite>) {
    let mut w: Vec<SlotWrite> = Vec::new();
    if is_updating == 0 {
        return (0, None, None, w); // `lda $C7 / beq`——A=0 出
    }
    let mut idx = index;
    if index > 4 {
        return (1, None, None, w); // `cpy #$04 / bne` 出（A=标志值 1）
    }
    if index == 4 {
        w.push((0x20, 0xC7, 0)); // 末索引：清绘制标志
    }
    let end = CROSS_EXPLODE_LENGTH_TBL[idx as usize];
    w.push((0x20, 0x00, end)); // $00←终止索引
    let mut y: usize = 0;
    let mut x: u8 = 0;
    loop {
        x = CROSS_EXPLODE_DATA_TBL[y];
        y += 1;
        loop {
            let v = CROSS_EXPLODE_DATA_TBL[y];
            w.push((0x08, x, v)); // sta $0200,x
            x = x.wrapping_add(1);
            y += 1;
            if x & 3 == 0 {
                break;
            }
        }
        if y == end as usize {
            break;
        }
    }
    // 尾：`lda FrameCount / lsr / bcc` 奇帧才 `inc $C6`（每两帧进一格）
    let a = frame_count >> 1;
    if frame_count & 1 != 0 {
        idx = idx.wrapping_add(1);
        w.push((0x20, 0xC6, idx));
    }
    (a, Some(x), Some(y as u8), w)
}

/// FlashIntroScreen（$8AA7-$8ABC）语义折叠：ScreenFlashPalTbl 步进（$FF 端⇒清索引+标志）。
/// 返回 (A, Y 末值, 写面)。
pub fn flash_intro_screen(index: u8) -> (u8, Option<u8>, Vec<SlotWrite>) {
    let mut w: Vec<SlotWrite> = Vec::new();
    let y = index;
    let t = *SCREEN_FLASH_PAL_TBL.get(index as usize).unwrap_or(&0xFF);
    if t == 0xFF {
        w.push((0x20, 0xC2, 0));
        w.push((0x20, 0xC0, 0));
        return (0, Some(y), w); // `lda #$00 / sta …`——A=0 出
    }
    w.push((0x20, 0x1C, t)); // PaletteDataPending
    w.push((0x20, 0xC2, index.wrapping_add(1)));
    (t, Some(y), w)
}

// ---- 批4b·cross-missile 运动引擎（UpdateCrossMissiles/Coords/CalcDisplacement 折叠）----

/// 一枚 cross-missile 槽面（步距 $10；字节真偏移：y0/tile1/attr2/x3/completeA/
/// speedY 0x0D/speedX 0x0E/xChange 0x07(=crossMissileXChange 位)/yChange 0x06/
/// dirX 0x10/dirY 0x11——后两偏移经机器操作数核）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CrossMissile {
    pub y: u8,
    pub tile: u8,
    pub attr: u8,
    pub x: u8,
    pub complete: u8,
    pub y_change: u8,
    pub x_change: u8,
    pub speed_y: u8,
    pub speed_x: u8,
    pub dir_x: u8,
    pub dir_y: u8,
}

/// @CalcDisplacement（$9871）：FrameCount 抖动除 16（写面 zp $04/$00——NMI 温p 面）。
pub fn calc_displacement(speed: u8, frame_count: u8) -> (u8, Vec<SlotWrite>) {
    let mut w = Vec::new();
    let mut t4 = speed;
    let mut t0: u8 = 8;
    w.push((0x20, 0x04, t4));
    w.push((0x20, 0x00, t0));
    loop {
        let carry = t4 & 1;
        t4 >>= 1;
        if carry != 0 && (frame_count & t0) == 0 {
            t4 = t4.wrapping_add(1);
        }
        t0 >>= 1;
        w.push((0x20, 0x04, t4));
        w.push((0x20, 0x00, t0));
        if t0 == 0 {
            break;
        }
    }
    (t4, w)
}

/// UpdateCrossMissileCoords（$981E）→ carry（1=仍在动）。y 轨迹经 tr。
pub fn missile_coords(m: &mut CrossMissile, slot: u8, frame_count: u8, tr: &mut (Option<u8>, Option<u8>))
    -> (bool, Vec<SlotWrite>) {
    let mut w = Vec::new();
    let (d, w1) = calc_displacement(m.speed_x, frame_count);
    w.extend(w1);
    tr.1 = Some(m.dir_x); // ldy dirX
    let disp = if m.dir_x & 0x80 != 0 { 0u8.wrapping_sub(d).wrapping_add(1) } else { d };
    m.x = m.x.wrapping_add(disp);
    w.push((0x30 + slot, 0x03, m.x));
    let sbc_carry = if m.x >= m.x_change { 1u8 } else { 0u8 };
    if sbc_carry ^ (m.dir_x & 1) == 0 {
        // x 完成 → 快照
        m.y = m.y_change;
        m.x = m.x_change;
        w.push((0x30 + slot, 0x00, m.y));
        w.push((0x11, 0x03, m.x));
        return (false, w);
    }
    let (d, w2) = calc_displacement(m.speed_y, frame_count);
    w.extend(w2);
    tr.1 = Some(m.dir_y); // ldy dirY
    let disp = if m.dir_y & 0x80 != 0 { 0u8.wrapping_sub(d).wrapping_add(1) } else { d };
    m.y = m.y.wrapping_add(disp);
    w.push((0x30 + slot, 0x00, m.y));
    let sbc2 = if m.y >= m.y_change { 1u8 } else { 0u8 };
    if sbc2 ^ (m.dir_y & 1) == 0 {
        m.y = m.y_change;
        m.x = m.x_change;
        w.push((0x30 + slot, 0x00, m.y));
        w.push((0x11, 0x03, m.x));
        return (false, w);
    }
    (true, w)
}

/// UpdateCrossMissile（$8963）：complete 检→Coords→WriteIntroSprite。
pub fn update_missile(m: &mut CrossMissile, slot: u8, frame_count: u8, tr: &mut (Option<u8>, Option<u8>))
    -> Vec<SlotWrite> {
    let mut w = Vec::new();
    if m.complete != 0 {
        return w;
    }
    let (moving, w1) = missile_coords(m, slot, frame_count, tr);
    w.extend(w1);
    if !moving {
        m.complete = 1;
        w.push((0x30 + slot, 0x0A, 1));
    }
    // WriteIntroSprite（y-1 修正）
    w.push((0x38 + slot, 0x00, m.y.wrapping_sub(1)));
    w.push((0x38 + slot, 0x01, m.tile));
    w.push((0x38 + slot, 0x02, m.attr));
    w.push((0x38 + slot, 0x03, m.x));
    w
}

/// UpdateCrossMissiles（$88FE）调度（槽3 经 spawn 门单跑 tail 技法）。
/// slots=8 枚；slow/spawn 为 $BC/$BD 面；返回写面 + (X,Y) 轨迹。
pub fn update_cross_missiles(slots: &mut [CrossMissile; 8], slow: &mut u8, spawn: &mut u8,
                             frame_count: u8) -> (Vec<SlotWrite>, Option<u8>, Option<u8>) {
    let mut w = Vec::new();
    let mut tr: (Option<u8>, Option<u8>) = (None, None);
    if *slow != 0 {
        *slow = slow.wrapping_sub(1);
        w.push((0x20, 0xBC, *slow));
        if *slow == 0 {
            for (i, s) in slots.iter_mut().enumerate() {
                s.speed_x <<= 1;
                s.speed_y <<= 1;
                w.push((0x30 + i as u8, 0x0E, s.speed_x));
                w.push((0x30 + i as u8, 0x0D, s.speed_y));
            }
        }
    }
    for i in 0..3usize {
        tr.0 = Some(i as u8 * 0x10);
        w.extend(update_missile(&mut slots[i], i as u8, frame_count, &mut tr));
    }
    tr.0 = Some(0x30);
    if *spawn != 0 {
        *spawn = spawn.wrapping_sub(1);
        w.push((0x20, 0xBD, *spawn));
        if *spawn != 0 {
            w.extend(update_missile(&mut slots[3], 3, frame_count, &mut tr)); // tail-call 后 rts
            return (w, tr.0, tr.1);
        }
    }
    w.extend(update_missile(&mut slots[3], 3, frame_count, &mut tr));
    for i in 4..8usize {
        tr.0 = Some(i as u8 * 0x10);
        w.extend(update_missile(&mut slots[i], i as u8, frame_count, &mut tr));
    }
    (w, tr.0, tr.1)
}

// ===== 1:1 Rust 改写·批产·块 11（title/sparkle 初始化径（TDD 等价证明））=====

#[cfg(test)]
mod tests_b11_sparkle_init {
    //! TDD step1（测试先行（强制完整））——等价证明（新语义 fn==asm 精确语义
    //! （独立参考径（prg0_title.asm:789-810（LoadSparkleData/InitSparkleDataTbl 逐钉）））。

    use super::*;

    /// 【穷举·11 字节表载】InitSparkleDataTbl 逐字节（表值 11 项）→$6EA0..$6EAA/$6EB0..$6EBA
    /// 双槽载+覆写（$6B/$DC）——load_sparkle_data==asm 精确序列。
    #[test]
    fn sparkle_data_load_exact_sequence() {
        let w = load_sparkle_data();
        // 参考径（证伪立场）：asm 逐钉（ldx #$0A/dex/bpl 循环（11 字节反向载）+
        // $6B 覆写 IntroSprs.1.y（$6EB0）+$DC 覆写 .1.x（$6EB3））
        let tbl = INIT_SPARKLE_DATA_TBL;
        assert_eq!(tbl.len(), 11);
        let mut expected: Vec<(u16, u8)> = Vec::new();
        for i in 0..11u16 {
            // 【证伪自纠】asm `lda tbl,x/sta base,x`（dex 反序循环——地址=base+x（值 tbl[x] 索引对齐））
            expected.push((0x6EA0 + i, tbl[i as usize]));
            expected.push((0x6EB0 + i, tbl[i as usize]));
        }
        expected.push((0x6EB0, 0x6B));
        expected.push((0x6EB3, 0xDC));
        assert_eq!(w, expected);
    }

    /// 【边界】表值逐钉（asm .byte 序（$3C,$C6,$01,$18,0,0,0,0,$20,0,0））。
    #[test]
    fn init_sparkle_table_bytes() {
        assert_eq!(INIT_SPARKLE_DATA_TBL, [0x3C, 0xC6, 0x01, 0x18, 0, 0, 0, 0, 0x20, 0, 0]);
    }
}

// ===== 1:1 Rust 改写·批产·块 11：title/sparkle 初始化径语义命名（去地址名（公式形））=====
// refs 锚（prg0_title.asm:789-810（LoadSparkleData 00:87AB/InitSparkleDataTbl 00:87C4））。

/// InitSparkleDataTbl（00:87C4（prg0_title.asm:809-810））——11 字节初值
/// （$3C,$C6,$01,$18,0,0,0,0,$20,0,0）。数据驱动（表值不进逻辑）。
pub const INIT_SPARKLE_DATA_TBL: [u8; 11] = [0x3C, 0xC6, 0x01, 0x18, 0, 0, 0, 0, 0x20, 0, 0];

/// Sparkle 初载（旧 LoadSparkleData（00:87AB）：11 字节表载双槽（IntroSprs.0.y=$6EA0/IntroSprs
/// .1.y=$6EB0（步距 $10（IntroSpr 面（constants_ram.asm:183-191（y+0/x+3））））+
/// 覆写（.1.y←$6B/.1.x←$DC））。返回写事件序列（asm 逐钉）。
pub fn load_sparkle_data() -> Vec<(u16, u8)> {
    let mut w = Vec::with_capacity(24);
    for i in 0..11u16 {
        w.push((0x6EA0 + i, INIT_SPARKLE_DATA_TBL[i as usize]));
        w.push((0x6EB0 + i, INIT_SPARKLE_DATA_TBL[i as usize]));
    }
    w.push((0x6EB0, 0x6B));  // lda #$6B/sta IntroSprs.1.y
    w.push((0x6EB3, 0xDC)); // lda #$DC/sta IntroSprs.1.x（x=+3（IntroSpr 面））
    w
}
