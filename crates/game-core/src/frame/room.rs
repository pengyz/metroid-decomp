//! 房间绘制引擎（DrawRoom $EAAA 族——r25 端口；对拍驱动）。
//!
//! asm 锚（prg7_engine.asm）：DrawRoom $EAAA（对象环）/DrawObject $EA60（3B 条目：
//! pos(%yyyyxxxx)/struct 索引/属性）/DrawStruct $EF8C（行流至 $FF）/DrawStructRow
//! $EF13（行头：低半字节=元 tile 计（0=16）/高半字节=x 偏移）/DrawMetatile $EF2B（4B
//! 组→TilePosTable 序 $21/$20/$01/$00）/UpdateAttrib $EF9E（属性位对：x=quadrant）。
//! 数据面=game-data::area::graphics（三件套烘焙）。

use game_data::area::graphics as gfx;

/// 元 tile 4 tile 落位偏移（**r25 勘误修**：asm TilePosTable x=3→0 取序=data[0]→$00
/// （左上）/data[1]→$01/data[2]→$20/data[3]→$21——旧表反序致四 tile 镜像错位）。
const TILE_POS: [u16; 4] = [0x00, 0x01, 0x20, 0x21];

/// 属性位掩码（公式形：清 x 位对——asm AttribMaskTable 语义）。
fn attrib_mask(x: u8) -> u8 {
    !(0x03u8 << (x * 2))
}

/// DrawRoom（$EAAA）——房对象环→RoomRAM 缓冲（base=RoomRAMA $6000）。
/// 返回 false=走敌门环/属性表尾（未端口面——调用方 fail-loud）。
pub fn draw_room(mem: &mut crate::frame::FrameMemory, room: u8) -> bool {
    let Some(stream) = gfx::ROOM_STREAMS.get(room as usize) else {
        return false;
    };
    // 【V2-lite】SelectRoomRAM（$EA05）：缓冲按名表位（EB85：PPUCTRL^ScrollDir）&1
    // ——名表 0→RoomRAMA($6000)/名表 3→RoomRAMB($6400)（双屏邻房径）。旧形硬编码 $6000
    // 致邻房装载落错缓冲（碰撞读 $6400 空=tile 00 固实——移动冻结根）。
    let base = if (mem.read_ram(crate::ram_map::PPUCTRL_ZP) ^ mem.read_ram(crate::ram_map::SCROLL_DIR)) & 1 != 0 {
        0x6400u16
    } else {
        0x6000u16
    };
    mem.write_ram(0x0037, (base & 0xFF) as u8); // RoomRAMPtr
    mem.write_ram(0x0038, (base >> 8) as u8);
    let room_palette = stream.first().copied().unwrap_or(0); // RoomPalette（首字节——SetupRoom 读）
    let mut i = 1usize; // 对象环起点=首字节后（RoomPalette 已消费——asm 形）
    loop {
        let Some(&b) = stream.get(i) else { return false };
        match b {
            0xFF => {
                // EndOfRoom（$EAF4）：RoomNumber←$F0（属性表尾命令——循环续面）
                mem.write_ram(0x005A, 0xF0);
                return true;
            }
            0xFE => {
                mem.write_ram(0x005A, 0xFE); // 空对象占位（LEABC）
                i += 1;
            }
            0xFD => {
                // EndOfObjs→敌门环（$EAD4——3B 条目：slot|type/敌型/位；type=低半字节）
                return enemy_loop(mem, stream, i + 1, room_palette);
            }
            _ => {
                // DrawObject：pos/struct 索引/属性 3B
                let pos = b;
                let struct_idx = stream.get(i + 1).copied().unwrap_or(0);
                let obj_palette = stream.get(i + 2).copied().unwrap_or(0);
                let y = (pos >> 4) as u16;
                let x = (pos & 0x0F) as u16;
                // RoomRAMWorkPtr = base + y*0x40 + x*2（y 单元=2 行/x 单元=2 tile）
                let work = base + y * 0x40 + x * 2;
                draw_struct(mem, work, struct_idx, obj_palette, room_palette);
                i += 3;
            }
        }
    }
}

/// DrawStruct（$EF8C）——结构行流（行头+元 tile 流至 $FF）。
fn draw_struct(
    mem: &mut crate::frame::FrameMemory,
    work: u16,
    struct_idx: u8,
    obj_palette: u8,
    room_palette: u8,
) {
    let Some(&s) = gfx::STRUCT_PTRS.get(struct_idx as usize) else { return };
    let _ = s;
    let Some(data) = gfx::STRUCTURES.get(struct_idx as usize) else { return };
    let mut idx = 0usize;
    let mut work = work;
    loop {
        let Some(&head) = data.get(idx) else { return };
        if head == 0xFF {
            return; // DrawStruct @RTS
        }
        idx += 1;
        // DrawStructRow：行头（低半字节=元 tile 计（0=16）/高半字节=x 偏移）
        let count = match head & 0x0F {
            0 => 16u8,
            n => n,
        };
        let x_off = ((head >> 4) as u16) * 2;
        let mut row_ptr = work + x_off;
        let mut counter = count;
        loop {
            // DrawMetatile
            let Some(&mt) = data.get(idx) else { return };
            idx += 1;
            let mt_base = (mt as usize) * 4;
            for k in 0..4u16 {
                let tile = gfx::METATILE_DEFS
                    .get(mt_base + k as usize)
                    .copied()
                    .unwrap_or(0);
                let at = row_ptr.wrapping_add(TILE_POS[k as usize]);
                mem.write_wram(at, tile);
            }
            update_attrib(mem, row_ptr, obj_palette, room_palette);
            row_ptr = row_ptr.wrapping_add(2);
            // 行满（&$1F==0）⇒提前换行（AdvanceRow）；否则计数递减
            if row_ptr & 0x1F == 0 {
                // struct index 前跳剩余（asm：struct index+counter-1→AdvanceRow）
                idx += counter as usize - 1;
                break;
            }
            counter -= 1;
            if counter == 0 {
                break;
            }
        }
        // AdvanceRow（$EF78）：work+=$40（下一行）
        work = work.wrapping_add(0x40);
    }
}


/// 门对象槽系统·任务卡 1——LoadDoor/SpawnDoorRoutine（$EB8C/$EB92
/// （findings/door-system-audit.md 任务卡 1 实录（A 级全文）））：
/// 棋盘槽选（y=((MapPosX+MapPosY)&1)<<1|side（side=info bit4（Amul16 CF=bit4））；
/// DoorSlots=[0x00,0x30,0x20,0x10]（$EC00））→DoorType[$387+x]←info&3；
/// 1|3（蓝/音乐蓝）⇒status[$380+x]←1；0|2（红/导弹）⇒$09←$0A/Y=MapPosX（左槽⇒-1）
/// →LEE41（$07←ItemX/$06←MapPosY）→CheckForItem（$EE4A（Y=$6886 计数/对表 $6885/$6886
/// 直索））→**已收（carry）⇒跳 init（status 保持 0）**/未收⇒status←1；
/// 位域写（hi[$38C+x]←NT/x[$38E+x]←DoorXs[side]/y[$38D+x]←$68/ScrollBlockOnNameTable3
/// [1^NT]|=DoorScrollBlocks[side]）。
fn load_door(mem: &mut crate::frame::FrameMemory, info: u8) {
    use crate::ram_map::*;
    const OBJECTS_BASE: u16 = 0x0380;
    const DOOR_SLOTS: [u8; 4] = [0x00, 0x30, 0x20, 0x10]; // $EC00（白右/白左/黑右/黑左）
    const DOOR_XS: [u8; 2] = [0xF0, 0x10]; // $EBFC（右/左）
    const DOOR_SCROLL_BLOCKS: [u8; 2] = [0x02, 0x01]; // $EBFE（右/左）
    let side = (info >> 4) & 1; // Amul16 CF=bit4（audit 实录（双径互证））
    let sum = mem.read_ram(MAP_POS_X).wrapping_add(mem.read_ram(MAP_POS_Y));
    let y_idx = (((sum & 1) << 1) | side) as usize;
    let x_off = DOOR_SLOTS[y_idx] as u16;
    let door_type = info & 3;
    mem.write_ram(OBJECTS_BASE + 7 + x_off, door_type); // DoorType（data07）
    let init = match door_type {
        1 | 3 => true, // 蓝/音乐蓝（@if_B 直入）
        _ => {
            // 红/导弹径：$09←$0A + LEE41 + CheckForItem（已收⇒跳 init）
            mem.write_ram(0x0009, 0x0A);
            let map_x = mem.read_ram(MAP_POS_X);
            let item_x = if x_off >= 0x10 { map_x.wrapping_sub(1) } else { map_x };
            let item_y = mem.read_ram(MAP_POS_Y); // LEE41（$06←MapPosY）
            let y0 = mem.read_wram(0x6886); // NumberOfUniqueItems（计数（含红门））
            let mut lo = [0u8; 0x100];
            let mut hi = [0u8; 0x100];
            for k in 0..0x100usize {
                lo[k] = mem.read_wram(0x6885 + k as u16);
                hi[k] = mem.read_wram(0x6886 + k as u16);
            }
            // 【A 级补钉】CheckForItem 进口 `ldy NumberOfUniqueItems / beq @notHave`
            // （:9425）——y0=0⇒即未收（不进环）；guard 落 call site（find_item_in_history
            // 共享面不动（其环契约=y∈{y0,…,0}（旧对拍对象等价域）））。
            let found = if y0 == 0 {
                false
            } else {
                crate::engine::find_item_in_history(y0, item_y, item_x, &lo, &hi)
            };
            !found // carry=已收⇒跳（status 保持 0）/未收⇒init
        }
    };
    if init {
        mem.write_ram(OBJECTS_BASE + x_off, 1); // status←1（Init 态）
    }
    let nt = crate::engine::get_name_table_at_scroll_dir(
        mem.read_ram(PPUCTRL_ZP), mem.read_ram(SCROLL_DIR));
    mem.write_ram(OBJECTS_BASE + 0x0C + x_off, nt); // hi
    mem.write_ram(OBJECTS_BASE + 0x0E + x_off, DOOR_XS[side as usize]); // x
    mem.write_ram(OBJECTS_BASE + 0x0D + x_off, 0x68); // y（恒 $68）
    let block = DOOR_SCROLL_BLOCKS[side as usize];
    let other = (nt ^ 1) as u16;
    let cur = mem.read_ram(SCROLL_BLOCK_NT3 + other);
    mem.write_ram(SCROLL_BLOCK_NT3 + other, cur | block);
}

/// EnemyLoop（$EAD4）——敌/门条目环（3B：slot&$F0|type/敌型/位%yyyyxxxx；$FF=EndOfRoom）。
/// type：0/3/5=ExitSub（no-op——A=type（asm 形））；1=LoadEnemy；2/4/6/7=门/梯/像/虫（未端口）。
fn enemy_loop(
    mem: &mut crate::frame::FrameMemory,
    stream: &[u8],
    start: usize,
    room_palette: u8,
) -> bool {
    let mut i = start;
    let mut guard = 0usize;
    loop {
        guard += 1;
        if guard > 256 {
            return false; // 环护栏（asm 形数据不环——防死环）
        }
        let Some(&b) = stream.get(i) else { return false };
        if b == 0xFF {
            end_of_room(mem);
            return true;
        }
        let ty = b & 0x0F;
        match ty {
            0 | 3 | 5 => i += ty as usize, // ExitSub（asm：A=type——推进形）
            1 => {
                if !load_enemy(mem, b, stream.get(i + 1).copied().unwrap_or(0),
                               stream.get(i + 2).copied().unwrap_or(0)) {
                    return false;
                }
                i += 3;
            }
            // 【占位臂记账】LoadElevator($EC04)/LoadStatues($EC2F)/LoadPipeBugHole($EC57)
            // ——梯/像/虫装载深件（随件）。既有占位臂模式（DIVERGENCES 记账非 panic）。
            // 【任务卡 1】type 2=LoadDoor（$EB8C→SpawnDoorRoutine（A 级（audit 实录）））
            // ——门对象槽 init 已端口（stride 2（`lda #$02/rts`（:8853）））。
            // 【stride 校】门/梯=2（EBE5/E962 `lda #$02/rts`）；像/虫=3（待考（下批 A 级核））。
            2 => {
                load_door(mem, stream.get(i + 1).copied().unwrap_or(0));
                i += 2;
            }
            4 | 6 | 7 => {
                let addr = match ty {
                    4 => 0xEC04, // LoadElevator
                    6 => 0xEC2F, // LoadStatues
                    _ => 0xEC57, // LoadPipeBugHole
                };
                crate::frame::DIVERGENCES.with(|d| d.borrow_mut().push(addr));
                i += if ty == 4 { 2 } else { 3 }; // 梯=2（A 级）/像虫=3（待考）
            }
            _ => return false,
        }
    }
}

/// EndOfRoom（$EAF4）——RoomNumber←$F0（属性表尾命令——SetupRoom 环续面）。
fn end_of_room(mem: &mut crate::frame::FrameMemory) {
    mem.write_ram(0x005A, 0xF0);
}

/// LoadEnemy（$EB06——GetEnemyData/GetEnemyType/LEB4D/LEB6E 全径）。
/// 结构址：Ens=$0400+slot（16B 步）/EnsExtra=$6AF4+slot（En/EnExtra 布局——constants:85/125）。
fn load_enemy(mem: &mut crate::frame::FrameMemory, b0: u8, etype: u8, pos: u8) -> bool {
    let x = (b0 & 0xF0) as u16; // 对象槽（×16 步）
    let en = 0x0400u16 + x; // Ens=2KB RAM 面（write_ram）
    let ex = 0x6AF4u16 + x; // EnsExtra=WRAM 面（write_wram）
    // IsSlotTaken（$EB7A）：status≠0 且 data05 bit1=0 ⇒ 槽已占用（跳过条目）
    if mem.read_wram(ex) != 0 && mem.read_ram(en + 5) & 0x02 == 0 {
        return true;
    }
    // GetEnemyType（$EB28）：specialAttribs←etype&$C0；type←etype&$3F（Boss 检=bit6——
    // Zoomer 型 bit6=0 不入（asm asl/bpl 形））
    mem.write_ram(en + 0x0F, etype & 0xC0); // specialAttribs(+0xF)（ROM 真值——旧 +0x11 越界）
    mem.write_wram(ex + 0x0E, etype & 0x3F); // EnsExtra.type
    // LEB4D（$EB4D）：y←(pos&$F0)|8；x←(pos<<4)|$0C；status←Resting(1)；isHit←0；hi←名表
    let ty = mem.read_wram(ex + 0x0E);
    mem.write_ram(en, (pos & 0xF0) | 0x08); // Ens.y
    mem.write_ram(en + 1, pos.wrapping_shl(4) | 0x0C); // Ens.x
    mem.write_wram(ex, 0x01); // EnsExtra.status←enemyStatus_Resting（constants.asm:121）
    mem.write_ram(en + 4, 0x00); // Ens.isHit
    // GetNameTableAtScrollDir（$EB85）：(PPUCTRL_ZP^ScrollDir)&1
    let hi = mem.read_ram(crate::ram_map::PPUCTRL_ZP) ^ mem.read_ram(crate::ram_map::SCROLL_DIR);
    mem.write_wram(ex + 0x07, hi & 0x01); // EnsExtra.hi
    // LEB6E：data05←shl（facing 位旋入 bit7）；InitEnemyFacing（$FB7B）：ror（L977B 位7）
    // +delay←InitDelayTbl；InitForce（$F85A）：data0D←ForceTbl；health←HealthTbl（tough×2）
    let t = ty as usize & 0x3F;
    let d5 = mem.read_ram(en + 5);
    let rotated = d5 << 1; // asl Ens.data05
    let facing = (game_data::area::graphics::ENEMY_FACING[t] & 0x80) != 0;
    let d5 = if facing { rotated | 0x80 } else { rotated & 0x7F }; // ror 形（carry→bit7）
    mem.write_ram(en + 5, d5);
    // 【goal-r16 复核修正 G1'】字段偏移 +2 错位修正（ROM 真值：FB7B `sta $0409`=delay+9，
    // F85A `sta $040D`=data0D+0xD / `sta $040B`=health+0xB）。旧形 delay→+0x0B（写进 health
    // 槽）⇒delay(+9) 恒 0⇒F3BE beq 跳过 TryBecomingActive⇒敌人恒 Resting 不激活。
    mem.write_ram(en + 0x09, game_data::area::graphics::ENEMY_INIT_DELAY[t]); // Ens.delay(+9)
    mem.write_ram(en + 0x0D, game_data::area::graphics::ENEMY_FORCE_SPEED_DELAY[t]); // data0D(+0xD)
    let mut hp = game_data::area::graphics::ENEMY_HEALTH[t];
    if mem.read_ram(en + 0x0F) & 0x80 != 0 {
        hp = hp.wrapping_shl(1); // tough 版双血（specialAttribs bit7——asm asl 形）
    }
    mem.write_ram(en + 0x0B, hp); // health(+0xB)
    true
}

/// SelectRoomRAM（$EA05）：名表 0→$6000/名表 3→$6400（RoomRAMPtr 高字节）。
pub fn select_room_ram(mem: &mut crate::frame::FrameMemory) {
    let hi = 0x60u8 | ((mem.read_ram(crate::ram_map::PPUCTRL_ZP) ^ mem.read_ram(crate::ram_map::SCROLL_DIR)) & 1) << 2;
    mem.write_ram(0x0037, 0x00); // RoomRAMPtr lo
    mem.write_ram(0x0038, hi);
}

/// InitTables（$EFF8）：RoomRAM 全填 $FF（空 tile）+属性表（$3C0 面）填 ATDataTable[RoomPalette&3]。
pub fn init_tables(mem: &mut crate::frame::FrameMemory) {
    let hi = mem.read_ram(0x0038); // RoomRAMPtr hi（$60/$64）
    // FillRoomRAM（$F01C）：$FF 填 3 页半（tile 区 $000-$3BF）
    for i in 0..0x3C0u16 {
        mem.write_wram(((hi as u16) << 8) | i, 0xFF);
    }
    // 属性表填（ATDataTable[$F018——00/55/AA/FF]——RoomPalette($68) 低 2 位）
    let fill = [0x00u8, 0x55, 0xAA, 0xFF][mem.read_ram(0x0068) as usize & 0x03];
    for i in 0..0x40u16 {
        mem.write_wram(((hi as u16 + 3) << 8) | (0xC0 + i), fill);
    }
}

/// AttribTableWrite（$EA13）——$F0-$F4 命令链（RoomNumber 低半字节=行索引；inc 自进）：
/// 0/2=no-op、1/3=WritePPUAttribTbl（属性行→PPU）、4=RoomFinished（$FF）。
pub fn attrib_table_write(mem: &mut crate::frame::FrameMemory) -> bool {
    let room = mem.read_ram(0x005A);
    let row = (room & 0x0F) as u16;
    mem.write_ram(0x005A, room.wrapping_add(1)); // inc RoomNumber
    match row {
        1 | 3 => {
            // WritePPUAttribTbl（$E5E2）：RoomRAM 属性行→PPU 属性表（MoreInit 拷贝冗余面）
            let hi = mem.read_ram(0x0038);
            let src = ((hi as u16 + 3) << 8) | 0xC0;
            let dst = 0x23C0u16 + (row - 1) * 0x20; // 属性行面（4 行 8B 组形随证）
            for k in 0..0x08u16 {
                let b = mem.read_wram(src + (row - 1) * 0x20 + k);
                mem.write_io(0x2006, (dst >> 8) as u8);
                mem.write_io(0x2006, dst as u8 | k as u8);
                mem.write_io(0x2007, b);
            }
            true
        }
        4 => {
            mem.write_ram(0x005A, 0xFF); // RoomFinished
            true
        }
        _ => true, // 0/2=ExitSub
    }
}
fn update_attrib(
    mem: &mut crate::frame::FrameMemory,
    ptr: u16,
    obj_palette: u8,
    room_palette: u8,
) {
    if obj_palette == room_palette {
        return; // 同室调色⇒无改（asm @RTS）
    }
    // 属性字节址（asm 算式）：(ptr>>2) 低 3 位 | (ptr>>4)&$38 | $C0，高字节 $63|(ptr_hi&4)
    let rel = ptr.wrapping_sub(0x6000) & 0x0FFF; // RoomRAM 相对
    let a0 = rel >> 2;
    let off = (a0 & 0x07) | ((a0 >> 2) & 0x38) | 0xC0;
    let hi = 0x63u16 | ((ptr >> 8) & 0x04);
    let addr = (hi << 8) | off;
    // 位对（x）：bit6/2 判定（asm bit/and 形）
    let mut x = 0u8;
    if ptr & 0x40 != 0 {
        x += 2;
    }
    if ptr & 0x02 != 0 {
        x += 1;
    }
    let cur = mem.read_wram(addr);
    let cleared = cur & attrib_mask(x);
    let new = cleared | ((obj_palette & 0x03) << (x * 2));
    mem.write_wram(addr, new);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::FrameMemory;

    /// 起始房（房 9）绘制形：RoomRAM 有 tile 落+敌门环通（双 Zoomer 条目——type=1）。
    #[test]
    fn draw_start_room_writes_tiles() {
        let mut m = FrameMemory::default();
        assert!(draw_room(&mut m, 9), "房 9 全径通（对象环+敌门环+EndOfRoom）");
        let tiles: Vec<u8> = (0..0x3C0u16).map(|i| m.read_wram(0x6000 + i)).collect();
        assert!(tiles.iter().any(|&t| t != 0), "RoomRAM 有 tile 落");
        assert_eq!(m.read_ram(0x0450 + 0x0B), gfx::ENEMY_HEALTH[5], "Zoomer 血量（槽 5——条目 51 形；health=+0xB ROM F85A）");
    }

    /// 属性位公式形（掩码对——x 位对清+置）。
    #[test]
    fn attrib_mask_formula() {
        assert_eq!(attrib_mask(0), 0xFC);
        assert_eq!(attrib_mask(1), 0xF3);
        assert_eq!(attrib_mask(2), 0xCF);
        assert_eq!(attrib_mask(3), 0x3F);
    }
}

#[cfg(test)]
mod r25_attr_probe {
    use super::*;

    /// 属性表落面（房 9 绘制后 $63C0 区——地面带组选核）。
    #[test]
    fn room9_attrib_table() {
        let mut m = crate::frame::FrameMemory::default();
        select_room_ram(&mut m);
        m.write_ram(0x0068, 0x03); // RoomPalette=3（房 9 首字节）
        init_tables(&mut m);
        let _ = draw_room(&mut m, 9);
        let attrs: Vec<u8> = (0..0x40u16).map(|i| m.read_wram(0x63C0 + i)).collect();
        println!("[attr-probe] 属性表: {:02X?}", attrs);
    }
}

// ===== 任务卡 1：门对象槽系统（TDD（测试先行（实现后补）））=====

#[cfg(test)]
mod tests_load_door {
    //! 判径（findings/door-system-audit.md 任务卡 1 实录）：棋盘槽选
    //! y=((MapPosX+MapPosY)&1)<<1|side；DoorSlots=[0x00,0x30,0x20,0x10]；
    //! DoorType[$387+x]←info&3；1|3⇒status[$380+x]←1；0|2⇒CheckForItem（史空=未收⇒1
    //! （已收⇒跳））；hi/x/y 位域+$09←$0A+ScrollBlock OR。
    use super::*;
    use crate::frame::FrameMemory;
    use crate::ram_map::{MAP_POS_X, MAP_POS_Y};

    fn mem_with(map_x: u8, map_y: u8) -> FrameMemory {
        let mut m = FrameMemory::default();
        m.write_ram(MAP_POS_X, map_x);
        m.write_ram(MAP_POS_Y, map_y);
        m
    }

    /// 【穷举·棋盘槽选】MapPosX奇偶×MapPosY奇偶×side×四型——DoorType 落址/落值逐钉
    /// +status 路径（1|3⇒1；0|2 空史⇒1）。
    #[test]
    fn exhaustive_checkerboard_slots() {
        for mx in [0u8, 1] {
            for my in [0u8, 1] {
                for side in [0u8, 1] {
                    for dtype in 0u8..4 {
                        let mut m = mem_with(mx, my);
                        load_door(&mut m, (side << 4) | dtype);
                        let y_idx = ((((mx.wrapping_add(my)) & 1) << 1) | side) as usize;
                        let x_off = [0x00u16, 0x30, 0x20, 0x10][y_idx];
                        assert_eq!(m.read_ram(0x0387 + x_off), dtype,
                                   "DoorType@{:03X}（mx={} my={} side={} t={})",
                                   0x0387 + x_off, mx, my, side, dtype);
                        // 1|3⇒status←1；0|2 空史（未收）⇒status←1
                        assert_eq!(m.read_ram(0x0380 + x_off), 1,
                                   "status init（t={}）", dtype);
                    }
                }
            }
        }
    }

    /// 【穷举·已开门径】item 历史含（MapPosX±1,MapPosY）⇒status 保持 0（红/导弹门）。
    #[test]
    fn opened_door_skips_init() {
        for side in [0u8, 1] {
            for dtype in [0u8, 2] {
                let mut m = mem_with(5, 7);
                // 史写入：count=2；(hi=$6886+2, lo=$6885+2)=（ItemX,ItemY）
                m.write_wram(0x6886, 2); // count（y0=2）
                let item_x = if side == 1 { 4 } else { 5 }; // 左门=MapPosX-1
                m.write_wram(0x6886 + 2, item_x); // hi[y=2]
                m.write_wram(0x6885 + 2, 7);      // lo[y=2]=MapPosY
                load_door(&mut m, (side << 4) | dtype);
                let y_idx = ((((5u8.wrapping_add(7)) & 1) << 1) | side) as usize;
                let x_off = [0x00u16, 0x30, 0x20, 0x10][y_idx];
                assert_eq!(m.read_ram(0x0380 + x_off), 0,
                           "已开（史含）⇒status 保持 0（side={} t={}）", side, dtype);
            }
        }
    }

    /// 【边界+异常路径】位域写（hi/x/y（$F0右/$10左/$68））+ScrollBlock OR（另一名表）
    /// +$09←$0A（红/导弹径）。
    #[test]
    fn field_writes_and_scroll_block() {
        for side in [0u8, 1] {
            let mut m = mem_with(0, 0);
            load_door(&mut m, (side << 4) | 1); // 蓝门（免 item 检）
            let x_off = if side == 1 { 0x30u16 } else { 0x00 }; // 白格（mx+my=0）
            let nt = crate::engine::get_name_table_at_scroll_dir(
                m.read_ram(crate::ram_map::PPUCTRL_ZP), m.read_ram(crate::ram_map::SCROLL_DIR));
            assert_eq!(m.read_ram(0x038C + x_off), nt, "hi（名表）");
            assert_eq!(m.read_ram(0x038E + x_off), if side == 1 { 0x10 } else { 0xF0 }, "x（DoorXs）");
            assert_eq!(m.read_ram(0x038D + x_off), 0x68, "y（恒 $68）");
            let other = (nt ^ 1) as u16;
            let want = if side == 1 { 0x01 } else { 0x02 };
            assert_eq!(m.read_ram(crate::ram_map::SCROLL_BLOCK_NT3 + other) & want, want,
                       "ScrollBlock OR（另一名表）");
        }
        // 红门径 $09←$0A
        let mut m = mem_with(0, 0);
        load_door(&mut m, 0x00);
        assert_eq!(m.read_ram(0x0009), 0x0A, "item id 写（$09←$0A）");
    }
}
