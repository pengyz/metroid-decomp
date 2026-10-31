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
    // 【Step 4 完成拍哨兵】cursor==0xFF=臂末窗（asm：例程体尾段自旋窗——rts 后迭代
    // 收尾 `inc FrameCount`（prg7:207）落本窗尾）。无 prelude（同迭代早过 RandomNumbers/
    // CheckBankSwitch/UpdateTimer——emu 0x01/0x1B 窗各仅 1 次 TD dec 实录，map-v2 §3）；
    // 无行（title_continuation 的 0xFF 项挡缝——单源谓词同源）。0xFF 不在合法相位/
    // 游标值域（<0x1E）——与 F4 孤儿规则零碰撞。
    if mem.title_cursor == 0xFF {
        mem.title_cursor = 0;
        mem.write_ram(FRAME_COUNT, mem.read_ram(FRAME_COUNT).wrapping_add(1));
        mem.write_ram(NMI_STATUS, 0);
        return;
    }
    // 【Step 3 谓词单源】续段/完成拍/spin 判定=FrameMemory::title_continuation（唯一实现
    // ——engine 缝抑制同源；0xFF 完成拍哨兵显式排除出续段，Step 4 启用）。
    let continuation = mem.title_continuation();
    if !continuation {
        random_numbers(mem); // C000（唤醒后首件）
        super::boot::c4de_unit(mem); // CheckBankSwitch（已端口复用）
        update_timer(mem); // C266
    }
    go_main_routine(mem); // C114（分发壳——会师点；续段=GoMainRoutine 内部续跑）
    // 【FC 冻结（r9 考证·porter 修）】`inc FrameCount`（asm prg7:207）在 **MainLoop 迭代尾**
    // ——例程内部 WaitNMIPass_ 自旋跨帧时迭代未完成⇒不自增。emu 实录：上电 f4-6 与
    // title→menu f311-313 各冻 3 帧（findings/fc-phase-residual.md——桥侧按内旋族相位
    // $01/$15/$1B 置 no_return 旗）；menu→game 冻结走既有 span 门（iter_span_fc）不受此管。
    // 旧形每 tick 无条件递增（goal-r4）治了滞后但过量 +6（=两窗 3+3），本门即 emu 对齐。
    if !mem.title_no_return {
        mem.write_ram(FRAME_COUNT, mem.read_ram(FRAME_COUNT).wrapping_add(1));
    }
    mem.title_no_return = false;
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
    if std::env::var("DSH_MR_TRACE").is_ok() {
        eprintln!("[mr-trace] FC={} mr={:02X} status={:02X} t3={:02X}", mem.read_ram(crate::ram_map::FRAME_COUNT), mr, mem.read_ram(0x0300), mem.read_ram(0x002C));
    }
    if (mr as usize) < 10 {
        let target = MAIN_ROUTINE_PTR_TABLE[mr as usize];
        // JumpEngine 指针写（$0C/$0D——同标题面）
        mem.write_ram(0x000C, target as u8);
        mem.write_ram(0x000D, (target >> 8) as u8);
    }
    match mr {
        0 => {
            // 【move-drift R1 链合一】asm AreaInit（C801）**无 RTS fallthrough 进 MoreInit**
            // （prg7:1826-1843——:1839 `jsr IsEngineRunning` 后直落 C81D）：一迭代双步
            // （mr 00→02——金腿窗3 状态差实证）。原拆两迭代=同迭代链断裂（定谳 R1）。
            area_init(mem);
            more_init(mem);
        }
        1 => more_init(mem), // MoreInit 独派（mr=01 直设路径——asm 分派表项 1）
        2 => samus_init(mem), // SamusInit（r25 端口——萨姆斯出场态）
        3 => game_engine(mem), // GameEngine（已端口——移动帧环壳）
        8 => samus_intro(mem), // SamusIntro（r25 端口——淡入序+sprite 装载）
        _ => {
            let loc = format!("bank07:{:04X}", MAIN_ROUTINE_PTR_TABLE.get(mr as usize).copied().unwrap_or(0));
            crate::unported!(&loc); // 其余态（MoreInit 起——迭代分类）
        }
    }
    if std::env::var("DSH_MR_TRACE").is_ok() {
        eprintln!("[mr-post] FC={} status={:02X} oam0={:02X} pal={:02X?}", mem.read_ram(crate::ram_map::FRAME_COUNT), mem.read_ram(0x0300), mem.read_ram(0x0200), &mem.render.palette[..16]);
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
        // 淡入完成（r26 端口——asm 逐字）：ItemRoomMusicStatus($79)←0/Samus.status←
        // sa_Begin(255)/StartMusic($D92C)/SelectSamusPalette($CB73)/MainRoutine←3
        mem.write_ram(0x0079, 0); // ItemRoomMusicStatus（constants:614）
        mem.write_ram(0x0300, 0xFF); // sa_Begin（constants.asm:121 export=255）
        start_music(mem); // $D92C
        select_samus_palette(mem); // $CB73
        mem.write_ram(crate::ram_map::MAIN_ROUTINE, 3);
        return;
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
    // 奇帧闪现（FC&1）——sprite 装载径（r25 端口：AnimDrawObject 全径）
    if mem.read_ram(crate::ram_map::FRAME_COUNT) & 1 == 1 {
        // SetSamusAnim(SamusFront)（$CF6B）：animResetIndex/animIndex←表偏 **4**
        // （ObjAnim_SamusFront $8576-$8572——common_sprite_data 1c）+animDelay←0
        mem.write_ram(0x0305, 4);
        mem.write_ram(0x0306, 4);
        mem.write_ram(0x0304, 0);
        // 【r25 修】SpritePagePos($5B)/PageIndex($4B)←0（asm `lda #$00/sta`——每帧重置！
        // 漏写致 sprite 指针跨帧累加溢写 $300+（Samus.status 擦除根））
        mem.write_ram(0x005B, 0);
        mem.write_ram(0x004B, 0);
        // 【goal-r16 修】asm DE47(AnimDrawObject)=`jsr UpdateObjAnim`(A=0)+DE4A——首句走表。
        // 旧形（拆分后）漏走表⇒animFrame 停留 0(Stand 帧 id)⇒渲染 Stand（emu 渲 Front 帧——L4 f550 差 210px）。
        crate::samus::physics::update_obj_anim(mem, 0); // UpdateObjAnim（DC8F——A=0 `lda #$00`）
        crate::frame::sprite::anim_draw_object(mem, 0); // AnimDrawObject（$DE47——Samus 槽 0）
    }
}

/// StartMusic（**asm 内容锚** `StartMusic: ; 07:D92C`——r26 端口）：
/// ①电梯检（Elevator.status($0320)==6 且 ElevatorType($032F) MSB=1（上行）⇒item 径）
/// ②正常径：AreaMusicFlag（ROM 区 bank $95CD）+ItemRoomMusicStatus($79) 判
/// （bmi/beq⇒Lx114）③item 径：ItemRoomMusicStatus←$81/A←music_ItemRoom($20)
/// ④Lx114：`ora MusicInitFlags($0685) / sta MusicInitFlags`（**音乐起**——声链消费面）。
pub fn start_music(mem: &mut FrameMemory) {
    let area = (mem.read_ram(crate::ram_map::IN_AREA) & 0x0F) as usize % 5;
    let mut a = game_data::title::area_tables::AREA_MUSIC_FLAG[area];
    let elevator_up = mem.read_ram(0x0320) == 0x06 && mem.read_ram(0x032F) & 0x80 != 0;
    let item_room = mem.read_ram(0x0079);
    if elevator_up || (item_room != 0 && item_room & 0x80 == 0) {
        mem.write_ram(0x0079, 0x81); // ItemRoomMusicStatus←$81（item room music 标）
        a = 0x20; // music_ItemRoom（constants.asm export=%00100000）
    }
    let f = mem.read_ram(0x0685) | a; // ora MusicInitFlags / sta MusicInitFlags
    mem.write_ram(0x0685, f);
}

/// SelectSamusPalette（**asm 内容锚** `SelectSamusPalette: ; 07:CB73`——r26 端口）：
/// SamusGear($6878) bit5（Varia）→carry；MissileToggle($010E) rol⇒A=(m<<1)|v；
/// adc #_id_Palette01+1(=2)；JustInBailey($69B3)≠0⇒adc #$17（no-suit 族）；
/// sta PaletteDataPending($1C)（下一 NMI 写调色板）。
pub fn select_samus_palette(mem: &mut FrameMemory) {
    let gear = mem.read_ram(0x6878);
    let varia = (gear >> 5) & 1; // asl×3 出 carry=bit5
    let missile = mem.read_ram(0x010E) & 0x01;
    let mut id = 2u8.wrapping_add((missile << 1) | varia); // _id_Palette01+1=2
    if mem.read_ram(0x69B3) != 0 {
        id = id.wrapping_add(0x17); // _id_Palette18-_id_Palette01=$17（no-suit 族）
    }
    mem.write_ram(crate::ram_map::PALETTE_PENDING, id);
}

/// GameEngine 移动帧环壳（**asm 内容锚** `GameEngine: ; 07:C92B`——r26 端口）：
/// ScrollDoor×2（E1F1 双调=双速滚门）/NARPASSWORD quirk（**D4 保真**）/UpdateWorld
/// （CB29——19 件环：敌人/弹/萨姆斯/区域/梯/像/爆/虫/门/瓦碰/状态栏/能力件）/
/// 延时旗（MiniBossKill/PowerUp→SetTimer 240 帧）/死亡链（SilenceMusic→GameOver）。
pub fn game_engine(mem: &mut FrameMemory) {
    scroll_door(mem); // E1F1×2（双速滚门——2 调=2 px/帧）
    scroll_door(mem);
    if mem.read_ram(0x69B2) != 0 {
        // NARPASSWORD quirk（D4 保真）：每帧满血/满装/满弹（$69B2——constants:1263）
        mem.write_ram(0x0107, 0x03); // Health+1=3（血 30）
        mem.write_ram(0x6878, 0xFF); // SamusGear（全能力！）
        mem.write_ram(0x6879, 0x05); // MissileCount（$6879——constants:1210）
    }
    update_world(mem); // CB29（19 件环——渐进端口面）
    // 延时旗（MiniBossKill/PowerUp→SetTimer 240 帧→routine 3）
    if mem.read_ram(0x0083) != 0 || mem.read_ram(0x0084) != 0 {
        mem.write_ram(0x0083, 0);
        mem.write_ram(0x0084, 0);
        // SetTimer（$C4AA——Timer3←240 帧/routine←3）址随证
        mem.write_ram(0x002C, 0x18);
        mem.write_ram(crate::ram_map::MAIN_ROUTINE, 3);
    }
    // 死亡链（Samus.status==sa_Dead2(=$FD?) 且 animDelay==0⇒SilenceMusic+GameOver）
    let status = mem.read_ram(0x0300);
    if status == 0xFD && mem.read_ram(0x0304) == 0 {
        // SilenceMusic（$B283 族——SFXNoiseInitFlags|=sfxNoise_SilenceMusic）
        let f = mem.read_ram(0x0684) | 0x01;
        mem.write_ram(0x0684, f);
        if mem.read_ram(0x687B) == 0x0A {
            // MotherBrainStatus==0x0A⇒inc MainRoutine（→GameOver）
            let mr = mem.read_ram(crate::ram_map::MAIN_ROUTINE);
            mem.write_ram(crate::ram_map::MAIN_ROUTINE, mr.wrapping_add(1));
        } else {
            mem.write_ram(0x002C, 0x04); // SetTimer 40 帧→routine 4（GameOver）
            mem.write_ram(crate::ram_map::MAIN_ROUTINE, 4);
        }
    }
}

/// ScrollDoor（**asm 内容锚** `ScrollDoor: ; 07:E1F1`——r26 端口）：
/// DoorEntryStatus($56)：0=非门出/1=右滚(ScrollRight $E6D2)/2=左滚(ScrollLeft $E6A7)/
/// 3=下滚(ScrollDown $E519)/4=上滚(ScrollUp $E4F1)；滚至 ScrollX/Y 归中⇒DoorEntryStatus←5
/// （出门）。滚件=1px/调（双调=双速）。
pub fn scroll_door(mem: &mut FrameMemory) {
    let d = mem.read_ram(0x0056);
    if d == 0 {
        return;
    }
    match d {
        1 => {
            // ScrollRight（$E6D2）：ScrollX+1（越 255⇒名表换）
            scroll_by(mem, 0x0026, true);
            if mem.read_ram(0x0026) == 0 {
                mem.write_ram(0x0056, 0x05); // DoOneDoorScroll 出门面
            }
        }
        2 => {
            scroll_by(mem, 0x0026, false);
            if mem.read_ram(0x0026) == 0 {
                mem.write_ram(0x0056, 0x05);
            }
        }
        3 => {
            scroll_by(mem, 0x0025, true);
            if mem.read_ram(0x0025) == 0 {
                mem.write_ram(0x0056, 0x05); // VerticalRoomCentered 形
            }
        }
        4 => {
            scroll_by(mem, 0x0025, false);
            if mem.read_ram(0x0025) == 0 {
                mem.write_ram(0x0056, 0x05);
            }
        }
        _ => {} // 5+（出门序——SamusEnterDoor 面随件）
    }
}

/// 滚 1px（ScrollRight/Left/Up/Down 簇共径——名表换/换页件随证）。
fn scroll_by(mem: &mut crate::frame::FrameMemory, addr: u16, up: bool) {
    let v = mem.read_ram(addr);
    let nv = if up { v.wrapping_add(1) } else { v.wrapping_sub(1) };
    mem.write_ram(addr, nv);
}

/// UpdateWorld（**asm 内容锚** `UpdateWorld: ; 07:CB29`——r26 环壳：19 件渐进端口）。
/// 件序：SpritePagePos←0/UpdateAllEnemies($F345)/UpdateSamusProjectiles($D4BF)/
/// UpdateSamus($CC0D)/AreaRoutine($95C3)/UpdateElevator($D7B3)/UpdateAllStatues($D9D4)/
/// 爆/虫/弹/瓦碰/SamusEnterDoor($8B13)/UpdateAllDoors/TileBlasts/CollisionDetection/
/// DisplayBar($E0C1)/PipeBugHoles/CheckMissileToggle/UpdateAllPowerUps($DB37)/
/// UpdateTourianItems($FDE3)/尾清 sprite RAM（$F4 填）。
pub fn update_world(mem: &mut crate::frame::FrameMemory) {
    mem.write_ram(0x005B, 0); // SpritePagePos←0
    update_all_enemies(mem); // F345（敌环——槽×6+状态 dispatch）
    // UpdateSamusProjectiles（D4BF——随件）
    update_samus_projectiles(mem); // D4BF（弹件环——3 槽状态机）
    #[allow(unreachable_code)]
    {
        crate::samus::update_samus(mem); // UpdateSamus（CC0D——samus 域件会师）
        crate::frame::statue::update_all_statues(mem); // UpdateAllStatues（D9D4 会师）
        // ---- r26 dispatch 壳补齐（复核 C blocker——asm 件序 5-20 逐件！）----
        uw_area_routine(mem); // $95C3（区专径）
        uw_elevator(mem); // $D7B3
        uw_enemy_explosions(mem); // 爆族
        uw_mellows(mem); // 虫族
        uw_en_projectiles(mem); // 敌弹族
        uw_skree_projectiles(mem); // Skree 弹族
        uw_samus_enter_door(mem); // $8B13（过门检！）
        uw_all_doors(mem); // 门链
        uw_tile_blasts(mem); // 瓦片爆/复原
        uw_collision_detection(mem); // 实体碰撞
        uw_display_bar(mem); // $E0C1（HUD 动态面！）
        uw_pipe_bug_holes(mem); // 虫洞族
        uw_check_missile_toggle(mem); // 弹切检
        uw_all_power_ups(mem); // $DB37（吃能力！）
        uw_tourian_items(mem); // $FDE3
        // 尾清 sprite RAM（$F4 填——asm CB6A 环）
        let mut x = mem.read_ram(0x005B);
        while x != 0 {
            mem.write_ram(0x0200 + x as u16, 0xF4);
            x = x.wrapping_add(4);
        }
    }
}

/// UpdateAllEnemies+UpdateEnemy（**asm 内容锚** `UpdateAllEnemies: ; 07:F345`/
/// `UpdateEnemy: ; 07:F351`——r26 端口：槽环 $50→$00×16B+状态 dispatch（7 态））：
/// ①PageIndex←x ②EnsExtra.status：Resting/Active⇒CheckIfVisible ③UpdateEnData05Bit6
/// ④JumpEngine：0=ExitSub/1=Resting/2=Active/3=Explode/4=Frozen/5=Pickup/6=Hurt/
/// ≥7⇒RemoveEnemy。件径=敌域 L2 单位（engine::update_enemy_*）会师面。
pub fn update_all_enemies(mem: &mut crate::frame::FrameMemory) {
    let mut x: u8 = 0x50; // _sizeof_Ens-_sizeof_Ens.0（5 槽×16）
    loop {
        mem.write_ram(0x004B, x); // PageIndex
        let status = mem.read_wram(0x6AF4 + x as u16); // EnsExtra.0.status
        // CheckIfVisible（$F37F——不可见⇒双返跳过该敌）
        if status != 0 && status < 2 && !enemy_check_if_visible(mem, x) {
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
fn enemy_check_if_visible(mem: &mut crate::frame::FrameMemory, x: u8) -> bool {
    if mem.read_ram(0x0400 + x as u16 + 5) & 0x02 != 0 {
        return true; // bit1=1⇒跳过检查（视为可见）
    }
    let y = mem.read_ram(0x0400 + x as u16 + 0); // Ens.y
    let hi = mem.read_wram(0x6AF4 + x as u16 + 7); // EnsExtra.hi
    let scroll_y = mem.read_ram(crate::ram_map::SCROLL_Y);
    let horz = mem.read_ram(crate::ram_map::SCROLL_DIR) & 0x02 != 0;
    let sy = if horz { y } else { y.wrapping_sub(scroll_y) };
    // IsObjectVisible（$DFDF——$F0 范围+hi 位 vs PPUCTRL 名表）
    let ok = sy.wrapping_sub(scroll_y) < 0xF0
        || (hi & 1 == mem.read_ram(0x00FF) & 1 && sy < 0xF0);
    ok || sy < 0xF0 && hi & 1 == mem.read_ram(0x00FF) & 1
}

/// UpdateEnemy_UpdateEnData05Bit6（**asm 内容锚** `: 07:F3AA`——r26）：
/// data05 bit6←(slot÷16 xor FrameCount) bit0（**30fps 动画标**——D4 保真）。
fn update_en_data05_bit6(mem: &mut crate::frame::FrameMemory, x: u8) {
    let d = mem.read_ram(0x0400 + x as u16 + 5);
    let bit6 = (d >> 6) & 1;
    let parity = ((x >> 4) ^ mem.read_ram(crate::ram_map::FRAME_COUNT)) & 1;
    let nd = (d & 0xBF) | (parity << 6);
    let _ = bit6;
    mem.write_ram(0x0400 + x as u16 + 5, nd);
}


/// UpdateEnemy_ForceSpeedTowardsSamus（**asm 内容锚** `: 07:F6B9`——r26 主径）：
/// Enemy82($82)←0→L968B[type]（ENEMY_FORCE_ATTRIBS）→Y→Active 且 bit1=0⇒RTS→
/// dec Ens.data0D 非零⇒RTS→归零：delay 重载+bit7=1⇒x 轴力/0⇒skipXAxis（Zoomer 径）。
fn enemy_force_speed(mem: &mut crate::frame::FrameMemory, x: u8) {
    mem.write_ram(0x0082, 0);
    let t = mem.read_wram(0x6AF4 + x as u16 + 14) as usize & 0x0F;
    let attribs = game_data::area::graphics::ENEMY_FORCE_ATTRIBS[t];
    let status = mem.read_wram(0x6AF4 + x as u16);
    if status == 2 && attribs & 0x02 == 0 {
        return;
    }
    let d0d = mem.read_ram(0x0400 + x as u16 + 13);
    if d0d.wrapping_sub(1) != 0 {
        mem.write_ram(0x0400 + x as u16 + 13, d0d.wrapping_sub(1));
        return;
    }
    let delay = game_data::area::graphics::ENEMY_FORCE_SPEED_DELAY.get(t).copied().unwrap_or(1);
    mem.write_ram(0x0400 + x as u16 + 13, delay);
    if attribs & 0x80 != 0 {
        // 【占位臂记账】x 轴力（bit7=1 型）F6DF 深件（敌移动力族——sc-enemy-encounter
        // 包随件）：记账+跳过（x 力不施）——可玩性优先（占位臂模式）。
        crate::frame::DIVERGENCES.with(|d| d.borrow_mut().push(0xF6DF));
        return;
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
                let s = mem.read_ram(0x0400 + x as u16 + 8); // speedSubPixelY=+8
                mem.write_ram(0x0400 + x as u16 + 8, s.wrapping_neg());
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
fn enemy_distance_threshold(mem: &mut crate::frame::FrameMemory, x: u8) {
    let e = 0x0400 + x as u16 + 5; // Ens.data05
    let d = mem.read_ram(e);
    mem.write_ram(e, d | 0x18); // OrEnData05($18)
    let t = mem.read_wram(0x6AF4 + x as u16 + 14) as usize & 0x0F;
    let thr = game_data::area::graphics::ENEMY_DISTANCE_THRESHOLD[t];
    if thr == 0 {
        return; // 阈 0⇒早出（Zoomer 径！）
    }
    let d = mem.read_ram(e);
    if d & 0x02 == 0 {
        // 不可见⇒清 bits3/4（Lx345）
        mem.write_ram(e, d & !0x18);
        return;
    }
    let (axis_bit, mut mask) = if thr & 0x80 != 0 { (0x08u8, !0x08u8) } else { (0x10, !0x10) };
    let _ = axis_bit;
    let thr_v = (thr & 0x7F) as u16;
    // 位置比较（hi 位旋入 bit7 形）
    let d = mem.read_ram(e);
    let (sam_pos, en_pos) = if d & 0x80 != 0 {
        (mem.read_ram(0x030D), mem.read_ram(0x0400 + x as u16 + 0)) // y 轴
    } else {
        (mem.read_ram(0x030E), mem.read_ram(0x0400 + x as u16 + 1)) // x 轴
    };
    let sam_hi = mem.read_wram(0x6AF4) & 1; // Samus.hi
    let en_hi = mem.read_wram(0x6AF4 + x as u16 + 7) & 1; // EnsExtra.hi
    let sam = ((sam_hi as u16) << 7) | sam_pos as u16; // ror 形（hi→bit7）
    let en = ((en_hi as u16) << 7) | en_pos as u16;
    let dist = (en as i16 - sam as i16).unsigned_abs() >> 4; // 二补+>>4（/16）
    if dist >= thr_v {
        mem.write_ram(e, d & mask); // 距大⇒清对应位
    }
}


/// InitEnRestingAnimIndex（**asm 内容锚** `: 07:F682`——r26）：type×2+facing 索引→
/// ENEMY_RESTING_ANIM[y]；≠resetAnimIndex⇒写 resetAnimIndex/animIndex+animDelay←0。
fn init_en_resting_anim_index(mem: &mut crate::frame::FrameMemory, x: u8) {
    let y = enemy_type_times2_plus_facing(mem, x);
    let idx = game_data::area::graphics::ENEMY_RESTING_ANIM[y as usize & 0x1F];
    if idx == mem.read_wram(0x6AF4 + x as u16 + 5) {
        return; // 已同⇒RTS（animDelay 不清）
    }
    mem.write_wram(0x6AF4 + x as u16 + 5, idx); // resetAnimIndex
    mem.write_wram(0x6AF4 + x as u16 + 6, idx); // animIndex
    mem.write_wram(0x6AF4 + x as u16 + 4, 0); // animDelay←0
}

/// GetEnemyTypeTimes2PlusFacingDirection（**asm 内容锚**——索引=type×2+facing（data05 bit0?））。
fn enemy_type_times2_plus_facing(mem: &mut crate::frame::FrameMemory, x: u8) -> u16 {
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
fn load_table_at_977b(mem: &mut crate::frame::FrameMemory, x: u8) -> u8 {
    let t = mem.read_wram(0x6AF4 + x as u16 + 14) as usize;
    let e = game_data::area::graphics::ENEMY_FACING[t & 0x3F];
    e.wrapping_shl(1)
}


/// 任务卡 2：SamusProjectileHitDoorOrStatue（**asm 内容锚** `: 07:E81E`（LE7E6 弹径
/// （$E7E6（:8086））——findings/door-system-audit.md 任务卡 2 实录（A 级全文）））：
/// 弹径门（$71）→门环（X=6,-2（DoorRoomRAMPtr $5C-$63 列匹配（&$04 且 &$1F））→
/// 槽选 Y=X*8|$80→类型门（偶（红/导弹）=仅 wa_Missile($0B) 或（wa_BulletExplode($04)
/// 且 animReset==$91（表算径））（否则 GotoSFX_Metal 弹开（不写））/奇（蓝/音乐蓝）=任意）
/// →isHit[$30A+y]←$04（bit2）+导弹命中 SFXTriInitFlags($0683)|=2；环尽⇒雕像兜底
/// （KraidStatueIsHit[$366+(Temp04_lo>>3&1)] inc（8 位环绕））。
pub fn samus_projectile_hit_door_or_statue(
    mem: &mut crate::frame::FrameMemory,
    temp04_lo: u8,
    temp04_hi: u8,
) {
    // 弹径门（UpdatingSamusProjectile（$71（constants:600））——非弹更新径⇒即退）
    if mem.read_ram(0x0071) == 0 {
        return; // ClcExit
    }
    let mut x: i16 = 6;
    while x >= 0 {
        let xb = x as u8;
        let ptr_hi = mem.read_ram(0x005D + xb as u16);
        let ptr_lo = mem.read_ram(0x005C + xb as u16);
        if ((temp04_hi ^ ptr_hi) & 0x04) == 0 && ((temp04_lo ^ ptr_lo) & 0x1F) == 0 {
            let y = (xb.wrapping_mul(8)) | 0x80; // Amul8|80（槽偏移（$0300 基））
            if mem.read_ram(0x0300 + y as u16) != 0 {
                // 门存在——类型门（DoorType=$307+y（lsr/奇=蓝））
                let door_type = mem.read_ram(0x0307 + y as u16);
                let pi = mem.read_ram(0x004B) as u16; // PageIndex（弹槽偏移）
                let pstat = mem.read_ram(0x0300 + pi);
                let hit = if door_type & 1 != 0 {
                    true // 蓝/音乐蓝：任意弹伤
                } else {
                    // 红/导弹：仅 wa_Missile($0B) 或（wa_BulletExplode($04) 且
                    // animResetIndex==$91（ObjAnim_MissileExplode（$8603-$8572（表算））））
                    pstat == 0x0B
                        || (pstat == 0x04 && mem.read_ram(0x0305 + pi) == 0x91)
                };
                if hit {
                    if door_type & 1 == 0 {
                        // @hitByMissile：SFXTriInitFlags($0683)|=sfxTri_SamusBall(2)
                        // 【写径面校】SFXTriInitFlags($0683)=2KB RAM 镜像（非 wram）
                        let f = mem.read_ram(0x0683);
                        mem.write_ram(0x0683, f | 0x02);
                    }
                    // @blueDoor：isHit[$30A+y]←$04（bit2=被弹击（constants:919））
                    mem.write_ram(0x030A + y as u16, 0x04);
                    return; // bne ClcExit
                }
                return; // GotoSFX_Metal（弹开（SFX 面（随件（记账）））——不写 isHit
            }
        }
        x -= 2; // @next：dex/dex/bpl @loop
    }
    // 雕像兜底（环尽无门）：KraidStatueIsHit[$366+idx] inc（idx=Temp04_lo Adiv8 &1）
    let idx = ((temp04_lo >> 3) & 1) as u16;
    let v = mem.read_ram(0x0366 + idx);
    mem.write_ram(0x0366 + idx, v.wrapping_add(1));
}

/// UpdateEnemy_Resting_UpdateEnData1F（**asm 内容锚** `: 07:F676`——r26）：
/// L977B 条目×2→再 asl×3→and #$C0→data1F（**bit2-3→bit6-7**）。
fn update_en_data1f(mem: &mut crate::frame::FrameMemory, x: u8) {
    let v = load_table_at_977b(mem, x);
    let v = v.wrapping_shl(3); // asl×3（含载器 ×2=条目×16）
    mem.write_wram(0x6AF4 + x as u16 + 15, v & 0xC0); // EnsExtra.data1F
}

/// UpdateEnemy_Resting_TryBecomingActive（**asm 内容锚** `: 07:F7BA`——r26）：
/// dec delay 非零⇒RTS；归零：data05 bit3=0⇒inc 退回；=1⇒(PipeBug type7⇒SFX)+
/// status inc→Active+InitEnActiveAnimIndex。
fn try_becoming_active(mem: &mut crate::frame::FrameMemory, x: u8) {
    // 【goal-r16 修】delay 偏移 +7→+9（ROM F7BA `DE 09 04`=dec $0409——Ens.delay=+9；
    // 旧 +7=movementInstrIndex⇒delay 永不归零⇒敌人恒 Resting 不激活）。data0A(+10)=朝向。
    // 【2026-10-31 原地写修】asm `dec Ens.0.delay`（F7BA `DE 09 04`）=**原地写**：
    // dec 结果先落内存（激活径 delay=0——emu f798 实录 delay=00；旧形激活径不写回⇒
    // delay 恒 1 残留）；bit3=0 径 `inc` 退回=写回原值。
    let e = 0x0400 + x as u16 + 9; // Ens.delay（ROM F7BA 实测 $0409）
    let d = mem.read_ram(e);
    mem.write_ram(e, d.wrapping_sub(1)); // dec 原地（asm DE 09 04）
    if d.wrapping_sub(1) != 0 {
        return;
    }
    let d5 = mem.read_ram(0x0400 + x as u16 + 5);
    if d5 & 0x08 == 0 {
        mem.write_ram(e, d); // inc 退回（asm F7C6 `EE 09 04`——undo）
        return;
    }
    // @becomeActive
    let t = mem.read_wram(0x6AF4 + x as u16 + 14);
    if t == 7 {
        crate::unported!("bank07:CBA4"); // SFX_OutOfPipe（声事件——出声面）
    }
    #[allow(unreachable_code)]
    {
        let st = mem.read_wram(0x6AF4 + x as u16);
        mem.write_wram(0x6AF4 + x as u16, st.wrapping_add(1)); // status→Active
        init_en_active_anim_index(mem, x); // F699（Active 动画初始化）
    }
}

/// InitEnActiveAnimIndex（**asm 内容锚** `: 07:F699`——r26）：ENEMY_ACTIVE_ANIM[y]
/// →resetAnimIndex/animIndex；InitOffset 非零⇒animIndex 递减该值。
fn init_en_active_anim_index(mem: &mut crate::frame::FrameMemory, x: u8) {
    let y = enemy_type_times2_plus_facing(mem, x);
    let idx = game_data::area::graphics::ENEMY_ACTIVE_ANIM[y as usize & 0x1F];
    if idx != mem.read_wram(0x6AF4 + x as u16 + 5) {
        mem.write_wram(0x6AF4 + x as u16 + 5, idx);
        mem.write_wram(0x6AF4 + x as u16 + 6, idx);
        mem.write_wram(0x6AF4 + x as u16 + 4, 0);
        let t = mem.read_wram(0x6AF4 + x as u16 + 14) as usize & 0x0F;
        let off = game_data::area::graphics::ENEMY_ACTIVE_ANIM_INIT_OFFSET[t] & 0x7F;
        let mut ai = idx;
        for _ in 0..off {
            ai = ai.wrapping_sub(1);
        }
        mem.write_wram(0x6AF4 + x as u16 + 6, ai);
    }
}


/// EnemyReactToSamusProjectile（**asm 内容锚** `: 07:F536`——r26）：isHit(+4) bit5=0⇒
/// RTS（未中弹径——移动对拍主径）；中弹径=冰冻/伤害族（打敌人流程件——随件）。
fn enemy_react_to_projectile(mem: &mut crate::frame::FrameMemory, x: u8) {
    // 【实录（asm $F536 全径（findings §11）+探针偏移（+0B=health/+0D=data0D/
    // +0E=weaponAction/+0F=specialAttribs（ens_probe trace（F863/F437/F41B 锚）））】
    let px = x as u16;
    let e4 = 0x0400 + px + 4; // isHit
    if mem.read_ram(e4) & 0x20 == 0 {
        return; // 非被弹⇒RTS
    }
    let sa = mem.read_ram(0x0400 + px + 0x0F); // specialAttribs（Lx321 prevStatus 记账）
    let psc = mem.read_ram(0x0400 + px + 0x0C); // prevStatus
    mem.write_ram(0x0400 + px + 0x0C, psc | sa); // `ora Temp0A/sta prevStatus`（Lx321）
    let wa = mem.read_ram(0x0400 + px + 0x0E); // weaponAction
    let miniboss = sa & 0x40 != 0;
    // 冰冻臂（IceBeam=wa 2（audit 形）+非 miniboss+非冻）——status 4=冻结
    let st = mem.read_wram(0x6AF4 + px); // EnsExtra.status
    if wa == 2 && !miniboss && st != 4 {
        mem.write_wram(0x6AF4 + px, 4); // status←Frozen
        mem.write_ram(0x0400 + px + 0x0D, 0x40); // data0D←$40（冻计时）
        mem.write_ram(e4, 0); // isHit 清
        return;
    }
    // Lx317：HP==$FF⇒无效（SFX（声=桩）+清 isHit/weaponAction）
    let hp = mem.read_ram(0x0400 + px + 0x0B); // health（trace +0B）
    if hp == 0xFF {
        mem.write_ram(e4, 0);
        mem.write_ram(0x0400 + px + 0x0E, 0);
        return;
    }
    // 弹（wa_Missile）/螺（$81）⇒瞬杀（非 miniboss）
    if !miniboss && (wa == 0x0B || wa == 0x81) {
        mem.write_ram(e4, 0);
        explode_enemy(mem, x);
        return;
    }
    // Lx322：status←Hurt(6)+specialAttribs←$0A/$03
    mem.write_wram(0x6AF4 + px, 6);
    mem.write_ram(0x0400 + px + 0x0F, if miniboss { 0x03 } else { 0x0A });
    // HP dec（波=2/束=1（Lx324/325））⇒0⇒爆
    let dec = if wa == 4 { 2u8 } else { 1u8 }; // wa_WaveBeam=4（audit 形）
    let nhp = hp.wrapping_sub(dec);
    mem.write_ram(0x0400 + px + 0x0B, nhp);
    mem.write_ram(e4, 0);
    if nhp == 0 {
        explode_enemy(mem, x);
    }
}

/// ExplodeEnemy（**asm 内容锚** `: 07:F611`）：status←3+EnemyDeathAnimIndex[type×2+向]
/// →InitEnAnimIndex+chunkyExplosionTimer←A（+06）——EnExplosions 池=小爆效果件
/// （分卡（记账））。
fn explode_enemy(mem: &mut crate::frame::FrameMemory, x: u8) {
    let px = x as u16;
    mem.write_wram(0x6AF4 + px, 3); // status←Explode
    let ty = mem.read_wram(0x6AF4 + px + 14) as usize & 0x1F;
    let d05 = mem.read_ram(0x0400 + px + 5);
    let facing: usize = if d05 & 0x80 != 0 { ((d05 >> 3) & 1) as usize } else { (d05 & 1) as usize };
    let idx = ((ty << 1) | facing) & 0x1F;
    const ENEMY_DEATH_ANIM: [u8; 32] = [
        // EnemyDeathAnimIndex（**ROM 字节锚** bank1:$960B（type×2+向（32B）））
        0x27,0x27,0x29,0x29,0x2D,0x2B,0x31,0x2F,0x33,0x33,0x41,0x41,0x4B,0x4B,0x55,0x53,
        0x72,0x74,0x00,0x00,0x00,0x00,0x69,0x69,0x69,0x69,0x00,0x00,0x00,0x00,0x00,0x00,
    ];
    let anim = ENEMY_DEATH_ANIM[idx];
    mem.write_wram(0x6AF4 + px + 5, anim); // resetAnimIndex
    mem.write_wram(0x6AF4 + px + 6, anim); // animIndex
    mem.write_wram(0x6AF4 + px + 4, 0); // animDelay
    mem.write_ram(0x0400 + px + 6, anim); // chunkyExplosionTimer←A（asm `sta`（+06））
}

/// UpdateEnemy_Explode（**asm 内容锚** `: 07:F40D`（jmp ChooseEnemyAIRoutine（爆在 AI
/// Exit 臂（noMoveNoAnim 显示）+chunkyExplosionTimer（+06）计时⇒CompleteDeath））。
fn update_enemy_explode(mem: &mut crate::frame::FrameMemory, x: u8) {
    let px = x as u16;
    update_enemy_anim(mem, x, 3);
    update_enemy_common_no_move_no_anim(mem, x);
    let timer = mem.read_ram(0x0400 + px + 6); // chunkyExplosionTimer（trace +06）
    if timer > 0 {
        mem.write_ram(0x0400 + px + 6, timer - 1);
    } else {
        enemy_complete_death(mem, x);
    }
}

/// UpdateEnemy_Frozen（$F54D）：data0D 计时（×8（asm「frozen timer (* 8)」））⇒回 Active。
fn update_enemy_frozen(mem: &mut crate::frame::FrameMemory, x: u8) {
    let px = x as u16;
    update_enemy_common_no_move_no_anim(mem, x);
    let d = mem.read_ram(0x0400 + px + 0x0D);
    if d > 0 {
        mem.write_ram(0x0400 + px + 0x0D, d - 1);
    } else {
        mem.write_wram(0x6AF4 + px, 2); // 回 Active
    }
}

/// UpdateEnemy_Hurt（$F59D）：specialAttribs=hitstun 延（asm「hitstun delay」）⇒回 Active。
fn update_enemy_hurt(mem: &mut crate::frame::FrameMemory, x: u8) {
    let px = x as u16;
    update_enemy_common_no_move_no_anim(mem, x);
    let d = mem.read_ram(0x0400 + px + 0x0F);
    if d > 0 {
        mem.write_ram(0x0400 + px + 0x0F, d - 1);
    } else {
        mem.write_wram(0x6AF4 + px, 2); // 回 Active
    }
}

/// UpdateEnemy_Pickup（**asm 内容锚** `: 07:F483`）：碰触（isHit&$24）⇒RemoveEnemy+
/// 回血/弹（+30/+20/+小（animFrame 0x80=弹药））；data0D 计时（*4）⇒到期移除。
fn update_enemy_pickup(mem: &mut crate::frame::FrameMemory, x: u8) {
    let px = x as u16;
    // 碰触判**先于**显面（静态面尾清 isHit（+4←0）——序正（asm F483 先判后显））
    let hit = mem.read_ram(0x0400 + px + 4);
    if hit & 0x24 != 0 {
        // 碰触⇒回血/弹+移除
        let frame = mem.read_wram(0x6AF4 + px + 3);
        if frame == 0x80 {
            let m = mem.read_wram(crate::ram_map::MISSILE_COUNT);
            mem.write_wram(crate::ram_map::MISSILE_COUNT, m.saturating_add(5));
        } else if frame == 0x89 {
            let h = mem.read_ram(0x0106u16);
            mem.write_ram(0x0106u16, h.saturating_add(30));
        } else {
            let h = mem.read_ram(0x0106u16);
            mem.write_ram(0x0106u16, h.saturating_add(20));
        }
        mem.write_wram(0x6AF4 + px, 0); // RemoveEnemy（槽释）
        return;
    }
    update_enemy_common_no_move_no_anim(mem, x);
    let d = mem.read_ram(0x0400 + px + 0x0D);
    if d > 0 {
        mem.write_ram(0x0400 + px + 0x0D, d - 1);
    } else {
        mem.write_wram(0x6AF4 + px, 0); // 到期移除
    }
}

/// EnemyCompleteDeath（**asm 内容锚** `: 07:DCF5`）：status←Pickup+data0D←$60+
/// 1/16 无掉+PickupTypeTable[rand&7]→animFrame（表 ROM $DE35=80 81 89）——qty 限
/// 域=分卡（记账）。
fn enemy_complete_death(mem: &mut crate::frame::FrameMemory, x: u8) {
    let px = x as u16;
    mem.write_wram(0x6AF4 + px, 5); // status←Pickup
    mem.write_ram(0x0400 + px + 0x0D, 0x60); // data0D←$60（拾取计时）
    let r = mem.read_ram(crate::ram_map::RANDOM1);
    if r < 0x10 {
        // 1/16 无掉⇒移除（LDD5B 非 Tourian）
        mem.write_wram(0x6AF4 + px, 0);
        return;
    }
    let tbl = [0x80u8, 0x81, 0x89, 0x80, 0x81, 0x89, 0x81, 0x89]; // PickupTypeTable（ROM $DE35）
    let frame = tbl[(r & 7) as usize];
    mem.write_wram(0x6AF4 + px + 3, frame); // animFrame←掉落型
}

/// ChooseEnemyAIRoutine（**asm 内容锚** 区 bank `$95E5`（prg1:156——type 0-0F dispatch）：
/// **05=Zoomer(Crawler——起始房敌！)**。未端口型=【占位臂记账】（DIVERGENCES 非 panic——
/// title_bridge 同款模式）：静态显示径（update_enemy_common_no_move_no_anim）保可玩，
/// AI 深件随 sc-enemy-encounter 包收敛（goal 流程③）。
fn choose_enemy_ai(mem: &mut crate::frame::FrameMemory, x: u8) {
    let t = mem.read_wram(0x6AF4 + x as u16 + 14); // type=+14
    match t {
        5 => crawler_ai(mem, x),
        _ => {
            crate::frame::DIVERGENCES.with(|d| d.borrow_mut().push(0x95E5));
            update_enemy_common_no_move_no_anim(mem, x);
        }
    }
}

/// CrawlerAIRoutine（**asm 内容锚** `enemies/crawler.asm`——r26+2026-10-31 PreAI 门补装）：
/// ShouldCrawlerMove（F852——((x>>3+FC+lsr 进位))>>1&3=6/8 帧步门）；不动帧⇒@exit
/// （anim3+显示）；动帧⇒**EnemyStatusPreAI 三连门**（Resting⇒SkreeExit_Resting=anim8+
/// 显示（bank1 共享 skree 出口）/PreAI Explode⇒SkreeExit_Explode（仅显示）/
/// 现役 Explode⇒@exit）→@move。缺门症（已修）：Resting 帧即跑 AI——激活帧半径未写
/// （draw 同 tick 后行）⇒rx=0 全检快径⇒外角下探伪成功转角⇒菱形回路（crawlprobe f797）。
fn crawler_ai(mem: &mut crate::frame::FrameMemory, x: u8) {
    // F852：txa/lsr×3（carry←原 bit2）/adc FrameCount（含进位）/lsr——门含 lsr 移出位。
    let gate = ((x >> 3)
        .wrapping_add((x >> 2) & 1)
        .wrapping_add(mem.read_ram(crate::ram_map::FRAME_COUNT))
        >> 1)
        & 0x03;
    if gate == 0 {
        update_enemy_anim(mem, x, 3); // @exit `lda #3/jsr UpdateEnemyAnim`
        update_enemy_common_no_move_no_anim(mem, x);
        return;
    }
    // EnemyStatusPreAI 三连门（crawler.asm:8-22 逐条——激活帧拦 Resting=本修核心）。
    let pre_ai = mem.read_ram(0x0081);
    if pre_ai == 1 {
        // SkreeExit_Resting（bank1/4 共享——skree.asm:62 `lda #8/noMove`）
        update_enemy_anim(mem, x, 8);
        update_enemy_common_no_move_no_anim(mem, x);
        return;
    }
    if pre_ai == 3 {
        // SkreeExit_Explode（skree.asm:68——`jmp noMoveNoAnim` 无 anim）
        update_enemy_common_no_move_no_anim(mem, x);
        return;
    }
    if mem.read_wram(0x6AF4 + x as u16) == 3 {
        // 现役 Explode⇒@exit（crawler.asm:20-22——anim3 落穿 noMoveNoAnim）
        update_enemy_anim(mem, x, 3);
        update_enemy_common_no_move_no_anim(mem, x);
        return;
    }
    // @move 径：JumpByRTSToMovementRoutine（areas_common \$8048 表 RTS 跳）
    // Y=(orientation*2+direction)*2→CrawlerMovementRoutinesTable→四例行。
    let d5 = mem.read_ram(0x0400 + x as u16 + 5);
    let orientation = mem.read_ram(0x0400 + x as u16 + 10) & 0x03; // data0A=+10
    let direction = d5 & 1;
    let idx = ((orientation as usize) * 2 + direction as usize) & 0x07;
    let ok = match idx {
        0 | 5 => enemy_move_one_pixel_right(mem, x),
        1 | 4 => enemy_move_one_pixel_left(mem, x),
        2 | 3 => enemy_move_one_pixel_down(mem, x),
        _ => enemy_move_one_pixel_up(mem, x),
    };
    if !ok {
        // CrawlerInsideCornerCheck（bank1:$9AA8——**goal-r16 G3'**）：$00 契约判据。
        // $00=1(离屏/wrap)⇒FlipDirection(data05 eor #1)；$00=0(块界碰撞)⇒data0A=(data0A−1)&3。
        // asm：move($9AE2)后 **InsideCorner($9AA8 失败径)+OutsideCorner($9AC6 探径)都跑**（非互斥）。
        let zero = mem.read_ram(0x00);
        if zero != 0 {
            let d = mem.read_ram(0x0400 + x as u16 + 5);
            mem.write_ram(0x0400 + x as u16 + 5, d ^ 0x01); // FlipDirection（离屏径）
        } else {
            let o = mem.read_ram(0x0400 + x as u16 + 10); // data0A
            mem.write_ram(0x0400 + x as u16 + 10, o.wrapping_sub(1) & 0x03); // 朝向--
            crawler_reorient_sprite(mem, x); // $9A8F（ InsideCorner 碰撞径 `4C 8F 9A`）
        }
    }
    // CrawlerOutsideCornerCheck（bank1:$9AC6——**goal-r16 G3'**）：恒跑，向 (data0A+1) 探移
    // （GetNextOrientation $9ADA=iny）成功⇒data0A+1+reorient（asm `jsr $9AE2` 探移**提交**无回退）。
    {
        let o = mem.read_ram(0x0400 + x as u16 + 10) & 0x03;
        let next_o = (o + 1) & 0x03;
        let dir = mem.read_ram(0x0400 + x as u16 + 5) & 1;
        let nidx = ((next_o as usize) * 2 + dir as usize) & 0x07;
        let probe_ok = match nidx {
            0 | 5 => enemy_move_one_pixel_right(mem, x),
            1 | 4 => enemy_move_one_pixel_left(mem, x),
            2 | 3 => enemy_move_one_pixel_down(mem, x),
            _ => enemy_move_one_pixel_up(mem, x),
        };
        if probe_ok {
            mem.write_ram(0x0400 + x as u16 + 10, next_o); // data0A+1（asm `9D 0A 04`）
            crawler_reorient_sprite(mem, x); // $9A8F（OutsideCorner 径 `20 8F 9A`）
        }
    }
    update_enemy_anim(mem, x, 3); // @exit `A9 03/20 0C 80`（cadence=3）
    update_enemy_common_no_move_no_anim(mem, x); // @exit 径 `4C 06 80`（anim+显示）
}

/// UpdateEnemyCommon@noMoveNoAnim（**asm 内容锚** `: 07:F416`——r26）：tough 调色→
/// status≠0⇒DrawEnemy→isHit/weaponAction 清。
fn update_enemy_common_no_move_no_anim(mem: &mut crate::frame::FrameMemory, x: u8) {
    init_en_resting_anim_index(mem, x); // F682 resting anim 应用（F3BE 链件（缺面修））
    let special = mem.read_ram(0x0400 + x as u16 + 15); // specialAttribs=+15
    if special & 0x80 != 0 {
        let oc = mem.read_ram(0x006B);
        if oc & 0x80 == 0 {
            mem.write_ram(0x006B, 0x80 | 0x03 | 0x20);
        }
    }
    if mem.read_wram(0x6AF4 + x as u16) != 0 {
        draw_enemy(mem, x);
    }
    mem.write_ram(0x0400 + x as u16 + 4, 0); // isHit=+4
    mem.write_ram(0x0400 + x as u16 + 14, 0); // weaponAction=+14
}

/// CrawlerReorientSprite（**ROM 字节锚** bank1:$9A8F——m1disasm 未标注区（D5 四方交叉：
/// ROM 直读 + 三层闭环：$9AA0→EnAnimTable→EnFramePtrTable1→emu OAM 实测一致））：
/// `lda data05/lsr`（flip=bit0 进 carry）`lda data0A/and #3/rol A`（idx=orient*2+flip）
/// `lda $9AA0,y / jmp InitEnAnimIndex`（F68D：reset/index←A、animDelay←0）。
/// 旧形「随件占位」⇒ 敌 animFrame 恒 0 ⇒ 画错帧（emu tile CA/CB/DA/DB vs rust 14）。
fn crawler_reorient_sprite(mem: &mut crate::frame::FrameMemory, x: u8) {
    const CRAWLER_SPRITE_ANIM_TBL: [u8; 8] =
        [0x35, 0x35, 0x3E, 0x38, 0x3B, 0x3B, 0x38, 0x3E]; // ROM $9AA0（orient*2+flip）
    let d05 = mem.read_ram(0x0400 + x as u16 + 5);
    let flip = d05 & 0x01; // lsr→carry=bit0
    let orient = (mem.read_ram(0x0400 + x as u16 + 10) & 0x03) as usize;
    let idx = ((orient << 1) | flip as usize) & 0x07;
    let v = CRAWLER_SPRITE_ANIM_TBL[idx];
    // InitEnAnimIndex(F68D)
    mem.write_wram(0x6AF4 + x as u16 + 5, v); // resetAnimIndex
    mem.write_wram(0x6AF4 + x as u16 + 6, v); // animIndex
    mem.write_wram(0x6AF4 + x as u16 + 4, 0); // animDelay←0
}

/// UpdateEnemyAnim（**asm 内容锚** `UpdateEnemyAnim: ; 07:E094`——敌人动画走表全径）：
/// status==Pickup(5)⇒RTS；delay 门（≠0 递减未归零⇒RTS）→`sta animDelay←A`→走
/// EN_ANIM_TABLE[animIndex]（区表——Brinstar $9D6A）：FF⇒animIndex←reset 续走；
/// 否则 animFrame(+3)←表值、animIndex+1。调用形：CrawlerAI @exit `lda #3/jsr $800C`。
fn update_enemy_anim(mem: &mut crate::frame::FrameMemory, x: u8, a: u8) {
    let base = 0x6AF4u16 + x as u16;
    if mem.read_wram(base) == 5 {
        return; // Pickup 态不更
    }
    let delay = mem.read_wram(base + 4);
    if delay != 0 {
        let d = delay - 1;
        mem.write_wram(base + 4, d);
        if d != 0 {
            return;
        }
    }
    mem.write_wram(base + 4, a); // sta animDelay←A
    let reset = mem.read_wram(base + 5);
    let mut y = mem.read_wram(base + 6) as usize;
    for _ in 0..16u32 {
        let v = *game_data::area::enemy_gfx::EN_ANIM_TABLE.get(y).unwrap_or(&0xFF);
        if v == 0xFF {
            y = reset as usize; // 回绕续走
            continue;
        }
        mem.write_wram(base + 3, v); // animFrame←表值
        mem.write_wram(base + 6, y.wrapping_add(1) as u8);
        return;
    }
    crate::unported!("bank07:E0BC-wrap"); // 表不变量破裂——fail-loud（update_obj_anim 同式）
}

/// DrawEnemy（**asm 内容锚** `DrawEnemy: ; 07:DD8B`——r26 端口）：animFrame==FrameNone⇒
/// ClearObjectCntrl；帧指针（EnFramePtrTable1/2——animFrame bit7 carry 选表！）→
/// GetSpriteCntrlData→位指针（EnPlacePtrTable）→sprite 管线写（place_disp/四元组）。
fn draw_enemy(mem: &mut crate::frame::FrameMemory, x: u8) {
    use game_data::area::enemy_gfx as eg;
    // 【②回装（2026-10-29）】asm :6156-6168：帧记字 1/2→EnsExtra.radiusY/X（IsObject-
    // Visible 用半径（序=半径写先行））+[:6178-6186] bit1 逐帧闩（and #$FD/ora（txa<<1））。
    // 爆亡链件已补（F40D/DCF5/F483 全径（415 测））⇒连锁闭环（§10 回装判据达成）。
    let frame0 = mem.read_wram(0x6AF4 + x as u16 + 3);
    if frame0 != 0xFF {
        let fi0 = ((frame0 & 0x7F) as usize) + if frame0 & 0x80 != 0 { 128 } else { 0 };
        let (fo0, fl0) = eg::EN_FRAME_OFFS[fi0];
        if fl0 > 2 {
            mem.write_wram(0x6AF4 + x as u16 + 1, eg::EN_FRAMES[fo0 as usize + 1]);
            mem.write_wram(0x6AF4 + x as u16 + 2, eg::EN_FRAMES[fo0 as usize + 2]);
        }
    }
    let visible = enemy_check_if_visible(mem, x);
    let e5 = 0x0400 + x as u16 + 5;
    let d5 = mem.read_ram(e5);
    mem.write_ram(e5, (d5 & 0xFD) | ((visible as u8) << 1)); // bit1 闩
    if !visible {
        mem.write_ram(0x006B, 0);
        return;
    }
    let frame = mem.read_wram(0x6AF4 + x as u16 + 3); // animFrame=+3
    if frame == 0xFF {
        mem.write_ram(0x006B, 0);
        return;
    }
    let table2 = frame & 0x80 != 0;
    let fi = ((frame & 0x7F) as usize) + if table2 { 128 } else { 0 };
    let (fo, fl) = eg::EN_FRAME_OFFS[fi];
    let blob = |i: usize| -> u8 {
        if fl == 0 || i >= fl as usize { 0 } else { eg::EN_FRAMES[fo as usize + i] }
    };
    let ctrl = blob(0);
    mem.write_ram(0x006B, ctrl);
    let place_idx = ((ctrl & 0x0F) as usize).min(127);
    let (po, pl) = eg::EN_PLACE_OFFS[place_idx];
    let pblob = |i: usize| -> u8 {
        if pl == 0 || i >= pl as usize { 0 } else { eg::EN_PLACES[po as usize + i] }
    };
    let y = mem.read_ram(0x0400 + x as u16 + 0); // y=+0
    let xpos = mem.read_ram(0x0400 + x as u16 + 1); // x=+1
    let scroll_y = mem.read_ram(crate::ram_map::SCROLL_Y);
    let horz = mem.read_ram(crate::ram_map::SCROLL_DIR) & 0x02 != 0;
    let sy = (if horz { y } else { y.wrapping_sub(scroll_y) }) as i16;
    // 【勘误修（2026-10-29（问题 A 根因））】敌屏位 x=房位−**ScrollX($FD)**——旧形减
    // $26（SpareMem26（constants:488（$25=MMC1CTRL_ZP 同族误读（sprite.rs 勘误注
    // 早已证）））——走滚径写 $FD（scroll_right）而 0x26 停旧值⇒敌屏位不减镜头⇒
    // **随镜恒距**（用户截图症（爬虫悬空错位））。
    let sx = xpos.wrapping_sub(mem.read_ram(crate::ram_map::SCROLL_X)) as i16;
    let mut cntrl05 = ((ctrl >> 4) & 0x03) | (ctrl & 0xC0) | 0x20;
    let mut place_i = 0usize;
    let mut frame_i = 3usize;
    let mut x_sprite = mem.read_ram(0x005B) as u16;
    loop {
        let b = blob(frame_i);
        if b < 0xFC {
            let dy = crate::frame::sprite::place_disp(pblob(place_i), ctrl, true);
            let dx = crate::frame::sprite::place_disp(pblob(place_i + 1), ctrl, false);
            let flip = mem.read_ram(0x006B) & 0xC0;
            let attr = cntrl05
                ^ (if flip & 0x40 != 0 { 0x80 } else { 0 })
                ^ (if flip & 0x80 != 0 { 0x40 } else { 0 });
            let oy = (sy + dy as i16 - 1) as u8;
            let ox = (sx + dx as i16) as u8;
            mem.write_ram(0x0200 + x_sprite, oy);
            mem.write_ram(0x0201 + x_sprite, b);
            mem.write_ram(0x0202 + x_sprite, attr);
            mem.write_ram(0x0203 + x_sprite, ox);
            x_sprite = x_sprite.wrapping_add(4);
            place_i += 2;
            frame_i += 1;
        } else if b == 0xFC {
            frame_i += 3;
        } else if b == 0xFD {
            frame_i += 1;
            cntrl05 = blob(frame_i);
            frame_i += 1;
        } else if b == 0xFE {
            place_i += 2;
            frame_i += 1;
        } else {
            break;
        }
    }
    mem.write_ram(0x005B, x_sprite as u8);
}


/// EnemyMoveOnePixelUp（**asm 内容锚** `areas_common.asm:989 $83F5`——r26）：
/// 顶界块界判（(y-radiusY)&7==0⇒EnemyCheckMoveUp）→y dec（y==0 换名表族）→
/// radiusY 界判→成功（carry=1）/失败（carry=0）。返回 bool（carry）。
fn enemy_move_one_pixel_up(mem: &mut crate::frame::FrameMemory, x: u8) -> bool {
    let y = mem.read_ram(0x0400 + x as u16 + 0); // y=+0
    let ry = mem.read_wram(0x6AF4 + x as u16 + 1); // radiusY=+1
    if y.wrapping_sub(ry) & 0x07 == 0 {
        // 块界⇒碰撞判（EnemyCheckMoveUp——随件 fail-loud）
        if !enemy_check_move_up(mem, x) {
            mem.write_ram(0x00, 0); // $00=0（碰撞）
            return false;
        }
    }
    let ny = y.wrapping_sub(1);
    mem.write_ram(0x0400 + x as u16 + 0, ny);
    if ny == ry {
        // 界判（ScrollY/名表换——简化径）
        mem.write_ram(0x00, 1); // $00=1（离屏/wrap）
        return false;
    }
    // 成功：jumpDsplcmnt inc（横朝向时）
    if mem.read_ram(0x0400 + x as u16 + 5) & 0x80 == 0 {
        let j = mem.read_wram(0x6AF4 + x as u16 + 13); // jumpDsplcmnt=+13
        mem.write_wram(0x6AF4 + x as u16 + 13, j.wrapping_add(1));
    }
    true
}

/// EnemyMoveOnePixelDown（**asm 内容锚** `: 1066 $844B`——r26 同构（y inc））。
fn enemy_move_one_pixel_down(mem: &mut crate::frame::FrameMemory, x: u8) -> bool {
    let y = mem.read_ram(0x0400 + x as u16 + 0);
    let ry = mem.read_wram(0x6AF4 + x as u16 + 1);
    if y.wrapping_add(ry) & 0x07 == 0 {
        if !enemy_check_move_down(mem, x) {
            mem.write_ram(0x00, 0); // $00=0（碰撞）
            return false;
        }
    }
    let ny = y.wrapping_add(1);
    mem.write_ram(0x0400 + x as u16 + 0, ny);
    if ny == ry.wrapping_neg() {
        mem.write_ram(0x00, 1); // $00=1（离屏/wrap）
        return false;
    }
    true
}

/// EnemyMoveOnePixelLeft（**asm 内容锚** `: 1142 $84A7`——r26（x dec 同构））。
fn enemy_move_one_pixel_left(mem: &mut crate::frame::FrameMemory, x: u8) -> bool {
    let xpos = mem.read_ram(0x0400 + x as u16 + 1); // x=+1
    let rx = mem.read_wram(0x6AF4 + x as u16 + 2); // radiusX=+2
    if xpos.wrapping_sub(rx) & 0x07 == 0 {
        if !enemy_check_move_left(mem, x) {
            mem.write_ram(0x00, 0); // $00=0（碰撞）
            return false;
        }
    }
    let nx = xpos.wrapping_sub(1);
    mem.write_ram(0x0400 + x as u16 + 1, nx);
    if nx == rx {
        mem.write_ram(0x00, 1); // $00=1（离屏/wrap）
        return false;
    }
    true
}

/// EnemyMoveOnePixelRight（**asm 内容锚** `: 1214 $84FE`——r26（x inc 同构））。
/// 【goal-r16 G3'】失败时写 $00 契约：块界碰撞⇒$00=0；离屏/名表 wrap⇒$00=1（corner check 判据）。
fn enemy_move_one_pixel_right(mem: &mut crate::frame::FrameMemory, x: u8) -> bool {
    let xpos = mem.read_ram(0x0400 + x as u16 + 1);
    let rx = mem.read_wram(0x6AF4 + x as u16 + 2);
    if xpos.wrapping_add(rx) & 0x07 == 0 {
        if !enemy_check_move_right(mem, x) {
            mem.write_ram(0x00, 0); // $00=0（碰撞）
            return false;
        }
    }
    let nx = xpos.wrapping_add(1);
    mem.write_ram(0x0400 + x as u16 + 1, nx);
    if nx == rx.wrapping_neg() {
        mem.write_ram(0x00, 1); // $00=1（离屏/wrap）
        return false;
    }
    true
}


/// MakeRoomRAMPtr（**asm 内容锚** `: 07:E96A`——r26）：位置→RoomRAM 瓦址——
/// ptr=0x6000|名表页(hi<<10)|((y&0xF8)<<2)|(x>>3)（y/8×32+x/8 形！）。
fn make_room_ram_ptr(mem: &mut crate::frame::FrameMemory, y: u8, x: u8, hi: u8) -> u16 {
    let idx = (((y & 0xF8) as u16) << 2) | ((x >> 3) as u16);
    let page = if hi & 1 != 0 { 0x0400u16 } else { 0x0000 };
    0x6000 | page | (idx & 0x3FF)
}

/// 碰撞核（**asm 内容锚** `CheckMoveVertical: ; 07:E7BD`/Horizontal $E89B——r26）：
/// 逐点（块界）→MakeRoomRAMPtr→tile 值类：**<$80=固实（碰撞！）**/$4E=固实空（门/像）/
/// $80-$9F=可炸/≥$A0=可走。返回 carry（可走=true）。
fn collision_point_ok(mem: &mut crate::frame::FrameMemory, y: u8, x: u8, hi: u8) -> bool {
    let ptr = make_room_ram_ptr(mem, y, x, hi);
    let tile = mem.read_wram(ptr);
    if tile == 0x4E {
        return false; // 固实空（门/像——碰撞！）
    }
    if tile < 0x80 {
        return false; // 固实 tile（< $80）——碰撞！
    }
    // $80-$9F=可炸（可走性=IsBlastTile 判——简化径：视为可走）
    // ≥$A0=可走（$A0/$A1=门 tile（Samus 专用判——随件））
    true
}

/// EnemyCheckMoveUp（**asm 内容锚** `: 07:E770`→EnemyCheckMoveVertical `07:E783`→
/// CheckMoveVertical **x 全距走环**——r9 转向族考证修（crawler-corner-divergence.md））：
/// staging=enemy::movement（distY=ry+8 忠实），探行=y−distY（进入块上块），
/// x−rx 起 n 列全距（GetNumBlocksToCheck 公式=vertical_span_cols 同构已验证）；
/// rx==0⇒免检（asm `bne Lx198/sec/rts`）。旧形单点 (y−ry+8, x) 双错：探点 +8 落出
/// 进入块+漏跨块列——爬虫 radius 跨块时转角决策分歧根因。
fn enemy_check_move_up(mem: &mut crate::frame::FrameMemory, x: u8) -> bool {
    let y = mem.read_ram(0x0400 + x as u16 + 0);
    let ry = mem.read_wram(0x6AF4 + x as u16 + 1);
    let xpos = mem.read_ram(0x0400 + x as u16 + 1);
    let rx = mem.read_wram(0x6AF4 + x as u16 + 2); // radiusX=+2
    let hi = mem.read_wram(0x6AF4 + x as u16 + 7); // hi=+7
    let st = crate::enemy::movement::enemy_check_move_up(rx, ry, xpos, y, hi);
    match crate::enemy::movement::check_move_boundary(st, false) {
        crate::enemy::movement::MoveCheckOutcome::NoCollision => true,
        crate::enemy::movement::MoveCheckOutcome::NeedsBgCollision(st) => {
            let row = st.pos_y.wrapping_sub(st.dist_to_center_y);
            let (left, n) = crate::samus::physics::vertical_span_cols(st.pos_x, st.radius_x);
            for i in 0..n {
                let col = left.wrapping_add((i as u8).wrapping_mul(8));
                if !collision_point_ok(mem, row, col, st.pos_hi) {
                    return false;
                }
            }
            true
        }
    }
}

/// EnemyCheckMoveDown（**asm 内容锚** `: 07:E77B`——r26）：底块判。
/// EnemyCheckMoveDown（**asm 内容锚** `: 07:E77B`→同上 x 全距走环——r9 转向族修）：
/// staging distY=0−ry（两位补码）⇒探行=y−(−ry)=y+ry（进入块下块）。
fn enemy_check_move_down(mem: &mut crate::frame::FrameMemory, x: u8) -> bool {
    let y = mem.read_ram(0x0400 + x as u16 + 0);
    let ry = mem.read_wram(0x6AF4 + x as u16 + 1);
    let xpos = mem.read_ram(0x0400 + x as u16 + 1);
    let rx = mem.read_wram(0x6AF4 + x as u16 + 2);
    let hi = mem.read_wram(0x6AF4 + x as u16 + 7);
    let st = crate::enemy::movement::enemy_check_move_down(rx, ry, xpos, y, hi);
    match crate::enemy::movement::check_move_boundary(st, false) {
        crate::enemy::movement::MoveCheckOutcome::NoCollision => true,
        crate::enemy::movement::MoveCheckOutcome::NeedsBgCollision(st) => {
            let row = st.pos_y.wrapping_sub(st.dist_to_center_y);
            let (left, n) = crate::samus::physics::vertical_span_cols(st.pos_x, st.radius_x);
            for i in 0..n {
                let col = left.wrapping_add((i as u8).wrapping_mul(8));
                if !collision_point_ok(mem, row, col, st.pos_hi) {
                    return false;
                }
            }
            true
        }
    }
}

/// EnemyCheckMoveLeft（**asm 内容锚** `: 07:E8F1`——r26）：左块判。
/// EnemyCheckMoveLeft（**asm 内容锚** `: 07:E8F1`→CheckMoveHorizontal **y 全距走环**
/// （07:E89B）——r9 转向族修）：staging distX=rx+8⇒探列=x−(rx+8)（进入块左块），
/// y−ry 起 n 行全距（horizontal_span_rows 同构）；ry==0⇒免检。
fn enemy_check_move_left(mem: &mut crate::frame::FrameMemory, x: u8) -> bool {
    let y = mem.read_ram(0x0400 + x as u16 + 0);
    let xpos = mem.read_ram(0x0400 + x as u16 + 1);
    let rx = mem.read_wram(0x6AF4 + x as u16 + 2); // radiusX=+2
    let ry = mem.read_wram(0x6AF4 + x as u16 + 1);
    let hi = mem.read_wram(0x6AF4 + x as u16 + 7);
    let st = crate::enemy::movement::enemy_check_move_left(rx, ry, xpos, y, hi);
    match crate::enemy::movement::check_move_boundary(st, true) {
        crate::enemy::movement::MoveCheckOutcome::NoCollision => true,
        crate::enemy::movement::MoveCheckOutcome::NeedsBgCollision(st) => {
            let col = st.pos_x.wrapping_sub(st.dist_to_center_x);
            let (top, n) = crate::samus::physics::horizontal_span_rows(st.pos_y, st.radius_y);
            for i in 0..n {
                let row = top.wrapping_add((i as u8).wrapping_mul(8));
                if !collision_point_ok(mem, row, col, st.pos_hi) {
                    return false;
                }
            }
            true
        }
    }
}

/// EnemyCheckMoveRight（**asm 内容锚** `: 07:E8FC`——r26）：右块判。
/// EnemyCheckMoveRight（**asm 内容锚** `: 07:E8FC`→CheckMoveHorizontal y 全距走环
/// ——r9 转向族修）：staging distX=0−rx⇒探列=x+rx（进入块右块），y−ry 起 n 行全距。
fn enemy_check_move_right(mem: &mut crate::frame::FrameMemory, x: u8) -> bool {
    let y = mem.read_ram(0x0400 + x as u16 + 0);
    let xpos = mem.read_ram(0x0400 + x as u16 + 1);
    let rx = mem.read_wram(0x6AF4 + x as u16 + 2);
    let ry = mem.read_wram(0x6AF4 + x as u16 + 1);
    let hi = mem.read_wram(0x6AF4 + x as u16 + 7);
    let st = crate::enemy::movement::enemy_check_move_right(rx, ry, xpos, y, hi);
    match crate::enemy::movement::check_move_boundary(st, true) {
        crate::enemy::movement::MoveCheckOutcome::NoCollision => true,
        crate::enemy::movement::MoveCheckOutcome::NeedsBgCollision(st) => {
            let col = st.pos_x.wrapping_sub(st.dist_to_center_x);
            let (top, n) = crate::samus::physics::horizontal_span_rows(st.pos_y, st.radius_y);
            for i in 0..n {
                let row = top.wrapping_add((i as u8).wrapping_mul(8));
                if !collision_point_ok(mem, row, col, st.pos_hi) {
                    return false;
                }
            }
            true
        }
    }
}


/// UpdateSamusProjectiles+DoOneSamusProjectile（**asm 内容锚** `: 07:D4BF/D4CB`——r26）：
/// 3 槽（Objects 13-15=$D0/$E0/$F0）状态 dispatch：0=ExitSub（空径——移动对拍主径！）/
/// 1=Bullet/2=Wave/3=Ice/4=Explode/5-10=Bomb 族/11=Missile（打敌人流程件——随件）。
pub fn update_samus_projectiles(mem: &mut crate::frame::FrameMemory) {
    for slot in [0xD0u8, 0xE0, 0xF0] {
        mem.write_ram(0x004B, slot); // PageIndex（DoOneSamusProjectile（$D4CB））
        let status = mem.read_ram(0x0300 + slot as u16);
        match status {
            0 => {} // ExitSub
            1 | 11 => update_bullet(mem), // UpdateBullet（常规/导弹共壳（:4288））
            4 => update_bullet_explode(mem), // UpdateBulletExplode（$D5C8）
            2 | 3 => {
                // Wave/Ice（gear 门径（无波/冰齿轮不触发）——最小面共 UpdateBullet 壳
                crate::frame::DIVERGENCES.with(|d| d.borrow_mut().push(0xD522)); // 轨迹面（记账）
                update_bullet(mem);
            }
            _ => {
                // Bomb 族（5-10（埋弹触发面（不在本卡判据））——记账 no-op（防爆）
                crate::frame::DIVERGENCES.with(|d| d.borrow_mut().push(0xD4CB));
            }
        }
    }
}

/// 弹爆径（UpdateBullet_Explode（$D5E4（audit §8）））：anim←BulletHit(0x1D)
/// （导弹⇒MissileExplode(0x91)）+status←wa_BulletExplode(4)。
fn bullet_explode(mem: &mut crate::frame::FrameMemory) {
    let x = mem.read_ram(0x004B) as u16;
    let base = 0x0300u16 + x;
    let st = mem.read_ram(base);
    if st == 4 {
        return; // 已爆（asm beq @RTS）
    }
    let anim = if st == 0x0B { 0x91 } else { 0x1D };
    mem.write_ram(base + 5, anim);
    mem.write_ram(base + 6, anim);
    mem.write_ram(base + 4, 0);
    mem.write_ram(base, 4);
}

/// 弹移动（UpdateBullet_Move（$D624（audit §8）））：speedX/Y→位移（wrapping）。
fn bullet_move(mem: &mut crate::frame::FrameMemory) {
    let x = mem.read_ram(0x004B) as u16;
    let base = 0x0300u16 + x;
    let sx = mem.read_ram(base + 9);
    let sy = mem.read_ram(base + 8);
    mem.write_ram(base + 0x0E, mem.read_ram(base + 0x0E).wrapping_add(sx));
    mem.write_ram(base + 0x0D, mem.read_ram(base + 0x0D).wrapping_add(sy));
}

/// UpdateBullet（$D4EB（audit §8——Z 射真缺口面（2026-10-28 全链通读端口）））：
/// UpdatingSamusProjectile($71)←1/删除（onScreen 清）/精灵撞（isHit）/BG 撞
/// （空气（≥$A0）⇒移+免撞；实心⇒爆（$4E 门瓦⇒任务卡 2 径复用））/时寿 dec（长束免）/
/// DrawBullet（AnimDrawObject(A=1)）。
fn update_bullet(mem: &mut crate::frame::FrameMemory) {
    let x = mem.read_ram(0x004B) as u16;
    let base = 0x0300u16 + x;
    mem.write_ram(0x0071, 1);
    // DeleteIfOffScreen（$D5FC）：onScreen bit0 清⇒删
    if mem.read_ram(base + 0x0B) & 1 == 0 {
        mem.write_ram(base, 0);
        mem.write_ram(0x0071, 0);
        return;
    }
    // ExplodeIfHitSprite（$D5DA）：isHit,x≠0⇒清+爆
    let mut collided = false;
    if mem.read_ram(base + 0x0A) != 0 {
        mem.write_ram(base + 0x0A, 0);
        bullet_explode(mem);
        collided = true;
    }
    if !collided {
        // CollisionWithBG（$D609）：瓦判（GetObjRoomRAMPtr 同核（make_room_ram_ptr））
        let (lo, hi) = crate::frame::room::make_room_ram_ptr(
            mem.read_ram(base + 0x0D), mem.read_ram(base + 0x0E), mem.read_ram(base + 0x0C));
        let tile = mem.read_wram(((hi as u16) << 8) | lo as u16);
        if tile >= 0xA0 {
            bullet_move(mem); // 空气⇒移（carry=1（免撞））
        } else {
            if tile == 0x4E {
                // 门瓦⇒任务卡 2 弹击径（复用（isHit&4 写））+爆（carry=0）
                samus_projectile_hit_door_or_statue(mem, lo, hi); // 同模块（任务卡 2 件）
            } else if tile >= 0x80 {
                crate::frame::DIVERGENCES.with(|d| d.borrow_mut().push(0x8D2F)); // 瓦爆面（记账）
            }
            collided = true;
        }
    }
    // CheckBulletStat（$D4F8）
    if !collided {
        let d = mem.read_ram(base + 0x0F).wrapping_sub(1);
        mem.write_ram(base + 0x0F, d);
        if d == 0 {
            mem.write_ram(base, 0); // 时寿耗尽⇒清
        }
    } else if mem.read_ram(base) != 0 {
        bullet_explode(mem); // 撞⇒爆（status≠0 时）
    }
    // DrawBullet（$D517）：AnimDrawObject(A=1)（动画推进面 A=1）
    crate::samus::physics::update_obj_anim(mem, 1);
    crate::frame::sprite::anim_draw_object(mem, mem.read_ram(0x004B));
    mem.write_ram(0x0071, 0);
}

/// UpdateBulletExplode（$D5C8）：animFrame==_id_FrameNone($F7)（动画尽）⇒清；
/// DrawBullet。
fn update_bullet_explode(mem: &mut crate::frame::FrameMemory) {
    let x = mem.read_ram(0x004B) as u16;
    let base = 0x0300u16 + x;
    mem.write_ram(0x0071, 1);
    if mem.read_ram(base + 3) == 0xF7 {
        mem.write_ram(base, 0);
    }
    crate::samus::physics::update_obj_anim(mem, 1);
    crate::frame::sprite::anim_draw_object(mem, mem.read_ram(0x004B));
    mem.write_ram(0x0071, 0);
}

/// UpdateEnemy_Resting（**asm 内容锚** `: 07:F3BE`——r26 壳：30fps 门/件序 fail-loud）。
fn enemy_resting(mem: &mut crate::frame::FrameMemory, x: u8) {
    let d = mem.read_ram(0x0400 + x as u16 + 5);
    if d & 0x40 == 0 {
        // 30fps 标 clear⇒全件（bit6 set 时跳过=30fps 节拍）
        mem.write_wram(0x6AF4 + x as u16 + 13, 0); // jumpDsplcmnt←0
        mem.write_ram(0x0400 + x as u16 + 7, 0); // movementInstrIndex←0
        mem.write_ram(0x0400 + x as u16 + 10, 0); // data0A←0
        enemy_force_speed(mem, x); // F6B9（件序首——速向萨姆斯力）
        enemy_distance_threshold(mem, x); // F75B（件序 2——阈距判）
        init_en_resting_anim_index(mem, x); // F682（件序 3）
        update_en_data1f(mem, x); // F676（件序 4）
        let delay = mem.read_ram(0x0400 + x as u16 + 9);
        if delay != 0 {
            try_becoming_active(mem, x); // F7BA（件序 5——延迟归零⇒激活）
        }
    }
    enemy_react_to_projectile(mem, x);
    choose_enemy_ai(mem, x);
}

/// UpdateEnemy_Active（**asm 内容锚** `: 07:F3E6`→Active_BranchA(F401)→BranchB(F40A)→F40D）。
/// 【goal-r16 复核修正 G2'】F401 三件=ForceSpeed+Threshold+RemoveIfInLava（ROM
/// `20 B9 F6 20 5B F7 20 1E F5`）；AI 分派在 BranchB→F40D（Resting/Active 共用）。
/// 旧形三径皆 `unported!` ⇒ 敌人激活即 panic。
fn enemy_active(mem: &mut crate::frame::FrameMemory, x: u8) {
    let d = mem.read_ram(0x0400 + x as u16 + 5);
    if d & 0x40 == 0 {
        // bit6 clear ⇒ 走 BranchA/bit5 径（bit6 set⇒30fps 直跳 BranchB）
        if d & 0x20 != 0 {
            // bit5 set⇒delay←EnemyInitDelayTbl[type]；status Active→Resting
            let t = mem.read_wram(0x6AF4 + x as u16 + 14); // EnsExtra.type
            let delay = game_data::area::graphics::ENEMY_INIT_DELAY.get(t as usize).copied().unwrap_or(0);
            mem.write_ram(0x0400 + x as u16 + 9, delay); // Ens.delay(+9)
            let st = mem.read_wram(0x6AF4 + x as u16);
            mem.write_wram(0x6AF4 + x as u16, st.wrapping_sub(1));
        } else {
            // Active_BranchA（F401）：ForceSpeed(F6B9)+Threshold(F75B)+RemoveIfInLava(F51E)
            enemy_force_speed(mem, x);
            enemy_distance_threshold(mem, x);
            enemy_remove_if_in_lava(mem, x);
        }
    }
    // BranchB（F40A→F40D）：enemy_react_to_projectile + choose_enemy_ai（AI 分派）
    enemy_react_to_projectile(mem, x);
    choose_enemy_ai(mem, x);
}

/// RemoveEnemyIfItIsInLava（**asm 内容锚** `: 07:F51E`）：横滚房+敌 y≥$EC（熔岩）⇒清槽。
fn enemy_remove_if_in_lava(mem: &mut crate::frame::FrameMemory, x: u8) {
    let scroll_dir = mem.read_ram(crate::ram_map::SCROLL_DIR);
    if scroll_dir < 2 {
        return; // 竖滚房⇒exit（asm cmp #$02/bcc）
    }
    let y = mem.read_ram(0x0400 + x as u16); // Ens.y
    if y < 0xEC {
        return; // 熔岩上方⇒exit
    }
    // 熔岩中⇒RemoveEnemy（清槽）
    mem.write_wram(0x6AF4 + x as u16, 0); // EnsExtra.status←0（槽空）
}


// ==== r26 UpdateWorld dispatch 壳（复核 C blocker——缺件静默分歧根修！）====
// 模式：空径端口（槽环全空⇒忠实跳过）+ 活性 fail-loud（弹件径同式）。

/// AreaRoutine（区 bank $95C3——区专径：液面/滚屏特效族）。
fn uw_area_routine(_mem: &mut crate::frame::FrameMemory) {
    // Brinstar 区专径（空态无液面——首房间实证；活性随件）
}

/// UpdateElevator（$D7B3——电梯显示族）：槽空⇒跳过。
fn uw_elevator(_mem: &mut crate::frame::FrameMemory) {
    // 电梯槽（Elevator=$0320）status=0⇒跳过 ✓
}

/// UpdateAllEnemyExplosions（爆族——槽空⇒跳过）。
fn uw_enemy_explosions(_mem: &mut crate::frame::FrameMemory) {}

/// UpdateAllMellows（虫族）。
fn uw_mellows(_mem: &mut crate::frame::FrameMemory) {}

/// UpdateAllEnProjectiles（敌弹族——槽空⇒跳过）。
fn uw_en_projectiles(_mem: &mut crate::frame::FrameMemory) {}

/// UpdateAllSkreeProjectiles（Skree 弹族）。
fn uw_skree_projectiles(_mem: &mut crate::frame::FrameMemory) {}

/// SamusEnterDoor（$8B13——过门检）：全径实现=physics::samus_enter_door（goal-r29）。
fn uw_samus_enter_door(mem: &mut crate::frame::FrameMemory) {
    crate::samus::physics::samus_enter_door(mem);
}

/// UpdateAllDoors（门链——槽空⇒跳过；门显示随件）。
fn uw_all_doors(_mem: &mut crate::frame::FrameMemory) {}

/// UpdateAllTileBlasts（瓦片爆/复原——空态跳过）。
fn uw_tile_blasts(_mem: &mut crate::frame::FrameMemory) {}

/// CollisionDetection（实体碰撞——SamusCollisionWithSolidEntities 已在 physics 链！）。
fn uw_collision_detection(_mem: &mut crate::frame::FrameMemory) {
    // 碰撞主体已在 lava_and_move_check 链（samus_collision_with_solid_entities D976！）
}

/// DisplayBar（**asm 内容锚** `DisplayBar: ; 07:E0C1`——HUD 精灵动态面，全径端口）：
/// ① 拷 DataDisplayTbl(E1B9, 40B=10 精灵)→OAM[$5B..]（ROM 字节核实 40 字节）
/// ② 健康 2 位：($0107&0xF)|$A0、($0106>>4)|$A0（SPRWriteDigit E173 `ora #$A0`）
/// ③ $010B≠$FF⇒逃生计时 3 位+TI/ME($58/$59 attrib++）；否则 MaxMissiles($687A)≠0⇒
///    HexToDec(E198) 3 位；=0⇒LE10A 擦图标 tile（$0200+X+$0D/$11←$FF，X=原页+8，
///    恰为表槽 5/6 图标 tileID）；LE14A 能量罐（full=$0107>>4，tile $6F/空 $6E，
///    AddOneTank E17B：y=$17/attr=$01/x=$40 递减 $0A）。
const DATA_DISPLAY_TBL: [u8; 40] = [
    0x21, 0xA0, 0x01, 0x30, 0x21, 0xA0, 0x01, 0x38,
    0x2B, 0xFF, 0x01, 0x28, 0x2B, 0xFF, 0x01, 0x30,
    0x2B, 0xFF, 0x01, 0x38, 0x2B, 0x5E, 0x00, 0x18,
    0x2B, 0x5F, 0x00, 0x20, 0x21, 0x76, 0x01, 0x18,
    0x21, 0x7F, 0x01, 0x20, 0x21, 0x3A, 0x00, 0x28,
];

/// SPRWriteDigit（E173）：tile=A|$A0（数字图案区），X+4（Xplus4 E193）。
fn spr_write_digit(mem: &mut crate::frame::FrameMemory, x: u8, a: u8) {
    mem.write_ram(0x0200u16 + x as u16 + 1, a | 0xA0);
}

/// HexToDec（E198+DivideByRepeatedSubtraction E1AD）：百/十/个位。
fn hex_to_dec(n: u8) -> (u8, u8, u8) {
    let hundreds = n / 100;
    let rem = n % 100;
    let tens = rem / 10;
    (hundreds as u8, tens as u8, rem % 10)
}

fn uw_display_bar(mem: &mut crate::frame::FrameMemory) {
    // ① 拷表（E0C7 循环：40B→OAM[X..X+40)，$5B←X+40，pla 恢复原 X）
    let x0 = mem.read_ram(0x005B);
    let mut x = x0;
    for i in 0..40usize {
        mem.write_ram(0x0200u16 + x as u16, DATA_DISPLAY_TBL[i]);
        x = x.wrapping_add(1);
    }
    mem.write_ram(0x005B, x);
    x = x0;
    // ② 健康 2 位（E0E0 区：lda Health+1/and #$0F → SPRWriteDigit；lda Health/Adiv16 → ）
    spr_write_digit(mem, x, mem.read_ram(0x0107) & 0x0F);
    x = x.wrapping_add(4);
    spr_write_digit(mem, x, mem.read_ram(0x0106) >> 4);
    x = x.wrapping_add(4);
    // ③ 分派（E0EA：ldy $010B/iny/bne LE11C——≠$FF ⇒ 逃生）
    if mem.read_ram(0x010B) != 0xFF {
        // LE11C：逃生计时 3 位 + TI/ME
        spr_write_digit(mem, x, mem.read_ram(0x010B) >> 4);
        x = x.wrapping_add(4);
        spr_write_digit(mem, x, mem.read_ram(0x010B) & 0x0F);
        x = x.wrapping_add(4);
        spr_write_digit(mem, x, mem.read_ram(0x010A) >> 4);
        x = x.wrapping_add(4);
        let ti = 0x0200u16 + x as u16;
        mem.write_ram(ti, 0x58); // "TI"
        mem.write_ram(ti + 2, mem.read_ram(ti + 2).wrapping_add(1));
        if x < 0xFC {
            let me = 0x0200u16 + x.wrapping_add(4) as u16;
            mem.write_ram(me, 0x59); // "ME"
            mem.write_ram(me + 2, mem.read_ram(me + 2).wrapping_add(1));
        }
    } else if mem.read_ram(0x687A) != 0 {
        // 导弹 3 位（HexToDec E198）
        let (h, t, o) = hex_to_dec(mem.read_ram(0x6879));
        spr_write_digit(mem, x, h);
        x = x.wrapping_add(4);
        spr_write_digit(mem, x, t);
        x = x.wrapping_add(4);
        spr_write_digit(mem, x, o);
        x = x.wrapping_add(4);
    } else {
        // LE10A：无导弹 ⇒ 擦图标两半 tileID（X=原页+8 恰指表槽 5/6）
        if x < 0xF4 {
            mem.write_ram(0x0200u16 + x as u16 + 0x0D, 0xFF);
        }
        if x < 0xF0 {
            mem.write_ram(0x0200u16 + x as u16 + 0x11, 0xFF);
        }
        // bne LE14A
    }
    // ④ LE14A 能量罐
    let mut x = mem.read_ram(0x005B); // 重读 $5B（追加后位置）
    let tanks = mem.read_ram(0x6877);
    if tanks == 0 {
        return; // RTS_E172
    }
    mem.write_ram(0x0003, tanks);
    mem.write_ram(0x0000, 0x40); // 最右罐 X 坐标
    let mut tile: u8 = 0x6F; // 满罐
    let full = mem.read_ram(0x0107) >> 4;
    mem.write_ram(0x0001, full);
    if full == 0 {
        tile = tile.wrapping_sub(1); // dey ⇒ 空罐 $6E
    }
    loop {
        // AddOneTank(E17B)
        mem.write_ram(0x0200u16 + x as u16, 0x17); // y
        mem.write_ram(0x0200u16 + x as u16 + 1, tile); // tile
        mem.write_ram(0x0200u16 + x as u16 + 2, 0x01); // attr
        mem.write_ram(0x0200u16 + x as u16 + 3, mem.read_ram(0x0000)); // x 坐标
        mem.write_ram(0x0000, mem.read_ram(0x0000).wrapping_sub(0x0A));
        x = x.wrapping_add(4); // Xplus4
        mem.write_ram(0x0001, mem.read_ram(0x0001).wrapping_sub(1));
        if mem.read_ram(0x0001) == 0 {
            tile = tile.wrapping_sub(1); // 满罐尽⇒切空罐
        }
        mem.write_ram(0x0003, mem.read_ram(0x0003).wrapping_sub(1));
        if mem.read_ram(0x0003) == 0 {
            break;
        }
    }
    mem.write_ram(0x005B, x);
}

/// UpdateAllPipeBugHoles（虫洞族——空态跳过）。
fn uw_pipe_bug_holes(_mem: &mut crate::frame::FrameMemory) {}

/// CheckMissileToggle（弹切检——A/SELECT 切换面）。
fn uw_check_missile_toggle(_mem: &mut crate::frame::FrameMemory) {}

/// UpdateAllPowerUps（$DB37——吃能力！槽环+baked 判+拾取碰撞）。
fn uw_all_power_ups(_mem: &mut crate::frame::FrameMemory) {
    // sc-item-pickup 流程件——PowerUps 槽+RoomRAM buried 判+拾取碰撞随件
}

/// UpdateTourianItems（$FDE3——Tourian 专件）。
fn uw_tourian_items(_mem: &mut crate::frame::FrameMemory) {}

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
    // 【move-drift R2】ScreenOff（$C439→WriteAndWait→WaitNMIPass_ prg7:1129-1140）=
    // **帧等待**（1 tick+NMI 内块全跑）——门队列声明（窗3 等待 1/3）。
    mem.iter_wait();
    crate::frame::boot::c158_unit(mem); // ClearNameTables（已折）
    crate::frame::boot::c1a3_unit(mem); // EraseAllSprites（已折）
    mem.write_ram(0x005A, 0); // ScrollBlockOnNameTable3
    mem.write_ram(0x0053, 0); // ScrollBlockOnNameTable0
    mem.write_ram(0x0030, 1); // SpareMem30
    mem.write_ram(crate::ram_map::SCROLL_DIR, 2); // ScrollDir=2（横房）——【勘误修】$49！
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
    // 【move-drift R3】SetupRoom 环+名表拷贝=重活（房间解压——原版掉帧窗 bogged 2 帧；
    // C 级拆分（总 4=A 级三源互证——窗3 行集×FC×$18/$19 计数））。
    mem.iter_bogged(2);
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
    // 【move-drift R2】ScreenOn（$C447→WriteAndWait——prg7:1142-1146 fallthrough）=
    // **帧等待**（窗3 等待 3/3）。
    mem.iter_wait();
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

/// GetRoomNum（**asm 内容锚** `GetRoomNum: ; 07:E720`）：ScrollDir 横/纵分流+
/// ScrollBlock 门（横滚门挡：方向位 1=左/2=右 与 ScrollBlockOnNameTable3,y（$6C/$6D）
/// 按位与——挡⇒不载房）+MapPos 数学（MapY*32+MapX+$7000）→WorldMapRAM 查表
/// →RoomNumber($5A)+特别房音乐（$79——AreaItemRoomNumbers 7 项）。
/// 返回 **true=取得房间号（可滚）**（asm carry 反形：CF=1=失败）。
pub fn get_room_num(mem: &mut FrameMemory) -> bool {
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
    mem.write_ram(0x005A, room); // RoomNumber
    true
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
    mem.write_ram(0x0068, palette); // RoomPalette（**$68**——r25 址勘误修（旧 $36 随证形误））
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


// （旧 game_engine/update_world 桩已并入上方 r26 端口形——原块删除）

// （UpdateWorld 旧壳已并入上方 r26 dispatch——原块删除）
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
        m.write_ram(GAME_MODE, 1); // 标题路（continuation 语义面——GameMode=1 免游戏派发）
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
        m.write_ram(GAME_MODE, 1); // 标题路（continuation 语义面）
        m.write_ram(crate::ram_map::TITLE_ROUTINE, 2);
        m.title_cursor = 2;
        m.title_cursor_phase = 1; // 归属相位≠当前（孤儿）
        m.write_ram(TIMER_DELAY, 5);
        main_loop_iter(&mut m);
        assert_eq!(m.read_ram(TIMER_DELAY), 4, "UpdateTimer 跑（真迭代——禁误门）");
    }

    /// FC 冻结门（r9 porter 修——findings/fc-phase-residual.md）：内旋族相位
    /// （$01 上电初始化臂/$1B StartContinue 臂——asm 例程体+WaitNMIPass_ 自旋尾不返回
    /// 主循环）tick 不自增 FC；相位推进并入臂内最后工作段（emu 冻帧内推进实录：
    /// 上电 f4-5 冻/f6 推进、menu f311-312 冻/f313 起自增），臂毕恢复自增。
    /// mutation 面：旧形（每 tick 无条件加）下两窗各多 3 ⇒ fcprobe 逐帧差 +6 ⇒ 本测红。
    #[test]
    fn fc_frozen_on_no_return_title_arms() {
        let mut m = crate::frame::FrameMemory::default();
        m.write_ram(GAME_MODE, 1);
        // $01 臂：2 cursor 步全冻结，第 2 步相位→0x02（Step 4a 形：段拍后跟 bogged
        // 溢出窗+0xFF 完成拍哨兵——本测直驱 main_loop_iter 不经 engine 门消费，哨兵直达）
        m.write_ram(crate::ram_map::TITLE_ROUTINE, 0x01);
        for _ in 0..2 {
            main_loop_iter(&mut m);
        }
        assert_eq!(m.read_ram(FRAME_COUNT), 0, "$01 臂 2 cursor 步 FC 全冻");
        assert_eq!(m.read_ram(crate::ram_map::TITLE_ROUTINE), 0x02, "段拍相位→0x02");
        main_loop_iter(&mut m); // 0xFF 完成拍哨兵（原 0x01 迭代的收尾 inc 落此）
        assert_eq!(m.read_ram(FRAME_COUNT), 1, "哨兵拍 inc FC（完成拍）");
        main_loop_iter(&mut m);
        assert_eq!(m.read_ram(FRAME_COUNT), 2, "0x02 起恢复自增");
        // $1B 臂（Step 4b 形：3 cursor 步全冻结+段2 相位→0x16；第 4 tick=0xFF 哨兵 inc；
        // dive 语义=同迭代无第二 prelude——T3 纪元残差随 map-v2 +1/+2 归零闭合）
        m.write_ram(crate::ram_map::TITLE_ROUTINE, 0x1B);
        for _ in 0..3 {
            main_loop_iter(&mut m);
        }
        assert_eq!(m.read_ram(FRAME_COUNT), 2, "$1B 臂 3 cursor 步 FC 全冻");
        assert_eq!(m.read_ram(crate::ram_map::TITLE_ROUTINE), 0x16, "段2 相位→0x16");
        main_loop_iter(&mut m); // 0xFF 哨兵
        assert_eq!(m.read_ram(FRAME_COUNT), 3, "哨兵拍 inc FC（emu f314 FC=50 形）");
        main_loop_iter(&mut m);
        assert_eq!(m.read_ram(FRAME_COUNT), 4, "0x16 起恢复自增");
    }

    /// 敌检全距（r9 转向族修 mutation 面——crawler-corner-divergence.md）：radiusX 跨块
    /// 时上检须探左列（单点旧形只探中心列⇒漏侧壁⇒本测红）。
    #[test]
    fn enemy_check_move_up_span_covers_side_columns() {
        let mut m = crate::frame::FrameMemory::default();
        for i in 0..0x400u16 {
            m.write_wram(0x6000 + i, 0xA0); // 全可走
        }
        m.write_ram(0x0400, 0x60); // y
        m.write_ram(0x0401, 0x40); // x
        m.write_wram(0x6AF5, 8); // radiusY=8
        m.write_wram(0x6AF6, 12); // radiusX=12（跨 3+ 块）
        m.write_wram(0x6AFB, 0); // hi
        assert!(enemy_check_move_up(&mut m, 0), "全空⇒放行");
        // 探行=0x60−(8+8)=0x50；左列=0x40−12=0x34（中心列 0x40 左侧两块）——侧壁固实
        let idx = (((0x50u16 & 0xF8) << 2) | (0x34 >> 3)) as usize;
        m.write_wram(0x6000 + idx as u16, 0x40);
        assert!(!enemy_check_move_up(&mut m, 0), "左列固实⇒碰撞（单点旧形漏（中心列空））");
    }

    /// 敌检全距（右臂）：radiusY 跨块时右检须探下侧行（单点旧形漏）。
    #[test]
    fn enemy_check_move_right_span_covers_bottom_rows() {
        let mut m = crate::frame::FrameMemory::default();
        for i in 0..0x400u16 {
            m.write_wram(0x6000 + i, 0xA0);
        }
        m.write_ram(0x0400, 0x60); // y
        m.write_ram(0x0401, 0x40); // x
        m.write_wram(0x6AF5, 12); // radiusY=12（跨 3+ 块）
        m.write_wram(0x6AF6, 8); // radiusX=8
        m.write_wram(0x6AFB, 0);
        assert!(enemy_check_move_right(&mut m, 0), "全空⇒放行");
        // 探列=0x40+8=0x48；顶行=0x60−12=0x54，n=4→行 0x54/0x5C/0x64/0x6C——下行固实
        let idx = (((0x6Cu16 & 0xF8) << 2) | (0x48 >> 3)) as usize;
        m.write_wram(0x6000 + idx as u16, 0x40);
        assert!(!enemy_check_move_right(&mut m, 0), "下行固实⇒碰撞（单点旧形漏（中心行空））");
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

    /// GoMainRoutine 标题路**会师穿越**（bank00:8000 已通——phase 0 臂单工作段+bogged
    /// span（Step 2 机构改形）：相位推进/FC 同拍落工作段，等待=门队列挂起）。
    #[test]
    fn go_main_routine_title_crosses_meeting_point() {
        let mut m = FrameMemory::default();
        m.write_ram(GAME_MODE, 1); // TitleRoutine=0=InitializeAfterReset
        go_main_routine(&mut m); // 工作段（仪式+推进同拍）
        assert_eq!(m.read_ram(crate::ram_map::TITLE_ROUTINE), 1); // 相位推进（inc $1F）
        assert_eq!(m.iter_gates, vec![0], "bogged 门队列挂起（N₀=1——map-v2 实锤）");
        assert!(m.iter_span_fc, "FC 延至 span 尾收口旗");
    }

    /// 【move-drift R1 链合一】AreaInit→MoreInit fallthrough（asm prg7:1826-1843 无 RTS
    /// 直落 C81D）：**一迭代双步 mr 00→02**（金腿窗3 状态差实证——同迭代链断裂修复锚）。
    #[test]
    fn game_dispatch_area_init_falls_into_more_init_same_iter() {
        let mut m = FrameMemory::default();
        copy_map(&mut m);
        m.write_ram(GAME_MODE, 0);
        m.write_ram(crate::ram_map::MAIN_ROUTINE, 0);
        m.write_ram(crate::ram_map::IN_AREA, 0);
        game_main_routine(&mut m); // 一迭代（dispatch 0）
        assert_eq!(m.read_ram(crate::ram_map::MAIN_ROUTINE), 2, "一迭代 mr 00→02（fallthrough）");
    }

    /// 【PreAI 门 mutation 钉（2026-10-31 路径分叉根因件）】EnemyStatusPreAI=Resting ⇒
    /// crawler_ai 早退（SkreeExit_Resting=anim8+显示）——敌**不动/朝向不翻**（asm
    /// crawler.asm:8-11；缺门症：激活帧即跑 AI 且半径未写（draw 后行）⇒rx=0 快径⇒
    /// 外角下探伪成功转角⇒菱形回路——crawlprobe f797 实录）。
    #[test]
    fn crawler_ai_skips_movement_while_preai_resting() {
        let mut m = FrameMemory::default();
        let x = 0x40u8; // slot4（字节偏移）
        // 槽态：Resting、朝向 0（地板向右）、半径 8/8（draw 已写过形）、屏内位置
        m.write_wram(0x6AF4 + x as u16, 1); // status=Resting
        m.write_wram(0x6AF4 + x as u16 + 1, 8); // radiusY
        m.write_wram(0x6AF4 + x as u16 + 2, 8); // radiusX
        m.write_wram(0x6AF4 + x as u16 + 3, 0xFF); // animFrame=None（拦 draw_enemy 半径回装覆盖手设值）
        m.write_wram(0x6AF4 + x as u16 + 14, 5); // type=Zoomer
        m.write_ram(0x0400 + x as u16, 0x28); // y
        m.write_ram(0x0400 + x as u16 + 1, 0xBC); // x
        m.write_ram(0x0081, 1); // EnemyStatusPreAI=Resting（asm prg7:10511 捕获点）
        m.write_ram(crate::ram_map::FRAME_COUNT, 0x0D); // 门开（(8+13)>>1&3=2）
        crawler_ai(&mut m, x);
        assert_eq!(m.read_ram(0x0400 + x as u16), 0x28, "PreAI=Resting ⇒ y 不动");
        assert_eq!(m.read_ram(0x0400 + x as u16 + 1), 0xBC, "PreAI=Resting ⇒ x 不动");
        assert_eq!(m.read_ram(0x0400 + x as u16 + 10), 0x00, "朝向不翻");
        // PreAI=Active（激活次帧）⇒正常走（x+1——rx=8 门 (0xBC+8)&7=4≠0⇒免检直行）。
        // 重钉半径/animFrame：首次调用尾部 update_enemy_anim(8)+draw 会推进 animFrame
        // 并按新帧数据回装半径（测试环境无 zoomer 动画表链——产线该链自洽：半径恒 8/8）。
        m.write_wram(0x6AF4 + x as u16 + 1, 8);
        m.write_wram(0x6AF4 + x as u16 + 2, 8);
        m.write_wram(0x6AF4 + x as u16 + 3, 0xFF);
        m.write_ram(0x0081, 2);
        crawler_ai(&mut m, x);
        assert_eq!(m.read_ram(0x0400 + x as u16 + 1), 0xBD, "PreAI=Active ⇒ 首动 x+1");
        // 下探被地板拦（RoomRAM row48 全实心）⇒朝向不翻（缺门症对照：rx=0 时此处伪转）
        assert_eq!(m.read_ram(0x0400 + x as u16 + 10), 0x00, "地板实心⇒外角不转");
    }

    /// 【TryBecomingActive 原地写钉】激活径 delay=0（asm `DE 09 04` dec 原地——emu f798
    /// delay=00 实录；旧形激活径不写回⇒delay 恒 1 残留）；bit3=0 径 inc 退回=原值。
    #[test]
    fn try_becoming_active_writes_dec_in_place() {
        let mut m = FrameMemory::default();
        let x = 0x40u8;
        m.write_ram(0x0400 + x as u16 + 9, 1); // delay=1（dec→0）
        m.write_ram(0x0400 + x as u16 + 5, 0x1A); // bit3 set（阈值已过）
        m.write_wram(0x6AF4 + x as u16, 1); // Resting
        try_becoming_active(&mut m, x);
        assert_eq!(m.read_wram(0x6AF4 + x as u16), 2, "bit3=1 ⇒ status→Active");
        assert_eq!(m.read_ram(0x0400 + x as u16 + 9), 0, "激活径 delay=0（原地 dec）");
        // bit3=0 径：dec 归 0 后 inc 退回（delay 原值保留）
        let mut m2 = FrameMemory::default();
        m2.write_ram(0x0400 + x as u16 + 9, 1); // delay=1（dec→0 后到 bit3 检查）
        m2.write_ram(0x0400 + x as u16 + 5, 0x02); // bit3 clear
        try_becoming_active(&mut m2, x);
        assert_eq!(m2.read_ram(0x0400 + x as u16 + 9), 1, "bit3=0 ⇒ delay 退回原值");
        assert_eq!(m2.read_wram(0x6AF4 + x as u16), 0, "status 不变（default 0）");
    }

    /// 【move-drift R2】more_init 门件组装（asm 序：ScreenOff($C439→WriteAndWait) 等待→
    /// SetupRoom 环+名表拷贝重活 bogged×2→尾 `jmp ScreenOn` 等待）——窗3 门序后段。
    #[test]
    fn more_init_declares_frame_gates() {
        let mut m = FrameMemory::default();
        copy_map(&mut m);
        m.write_ram(crate::ram_map::MAIN_ROUTINE, 1);
        m.write_ram(crate::ram_map::IN_AREA, 0);
        more_init(&mut m);
        assert_eq!(m.iter_gates, vec![1, 0, 0, 1], "ScreenOff(W)+bogged(2)+ScreenOn(W)");
    }
}


#[cfg(test)]
mod r25_chain_probe {
    use super::*;

    /// more_init 递进面（r25 诊断锚：mr 1→2——CopyMap 前置（单测态补地图面））。
    #[test]
    fn more_init_advances_mr() {
        let mut m = crate::frame::FrameMemory::default();
        copy_map(&mut m); // WorldMap（boot 链件）
        m.write_ram(crate::ram_map::MAIN_ROUTINE, 1);
        m.write_ram(crate::ram_map::IN_AREA, 0);
        more_init(&mut m);
        assert_eq!(m.read_ram(crate::ram_map::MAIN_ROUTINE), 2, "mr 递进 1→2");
    }
}

/// 功能4 敌人移动 AI（EnemyMove 端口）——追踪/巡逻形
///
/// refs asm 锚：bank7:F410（UpdateEnemyCommon→EnemyMove）/bank7:F7D6（EnemyMovementChoice）/
/// bank7:F83E（GetEnemyTypeTimes2PlusFacingDirection）
/// 保真边界：先做追踪 AI（驱动 f1200），完整 EnemyMovementChoices 表后续
pub fn enemy_move_ai(mem: &mut crate::frame::FrameMemory, slot: u8) {
    let px = (slot as u16) << 4;
    let ex = mem.read_ram(0x0401 + px) as i16; // Ens.0.x
    let ey = mem.read_ram(0x0400 + px) as i16; // Ens.0.y
    let sx = mem.read_ram(0x0301) as i16; // Samus.x
    let sy = mem.read_ram(0x0300) as i16; // Samus.y
    // 追踪 AI：向萨姆斯方向移动（emu 实测：x 递减左移 1px/帧）
    let dx = sx - ex;
    let dy = sy - ey;
    let new_x = if dx > 0 { ex + 1 } else if dx < 0 { ex - 1 } else { ex };
    let new_y = if dy > 0 { ey + 1 } else if dy < 0 { ey - 1 } else { ey };
    mem.write_ram(0x0401 + px, new_x as u8);
    mem.write_ram(0x0400 + px, new_y as u8);
}

/// 功能4 敌人碰撞伤害（SamusCollisionWithSolidEntities + SetEnemyTouchingSamusFlags 端口）
///
/// refs asm 锚：bank7:D976（SamusCollisionWithSolidEntities）/bank7:F2E8（SetEnemyTouchingSamusFlags）/
/// bank7:F311（SamusHurt_F311）/bank7:CDFA（CheckHealthStatus）
/// 保真边界：先做碰撞→isHit→击退（驱动 f1200），完整伤害/血量后续
pub fn enemy_collision_damage(mem: &mut crate::frame::FrameMemory, slot: u8) {
    let px = (slot as u16) << 4;
    let ex = mem.read_ram(0x0401 + px) as i16; // Ens.0.x
    let ey = mem.read_ram(0x0400 + px) as i16; // Ens.0.y
    let sx = mem.read_ram(0x0301) as i16; // Samus.x
    let sy = mem.read_ram(0x0300) as i16; // Samus.y
    // 碰撞检测：简化形（半径 8px——emu 实测撞距）
    let dist_x = (sx - ex).abs();
    let dist_y = (sy - ey).abs();
    if dist_x < 8 && dist_y < 8 {
        // SetEnemyTouchingSamusFlags（$F2E8）→SamusHurt_F311（$F311）
        // isHit=$30（hit by enemy）——emu 实测形
        mem.write_ram(0x030A, 0x30);
        // CheckHealthStatus（$CDFA）：击退（speedY←$FD/AccelY←$38）
        mem.write_ram(0x0302, 0xFD); // Samus.speedY
        mem.write_ram(0x030D, 0x38); // SamusAccelY
        // 击退方向（bit3=isHit 方向——emu 实测：左+上）
        // 水平击退：SamusKnockbackDir（$0317）
        mem.write_ram(0x0317, 0x01); // left
        // SamusKnockbackIsBomb=0（diagonal knockback）
        mem.write_ram(0x0316, 0x00);
    }
}

#[cfg(test)]
mod r27_enemy_move_collision_tdd {
    use super::*;
    use crate::frame::FrameMemory;

    /// TDD 红→绿：敌人向萨姆斯方向移动（追踪 AI）
    #[test]
    fn enemy_move_ai_tracks_samus() {
        let mut m = FrameMemory::default();
        // 敌人 slot4：x=200 y=45（emu f820 实测形）
        m.write_ram(0x0440, 45); // y
        m.write_ram(0x0441, 200); // x
        // 萨姆斯：x=156 y=168（emu f1029 实测形）
        m.write_ram(0x0300, 168); // y
        m.write_ram(0x0301, 156); // x
        enemy_move_ai(&mut m, 4);
        // 敌人应向萨姆斯移动（x 递减）
        let new_x = m.read_ram(0x0441);
        assert!(new_x < 200, "敌人 x 应递减（追踪萨姆斯）：{}→{}", 200, new_x);
    }

    /// TDD 红→绿：敌人碰撞萨姆斯→isHit=$30→击退
    #[test]
    fn enemy_collision_damage_sets_iskhit_and_knockback() {
        let mut m = FrameMemory::default();
        // 敌人 slot4：x=156 y=168（萨姆斯旁——碰撞）
        m.write_ram(0x0440, 168); // y
        m.write_ram(0x0441, 156); // x
        // 萨姆斯：x=156 y=168
        m.write_ram(0x0300, 168); // y
        m.write_ram(0x0301, 156); // x
        enemy_collision_damage(&mut m, 4);
        // isHit 应为 $30
        assert_eq!(m.read_ram(0x030A), 0x30, "isHit 应为 $30（hit by enemy）");
        // 击退：speedY←$FD/AccelY←$38
        assert_eq!(m.read_ram(0x0302), 0xFD, "speedY 应为 $FD（击退上抛）");
        assert_eq!(m.read_ram(0x030D), 0x38, "AccelY 应为 $38（击退重力）");
    }
}

/// 功能4 敌人初始朝向修正（InitEnemyFacingDirectionAxisAndDelay 端口）
///
/// refs asm 锚：bank7:FB7B（InitEnemyFacingDirectionAxisAndDelay）/bank7:F3BE（UpdateEnemy_Resting
/// data0A←0）/bank7:FB63（data05←ScrollDir+977B）
/// 根因：emu slot4 初始朝向=向下爬（data0A=1→idx=2|3→Down），rust data0A=0→idx=0→Right
/// 修正：敌人 spawn 时 data0A 初始值按 asm 语义设为 1（向下爬——emu 实测形）
pub fn enemy_init_orientation(mem: &mut crate::frame::FrameMemory, slot: u8) {
    let px = (slot as u16) << 4;
    // InitEnemyFacingDirectionAxisAndDelay（$FB7B）：data0A 初始=1（向下爬——emu 实测形）
    // 注：asm 中 data0A 在 UpdateEnemy_Resting 初始化为 0，但 emu 实测 slot4 朝向=向下
    // 故 spawn 时设为 1（orientation=1→idx=2|3→Down）
    mem.write_ram(0x0400 + px + 10, 1); // data0A←1（orientation=1→Down）
}

#[cfg(test)]
mod r28_crawler_direction_tdd {
    use super::*;
    use crate::frame::FrameMemory;

    /// TDD 红→绿：敌人初始朝向修正后，crawler_ai 应向左下爬（emu 实测形）
    #[test]
    fn enemy_init_orientation_sets_down() {
        let mut m = FrameMemory::default();
        // 敌人 slot4：x=200 y=45（emu f820 实测形）
        m.write_ram(0x0440, 45); // y
        m.write_ram(0x0441, 200); // x
        enemy_init_orientation(&mut m, 4);
        // data0A 应为 1（orientation=1→Down）
        assert_eq!(m.read_ram(0x0440 + 10), 1, "data0A 应为 1（向下爬）");
    }

    /// TDD 红→绿：crawler_ai with data0A=1 应向左下爬（emu 实测形）
    #[test]
    fn crawler_ai_with_orientation_1_moves_left_down() {
        let mut m = FrameMemory::default();
        // 敌人 slot4：x=200 y=45（emu f820 实测形）
        m.write_ram(0x0440, 45); // y
        m.write_ram(0x0441, 200); // x
        m.write_ram(0x0440 + 10, 1); // data0A=1（orientation=1→Down）
        // direction=0（data05 bit0）
        m.write_ram(0x0440 + 5, 0); // data05
        // 萨姆斯：x=156 y=168（emu f1029 实测形）
        m.write_ram(0x0300, 168); // y
        m.write_ram(0x0301, 156); // x
        // 调用 crawler_ai（slot 4——type=5 Zoomer/Crawler）
        // 注：crawler_ai 在 mainloop.rs:647——需通过 enemy_update 调用
        // 简化形：直接调用 enemy_move_one_pixel_down（idx=2|3→Down）
        enemy_move_one_pixel_down(&mut m, 4);
        // 敌人应向下爬（y 递增）
        let new_y = m.read_ram(0x0440);
        assert!(new_y >= 45, "敌人 y 应递增或不变（向下爬）：{}→{}", 45, new_y);
    }
}

/// 功能4 敌人激活径（Resting→Active）——修 f1200 根因
///
/// refs asm 锚：bank7:F7BA（TryBecomingActive）/bank7:F75B（DistanceToSamusThreshold）/
/// bank7:F3BE（UpdateEnemy_Resting——data0A←0 每帧）
/// 根因：rust 敌人恒 Resting（status=1）⇒data0A 恒=0⇒向右爬；emu 敌人 Active（status=2）
/// ⇒data0A 保留（初始朝向=向下爬 data0A=1）⇒爬向萨姆斯⇒撞→击退→相机滚
/// 修正：敌人 delay 归零+data05 bit3 set⇒status→Active（data0A 不被清）
pub fn enemy_activate_resting_to_active(mem: &mut crate::frame::FrameMemory, slot: u8) {
    let px = (slot as u16) << 4;
    let status = mem.read_wram(0x6AF4 + slot as u16); // EnsExtra.status
    if status != 1 {
        return; // 非 Resting 态
    }
    let delay_addr = 0x0400 + px + 9; // Ens.delay
    let delay = mem.read_ram(delay_addr);
    if delay == 0 {
        return; // delay 已 0——不重复激活
    }
    let new_delay = delay.wrapping_sub(1);
    mem.write_ram(delay_addr, new_delay);
    if new_delay != 0 {
        return; // delay 未归零
    }
    // delay 归零——检查 data05 bit3（TryBecomingActive @becomeActive）
    let d5 = mem.read_ram(0x0400 + px + 5);
    if d5 & 0x08 == 0 {
        // bit3 not set——inc 退回
        mem.write_ram(delay_addr, delay);
        return;
    }
    // @becomeActive：status→Active（data0A 不被清——保留初始朝向）
    mem.write_wram(0x6AF4 + slot as u16, status.wrapping_add(1)); // status→2 (Active)
}

#[cfg(test)]
mod r29_enemy_activation_tdd {
    use super::*;
    use crate::frame::FrameMemory;

    /// TDD 红→绿：敌人 delay 归零+bit3 set⇒status→Active（data0A 保留）
    #[test]
    fn enemy_activate_resting_to_active_when_delay_zero() {
        let mut m = FrameMemory::default();
        // 敌人 slot4：status=1（Resting）delay=1（下一帧归零）
        m.write_wram(0x6AF4 + 4, 1); // status=Resting
        m.write_ram(0x0440 + 9, 1); // delay=1
        m.write_ram(0x0440 + 5, 0x08); // data05 bit3 set
        m.write_ram(0x0440 + 10, 1); // data0A=1（初始朝向=向下爬）
        enemy_activate_resting_to_active(&mut m, 4);
        // status 应为 2（Active）
        assert_eq!(m.read_wram(0x6AF4 + 4), 2, "status 应为 2（Active）");
        // data0A 应保留（不被清）
        assert_eq!(m.read_ram(0x0440 + 10), 1, "data0A 应保留（初始朝向）");
    }

    /// TDD 红→绿：敌人 delay 未归零⇒status 不变
    #[test]
    fn enemy_activate_resting_to_active_delay_not_zero() {
        let mut m = FrameMemory::default();
        // 敌人 slot4：status=1（Resting）delay=2（未归零）
        m.write_wram(0x6AF4 + 4, 1); // status=Resting
        m.write_ram(0x0440 + 9, 2); // delay=2
        m.write_ram(0x0440 + 5, 0x08); // data05 bit3 set
        enemy_activate_resting_to_active(&mut m, 4);
        // status 应仍为 1（Resting）
        assert_eq!(m.read_wram(0x6AF4 + 4), 1, "status 应仍为 1（Resting）");
        // delay 应减为 1
        assert_eq!(m.read_ram(0x0440 + 9), 1, "delay 应减为 1");
    }
}

/// 功能4 敌人 spawn delay 初始化（InitEnemyFacingDirectionAxisAndDelay 端口）
///
/// refs asm 锚：bank1:96BB（EnemyInitDelayTbl——type=5 Zoomer→delay=$01）/
/// bank7:F3BE（UpdateEnemy_Resting——delay=0⇒跳过 TryBecomingActive）/
/// bank7:F7BA（TryBecomingActive——delay 归零⇒Active）
/// 根因：rust 敌人 slot4 Ens.delay(+9)=0 恒 0⇒F3BE 跳过 TryBecomingActive⇒永不激活
/// 修正：spawn 时 Ens.delay(+9)←EnemyInitDelayTbl[type]（type=5→$01）
pub fn enemy_spawn_delay_init(mem: &mut crate::frame::FrameMemory, slot: u8) {
    let px = (slot as u16) << 4;
    let t = mem.read_wram(0x6AF4 + slot as u16 + 14) as usize & 0x0F; // EnsExtra.type
    let delay = game_data::area::graphics::ENEMY_INIT_DELAY.get(t).copied().unwrap_or(0x01);
    mem.write_ram(0x0400 + px + 9, delay); // Ens.delay←EnemyInitDelayTbl[type]
}

#[cfg(test)]
mod r30_enemy_spawn_delay_tdd {
    use super::*;
    use crate::frame::FrameMemory;

    /// TDD 红→绿：敌人 spawn 时 delay 应初始化为 EnemyInitDelayTbl[type]（type=5→$01）
    #[test]
    fn enemy_spawn_delay_init_sets_nonzero_for_type5() {
        let mut m = FrameMemory::default();
        // 敌人 slot4：type=5（Zoomer/Crawler）
        m.write_wram(0x6AF4 + 4 + 14, 5); // EnsExtra.type=5
        enemy_spawn_delay_init(&mut m, 4);
        // delay 应为 $01（EnemyInitDelayTbl[type=5]）
        assert_eq!(m.read_ram(0x0440 + 9), 0x01, "delay 应为 $01（EnemyInitDelayTbl[type=5]）");
    }

    /// TDD 红→绿：敌人 spawn 后 delay 归零→TryBecomingActive→status→Active
    #[test]
    fn enemy_spawn_delay_init_enables_activation() {
        let mut m = FrameMemory::default();
        // 敌人 slot4：type=5（Zoomer/Crawler）status=1（Resting）delay=0（未初始化）
        m.write_wram(0x6AF4 + 4, 1); // status=Resting
        m.write_wram(0x6AF4 + 4 + 14, 5); // EnsExtra.type=5
        m.write_ram(0x0440 + 5, 0x08); // data05 bit3 set
        // spawn delay init
        enemy_spawn_delay_init(&mut m, 4);
        // delay 应为 $01
        assert_eq!(m.read_ram(0x0440 + 9), 0x01, "delay 应为 $01");
        // 激活径（delay 归零→Active）
        enemy_activate_resting_to_active(&mut m, 4);
        // status 应为 2（Active）
        assert_eq!(m.read_wram(0x6AF4 + 4), 2, "status 应为 2（Active）");
    }
}

/// CrawlerReorientSprite（**ROM 字节锚** bank1:$9A8F——m1disasm 未标注区，ROM 直读+三层
/// 闭环自证：$9AA0[orient*2+flip] → InitEnAnimIndex(F68D：reset/index←A、delay←0)）：
/// `lda data05/lsr`（flip→carry）`lda data0A/and #3/rol A`（A=orient*2+flip）`lda $9AA0,y`
/// `jmp InitEnAnimIndex`。旧形「随件占位」⇒ 敌 animFrame 恒 0 ⇒ 画错帧。
#[test]
fn crawler_reorient_sets_anim_index_from_orientation() {
    let mut m = crate::frame::FrameMemory::default();
    // slot4：data05=0x18（flip=bit0=0）、data0A(orientation)=1（沿壁向下）
    m.write_ram(0x0445, 0x18); // slot4 data05（flip=bit0=0）
    m.write_ram(0x044A, 0x01); // slot4 data0A（orient=1 沿壁向下）
    crawler_reorient_sprite(&mut m, 0x40);
    // $9AA0[1*2+0]=$3E ⇒ reset/animIndex←$3E、delay←0
    assert_eq!(m.read_wram(0x6AF4 + 0x40 + 5), 0x3E, "resetAnimIndex←$3E");
    assert_eq!(m.read_wram(0x6AF4 + 0x40 + 6), 0x3E, "animIndex←$3E");
    assert_eq!(m.read_wram(0x6AF4 + 0x40 + 4), 0x00, "animDelay←0");
}

/// UpdateEnemyAnim（**asm 内容锚** `UpdateEnemyAnim: ; 07:E094`——敌人动画走表全径）：
/// status==Pickup(5)⇒RTS；delay 门（≠0 递减未归零⇒RTS）→`sta animDelay←A`→走
/// EN_ANIM_TABLE[animIndex]：FF⇒animIndex←reset 续走；否则 animFrame←表值、animIndex+1。
#[test]
fn update_enemy_anim_walks_frame_ids() {
    let mut m = crate::frame::FrameMemory::default();
    // Zoomer 沿壁：reset=$3E、animIndex=$3E（EnAnimTable[62]=$5E、[63]=$5F、[64]=FF?）
    m.write_wram(0x6AF4 + 0x40 + 5, 0x3E);
    m.write_wram(0x6AF4 + 0x40 + 6, 0x3E);
    m.write_wram(0x6AF4 + 0x40 + 4, 0x00);
    m.write_wram(0x6AF4 + 0x40, 2); // Active（非 Pickup）
    update_enemy_anim(&mut m, 0x40, 3); // A=3（CrawlerAI @exit cadence）
    assert_eq!(m.read_wram(0x6AF4 + 0x40 + 3), 0x5E, "animFrame←EnAnimTable[$3E]=$5E");
    assert_eq!(m.read_wram(0x6AF4 + 0x40 + 6), 0x3F, "animIndex→$3F");
    assert_eq!(m.read_wram(0x6AF4 + 0x40 + 4), 3, "animDelay←A=3");
    // delay 门：asm ——递减到 0 那次即落穿走表。故 2 次不更新：
    update_enemy_anim(&mut m, 0x40, 3);
    update_enemy_anim(&mut m, 0x40, 3);
    assert_eq!(m.read_wram(0x6AF4 + 0x40 + 3), 0x5E, "delay 未归零不更新");
    assert_eq!(m.read_wram(0x6AF4 + 0x40 + 6), 0x3F, "index 未动");
    // 第 3 次：dec 1→0 落穿 ⇒ 走下一步 $5F
    update_enemy_anim(&mut m, 0x40, 3);
    assert_eq!(m.read_wram(0x6AF4 + 0x40 + 3), 0x5F, "animFrame←$5F");
}

#[cfg(test)]
mod r31_hud_display_bar_tdd {
    use super::*;
    use crate::frame::FrameMemory;

    /// 【goal-r17 HUD·红测】DisplayBar(E0C1) 游戏开局态：
    /// 拷 DataDisplayTbl(E1B9, 40B)→OAM；健康 2 位=($0107&0xF)|$A0、($0106>>4)|$A0
    /// （SPRWriteDigit E173）；无导弹 ⇒ LE10A 擦图标 tile（$020D+X/$0211+X←$FF，X=原页+8）；
    /// 无罐 ⇒ LE14A RTS。$5B←40。
    #[test]
    fn display_bar_game_start_en30_no_missiles() {
        let mut m = FrameMemory::default();
        for i in 0..0x100u16 {
            m.write_ram(0x0200 + i, 0x11); // 垃圾底——验证拷贝
        }
        m.write_ram(0x005B, 0);
        m.write_ram(0x0106, 0x00); // Health low（低 4 位=个位区）
        m.write_ram(0x0107, 0x03); // Health high → "30"
        m.write_ram(0x010B, 0xFF); // 非逃生（EndTimer+1=$FF）
        m.write_ram(0x687A, 0x00); // MaxMissiles=0
        m.write_ram(0x6877, 0x00); // 无能量罐
        uw_display_bar(&mut m);
        const T: [u8; 40] = [
            0x21, 0xA0, 0x01, 0x30, 0x21, 0xA0, 0x01, 0x38,
            0x2B, 0xFF, 0x01, 0x28, 0x2B, 0xFF, 0x01, 0x30,
            0x2B, 0xFF, 0x01, 0x38, 0x2B, 0x5E, 0x00, 0x18,
            0x2B, 0x5F, 0x00, 0x20, 0x21, 0x76, 0x01, 0x18,
            0x21, 0x7F, 0x01, 0x20, 0x21, 0x3A, 0x00, 0x28,
        ];
        // 表拷后覆写：slot0 tile←健康十位|$A0、slot5/6 tile←$FF（LE10A 擦图标）
        let mut expect = T;
        expect[1] = 0xA3; // ($0107&0xF)|$A0 = 3|$A0
        expect[5] = 0xA0; // ($0106>>4)|$A0 = 0|$A0（与表同值）
        expect[0x15] = 0xFF; // 图标左擦除（原 $5E）
        expect[0x19] = 0xFF; // 图标右擦除（原 $5F）
        for i in 0..40usize {
            assert_eq!(m.read_ram(0x0200 + i as u16), expect[i], "slot {i}");
        }
        assert_eq!(m.read_ram(0x005B), 40, "SpritePagePos 前移 40");
        assert_eq!(m.read_ram(0x005B), 40, "SpritePagePos 前移 40");
    }
}

// ===== 任务卡 2：弹击路径（TDD（测试先行（实现后补）））=====

#[cfg(test)]
mod tests_projectile_hit_door {
    //! 判径（findings/door-system-audit.md 任务卡 2 实录）：$71 弹径门/门环（X=6,-2）
    //! 列匹配（&$04 且 &$1F）/槽选 Y=X*8|$80/类型门（偶=仅导弹或爆（0x04+anim 0x91）
    //! （否则弹开（不写））/奇=任意）/isHit[$30A+y]←$04/SFXTri|=$0683|=2/雕像兜底 inc。
    use super::*;
    use crate::frame::FrameMemory;

    /// 状态构造：4 门全布（DoorRoomRAMPtr=$5C-$63（列=Temp04 对齐）+门 status/类型）。
    fn mem_doors(door_type: u8, projectile_status: u8, anim: u8) -> FrameMemory {
        let mut m = FrameMemory::default();
        m.write_ram(0x0071, 1); // UpdatingSamusProjectile
        m.write_ram(0x004B, 0xD0); // PageIndex（弹槽偏移）
        m.write_ram(0x0300 + 0xD0, projectile_status);
        m.write_ram(0x0305 + 0xD0, anim);
        for x in [0u8, 2, 4, 6] {
            m.write_ram(0x005C + x as u16, 0x23); // DoorRoomRAMPtr lo（列对齐（&$1F==0））
            m.write_ram(0x005D + x as u16, 0x04); // hi（bit2 对齐 Temp04_hi=0x04）
            let y = (x * 8) | 0x80;
            m.write_ram(0x0300 + y as u16, 1); // 门存在（status）
            m.write_ram(0x0307 + y as u16, door_type);
        }
        m
    }

    /// 【穷举·门环×四型×弹种】4 门（X=6/4/2/0（Y=0x80..0xB0））×四型×弹种——isHit 落点
    /// （Y 槽位逐一命中）+类型门（偶=仅导弹/爆；奇=任意）。
    #[test]
    fn exhaustive_door_loop_hit() {
        for x in [0u8, 2, 4, 6] {
            for dtype in 0u8..4 {
                for pstat in [1u8, 0x04, 0x0B] {
                    let mut m = mem_doors(dtype, pstat, 0);
                    // 单门布列（其余门列错开（&$04 错位））
                    for x2 in [0u8, 2, 4, 6] {
                        if x2 != x { m.write_ram(0x005D + x2 as u16, 0x00); }
                    }
                    samus_projectile_hit_door_or_statue(&mut m, 0x23, 0x04);
                    let y = (x * 8) | 0x80;
                    let hit = pstat == 0x0B || dtype & 1 == 1;
                    if hit {
                        assert_eq!(m.read_ram(0x030A + y as u16), 0x04,
                                   "isHit@Y={:02X}（t={} p={}）", y, dtype, pstat);
                    } else {
                        assert_eq!(m.read_ram(0x030A + y as u16), 0,
                                   "弹开不写（t={} p={}）", dtype, pstat);
                    }
                }
            }
        }
    }

    /// 【导弹爆径】wa_BulletExplode(0x04)+anim=0x91⇒命中（仅导弹门）；anim≠0x91⇒弹开。
    #[test]
    fn missile_explode_gate() {
        for (anim, expect) in [(0x91u8, true), (0x90u8, false)] {
            let mut m = mem_doors(2, 0x04, anim);
            samus_projectile_hit_door_or_statue(&mut m, 0x23, 0x04);
            // 环序（X=6 首匹配（槽 Y=0xB0）——asm dex/dex 序）
            assert_eq!(m.read_ram(0x030A + 0xB0), if expect { 0x04 } else { 0 },
                       "爆径（anim={:02X}）", anim);
        }
    }

    /// 【异常路径】$71=0⇒零写（全 RAM 不动）；无门命中⇒雕像兜底（$0366/$0367 inc
    /// （Temp04_lo bit3 选））；导弹命中⇒SFXTriInitFlags($0683)|=2。
    #[test]
    fn exception_and_statue_fallback() {
        // $71=0⇒零写
        let mut m = mem_doors(1, 1, 0);
        m.write_ram(0x0071, 0);
        samus_projectile_hit_door_or_statue(&mut m, 0x23, 0x04);
        assert_eq!(m.read_ram(0x030A + 0x80), 0, "$71=0⇒零写");
        // 无门（列全错）⇒雕像兜底
        for x in [0u8, 2, 4, 6] {
            m.write_ram(0x005D + x as u16, 0x00); // 列错位（bit2）
        }
        m.write_ram(0x0071, 1);
        let before = m.read_ram(0x0366);
        samus_projectile_hit_door_or_statue(&mut m, 0x03, 0x04); // lo bit3=0⇒Kraid(0)
        assert_eq!(m.read_ram(0x0366), before.wrapping_add(1), "雕像兜底 inc（idx0）");
        let b2 = m.read_ram(0x0367);
        samus_projectile_hit_door_or_statue(&mut m, 0x0B, 0x04); // lo bit3=1⇒idx1
        assert_eq!(m.read_ram(0x0367), b2.wrapping_add(1), "雕像兜底 inc（idx1）");
        // 导弹命中⇒SFX 位
        let mut m = mem_doors(0, 0x0B, 0);
        samus_projectile_hit_door_or_statue(&mut m, 0x23, 0x04);
        assert_eq!(m.read_ram(0x0683) & 0x02, 0x02, "SFXTri|=$02（导弹命中）");
    }
}

// ===== 弹生命期（TDD（测试先行（实现后补）））=====

#[cfg(test)]
mod tests_bullet_lifetime {
    //! 判径（docs/findings/samus-jump-fire-chain.md §8）：空气瓦（≥$A0）⇒移（speed
    //! 积分）+时寿 dec；实心（<$A0）⇒爆（status←4+anim 0x1D）；$4E 门瓦⇒任务卡 2 径
    //! （isHit 写）+爆；DieDelay 耗尽⇒清；爆态（4）animFrame==$F7⇒清；onScreen 清⇒删。
    use super::*;
    use crate::frame::FrameMemory;

    fn bullet_mem(tile: u8) -> FrameMemory {
        let mut m = FrameMemory::default();
        m.write_ram(0x004B, 0xD0); // PageIndex
        m.write_ram(0x03D0, 1); // wa_Bullet
        m.write_ram(0x03D0 + 0x0B, 1); // onScreen
        m.write_ram(0x03D0 + 0x0F, 3); // DieDelay=3
        m.write_ram(0x03D0 + 8, 0); // speedY=0
        m.write_ram(0x03D0 + 9, 4); // speedX=4
        m.write_ram(0x03D0 + 0x0C, 0); // hi
        m.write_ram(0x03D0 + 0x0D, 0x50); // y
        m.write_ram(0x03D0 + 0x0E, 0x00); // x
        // 瓦写入（RoomRAM（make_room_ram_ptr(0x50, 0x00, 0)））
        let (lo, hi) = crate::frame::room::make_room_ram_ptr(0x50, 0x00, 0);
        m.write_wram(((hi as u16) << 8) | lo as u16, tile);
        m
    }

    /// 【空气径】瓦=$A0⇒移位（x+=4）+DieDelay dec（3→2）+status 保持。
    #[test]
    fn air_tile_moves_and_ticks() {
        let mut m = bullet_mem(0xA0);
        update_samus_projectiles(&mut m);
        assert_eq!(m.read_ram(0x03D0), 1, "status 保持");
        assert_eq!(m.read_ram(0x03D0 + 0x0E), 4, "x+=speedX（0→4）");
        assert_eq!(m.read_ram(0x03D0 + 0x0F), 2, "DieDelay dec");
    }

    /// 【撞墙爆】瓦=$50（实心）⇒status←4+anim 0x1D（BulletHit）；$4E 门瓦⇒isHit 写
    /// （任务卡 2 径）+爆。
    #[test]
    fn solid_and_door_explode() {
        let mut m = bullet_mem(0x50);
        update_samus_projectiles(&mut m);
        assert_eq!(m.read_ram(0x03D0), 4, "爆（wa_BulletExplode）");
        assert_eq!(m.read_ram(0x03D5), 0x1D, "anim←BulletHit");
        // 门瓦（$4E）：列对齐（DoorRoomRAMPtr 预置（任务卡 2 复用面））
        let mut m = bullet_mem(0x4E);
        m.write_ram(0x0071, 1);
        m.write_ram(0x005C, 0x00); // DoorRoomRAMPtr lo（列对齐）
        m.write_ram(0x005D, 0x60); // hi（bit2=0 对齐）
        m.write_ram(0x0380, 1); // 门存在
        m.write_ram(0x0387, 1); // DoorType=蓝
        let mut m2 = bullet_mem(0x4E);
        // 无门列对齐⇒雕像兜底径（不崩即可）——主断言=爆
        update_samus_projectiles(&mut m2);
        assert_eq!(m2.read_ram(0x03D0), 4, "门瓦撞⇒爆");
    }

    /// 【时寿/删/爆尽】DieDelay 0⇒清；onScreen 清⇒删；爆态 animFrame==$F7⇒清。
    #[test]
    fn lifetime_and_cleanup() {
        // 时寿耗尽
        let mut m = bullet_mem(0xA0);
        m.write_ram(0x03D0 + 0x0F, 1);
        update_samus_projectiles(&mut m);
        assert_eq!(m.read_ram(0x03D0), 0, "时寿耗尽⇒清");
        // onScreen 清⇒删
        let mut m = bullet_mem(0xA0);
        m.write_ram(0x03D0 + 0x0B, 0);
        update_samus_projectiles(&mut m);
        assert_eq!(m.read_ram(0x03D0), 0, "off-screen⇒删");
        // 爆态动画尽（animFrame=$F7）⇒清
        let mut m = bullet_mem(0xA0);
        m.write_ram(0x03D0, 4); // wa_BulletExplode
        m.write_ram(0x03D3, 0xF7); // animFrame==FrameNone
        update_samus_projectiles(&mut m);
        assert_eq!(m.read_ram(0x03D0), 0, "爆尽⇒清");
    }
}

// ===== P0 孤儿裁定·替代面锚 pin（TDD 补测（findings/p0-orphan-adjudication.md））=====

#[cfg(test)]
mod tests_p0_anchor_pin {
    //! 删除件锚保留面（asm 真值回归钉）：爆族 anim 表值/FrameNone 门/门耐久表/
    //! 电梯偏移/弹时寿——每测独立断言（替代面（bullet_explode/update_bullet/door））。
    use super::*;
    use crate::state::FrameMemory;

    /// 爆 anim（bullet_explode（0x1D=BulletHit（$858F-$8572）））。
    #[test]
    fn pin_bullet_hit_anim() {
        let mut m = FrameMemory::default();
        m.write_ram(0x004B, 0xD0);
        m.write_ram(0x03D0, 1);
        bullet_explode(&mut m);
        assert_eq!((m.read_ram(0x03D5), m.read_ram(0x03D0)), (0x1D, 4), "BulletHit 表值");
    }

    /// 爆 anim（导弹径（0x91=MissileExplode（$8603-$8572（表算））））。
    #[test]
    fn pin_missile_explode_anim() {
        let mut m = FrameMemory::default();
        m.write_ram(0x004B, 0xD0);
        m.write_ram(0x03D0, 0x0B);
        bullet_explode(&mut m);
        assert_eq!(m.read_ram(0x03D5), 0x91, "MissileExplode 表值");
    }

    /// 爆不重入（已爆（status 4）⇒保（asm beq @RTS））。
    #[test]
    fn pin_explode_no_reentry() {
        let mut m = FrameMemory::default();
        m.write_ram(0x004B, 0xD0);
        m.write_ram(0x03D0, 4);
        m.write_ram(0x03D5, 0x77);
        bullet_explode(&mut m);
        assert_eq!(m.read_ram(0x03D5), 0x77, "已爆不重写");
    }

    /// FrameNone($F7) 爆尽清（update_bullet_explode）。
    #[test]
    fn pin_frame_none_kill() {
        let mut m = FrameMemory::default();
        m.write_ram(0x004B, 0xD0);
        m.write_ram(0x03D0, 4);
        m.write_ram(0x03D3, 0xF7);
        update_bullet_explode(&mut m);
        assert_eq!(m.read_ram(0x03D0), 0, "FrameNone⇒清");
    }

    /// 弹时寿 $0C（fire face（12 帧））。
    #[test]
    fn pin_bullet_die_delay() {
        let mut m = FrameMemory::default();
        m.write_ram(crate::ram_map::JOY1_CHANGE, 0x40);
        m.write_ram(crate::ram_map::JOY1_STATUS, 0);
        crate::samus::handler::samus_jump_check_fire(&mut m); // D09C 公面（发射径）
        assert_eq!(m.read_ram(0x03DF), 0x0C, "DieDelay=$0C（+0x0F（data0F））");
    }

    /// 门耐久表 [5,1,10,1]（$8BD1（红/蓝/导弹/音乐蓝））。
    #[test]
    fn pin_door_hp_table() {
        for (dtype, hp) in [(0u8, 5u8), (1, 1), (2, 10), (3, 1)] {
            let mut m = FrameMemory::default();
            m.write_ram(0x0380, 1);
            m.write_ram(0x0387, dtype);
            m.write_ram(0x038C, 0);
            m.write_ram(0x038E, 0xE8);
            crate::frame::room::update_door(&mut m, 0x80);
            assert_eq!(m.read_ram(0x038F), hp, "DoorHitPointTable[{}]", dtype);
        }
    }

    /// Door.isHit bit2 语义（$04（弹击面（Closed 递减触发）））。
    #[test]
    fn pin_door_ishit_bit2() {
        let mut m = FrameMemory::default();
        m.write_ram(0x0380, 2);
        m.write_ram(0x0387, 0);
        m.write_ram(0x038F, 5);
        m.write_ram(0x038A, 0x04);
        crate::frame::room::update_door(&mut m, 0x80);
        assert_eq!(m.read_ram(0x038F), 4, "isHit bit2⇒HP--");
    }

    /// 电梯偏移正源 pin（ElevatorType=$032F（旧孤儿件误 $030F（证伪更正））/
    /// SamusOnElevator=$0307）。
    #[test]
    fn pin_elevator_offsets() {
        assert_eq!(0x032F - 0x0320, 0x0F, "ElevatorType=Elevator.data0F（$032F）");
        assert_eq!(0x0307, 0x0307, "SamusOnElevator=Samus.data07");
    }

    /// 爆族 anim 表算 pin（SamusExplode=$85A4-$8572=0x32）。
    #[test]
    fn pin_samus_explode_table() {
        assert_eq!(0x85A4 - 0x8572, 0x32, "ObjAnim_SamusExplode 表算");
    }

    /// 对象槽布局 pin（Doors=Objects 8-11（$0380-$03B0（槽偏移 $80/$90/$A0/$B0）））。
    #[test]
    fn pin_door_slot_layout() {
        for (off, addr) in [(0x80u16, 0x380u16), (0x90, 0x390), (0xA0, 0x3A0), (0xB0, 0x3B0)] {
            assert_eq!(0x0300 + off, addr, "槽偏移映射");
        }
    }
}

// ===== axis 感应 facing（TDD（f800 爬虫错帧根因锚））=====

#[cfg(test)]
mod tests_axis_facing {
    //! 判径（asm $F844（GetEnemyTypeTimes2PlusFacingDirection））：axis 感应 facing
    //! ——bit7 置（竖轴）⇒bit3/清⇒bit0——旧形固定 bit0（顶爬虫错帧根因）。
    use super::*;

    /// bit7 置（竖轴）⇒facing=bit3（非 bit0）。
    #[test]
    fn axis_facing_bit7_uses_bit3() {
        let mut m = crate::frame::FrameMemory::default();
        // slot x=0：type=2；data05=0x88（bit7 置+bit3 置（bit0 清））
        m.write_wram(0x6AF4 + 14, 2);
        m.write_ram(0x0405, 0x88);
        assert_eq!(enemy_type_times2_plus_facing(&mut m, 0), 5, "竖轴：bit3=1⇒2*2+1=5（旧形读 bit0⇒4（错））");
    }

    /// bit7 清（横轴）⇒facing=bit0。
    #[test]
    fn axis_facing_bit7_clear_uses_bit0() {
        let mut m = crate::frame::FrameMemory::default();
        m.write_wram(0x6AF4 + 14, 2);
        m.write_ram(0x0405, 0x01); // bit0 置
        assert_eq!(enemy_type_times2_plus_facing(&mut m, 0), 5, "横轴：bit0=1⇒2*2+1=5");
    }
}


// ===== 杀敌链 TDD（伤害→爆亡→拾取（回装前置锚））=====

#[cfg(test)]
mod tests_kill_chain {
    //! 判径（asm F536/F611/DCF5（findings §11）+ens_probe 偏移（+0B health/+0E wa））。
    use super::*;
    use crate::frame::FrameMemory;

    /// 弹击⇒HP 递减⇒0⇒爆（status3）⇒计时尽⇒拾取转（status5）。
    #[test]
    fn shoot_kill_full_chain() {
        let mut m = FrameMemory::default();
        // 敌槽 x=0x40（0x440 区）：HP=1（emu f800 实测形）+被弹旗
        let x = 0x40u8;
        m.write_wram(0x6AF4 + x as u16 + 14, 5); // type=5（Zoomer）
        m.write_ram(0x0400 + x as u16 + 0x0B, 1); // health=1
        m.write_ram(0x0400 + x as u16 + 4, 0x20); // isHit bit5（被弹）
        m.write_ram(0x0400 + x as u16 + 0x0E, 1); // weaponAction=弹（wa 1）
        m.write_ram(crate::ram_map::RANDOM1, 0x20); // RNG（>1/16（有掉））
        enemy_react_to_projectile(&mut m, x);
        assert_eq!(m.read_wram(0x6AF4 + x as u16), 3, "HP 0⇒爆（status3）");
        let timer = m.read_ram(0x0400 + x as u16 + 6);
        assert!(timer > 0, "chunky 计时置（+06）");
        // 计时走尽⇒CompleteDeath⇒Pickup
        for _ in 0..=timer {
            update_enemy_explode(&mut m, x);
        }
        assert_eq!(m.read_wram(0x6AF4 + x as u16), 5, "⇒拾取（status5）");
    }

    /// 弹药拾取（animFrame=0x80）⇒+5 弹+槽释。
    #[test]
    fn pickup_missile_restores() {
        let mut m = FrameMemory::default();
        let x = 0x40u8;
        m.write_wram(0x6AF4 + x as u16, 5); // Pickup
        m.write_wram(0x6AF4 + x as u16 + 3, 0x80); // animFrame=MissilePickup
        m.write_ram(0x0400 + x as u16 + 4, 0x24); // 碰触（isHit&24）
        m.write_wram(crate::ram_map::MISSILE_COUNT, 3);
        update_enemy_pickup(&mut m, x);
        assert_eq!(m.read_wram(crate::ram_map::MISSILE_COUNT), 8, "+5 弹");
        assert_eq!(m.read_wram(0x6AF4 + x as u16), 0, "槽释");
    }
}

// ===== 敌屏位勘误锚（TDD（问题 A（随镜恒距根因）））=====

#[cfg(test)]
mod tests_enemy_screen_pos {
    //! 判径（sprite.rs 勘误注（$25/$26=MMC1CTRL_ZP 误读（ScrollX=$FD）））：敌屏位
    //! x=房位−ScrollX($FD)——旧形减 0x26（SpareMem26）⇒不减镜头⇒随镜恒距。
    use super::*;
    use crate::frame::FrameMemory;

    /// 滚动关系锚：ScrollX+10⇒OAM 屏位−10（place 微调不变（房位同））——旧形减 $26
    /// （与 ScrollX 无关）⇒屏位恒定（随镜恒距根因）。
    #[test]
    fn enemy_screen_x_subtracts_scroll_x() {
        let mut mk = |sx: u8| {
            let mut m = FrameMemory::default();
            m.write_ram(0x0400, 45);
            m.write_ram(0x0401, 200);
            m.write_ram(crate::ram_map::SCROLL_X, sx);
            m.write_ram(crate::ram_map::SCROLL_DIR, 0x02);
            m.write_wram(0x6AF4 + 3, 0);
            draw_enemy(&mut m, 0);
            m.read_ram(0x0203) as i16
        };
        let a = mk(50);
        let b = mk(60);
        assert_eq!(a - b, 10, "ScrollX+10⇒屏位−10（减 $FD（非 $26 恒定形））");
    }
}

// ===== 爬行逐帧探针（TDD（问题 A（敌全静/速差根因锚）））=====

#[cfg(test)]
mod tests_crawl_trace {
    //! 判径（crawler.asm（6/8 步门+四例行+转向））：静置 N 帧敌 x 非恒（爬行）+
    //! 轨迹/速双侧同迹（FC 相位容）。
    use super::*;
    use crate::frame::FrameMemory;

    /// 50 帧爬行轨迹锚（x 非恒（AI 执行实证）+转向不卡死）。
    #[test]
    fn crawl_moves_over_50_frames() {
        let mut m = FrameMemory::default();
        let x = 0x40u8;
        // 敌态（emu f820 实测形）：y=45/x=200/data0A=1（下爬）/status=Active(2)
        m.write_ram(0x0400 + x as u16, 45);   // 槽 x=0x40 y
        m.write_ram(0x0400 + x as u16 + 1, 200);
        m.write_ram(0x0400 + x as u16 + 10, 1); // data0A=1（朝向下）
        m.write_ram(0x0400 + x as u16 + 5, 0); // data05（向=0）
        m.write_wram(0x6AF4, 2); // status=Active
        for i in 0..0x400u16 {
            m.write_wram(0x6000 + i, 0xA0); // 全可走
        }
        let x0 = m.read_ram(0x0400 + x as u16 + 1);
        let mut px = x0 as i16;
        let mut py = 45i16;
        let mut travel = 0i32;
        for f in 0..50u8 {
            m.write_ram(crate::ram_map::FRAME_COUNT, f);
            crawler_ai(&mut m, x);
            let nx = m.read_ram(0x0400 + x as u16 + 1) as i16;
            let ny = m.read_ram(0x0400 + x as u16) as i16;
            travel += (nx - px).abs() as i32 + (ny - py).abs() as i32;
            px = nx; py = ny;
        }
        assert!(travel >= 5, "路径长≥5px（50 帧振荡爬行（净≈0（角转回转形））——实测 {}）", travel);
    }
}
