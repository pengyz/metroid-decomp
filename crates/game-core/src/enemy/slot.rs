//! 敌槽 16B 状态块域模型（typed view 的语义侧；地址侧=memmap::enemy_slots）。
//!
//! **命名证据（task-18 rename 批）**：对表 m1disasm `EnExtra` 结构（refs/m1disasm/SRC/
//! constants_ram.asm:.struct EnExtra 内容锚——16 字段全命名）+ 三重自证：
//! ①F28E/F295 行为触达（A 级）②Object 结构同构（同名 radius/anim/hi 族，$0300 面）
//! ③用例面静态推理（B 级，可复现）——台账 N-0031..N-0045 逐字段登记。
//! 社区 Unknown 四字段（subPixel/accel 族）经**亚像素累积物理用例**（areas_common
//! 「apply sub-pixel speed to sub-pixel position」）+ IntX/IntY 同构自证；
//! `data1F`=伪语义名（地址后缀）拒采留 `_todo_0f`（线索注随字段）。

/// 敌槽状态块（16B）。槽语义视图与字节偏移契约面经 memmap::enemy_slots codec 互转。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnemySlot {
    /// 敌状态（A 级）：bank07:F28E `BD F4 6A`/`C9 04`（CMP #$04）+ EnExtra 注「#$00=Enemy
    /// slot not in use, #$04=Enemy frozen」+ nmikstas Bank03 L98A4 `LDA #$04/STA $6AF4,X`。
    pub status: u8,
    /// Y 向半径（B 级同构）：EnExtra.radiusY「Distance in pixels from middle of enemy to
    /// top or botom」+ Object.radiusY（$0301 面）同名同注 + `Samus.radiusY` 用例。
    pub radius_y: u8,
    /// X 向半径（B 级同构）：EnExtra.radiusX + Object.radiusX（$0302 面）。
    pub radius_x: u8,
    /// 动画帧号（B 级同构）：EnExtra.animFrame「Index into enemy animation frame data」+
    /// Object.animFrame（$0303 面）+ `Samus.animFrame` 用例。
    pub anim_frame: u8,
    /// 动画帧延时（B 级）：EnExtra.animDelay「Number of frames to delay between animation
    /// frames」+ Object.animDelay（$0304 面）+ `EnsExtra.0.animDelay,y` 用例。
    pub anim_delay: u8,
    /// 动画序列重置号（B 级）：EnExtra.resetAnimIndex「Index to beginning of animation
    /// sequence」+ Object.animResetIndex 同构 + `EnsExtra.0.resetAnimIndex,y` 用例。
    pub reset_anim_index: u8,
    /// 当前动画号（B 级同构）：EnExtra.animIndex「Index to current animation」+
    /// Object.animIndex（$0306 面）+ `Samus.animIndex` 用例。
    pub anim_index: u8,
    /// nametable 高位（B 级同构）：EnExtra.hi「#$00=Enemy on name table 0, #$01=Enemy on
    /// name table 3」+ Object.hi（$030C 面）同注。
    pub hi: u8,
    /// Y 亚像素位（B 级物理实证）：areas_common「apply sub-pixel speed to sub-pixel
    /// position」`lda subPixelY/clc/adc speedSubPixelY/sta subPixelY/lda #$00/adc speedY`
    /// ——亚像素累积物理（IntY 同构）。
    pub sub_pixel_y: u8,
    /// X 亚像素位（B 级物理实证）：同 subPixelY 面（X 轴，areas_common 同型段）。
    pub sub_pixel_x: u8,
    /// Y 加速度（B 级）：`lda EnAccelYTable,y/sta EnsExtra.0.accelY,x` 表喂给用例
    /// （IntY 积分物理同构）；社区注「Unknown」=角色未证，名自证于表名互证。
    pub accel_y: u8,
    /// X 加速度（B 级）：`lda EnAccelXTable,y/sta EnsExtra.0.accelX,x`（IntX 同构）。
    pub accel_x: u8,
    /// 备用字节（B 级）：EnExtra.spare1C 注「Unused」+ 全仓用例 grep **零命中**自证
    /// （spare1C=地址后缀伪语义名拒采，取 spare）。
    pub spare: u8,
    /// 跳跃位移（B 级）：EnExtra.jumpDsplcmnt「Number of pixels vertically/horizontally
    /// displaced from jump point; skree blow up delay」+ EnemyMovementInstr_ClearEnJump-
    /// Dsplcmnt 宏（$FD）互证 + `sta EnsExtra.0.jumpDsplcmnt,x` 用例。
    pub jump_dsplcmnt: u8,
    /// 敌类型号（A 级）：bank07:F295 `BD 02 6B` → `8D 0F 01`/`A8`（TAY→$968B 表查）+
    /// EnExtra.type「Enemy type used as index into enemy data tables」。
    pub type_: u8,
    /// 【留守 _todo_】社区名 data1F=地址后缀伪语义名**拒采**（AGENTS §3.2）；语义线索
    /// （C 级）：EnExtra 注「For EnemyFlipAfterDisplacement: #$00=abs / #$40=signed /
    /// #$80|$C0=negated…For EnemyIfMoveFailed: bounce/land 四态」=移动指令位模式修饰器。
    pub _todo_0f: u8,
}

impl EnemySlot {
    /// 槽字节宽（16B 步长，memmap 侧常量同源互检）。
    pub const SIZE: usize = 16;

    /// 字节偏移口径 → 槽视图。
    pub fn from_bytes(b: [u8; Self::SIZE]) -> Self {
        Self {
            status: b[0],
            radius_y: b[1],
            radius_x: b[2],
            anim_frame: b[3],
            anim_delay: b[4],
            reset_anim_index: b[5],
            anim_index: b[6],
            hi: b[7],
            sub_pixel_y: b[8],
            sub_pixel_x: b[9],
            accel_y: b[10],
            accel_x: b[11],
            spare: b[12],
            jump_dsplcmnt: b[13],
            type_: b[14],
            _todo_0f: b[15],
        }
    }

    /// 槽视图 → 字节偏移口径。
    pub fn to_bytes(self) -> [u8; Self::SIZE] {
        [
            self.status,
            self.radius_y,
            self.radius_x,
            self.anim_frame,
            self.anim_delay,
            self.reset_anim_index,
            self.anim_index,
            self.hi,
            self.sub_pixel_y,
            self.sub_pixel_x,
            self.accel_y,
            self.accel_x,
            self.spare,
            self.jump_dsplcmnt,
            self.type_,
            self._todo_0f,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// rename 后槽视图⇄字节面往返不丢（offset_view 测试同源面）。
    #[test]
    fn slot_view_roundtrip_after_rename() {
        let b: [u8; 16] = core::array::from_fn(|i| (i as u8).wrapping_mul(7));
        assert_eq!(EnemySlot::from_bytes(b).to_bytes(), b);
    }
}
