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
    mem.set_room_number(0); // ScrollBlockOnNameTable3
    mem.write_ram(0x0053, 0); // ScrollBlockOnNameTable0
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

