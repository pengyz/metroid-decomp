//! 调度：主循环入口、主例程表分派、暂停/密码/死亡/换区、区域初始化、计时与音乐起播。
//!
//! 由 `frame/mainloop.rs` 原样搬出（纯搬移）。asm 根：`MainLoop ; 07:C0BC`、
//! `GoMainRoutine ; 07:C114`、`MainRoutinePtrTable 07:C141`、`UpdateTimer ; 07:C266`、
//! `IsEngineRunning ; 07:CA18`。

use crate::frame::mainloop::*;
use crate::frame::FrameMemory;
use crate::ram_map;
use crate::ram_map::*;

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
    // 【goal-轮56 证伪更正（§3.6）】FC 前置增量（轮26 形重试）L4 判定证伪——match 骤降
    // （1/13 等 vs 基线 55/97）⇒回退。金腿 FC=+1 恒差（ram.hex 直证）=**症状非根**——
    // FC 依赖行为（gate/toggle/相位）随移全错；真根=FC 读面各自的相位语义（非全局移）。
    if !mem.title_no_return && !mem.fc_inced_this_tick {
        mem.write_ram(FRAME_COUNT, mem.read_ram(FRAME_COUNT).wrapping_add(1));
    }
    mem.title_no_return = false;
    mem.fc_inced_this_tick = false;
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

/// 游戏侧主例程分发（**asm 内容锚**——T45-3 移动臂首靶；C11B+ 面）：
/// START 检查（暂停切换）→`lda MainRoutine($1E)/jsr JumpEngine`→MainRoutinePtrTable
/// 10 项分派。未武装臂=per-entry 爆（移动臂迭代分类面）。
pub fn game_main_routine(mem: &mut FrameMemory) {
    // 【抢救件#2（A 级）】asm `GoMainRoutine: ; 07:C114` @endIf_A 段（prg7_engine.asm:311-338）：
    // `lda Joy1Change / and #BUTTON_START / beq @endIf_B` → 命中后
    // `lda MainRoutine / cmp #_id_GameEngine(3) / beq → PauseMode(5)` 或
    // `cmp #_id_PauseMode(5) / bne @endIf_B`（既非 3 也非 5 ⇒ 忽略 START）→
    // `sta MainRoutine` / `lda GamePaused / eor #$01 / sta GamePaused` / `jsr PauseMusic($CB92)`。
    // **注意顺序**：切换发生在本帧分派**之前**，故同一帧就跑新例程（asm 原序）。
    start_pause_toggle(mem);
    let mr = mem.read_ram(crate::ram_map::MAIN_ROUTINE);
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
            // 【能力移植（2026-10-31）】PowerUps 槽空标初值=**$FF**（SpawnPowerUp（EDFE）
            // `lda #$FF/cmp type` 空槽判/UpdateOnePowerUp 同标——游戏起始面（RAM 填充面
            // 细节随核；观测=开局无物品 ✓）。
            mem.write_ram(0x0748, 0xFF);
            mem.write_ram(0x0750, 0xFF);
            area_init(mem);
            more_init(mem);
        }
        1 => more_init(mem), // MoreInit 独派（mr=01 直设路径——asm 分派表项 1）
        2 => samus_init(mem), // SamusInit（r25 端口——萨姆斯出场态）
        3 => game_engine(mem), // GameEngine（已端口——移动帧环壳）
        8 => samus_intro(mem), // SamusIntro（r25 端口——淡入序+sprite 装载）
        9 => {
            // 【缝会计卡】WaitTimer（$C494——SetTimer 计时等待径）：Timer3≠0⇒出（冻结
            // 保持——能力动画/GameOver 前置窗）；尽⇒NextRoutine($20)→MainRoutine
            // （GameOver(4)/GoPassword(6) 免起乐；否则 StartMusic（D92C）后置——
            // asm :2494-2505 全径）。
            if mem.read_ram(crate::ram_map::TIMER3) == 0 {
                let next = mem.read_ram(0x0020); // NextRoutine（constants_ram:480）
                if next != 4 && next != 6 {
                    start_music(mem); // D92C（区乐/ItemRoom 乐——声链消费面）
                }
                mem.write_ram(crate::ram_map::MAIN_ROUTINE, next);
            }
        }
        4 => prepare_game_over(mem), // PrepareGameOver（$C9A6——死亡链第二段；见函数注释）
        5 => pause_mode(mem),        // PauseMode（$C9B1——暂停退出→密码屏）
        6 => go_password(mem),       // GoPassword（$C9C4——切标题页显示密码）
        7 => increment_routine(mem), // IncrementRoutine（$C155——`inc MainRoutine/rts`）
        _ => {
            let loc = format!("bank07:{:04X}", MAIN_ROUTINE_PTR_TABLE.get(mr as usize).copied().unwrap_or(0));
            crate::unported!(&loc); // 其余态（MoreInit 起——迭代分类）
        }
    }
}

/// PrepareGameOver（**asm 内容锚** `PrepareGameOver: ; 07:C9A6`——`prg7_engine.asm:2124-2130`）：
/// ```text
/// lda #_id_GameOver.b      ; $1C
/// sta TitleRoutine($1F)
/// lda #$00+1
/// sta BankSwitchPending($24)
/// jmp ScreenOff            ; $C439 = `and #~(BG_ON|OBJ_ON)` → WriteAndWait 写影子 → WaitNMIPass 等一 NMI
/// ```
/// **触发链（A 级，全 asm 逐行）**：`GameEngine` 检 `Samus.status==sa_Dead2 ∧ animDelay==0`
/// → `SilenceMusic` → `MotherBrainStatus==$0A ? inc MainRoutine : SetTimer(#$04, x=4)`
/// （prg7:2066-2075，两条死亡径最终都落 **mr=4**）→ `WaitTimer(mr=9)` 等 `Timer3` 尽 →
/// `MainRoutine←NextRoutine(4)`（prg7:1224-1231，`NextRoutine∈{4,6}` 免起乐）→ 本函数。
/// 之后：下一帧帧界 `CheckBankSwitch` 消费 `$24=1` → `InitBank0`（`GameMode←1`）→
/// 标题轴接管 → `TitleRoutine=0x1C GameOver $939E`（GAME OVER 2.6s）→ `0x19 DisplayPassword`
/// → `0x1A WaitForSTART` → `jmp CheckPassword` → 有效密码回 `InitializeGame`。
///
/// **ScreenOff 的落地相位**：asm 只写影子 `PPUMASK_ZP`，真寄存器由 `WriteAndWait` 等的
/// 那个 NMI 里 `WritePPUCtrl`（`sta PPUMASK`）落 —— 与 `initialize_game` 的 ScreenNmiOff
/// 同构（实测不显式写 `$2001` 会晚一帧，见 b92 暗窗定谳），故此处同样双写。
pub fn prepare_game_over(mem: &mut FrameMemory) {
    mem.write_ram(crate::ram_map::TITLE_ROUTINE, 0x1C); // _id_GameOver
    mem.write_ram(crate::ram_map::BANK_SWITCH_PENDING, 0x01); // 切标题页（BankTable[0]=bank0）
    let pm = mem.read_ram(0x00FE) & !0x18; // PPUMASK_ZP &= ~(BG_ON|OBJ_ON)
    mem.write_ram(0x00FE, pm);
    mem.write_io(0x2001, pm); // WriteAndWait → 该 NMI 落真寄存器
}

/// `GoMainRoutine` 的 START 暂停开关（**asm 内容锚** `07:C114` @endIf_A 段，`prg7_engine.asm:311-338`）：
/// `Joy1Change & BUTTON_START` 命中且 `MainRoutine ∈ {3 GameEngine, 5 PauseMode}` 时
/// 双向切换 + `GamePaused ^= 1` + `PauseMusic`；其余态忽略 START（asm `bne @endIf_B`）。
/// 返回是否发生切换（供单测；生产路径不依赖返回值）。
pub fn start_pause_toggle(mem: &mut FrameMemory) -> bool {
    if mem.read_ram(crate::ram_map::JOY1_CHANGE) & 0x10 == 0 {
        return false; // BUTTON_START=%0001_0000（hardware.asm:93）
    }
    let now = mem.read_ram(crate::ram_map::MAIN_ROUTINE);
    let next = match now {
        3 => 5u8, // GameEngine → PauseMode
        5 => 3u8, // PauseMode → GameEngine
        _ => return false,
    };
    mem.write_ram(crate::ram_map::MAIN_ROUTINE, next);
    let gp = mem.read_ram(crate::ram_map::GAME_PAUSED) ^ 1; // `eor #$01`
    mem.write_ram(crate::ram_map::GAME_PAUSED, gp);
    super::sound_hook::pause_music(mem); // `jsr PauseMusic($CB92)`
    true
}

/// PauseMode（**asm 内容锚** `PauseMode: ; 07:C9B1`——`prg7_engine.asm:2139-2166`）：
/// ```text
/// lda Joy2Status($15)
/// and #BUTTON_A|BUTTON_UP   ; $88
/// eor #BUTTON_A|BUTTON_UP
/// bne @RTS                  ; 不是「P2 的 A+UP 同时按住」⇒ 出
/// ldy EndTimer+1($010B) / iny / bne @RTS   ; 逃脱计时器活动 ⇒ 不能退出
/// sta GamePaused            ; A=0（eor 结果）⇒ GamePaused←0
/// inc MainRoutine           ; 5 → 6 = GoPassword
/// ```
/// **这是 mr=6 的唯一产生点**（不是 `NextRoutine`——`NextRoutine` 从不置 6，
/// `WaitTimer` 里对 6 的比较是死分支）。原版语义：暂停画面上按住 **2P 的 A+UP**
/// 即退出到「显示密码」屏。
pub fn pause_mode(mem: &mut FrameMemory) {
    // `and #$88 / eor #$88` ⇒ 全按才为 0
    if mem.read_ram(crate::ram_map::JOY2_STATUS) & 0x88 != 0x88 {
        return;
    }
    // `ldy EndTimer+1 / iny / bne` ⇒ 高字节非 0 则出
    if mem.read_ram(crate::ram_map::END_TIMER + 1) != 0 {
        return;
    }
    mem.write_ram(crate::ram_map::GAME_PAUSED, 0); // `sta GamePaused`（A=0）
    let mr = mem.read_ram(crate::ram_map::MAIN_ROUTINE);
    mem.write_ram(crate::ram_map::MAIN_ROUTINE, mr.wrapping_add(1)); // `inc MainRoutine`
}

/// GoPassword（**asm 内容锚** `GoPassword: ; 07:C9C4`——`prg7_engine.asm:2170-2182`）：
/// ```text
/// lda #_id_DisplayPassword($19) / sta TitleRoutine
/// lda #$00+1 / sta BankSwitchPending     ; 切标题页
/// SFXNoiseInitFlags |= sfxNoise_SilenceMusic
/// jmp ScreenOff
/// ```
/// 之后：帧界 `CheckBankSwitch` → `InitBank0`（`GameMode←1`）→ 标题轴 `0x19 DisplayPassword`
/// （`ClearAll` + `PreparePPUProcess($937F)` "PASS WORD" + `InitPasswordFontGFX` +
/// `CalculatePassword` + `NMIOn` + `PasswordToScreen` + `PaletteDataPending←Palette12+1`）
/// → 相位→`0x1A WaitForSTART` → `jmp CheckPassword` → 有效密码回 `InitializeGame`。
pub fn go_password(mem: &mut FrameMemory) {
    mem.write_ram(crate::ram_map::TITLE_ROUTINE, 0x19); // _id_DisplayPassword
    mem.write_ram(crate::ram_map::BANK_SWITCH_PENDING, 0x01);
    super::sound_hook::silence_music(mem); // $CB8E
    let pm = mem.read_ram(0x00FE) & !0x18;
    mem.write_ram(0x00FE, pm);
    mem.write_io(0x2001, pm); // WriteAndWait 的 NMI 落真寄存器（同 prepare_game_over）
}

/// IncrementRoutine（**asm 内容锚** `IncrementRoutine: ; 07:C155`）= `inc MainRoutine / rts`。
pub fn increment_routine(mem: &mut FrameMemory) {
    let mr = mem.read_ram(crate::ram_map::MAIN_ROUTINE);
    mem.write_ram(crate::ram_map::MAIN_ROUTINE, mr.wrapping_add(1));
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
/// RinkaSpawners .0/.1=$8B/$8E）⑦InArea&$0F：0=Brinstar（Y=$27）/else=$2F+
/// `lsr ScrollDir($49)`（`07:C902` ROM=46 49，已定谳端口）⑧MirrorCntrl($FA)/MissilePickupQtyMax($93)/
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
        // 非 Brinstar（asm `SamusInit` prg7_engine.asm:1996-2005，`07:C900`）：
        //   lda InArea / and #$0F / beq Lx002(07:C908)
        //   lsr ScrollDir  ; 07:C902  ROM 逐字节 = 46 49（lsr $49 zp）
        //   ldy #$2F       ; 横向镜像 + 拾取上限
        // ROM 实证（bank7 fixed, file 0x1C900）= 29 0F F0 04 **46 49** A0 2F。
        // 旧形为 fail-loud 桩（注释「ScrollAddr 未定谳」）——ROM 直读后地址已定谳，
        // 按 asm 真形端口：lsr 的 carry 出被随后 ldy 丢弃，故只写回右移结果。
        let sd = mem.read_ram(crate::ram_map::SCROLL_DIR);
        mem.write_ram(crate::ram_map::SCROLL_DIR, sd >> 1);
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
        // 【r14 双调根修】DE47 形=单走表（旧形显式走表+anim_draw_object 内再走=双调 ✗）
        crate::frame::sprite::anim_draw_object(mem, 0, 0); // AnimDrawObject（$DE47——A=0 单走）
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
    // 【缝会计卡·检点勘误（2026-11-05）】asm 旗址=MiniBossKillDelayFlag($0108)/
    // PowerUpDelayFlag($0109)（constants_ram:809-810）——旧形读 $0083/$0084（错位）
    // ⇒拾取旗永不可见⇒SetTimer 链不发（emu f1521 拾取即冻 240 帧实录差源）。
    if mem.read_ram(0x0108) != 0 || mem.read_ram(0x0109) != 0 {
        mem.write_ram(0x0108, 0);
        mem.write_ram(0x0109, 0);
        // 【缝会计卡·SetTimer 写面勘误（2026-11-05）】asm C4AA 三写=Timer3←A（$18=24×10
        // =240 帧）+NextRoutine($20)←X（3=GameEngine）+**MainRoutine←_id_WaitTimer(9)**
        // （`lda #_id_WaitTimer/bne SetMainRoutine`）——旧形只写 Timer3+MainRoutine←3
        // （下例行误当本行）⇒无 WaitTimer 冻结⇒拾取后萨姆斯续动（emu 冻 240 帧能力
        // 动画实录（f1522 起 MainRoutine=9 恒驻））。
        mem.write_ram(crate::ram_map::TIMER3, 0x18);
        mem.write_ram(0x0020, 3); // NextRoutine（constants_ram:480）
        mem.write_ram(crate::ram_map::MAIN_ROUTINE, 9); // _id_WaitTimer
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
            // SetTimer 40 帧→routine 4（GameOver）——同 C4AA 三写形（缝会计卡勘误）
            mem.write_ram(crate::ram_map::TIMER3, 0x04);
            mem.write_ram(0x0020, 4); // NextRoutine←4（_id_PrepareGameOver）
            mem.write_ram(crate::ram_map::MAIN_ROUTINE, 9); // _id_WaitTimer
        }
    }
}

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

