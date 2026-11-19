//! 房间装载与区域初始化：MoreInit/房间链、CopyAreaPointers、GetRoomNum、SetupRoom、屏开关。
//!
//! 由 `frame/mainloop.rs` 原样搬出（纯搬移）。asm 根：`MoreInit ; 07:C81D`、
//! `CopyAreaPointers 07:C8B0`、`GetRoomNum ; 07:E720`、`SetupRoom ; 07:EA2B`、`ScreenNmiOff ; 07:C45D`。

use crate::frame::tick::*;
use crate::frame::Manager;
use crate::ram_map;

/// MoreInit（**asm 内容锚** `MoreInit: ; 07:C81D`——区域/房间链头）：
/// PaletteDataPending←1/SpareMem75←$FF/四清（AtEnding/DoorEntryStatus/SamusDoorData/
/// UpdatingSamusProjectile←0）/清环（$7A-$DE+$300-$3FE——**off-by-one bug=D4 保真**
/// （「should clear $DF」注））/ScreenOff/ClearNameTables（已折）/EraseAllSprites（已折）
/// /ScrollBlock 族清/ScrollDir←2/AreaMapPos→MapPos/PaletteToggle/RoomNumber←$FF/
/// CopyAreaPointers→GetRoomNum→SetupRoom 环（房间链——嵌套桩）。
pub fn more_init(mem: &mut Manager) {
    mem.write_ram(crate::ram_map::PALETTE_PENDING, 1); // Palette00+1
    mem.write_ram(0x0075, 0xFF);
    for a in [crate::ram_map::AT_ENDING, 0x0056, 0x0057, 0x0055] {
        mem.write_ram(a, 0); // 四清（DoorEntryStatus=$56/SamusDoorData=$57/UpdatingSamusProjectile=$55）
    }
    // 清环（$7A-$DE+$300-$3FE——off-by-one bug 保真（D4））
    for a in 0x7A..=0xDEu16 {
        mem.write_ram(a, 0);
    }
    for a in 0x0300..=0x03FEu16 {
        mem.write_ram(a, 0);
    }
    // 【F4 整改（2026-11-12）】asm `jsr ScreenOff ;($C439)`（prg7_engine.asm:1871，清环后
    // ClearNameTables 前）= `lda PPUMASK_ZP($FE) / and #~(BG_ON|OBJ_ON)`——影子寄存器清
    // 渲染位；真 PPUMASK 由 NMI 内 WritePPUCtrl 落。rust 旧形漏此写（r9-F4）。
    let pm = mem.read_ram(0x00FE) & !0x18;
    mem.write_ram(0x00FE, pm);
    mem.write_ram(crate::ram_map::NMI_STATUS, 0); // ScreenOff 域形（屏/可见性关）
    // 【move-drift R2】ScreenOff（$C439→WriteAndWait→WaitNMIPass_ prg7:1129-1140）=
    // **帧等待**（1 tick+NMI 内块全跑）——门队列声明（窗3 等待 1/3）。
    mem.iter_wait();
    crate::frame::boot::clear_name_tables(mem); // ClearNameTables（已折）
    crate::frame::boot::erase_all_sprites(mem); // EraseAllSprites（已折）
    // 名表门挡双清（`stx ScrollBlockOnNameTable3` / `stx ScrollBlockOnNameTable0`，
    // X=0——prg7_engine.asm:1876-1877）。旧形误写 `$5A RoomNumber` 与 `$53 WalkSoundDelay`：
    // `$5A` 在本函数后段反正写 `$FF`（无差），`$53` 是步音延时计数器、MoreInit 不碰它；
    // 真正的 `$6C/$6D` 则整场没人清 ⇒ 区域切换后残留门挡位，`get_room_num` 读 `$6C+y`
    // 命中门挡位即拒滚（Fix1-B）。
    clear_scroll_blocks(mem);
    mem.write_ram(0x0030, 1); // SpareMem30
    mem.write_ram(crate::ram_map::SCROLL_DIR, 2); // ScrollDir=2（横房）——【勘误修】$49！
    // AreaMapPosX/Y（各 area bank $95D7/$95D8）——**走区头段**，不再有 Rust 抄本。
    let hdr = game_data::assets::area_header((mem.read_ram(crate::ram_map::IN_AREA) & 0x0F) as u8);
    mem.write_ram(0x0050, hdr.map_x); // MapPosX（constants_ram:543）
    mem.write_ram(0x004F, hdr.map_y); // MapPosY（constants_ram:542）
    // AreaPaletteToggle（各 area bank $95DA，ROM）——**旧读 $6885 是错址**（WRAM 项扫计数区
    // ⇒ 恒 0 ⇒ 电梯换区 `PaletteToggle^=7` 后 pending 差 1 ⇒ 载错调色板；L4 elevprobe2 实证）。
    let pt = game_data::assets::area_pal_toggle(mem.read_ram(crate::ram_map::IN_AREA))
        .unwrap_or_else(|| crate::unported!("bank07:CA30-pal-toggle"));
    mem.write_ram(0x0076, pt); // PaletteToggle（constants_ram:609）
    mem.set_room_number(0xFF); // RoomNumber=$FF（constants_ram:567——undefined room）
    copy_area_pointers(mem); // C8B0（已端口）
    get_room_num(mem); // E720（已端口——壳）
    // 【r25 对拍根】asm LC86F 房间环：`jsr SetupRoom/ldy RoomNumber/iny/bne LC86F`
    // ——循环至 RoomNumber==FF（多房装载序）。
    loop {
        setup_room(mem); // EA2B（DrawRoom 未端口面=fail-loud 随爆）
        if mem.room_number().wrapping_add(1) == 0 {
            break;
        }
    }
    // RoomRAM($6000+)→PPU 名表 #0($2000) 4 页拷贝（asm Lx001 `lda ($00),y/sta PPUDATA`
    // ——**黑屏根件**：名表从 RoomRAM 落 RenderState）。
    // 【move-drift R3】SetupRoom 环+名表拷贝=重活（房间解压——原版掉帧窗 bogged 2 帧；
    // C 级拆分（总 4=A 级三源互证——窗3 行集×FC×$18/$19 计数））。
    mem.iter_bogged(2);
    // 【址勘误 r28-1】asm prg7:1899-1901 `ldy RoomRAMPtr+1 / sty $01 / ldy RoomRAMPtr /
    // sty $00`——RoomRAMPtr=$39/$3A（constants_ram:519-521）；旧形误取 $37/$38（WorkPtr）。
    let room_ptr = mem.room().room_ptr();
    mem.write_ram(0x0000, room_ptr as u8); // RoomRAMPtr lo → $00
    mem.write_ram(0x0001, (room_ptr >> 8) as u8); // RoomRAMPtr hi → $01
    mem.write_io(0x2006, 0x20);
    mem.write_io(0x2006, 0x00);
    let mut hi = mem.read_ram(0x0001);
    for _page in 0..4u16 {
        for i in 0..256u16 {
            // 源址=(hi<<8)|i 的 WRAM 视（$6000 起——全址形（read_wram 契约））
            let addr = ((hi as u16) << 8) | i;
            let b = mem.read_wram(0x6000 | (addr & 0x1FFF));
            mem.write_io(0x2007, b);
        }
        hi = hi.wrapping_add(1);
    }
    mem.write_ram(0x002B, 0); // DoorPaletteChangeDir
    mem.write_ram(crate::ram_map::PALETTE_PENDING, 1);
    mem.write_ram(0x0030, 1); // SpareMem30
    mem.write_ram(crate::ram_map::MAIN_ROUTINE,
                  mem.read_ram(crate::ram_map::MAIN_ROUTINE).wrapping_add(1)); // inc→SamusInit
    // jmp ScreenOn（$C447）：`lda PPUMASK_ZP / ora #$1E / bne WriteAndWait`——
    // `WriteAndWait`（07:C43D）写影子后 `WaitNMIPass_`（07:C43F）等**一个 NMI**，
    // 真 PPUMASK 由该 NMI 内的 `WritePPUCtrl`（07:C44D）落 ⇒ 点亮发生在**下一帧**。
    // 【转场暗窗建模（2026-11-12）】旧形在本 tick 内直接 `$FE|=$1E` ⇒ ScreenOff→…→
    // ScreenOn 同 tick 收完，暗窗消失（emu 实测暗窗 7 帧 f1191–f1197）。现改为延迟
    // 落影：置 `deferred_screen_on`，由帧门队列排空（迭代完成点=本迭代 WaitNMIPass
    // 全部兑现）时写 $FE——影写与真寄存器落点各差一帧，与 asm 两段等待同相位。
    mem.deferred_screen_on = true;
    // 【move-drift R2】ScreenOn（$C447→WriteAndWait——prg7:1142-1146 fallthrough）=
    // **帧等待**（窗3 等待 3/3）。
    mem.iter_wait();
}

/// `ScreenNmiOff ; 07:C45D`（asm 逐行）：影子 `$FE &= ~(BG_ON|OBJ_ON)` → `jsr WriteAndWait`
/// （等一个 NMI）→ 影子 `$FF &= ~PPUCTRL_VBLKNMI_ON` → **`sta PPUCTRL` 写真寄存器**。
///
/// 【相位口径】真 PPUMASK 由 `WriteAndWait` 那个 NMI 内的 `WritePPUCtrl ; 07:C44D` 落
/// （写的是已被清位的影子 ⇒ 暗屏）；本层 tick 原子执行，等价近似=**此刻即写真寄存器**
/// （与 `apply_deferred_screen_on` 同口径）。`$2000` 的 NMI 关位按 asm 也是立刻写。
///
/// 【为什么必须写真寄存器】只在 `$FE` 上清位、不写 `$2001` 时，渲染仍用上一帧的
/// `RenderState.ppumask`（亮）⇒ 换区/进场窗口的**整屏暗窗缺失**。实证：elevprobe2
/// f1206/f1207 emu 全黑 61440 px vs rust 亮屏 13115/12777 px 失配。
pub fn screen_nmi_off(mem: &mut Manager) {
    let pm = mem.read_ram(crate::ram_map::PPUMASK_ZP) & !0x18;
    mem.write_ram(crate::ram_map::PPUMASK_ZP, pm);
    mem.write_io(0x2001, pm); // WriteAndWait 的 NMI 落点（同 tick 近似）
    let pc = mem.read_ram(crate::ram_map::PPUCTRL_ZP) & !0x80;
    mem.write_ram(crate::ram_map::PPUCTRL_ZP, pc);
    mem.write_io(0x2000, pc); // asm `sta PPUCTRL`（NMI 关）
}

/// ScreenOn 延迟落影（**转场暗窗建模**）——帧门队列排空（迭代完成点）时把
/// `deferred_screen_on` 兑现成 `PPUMASK_ZP |= $1E`（asm `WriteAndWait` 写影子；
/// 真 PPUMASK 由下一 tick 的 `WritePPUCtrl`（07:C44D）落）。旗未置时零动
/// （防误触发：正常帧每 tick 都会走到此点）。
pub fn apply_deferred_screen_on(mem: &mut Manager) {
    if !mem.deferred_screen_on {
        return;
    }
    mem.deferred_screen_on = false;
    let fe = mem.read_ram(0x00FE) | 0x1E;
    mem.write_ram(0x00FE, fe); // ScreenOn 影子写（07:C447 `ora #$1E`）
    // 与 ScreenOff 同相位口径：`WriteAndWait` 的等待=NMI 内 `WritePPUCtrl` 落真寄存器，
    // 该帧即已点亮 ⇒ 此处同步写真 0x2001（emu 实测 f1198 复明；见逐帧 probe 实录）。
    mem.write_io(0x2001, fe);
}

/// 区序索引（`InArea & $0F`）。
///
/// **越界 fail-loud**：原版只有 0..=4（Br/No/Kr/To/Ri）。旧形 `% 5` 会把 `$05/$0F`
/// 静默折回 Brinstar/Ridley——数据合法、行为错、且不记账，属最难查的一类。
/// 契约：`InArea` 的 `$10` 位是「进游戏/区号」标记，低 4 位才是区号；低 4 位 ≥5 是移植侧
/// 错误（ROM 不会产生），故在此爆在 `bank07:CA30`（`BankTable` 所在）而不是猜一个区。
pub fn area_idx(in_area: u8) -> usize {
    match in_area & 0x0F {
        0..=4 => (in_area & 0x0F) as usize,
        _ => crate::unported!("bank07:CA30-area_idx"),
    }
}

/// CopyMap（**asm 内容锚** `CopyMap: ; 00:A93E`——WorldMap→WRAM $7000-$73FF 1KB）。
/// 调用点=InitBank0（prg7_engine.asm:1382——**复位链件**：boot 期一次）。
pub fn copy_map(mem: &mut Manager) {
    for (i, b) in game_data::assets::world_map().iter().enumerate() {
        mem.write_wram(0x7000 + i as u16, *b);
    }
}

/// CopyAreaPointers（**asm 内容锚** `CopyAreaPointers: ; 07:C8B0`）：14B 拷
/// AreaPointers_ROM→AreaPointers_RAM（$3B-$48——7 指针结构体）。
/// ROM 侧按 `InArea&$0F` 选区 bank（五区已烘焙——区域未覆盖时 fail-loud D7）。
pub fn copy_area_pointers(mem: &mut Manager) {
    let area = mem.read_ram(crate::ram_map::IN_AREA) & 0x0F;
    let rom = AREA_POINTERS_BY_AREA
        .get(area as usize)
        .unwrap_or_else(|| crate::unported!("bank07:959A-area"));
    for (i, b) in rom.iter().enumerate() {
        mem.write_ram(0x003B + i as u16, *b);
    }
}

/// GetRoomNum（**asm 内容锚** `GetRoomNum: ; 07:E720`）：ScrollDir 横/纵分流+
/// ScrollBlock 门（横滚门挡：方向位 1=左/2=右 与 ScrollBlockOnNameTable3,y（$6C/$6D）
/// 按位与——挡⇒不载房）+MapPos 数学（MapY*32+MapX+$7000）→WorldMapRAM 查表
/// →RoomNumber($5A)+特别房音乐（$79——AreaItemRoomNumbers 7 项）。
/// 返回 **true=取得房间号（可滚）**（asm carry 反形：CF=1=失败）。
pub fn get_room_num(mem: &mut Manager) -> bool {
    let dir = mem.read_ram(crate::ram_map::SCROLL_DIR); // ScrollDir
    if dir >= 2 {
        // 横滚面（asm `lsr/beq` 分流：2/3=横）：ScrollBlock 门（门挡=停滚语义）
        let a: u8 = if dir == 2 { 1 } else { 2 }; // asm `rol/adc #$FF`：1（左）/2（右）
        // OnNameTable0（EC93）：PPUCTRL_ZP($FF)&3==0⇒Y=1（名表 0）/Y=0（名表 3）
        let y = if mem.read_ram(0x00FF) & 0x03 == 0 { 1u16 } else { 0 };
        if mem.read_ram(0x006C + y) & a != 0 {
            return false; // 门挡（asm `sec/bne RTS_E76F`——不滚不载房）
        }
    }
    // 纵面（LE733）：$00=(MapY*32)+MapX+$7000（WorldMapRAM 索引）→`lda ($00),y`→
    // $FF=未用（RTS carry 置）→`sta RoomNumber($5A)`。
    let map_y = mem.read_ram(0x004F) as u16;
    let map_x = mem.read_ram(0x0050) as u16;
    let idx = (map_y << 5).wrapping_add(map_x); // MapY*32+MapX（asm rol×2 域形）
    let room = mem.read_wram(0x7000 + idx); // WorldMapRAM=$7000（全址形）
    if room == 0xFF {
        return false; // 未用房间（CF=1——不能滚/不载房）
    }
    mem.set_room_number(room); // RoomNumber
    true
}

/// 【多帧载房任务机（A-lite）】开窗：把 `RoomNumber` 的「载房请求」升格为跨帧任务。
///
/// asm 依据（`prg7_engine.asm:8693 SetupRoom ; 07:EA2B` + `:8672 AttribTableWrite ; 07:EA13`
/// + `:8876 EndOfRoom ; 07:EAF4`）：`DrawRoom` 的装载循环与随后的属性表写点都在**同一次
/// `SetupRoom` 调用**里，而这次调用跨过 NMI（Mesen 逐帧实测 room0/23/8 = 冻结 1/2/3 帧，
/// `RoomNumber` 在整个窗里保持**要载的房号**，`$33/$34 RoomPtr` 每帧前进一截）。
/// 窗结束后进入固定三段尾段：帧边界 `F1 → F3 → FF`（`F0` 那格是空转 `ExitSub`）。
///
/// 本函数只登记任务相位；真正的装载仍由 [`setup_room`] 一次跑完（`RoomPtr` 分块是已登记
/// 缺口，见 findings 同节「未修清单」）。
pub fn begin_room_load_window(mem: &mut Manager, room: u8) {
    mem.room_load_phase = 1;
    mem.room_load_room = room;
    mem.room_load_tail = 0;
}

/// 「这一次 `check_update_name_table` 是不是一次**新的**载房请求」。
///
/// 判据（对抗复核 P1）：房号 `< $F0` **且**（任务机空闲 **或** 房号 ≠ 在装房号）。
/// 于是窗内（相位 1、房号保持不变）不算新请求；尾段（相位 2，游标 `$F0..$FE`）自然不满足
/// `< $F0`；尾段期间真来的新请求（门传送/电梯/`GetRoomNum` 重写 `$5A`）会**抢占**旧任务。
pub fn is_new_load_request(mem: &Manager, rn: u8) -> bool {
    rn != 0xFF && rn < 0xF0 && (mem.room_load_phase == 0 || rn != mem.room_load_room)
}

/// 任务机一次「帧门排空」步进（`Engine::end_of_tick_gate` 的 `iter_gates.is_empty()` 分支调用）。
///
/// 返回 true 表示本拍推进了任务机（调用方无需其它处理）。
pub fn room_load_gate_step(mem: &mut Manager) -> bool {
    match mem.room_load_phase {
        1 => {
            // 冻结窗结束：`DrawRoom` 尾巴的 `EndOfRoom ; 07:EAF4` 把 RoomNumber 置 $F0
            // （`draw_room` 已经做过，但窗内被我们改回房号）
            // ⇒ 窗末那一帧的边界值是 $F0 的**第一格进度**，即 F1（`F0` 走空转 ExitSub）。
            mem.room_load_phase = 2;
            mem.room_load_tail = 0;
            mem.set_room_number(0xF0);
            advance_room_load_tail(mem);
            true
        }
        2 => {
            advance_room_load_tail(mem);
            // 【载房完成帧的「剩余滚动调用」补偿（2026-11-19 尾游标修复）】
            // asm：装载工作在 `jsr SetupRoom` 的**那一次滚动调用**内完成并置 `$F0`
            // （`DrawRoom → EndOfRoom ; 07:EAF4`），该帧**剩余的滚动调用**继续推尾游标
            // （`CheckUpdateNameTable ; 07:E54A` 首件 `jsr SetupRoom` → `>= $F0` 走
            // `AttribTableWrite ; 07:EA13` 一步/调用，且在 `RoomNumber+1==0` 守卫**之前**）。
            // Rust：冻结窗按整拍吞迭代 ⇒ 完成帧的剩余调用一个都不发生，尾游标相对
            // `ScrollX` 永久晚一格。后果：门入场滚动（`$0056 != 0`，`ScrollX` 每帧 +2）在
            // `ScrollX=8`（`idx=(ScrollX&$F8)>>3=1` 的**唯一** kick 窗）那次调用里守卫仍见
            // `$F3` ⇒ 名表第 1 列永不写入 ⇒ Brinstar 47 房扫里 35 房（B 类）的属性/tile 缺列。
            // 故在尾段第一拍补推一格（等价于那次被吞掉的调用）。
            // 判据=门入场滚动在跑：`case-morphball-probe` 载房完成帧 `$0056=0`、其后 17 帧
            // `ScrollX` 恒 1（emu 停在 `$F0`，无任何滚动调用）⇒ 不补，基线逐格不变。
            if mem.room_load_tail == 2 && mem.read_ram(crate::ram_map::DOOR_ENTRY_STATUS) != 0 {
                advance_room_load_tail(mem);
            }
            if mem.room_load_tail >= 3 {
                mem.room_load_phase = 0;
                mem.room_load_room = 0xFF;
            }
            true
        }
        _ => false,
    }
}

/// 尾段一格（帧边界目标：第 1 格 → `F1`、第 2 格 → `F3`、第 3 格 → `FF`）。
///
/// asm：`AttribTableWrite` 先 `inc RoomNumber` 再按 `and #$0F` 查 5 项表
/// （`ExitSub / WritePPUAttribTbl / ExitSub / WritePPUAttribTbl / RoomFinished`）
/// ⇒ 一「格」= 一串连续调用，直到游标抵达该格的目标值。
fn advance_room_load_tail(mem: &mut Manager) {
    let target: u8 = match mem.room_load_tail {
        0 => 0xF1,
        1 => 0xF3,
        _ => 0xFF,
    };
    // 每格最多 2 次调用（F0→F1；F1→F3；F3→FF），用 3 做硬上界。
    //
    // 【fail-loud（2026-11-19 对抗复核 P2）】上界用尽仍未达目标 = 游标不在 F0..F4 格点上
    // （外来改写 `RoomNumber`、`room_ptr` 未设等）⇒ 按仓库 D7 纪律**当场炸在 bank:addr**，
    // 不许 `debug_assert` 在 release 里被编译掉、把游标走成一个看似合法的房号（那样下一帧
    // `is_load` 会真的再整房重载，症状离根因极远）。
    let mut calls = 0;
    while mem.room_number() != target {
        if calls == 3 {
            crate::unported!("bank07:EA13-tail-cursor");
        }
        crate::world::room_draw::attrib_table_write(mem);
        calls += 1;
    }
    mem.room_load_tail = mem.room_load_tail.saturating_add(1);
}

/// SetupRoom 壳（**asm 内容锚** `SetupRoom: ; 07:EA2B`）：RoomNumber 分支（$FF=RTS/
/// $FE=占位跳/≥$F0=AttribTableWrite/其余=载房）→RoomPtrTable 查表（**AreaPointers_RAM
/// .RoomPtrTable=$3B/$3C 间接**——`lda (AreaPointers_RAM.RoomPtrTable),y`）→RoomPtr($33/$34)
/// →RoomPalette 读→AddToRoomPtr/SelectRoomRAM/InitTables→DrawRoom（**S6-7 房间绘制链**
/// ——嵌套桩：game-data 房间模型协同）。
pub fn setup_room(mem: &mut Manager) {
    mem.work_cycles = mem.work_cycles.wrapping_add(crate::frame::cycles::LOAD_SHELL); // 固定壳
    // 【2026-11-14 顺序更正】asm `SetupRoom ; 07:EA2B` 的次序是：
    // `lda RoomNumber / cmp #$FF beq RTS / cmp #$FE beq DrawRoom / cmp #$F0 bcs AttribTableWrite`
    // → `jsr DeleteOffscreenRoomSprites` → `jsr ScanForItems`。
    // 旧形把 ScanForItems 提到最前——未定义房（$FF）也会先吃一次特项扫描（错序）。
    let room = mem.room_number();
    if room == 0xFF {
        return; // 未定义房（RTS）
    }
    if room == 0xFE {
        crate::unported!("bank07:EA5D"); // 占位字节路
    }
    if room >= 0xF0 {
        // AttribTableWrite（$EA13——r25 端口：$F0-$F4 命令链）
        crate::world::room_draw::attrib_table_write(mem);
        return;
    }
    // DeleteOffscreenRoomSprites（$EC9B）——载房前清「正在进入视野的名表」里的陈旧对象。
    crate::world::room_draw::delete_offscreen_room_sprites(mem);
    // ScanForItems（$ED98）——当前图格特项扫描 → PowerUp 装载。
    crate::pickup::spawn::scan_for_items(mem);
    // RoomPtrTable 查表（**asm `lda (AreaPointers_RAM.RoomPtrTable),y` 经 $3B/$3C 间接**——
    // 基址拷自当前区 bank $959A，表按区分发：0=Brinstar $A314 47 项 / 1=Norfair $A21B 46 项；
    // 折叠等价：直查该区烘焙表（表值与 ROM 逐字节同——各件头注自证），asm 形=间接查 ROM）。
    let area = mem.read_ram(crate::ram_map::IN_AREA) & 0x0F;
    let ptr = game_data::assets::room_ptr(area, room as usize)
        .unwrap_or_else(|| crate::unported!("bank07:A314-RoomPtrTable-oob"));
    mem.write_ram(0x0033, ptr as u8); // RoomPtr lo（constants_ram:508）
    mem.write_ram(0x0034, (ptr >> 8) as u8); // RoomPtr hi
    // 首字节读（房间数据——RoomPalette=$36 址随证）+SelectRoomRAM($EA05)/InitTables($EFF8)
    // +DrawRoom 链（S6-7 绘制面——r25 端口）。
    crate::world::room_draw::select_room_ram(mem);
    let room_stream = game_data::assets::room_streams(area)
        .and_then(|ss| ss.get(room as usize));
    let palette = room_stream.and_then(|s| s.first().copied()).unwrap_or(0);
    mem.write_ram(0x0068, palette); // RoomPalette（**$68**——r25 址勘误修（旧 $36 随证形误））
    crate::world::room_draw::init_tables(mem);
    if !crate::world::room_draw::draw_room(mem, room) {
        crate::unported!("bank07:EAD4"); // 敌门环未竟件（type 2/4/6/7 族）
    }
}


/// `MoreInit` 的名表门挡双清（`stx ScrollBlockOnNameTable3` / `stx ScrollBlockOnNameTable0`，
/// X=0——`refs/m1disasm/SRC/prg7_engine.asm:1876-1877`）。
///
/// 单独成函数是为了**可测**：`more_init` 整体会进 SetupRoom 环（含 fail-loud 未端口面），
/// 单测只钉这一段 asm 的写面。
fn clear_scroll_blocks(mem: &mut Manager) {
    mem.set_scroll_block_nt3(0);
    mem.set_scroll_block_nt0(0);
}

#[cfg(test)]
mod tests_room_load_task {
    use crate::frame::Manager;

    /// 【任务机 TDD 1】窗内 `RoomNumber` 必须是**要载的房号**，不是 `$F0`。
    ///
    /// 修前（2026-11-19 之前）：`draw_room` 尾巴 `EndOfRoom ; 07:EAF4` 当即把 RoomNumber
    /// 置 `$F0`，于是 Rust 帧边界看到 `F0`，而 Mesen 三房实录窗内恒为房号（room8 f900–f903
    /// 恒 `8`）⇒ 此断言在旧形必红。
    #[test]
    fn window_holds_requested_room_number() {
        let mut m = Manager::default();
        m.set_room_number(8);
        super::begin_room_load_window(&mut m, 8);
        m.set_room_number(8); // scroll 侧的「改回房号」动作（与真实调用同序）
        assert_eq!(m.room_number(), 8, "窗内保持房号");
        assert_eq!(m.room_load_phase, 1);
        assert_eq!(m.room_load_room, 8);
    }

    /// 【任务机 TDD 2】尾段三格的帧边界依次是 `F1 → F3 → FF`（Mesen room0/23/8 三房实录）。
    #[test]
    fn tail_walks_f1_f3_ff() {
        let mut m = Manager::default();
        m.room_load_phase = 1;
        m.room_load_room = 8;
        m.room().set_room_ptr(0x6000); // 真实流里由 setup_room 的 SelectRoomRAM 写入
        m.set_room_number(0xF0); // 窗末：EndOfRoom 的 $F0 起点

        assert!(super::room_load_gate_step(&mut m)); // 窗结束那一拍 = 尾段第 1 格
        assert_eq!(m.room_number(), 0xF1, "第 1 格边界 F1");

        super::room_load_gate_step(&mut m);
        assert_eq!(m.room_number(), 0xF3, "第 2 格边界 F3（同拍推进两格）");

        super::room_load_gate_step(&mut m);
        assert_eq!(m.room_number(), 0xFF, "第 3 格边界 FF（RoomFinished）");
        assert_eq!(m.room_load_phase, 0, "任务机归位");
    }

    /// 【P1 修正 TDD】尾段期间真来的新载房请求必须被识别为「新请求」（抢占旧任务），
    /// 而窗内同一房号不算新请求。旧形（`phase == 0` 单条件）在尾段会把新请求判成「非请求」
    /// ⇒ `setup_room` 装载了新房却丢掉掉帧窗与 `work_cycles` 清零（复核 P1 的反例）。
    #[test]
    fn new_request_predicate_handles_window_and_tail() {
        let mut m = Manager::default();
        // 空闲：任何房号请求都算新请求
        assert!(super::is_new_load_request(&m, 8));
        // 窗内（相位 1，在装 8）：同一房号不算、不同房号算
        super::begin_room_load_window(&mut m, 8);
        assert!(!super::is_new_load_request(&m, 8), "窗内同房号不是新请求");
        assert!(super::is_new_load_request(&m, 9), "窗内换房号=新请求（抢占）");
        // 尾段（相位 2）：游标 >= $F0 不算请求
        m.room_load_phase = 2;
        assert!(!super::is_new_load_request(&m, 0xF1));
        assert!(!super::is_new_load_request(&m, 0xFF));
        // 尾段期间真来新房号 → 新请求
        assert!(super::is_new_load_request(&m, 9));
    }

    /// 【bundle 起跑回归（第二轮复核 P1）】上电/标题 bundle 的 `$005A=$00`+`$33/$34=$0000`
    /// 曾被启发式误判成「正在装 0 号房」⇒ 开局幻影尾段（`$5A` 被推成 `F0→F1→F3→FF`）。
    /// 这条钉住：恢复后任务机必须是**空闲**，且门步不得动 `$5A`。
    #[test]
    fn power_on_restore_leaves_task_idle_and_gate_step_is_noop() {
        let mut ram = vec![0u8; 0x800]; // 全 0 = 上电初值（$5A=0、$33/$34=0）
        let m = Manager::restore_mss(&ram, &vec![0u8; 0x2000], crate::frame::memory::Registers::default());
        assert_eq!(m.room_load_phase, 0, "上电初值不得被判成窗内");
        ram[0x5A] = 0; // 显式重申
        let mut m = Manager::restore_mss(&ram, &vec![0u8; 0x2000], crate::frame::memory::Registers::default());
        assert!(!super::room_load_gate_step(&mut m), "空闲态门步不推进");
        assert_eq!(m.room_number(), 0, "门步不得改写 $5A（幻影尾段的症状）");
    }

    /// 【P2 修正 TDD】`restore_mss` 必须从 `$005A` 重建窗相位：窗内 RAM 里是**房号**，
    /// 恢复后若相位仍为 0，`is_load` 会立刻为真 ⇒ 同一房间被装载第二遍（重复掉帧窗 +
    /// 敌人/道具重新扫描的二次副作用）。
    #[test]
    fn restore_mss_rebuilds_window_phase_from_room_number() {
        let ram = vec![0u8; 0x800];
        let mut ram_window = ram.clone();
        ram_window[crate::ram_map::ROOM_NUMBER as usize & 0x7FF] = 8;
        ram_window[crate::world::room_ram::ROOM_PTR as usize & 0x7FF] = 0x00; // RoomRAMPtr 低
        ram_window[(crate::world::room_ram::ROOM_PTR + 1) as usize & 0x7FF] = 0x60; // ⇒ $6000（WRAM 域）
        let m = Manager::restore_mss(&ram_window, &vec![0u8; 0x2000], crate::frame::memory::Registers::default());
        assert_eq!(m.room_load_phase, 1, "窗内快照 ⇒ 恢复为窗相位");
        assert_eq!(m.room_load_room, 8);
        // 【上电初值边界（复核 P1）】`$5A=$00` 且 `$33/$34=$00`（标题屏/密码屏 bundle 的实值）
        // 不是「正在装 0 号房」——必须重建为**空闲**，否则开局幻影尾段。
        let m3 = Manager::restore_mss(&ram, &vec![0u8; 0x2000], crate::frame::memory::Registers::default());
        assert_eq!(m3.room_load_phase, 0, "$5A=0 且 room_ptr=0（上电初值）按空闲重建");
        let mut ram_idle = ram.clone();
        ram_idle[crate::ram_map::ROOM_NUMBER as usize & 0x7FF] = 0xFF; // 正常游玩态=无请求
        let m2 = Manager::restore_mss(&ram_idle, &vec![0u8; 0x2000], crate::frame::memory::Registers::default());
        assert_eq!(m2.room_load_phase, 0, "$5A=$FF（无请求）按空闲重建");
    }

    /// 【尾游标补偿 TDD（2026-11-19）】门入场滚动在跑时，尾段第一拍必须**多推一格**
    /// （=补回被冻结窗吞掉的那次滚动调用）。
    ///
    /// 判据来源：`CheckUpdateNameTable ; 07:E54A` 首件 `jsr SetupRoom` → `>=$F0` 走
    /// `AttribTableWrite ; 07:EA13`，一步/调用且**在 `RoomNumber+1==0` 守卫之前**；装载在
    /// `EndOfRoom ; 07:EAF4` 置 `$F0` 后，同帧剩余调用继续推进。Rust 冻结窗整拍吞迭代 ⇒
    /// 少一次调用。副作用可观测：`ScrollX=8`（`idx=1` 唯一 kick 窗）错过 ⇒ 名表第 1 列缺。
    #[test]
    fn door_scroll_compensates_one_tail_step() {
        // 门入场滚动在跑（$0056=01）：尾段第一拍多推一格 ⇒ F1 → FF（跳两格）。
        let mut m = Manager::default();
        m.room_load_phase = 1;
        m.room_load_room = 0;
        m.room().set_room_ptr(0x6000);
        m.set_room_number(0xF0);
        m.write_ram(crate::ram_map::DOOR_ENTRY_STATUS, 1);
        super::room_load_gate_step(&mut m); // 窗末：F0 → F1（原有第一格）
        assert_eq!(m.room_number(), 0xF1, "窗末第一格 F1");
        super::room_load_gate_step(&mut m); // 尾段第一拍：F1 → F3（原有）→ 补偿 → FF
        assert_eq!(
            m.room_number(),
            0xFF,
            "门入场滚动在跑 ⇒ 尾段第一拍补推一格（F3 → FF）"
        );

        // 非门入场（$0056=00）：不补偿，逐格推进（基线行为不变）。
        let mut n = Manager::default();
        n.room_load_phase = 1;
        n.room_load_room = 0;
        n.room().set_room_ptr(0x6000);
        n.set_room_number(0xF0);
        n.write_ram(crate::ram_map::DOOR_ENTRY_STATUS, 0);
        super::room_load_gate_step(&mut n);
        assert_eq!(n.room_number(), 0xF1, "窗末第一格 F1");
        super::room_load_gate_step(&mut n);
        assert_eq!(n.room_number(), 0xF3, "非门入场 ⇒ 不补，仍 F3");
        super::room_load_gate_step(&mut n);
        assert_eq!(n.room_number(), 0xFF, "第三格 FF");
    }

    /// 尾段落完后任务机必须回空闲，否则下一次真实载房会被 `is_load` 的相位闸永久挡住。
    #[test]
    fn task_returns_to_idle_and_allows_next_load() {
        let mut m = Manager::default();
        m.room_load_phase = 1;
        m.room_load_room = 23;
        m.room().set_room_ptr(0x6400);
        m.set_room_number(0xF0);
        for _ in 0..3 {
            super::room_load_gate_step(&mut m);
        }
        assert_eq!((m.room_load_phase, m.room_load_tail), (0, 3));
        assert_eq!(m.room_number(), 0xFF);
        assert!(!super::room_load_gate_step(&mut m), "空闲态不推进");
    }
}

#[cfg(test)]
mod tests_room_load {
    use crate::frame::Manager;

    /// Fix1-B 回归：MoreInit 清的是 `$6C/$6D`（门挡），**不是** `$5A RoomNumber` /
    /// `$53 WalkSoundDelay`——旧形写错地址 ⇒ 区域切换后门挡位残留 ⇒ `get_room_num`
    /// 读 `$6C+y` 命中门挡位即拒滚（`room_load.rs:189`）。
    #[test]
    fn more_init_clears_door_scroll_blocks() {
        let mut m = Manager::default();
        // 模拟区域切换前的残留门挡位（bit0/bit1）。
        m.set_scroll_block_nt3(0x02);
        m.set_scroll_block_nt0(0x01);
        m.set_room_number(0x5A); // 旧形误写目标：值必须保持不变
        m.write_ram(0x0053, 0x07); // 旧形误写目标：步音延时必须保持不变
        super::clear_scroll_blocks(&mut m);
        assert_eq!((m.scroll_block_nt3(), m.scroll_block_nt0()), (0, 0));
        assert_eq!(m.read_ram(crate::ram_map::SCROLL_BLOCK_NT3), 0);
        assert_eq!(m.read_ram(crate::ram_map::SCROLL_BLOCK_NT0), 0);
        assert_eq!(m.room_number(), 0x5A, "MoreInit 不清 RoomNumber（后段才写 $FF）");
        assert_eq!(m.read_ram(0x0053), 0x07, "MoreInit 不碰 WalkSoundDelay=$53");
    }

    /// Fix2-B：**打穿 `more_init` 本体**（不是只测 helper）——残留门挡位必须在
    /// 区域初始化里被清掉，而 `$53 WalkSoundDelay` 保持原值。
    ///
    /// asm 锚：`MoreInit ; 07:C81D`（`prg7_engine.asm:1876-1877`）
    /// `stx ScrollBlockOnNameTable3 / stx ScrollBlockOnNameTable0`（X=0）。
    #[test]
    fn more_init_clears_stale_scroll_blocks_through_entry() {
        let mut m = Manager::default();
        crate::frame::boot::copy_map(&mut m); // WorldMap（boot 链件），与 frame::tests 同前置
        m.write_ram(crate::ram_map::MAIN_ROUTINE, 1);
        m.write_ram(crate::ram_map::IN_AREA, 0);
        m.set_scroll_block_nt3(0x02); // 上次区域的残留门挡位（bit1=右门）
        m.set_scroll_block_nt0(0x01); // bit0=左门
        m.write_ram(0x0053, 0x07); // WalkSoundDelay：MoreInit 不得触碰
        super::more_init(&mut m);
        assert_eq!(
            (m.scroll_block_nt3(), m.scroll_block_nt0()),
            (0, 0),
            "MoreInit 必须清 $6C/$6D（asm 07:C81D 的 stx 双清）"
        );
        assert_eq!(
            m.read_ram(crate::ram_map::SCROLL_BLOCK_NT3)
                | m.read_ram(crate::ram_map::SCROLL_BLOCK_NT0),
            0,
            "typed 域与 RAM 面一致（$6C/$6D 走 typed，不再落 storage）"
        );
        assert_eq!(m.read_ram(0x0053), 0x07, "MoreInit 不清 WalkSoundDelay=$53");
    }
}
