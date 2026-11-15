//! `UpdateAllEnemies ; 07:F345` / `UpdateEnemy ; 07:F351` —— 敌槽环与七态分派。
//!
//! 本文件由 `frame/mainloop.rs` 原样搬出（纯搬移）：槽环 `$50→$00`（16 字节步长）、
//! 可见性门、`data05` bit6 翻转、JumpEngine 七态（0 ExitSub / 1 Resting / 2 Active /
//! 3 Explode / 4 Frozen / 5 Pickup / 6 Hurt / ≥7 RemoveEnemy）、以及两个已端口的 AI
//! 派发（`skree_ai` / `crawler_ai`）与其辅助件。

use crate::enemy::{framework::*, anim::*, skree::*, crawler::*};
use crate::enemy::prep::*;
use crate::frame::tick::*;
use crate::ram_map;

/// UpdateAllEnemies+UpdateEnemy（**asm 内容锚** `UpdateAllEnemies: ; 07:F345`/
/// `UpdateEnemy: ; 07:F351`——r26 端口：槽环 $50→$00×16B+状态 dispatch（7 态））：
/// ①PageIndex←x ②EnsExtra.status：Resting/Active⇒CheckIfVisible ③UpdateEnData05Bit6
/// ④JumpEngine：0=ExitSub/1=Resting/2=Active/3=Explode/4=Frozen/5=Pickup/6=Hurt/
/// ≥7⇒RemoveEnemy。件径=敌域 L2 单位（engine::update_enemy_*）会师面。
pub fn update_all_enemies(mem: &mut crate::frame::FrameMemory) {
    let mut x: u8 = 0x50; // _sizeof_Ens-_sizeof_Ens.0（5 槽×16）
    loop {
        // 【goal-轮45 可见位直证】敌B bit1（金方冻结=离屏）。
    // 【goal-轮97 updateAll 径直证】敌更新调用（金方冻径）。
    // 【goal-轮144 updateAll 遍历径直证】敌更新调用链（paradox 解）。
    mem.write_ram(0x004B, x); // PageIndex
        let status = mem.read_wram(0x6AF4 + x as u16); // EnsExtra.0.status
        // CheckIfVisible（$F37F——不可见⇒双返跳过该敌）
        // 【敌槽生成/位形卡（2026-11-05）】asm F351 适用域=**status 1|2（Resting|Active）**
        // （`ldy status/beq 跳0 / cpy #$03/bcs 跳≥3`）——旧形 `status<2` 只护 Resting，
        // **Active(2) 漏门**⇒morphball 敌B f847 离屏仍跑 AI（emu 双返回全冻）⇒f1278
        // 敌撞击退单腿⇒七格残差（goal-轮45 探针注「敌B bit1（金方冻结=离屏）」收口）。
        if status != 0 && status < 3 && !enemy_check_if_visible(mem, x) {
            x = if x == 0 { break } else { x - 0x10 };
            continue;
        }
        update_en_data05_bit6(mem, x); // $F3AA（30fps 标位翻转）
        // EnemyStatusPreAI←status（asm prg7:10511 `lda EnsExtra.0.status/sta EnemyStatusPreAI`
        // ——分派前捕获，激活同帧 AI 例程据它拦 Resting（crawler.asm:8-11——路径分叉根因件
        // ①：缺此门则激活帧即跑 AI 且半径未写（draw 在 AI 后）⇒rx=0 快径⇒外角下探伪成功）。
        mem.write_ram(0x0081, status); // EnemyStatusPreAI（constants_ram:633 zp $81）
        match status {
            0 => {} // ExitSub（rts）
            1 => enemy_resting(mem, x), // F3BE
            2 => enemy_active(mem, x),  // F3E6
            3 => update_enemy_explode(mem, x), // F40D 爆亡（计时→CompleteDeath）
            4 => update_enemy_frozen(mem, x),  // F54D 冻（计时→回活）
            5 => update_enemy_pickup(mem, x),  // F57D 拾取（碰触→回血/移除）
            6 => update_enemy_hurt(mem, x),    // F59D Hurt（计时→回活）
            _ => crate::unported!("bank07:FA18"), // RemoveEnemy（状态≥7）
        }
        if x == 0 {
            break;
        }
        x -= 0x10;
    }
}

/// UpdateEnemy_CheckIfVisible（**asm 内容锚** `: 07:F37F`——r26）：
/// data05 bit1≠0⇒退出；否则位置/半径→IsObjectVisible⇒**不可见双返**（跳过该敌）。
pub(crate) fn enemy_check_if_visible(mem: &mut crate::frame::FrameMemory, x: u8) -> bool {
    // 【goal-轮64 闩置位帧直证】draw visible 写面（闩自持差源）。
    if mem.read_ram(0x0400 + x as u16 + 5) & 0x02 != 0 {
        return true; // $F37F bit1 闩=1 ⇒ 免 fresh 检（视为可见）
    }
    enemy_check_if_visible_inner(mem, x)
}


#[cfg(test)]
mod tests_deferred_screen_on {
    use super::*;
    use crate::state::FrameMemory;

    /// 转场暗窗建模锚（TDD 红→绿）：MoreInit 尾的 ScreenOn 影子写必须**延迟**到帧门
    /// 排空（迭代完成点）才落 $FE——asm `WriteAndWait`（07:C43D）写影子后
    /// `WaitNMIPass_`（07:C43F）等一个 NMI，真 PPUMASK 由下一帧 `WritePPUCtrl` 落。
    #[test]
    fn deferred_screen_on_applies_at_drain_point() {
        let mut m = FrameMemory::default();
        m.write_ram(0x00FE, 0x06); // ScreenOff 后的影子（转场中）
        m.deferred_screen_on = true;
        apply_deferred_screen_on(&mut m);
        assert_eq!(m.read_ram(0x00FE), 0x1E, "排空点应把 ScreenOn 影子落 $FE");
        assert_eq!(m.render.ppumask, 0x1E, "同相位口径：真寄存器同步点亮（WriteAndWait 的 NMI 落点）");
        assert!(!m.deferred_screen_on, "落影后旗须清零（单次）");
    }

    /// 对照（防误触发）：旗未置时每 tick 都会走到排空点，必须零动。
    /// 【F7 加固（2026-11-12，r9 复核 C 面）】真寄存器预置**哨兵 0x06**（非默认 0x1E）——
    /// 旧形预置 0x1E 时「无条件写 0x1E」的坏实现与本测同过（弱断言）；哨兵使任何
    /// 无旗写动作立即暴露。整链移除的防线=L4 锚格 f1191 dark-window/f1197 last-dark
    /// （0px，接线断即暗窗消失→与 emu 黑窗实录失配）——单测不重复该职责。
    #[test]
    fn deferred_screen_on_inert_without_flag() {
        let mut m = FrameMemory::default();
        m.write_ram(0x00FE, 0x06);
        m.render.ppumask = 0x06; // 哨兵（ScreenOff 态真值，非默认 0x1E）
        apply_deferred_screen_on(&mut m);
        assert_eq!(m.read_ram(0x00FE), 0x06, "无旗不得改 $FE");
        assert_eq!(m.render.ppumask, 0x06, "无旗不得改真寄存器（哨兵 0x06 不变）");
        assert!(!m.deferred_screen_on, "无旗时旗不得被误置");
    }
}

#[cfg(test)]
mod tests_visibility {
    use super::*;

    /// 【IsObjectVisible 忠实重建钉（2026-10-31——crawlprobe slot2 残差根因）】asm 07:DFDF
    /// HorzScrollCheck：异名表+（x≥ScrollX）⇒隐形（载入邻名表敌在 PPU 翻转前不可见）；
    /// 同名表⇒rx<screenX 可见。旧形横房 y<240 近似恒真=误可见。
    #[test]
    fn enemy_visibility_horz_nametable_and_edge_rules() {
        let mut m = crate::frame::FrameMemory::default();
        let x = 0x40u8; // slot4
        m.write_ram(crate::ram_map::SCROLL_DIR, 3); // 横房右滚
        m.write_ram(0x00FF, 0x00); // PPU 当前名表 0
        m.write_ram(crate::ram_map::SCROLL_X, 0x03);
        m.write_ram(crate::ram_map::SCROLL_Y, 0);
        // 异名表敌（hi=1）：x≥ScrollX ⇒ 隐形（载入侧名表在 PPU 翻转前不可见）
        m.write_ram(0x0400 + x as u16, 0x38);
        m.write_ram(0x0400 + x as u16 + 1, 0x8C);
        m.write_wram(0x6AF4 + x as u16 + 7, 1); // hi=1
        m.write_wram(0x6AF4 + x as u16 + 2, 2); // radiusX=2
        assert!(!enemy_check_if_visible(&mut m, x), "异名表+x≥ScrollX ⇒ 隐形");
        // 同名表敌（hi=0）：rx(8)<screenX(0x8C-3) ⇒ 可见
        m.write_wram(0x6AF4 + x as u16 + 7, 0);
        m.write_wram(0x6AF4 + x as u16 + 2, 8);
        assert!(enemy_check_if_visible(&mut m, x), "同名表 rx<screenX ⇒ 可见");
        // 同名表贴左缘：screenX≤rx ⇒ 隐形（asm cmp/bcc 形——quirk 保真）
        m.write_ram(0x0400 + x as u16 + 1, 0x05); // screenX=2 ≤ rx=8
        assert!(!enemy_check_if_visible(&mut m, x), "screenX≤rx ⇒ 隐形（asm quirk）");
        // bit1 闩=1 ⇒ 免 fresh 检（$F37F 首件）
        m.write_ram(0x0400 + x as u16 + 5, 0x02);
        assert!(enemy_check_if_visible(&mut m, x), "bit1 闩直通");
    }
}

/// UpdateEnemy_UpdateEnData05Bit6（**asm 内容锚** `: 07:F3AA`——r26）：
/// data05 bit6←(slot÷16 xor FrameCount) bit0（**30fps 动画标**——D4 保真）。
pub(crate) fn update_en_data05_bit6(mem: &mut crate::frame::FrameMemory, x: u8) {
    let d = mem.read_ram(0x0400 + x as u16 + 5);
    let bit6 = (d >> 6) & 1;
    // 【goal-轮33/34 证伪记账】FC 相位两假说（全局/toggle 单点 FC+1）皆实测劣化
    // （40/230、9/230←94/230）⇒回退——f1838 d05 bit6 差**非 FC 相位根**（§3.6 原地记）。
    // 【goal-轮38 位定义自证】constants_ram:96/156：bit0=向位（0=右/1=左）/bit1=可见旗
    // （IsObjectVisible）；bit7=轴向（FB7B ror L977B）/bit6=toggle 翻转/bit4-3=阈距——
    // **bit2/bit5 位源未明**（constants 无注——余项记账）。
    // 【goal-轮43 bit5 定案】bit5=**静息请求位**（enemy_active `and #$20` 读面（:10604）：
    // 置⇒delay←InitDelayTbl+status--（Active→Resting））——写源=spawn init（:12049
    // `sta data05`——facing+axis 组合 bit5=0）；运行期无独立写源（读面位 ✓）。
    // 【goal-轮45 坠落 delay 相位记账】Skree 坠落起点：金 f1878 delay=1/f1880 起移；我
    // f1878 delay=0/f1880 已移——**delay 倒数 1 帧相位差**（移动串 delay 面）=余项。
    // 【goal-轮59 坠落 delay 相位续查】EnemyGetDeltaY（$8244——movementIndex+instr
    // 字符串走表）delay 语义：control 字节（$FA-FF）+delay 倒数——1 帧相位=串位 instr
    // 推进相位面（下轮 asm 逐件对表 $8244 串走表）。
    crate::frame::DIVERGENCES.with(|d| d.borrow_mut().push(0x0FB7B));
    // 【goal-轮57 FC 读面相位逐点（L4 判定）】toggle 单点 FC+1（轮34 形——轮33/34 用
    // L3 判定劣化；本轮 L4 重判）。金腿 FC=+1 恒差（ram.hex 直证）⇒toggle 读面相位
    // =+1（逐点修——非全局移）。
    // 【goal-轮66 判定面分歧记账】toggle FC+1 后：L3 状态面 9/230（回归 ✗）vs L4 视觉
    // 面 55/97（中性 ✓）——**判定面分歧**（L4=验收标准（用户裁定）⇒保留；L3 回归=
    // 状态相位面连锁（f1836 起爬虫转角 d05 位差提前）——余项）。
    // 【goal-轮修 asm 合形（已回退——§3.1）】F3AA parity=((x/16)^FC)&1 asm 合形删 +1
    // ⇒L4 55→53（−2 劣化）⇒**回退保留 +1**（轮57 形）——bit6 相位假说（f797-801 反相）
    // 被 L4 证伪（+1 实为补偿某物——FC 更新时点 vs 采样时点假说余）。
    // 【2026-11-04 更正（H3 根修——findings/h3-enemy-slot-residual.md §2）】上列 +1
    // 补偿**失配定谳**：金腿 FC=+1 恒差是 r14 FC 对齐前的错位（补偿把公式对到错位腿）；
    // FC 对齐后 7 采集点双腿全同（f800 全 6 槽反相实证=+1 过补）⇒asm 合形直读 FC。
    let parity = ((x >> 4) ^ mem.read_ram(crate::ram_map::FRAME_COUNT)) & 1;
    let nd = (d & 0xBF) | (parity << 6);
    let _ = bit6;
    mem.write_ram(0x0400 + x as u16 + 5, nd);
    // 【goal-轮1 公式直证】f797 bit6 实测与公式矛盾——打印 (x>>4)/FC/parity/写值。
    // 【goal-轮35 写面直比】d05 写点全打印（f1838 位差源直证）。
}


/// UpdateEnemy_ForceSpeedTowardsSamus（**asm 内容锚** `: 07:F6B9`——r26 主径）：
/// Enemy82($82)←0→L968B[type]（ENEMY_FORCE_ATTRIBS）→Y→Active 且 bit1=0⇒RTS→
/// dec Ens.data0D 非零⇒RTS→归零：delay 重载+bit7=1⇒x 轴力/0⇒skipXAxis（Zoomer 径）。
/// `UpdateEnemy_Active` 的 **x 轴力**段（asm `07:F6DF`–`07:F70F`，含
/// `EnemyIfMoveFailedHorizontal_Bounce@flipSpeed ; 00:81DA`）：
/// ①`AndEnData05 #~$01` 清 facing；②横滚房先比名表（`LoadEnHiToYAndLoadEorHiToCarry
/// ; 07:F752` = `EnsExtra.hi ^ Samus.hi` 的 bit0 进 carry），异名表 ⇒ facing =
/// `EnsExtra.hi ^ PPUCTRL_ZP` 的 bit0；同名表/竖滚房 ⇒ `cmp Samus.x`：
/// `enX == Samus.x` 时 `inc Enemy82($0082)`，carry(=enX≥Samus.x) 即 facing；
/// ③`OrEnData05` 写 bit0；④`lsr/ror` 把 bit0 挪到 bit7 后与 `speedX` 的符号异或，
/// 同号（已朝萨姆斯）⇒ 返回；异号 ⇒ flipSpeed：`L968B[type]` bit5 置 ⇒ 直接返回
/// （movement-string 型不吃速度翻转），否则按 `L977B[type]` bit7 决定是否先取负
/// `speedSubPixelX`（`sec / lda #0 / sbc` 链——subPixelX≠0 时借位使 speedX 落在
/// `-speedX-1`，与 Y 轴同族的 sbc 链 quirk）。
// `En` 结构字段偏移（**ROM 字节自证**，见 `docs/findings/enemy-xaxis-force-port.md` §11）：
// `+6` = movementInstrIndex / speedSubPixelY / chunkyExplosionTimer 的**同字节别名**；
// 不能按 `constants_ram.asm` 的 `.struct En` 字面顺序读（照字面会把 +6/+7/+8/+9 全部错位）。
pub(crate) const EN_INSTR: u16 = 6;
pub(crate) const EN_SUBPIXEL_X: u16 = 7;
pub(crate) const EN_SUBPIXEL_Y: u16 = 6;
pub(crate) const EN_MOV_IDX: u16 = 8;
pub(crate) const EN_DELAY: u16 = 9;










