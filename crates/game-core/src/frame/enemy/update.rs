//! `UpdateAllEnemies ; 07:F345` / `UpdateEnemy ; 07:F351` —— 敌槽环与七态分派。
//!
//! 本文件由 `frame/mainloop.rs` 原样搬出（纯搬移）：槽环 `$50→$00`（16 字节步长）、
//! 可见性门、`data05` bit6 翻转、JumpEngine 七态（0 ExitSub / 1 Resting / 2 Active /
//! 3 Explode / 4 Frozen / 5 Pickup / 6 Hurt / ≥7 RemoveEnemy）、以及两个已端口的 AI
//! 派发（`skree_ai` / `crawler_ai`）与其辅助件。

use crate::frame::mainloop::*;
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

// 【goal-轮37 记账】可见旗写面（asm :6178-6186 `and #$FD/ora (flag<<1)/sta data05`）
// 在**绘径**（非 F37F 检）——F37F 闩位读面已折；写面随件（DIVERGENCES 记账臂）。
pub(crate) fn enemy_check_if_visible_inner(mem: &mut crate::frame::FrameMemory, x: u8) -> bool {
    // 【goal-轮61 可见位判输入直证】hi/same_nt/carry（金方 bit1=0 在屏却判不可见源）。
    // IsObjectVisible（**asm 内容锚** `IsObjectVisible: ; 07:DFDF`——2026-10-31 忠实重建）：
    // 旧形横房分支 y<240 近似恒真且丢名表/横滚轴——载入邻名表敌（hi≠PPU 当前 NT）被误判
    // 可见 ⇒ Skree（slot2）提前可见+激活（crawlprobe f919-959 残差根因）。asm 判定链：
    // screenY=y−ScrollY/screenX=x−ScrollX → 横房走 HorzScrollCheck（E01C）：入 carry=
    // （x≥ScrollX）（ScreenX sbc 残留）；异名表+carry 置⇒隐形/异名表+carry 清⇒rx+screenX
    // <256 可见；同名表⇒carry 清隐形/rx<screenX 可见。纵房 VertScrollCheck（DFF6）同构
    // （cpy ScrollY 出 carry；异名表 sbc #$0F 带 C=0 ⇒ −16；y 轴检用 radiusX——asm quirk）。
    let y = mem.read_ram(0x0400 + x as u16 + 0);
    let xpos = mem.read_ram(0x0400 + x as u16 + 1);
    let hi = mem.read_wram(0x6AF4 + x as u16 + 7);
    let rx = mem.read_wram(0x6AF4 + x as u16 + 2); // Temp09_RadiusX（y 轴检也用 rx——asm 注 "why is $09"）
    let scroll_y = mem.read_ram(crate::ram_map::SCROLL_Y);
    let scroll_x = mem.read_ram(crate::ram_map::SCROLL_X);
    let screen_y = y.wrapping_sub(scroll_y); // Temp10_ScreenY
    let screen_x = xpos.wrapping_sub(scroll_x); // Temp0E_ScreenX
    let same_nt = (hi ^ mem.read_ram(0x00FF)) & 0x01 == 0;
    if mem.read_ram(crate::ram_map::SCROLL_DIR) & 0x02 != 0 {
        let carry = xpos >= scroll_x; // sec/sbc ScrollX 残留
        if !same_nt {
            if carry {
                return false; // bcs LE036（异名表+敌在滚轴右⇒隐形）
            }
            return (rx as u16 + screen_x as u16) < 0x100; // bcc RTS_E037=可见
        }
        if !carry {
            return false; // bcc LE036
        }
        return rx < screen_x; // bcc RTS_E037=可见
    }
    let carry = y >= scroll_y; // cpy ScrollY 残留
    if !same_nt {
        if carry {
            return false; // bcs LE01A
        }
        let sy2 = screen_y.wrapping_sub(16); // sbc #$0F（C=0 ⇒ −16）
        // asm `clc/adc` 8-bit 和会回绕（cmp 按回绕值判）——verifier 复核角部勘误（正常
        // 半径不可达，位精确对齐）。
        return rx.wrapping_add(sy2) < 0xF0; // cmp #$F0/bcc=可见
    }
    if !carry {
        return false; // bcc LE01A
    }
    rx < screen_y // cmp/bcc=可见
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

pub(crate) fn enemy_force_speed_x_axis(mem: &mut crate::frame::FrameMemory, x: u8, attribs: u8) {
    let e = 0x0400 + x as u16 + 5;
    mem.write_ram(e, mem.read_ram(e) & !0x01); // AndEnData05 #~$01
    let bit0 = enemy_facing_towards_samus_bit(mem, x);
    let d = mem.read_ram(e);
    mem.write_ram(e, d | bit0); // OrEnData05
    // lsr/ror：bit0 → bit7；再 eor speedX 的符号位
    let facing_left = bit0 != 0;
    let sx = mem.read_ram(0x0400 + x as u16 + 3); // Ens.speedX
    if facing_left == (sx & 0x80 != 0) {
        return; // bpl @endIf_D（已朝萨姆斯）
    }
    if attribs & 0x20 != 0 {
        return; // flipSpeed 首件：L968B bit5 ⇒ rts
    }
    let tbl = load_table_at_977b(mem, x); // A = L977B[type]（carry = 原字节 bit7）
    let mut c_in = 1u8; // sec
    if tbl & 0x80 != 0 {
        // **speedSubPixelX = En+7**（ROM 自证 `$81E8: A9 00 / FD 07 04 / 9D 07 04`；
        // `+9` 是 `delay`——r9 复核抓出本行原写 +9，会把 delay 取负）。
        let s = mem.read_ram(0x0400 + x as u16 + EN_SUBPIXEL_X);
        mem.write_ram(0x0400 + x as u16 + EN_SUBPIXEL_X, s.wrapping_neg());
        c_in = if s == 0 { 1 } else { 0 }; // sbc 借位传出（无借位 ⇔ s==0）
    }
    let v = sx.wrapping_neg().wrapping_sub((1 - c_in) as u8);
    mem.write_ram(0x0400 + x as u16 + 3, v);
}

/// x 轴朝向位（`07:F6F5`–`07:F70A`）：横滚房异名表 ⇒ `EnsExtra.hi ^ PPUCTRL_ZP`
/// 的 bit0；否则 `enX ≥ Samus.x`（`enX == Samus.x` 时先 `inc Enemy82($0082)`）。
pub(crate) fn enemy_facing_towards_samus_bit(mem: &mut crate::frame::FrameMemory, x: u8) -> u8 {
    let dir = mem.read_ram(crate::ram_map::SCROLL_DIR);
    if dir >= 0x02 {
        let en_hi = mem.read_wram(0x6AF4 + x as u16 + 7); // EnsExtra.hi
        let sam_hi = mem.read_ram(crate::ram_map::SAMUS_HI);
        if (en_hi ^ sam_hi) & 0x01 != 0 {
            return (en_hi ^ mem.read_ram(crate::ram_map::PPUCTRL_ZP)) & 0x01;
        }
    }
    let ex = mem.read_ram(0x0400 + x as u16 + 1);
    let sx = mem.read_ram(crate::ram_map::SAMUS_X);
    if ex == sx {
        let v = mem.read_ram(0x0082);
        mem.write_ram(0x0082, v.wrapping_add(1)); // inc Enemy82（不影响 carry）
    }
    (ex >= sx) as u8
}

pub(crate) fn enemy_force_speed(mem: &mut crate::frame::FrameMemory, x: u8) {
    mem.write_ram(0x0082, 0);
    let t = mem.read_wram(0x6AF4 + x as u16 + 14) as usize & 0x0F;
    let attribs = game_data::assets::enemy_force_attribs(crate::frame::cur_area(mem))[t];
    let status = mem.read_wram(0x6AF4 + x as u16);
    if status == 2 && attribs & 0x02 == 0 {
        return;
    }
    let d0d = mem.read_ram(0x0400 + x as u16 + 13);
    if d0d.wrapping_sub(1) != 0 {
        mem.write_ram(0x0400 + x as u16 + 13, d0d.wrapping_sub(1));
        return;
    }
    let delay = game_data::assets::enemy_force_speed_delay(crate::frame::cur_area(mem)).get(t).copied().unwrap_or(1);
    mem.write_ram(0x0400 + x as u16 + 13, delay);
    if attribs & 0x80 != 0 {
        // x 轴力（asm `UpdateEnemy_Active` 内 F6DF–F70F；占位臂已端口 2026-11-14）
        enemy_force_speed_x_axis(mem, x, attribs);
    }
    // @skipXAxis（F70F）y 轴力：data05 bit2 清+名表/位比较→bit2 置+速向力
    let e = 0x0400 + x as u16 + 5;
    let d = mem.read_ram(e);
    mem.write_ram(e, d & !0x04); // AndEnData05(~$04)
    let dir = mem.read_ram(crate::ram_map::SCROLL_DIR);
    let mut en_under = false;
    if dir < 0x02 {
        // 竖房：同名表判（简化径：y 比较）
        let en_y = mem.read_ram(0x0400 + x as u16 + 0);
        let sam_y = mem.read_ram(0x030D);
        if en_y != sam_y {
            en_under = en_y > sam_y; // carry=敌在萨姆斯下
        } else {
            mem.write_ram(0x0082, mem.read_ram(0x0082).wrapping_add(2)); // Enemy82 bit1
        }
    } else {
        let en_y = mem.read_ram(0x0400 + x as u16 + 0);
        let sam_y = mem.read_ram(0x030D);
        if en_y != sam_y {
            en_under = en_y > sam_y;
        } else {
            mem.write_ram(0x0082, mem.read_ram(0x0082).wrapping_add(2));
        }
    }
    if en_under {
        mem.write_ram(e, mem.read_ram(e) | 0x04); // OrEnData05 bit2
    }
    // 速向力（face bit2 vs speedY 符号——**异向⇒翻**（asm F73E `eor speedY/bpl RTS`：
    // bit2⊕sy₇=0 跳过）——【2026-10-31 条件取反修】旧形 `want_up=bit2==0` 把判据写反
    // ⇒敌在萨姆斯上方（bit2=0、speedY=0）时误翻 ⇒data02 残留 0x80（emu 恒 00——
    // crawlprobe f797 残差字节根因）。
    let d = mem.read_ram(e);
    let face_up = d & 0x04 != 0; // bit2=1=朝上（敌在萨姆斯下——OrEnData05 刚写）
    let sy = mem.read_ram(0x0400 + x as u16 + 2); // speedY
    if face_up != (sy & 0x80 != 0) {
        // flipSpeed（asm $820F 垂直形）：L968B bit5 门（置位⇒跳过——movement-string 型）
        // →977b 表 bit7 门（加速型⇒sub-pixel 取负）→speedY 取负。sbc 链带 sec 进位
        // （0−x=补码；sub-pixel sbc 的借位传入 speedY sbc——s≠0 时 speedY'=!sy quirk）。
        if attribs & 0x20 == 0 {
            let tbl = load_table_at_977b(mem, x);
            let mut c_in = 1u8; // sec（asm $821B）
            if tbl & 0x80 != 0 {
                // **speedSubPixelY = En+6**（与 movementInstrIndex 同字节别名；ROM 自证
                // `$8343: 7D 06 04 / 9D 06 04`、`$822C: FD 06 04 / 9D 06 04`；`+8` 是
                // movementIndex——r9 复核抓出本行原写 +8）。
                let s = mem.read_ram(0x0400 + x as u16 + EN_SUBPIXEL_Y);
                mem.write_ram(0x0400 + x as u16 + EN_SUBPIXEL_Y, s.wrapping_neg());
                c_in = if s == 0 { 1 } else { 0 }; // sbc 借位传出（无借位⇔s==0）
            }
            let v = sy.wrapping_neg().wrapping_sub((1 - c_in) as u8);
            mem.write_ram(0x0400 + x as u16 + 2, v); // speedY 取负（sbc 链忠实）
        }
    }
}


/// UpdateEnemy_EnData05DistanceToSamusThreshold（**asm 内容锚** `: 07:F75B`——r26 全径）：
/// ①data05 bits3/4 置+掩码默认 ②阈表（ENEMY_DISTANCE_THRESHOLD[type]）为 0⇒早出
/// （**Zoomer 径！**）③bit1（不可见）⇒清 bits3/4 ④bit7 轴选→掩 bit3/4 ⑤|敌-萨|>>4
/// vs 阈&$7F：距大⇒清对应位。Or/AndEnData05=bits3/4 掩写件。
pub(crate) fn enemy_distance_threshold(mem: &mut crate::frame::FrameMemory, x: u8) {
    let e = 0x0400 + x as u16 + 5; // Ens.data05
    let d = mem.read_ram(e);
    mem.write_ram(e, d | 0x18); // OrEnData05($18)
    let t = mem.read_wram(0x6AF4 + x as u16 + 14) as usize & 0x0F;
    let thr = game_data::assets::enemy_distance_threshold(crate::frame::cur_area(mem))[t];
    if thr == 0 {
        return; // 阈 0⇒早出（Zoomer 径！）
    }
    let d = mem.read_ram(e);
    if d & 0x02 == 0 {
        // 不可见⇒清 bits3/4（Lx345）
        mem.write_ram(e, d & !0x18);
        return;
    }
    // asm `ldy #~$08` 默认；`asl / bcs Lx342`——阈 bit7 置 ⇒ 掩 bit3，否则 `ldy #~$10` 掩 bit4。
    let mask = if thr & 0x80 != 0 { !0x08u8 } else { !0x10u8 };
    let thr_v = thr & 0x7F;
    let d = mem.read_ram(e);
    // asm：轴由 d05 bit7 选（置⇒y 轴，清⇒x 轴）
    let (sam_pos, en_pos) = if d & 0x80 != 0 {
        (mem.read_ram(crate::ram_map::SAMUS_Y), mem.read_ram(0x0400 + x as u16))
    } else {
        (mem.read_ram(crate::ram_map::SAMUS_X), mem.read_ram(0x0400 + x as u16 + 1))
    };
    // 【2026-11-15 修】Samus.hi 原读 $6AF4（=EnsExtra 槽 0 的 status）——真址 Objects 槽 0 的
    // hi = $030C（constants_ram Objects 结构 +0x0C）。错址令阈值判据整体失真。
    let sam_hi = mem.read_ram(crate::ram_map::SAMUS_HI) & 1;
    let en_hi = mem.read_wram(0x6AF4 + x as u16 + 7) & 1; // EnsExtra.hi（$6AFB 起）
    // asm `lsr / ror Temp`：位置右移 1 位、hi 位旋入 bit7 ⇒ 值 = (hi*256+pos)/2（8 位模）
    let sam = ((sam_hi << 7) | (sam_pos >> 1)) as u8;
    let en = ((en_hi << 7) | (en_pos >> 1)) as u8;
    // `sec / sbc` 8 位；负⇒二补取绝对值；再 `lsr`×3 ⇒ 合计 /16
    let diff = en.wrapping_sub(sam);
    let abs = if (diff as i8) < 0 { diff.wrapping_neg() } else { diff };
    let dist = abs >> 3;
    if dist >= thr_v {
        mem.write_ram(e, d & mask); // 距大⇒清对应位
    }
}


/// InitEnRestingAnimIndex（**asm 内容锚** `: 07:F682`——r26）：type×2+facing 索引→
/// ENEMY_RESTING_ANIM[y]；≠resetAnimIndex⇒写 resetAnimIndex/animIndex+animDelay←0。
pub(crate) fn init_en_resting_anim_index(mem: &mut crate::frame::FrameMemory, x: u8) {
    let y = enemy_type_times2_plus_facing(mem, x);
    let idx = game_data::assets::enemy_resting_anim(crate::frame::cur_area(mem))[y as usize & 0x1F];
    if idx == mem.read_wram(0x6AF4 + x as u16 + 5) {
        return; // 已同⇒RTS（animDelay 不清）
    }
    mem.write_wram(0x6AF4 + x as u16 + 5, idx); // resetAnimIndex
    mem.write_wram(0x6AF4 + x as u16 + 6, idx); // animIndex
    mem.write_wram(0x6AF4 + x as u16 + 4, 0); // animDelay←0
}

/// GetEnemyTypeTimes2PlusFacingDirection（**asm 内容锚**——索引=type×2+facing（data05 bit0?））。
pub(crate) fn enemy_type_times2_plus_facing(mem: &mut crate::frame::FrameMemory, x: u8) -> u16 {
    let t = mem.read_wram(0x6AF4 + x as u16 + 14) as u16; // EnsExtra.type
    // 【实锤②修（2026-10-29）】asm GetEnemyTypeTimes2PlusFacingDirection（$F844）=
    // **axis 感应 facing**（`lda data05/bpl @common/lsr lsr/@common: lsr/lda type/rol`）
    // ——bit7 置（竖轴）⇒carry=bit3/清⇒carry=bit0——旧形固定 bit0（注带「?」）
    // ⇒顶爬虫（竖轴）读错位⇒EnemyRestingAnim 表错项⇒**持续错帧**（f800 106px 根因）。
    let d05 = mem.read_ram(0x0400 + x as u16 + 5);
    let facing: u16 = if d05 & 0x80 != 0 { ((d05 >> 3) & 1) as u16 } else { (d05 & 1) as u16 };
    (t * 2 + facing) & 0x1F
}


/// LoadTableAt977B（**asm 内容锚** `areas_common.asm:184 $80B0`——L977B[type]×2）。
pub(crate) fn load_table_at_977b(mem: &mut crate::frame::FrameMemory, x: u8) -> u8 {
    let t = mem.read_wram(0x6AF4 + x as u16 + 14) as usize;
    let e = game_data::assets::enemy_facing(crate::frame::cur_area(mem))[t & 0x3F];
    e.wrapping_shl(1)
}
