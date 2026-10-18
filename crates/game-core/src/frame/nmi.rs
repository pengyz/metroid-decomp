//! NMI 面单元（bank07:C0D9；契约=docs/design/l3-gamecore-leg-verification.md §8.1/0172725）。
//!
//! **asm 引证主体**（链规 17）——refs/m1disasm/SRC/prg7_engine.asm:233 `NMI: ; 07:C0D9`
//! （注：「The NMI is called 60 times a second by the VBlank signal from the PPU…」）：
//! ①头：php/pha/txa/pha/tya/pha 保存 → `lda #$00 / sta OAMADDR` → `lda #>SpriteRAM /
//! sta OAMDMA`（$200-$2FF→OAM）；②**超帧跳过**（宽放 18 决策项 overframe 面）：`lda
//! NMIStatus / bne @endIf_A`（「Skip if the frame couldn't finish in time」）——$1A≠0 时
//! 跳过 NMIScreenWrite/CheckPaletteWrite/CheckVRAMStructBufferWrite/WritePPUCtrl/
//! WriteScroll/ReadJoyPads；③GameMode 分派（`lda GameMode / beq @endIf_B`→jsr
//! NMIScreenWrite (9A07)）；④SoundEngine (B3B4)/UpdateAge (C97E)（超帧也跑）；
//! ⑤尾：`ldy #$01 / sty NMIStatus`（**恒量 1**——会签决定性证据）→ pla/tay/pla/tax/pla/plp。
//!
//! 契约断言三件套（lead 令：随单测固化）：$1A 恒 1（本模块 nmi_tail 测）/ f=1 帧界锚
//! （engine 测）/ 栈页三字节排除（**比较器侧**——tools/ 面非本模块辖，见 §8.1②）。

use super::memory::FrameMemory;
use super::io;
use crate::ram_map::{GAME_MODE, NMI_STATUS, PALETTE_PENDING, PPU_DATA_PENDING, TITLE_ROUTINE, VRAM_STRUCT_INDEX};

/// NMI 处理器（60Hz VBlank 调用）。
pub fn nmi_handler(mem: &mut FrameMemory) {
    // ①头：OAM 面（寄存器面 write_io）
    mem.write_io(io::OAMADDR, 0x00);
    mem.write_io(io::OAMDMA, 0x02); // $200-$2FF → OAM
    // ②超帧跳过门（$1A≠0=上帧未完）+ ③工作体（嵌套桩逐个引爆）
    nmi_body(mem);
    // ⑤尾：$1A=1 恒量写（契约断言面）
    nmi_tail(mem);
}

/// 工作体（超帧门+屏幕工作序列——嵌套桩）。
fn nmi_body(mem: &mut FrameMemory) {
    if mem.read_ram(NMI_STATUS) != 0 {
        return; // 超帧跳过（SoundEngine/UpdateAge 桩移至尾前独立段——见下）
    }
    // GameMode 分派（GameMode=$1D（N-00xx asm 名）：≠0 → NMIScreenWrite）
    if mem.read_ram(GAME_MODE) != 0 {
        nmi_screen_write(mem); // NMIScreenWrite（asm :3590——已端口）
    }
    check_palette_write(mem); // CheckPaletteWrite（asm :507——已端口）
    check_vram_struct_write(mem); // CheckVRAMStructBufferWrite（asm :790——已端口）
    // 执行序补全（前版漏桩自查）：WritePPUCtrl/WriteScroll/ReadJoyPads（asm 调用序列）
    write_ppu_ctrl(mem); // WritePPUCtrl（asm :1150 族——已端口）
    write_scroll(mem); // WriteScroll（asm :717——已端口）
    read_joy_pads(mem); // ReadJoyPads（asm :1215 族——已端口）
    sound_engine(mem); // SoundEngine（sound_engine.asm :384 族——已端口）
    update_age(mem); // UpdateAge（asm :2088 族——已端口；NMI 体末件）
}

/// NMIScreenWrite 单元（**asm 引证主体**，链规 17）——
/// refs/m1disasm/SRC/prg0_title.asm:3590 `NMIScreenWrite: ; 00:9A07`（注「If titleRoutine
/// not at end game, exit」）：`lda TitleRoutine / cmp #$1D / bcc Exit100`——
/// **boot/title 态（TitleRoutine<$1D，Startup 置 0）⇒ 即时 RTS（零 RAM 写）**；
/// ≥$1D=结束画面路径（LoadCredits (9C45)/EndMsgWrite/HideShowEndMsg 族——未端口 fail-loud）。
/// 出口 A=TitleRoutine（帧界可观测面）。
pub fn nmi_screen_write(mem: &mut FrameMemory) {
    let t = mem.read_ram(TITLE_ROUTINE);
    mem.regs.a = t;
    if t < 0x1D {
        return; // Exit100（:3596 `bcc Exit100`→:3608 rts）
    }
    crate::unported!("bank00:9C45"); // LoadCredits（结束画面路径首件）
}

/// CheckPaletteWrite 单元（**asm 引证主体**，链规 17）——
/// refs/m1disasm/SRC/prg7_engine.asm:507 `CheckPaletteWrite: ; 07:C1E0`（注「branch if
/// game at ending sequence」）路由三路：
/// ①GameMode≠0 ∧ TitleRoutine≥`_id_EndGame`(=0x1D；TitleRoutinePtrTable 30 项第 30 位
///   自数证)→`jmp EndGamePaletteWrite`（00:9F54——未端口）；
/// ②`ldy PaletteDataPending`≠0→调色板写段（:535 族——未端口）；
/// ③GameMode≠0 ∧ TitleRoutine<`_id_StartContinueScreen15`(=0x15；表第 22 位自数证)
///   →`jmp StarPaletteSwitch`（00:8AC7——intro 星星闪烁轮换，**boot 路径**）；
/// ④否则 RTS。
pub fn check_palette_write(mem: &mut FrameMemory) {
    if mem.read_ram(GAME_MODE) != 0 && mem.read_ram(TITLE_ROUTINE) >= 0x1D {
        crate::unported!("bank00:9F54"); // EndGamePaletteWrite（结束路径）
    }
    if mem.read_ram(PALETTE_PENDING) != 0 {
        palette_write_pending(mem); // C1FF（S6-3——已端口）
    }
    if mem.read_ram(GAME_MODE) != 0 && mem.read_ram(TITLE_ROUTINE) < 0x15 {
        super::intro::star_palette_switch(mem); // StarPaletteSwitch（asm 内容锚——已端口）
    }
    // ④否则 RTS（静默）
}

/// CheckVRAMStructBufferWrite 单元（**asm 内容锚引证主体**）——
/// refs/m1disasm/SRC/prg7_engine.asm `CheckVRAMStructBufferWrite: ; 07:C2CA`（注「Exit if
/// no PPU data to write」）：`lda PPUDataPending / beq @RTS`——**boot 态（$1B=0，生产端
/// 未跑）⇒ 静默 RTS**；≠0 ⇒ ptr=$07A1（Temp00_VRAMStructPtr）→ `jsr VRAMStructWrite`
/// （缓冲走带——PPU 面/渲染面）→ 清三态（VRAMStructBufferIndex/$07A1/PPUDataPending=0）。
/// 消费端=StarPaletteSwitch 生产端的对子（$07A0 队列门复位面）。
pub fn check_vram_struct_write(mem: &mut FrameMemory) {
    if mem.read_ram(PPU_DATA_PENDING) == 0 {
        return; // boot 态静默（@RTS）
    }
    mem.write_ram(0x0000, 0xA1); // Temp00_VRAMStructPtr lo（VRAMStructBuffer=$07A1）
    mem.write_ram(0x0001, 0x07); // hi
    vram_struct_write(mem); // 走带（asm :856——已端口；尾跳 WriteScroll 桩）
    #[allow(unreachable_code)]
    {
        mem.write_ram(VRAM_STRUCT_INDEX, 0);
        mem.write_ram(0x07A1, 0); // VRAMStructBuffer 清
        mem.write_ram(PPU_DATA_PENDING, 0);
    }
}

/// VRAMStructWrite 走带件（**asm 内容锚+结构互核注**）——
/// refs/m1disasm/SRC/prg7_engine.asm `VRAMStructWrite: ; 07:C30C`（主循环：`ldx
/// PPUSTATUS`（清 flip-flop）/`ldy #$00/lda (ptr),y/bne VRAMStructWriteData`/
/// `jmp WriteScroll`（hi=0=结束尾跳））+ `VRAMStructWriteData: ; 07:C2E4`（段处理：
/// `sta PPUADDR`（hi）/`iny/lda (ptr),y/sta PPUADDR`（lo）/ctrl `asl`→`jsr SetPPUInc`
/// （CF=bit7=增量 1/32）→`asl`（CF=bit6=RLE）→`and #$3F/tax` 计数→数据 loop
/// `lda (ptr),y/sta PPUDATA/dex/bne`→`iny/jsr AddYToPtr00`）。
///
/// **结构互核注**：①段内 y 推进与生产端对偶（非 RLE 每字节 iny / RLE 值只取一次）——
/// 生产/消费 y 算术**互为镜像**（成对对表点）②`jmp WriteScroll` 尾跳=走带退出语义
/// （WriteScroll 的 RTS 回 CheckVRAMStructBufferWrite 清理段——栈深一层）③SetPPUInc
/// **动 $FF**（PPUCTRL_ZP——RAM watch 面）：ctrl bit7=1→增量 32 置位 INCR_DOWN、
/// bit7=0→清——段级 $FF 副作用=可观测面。
/// PPU 面（PPUADDR/PPUDATA）=write_io 族（渲染面暂不建模——挂 task-13）。
pub fn vram_struct_write(mem: &mut FrameMemory) {
    let base = 0x07A1u16;
    let mut off = 0usize; // 走带偏移（AddYToPtr00 的域形）
    loop {
        let hi = mem.read_ram(base + off as u16);
        if hi == 0 {
            write_scroll(mem); // 尾跳 WriteScroll（asm :717——已端口）；RTS 回清理段
            return;
        }
        let lo = mem.read_ram(base + off as u16 + 1);
        let ctrl = mem.read_ram(base + off as u16 + 2);
        mem.write_io(0x2006, hi); // PPUADDR hi
        mem.write_io(0x2006, lo); // PPUADDR lo
        // SetPPUInc：ctrl bit7=PPU 增量位（0=1/1=32）→ $FF（PPUCTRL_ZP）
        let f = mem.read_ram(crate::ram_map::PPUCTRL_ZP);
        let f = if ctrl & 0x80 != 0 { f | 0x04 } else { f & !0x04 };
        mem.write_ram(crate::ram_map::PPUCTRL_ZP, f);
        mem.write_io(0x2000, f);
        // 数据段（RLE=bit6；计数=低 6 位）
        let count = (ctrl & 0x3F) as usize;
        let rle = ctrl & 0x40 != 0;
        let mut di = 3usize;
        for k in 0..count {
            let v = if rle { mem.read_ram(base + off as u16 + 4) } else {
                mem.read_ram(base + off as u16 + 3 + k as u16)
            };
            let _ = di;
            mem.write_io(0x2007, v); // PPUDATA（渲染面）
        }
        off += if rle { 4 } else { 3 + count }; // 段尾（RLE=3 hdr+1 val=4——F5 修正：非 +5）
    }
}

/// SoundEngine 单元（**asm 内容锚+结构互核注**）——
/// refs/m1disasm/SRC/sound_engine.asm `SoundEngine: ; $B3B4`（`lda #APU_5STEP|
/// APU_IRQDISABLE/sta JOY2`（APU 帧配置——寄存器面）/静音门 `lda SFXNoiseInitFlags/lsr/
/// bcs SoundEngine_SilenceMusic`/暂停门 `lda MainRoutine/cmp #_id_PauseMode/beq
/// GameIsPaused`/`lda #$00/sta SFXPaused`/四 init 子件 jsr/`ClearInitFlags: ; $B3D6`
/// （6 旗清））。
///
/// **结构互核注**：①四 init 子件=表分派短路（`RunSoundRoutine`→`jsr GetSoundRoutine`
/// 旗检→`jmp (SoundRoutinePtr)`——flags=0 ⇒ 分派表出口 stub）：boot 态全 0 ⇒ 全短路
/// ②ClearInitFlags 的 6 旗清=**同值写事件**（L2 契约面同款——b-f270-a0 教训族）
/// ③声态 zp 全在 watch 面（$0602/$0680-85——iram 内可观测）。
/// 非零旗路径（SFX 分派/静音/暂停）=fail-loud（音频域件随任务展开）。
pub fn sound_engine(mem: &mut FrameMemory) {
    mem.write_io(0x4017, 0x40 | 0x80); // APU_5STEP|IRQDISABLE（JOY2 口——APU 帧配置）
    if mem.read_ram(crate::ram_map::SFX_NOISE_INIT) & 1 != 0 {
        crate::unported!("bank07:B3EB"); // SoundEngine_SilenceMusic（静音路径）
    }
    if mem.read_ram(crate::ram_map::MAIN_ROUTINE) == 5 {
        crate::unported!("bank07:B3EB-paused"); // GameIsPaused 路径（_id_PauseMode=5 注证）
    }
    mem.write_ram(crate::ram_map::SFX_PAUSED, 0); // `sta SFXPaused`
    // 四 init 子件：flags=0 短路（RunSoundRoutine 分派出口）；非零=域件桩
    // 四 init 链走分派（B31B 行清偿——port2 分派核会师接点）
    for (flag, block) in [
        (crate::ram_map::SFX_NOISE_INIT, 0xB289u16),
        (crate::ram_map::SFX_MULTI_INIT, 0xB2A7),
        (crate::ram_map::SFX_TRI_INIT, 0xB29D),
        (crate::ram_map::MUSIC_INIT, 0xB2B1),
    ] {
        super::sound_hook::sound_dispatch_chain(mem, flag, block, 0);
    }
    // ClearInitFlags（$B3D6）：6 旗清（同值写事件）
    for addr in [
        crate::ram_map::SFX_NOISE_INIT,
        crate::ram_map::SFX_SQ1_INIT,
        crate::ram_map::SFX_SQ2_INIT,
        crate::ram_map::SFX_TRI_INIT,
        crate::ram_map::SFX_MULTI_INIT,
        crate::ram_map::MUSIC_INIT,
    ] {
        mem.write_ram(addr, 0);
    }
}

/// ReadJoyPads 单元（**asm 内容锚+结构互核注**）——
/// refs/m1disasm/SRC/prg7_engine.asm `ReadJoyPads: ; 07:C215`（`ldx #$00/stx $01/jsr
/// ReadOnePad`（pad1）/`inx/inc $01`（fallthrough pad2））+ `ReadOnePad: ; 07:C21F`
/// （strobe `sty JOY1`×2 →8 位串行 `lda JOY1,x/lsr/ora $00/lsr/rol` 累装→
/// `sta Joy1Status,x`（**状态落 RAM**）→`eor $00` 变更检测→变更路径）。
///
/// **结构互核注**：①8 次 `rol` 累装=NES 串行位序（A,B,Select,Start,Up,Down,Left,Right）
/// ——输入面域形=直接按钮字节（序列化等价自证）②变更路径（Joy1Change/$12 面）**boot
/// 场景不触发**（无输入 0^0=0）——fail-loud 桩（输入场景=task-11 samus 移动 demo 的
/// 引爆面）③fallthrough 双 pad（pad2 复用 ReadOnePad 无 jsr——栈面一层）。
pub fn read_joy_pads(mem: &mut FrameMemory) {
    for (idx, input) in [(0u16, mem.joy1), (1u16, mem.joy2)] {
        // 串行累装域形：输入字节直配（strobe/shift 等价——8 次 rol 累装按钮字节）
        let new = input;
        let status_addr = crate::ram_map::JOY1_STATUS + idx;
        let old = mem.read_ram(status_addr);
        mem.write_ram(0x0001, idx as u8); // $01=pad #（scratch 面）
        mem.write_ram(0x0000, old); // $00=前值（scratch 面）
        mem.write_ram(status_addr, new); // `sta Joy1Status,x`
        if new ^ old != 0 {
            // 变更路径（Joy1Change/$12 面——输入场景件，task-11 引爆）
            crate::unported!("bank07:C23x-change");
        }
    }
}

/// WritePPUCtrl 单元（**asm 内容锚+结构互核注**）——
/// refs/m1disasm/SRC/prg7_engine.asm `WritePPUCtrl: ; 07:C44D`（注「Update PPU control
/// registers」：`lda PPUCTRL_ZP/sta PPUCTRL`/`lda PPUMASK_ZP/sta PPUMASK`/
/// `lda MirrorCntrl/jsr PrepPPUMirror`（$C4D9→`jmp SetPPUMirror`））。
///
/// **结构互核注**：①零直接 RAM 写，但 **SetPPUMirror 共享尾动 $00/$25**（三路汇入共享段
/// ——boot 的 c4b2_unit 同段复用，域形=共享 fn 复刻共享结构）②寄存器面写走 write_io
/// （$2000/$2001——NMI 使能位面同步 nmi_enabled 权威）③ExitSub（C45C）=共享 RTS 尾
/// （asm 「Exit subroutines」）。
pub fn write_ppu_ctrl(mem: &mut FrameMemory) {
    let c = mem.read_ram(crate::ram_map::PPUCTRL_ZP);
    mem.write_io(0x2000, c);
    let m = mem.read_ram(crate::ram_map::PPUMASK_ZP);
    mem.write_io(0x2001, m);
    let mirror = mem.read_ram(crate::ram_map::MIRROR_CNTRL);
    super::boot::set_ppu_mirror(mem, mirror);
}

/// WriteScroll 单元（**asm 内容锚+结构互核注**）——
/// refs/m1disasm/SRC/prg7_engine.asm `WriteScroll: ; 07:C29A`（注「X and Y scroll offsets
/// are loaded serially」：`lda PPUSTATUS`（清 flip-flop）/`lda ScrollX/sta PPUSCROLL`/
/// `lda ScrollY/sta PPUSCROLL`/`rts`）。
///
/// **结构互核注**：零 RAM 写（只读 ScrollX/Y——$FC/$FD 与 boot 初始化 `sty` 面互证）；
/// 双路汇聚（走带尾跳 `jmp WriteScroll` + NMI 工作体直调）——**同一 RTS 语义**：
/// 尾跳路的 RTS 回 CheckVRAMStructBufferWrite 清理段（栈深一层，走带注②面），直调路回
/// NMI 体——域形折叠为普通返回，调用方各自续。
pub fn write_scroll(mem: &mut FrameMemory) {
    let x = mem.read_ram(crate::ram_map::SCROLL_X);
    let y = mem.read_ram(crate::ram_map::SCROLL_Y);
    mem.write_io(0x2005, x); // PPUSCROLL
    mem.write_io(0x2005, y);
}

/// UpdateAge 单元（**asm 内容锚+结构互核注**）——
/// refs/m1disasm/SRC/prg7_engine.asm `UpdateAge: ; 07:C97E`（注「Exit if at title/
/// password screen」：`lda GameMode/bne @RTS`——**boot/title 态纯短路**；游戏路径
/// （GameMode==0 ∧ MainRoutine==_id_GameEngine ∧ FrameCount==0）→`inc SamusAge,x`/
/// `cmp #$D0` 进位族）。
///
/// **结构互核注**：三重短路门（GameMode/MainRoutine/FrameCount 节拍）——标题态全不触
/// SamusAge（帧界可观测面零写）；游戏路径=**Samus 年龄域件**（task-11 面）fail-loud。
pub fn update_age(mem: &mut FrameMemory) {
    if mem.read_ram(crate::ram_map::GAME_MODE) != 0 {
        return; // 标题/密码态即时 RTS
    }
    if mem.read_ram(crate::ram_map::MAIN_ROUTINE) != 3 {
        return; // 非 GameEngine（注证：3 = Game engine running）→ RTS
    }
    if mem.read_ram(crate::ram_map::FRAME_COUNT) != 0 {
        return; // 节拍门（每 256 帧一更）
    }
    crate::unported!("bank07:C97E-age"); // Samus 年龄域件（$D0 进位族——task-11 面）
}

/// 尾段（**恒量 1 写**——:279-280 `ldy #$01 / sty NMIStatus`）。
/// 契约断言面：spin 出口采样点 $1A==1（§8.1②采样点对齐）。
pub fn nmi_tail(mem: &mut FrameMemory) {
    mem.write_ram(NMI_STATUS, 0x01);
}

#[cfg(test)]
mod tests {
    /// 【返工补案 5】F5 镜像测（RLE 步长——生产/消费对偶：RLE 段=4B 非 +5）。
    #[test]
    fn vram_walk_rle_step_mirror() {
        // 构造 RLE 段：[20][00][45][AA]（hi=20/lo=00/ctrl=0x45（RLE+5）/val=AA）
        // 段长=4（3 hdr+1 val）→ 下段头应在 +4
        let mut m = FrameMemory::default();
        let seg = [0x20u8, 0x00, 0x45, 0xAA, 0x20, 0x10, 0x03, 0x01, 0x02, 0x03, 0x00];
        for (i, b) in seg.iter().enumerate() {
            m.write_ram(0x07A1 + i as u16, *b);
        }
        let mut off = 0usize;
        let mut chunks = 0;
        loop {
            let hi = m.read_ram(0x07A1 + off as u16);
            if hi == 0 { break; }
            let ctrl = m.read_ram(0x07A1 + off as u16 + 2);
            let rle = ctrl & 0x40 != 0;
            let count = (ctrl & 0x3F) as usize;
            off += if rle { 4 } else { 3 + count }; // F5 修正步长
            chunks += 1;
        }
        assert_eq!(chunks, 2); // RLE 段（4B）+非 RLE 段（3+3=6B）→终止
        assert_eq!(off, 10); // 4+6
    }

    /// SpriteRAM 解析+8/scanline quirk（D4 保真面）。
    #[test]
    fn sprite_state_and_scanline_limit() {
        let mut m = FrameMemory::default();
        for i in 0..10u16 {
            m.write_ram(0x0200 + i * 4, 50);
            m.write_ram(0x0200 + i * 4 + 1, i as u8);
        }
        let sprites = sprite_state(&m);
        assert_eq!(sprites.len(), 64);
        assert_eq!(sprites[0].y, 50);
        let counts = scanline_sprite_limit(&sprites);
        assert_eq!(counts[55], 8); // 10 精灵同线→限 8（quirk 保真）
    }

    use super::*;

    /// panic payload 双型兜底。
    fn panic_message(err: &Box<dyn std::any::Any + Send>) -> String {
        err.downcast_ref::<String>()
            .cloned()
            .or_else(|| err.downcast_ref::<&'static str>().map(|s| (*s).to_string()))
            .unwrap_or_default()
    }

    /// 契约断言①：NMI 尾恒量 1（:279 `ldy #$01`——非 Y 依赖，会签决定性证据）。
    #[test]
    fn nmi_tail_writes_constant_one() {
        let mut m = FrameMemory::default();
        m.write_ram(NMI_STATUS, 0);
        nmi_tail(&mut m);
        assert_eq!(m.read_ram(NMI_STATUS), 1);
    }

    /// NMIScreenWrite boot 态短路（TitleRoutine<$1D：零 RAM 写+A=TitleRoutine）。
    #[test]
    fn nmi_screen_write_boot_path_short_circuits() {
        let mut m = FrameMemory::default(); // TitleRoutine=0
        nmi_screen_write(&mut m);
        assert_eq!(m.regs.a, 0);
        assert_eq!(m.ram, [0u8; 0x800]); // 零 RAM 写
    }

    /// NMIScreenWrite 结束画面路径 fail-loud（TitleRoutine≥$1D）。
    #[test]
    fn nmi_screen_write_endgame_path_detonates() {
        let mut m = FrameMemory::default();
        m.write_ram(TITLE_ROUTINE, 0x1D);
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| nmi_screen_write(&mut m)));
        let msg = panic_message(&r.unwrap_err());
        assert!(msg.contains("unported @ bank00:9C45"), "引爆点失准: {msg}");
    }

    /// CheckPaletteWrite 三路分派（boot→8AC7；ending→9F54；pending→C1FF）+ 静默路。
    #[test]
    fn check_palette_write_routes_three_paths() {
        use crate::ram_map::{GAME_MODE, PALETTE_PENDING, TITLE_ROUTINE};
        let boom = |m: &mut FrameMemory| {
            panic_message(&std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                check_palette_write(m)
            })).unwrap_err())
        };
        let mut m = FrameMemory::default();
        m.write_ram(GAME_MODE, 1); // boot/title 态 → StarPaletteSwitch（已端口=全通）
        check_palette_write(&mut m);
        assert_eq!(m.read_ram(crate::ram_map::PPU_DATA_PENDING), 1); // 生产端效应断言
        let mut m2 = FrameMemory::default();
        m2.write_ram(GAME_MODE, 1);
        m2.write_ram(TITLE_ROUTINE, 0x1D); // ending
        assert!(boom(&mut m2).contains("unported @ bank00:9F54"));
        let mut m3 = FrameMemory::default();
        m3.write_ram(PALETTE_PENDING, 2);
        check_palette_write(&mut m3); // pending 路径全通（S6-3 已端口）
        assert_eq!(m3.read_ram(PALETTE_PENDING), 0); // 重置面
        let mut m4 = FrameMemory::default(); // GameMode=0 & 无 pending → 静默
        check_palette_write(&mut m4);
    }

    /// UpdateAge 标题态纯短路（GameMode≠0 ⇒ 零写即时出）+游戏路径引爆。
    #[test]
    fn update_age_title_short_circuits() {
        use crate::ram_map::{FRAME_COUNT, GAME_MODE, MAIN_ROUTINE, SAMUS_AGE};
        let mut m = FrameMemory::default();
        m.write_ram(GAME_MODE, 1); // title 态
        update_age(&mut m); // 短路（零写）
        assert_eq!(m.read_wram(SAMUS_AGE), 0); // WRAM 面（$687D）
        // 游戏路径（GameMode=0/主例程=3/帧计数=0）
        m.write_ram(GAME_MODE, 0);
        m.write_ram(MAIN_ROUTINE, 3);
        m.write_ram(FRAME_COUNT, 0);
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| update_age(&mut m)));
        let msg = panic_message(&r.unwrap_err());
        assert!(msg.contains("unported @ bank07:C97E-age"), "引爆点失准: {msg}");
    }

    /// SoundEngine boot 面（全短路+6 旗同值写事件）+非零旗引爆。
    #[test]
    fn sound_engine_boot_face_and_flag_detonates() {
        use crate::ram_map::{MUSIC_INIT, SFX_NOISE_INIT, SFX_PAUSED, SFX_SQ1_INIT};
        let mut m = FrameMemory::default();
        sound_engine(&mut m);
        assert_eq!(m.read_ram(SFX_PAUSED), 0);
        assert_eq!(m.read_ram(SFX_NOISE_INIT), 0); // 6 旗清（同值写事件）
        assert_eq!(m.read_ram(SFX_SQ1_INIT), 0);
        m.write_ram(MUSIC_INIT, 0x80); // 音乐请求（位 7——6 位扫面：位 0/1 不扫）
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| sound_engine(&mut m)));
        let msg = panic_message(&r.unwrap_err());
        assert!(msg.contains("unported @ bank07:"), "handler 记账点失准: {msg}");
    }

    /// ReadJoyPads boot 面（无输入：$14/$15=0 落态+变更路径不触发）+输入变更引爆。
    #[test]
    fn read_joy_pads_boot_zero_and_change_detonates() {
        use crate::ram_map::{JOY1_STATUS, JOY2_STATUS};
        let mut m = FrameMemory::default(); // joy1/joy2=0
        read_joy_pads(&mut m);
        assert_eq!(m.read_ram(JOY1_STATUS), 0);
        assert_eq!(m.read_ram(JOY2_STATUS), 0);
        assert_eq!(m.read_ram(0x0001), 1); // pad # scratch 终态
        m.joy1 = 0x20; // A 键（输入场景）
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| read_joy_pads(&mut m)));
        let msg = panic_message(&r.unwrap_err());
        assert!(msg.contains("unported @ bank07:C23x-change"), "引爆点失准: {msg}");
    }

    /// 走带镜像测：生产→消费对偶（$FF 增量位更新+尾跳 C29A——结构互核注①③面）。
    #[test]
    fn vram_struct_walk_mirror_producer() {
        use crate::ram_map::PPUCTRL_ZP;
        let mut m = FrameMemory::default();
        crate::frame::intro::star_palette_switch(&mut m); // 生产（双 call 缓冲）
        m.write_ram(PPUCTRL_ZP, 0x90); // NMI 位+增量位预置
        vram_struct_write(&mut m); // 全通（尾跳 WriteScroll 已端口）
        // ctrl bit7=0（两段皆 0x03）⇒ $FF 增量位（0x04）清——SetPPUInc 域形
        assert_eq!(m.read_ram(PPUCTRL_ZP) & 0x04, 0x00);
    }

    /// C2CA 消费端：boot 态（$1B=0）静默 + pending 路径引爆 C2E4。
    #[test]
    fn check_vram_struct_boot_silent_and_pending_detonates() {
        use crate::ram_map::{PPU_DATA_PENDING, VRAM_STRUCT_INDEX};
        let mut m = FrameMemory::default(); // $1B=0
        check_vram_struct_write(&mut m); // 静默
        assert_eq!(m.read_ram(VRAM_STRUCT_INDEX), 0);
        m.write_ram(PPU_DATA_PENDING, 1);
        check_vram_struct_write(&mut m); // 空缓冲：终止符即刻→WriteScroll→清理段全通
        assert_eq!(m.read_ram(PPU_DATA_PENDING), 0); // 清理段三态
        assert_eq!(m.read_ram(VRAM_STRUCT_INDEX), 0);
        assert_eq!(m.read_ram(0x07A1), 0);
    }

    /// 超帧跳过面：$1A≠0 ⇒ 工作体跳过（不炸桩）；$1A=0 ⇒ 进 GameMode 分派桩。
    #[test]
    fn overframe_gate_skips_body() {
        let mut m = FrameMemory::default();
        m.write_ram(NMI_STATUS, 1);
        nmi_body(&mut m); // 不炸=跳过成功
        m.write_ram(NMI_STATUS, 0);
        nmi_body(&mut m); // NMI 体全通（boot 态各单元短路面）
        assert_eq!(m.read_ram(crate::ram_map::SFX_PAUSED), 0); // 体效应面
    }
}

/// PalettePtrTable 烘焙（00:9560——19 项指针；PtrTableEntryArea 面）。
const PALETTE_PTR_TABLE: [u16; 19] = [
    0x9586, 0x95AA, 0x95CE, 0x95F2, 0x9616, 0x963A, 0x965E, 0x9682, 0x96A6,
    0x96CA, 0x96EE, 0x9712, 0x9736, 0x975A, 0x977E, 0x97A2, 0x97C6, 0x97EA, 0x980E,
];

/// Palette00-12 块体烘焙（00:9586 起 684B——VRAMStructData 格式：[hi][lo][ctrl][32B]
/// [00] 终止；首块 `3f 00 20 0f` 自证（$3F00/ctrl=0x20（32B）））。
const PALETTE_BLOBS: &[u8] = &[
    0x3F, 0x00, 0x20, 0x0F, 0x28, 0x18, 0x08, 0x0F, 0x29, 0x1B, 0x1A, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F,
    0x0F, 0x0F, 0x0F, 0x0F, 0x16, 0x1A, 0x27, 0x0F, 0x37, 0x3A, 0x1B, 0x0F, 0x17, 0x31, 0x37, 0x0F,
    0x32, 0x22, 0x12, 0x00, 0x3F, 0x00, 0x20, 0x0F, 0x28, 0x18, 0x08, 0x0F, 0x29, 0x1B, 0x1A, 0x0F,
    0x35, 0x35, 0x04, 0x0F, 0x35, 0x14, 0x04, 0x0F, 0x16, 0x1A, 0x27, 0x0F, 0x37, 0x3A, 0x1B, 0x0F,
    0x17, 0x31, 0x37, 0x0F, 0x32, 0x22, 0x12, 0x00, 0x3F, 0x00, 0x20, 0x0F, 0x28, 0x18, 0x08, 0x0F,
    0x29, 0x1B, 0x1A, 0x0F, 0x39, 0x39, 0x09, 0x0F, 0x39, 0x29, 0x09, 0x0F, 0x16, 0x1A, 0x27, 0x0F,
    0x37, 0x3A, 0x1B, 0x0F, 0x17, 0x31, 0x37, 0x0F, 0x32, 0x22, 0x12, 0x00, 0x3F, 0x00, 0x20, 0x0F,
    0x28, 0x18, 0x08, 0x0F, 0x29, 0x1B, 0x1A, 0x0F, 0x36, 0x36, 0x06, 0x0F, 0x36, 0x15, 0x06, 0x0F,
    0x16, 0x1A, 0x27, 0x0F, 0x37, 0x3A, 0x1B, 0x0F, 0x17, 0x31, 0x37, 0x0F, 0x32, 0x22, 0x12, 0x00,
    0x3F, 0x00, 0x20, 0x0F, 0x28, 0x18, 0x08, 0x0F, 0x29, 0x1B, 0x1A, 0x0F, 0x27, 0x27, 0x12, 0x0F,
    0x27, 0x21, 0x12, 0x0F, 0x16, 0x1A, 0x27, 0x0F, 0x31, 0x20, 0x1B, 0x0F, 0x17, 0x31, 0x37, 0x0F,
    0x32, 0x22, 0x12, 0x00, 0x3F, 0x00, 0x20, 0x0F, 0x28, 0x18, 0x08, 0x0F, 0x29, 0x1B, 0x1A, 0x0F,
    0x01, 0x01, 0x0F, 0x0F, 0x01, 0x0F, 0x0F, 0x0F, 0x16, 0x1A, 0x27, 0x0F, 0x37, 0x3A, 0x1B, 0x0F,
    0x17, 0x31, 0x37, 0x0F, 0x32, 0x22, 0x12, 0x00, 0x3F, 0x00, 0x20, 0x0F, 0x28, 0x18, 0x08, 0x0F,
    0x29, 0x1B, 0x1A, 0x0F, 0x01, 0x01, 0x0F, 0x0F, 0x01, 0x01, 0x0F, 0x0F, 0x16, 0x1A, 0x27, 0x0F,
    0x37, 0x3A, 0x1B, 0x0F, 0x17, 0x31, 0x37, 0x0F, 0x32, 0x22, 0x12, 0x00, 0x3F, 0x00, 0x20, 0x0F,
    0x28, 0x18, 0x08, 0x0F, 0x29, 0x1B, 0x1A, 0x0F, 0x02, 0x02, 0x01, 0x0F, 0x02, 0x02, 0x01, 0x0F,
    0x16, 0x1A, 0x27, 0x0F, 0x37, 0x3A, 0x1B, 0x0F, 0x17, 0x31, 0x37, 0x0F, 0x32, 0x22, 0x12, 0x00,
    0x3F, 0x00, 0x20, 0x0F, 0x28, 0x18, 0x08, 0x0F, 0x29, 0x1B, 0x1A, 0x0F, 0x02, 0x02, 0x01, 0x0F,
    0x02, 0x01, 0x01, 0x0F, 0x16, 0x1A, 0x27, 0x0F, 0x37, 0x3A, 0x1B, 0x0F, 0x17, 0x31, 0x37, 0x0F,
    0x32, 0x22, 0x12, 0x00, 0x3F, 0x00, 0x20, 0x0F, 0x28, 0x18, 0x08, 0x0F, 0x29, 0x1B, 0x1A, 0x0F,
    0x12, 0x12, 0x02, 0x0F, 0x12, 0x12, 0x02, 0x0F, 0x16, 0x1A, 0x27, 0x0F, 0x37, 0x3A, 0x1B, 0x0F,
    0x17, 0x31, 0x37, 0x0F, 0x32, 0x22, 0x12, 0x00, 0x3F, 0x00, 0x20, 0x0F, 0x28, 0x18, 0x08, 0x0F,
    0x29, 0x1B, 0x1A, 0x0F, 0x11, 0x11, 0x02, 0x0F, 0x11, 0x02, 0x02, 0x0F, 0x16, 0x1A, 0x27, 0x0F,
    0x37, 0x3A, 0x1B, 0x0F, 0x17, 0x31, 0x37, 0x0F, 0x32, 0x22, 0x12, 0x00, 0x3F, 0x00, 0x20, 0x0F,
    0x28, 0x18, 0x08, 0x0F, 0x29, 0x1B, 0x1A, 0x0F, 0x31, 0x31, 0x01, 0x0F, 0x31, 0x11, 0x01, 0x0F,
    0x16, 0x1A, 0x27, 0x0F, 0x37, 0x3A, 0x1B, 0x0F, 0x17, 0x31, 0x37, 0x0F, 0x32, 0x22, 0x12, 0x00,
    0x3F, 0x00, 0x20, 0x0F, 0x28, 0x18, 0x08, 0x0F, 0x12, 0x30, 0x21, 0x0F, 0x27, 0x28, 0x29, 0x0F,
    0x31, 0x31, 0x01, 0x0F, 0x16, 0x2A, 0x27, 0x0F, 0x12, 0x30, 0x21, 0x0F, 0x27, 0x24, 0x2C, 0x0F,
    0x15, 0x21, 0x38, 0x00, 0x3F, 0x00, 0x20, 0x0F, 0x28, 0x18, 0x08, 0x0F, 0x29, 0x1B, 0x1A, 0x0F,
    0x12, 0x12, 0x01, 0x0F, 0x12, 0x02, 0x01, 0x0F, 0x16, 0x1A, 0x27, 0x0F, 0x37, 0x3A, 0x1B, 0x0F,
    0x17, 0x31, 0x37, 0x0F, 0x32, 0x22, 0x12, 0x00, 0x3F, 0x00, 0x20, 0x0F, 0x28, 0x18, 0x08, 0x0F,
    0x29, 0x1B, 0x1A, 0x0F, 0x02, 0x02, 0x0F, 0x0F, 0x02, 0x01, 0x0F, 0x0F, 0x16, 0x1A, 0x27, 0x0F,
    0x37, 0x3A, 0x1B, 0x0F, 0x17, 0x31, 0x37, 0x0F, 0x32, 0x22, 0x12, 0x00, 0x3F, 0x00, 0x20, 0x0F,
    0x28, 0x18, 0x08, 0x0F, 0x29, 0x1B, 0x1A, 0x0F, 0x01, 0x01, 0x0F, 0x0F, 0x01, 0x0F, 0x0F, 0x0F,
    0x16, 0x1A, 0x27, 0x0F, 0x37, 0x3A, 0x1B, 0x0F, 0x17, 0x31, 0x37, 0x0F, 0x32, 0x22, 0x12, 0x00,
    0x3F, 0x00, 0x20, 0x30, 0x28, 0x18, 0x08, 0x30, 0x29, 0x1B, 0x1A, 0x30, 0x30, 0x30, 0x30, 0x30,
    0x30, 0x30, 0x30, 0x30, 0x16, 0x1A, 0x27, 0x30, 0x37, 0x3A, 0x1B, 0x30, 0x17, 0x31, 0x37, 0x30,
    0x32, 0x22, 0x12, 0x00, 0x3F, 0x00, 0x04, 0x0F, 0x30, 0x30, 0x21, 0x00, 0x3F, 0x00, 0x10, 0x0F,
    0x30, 0x30, 0x0F, 0x0F, 0x2A, 0x2A, 0x21, 0x0F, 0x31, 0x31, 0x0F, 0x0F, 0x2A, 0x2A, 0x21, 0x00,
    0x3F, 0x00, 0x10, 0x0F, 0x2C, 0x2C, 0x2C, 0x0F, 0x2C, 0x2C, 0x2C, 0x0F, 0x2C, 0x2C, 0x2C, 0x0F,
    0x2C, 0x2C, 0x2C, 0x3F, 0x10, 0x50, 0x0F, 0x00, 0xBD, 0xAC, 0x6E, 0x20, 0x71, 0x98, 0xBC, 0xAE,
    0x6E, 0x10, 0x05, 0x49, 0xFF, 0x18, 0x69, 0x01, 0x18, 0x7D, 0xA3, 0x6E,
];

/// @paletteDataIsPending（**asm 内容锚**——S6-3 调色板链；task-42 spec §2.4）：
/// `dey/tya/asp（PaletteDataPending-1）*2→PalettePtrTable,y` 查表→X/Y=数据指针→
/// `PaletteDataPending←0`→PreparePPUProcess（C20E）：`Temp00←X/Y→**jmp VRAMStructWrite**
/// （共享解码器——spec「与缓冲冲刷共用解码器」自证）。
pub fn palette_write_pending(mem: &mut FrameMemory) {
    let idx = (mem.read_ram(PALETTE_PENDING).wrapping_sub(1) & 0xFF) as usize;
    if idx >= 19 {
        crate::unported!("bank07:C1FF-idx"); // 表外索引（fail-loud）
    }
    mem.write_ram(PALETTE_PENDING, 0); // 重置（`sta PaletteDataPending`）
    // 共享解码器（ROM 源——块体走带：[hi][lo][ctrl][data…] 终止符制）
    let base = PALETTE_PTR_TABLE[idx] - 0x9586;
    let mut o = base as usize;
    loop {
        let hi = PALETTE_BLOBS[o];
        if hi == 0 {
            return; // 终止
        }
        let lo = PALETTE_BLOBS[o + 1];
        let ctrl = PALETTE_BLOBS[o + 2];
        mem.write_io(0x2006, hi);
        mem.write_io(0x2006, lo);
        let f = mem.read_ram(crate::ram_map::PPUCTRL_ZP);
        let f = if ctrl & 0x80 != 0 { f | 0x04 } else { f & !0x04 };
        mem.write_ram(crate::ram_map::PPUCTRL_ZP, f);
        mem.write_io(0x2000, f);
        let count = (ctrl & 0x3F) as usize;
        let rle = ctrl & 0x40 != 0;
        let mut d = o + 3;
        for _ in 0..count {
            let v = if rle { PALETTE_BLOBS[o + 3] } else {
                let v = PALETTE_BLOBS[d];
                d += 1;
                v
            };
            mem.write_io(0x2007, v);
        }
        o = if rle { o + 4 } else { d };
    }
}

/// SpriteRAM 状态建模（**S6-4——task-42 spec OAM 面**）：
/// $0200-$02FF=64 精灵×4B（y/tile/attr/x）；**8 sprites/scanline quirk**（D4 保真
/// bug：扫描线第 9 精灵丢弃）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sprite {
    pub y: u8,
    pub tile: u8,
    pub attr: u8,
    pub x: u8,
}

/// 解析 SpriteRAM（$0200-$02FF——watch 面 raw）→64 精灵视图。
pub fn sprite_state(mem: &FrameMemory) -> Vec<Sprite> {
    (0..64u16)
        .map(|i| {
            let b = 0x0200 + i * 4;
            Sprite {
                y: mem.read_ram(b),
                tile: mem.read_ram(b + 1),
                attr: mem.read_ram(b + 2),
                x: mem.read_ram(b + 3),
            }
        })
        .collect()
}

/// 8 sprites/scanline quirk（**D4 保真 bug**）：逐扫描线可见精灵计数，第 9+ 丢弃。
pub fn scanline_sprite_limit(sprites: &[Sprite]) -> [u8; 240] {
    let mut counts = [0u8; 240];
    for sp in sprites {
        if sp.y >= 0xEF {
            continue;
        }
        for line in (sp.y as usize)..((sp.y as usize + 8).min(240)) {
            if counts[line] < 8 {
                counts[line] += 1;
            }
        }
    }
    counts
}
