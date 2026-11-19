//! 相机滚动行为：`ScrollLeft ; 07:E69E` / `ScrollRight ; 07:E6D2` / `ScrollUp ; 07:E4F1` /
//! `ScrollDown ; 07:E519` 与名表流式装载（列/行 kick）。
//!
//! 这些函数原本住在 `samus/physics.rs`——它们与萨姆斯只有一处耦合（由碰撞结果触发），
//! 语义上属于相机：唯一允许写 `SCROLL_*` / `MAP_POS_*` / 名表的一方。

use crate::frame::Manager;
use crate::ram_map;

/// `SCRN_VY` = 240（一屏高）。asm 宏值见 `constants.asm`（`SCRN_VY = $F0`）。
const SCRN_VY: u8 = 240;

/// CheckUpdateNameTable（**asm 内容锚** `CheckUpdateNameTable: ; 07:E54A`）：
/// ①`jsr SetupRoom`（**邻房装载**——RoomNumber≠FF（任务未竟）⇒出；多帧装载径）②横滚
/// 列流送门（`ScrollX&7==阈值`（右 0/左 7））③EndOfRoomHorizontal（`ScrollDir==
/// TempScrollDir($4A)` 防重流）④UpdateNameTable（E590）：列流送——tile index=
/// (ScrollX&0xF8)/8；源=RoomRAM 缓冲（$6000/$6400 按名表位（EB85：PPUCTRL^ScrollDir））/
/// 目标=PPU 名表（$2000/$2C00——PPUAddrs 表 E560）；30 块竖写（控制位 $1E|$80）。
/// 【V2-lite】流送直写 PPU 面（金径=VRAMStructBuffer+NMI 刷（WritePPUByte $C36B）——
/// 行级等价；缓冲面 $07A0+ 的 L3 对拍差异随 V2 收敛）。
pub(crate) fn check_update_name_table(mem: &mut Manager) {
    // ①邻房装载（E54A 首件——ScrollRight/Left 每像素调；多帧（RoomNumber 任务机））
    // 【载房窗 bog（2026-10-31 重考定谳）】ScrollRight/Left 的 ScrollX==0 特殊径（asm
    // E6E4-E6EA：inc MapPosX+GetRoomNum）⇒ RoomNumber←新地图格房号 ⇒ 本调用 setup_room
    // **全房重载**（InitTables+DrawRoom+EnemyLoop×LoadEnemy+EndOfRoom+kick）——emu 实测
    // 每次载房=2 bogged 窗（crawlprobe f916-917 FC 冻 132（0x84→f918=0x85 单 inc）/
    // TD 冻 9；goal-r22 注 f1064-1065 同族第二窗）。「逐列写 bog」归因已证伪摘除
    // （本文件 2026-10-28 更正注），载房窗归因正确但当时未落门。NMIStatus=1⇒NMI 内块跳
    // （asm prg7:251）。FC 不走 iter_span_fc（滚动迭代=普通游戏迭代——main_loop_iter 尾
    // inc 即该迭代唯一一次；emu 实录 f915=132→f918=133→f919=134 每 iteration 单 inc；
    // span_fc 只适用于工作拍 FC 被冻结的 title 臂形——双加=+1 相位差根因，f918 实录）。
    let rn = mem.room_number();
    // 【多帧载房任务机（2026-11-19 A-lite）】窗内 `RoomNumber` 保持**要载的房号**（emu 帧边界
    // 实录），所以「是否又是载房请求」不能只看 `RoomNumber`——必须叠加任务机相位：窗已开时
    // 后续像素的 `check_update_name_table` 不得再次触发整房装载。
    let is_load = rn != 0xFF && rn < 0xF0 && mem.room_load_phase == 0;
    if is_load {
        mem.work_cycles = 0; // 周期累加起点（载房工作量——room.rs LOAD_* 模型）
    }
    crate::frame::tick::setup_room(mem);
    if is_load {
        // 开窗：`draw_room` 尾巴的 `EndOfRoom ; 07:EAF4` 把 RoomNumber 置了 $F0，但 emu 在整个
        // 冻结窗里保持**房号**（room8：f900–f903 恒 8）⇒ 这里改回房号，$F0 留到窗末。
        crate::world::room_load::begin_room_load_window(mem, rn);
        mem.set_room_number(rn);
    }
    if is_load {
        // 【缝会计卡·掉帧窗模型（2026-11-05——findings/morphball-trajectory-root.md
        // §12）】旧形硬编码 2——emu 实测随工作量变（room23（504 tile）=2 / room8
        // （1024 tile）=3——morphball 812/1344 双窗实录；「每次载房=2」系 crawlprobe
        // 2 例校准的以偏概全（2026-11-05 证伪更正））。机制=工作超帧跨 NMI 数：
        // dropped=floor((基底+载房工作周期)/29780)（常量=asm 6502 逐条计数——
        // room.rs LOAD_* 注）。行为后果=消费端输入相位（同 FC 迭代喂 joy[label]——
        // 窗长差 1 ⇒转脚帧 accel 滞 1 帧（goal-轮134 下游链））。
        let dropped = (crate::world::room_draw::LOAD_BASE_CYCLES + mem.work_cycles)
            / crate::world::room_draw::FRAME_CYCLES;
        // 【case-002 归因卡·临时探针（2026-11-05）】载房工作量+计数实测。
        mem.iter_bogged(dropped.min(255) as u8);
        // 【f673 载房超帧卡（2026-11-19）】本帧被载房窗吞掉：asm 的像素写点
        // （`Lx180: ; 07:E65E` / `inc Samus.x ; go right` = `07:E692`）在 `jsr ScrollLeft`
        // **之后**，故该写必然落到装载窗结束那一帧（emu 探针 f673 DECx=0、f674/f675 纯
        // lag、f676 DECx=1 落地）。标位由 `move_samus_left/right` 消费（挂起 1 px），
        // 由 `Engine::end_of_tick_gate` 门队列排空那一拍兑现。
        mem.samus().set_load_consumed_frame(true);
    }
    if mem.room_number().wrapping_add(1) != 0 {
        return; // RoomNumber≠FF：装载任务未竟
    }
    let dir = mem.read_ram(crate::ram_map::SCROLL_DIR);
    if dir & 2 == 0 {
        // 竖滚径（CheckUpdateNameTableVertical ; 07:E571 → EndOfRoomVertical $E57C）
        // 阈值表 `CheckUpdateNameTableVertical_ScrollYThresholds ; 07:E55E` = `07 00`
        // （ScrollDir 0=向上⇒7 / 1=向下⇒0）。
        let threshold: u8 = if dir == 1 { 0 } else { 7 };
        if mem.read_ram(crate::ram_map::SCROLL_Y) & 0x07 != threshold {
            return;
        }
        if dir != mem.read_ram(0x004A) {
            return; // TempScrollDir 防重流
        }
        stream_row_kick(mem);
        return;
    }
    let threshold: u8 = if dir == 3 { 0 } else { 7 };
    if mem.read_ram(crate::ram_map::SCROLL_X) & 0x07 != threshold {
        return;
    }
    if dir != mem.read_ram(0x004A) {
        return; // TempScrollDir 防重流
    }
    // ④列流送（E590——30 块竖写：PPU inc=32）——共用核（stream_column_kick）
    stream_column_kick(mem);
}

/// 列写核（EndOfRoomHorizontal（$E70C）/CheckUpdateNameTableHorizontal 共用径
/// （audit（scroll-column-streaming.md §1）））：Temp00=(ScrollX&0xF8)/8（列 idx）；
/// GetNameAddrs（GetNT=(PPUCTRL^ScrollDir)&1→PPU $20/$2C+RoomRAM $60/$64）；
/// 30 行竖写（dst+idx+row*32（PPU inc=32 形））。
pub fn stream_column_kick(mem: &mut Manager) {
    // 【证伪更正（2026-10-28）】goal-r22「列写=PPU 重活⇒NMI bogged 2 帧」=**误归因**
    // ——emu 实测 f916-917/f1064-1065 各停 2 帧=**载房窗 bog**（GetRoomNum 越界载房
    // （more_init 序列 [1,0,0,1] 已模））非逐列写 bog。逐列写 bog⇒移动相移：
    // L4 实证（摘除前后）：f950 18574→1134px（回基线）/l380 8645→1105px。**已永久移除**。
    // 【asm 正源】VRAMStructWrite 入口复位地址锁存（$2002 读（render.rs reset_addr_latch
    // 注））——防 2006 对相位错位（列写落错址根因面）。
    mem.render.reset_addr_latch();
    let dir = mem.read_ram(crate::ram_map::SCROLL_DIR);
    let idx = (mem.read_ram(crate::ram_map::SCROLL_X) & 0xF8) >> 3;
    let y = (mem.read_ram(0x00FF) ^ dir) & 1; // GetNameTableAtScrollDir（EB85）
    let src = 0x6000u16 + (y as u16) * 0x400; // RoomRAMAddrs（E562）
    let dst = if y == 0 { 0x2000u16 } else { 0x2C00 }; // PPUAddrs（E560）
    for row in 0..30u16 {
        let addr = dst + idx as u16 + row * 32;
        let v = mem.read_wram(src + idx as u16 + row * 32);
        mem.write_io(0x2006, (addr >> 8) as u8);
        mem.write_io(0x2006, (addr & 0xFF) as u8);
        mem.write_io(0x2007, v);
    }
}

/// 行写核（`EndOfRoomVertical ; 07:E57C` → `UpdateNameTable ; 07:E590` 的竖滚分支）：
/// `Temp00 = (ScrollY & $F8) << 2`（= 行号×32，RoomRAM/PPU 同偏移）；`GetNameAddrs`
/// 给 NT 高低字节；control byte = `@controlBitsTable[ScrollDir>>1]` = **$20**
/// （PPU inc=1、长度 32）⇒ 竖滚一次写**一整行 32 字节线性**（与横滚的 30 行竖写对偶）。
pub fn stream_row_kick(mem: &mut Manager) {
    mem.render.reset_addr_latch();
    let dir = mem.read_ram(crate::ram_map::SCROLL_DIR);
    let off = ((mem.read_ram(crate::ram_map::SCROLL_Y) & 0xF8) as u16) << 2;
    let y = (mem.read_ram(0x00FF) ^ dir) & 1; // GetNameAddrs（$E564 经 GetNameTableAtScrollDir）
    let src = 0x6000u16 + (y as u16) * 0x400 + off;
    let dst = if y == 0 { 0x2000u16 } else { 0x2C00 } + off;
    for i in 0..32u16 {
        let v = mem.read_wram(src + i);
        let a = dst + i;
        mem.write_io(0x2006, (a >> 8) as u8);
        mem.write_io(0x2006, (a & 0xFF) as u8);
        mem.write_io(0x2007, v);
    }
}

/// GetRoomNum 横滚门（asm E720——门检+图格查房已并入 get_room_num 单径）。返回
/// true=可滚（asm carry 反形）。
pub(crate) fn scroll_block_gate(mem: &mut Manager) -> bool {
    crate::frame::tick::get_room_num(mem)
}

/// ScrollLeft（**asm 内容锚** `ScrollLeft: ; 07:E6A7`）：滚屏 1px。返回 carry
/// （true=滚动成功——屏内 x 不动；false=不能滚）。
pub(crate) fn scroll_left(mem: &mut Manager) -> bool {
    let dir = mem.camera().dir();
    if dir != crate::world::camera::dir::LEFT {
        if dir != crate::world::camera::dir::RIGHT {
            return false; // 竖滚不能横滚（@cantScroll sec）
        }
        mem.camera().set_dir(crate::world::camera::dir::LEFT); // dec ScrollDir（3→2=左）
        if mem.camera().scroll_x() != 0 {
            let mx = mem.camera().map_x().wrapping_sub(1);
            mem.camera().set_map_x(mx);
        }
    }
    if mem.camera().scroll_x() == 0 {
        let mx = mem.camera().map_x().wrapping_sub(1);
        mem.camera().set_map_x(mx);
        if !scroll_block_gate(mem) {
            // @atLeftBound：MapPosX 复位（inc）——CF=1 不能滚
            let mx = mem.camera().map_x().wrapping_add(1);
            mem.camera().set_map_x(mx);
            return false;
        }
        toggle_name_table(mem);
    }
    let sx = mem.camera().scroll_x().wrapping_sub(1);
    mem.camera().set_scroll_x(sx);
    check_update_name_table(mem);
    true
}

/// ScrollRight（**asm 内容锚** `ScrollRight: ; 07:E6D2`）：滚屏 1px。返回 carry。
/// 【goal-轮47 asm 理解（用户令：问题定位优先读 refs/m1disasm）】E6D2 全径语义：
/// ScrollDir 2=左/3=右（cmp #$03 beq 续/`cmp #$02 bne` 竖滚禁横+inc ScrollDir 转向）
/// +ScrollX==0 特殊径（inc MapPosX/GetRoomNum CF=1⇒界（dec MapPosX/sec））+inc ScrollX
/// +0 环绕⇒ToggleNameTable+CheckUpdateNameTable——**MapPosX/名表切/界判定三联动**。
pub(crate) fn scroll_right(mem: &mut Manager) -> bool {
    let dir = mem.camera().dir();
    if dir != crate::world::camera::dir::RIGHT {
        if dir != crate::world::camera::dir::LEFT {
            return false; // 竖滚不能横滚
        }
        mem.camera().set_dir(crate::world::camera::dir::RIGHT); // inc ScrollDir（2→3=右）
        if mem.camera().scroll_x() != 0 {
            let mx = mem.camera().map_x().wrapping_add(1);
            mem.camera().set_map_x(mx);
        }
    }
    if mem.camera().scroll_x() == 0 {
        let mx = mem.camera().map_x().wrapping_add(1);
        mem.camera().set_map_x(mx);
        if !scroll_block_gate(mem) {
            // @atRightBound：MapPosX 复位（dec）——CF=1 不能滚
            let mx = mem.camera().map_x().wrapping_sub(1);
            mem.camera().set_map_x(mx);
            return false;
        }
    }
    let sx = mem.camera().scroll_x().wrapping_add(1);
    mem.camera().set_scroll_x(sx);
    if sx == 0 {
        toggle_name_table(mem); // inc 溢出（FF→00）⇒名表换
    }
    check_update_name_table(mem);
    true
}

pub(crate) fn scroll_up(mem: &mut Manager) -> bool {
    let dir = mem.read_ram(crate::ram_map::SCROLL_DIR);
    if dir != 0 {
        if dir != 1 {
            return false; // 横滚不能竖滚（@cantScroll）
        }
        mem.write_ram(crate::ram_map::SCROLL_DIR, 0);
        if mem.read_ram(crate::ram_map::SCROLL_Y) != 0 {
            mem.write_ram(crate::ram_map::MAP_POS_Y,
                          mem.read_ram(crate::ram_map::MAP_POS_Y).wrapping_sub(1));
        }
    }
    let mut x = mem.read_ram(crate::ram_map::SCROLL_Y);
    if x == 0 {
        mem.write_ram(crate::ram_map::MAP_POS_Y,
                      mem.read_ram(crate::ram_map::MAP_POS_Y).wrapping_sub(1));
        if !scroll_block_gate(mem) {
            // @atTopBound：上界（GetRoomNum CF=1）——MapPosY 复位
            mem.write_ram(crate::ram_map::MAP_POS_Y,
                          mem.read_ram(crate::ram_map::MAP_POS_Y).wrapping_add(1));
            return false;
        }
        toggle_name_table(mem);
        x = SCRN_VY;
    }
    x = x.wrapping_sub(1);
    // ScrollVertically_Merge
    mem.write_ram(crate::ram_map::SCROLL_Y, x);
    check_update_name_table(mem);
    true
}

/// ScrollDown（**asm 内容锚** `ScrollDown: ; 07:E519`——prg7:7549-7556）：竖滚 1px 向下。
///
/// ```text
/// ldx ScrollDir / dex / beq @currentlyScrollingDown  ; 1=正在向下
///   bpl cantScroll                                    ; 2/3=横滚⇒禁
///   inc ScrollDir              ; 0（向上）⇒1
///   lda ScrollY / beq @currentlyScrollingDown
///   inc MapPosY
/// @currentlyScrollingDown:
///   lda ScrollY / bne @noNewRoom
///     inc MapPosY / jsr GetRoomNum / bcs atBottomBound
/// @noNewRoom:
///   ldx ScrollY / cpx #SCRN_VY-1 / bne @noNameTableSwitch
///     jsr ToggleNameTable / ldx #$FF
/// @noNameTableSwitch: inx / ScrollVertically_Merge: stx ScrollY
/// ```
pub(crate) fn scroll_down(mem: &mut Manager) -> bool {
    let dir = mem.read_ram(crate::ram_map::SCROLL_DIR);
    if dir != 1 {
        if dir != 0 {
            return false; // 横滚（2/3）不能竖滚（dex 后 bpl ⇒ cantScroll）
        }
        mem.write_ram(crate::ram_map::SCROLL_DIR, 1);
        if mem.read_ram(crate::ram_map::SCROLL_Y) != 0 {
            mem.write_ram(crate::ram_map::MAP_POS_Y,
                          mem.read_ram(crate::ram_map::MAP_POS_Y).wrapping_add(1));
        }
    }
    if mem.read_ram(crate::ram_map::SCROLL_Y) == 0 {
        mem.write_ram(crate::ram_map::MAP_POS_Y,
                      mem.read_ram(crate::ram_map::MAP_POS_Y).wrapping_add(1));
        if !scroll_block_gate(mem) {
            // atBottomBound：下界——MapPosY 复位
            mem.write_ram(crate::ram_map::MAP_POS_Y,
                          mem.read_ram(crate::ram_map::MAP_POS_Y).wrapping_sub(1));
            return false;
        }
    }
    let mut x = mem.read_ram(crate::ram_map::SCROLL_Y);
    if x == SCRN_VY - 1 {
        toggle_name_table(mem);
        x = 0xFF;
    }
    x = x.wrapping_add(1);
    mem.write_ram(crate::ram_map::SCROLL_Y, x);
    check_update_name_table(mem);
    true
}

/// ToggleNameTable（**asm 内容锚** `ToggleNameTable: ; 07:E9B7`）：PPUCTRL_ZP^=3。
pub(crate) fn toggle_name_table(mem: &mut Manager) {
    let ff = mem.read_ram(0x00FF) ^ 0x03;
    mem.write_ram(0x00FF, ff);
}