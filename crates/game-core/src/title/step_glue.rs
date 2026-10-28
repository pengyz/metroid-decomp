//! title/step_glue —— 标题链运行时胶水（verify_shim 投影层退役迁移件）。
//!
//! 来源=旧 verify_shim title 适配段（git 23081ae2 谱系）机械变换：L2In→&TitleFace
//! （face 已含 ram/wram/a/x/y 面）/L2Out→GlueOut（writes+a/x/y）——**逐语句等价迁移**
//! （域调用（TitleSequence/FadeDriver/EffectDriver）与 memmap::title_state 写事件映射
//! 零改动）。投影契约（L2In/L2Out/写集三方全等）随 L2 基础设施退役；本件=运行时
//! 域件直调面（语义层唯一事实源=域件，胶水只做取数/映射）。

use super::TitleFace;
use crate::memmap;

/// 胶水出参（writes+a/x/y 透传——dispatch_step 侧组 TitleStep）。
pub(crate) struct GlueOut {
    pub writes: Vec<(u16, u8)>,
    pub a: u8,
    pub x: u8,
    pub y: u8,
}

/// 同址 last-wins 写事件收敛（旧 verify_shim::merge_writes 逐语句迁移）。
fn merge_writes(events: Vec<(u16, u8)>) -> Vec<(u16, u8)> {
    let mut map = std::collections::BTreeMap::new();
    for (a, v) in events {
        map.insert(a, v);
    }
    map.into_iter().collect()
}

fn title_helper_out(face: &TitleFace, o: crate::title::HelperOutcome) -> GlueOut {
    let mut events: Vec<(u16, u8)> = Vec::new();
    if let Some(p) = o.phase {
        events.push(memmap::title_state::write_event_phase(p));
    }
    if let Some(t) = o.timer3 {
        events.push(memmap::title_state::write_event_timer3(t));
    }
    if let Some((cb, c9)) = o.cleared {
        // ClearSpareMem 写序：`85 cb` 先、`85 c9` 后（事件序随 ROM；终值映射收敛）
        events.push(memmap::title_state::write_event_spare_cb(cb));
        events.push(memmap::title_state::write_event_spare_c9(c9));
    }
    GlueOut { a: o.a_out, x: face.x, y: face.y, writes: merge_writes(events) }
}

/// 批1 入口 0A/0B 适配（注册名 `sub_IncTitleRoutine` ← bank00:$806E，同码折叠；
/// stops=[] 哨兵窗——`e6 1f 60` 单 RTS）。
pub(crate) fn title_inc(face: &TitleFace) -> GlueOut {
    let mut t = crate::title::TitleSequence::new(memmap::title_state::read_phase(&face.ram));
    let o = t.advance(face.a);
    title_helper_out(face, o)
}

/// 批1 入口 10 适配（注册名 `sub_ClearSpareMem` ← bank00:$8068，**落穿共享尾**：
/// 清 $CB/$9…$C9 两址 + 推进 + A←0）。
pub(crate) fn title_clear_spare(face: &TitleFace) -> GlueOut {
    let mut t = crate::title::TitleSequence::new(memmap::title_state::read_phase(&face.ram));
    let o = t.clear_spare_and_advance();
    title_helper_out(face, o)
}

/// 批1 入口 0F 适配（注册名 `sub_DelayIntroReplay` ← bank00:$8283，推进+Timer3←$10）。
pub(crate) fn title_replay_delay(face: &TitleFace) -> GlueOut {
    let mut t = crate::title::TitleSequence::new(memmap::title_state::read_phase(&face.ram));
    let o = t.set_replay_delay();
    title_helper_out(face, o)
}

/// 批1 入口 13/14 适配（注册名 `sub_TitleRoutineReturn` ← bank00:$82F3 同码 `60` 空臂）。
pub(crate) fn title_no_op(face: &TitleFace) -> GlueOut {
    let t = crate::title::TitleSequence::new(memmap::title_state::read_phase(&face.ram));
    let o = t.no_op(face.a);
    title_helper_out(face, o)
}

// ---- task-23 批2·④编排簇适配（00/0C/11；01/12 L3 降级不设案——见 title 模块底注）----

/// 编排结果 → GlueOut（语义标签经 memmap::title_state 映射；WRAM 填充/精灵拷贝展开）。
fn title_orch_out(face: &TitleFace, o: crate::title::OrchestrationOutcome) -> GlueOut {
    let mut events: Vec<(u16, u8)> = Vec::new();
    if let Some(p) = o.phase {
        events.push(memmap::title_state::write_event_phase(p));
    }
    for (tag, v) in &o.writes {
        if *tag == "OBJECTS_CLEAR" {
            continue; // 展开在下方统一推
        }
        events.push(memmap::title_state::write_event_tag(tag, *v));
    }
    if o.writes.iter().any(|(t, _)| *t == "OBJECTS_CLEAR") {
        events.extend(memmap::title_state::objects_clear_events());
    }
    if let Some(fill) = &o.wram_fill {
        for (addr, v) in fill {
            events.push((*addr, *v));
        }
    }
    if let Some(sprites) = &o.sprite_copy {
        for (addr, v) in sprites {
            events.push((*addr, *v));
        }
    }
    GlueOut { a: o.a_out, x: o.x_out, y: o.y_out, writes: merge_writes(events) }
}

/// 批2 入口 00 适配（注册名 `sub_InitializeAfterReset` ← bank00:$8071，stops=[] 哨兵窗——
/// 尾经 LoadStarSprites $98AE 的 RTS）。
pub(crate) fn title_init_after_reset(face: &TitleFace) -> GlueOut {
    let mut t = crate::title::TitleSequence::new(memmap::title_state::read_phase(&face.ram));
    let o = t.init_after_reset(&face.wram[0x0E00..0x0EA0]); // 星精灵源 $6E00-$6E9F
    title_orch_out(face, o)
}

/// 批2 入口 0C 适配（注册名 `sub_ChangeIntroNameTable` ← bank00:$822E）。
pub(crate) fn title_change_nametable(face: &TitleFace) -> GlueOut {
    let mut t = crate::title::TitleSequence::new(memmap::title_state::read_phase(&face.ram));
    let ff = face.ram[crate::ram_map::PPUCTRL_ZP as usize & 0x7FF];
    let o = t.change_nametable(ff);
    title_orch_out(face, o)
}

/// 批2 入口 11 适配（注册名 `sub_PrepIntroRestart` ← bank00:$82A3，双路径）。
pub(crate) fn title_prep_restart(face: &TitleFace) -> GlueOut {
    let mut t = crate::title::TitleSequence::new(memmap::title_state::read_phase(&face.ram));
    let timer3 = face.ram[memmap::title_state::TIMER3 as usize & 0x7FF];
    let ff = face.ram[crate::ram_map::PPUCTRL_ZP as usize & 0x7FF];
    let music = face.ram[memmap::title_state::INTRO_MUSIC_RESTART as usize & 0x7FF];
    let o = t.prep_restart(timer3, ff, music);
    title_orch_out(face, o)
}

// ---- task-23 批3·①淡变族适配（FadeDriver 同形折叠 4 员→4 注册面）----

/// 淡变结果 → GlueOut（标签经 memmap 映射；y=None 透传）。
fn fade_out(face: &TitleFace, o: crate::title::FadeOutcome) -> GlueOut {
    let mut events: Vec<(u16, u8)> = Vec::new();
    if let Some(p) = o.phase {
        events.push(memmap::title_state::write_event_phase(p));
    }
    for (tag, v) in &o.writes {
        let tag = if *tag == "PENDING_PAL" { "PALETTE_PENDING" } else { *tag };
        let addr = match tag {
            "PALETTE_PENDING" => crate::ram_map::PALETTE_PENDING,
            "PALETTE_INDEX" => memmap::title_state::PALETTE_INDEX,
            "FADE_INDEX" => memmap::title_state::FADE_INDEX,
            "SPARE_CB" => memmap::title_state::SPARE_CB,
            "TIMER3" => memmap::title_state::TIMER3,
            "PPUCTRL_ZP" => crate::ram_map::PPUCTRL_ZP,
            other => panic!("fade 未知写标签 {other}——fail-loud"),
        };
        events.push((addr, *v));
    }
    GlueOut { a: o.a_out, x: face.x, y: o.y_out.unwrap_or(face.y), writes: merge_writes(events) }
}

/// 淡变面解码（zp 快照）。
fn fade_face(face: &TitleFace) -> crate::title::FadeFace {
    let r = |a: u16| face.ram[a as usize & 0x7FF];
    crate::title::FadeFace {
        timer3: r(memmap::title_state::TIMER3),
        frame_count: r(crate::ram_map::FRAME_COUNT),
        pal_index: r(memmap::title_state::PALETTE_INDEX),
        fade_index: r(memmap::title_state::FADE_INDEX),
        phase: r(crate::ram_map::TITLE_ROUTINE),
        ppuctrl: r(crate::ram_map::PPUCTRL_ZP),
    }
}

/// 批3 入口 02 适配（`sub_FadeInDelay` ← bank00:$80F9）。
pub(crate) fn title_fade_delay(face: &TitleFace) -> GlueOut {
    fade_out(face, crate::title::FadeDriver::DelaySetup.tick(&fade_face(face)))
}

/// 批3 入口 03 适配（`sub_METROIDFadeIn` ← bank00:$812C）。
pub(crate) fn title_metroid_fade_in(face: &TitleFace) -> GlueOut {
    fade_out(face, crate::title::FadeDriver::MetroidIn.tick(&fade_face(face)))
}

/// 批3 入口 0D 适配（`sub_MessageFadeIn` ← bank00:$8243）。
pub(crate) fn title_message_fade_in(face: &TitleFace) -> GlueOut {
    fade_out(face, crate::title::FadeDriver::MessageIn.tick(&fade_face(face)))
}

/// 批3 入口 0E 适配（`sub_MessageFadeOut` ← bank00:$8263）。
pub(crate) fn title_message_fade_out(face: &TitleFace) -> GlueOut {
    fade_out(face, crate::title::FadeDriver::MessageOut.tick(&fade_face(face)))
}

// ---- task-23 批4a·②特效簇适配（04/05/06）----

/// 特效结果 → GlueOut（槽/调色板标签经 memmap::title_state::effect_event）。
fn effect_out(face: &TitleFace, o: crate::title::EffectOutcome, init: Option<[u8; 11]>) -> GlueOut {
    let mut events: Vec<(u16, u8)> = Vec::new();
    if let Some(p) = o.phase {
        events.push(memmap::title_state::write_event_phase(p));
    }
    for tag in &o.writes {
        events.extend(memmap::title_state::effect_event(*tag));
    }
    if o.sparkle_init {
        if let Some(init) = init {
            for (i, v) in init.iter().enumerate() {
                events.push((memmap::title_state::INTRO_SPR_BASE + i as u16, *v));
                events.push((memmap::title_state::INTRO_SPR_BASE + memmap::title_state::INTRO_SPR_STRIDE + i as u16, *v));
            }
        }
        // LoadSparkleData 尾补丁（`a9 6b 8d b0 6e a9 dc 8d b3 6e`——注释≠语义样本：列表注
        // 描述补丁后状态，代码含同值装载+两笔硬编码补丁）
        events.push((memmap::title_state::INTRO_SPR_BASE + memmap::title_state::INTRO_SPR_STRIDE, 0x6B));
        events.push((memmap::title_state::INTRO_SPR_BASE + memmap::title_state::INTRO_SPR_STRIDE + 3, 0xDC));
        events.push(memmap::title_state::write_event_timer3(0x18));
    }
    GlueOut { a: o.a_out, x: o.x_out.unwrap_or(face.x), y: o.y_out.unwrap_or(face.y),
            writes: merge_writes(events) }
}

/// 批4a 入口 04 适配（`sub_LoadFlashTimer` ← bank00:$8142）。
pub(crate) fn title_flash_timer(face: &TitleFace) -> GlueOut {
    let f = crate::title::EffectFace {
        timer3: face.ram[memmap::title_state::TIMER3 as usize & 0x7FF],
        frame_count: face.ram[crate::ram_map::FRAME_COUNT as usize & 0x7FF],
        pal_index: face.ram[memmap::title_state::PALETTE_INDEX as usize & 0x7FF],
        phase: face.ram[crate::ram_map::TITLE_ROUTINE as usize & 0x7FF],
        complete0: 0, complete1: 0,
    };
    let mut top = crate::title::sparkle::SparkleSlot::default();
    let mut bottom = crate::title::sparkle::SparkleSlot::default();
    let o = crate::title::EffectDriver::FlashTimerSetup.tick(&f, &mut top, &mut bottom);
    effect_out(face, o, None)
}

/// 批4a 入口 05 适配（`sub_FlashEffect` ← bank00:$8109）。
pub(crate) fn title_flash_effect(face: &TitleFace) -> GlueOut {
    let f = crate::title::EffectFace {
        timer3: face.ram[memmap::title_state::TIMER3 as usize & 0x7FF],
        frame_count: face.ram[crate::ram_map::FRAME_COUNT as usize & 0x7FF],
        pal_index: face.ram[memmap::title_state::PALETTE_INDEX as usize & 0x7FF],
        phase: face.ram[crate::ram_map::TITLE_ROUTINE as usize & 0x7FF],
        complete0: 0, complete1: 0,
    };
    let mut top = crate::title::sparkle::SparkleSlot::default();
    let mut bottom = crate::title::sparkle::SparkleSlot::default();
    let o = crate::title::EffectDriver::FlashStep.tick(&f, &mut top, &mut bottom);
    effect_out(face, o, Some(crate::title::sparkle::INIT_SPARKLE_DATA))
}

/// 槽面解码（memmap 读→SparkleSlot）。
fn decode_sparkle(face: &TitleFace, base: u16) -> crate::title::sparkle::SparkleSlot {
    let r = |off: u16| face.wram.get(base as usize + off as usize - 0x6000).copied().unwrap_or(0);
    crate::title::sparkle::SparkleSlot {
        y: r(0), tile_id: r(1), attrib: r(2), x: r(3), index: r(4), next_delay: r(5),
        y_change: r(6), x_change: r(7), change_delay: r(8), byte_type: r(9), complete: r(0x0A),
    }
}

/// 批4a 入口 06 适配（`sub_METROIDSparkle` ← bank00:$814D）。
pub(crate) fn title_sparkle_step(face: &TitleFace) -> GlueOut {
    let f = crate::title::EffectFace {
        timer3: face.ram[memmap::title_state::TIMER3 as usize & 0x7FF],
        frame_count: face.ram[crate::ram_map::FRAME_COUNT as usize & 0x7FF],
        pal_index: 0,
        phase: face.ram[crate::ram_map::TITLE_ROUTINE as usize & 0x7FF],
        complete0: face.wram.get(0x0EAA).copied().unwrap_or(0),  // $6EAA=complete 槽0（字节真 `ad aa 6e`）
        complete1: face.wram.get(0x0EBA).copied().unwrap_or(0),  // $6EBA=complete 槽1（`2d ba 6e`）
    };
    let mut top = decode_sparkle(face, memmap::title_state::INTRO_SPR_BASE);
    let mut bottom = decode_sparkle(face, memmap::title_state::INTRO_SPR_BASE + memmap::title_state::INTRO_SPR_STRIDE);
    let o = crate::title::EffectDriver::SparkleStep.tick(&f, &mut top, &mut bottom);
    effect_out(face, o, None)
}

/// 批4b 入口 07 适配（`sub_METROIDFadeOut` ← bank00:$8163；FadeDriver::MetroidOut 臂）。
pub(crate) fn title_metroid_fade_out(face: &TitleFace) -> GlueOut {
    let f = crate::title::FadeFace {
        timer3: face.ram[memmap::title_state::TIMER3 as usize & 0x7FF],
        frame_count: face.ram[crate::ram_map::FRAME_COUNT as usize & 0x7FF],
        pal_index: 0,
        fade_index: face.ram[memmap::title_state::FADE_INDEX as usize & 0x7FF],
        phase: face.ram[crate::ram_map::TITLE_ROUTINE as usize & 0x7FF],
        ppuctrl: face.ram[crate::ram_map::PPUCTRL_ZP as usize & 0x7FF],
    };
    let o = crate::title::FadeDriver::MetroidOut.tick(&f);
    let mut events: Vec<(u16, u8)> = Vec::new();
    if let Some(p) = o.phase {
        events.push(memmap::title_state::write_event_phase(p));
    }
    for (tag, v) in &o.writes {
        let addr = match *tag {
            "TIMER3" => memmap::title_state::TIMER3,
            "CROSS_SLOW_DELAY" => memmap::title_state::CROSS_SLOW_DELAY,
            "SECOND_CROSS" => memmap::title_state::SECOND_CROSS,
            "PENDING_PAL" => crate::ram_map::PALETTE_PENDING,
            "FADE_INDEX" => memmap::title_state::FADE_INDEX,
            other => panic!("metroid_out 未知写标签 {other}——fail-loud"),
        };
        events.push((addr, *v));
    }
    for tag in &o.slot_writes {
        events.extend(memmap::title_state::effect_event(*tag));
    }
    GlueOut { a: o.a_out, x: o.x_out.unwrap_or(face.x), y: o.y_out.unwrap_or(face.y), writes: merge_writes(events) }
}

/// 批4b 入口 09 适配（`sub_MoreCrosshairs` ← bank00:$81D1；组合臂）。
pub(crate) fn title_crosshair_continue(face: &TitleFace) -> GlueOut {
    let r = |a: u16| face.ram[a as usize & 0x7FF];
    // $0305/$0306 属 RAM $0300 块（归一 $0000-$07FF 面）
    let anim_reset = face.ram[0x0305 & 0x7FF];
    let o = crate::title::crosshair_continue(
        r(crate::ram_map::FLASH_SCREEN), r(memmap::title_state::FLASH_INDEX),
        r(memmap::title_state::CROSS_EXPLODE_INDEX), r(memmap::title_state::IS_UPDATING_CROSS),
        r(crate::ram_map::FRAME_COUNT), anim_reset, r(crate::ram_map::TITLE_ROUTINE),
        &mut crate::title::sparkle::SparkleSlot::default(),
        &mut crate::title::sparkle::SparkleSlot::default());
    let mut events: Vec<(u16, u8)> = Vec::new();
    if let Some(p) = o.phase {
        events.push(memmap::title_state::write_event_phase(p));
    }
    for tag in &o.writes {
        events.extend(memmap::title_state::effect_event(*tag));
    }
    GlueOut { a: o.a_out, x: o.x_out.unwrap_or(face.x), y: o.y_out.unwrap_or(face.y),
            writes: merge_writes(events) }
}

/// 批4b 入口 08 适配（`sub_Crosshairs` ← bank00:$8182）。
pub(crate) fn title_crosshairs_step(face: &TitleFace) -> GlueOut {
    let r = |a: u16| face.ram[a as usize & 0x7FF];
    let mut slots: [crate::title::sparkle::CrossMissile; 8] = Default::default();
    for (i, s) in slots.iter_mut().enumerate() {
        let base = memmap::title_state::INTRO_SPR_BASE as usize + i * 0x10;
        let wr = |off: usize| face.wram.get(base + off - 0x6000).copied().unwrap_or(0);
        // 字节真偏移（机器操作数）：y0/tile1/attr2/x3/xChange6/yChange7/completeA/
        // speedX C/speedY D/dirX E/dirY F——`.db` 行=同偏移别名（0x06/0x07 两语义并用）
        s.y = wr(0); s.tile = wr(1); s.attr = wr(2); s.x = wr(3);
        s.complete = wr(0x0A); s.x_change = wr(0x06); s.y_change = wr(0x07);
        s.speed_x = wr(0x0C); s.speed_y = wr(0x0D);
        s.dir_x = wr(0x0E); s.dir_y = wr(0x0F);
    }
    let mut slow = r(memmap::title_state::CROSS_SLOW_DELAY);
    let mut spawn = r(0x00BD);
    let star_src: Vec<u8> = (0..0xA0u16)
        .map(|y| face.wram.get(0x0E00 + y as usize).copied().unwrap_or(0)).collect();
    let o = crate::title::crosshairs_step(
        r(crate::ram_map::FLASH_SCREEN), r(memmap::title_state::TIMER3),
        r(memmap::title_state::FLASH_INDEX), r(memmap::title_state::SECOND_CROSS),
        r(crate::ram_map::FRAME_COUNT), r(memmap::title_state::CROSS_EXPLODE_INDEX),
        r(memmap::title_state::IS_UPDATING_CROSS), r(crate::ram_map::TITLE_ROUTINE),
        &mut slots, &mut slow, &mut spawn, &star_src);
    let mut events: Vec<(u16, u8)> = Vec::new();
    if let Some(p) = o.phase {
        events.push(memmap::title_state::write_event_phase(p));
    }
    for tag in &o.writes {
        events.extend(memmap::title_state::effect_event(*tag));
    }
    GlueOut { a: o.a_out, x: o.x_out.unwrap_or(face.x), y: o.y_out.unwrap_or(face.y),
            writes: merge_writes(events) }
}

// ---- task-23 批5·⑤菜单簇适配（16/1A；jmp 出口走 stops 面）----

/// 菜单结果 → GlueOut（直址写面）。
fn menu_out(face: &TitleFace, o: crate::title::MenuOutcome) -> GlueOut {
    GlueOut { a: o.a_out, x: o.x_out.unwrap_or(face.x), y: o.y_out.unwrap_or(face.y),
            writes: merge_writes(o.writes) }
}

/// 批6a 入口 19 适配（`sub_DisplayPassword` ← bank00:9359 门面——直域调迁移）。
pub(crate) fn title_password_gate(face: &TitleFace) -> GlueOut {
    let timer3 = face.ram[memmap::title_state::TIMER3 as usize & 0x7FF];
    menu_out(face, crate::title::password_display_gate(timer3))
}
