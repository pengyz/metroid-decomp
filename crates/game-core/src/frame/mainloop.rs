//! 主循环族（bank07:C0BC 体；two-layer 域模块——frame/ 互斥面）。
//!
//! **asm 内容锚+结构互核注**（施工令②精化版）——
//! refs/m1disasm/SRC/prg7_engine.asm 主循环体（C0BC 族）：`jsr CheckBankSwitch`→
//! `jsr UpdateTimer`→`jsr GoMainRoutine`→`inc FrameCount`→`lda #$00/sta NMIStatus`→
//! spin（`@loop_waitNMIEnd: tay/lda NMIStatus/bne`）→`jsr RandomNumbers`→`jmp MainLoop`。
//!
//! **tick 内序**（契约 §8.1）：NMI 尾写 1（唤醒）→ RandomNumbers → C0BC 体 → 清 0 →
//! spin yield——本模块=一迭代；**spin=跨帧 yield**（引擎相位机）。
//!
//! **会师点**：GoMainRoutine 标题路=`jmp MainTitleRoutine`（**00:8000**——port2 task-23
//! title 簇头；TitleRoutinePtrTable 31 项分发表）——分发壳↔方法集对表点。

use super::memory::FrameMemory;
use crate::ram_map::{
    FRAME_COUNT, GAME_MODE, NMI_STATUS, RANDOM1, RANDOM2, TIMER1, TIMER2,
    TIMER3, TIMER_DELAY,
};

/// 主循环一迭代（C0BC 体；spin 唤醒已由本 tick NMI 完成）。
pub fn main_loop_iter(mem: &mut FrameMemory) {
    random_numbers(mem); // C000（唤醒后首件）
    super::boot::c4de_unit(mem); // CheckBankSwitch（已端口复用）
    update_timer(mem); // C266
    go_main_routine(mem); // C114（分发壳——会师点）
    mem.write_ram(FRAME_COUNT, mem.read_ram(FRAME_COUNT).wrapping_add(1)); // `inc FrameCount`
    mem.write_ram(NMI_STATUS, 0); // 清 0（spin 入口）
    // spin yield（跨帧——引擎相位机）
}

/// RandomNumbers 单元（**asm 内容锚+结构互核注**）——
/// refs/m1disasm/SRC/prg7_engine.asm `RandomNumbers: ; 07:C000`（注「RandomNumber1 is
/// increased by #$19 every frame / RandomNumber2 … by #$5F every frame」；体：
/// `ldx #$05` 循环内 `RandomNumber1+=5/RandomNumber2+=$13`×5=**每调 +$19/+$5F**）。
///
/// **结构互核注**：注释值=循环体×5 的合成值（$05×5=$19 ✓/$13×5=$5F ✓——注与体互证）；
/// A 终值=RandomNumber1。
pub fn random_numbers(mem: &mut FrameMemory) {
    let n1 = mem.read_ram(RANDOM1).wrapping_add(0x19);
    let n2 = mem.read_ram(RANDOM2).wrapping_add(0x5F);
    mem.write_ram(RANDOM1, n1);
    mem.write_ram(RANDOM2, n2);
    mem.regs.a = n1;
}


/// MainRoutinePtrTable 烘焙（07:C141——10 项；表址自证）。
const MAIN_ROUTINE_PTR_TABLE: [u16; 10] = [
    0xC801, 0xC81D, 0xC8D1, 0xC92B, 0xC9A6, // AreaInit/MoreInit/SamusInit/GameEngine/PrepareGameOver
    0xC9B1, 0xC9C4, 0xC155, 0xC9D7, 0xC494, // PauseMode/GoPassword/IncrementRoutine/SamusIntro/WaitTimer
];

/// 游戏侧主例程分发（**asm 内容锚**——T45-3 移动臂首靶；C11B+ 面）：
/// START 检查（暂停切换）→`lda MainRoutine($1E)/jsr JumpEngine`→MainRoutinePtrTable
/// 10 项分派。未武装臂=per-entry 爆（移动臂迭代分类面）。
pub fn game_main_routine(mem: &mut FrameMemory) {
    let mr = mem.read_ram(crate::ram_map::MAIN_ROUTINE);
    if (mr as usize) < 10 {
        let target = MAIN_ROUTINE_PTR_TABLE[mr as usize];
        // JumpEngine 指针写（$0C/$0D——同标题面）
        mem.write_ram(0x000C, target as u8);
        mem.write_ram(0x000D, (target >> 8) as u8);
    }
    match mr {
        0 => area_init(mem), // AreaInit（已端口——移动臂迭代驱动首件）
        1 => more_init(mem), // MoreInit（已端口——区域/房间链头）
        3 => game_engine(mem), // GameEngine（已端口——移动帧环壳）
        _ => {
            let loc = format!("bank07:{:04X}", MAIN_ROUTINE_PTR_TABLE.get(mr as usize).copied().unwrap_or(0));
            crate::unported!(&loc); // 其余态（MoreInit 起——迭代分类）
        }
    }
}

/// AreaInit（**asm 内容锚** `AreaInit: ; 07:C801`——游戏侧区域初始化）：
/// `ScrollX($26)/ScrollY($25)←0`/`PPUCTRL_ZP($FF)&=$FC`（名表 0）/`inc MainRoutine`
/// （→MoreInit）/`ABStatus←Joy1Status&(A|B)`/`jsr EraseAllSprites`（C1A3 已折）/
/// `lda #$10/jsr IsEngineRunning`（CA18——页切检桩）。
pub fn area_init(mem: &mut FrameMemory) {
    mem.write_ram(crate::ram_map::SCROLL_X, 0);
    mem.write_ram(crate::ram_map::SCROLL_Y, 0);
    let ff = mem.read_ram(crate::ram_map::PPUCTRL_ZP) & 0xFC;
    mem.write_ram(crate::ram_map::PPUCTRL_ZP, ff);
    mem.write_ram(crate::ram_map::MAIN_ROUTINE, mem.read_ram(crate::ram_map::MAIN_ROUTINE).wrapping_add(1));
    let ab = mem.read_ram(crate::memmap::title_state::JOY1_STATUS) & 0xC0;
    mem.write_ram(0x0027, ab); // ABStatus（:254 族）
    crate::frame::boot::c1a3_unit(mem); // EraseAllSprites（已折）
    is_engine_running(mem, 0x10); // CA18（已端口——A=$10 Brinstar 页备）
}

/// BankTable 烘焙（07:CA30——16 项（A&0xF 索引→BankSwitchPending 值））。
const BANK_TABLE: [u8; 16] = [0x02, 0x03, 0x05, 0x04, 0x06, 0x48, 0x48, 0x20, 0x96, 0xCA, 0xAD, 0x84, 0x68, 0x10, 0x0D, 0x29];

/// IsEngineRunning（**asm 内容锚** `IsEngineRunning: ; 07:CA18`）：
/// `ldy MainRoutine/cpy #IncrementRoutine(7)/beq RTS`（淡入/等待）→`cpy #GameEngine(3)/
/// beq SwitchBank`→否则 RTS。SwitchBank（CA23）：`sta InArea($74)/and #$0F/tay/
/// lda BankTable,y/sta BankSwitchPending($28)/jmp CheckBankSwitch`（已折）。
pub fn is_engine_running(mem: &mut FrameMemory, a_in: u8) {
    let mr = mem.read_ram(crate::ram_map::MAIN_ROUTINE);
    if mr == 7 {
        return; // IncrementRoutine（淡入/等待——RTS）
    }
    if mr != 3 {
        return; // 非 GameEngine——RTS（AreaInit 调用点=此路（MainRoutine=1））
    }
    // SwitchBank
    mem.write_ram(crate::ram_map::IN_AREA, a_in);
    let idx = (a_in & 0x0F) as usize;
    mem.write_ram(crate::ram_map::BANK_SWITCH_PENDING, BANK_TABLE[idx]);
    crate::frame::boot::c4de_unit(mem); // CheckBankSwitch（已折）
}

/// MoreInit（**asm 内容锚** `MoreInit: ; 07:C81D`——区域/房间链头）：
/// PaletteDataPending←1/SpareMem75←$FF/四清（AtEnding/DoorEntryStatus/SamusDoorData/
/// UpdatingSamusProjectile←0）/清环（$7A-$DE+$300-$3FE——**off-by-one bug=D4 保真**
/// （「should clear $DF」注））/ScreenOff/ClearNameTables（已折）/EraseAllSprites（已折）
/// /ScrollBlock 族清/ScrollDir←2/AreaMapPos→MapPos/PaletteToggle/RoomNumber←$FF/
/// CopyAreaPointers→GetRoomNum→SetupRoom 环（房间链——嵌套桩）。
pub fn more_init(mem: &mut FrameMemory) {
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
    mem.write_ram(crate::ram_map::NMI_STATUS, 0); // ScreenOff 域形（屏/可见性关）
    crate::frame::boot::c158_unit(mem); // ClearNameTables（已折）
    crate::frame::boot::c1a3_unit(mem); // EraseAllSprites（已折）
    mem.write_ram(0x005A, 0); // ScrollBlockOnNameTable3
    mem.write_ram(0x0053, 0); // ScrollBlockOnNameTable0
    mem.write_ram(0x0030, 1); // SpareMem30
    mem.write_ram(0x0003, 2); // ScrollDir=2（横房）
    let map_x = mem.read_ram(0x6884); // AreaMapPosX（WRAM 源——随考证）
    mem.write_ram(0x0050, map_x); // MapPosX（constants_ram:543）
    let map_y = mem.read_ram(0x6883); // AreaMapPosY
    mem.write_ram(0x004F, map_y); // MapPosY（constants_ram:542）
    let pt = mem.read_ram(0x6885); // AreaPaletteToggle
    mem.write_ram(0x0076, pt); // PaletteToggle（constants_ram:609）
    mem.write_ram(0x005A, 0xFF); // RoomNumber=$FF（constants_ram:567——undefined room）
    copy_area_pointers(mem); // C8B0（已端口）
    get_room_num(mem); // E720（已端口——壳）
    setup_room(mem); // EA2B（已端口——壳）
}

/// AreaPointers_ROM 烘焙（prg1 Brinstar——`14 a3 72 a3 f0 ae e0 9d e0 9e 0e 9f 6a 9d`
/// ROM 命中（$959A）——7 指针：RoomPtrTable/StructPtrTable/MetatileDefs/EnFramePtrTable1/2/
/// EnPlacePtrTable/EnAnimTable）。
const AREA_POINTERS_ROM: [u8; 14] = [0x14, 0xA3, 0x72, 0xA3, 0xF0, 0xAE, 0xE0, 0x9D, 0xE0, 0x9E, 0x0E, 0x9F, 0x6A, 0x9D];

/// CopyAreaPointers（**asm 内容锚** `CopyAreaPointers: ; 07:C8B0`）：14B 拷
/// AreaPointers_ROM→AreaPointers_RAM（$3B-$48——7 指针结构体）。
pub fn copy_area_pointers(mem: &mut FrameMemory) {
    for (i, b) in AREA_POINTERS_ROM.iter().enumerate() {
        mem.write_ram(0x003B + i as u16, *b);
    }
}

/// GetRoomNum 壳（**asm 内容锚** `GetRoomNum: ; 07:E720`）：ScrollDir 横/纵分流门+
/// ScrollBlock 门（横滚门挡）+MapPos 数学（MapPosY*16+MapPosX）→RoomPtrTable 查表
/// →RoomNumber($5A)。表查=数据面（game-data 房间模型协同——S6-7）。
pub fn get_room_num(mem: &mut FrameMemory) {
    let dir = mem.read_ram(0x0003); // ScrollDir
    if dir & 1 != 0 {
        // 横滚面：ScrollBlock 门（门挡=停滚语义）
        let _a = dir.wrapping_add(0xFF).wrapping_add(1); // A=1（左）/2（右）
        // OnNameTable0（EC93）/ScrollBlock 门——门挡时 RTS（不载房）
        crate::unported!("bank07:EC93"); // OnNameTable0+横滚门面
    }
    // 纵面（LE733）：$00=(MapY*32)+MapX+$7000（WorldMapRAM 索引）→`lda ($00),y`→
    // $FF=未用（RTS carry 置）→`sta RoomNumber($5A)`。
    let map_y = mem.read_ram(0x004F) as u16;
    let map_x = mem.read_ram(0x0050) as u16;
    let idx = (map_y << 5).wrapping_add(map_x); // MapY*32+MapX（asm rol×2 域形）
    let room = mem.read_wram(0x7000 + idx); // WorldMapRAM=$7000（全址形）
    if room == 0xFF {
        return; // 未用房间（carry 置——RTS）
    }
    mem.write_ram(0x005A, room); // RoomNumber
}

/// SetupRoom 壳（**asm 内容锚** `SetupRoom: ; 07:EA2B`）：RoomNumber 分支（$FF=RTS/
/// $FE=占位跳/≥$F0=AttribTableWrite/其余=载房）→RoomPtrTable 查表（**AreaPointers_RAM
/// .RoomPtrTable=$3B/$3C 间接**——`lda (AreaPointers_RAM.RoomPtrTable),y`）→RoomPtr($33/$34)
/// →RoomPalette 读→AddToRoomPtr/SelectRoomRAM/InitTables→DrawRoom（**S6-7 房间绘制链**
/// ——嵌套桩：game-data 房间模型协同）。
pub fn setup_room(mem: &mut FrameMemory) {
    let room = mem.read_ram(0x005A);
    if room == 0xFF {
        return; // 未定义房（RTS）
    }
    if room == 0xFE {
        crate::unported!("bank07:EA5D"); // 占位字节路
    }
    if room >= 0xF0 {
        crate::unported!("bank07:EA9x"); // AttribTableWrite（属性表数据路）
    }
    // RoomPtrTable 查表（AreaPointers_RAM.RoomPtrTable=$3B/$3C 间接——基址拷自 $959A
    // = $A314（Brinstar）——表 47 项烘焙（prg1 提取自证：A441/A454/A45C…））。
    let y = (room as usize) << 1;
    let lo = ROOM_PTR_TABLE[y] as u16;
    let hi = ROOM_PTR_TABLE[y + 1] as u16;
    mem.write_ram(0x0033, lo as u8); // RoomPtr（constants_ram:508）
    mem.write_ram(0x0034, hi as u8);
    // 首字节读（房间数据——RoomPalette）+DrawRoom 链（S6-7 绘制面）
    crate::unported!("bank07:EAAA"); // DrawRoom（S6-7 房间绘制链——嵌套桩）
}

/// RoomPtrTable 烘焙（prg1 Brinstar $A314——47 项；提取自证：A441/A454/A45C…AC6A）。
const ROOM_PTR_TABLE: [u8; 94] = [
        0x41, 0xA4, 0x54, 0xA4, 0x5C, 0xA4, 0x80, 0xA4, 0xBB, 0xA4, 0xED, 0xA4, 0x24, 0xA5, 0x5A, 0xA5,
        0x87, 0xA5, 0xB9, 0xA5, 0xDD, 0xA5, 0x15, 0xA6, 0x35, 0xA6, 0x61, 0xA6, 0x8D, 0xA6, 0xB1, 0xA6,
        0xDB, 0xA6, 0x15, 0xA7, 0x3C, 0xA7, 0x68, 0xA7, 0x8B, 0xA7, 0xA3, 0xA7, 0xD0, 0xA7, 0xF1, 0xA7,
        0x1B, 0xA8, 0x5B, 0xA8, 0x8B, 0xA8, 0xB1, 0xA8, 0xE7, 0xA8, 0x10, 0xA9, 0x2B, 0xA9, 0x6B, 0xA9,
        0x97, 0xA9, 0xC6, 0xA9, 0xF6, 0xA9, 0x20, 0xAA, 0x56, 0xAA, 0xA4, 0xAA, 0xE6, 0xAA, 0x19, 0xAB,
        0x48, 0xAB, 0x71, 0xAB, 0x92, 0xAB, 0xBF, 0xAB, 0x24, 0xAC, 0x4D, 0xAC, 0x6A, 0xAC,
];


/// GameEngine 移动帧环壳（**asm 内容锚** `GameEngine: ; 07:C92B`）：
/// ScrollDoor×2（E1F1 双调=双速滚门）/NARPASSWORD quirk（**D4 保真**——每帧全能力
/// /血/弹）/UpdateWorld（CB29——**m2 深链核心**：Samus/敌人/房间瓦更新——嵌套桩）/
/// 延时旗（MiniBossKill/PowerUp→SetTimer 240 帧）/死亡链（animDelay 爆炸等待→
/// SilenceMusic→MotherBrain 检→GameOver）。
pub fn game_engine(mem: &mut FrameMemory) {
    crate::unported!("bank07:E1F1"); // ScrollDoor×2（门滚——随件）
    // NARPASSWORD quirk（D4 保真：每帧全能力）
    if mem.read_ram(0x0686) != 0 {
        // NARPASSWORD 面（址随考——$686 族）
        mem.write_ram(crate::ram_map::HEALTH_CHANGE + 1, 0x03); // Health+1
        mem.write_ram(crate::ram_map::SAMUS_GEAR, 0xFF);
        mem.write_ram(crate::ram_map::MISSILE_COUNT, 0x05);
    }
    update_world(mem); // CB29（m2 深链核心——已端口壳）
}

/// UpdateWorld 壳（**asm 内容锚** `UpdateWorld: ; 07:CB29`——m2 深链 19 例程面）：
/// SpritePagePos←0→UpdateAllEnemies/UpdateSamusProjectiles/**UpdateSamus（CC0D
/// ——samus 域件已折会师）**/AreaRoutine/UpdateElevator/**UpdateAllStatues（D9D4
/// ——T45-1 已折）**/…/CollisionDetection（D976/F1FA 族）/…→精灵清尾（$F4 残槽填充）。
pub fn update_world(mem: &mut FrameMemory) {
    mem.write_ram(0x0006, 0); // SpritePagePos←0（:256 族）
    crate::unported!("bank07:F345"); // UpdateAllEnemies（敌面——port2 域）
    // 例程序（随件逐个）：UpdateSamusProjectiles(D4BF)/UpdateSamus(CC0D——samus 域
    // 会师)/AreaRoutine(95C3)/UpdateElevator(D7B3)/UpdateAllStatues(D9D4 已折)/
    // UpdateAllEnemyExplosions/UpdateAllMellows/UpdateAllEnProjectiles/
    // UpdateAllSkreeProjectiles/SamusEnterDoor(8B13)/UpdateAllDoors(门链已考)/
    // UpdateAllTileBlasts/CollisionDetection(D976/F1FA)/DisplayBar(E0C1)/
    // UpdateAllPipeBugHoles/CheckMissileToggle/UpdateAllPowerUps(DB37)/
    // UpdateTourianItems(FDE3)→精灵清尾（$F4 残槽）。
}
/// UpdateTimer 单元（**asm 内容锚**）——`UpdateTimer: ; 07:C266`：TimerDelay 预分频
/// （dec/bpl——10 帧节拍，归零重置 9 并含 Timer3）→ Timer1/2/3 非零递减。
pub fn update_timer(mem: &mut FrameMemory) {
    // 寄存器残迹注（B 案 $69 机理）：asm 定时环 `ldx #$02/lda $2A,x/dex/bpl`——
    // **dex 出口 X=$FF**（JumpEngine `stx TempX($69)` 保存面源）。
    mem.regs.x = 0xFF;
    let delay = mem.read_ram(TIMER_DELAY).wrapping_sub(1);
    mem.write_ram(TIMER_DELAY, delay);
    let last = if (delay as i8) >= 0 { 1 } else {
        mem.write_ram(TIMER_DELAY, 0x09);
        2
    };
    for i in (0..=last).rev() {
        let addr = TIMER1 + i as u16;
        let v = mem.read_ram(addr);
        if v != 0 {
            mem.write_ram(addr, v - 1);
        }
    }
}

/// GoMainRoutine 分发壳（**asm 内容锚+结构互核注**）——`GoMainRoutine: ; 07:C114`：
/// GameMode≠0 ⇒ `jmp MainTitleRoutine`（**00:8000 会师点**——port2 title 簇头）；
/// GameMode=0 ⇒ START 检查/主例程分发（游戏面——task-11 面 fail-loud）。
pub fn go_main_routine(mem: &mut FrameMemory) {
    if mem.read_ram(GAME_MODE) != 0 {
        super::title_bridge::main_title_routine(mem); // **会师接线**（bank00:8000 穿越）
        return; // 标题路终（asm：`jmp MainTitleRoutine` 尾跳语义）
    }
    game_main_routine(mem); // 游戏侧分发（C11B+——T45-3 移动臂首靶）
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RandomNumbers 合成值自证（+$19/+$5F——注×5 互证面）。
    #[test]
    fn random_numbers_accumulates() {
        let mut m = FrameMemory::default();
        m.write_ram(RANDOM1, 0x11); // boot 初值（C0AC 自证）
        m.write_ram(RANDOM2, 0xFF);
        random_numbers(&mut m);
        assert_eq!(m.read_ram(RANDOM1), 0x2A);
        assert_eq!(m.read_ram(RANDOM2), 0x5E);
        assert_eq!(m.regs.a, 0x2A);
    }

    /// UpdateTimer 预分频+三计时器递减。
    #[test]
    fn update_timer_prescaler_and_decrement() {
        let mut m = FrameMemory::default();
        m.write_ram(TIMER_DELAY, 3);
        m.write_ram(TIMER1, 5);
        m.write_ram(TIMER2, 0); // 零不减
        update_timer(&mut m);
        assert_eq!(m.read_ram(TIMER_DELAY), 2); // 未归零（不含 Timer3）
        assert_eq!(m.read_ram(TIMER1), 4);
        assert_eq!(m.read_ram(TIMER2), 0);
        m.write_ram(TIMER_DELAY, 0); // dec→0xFF 负值→重置 9+含 Timer3
        m.write_ram(TIMER3, 2);
        update_timer(&mut m);
        assert_eq!(m.read_ram(TIMER_DELAY), 9);
        assert_eq!(m.read_ram(TIMER3), 1);
    }

    /// GoMainRoutine 标题路**会师穿越**（bank00:8000 已通——phase 0 臂推进效应）。
    #[test]
    fn go_main_routine_title_crosses_meeting_point() {
        let mut m = FrameMemory::default();
        m.write_ram(GAME_MODE, 1); // TitleRoutine=0=InitializeAfterReset
        go_main_routine(&mut m); // 桥穿越（phase 0 臂执行）
        assert_eq!(m.read_ram(crate::ram_map::TITLE_ROUTINE), 1); // 相位推进（inc $1F）
    }
}

