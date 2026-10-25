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
///
/// **迭代序门**（r14 复核 F3/F4 修订版——asm 内容锚）：主循环体四件
/// （RandomNumbers C000/CheckBankSwitch/UpdateTimer C266/inc FrameCount）=**每迭代恰好
/// 一次**；spin/等待段 CPU 自旋在 GoMainRoutine 内部 ⇒ 迭代未完成、四件全冻结
/// （O f3→f7 四帧窗口仅 1 次 UpdateTimer dec 实证）。
/// · 续段判定=入口 title_cursor>0 且**归属相位==当前相位**（F4：相位推进后残留的
///   孤儿 cursor=新相位首个真迭代——禁误门为续段）。
/// · 迭代收尾（inc FrameCount）在**步骤完成点**执行：游标归零 或 分发推进了相位
///   （孤儿游标=步骤边界）。
/// · regs.a 连续性：续段不跑 random_numbers（其写 regs.a）——分发臂继承段间 A 态（F3）。
pub fn main_loop_iter(mem: &mut FrameMemory) {
    let phase_now = mem.read_ram(crate::ram_map::TITLE_ROUTINE);
    // 【r23】续段=同相位续段（F4 孤儿语义不动）**或 spin 等待段**（title_spin>0——臂的
    // 跨帧等待段正交机构）。
    let continuation = (mem.title_cursor > 0 && mem.title_cursor_phase == phase_now)
        || mem.title_spin > 0;
    if !continuation {
        random_numbers(mem); // C000（唤醒后首件）
        super::boot::c4de_unit(mem); // CheckBankSwitch（已端口复用）
        update_timer(mem); // C266
    }
    go_main_routine(mem); // C114（分发壳——会师点；续段=GoMainRoutine 内部续跑）
    // 【r23 修（$C3 根收窄实验）】`inc FrameCount`（GoMainRoutine 返回点）=**非续段逐迭代**
    // （asm 形：每迭代收尾一 inc——等待帧非迭代不达）；原 step_done（尾段）判据把补帧
    // 步的 FC 拖到尾段 ⇒ NMI 间 FC=0 窗宽 2（$C3 星门复燃根）。续段（cursor>0 同相）不计。
    if !continuation {
        mem.write_ram(FRAME_COUNT, mem.read_ram(FRAME_COUNT).wrapping_add(1));
    }
    mem.write_ram(NMI_STATUS, 0); // 清 0（spin 入口）
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
        2 => samus_init(mem), // SamusInit（r25 端口——萨姆斯出场态）
        3 => game_engine(mem), // GameEngine（已端口——移动帧环壳）
        8 => samus_intro(mem), // SamusIntro（r25 端口——淡入序+sprite 装载）
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

/// SamusInit（**asm 内容锚** `SamusInit: ; 07:C8D1`——r25 端口：萨姆斯出场态）：
/// ①MainRoutine←SamusIntro(9) ②Timer3←$2C（440 帧淡入）③IntroMusic（$CBFD——
/// SFXMultiInitFlags($0684)←$10 音乐起）④Samus.status←_id_Palette13+1=**$14**（淡入态）
/// ⑤InvincibleDelay←0 ⑥敌槽清（X=$FF：PipeBugHoles .0/.1.status+.1.y（**BUG 保留**（D4）
/// asm 注「should be .2/.3」）+.2.status=$728/$730/$732/$738、EndTimer($010A/$010B)、
/// RinkaSpawners .0/.1=$8B/$8E）⑦InArea&$0F：0=Brinstar（Y=$27）/else=$2F+ScrollDir lsr
/// （址未定谳——fail-loud（不猜写））⑧MirrorCntrl($FA)/MissilePickupQtyMax($93)/
/// EnergyPickupQtyMax($94)←Y ⑨Samus.y←AreaSamusY[area]（$95D9 五值烘焙）/x←$80/
/// hi←PPUCTRL_ZP&1 ⑩Health($0106)←30（$00/$03）。
pub fn samus_init(mem: &mut FrameMemory) {
    use crate::ram_map::{IN_AREA, MIRROR_CNTRL, MAIN_ROUTINE, SAMUS_STATUS, SAMUS_Y};
    mem.write_ram(MAIN_ROUTINE, 8); // SamusIntro（下一帧——表序 _id_SamusIntro=8（r25 正））
    mem.write_ram(0x002C, 0x2C); // Timer3（440 帧淡入）
    mem.write_ram(crate::ram_map::SFX_MULTI_INIT, 0x10); // IntroMusic（$CBFD）
    mem.write_ram(SAMUS_STATUS, 0x14); // _id_Palette13+1（淡入态）
    mem.write_ram(0x0070, 0); // SamusInvincibleDelay
    for a in [0x0728u16, 0x0730, 0x0732, 0x0738] {
        mem.write_ram(a, 0xFF); // PipeBugHoles（含 .1.y BUG 位——D4 保留）
    }
    mem.write_ram(0x010A, 0xFF);
    mem.write_ram(0x010B, 0xFF); // EndTimer（escape 未活）
    mem.write_ram(0x008B, 0xFF);
    mem.write_ram(0x008E, 0xFF); // RinkaSpawners.0/.1
    let y = if mem.read_ram(IN_AREA) & 0x0F == 0 {
        0x27u8 // Brinstar
    } else {
        // 非 Brinstar：`lsr ScrollDir`+Y=$2F（ScrollAddr 未定谳——fail-loud 不猜写）
        crate::unported!("bank07:C902-ScrollDir");
        0x2F
    };
    mem.write_ram(MIRROR_CNTRL, y);
    mem.write_ram(0x0093, y); // MissilePickupQtyMax
    mem.write_ram(0x0094, y); // EnergyPickupQtyMax
    let area = (mem.read_ram(IN_AREA) & 0x0F) as usize;
    let y_pos = *game_data::title::area_tables::AREA_SAMUS_Y
        .get(area)
        .unwrap_or(&game_data::title::area_tables::AREA_SAMUS_Y[0]);
    mem.write_ram(SAMUS_Y, y_pos);
    mem.write_ram(0x030E, 0x80); // Samus.x
    mem.write_ram(0x030C, mem.read_ram(crate::ram_map::PPUCTRL_ZP) & 0x01); // Samus.hi
    mem.write_ram(0x0106, 0x00);
    mem.write_ram(0x0107, 0x03); // Health=30
}

/// SamusIntro（**asm 内容锚** `SamusIntro: ; 07:C9D7`——r25 端口：淡入序+sprite 装载）：
/// ①EraseAllSprites ②Timer3==0⇒淡入完成（ItemRoomMusicStatus←0/Samus.status←sa_Begin(255)/
/// StartMusic($D92C)/SelectSamusPalette($CB73)/MainRoutine←GameEngine(3)——双音乐件随件）
/// ③Timer3≥$1F⇒退出（310 帧内不画）④SamusFadeInTimeTbl($CA13: 1E/14/0B/04/FF) 对照：
/// Timer3==表[y-$14]⇒inc Samus.status+PaletteDataPending←y（淡入调色步进）⑤奇帧（FC&1）
/// 闪现：SetSamusAnim(SamusFront)+SpritePagePos←0+PageIndex←0+AnimDrawObject($DE47)。
pub fn samus_intro(mem: &mut FrameMemory) {
    crate::frame::boot::c1a3_unit(mem); // EraseAllSprites（$C1A3）
    let status = mem.read_ram(0x0300); // Samus.status（$300）
    let t3 = mem.read_ram(0x002C); // Timer3（$2C）
    if t3 == 0 {
        // 淡入完成（音乐件随件爆）
        mem.write_ram(0x009A, 0); // ItemRoomMusicStatus（址随证）
        mem.write_ram(0x0300, 0xFF); // sa_Begin（constants.asm:121 export=255）
        crate::unported!("bank07:D92C"); // StartMusic+SelectSamusPalette（淡出径随件）
        #[allow(unreachable_code)]
        {
            mem.write_ram(crate::ram_map::MAIN_ROUTINE, 3);
            return;
        }
    }
    if t3 >= 0x1F {
        return; // 310 帧内不画（asm `cmp #$1F / bcs`）
    }
    // SamusFadeInTimeTbl（$CA13）：Timer3==表[y-$14]⇒inc status+PaletteDataPending←y
    let idx = status.wrapping_sub(0x14) as usize;
    const FADE_TBL: [u8; 5] = [0x1E, 0x14, 0x0B, 0x04, 0xFF]; // $CA13 五值（ROM 锚）
    if idx < FADE_TBL.len() && t3 == FADE_TBL[idx] {
        mem.write_ram(0x0300, status.wrapping_add(1));
        mem.write_ram(crate::ram_map::PALETTE_PENDING, status); // sty（Y=status）
    }
    // 奇帧闪现（FC&1）——sprite 装载径
    if mem.read_ram(crate::ram_map::FRAME_COUNT) & 1 == 1 {
        crate::unported!("bank07:DE47"); // AnimDrawObject（sprite 管线——下一缺环）
    }
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
    let map_x = game_data::title::world_map::AREA_MAP_POS
        [area_idx(mem.read_ram(crate::ram_map::IN_AREA))].0; // ROM $95D7（r25 正源）
    mem.write_ram(0x0050, map_x); // MapPosX（constants_ram:543）
    let map_y = game_data::title::world_map::AREA_MAP_POS
        [area_idx(mem.read_ram(crate::ram_map::IN_AREA))].1; // ROM $95D8
    mem.write_ram(0x004F, map_y); // MapPosY（constants_ram:542）
    let pt = mem.read_ram(0x6885); // AreaPaletteToggle
    mem.write_ram(0x0076, pt); // PaletteToggle（constants_ram:609）
    mem.write_ram(0x005A, 0xFF); // RoomNumber=$FF（constants_ram:567——undefined room）
    copy_area_pointers(mem); // C8B0（已端口）
    get_room_num(mem); // E720（已端口——壳）
    // 【r25 对拍根】asm LC86F 房间环：`jsr SetupRoom/ldy RoomNumber/iny/bne LC86F`
    // ——循环至 RoomNumber==FF（多房装载序）。
    loop {
        setup_room(mem); // EA2B（DrawRoom 未端口面=fail-loud 随爆）
        if mem.read_ram(0x005A).wrapping_add(1) == 0 {
            break;
        }
    }
    // RoomRAM($6000+)→PPU 名表 #0($2000) 4 页拷贝（asm Lx001 `lda ($00),y/sta PPUDATA`
    // ——**黑屏根件**：名表从 RoomRAM 落 RenderState）。
    mem.write_ram(0x0000, mem.read_ram(0x0037)); // RoomRAMPtr→$00/$01（asm $37/$38 面）
    mem.write_ram(0x0001, mem.read_ram(0x0038));
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
    // jmp ScreenOn（$FE|=$1E 面——shell 组合点随件；先事件形）
    let fe = mem.read_ram(0x00FE);
    mem.write_ram(0x00FE, fe | 0x1E);
}

/// 区序索引（InArea&$0F 越界稳——AREA_MAP_POS 五项）。
pub fn area_idx(in_area: u8) -> usize {
    (in_area & 0x0F) as usize % 5
}

/// CopyMap（**asm 内容锚** `CopyMap: ; 00:A93E`——WorldMap→WRAM $7000-$73FF 1KB）。
/// 调用点=InitBank0（prg7_engine.asm:1382——**复位链件**：boot 期一次）。
pub fn copy_map(mem: &mut FrameMemory) {
    for (i, b) in game_data::title::world_map::WORLD_MAP.iter().enumerate() {
        mem.write_wram(0x7000 + i as u16, *b);
    }
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
        // AttribTableWrite（$EA13——r25 端口：$F0-$F4 命令链）
        crate::frame::room::attrib_table_write(mem);
        return;
    }
    // RoomPtrTable 查表（AreaPointers_RAM.RoomPtrTable=$3B/$3C 间接——基址拷自 $959A
    // = $A314（Brinstar）——表 47 项烘焙（prg1 提取自证：A441/A454/A45C…））。
    let y = (room as usize) << 1;
    let lo = ROOM_PTR_TABLE[y] as u16;
    let hi = ROOM_PTR_TABLE[y + 1] as u16;
    mem.write_ram(0x0033, lo as u8); // RoomPtr（constants_ram:508）
    mem.write_ram(0x0034, hi as u8);
    // 首字节读（房间数据——RoomPalette=$36 址随证）+SelectRoomRAM($EA05)/InitTables($EFF8)
    // +DrawRoom 链（S6-7 绘制面——r25 端口）。
    let sidx = y / 2;
    let _ = sidx;
    crate::frame::room::select_room_ram(mem);
    let room_stream = game_data::area::graphics::ROOM_STREAMS.get(room as usize);
    let palette = room_stream.and_then(|s| s.first().copied()).unwrap_or(0);
    mem.write_ram(0x0036, palette); // RoomPalette（$36 址随证——InitTables 索引源）
    crate::frame::room::init_tables(mem);
    if !crate::frame::room::draw_room(mem, room) {
        crate::unported!("bank07:EAD4"); // 敌门环未竟件（type 2/4/6/7 族）
    }
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
    // UpdateAllEnemies（F345——port2 敌面域；随件）
    crate::unported!("bank07:F345");
    // UpdateSamusProjectiles（D4BF——随件）
    crate::unported!("bank07:D4BF");
    crate::samus::update_samus(mem); // UpdateSamus（CC0D——**samus 域件会师**！）
    crate::frame::statue::update_all_statues(mem); // UpdateAllStatues（D9D4——T45-1 会师）
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

    /// 迭代序门（r14 复核 F3/F4）：续段 tick 冻结 RandomNumbers/CheckBankSwitch/UpdateTimer
    /// （等待段自旋于 GoMainRoutine 内部——循环体四件每迭代一次）。
    #[test]
    fn continuation_tick_freezes_iteration_items() {
        let mut m = crate::frame::FrameMemory::default();
        m.write_ram(crate::ram_map::TITLE_ROUTINE, 1);
        m.title_cursor = 1;
        m.title_cursor_phase = 1; // 同相位续段（真续段）
        m.write_ram(crate::ram_map::RANDOM1, 0x10);
        m.write_ram(crate::ram_map::RANDOM2, 0x20);
        m.write_ram(TIMER_DELAY, 5);
        m.write_ram(FRAME_COUNT, 7);
        main_loop_iter(&mut m);
        assert_eq!(m.read_ram(crate::ram_map::RANDOM1), 0x10, "RandomNumbers 冻结");
        assert_eq!(m.read_ram(crate::ram_map::RANDOM2), 0x20, "RandomNumbers 冻结");
        assert_eq!(m.read_ram(TIMER_DELAY), 5, "UpdateTimer 冻结");
    }

    /// F4 孤儿游标=新相位首个真迭代（相位推进后残留 cursor——禁误门）：循环体跑。
    #[test]
    fn orphan_cursor_after_phase_advance_is_fresh_iteration() {
        let mut m = crate::frame::FrameMemory::default();
        m.write_ram(crate::ram_map::TITLE_ROUTINE, 2);
        m.title_cursor = 2;
        m.title_cursor_phase = 1; // 归属相位≠当前（孤儿）
        m.write_ram(TIMER_DELAY, 5);
        main_loop_iter(&mut m);
        assert_eq!(m.read_ram(TIMER_DELAY), 4, "UpdateTimer 跑（真迭代——禁误门）");
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

    /// GoMainRoutine 标题路**会师穿越**（bank00:8000 已通——phase 0 臂单工作段+SpinWait
    /// （r23 机构改形）：相位推进/FC 同拍落工作段）。
    #[test]
    fn go_main_routine_title_crosses_meeting_point() {
        let mut m = FrameMemory::default();
        m.write_ram(GAME_MODE, 1); // TitleRoutine=0=InitializeAfterReset
        go_main_routine(&mut m); // 工作段（仪式+推进同拍）
        assert_eq!(m.read_ram(crate::ram_map::TITLE_ROUTINE), 1); // 相位推进（inc $1F）
        assert_eq!(m.title_spin, 1, "spin 等待段挂起");
    }
}

