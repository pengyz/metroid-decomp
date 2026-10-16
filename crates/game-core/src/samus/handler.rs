//! Samus 状态机脊柱（bank07:CC0D 族；task-25 首件——先立脊柱后填肉范式③）。
//!
//! **asm 内容锚+结构互核注**（施工令②精化版）——
//! refs/m1disasm/SRC/prg7_engine.asm `UpdateSamus: ; 07:CC0D`（`ldx #$00/stx PageIndex`/
//! `inx/stx IsSamus`/`jsr GoSamusHandler`/`dec IsSamus`/`rts`）+ `GoSamusHandler: ;
//! 07:CC1A`（`lda Samus.status/bmi SamusStand`（bit7=站立）/`jsr JumpEngine`（$C27C
//! 表分派）/10 项 `.word` 表）。
//!
//! **结构互核注**：①10 态分发表=GoMainRoutine/title 31 项同构法第三次用（脊柱范式）
//! ②`bmi` 位 7=站立旗（Samus.status 位语义——Object 结构 $0300 面 status 注
//! 「0=object slot not in use」+位 7 站立扩展）③JumpEngine 内联表返回址读栈（`pla`×2）
//! ——栈面一层，域形=match 分派复刻。
//!
//! R-mv 挂点（spec 源=docs/spec/samus-movement.md）：状态转移=规则触发面。
//!
//! 地址面：PageIndex=$4B/IsSamus=$55（constants_ram 内容锚）；Samus.status=$0300
//! （Object 结构首字段——constants_ram `.struct Object` 内容锚）。

use crate::frame::FrameMemory;
use crate::ram_map::{IS_SAMUS, PAGE_INDEX, SAMUS_STATUS};

/// Samus 处理态（GoSamusHandler 表序自证——asm `.word` 10 项）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SamusHandler {
    /// $CC36 站立（含位 7 站立旗直入）。
    Stand,
    /// $CCC2 奔跑。
    Run,
    /// $D002 跳跃。
    Jump,
    /// $D0E1 滚动。
    Roll,
    /// $D198 上指。
    PntUp,
    /// $D3A8 门内滚动。
    Door,
    /// $D002 上指跳跃（表第 7 项——与 Jump 同标的 asm 侧）。
    JumpPntUp,
    /// $D41A 死亡。
    Dead,
    /// $D41F 二次死亡。
    Dead2,
    /// $D423 电梯。
    Elevator,
}

impl SamusHandler {
    /// 表序→态（GoSamusHandler `.word` 序自证）。
    pub fn from_index(i: u8) -> Option<Self> {
        Some(match i {
            0 => Self::Stand,
            1 => Self::Run,
            2 => Self::Jump,
            3 => Self::Roll,
            4 => Self::PntUp,
            5 => Self::Door,
            6 => Self::JumpPntUp,
            7 => Self::Dead,
            8 => Self::Dead2,
            9 => Self::Elevator,
            _ => return None,
        })
    }

    /// 态→处理件址（fail-loud 引爆面——各状态处理族随 move 相队列填）。
    pub fn handler_addr(self) -> &'static str {
        match self {
            Self::Stand => "bank07:CC36",
            Self::Run => "bank07:CCC2",
            Self::Jump | Self::JumpPntUp => "bank07:D002",
            Self::Roll => "bank07:D0E1",
            Self::PntUp => "bank07:D198",
            Self::Door => "bank07:D3A8",
            Self::Dead => "bank07:D41A",
            Self::Dead2 => "bank07:D41F",
            Self::Elevator => "bank07:D423",
        }
    }
}

/// UpdateSamus（asm :2705 内容锚）：PageIndex=0/IsSamus=1→处理→IsSamus=0。
pub fn update_samus(mem: &mut FrameMemory) {
    mem.write_ram(PAGE_INDEX, 0); // `ldx #$00/stx PageIndex`
    mem.write_ram(IS_SAMUS, 1); // `inx/stx IsSamus`
    go_samus_handler(mem);
    mem.write_ram(IS_SAMUS, 0); // `dec IsSamus`
}

/// GoSamusHandler 分发壳（asm :2716 族内容锚）：status 位 7=站立直入；否则表序分派。
pub fn go_samus_handler(mem: &mut FrameMemory) {
    let status = mem.read_ram(SAMUS_STATUS);
    let handler = if status & 0x80 != 0 {
        SamusHandler::Stand // `bmi SamusStand`
    } else {
        match SamusHandler::from_index(status & 0x0F) {
            Some(h) => h,
            None => crate::unported!("bank07:CC1A-index"), // 表外索引（0x0A-0x0F）
        }
    };
    crate::unported!(handler.handler_addr());
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 10 态表序自证（from_index 全覆盖）+分发引爆面。
    #[test]
    fn samus_handler_dispatch_ten_states() {
        for (i, addr) in [
            (0u8, "bank07:CC36"), (1, "bank07:CCC2"), (2, "bank07:D002"), (3, "bank07:D0E1"),
            (4, "bank07:D198"), (5, "bank07:D3A8"), (6, "bank07:D002"), (7, "bank07:D41A"),
            (8, "bank07:D41F"), (9, "bank07:D423"),
        ] {
            assert_eq!(SamusHandler::from_index(i).unwrap().handler_addr(), addr);
        }
        assert!(SamusHandler::from_index(10).is_none());
    }

    /// UpdateSamus 面：PageIndex/IsSamus 落态+位 7 站立直入引爆。
    #[test]
    fn update_samus_spine_face() {
        let mut m = FrameMemory::default();
        m.write_ram(SAMUS_STATUS, 0x80); // 站立旗
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| update_samus(&mut m)));
        let err = r.unwrap_err();
        let msg = err.downcast_ref::<String>().cloned()
            .or_else(|| err.downcast_ref::<&'static str>().map(|s| (*s).to_string()))
            .unwrap_or_default();
        assert!(msg.contains("unported @ bank07:CC36"), "站立直入失准: {msg}");
        assert_eq!(m.read_ram(PAGE_INDEX), 0);
        assert_eq!(m.read_ram(IS_SAMUS), 1); // dec 前态（引爆先于收尾）
    }
}
