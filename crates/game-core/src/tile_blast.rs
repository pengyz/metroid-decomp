//! game-core::tile_blast —— tile blast 复活域（UpdateTileBlast_Respawned 语义侧）。
//!
//! asm 主文档（链规 17 内容锚引证+结构互核注）：refs/m1disasm/SRC/prg7_engine.asm:12673
//! `UpdateTileBlast_Respawned: ; 07:FE83`——**段界 FE83-FECB（73B/34 条指令）**：
//! 【2026-10-15 结构互核更正】r2 报告「$FE83-$FECA 代码 72B」切掉尾字节——ROM 逐字节
//! `a9 00 9d 00 05 … 85 6e 4c 92 ce`（JMP `4C 92 CE` 跨 FEC9-FECB），m1disasm 下一标签
//! `GetVRAMStringPtr: ; 07:FECC` 双证；**r2 边界以 FE83-FECB 为准**。
//!
//! 语义（A 级：ROM 逐字节 × m1disasm 段 × L2 九案三方全等）：
//! 复活的 tile blast 落在 Samus 身上 ⇒ 伤害序列。流=删槽 routine→房间 RAM 指针 +0x21
//! （**低字节进位丢弃 quirk**：`adc #$21` 无高位进位传播）→元块中心（FF3C 数学）→
//! X 槽位（元块）/Y 槽位（Samus）位置装配→8×8 判定盒（半径=4+radius）→碰撞（F1FA，
//! carry=重叠判定）⇒ 伤害链（F311→F338→F27B→F323）→HealthChange←$50→JMP $CE92。
//!
//! 调用集（比 r2 记录多一员——`jsr SamusHurt_F311` r2 漏记）：FF3C GetPosAtNameTableAddr、
//! F186 GetObjectYSlotPosition、F1FA CheckCollisionOfXSlotAndYSlot、F311 SamusHurt。
//! 「注释≠语义」观察（第六案族）：FE83 头注「($03, $02) = position of center of metatile」
//! 描述的是 FF3C 的**输出**（前文先装配 $00/$01 指针）——注释位置≠执行顺序。

/// 8×8 判定盒基数（asm：`lda #$04 / clc / adc Samus.radiusY`——盒半径=4+radius）。
pub const HITBOX_BASE: u8 = 0x04;

/// 复活元块中心（FF3C GetPosAtNameTableAddr 语义折叠）：入参=（房间 RAM 指针+0x21 后的）
/// nametable 地址（$01.$00 = ------yy yyyxxxxx）→（Y 像素=yyyyy000、X 像素=xxxxx000）。
/// A 级：bank07:FF3C-FF53 逐字节（`and #%11100000`/`lsr`+`ror` 链/`and #%00011111`+Amul8）。
pub fn metatile_center(nametable_addr: u16) -> (u8, u8) {
    let lo = nametable_addr as u8;
    let hi = (nametable_addr >> 8) as u8;
    // $02 链：(hi&3)<<6 | (lo&0xE0)>>2 —— 逐 ror 步进等价（FF3C `lsr A/ror $02` ×2）
    let cy = ((hi & 0x03) << 6) | ((lo & 0xE0) >> 2);
    let cx = (lo & 0x1F) << 3; // Amul8
    (cy, cx)
}

/// 房间 RAM 指针 → nametable 地址（**低字节 +0x21 进位丢弃 quirk**——`adc #$21` 不带
/// 高位进位；0x0508=$FF 夹具=b-fe83-hi-unequal-x-hit 钉死）。
pub fn respawn_nametable_addr(room_ram_ptr: u16) -> u16 {
    let lo = (room_ram_ptr as u8).wrapping_add(0x21);
    (room_ram_ptr & 0xFF00) | lo as u16
}

/// 伤害序列入参（F311 链的语义面；dist_hi=碰撞差值高字节位面、dist_x=碰撞差值 X）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HurtParams {
    pub dist_hi: u8,
    pub dist_x: u8,
    pub is_hit_old: u8,
}

/// 伤害序列结果（F311→F338→F27B→F323 语义折叠；HealthChange 清零面除外——$6E 终值
/// 由调用方覆盖为 $50，见 verify_shim::fe83 注）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HurtOutcome {
    /// SamusHurt010F 暂存（固定 $E0，语义未证 C 级——「never read」constants_ram:819）。
    pub hurt_scratch: u8,
    /// isHit 新值（A=($10<<3) 后 OR 旧值——F338 `asl×3`+F27B `ora/sta`）。
    pub is_hit: u8,
    /// SamusKnockbackIsBomb（$0F==0 ⇒ 0=斜向；否则 1=竖直——F311 Lx295 语义）。
    pub knockback: u8,
}

/// 伤害序列（F311 SamusHurt_F311 链语义折叠；Y=0 槽固定=Samus）。
/// A 级：bank07:F311-F329 + F338-F33F + F27B-F281 逐字节；$030A+Y 写面复用
/// memmap::enemy_attrs（Object.isHit 双语境——constants_ram:228）。
pub fn hurt_sequence(p: &HurtParams) -> HurtOutcome {
    let is_hit = ((p.dist_hi << 3) & 0xFF) | p.is_hit_old;
    HurtOutcome {
        hurt_scratch: 0xE0,
        is_hit,
        knockback: if p.dist_x == 0 { 0 } else { 1 },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// FF3C 数学：0x0564 → Y=0x58 X=0x20（L2 案 b-fe83-* 同源断言）；
    /// 0x0720（0x07FF+0x21 进位丢弃产物）→ Y=0xC8 X=0x00。
    #[test]
    fn metatile_center_matches_ff3c_math() {
        assert_eq!(metatile_center(0x0564), (0x58, 0x20));
        assert_eq!(metatile_center(0x0720), (0xC8, 0x00));
    }

    /// 进位丢弃 quirk（0x07FF+0x21 → 0x0720 而非 0x0820）。
    #[test]
    fn respawn_addr_drops_carry() {
        assert_eq!(respawn_nametable_addr(0x07FF), 0x0720);
        assert_eq!(respawn_nametable_addr(0x0543), 0x0564);
    }

    /// 伤害链折叠（isHit OR 旧值/knockback 两路/暂存 $E0）。
    #[test]
    fn hurt_sequence_folds_f311_chain() {
        let h = hurt_sequence(&HurtParams { dist_hi: 5, dist_x: 3, is_hit_old: 0 });
        assert_eq!(h, HurtOutcome { hurt_scratch: 0xE0, is_hit: 0x28, knockback: 1 });
        let h = hurt_sequence(&HurtParams { dist_hi: 6, dist_x: 0, is_hit_old: 0x20 });
        assert_eq!(h, HurtOutcome { hurt_scratch: 0xE0, is_hit: 0x30, knockback: 0 });
    }
}
