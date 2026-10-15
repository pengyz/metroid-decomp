//! game-data-inspect —— brinstar 批解析对拍（task-17 r2）。
//!
//! 字节级复现命令（判据①）：
//!   cargo run -p game-data --bin game-data-inspect -- "rom/Metroid (USA).nes"
//!
//! 产出：47 房批解析 + 50 结构批解析 + L2 敌/门段（tail）长度/首字节聚类
//!（C→B 自证素材：变长段结构一致性实证）。

use game_data::{parse_enemy_anim, parse_enemy_anim_seq, parse_spec_items, parse_vram_struct, parse_ptr_table, parse_room, parse_structure, brinstar};
use std::collections::BTreeMap;
use std::env;
use std::fs;

fn at(addr: u16) -> usize {
    (addr.wrapping_sub(0x8000)) as usize
}

fn main() {
    let path = env::args().nth(1).expect("用法: game-data-inspect <rom.nes>");
    let rom = fs::read(&path).expect("rom 读取失败");
    let prg = &rom[0x10..0x10 + 0x20000];
    let b1 = &prg[0x4000..0x8000];

    // L1：房间指针表（47 词，m1disasm room_ptrs 注）
    let (ptrs, _) = parse_ptr_table(&b1[at(brinstar::ROOM_PTRS.1)..], 47).unwrap();
    let (sptrs, _) = parse_ptr_table(&b1[at(brinstar::STRUCT_PTRS.1)..], 50).unwrap();
    println!("L1 房间指针 47 词：[0]=${:04X}（期望 ${:04X}）", ptrs[0], brinstar::ROOM00.1);
    println!("L1 结构指针 50 词：[0]=${:04X}（期望 ${:04X}）", sptrs[0], brinstar::STRUCTURE00.1);
    assert_eq!(ptrs[0], brinstar::ROOM00.1, "L1 对拍失败");
    assert_eq!(sptrs[0], brinstar::STRUCTURE00.1, "L1 对拍失败");

    // L2 批解析：47 房 + tail 聚类（敌/门变长段 C→B 自证）
    let mut n_obj = 0usize;
    let mut tails: Vec<(u16, Vec<u8>)> = Vec::new();
    let mut tail_first: BTreeMap<u8, usize> = BTreeMap::new();
    let mut tail_len: BTreeMap<usize, usize> = BTreeMap::new();
    let mut bad = 0usize;
    for (i, w) in ptrs.iter().enumerate() {
        let o = at(*w);
        match parse_room(&b1[o..]) {
            Some((r, n)) => {
                n_obj += r.objects.len();
                *tail_len.entry(r.tail.len()).or_default() += 1;
                if let Some(&f) = r.tail.first() {
                    *tail_first.entry(f).or_default() += 1;
                }
                tails.push((*w, r.tail.clone()));
                let _ = n;
            }
            None => {
                bad += 1;
                println!("  解析失败：Room{:02X} @ ${:04X}", i, w);
            }
        }
    }
    println!("L2 批解析：47 房全通过={} 失败={} 对象段总数={}", bad == 0, bad, n_obj);
    println!("L2 tail 长度聚类（len: 房数）: {:?}", tail_len);
    println!("L2 tail 首字节聚类（type: 房数）: {:?}", tail_first);
    let ff_ok = tails.iter().filter(|(_, t)| t.last() == Some(&0xFF)).count();
    println!("L2 tail $FF 终止一致性: {}/47", ff_ok);

    // L3 批解析：50 结构
    let mut rows_total = 0usize;
    let mut tiles_total = 0usize;
    let mut s_bad = 0usize;
    for (i, w) in sptrs.iter().enumerate() {
        let o = at(*w);
        match parse_structure(&b1[o..]) {
            Some((rows, _n)) => {
                rows_total += rows.len();
                tiles_total += rows.iter().map(|r| r.len()).sum::<usize>();
            }
            None => {
                s_bad += 1;
                println!("  结构解析失败：Structure{:02X} @ ${:04X}", i, w);
            }
        }
    }
    println!("L3 批解析：50 结构全通过={} 失败={} 行={} 元块={}",
        s_bad == 0, s_bad, rows_total, tiles_total);
    // L4 索引闭合：结构引用的元块索引×4 < MetatileDefs 表长（按 m1disasm 91 行×4B ≈ 40 项估）
    println!("L4 元块表锚点：${:04X}", brinstar::METATILE_DEFS.1);
    // L3b：元块索引闭合（L4 判据④）：结构引用 max_idx < MetatileDefs 表长（65，asm 数得）
    let mut max_idx = 0u8;
    for w in &sptrs {
        let o = at(*w);
        if let Some((rows, _)) = parse_structure(&b1[o..]) {
            for row in rows {
                for &m in &row {
                    max_idx = max_idx.max(m);
                }
            }
        }
    }
    println!("L4 索引闭合：max 元块索引={} ×4={} < 表长 {}B → {}",
        max_idx, max_idx as usize * 4, brinstar::METATILE_COUNT * 4,
        if (max_idx as usize) < brinstar::METATILE_COUNT { "✓" } else { "✗ 违例" });
    assert!((max_idx as usize) < brinstar::METATILE_COUNT, "L4 索引闭合失败");

    // ③ palettes 模块批解析（VRAMStruct 线格式，macros.asm:32-56/71-73 锚证）
    let (pptrs, _) = parse_ptr_table(&b1[at(brinstar::PALETTE_PTRS.1)..], brinstar::PALETTE_COUNT).unwrap();
    println!("L5 调色板指针 {} 词：[0]=${:04X}（期望 ${:04X}）[1]=${:04X}（期望 ${:04X}）",
        brinstar::PALETTE_COUNT, pptrs[0], brinstar::PALETTE00.1, pptrs[1], brinstar::PALETTE01.1);
    assert_eq!(pptrs[0], brinstar::PALETTE00.1, "palette 对拍失败");
    assert_eq!(pptrs[1], brinstar::PALETTE01.1, "palette 对拍失败");
    let mut pal_ok = 0usize;
    let mut pal_bad = 0usize;
    let mut pal_bytes = 0usize;
    for w in &pptrs {
        let o = at(*w);
        match parse_vram_struct(&b1[o..]) {
            Some((recs, _n)) => {
                pal_ok += 1;
                for r in &recs {
                    let a = match r {
                        game_data::VramRecord::Data { ppu_addr, data } => {
                            pal_bytes += data.len();
                            *ppu_addr
                        }
                        game_data::VramRecord::Repeat { ppu_addr, .. } => *ppu_addr,
                    };
                    assert!(matches!(a, 0x3F00..=0x3F1F), "ppu 地址越界 ${:04X}", a);
                }
            }
            None => pal_bad += 1,
        }
    }
    println!("L5 批解析：28 调色板通过={} 失败={} 数据字节={}（ppu 址全落 $3Fxx 面 ✓）",
        pal_ok, pal_bad, pal_bytes);
    assert_eq!(pal_bad, 0, "palette 批解析失败");

    // ③ items 模块：asm 标签集互核（结构互核注：自建解码走址 ↔ m1disasm 标签坐标）
    const SPEC_ITMS: u16 = 0xA3D6; // items.asm:56 SpecItmsTbl_{AREA} ; 01:A3D6
    let (items, used) = parse_spec_items(&b1[at(SPEC_ITMS)..], SPEC_ITMS)
        .expect("items 解析失败");
    // asm 标签坐标（items.asm @y/@@x 注址全集，工具外人工抄录=互核基准）
    let asm_labels: &[u16] = &[
        0xA3D9, 0xA3DE, 0xA3E7, 0xA3ED, 0xA3F6, 0xA3FC, 0xA405, 0xA409,
        0xA412, 0xA418, 0xA41F, 0xA425, 0xA42D, 0xA433, 0xA43C,
    ];
    let mut walk_addrs: Vec<u16> = items.iter().map(|i| i.addr).collect();
    walk_addrs.sort_unstable();
    let mut lab = asm_labels.to_vec();
    lab.sort_unstable();
    println!("③ items 批解析：{} 条目（{}B）；asm 标签互核：walk={} vs labels={} → {}",
        items.len(), used, walk_addrs.len(), lab.len(),
        if walk_addrs == lab { "✓ 全合" } else { "✗ 分歧" });
    assert_eq!(walk_addrs, lab, "items 走址与 asm 标签分歧");
    let mut tdist = std::collections::BTreeMap::new();
    for i in &items {
        *tdist.entry(i.item_type).or_insert(0usize) += 1;
    }
    println!("③ items 类型分布（type:数，语义见 items.asm:45-54 注释 C 级）: {:?}", tdist);
    // ③b enemy_sprite_data：帧表 148 词 + EnAnim 逐标签入口互核（共享尾结构实证：
    // 标签=任意入口，非断链序列——互核=每 asm 标签均起合法链至 $FF）
    const FRAME_PTRS: u16 = 0x9DE0; // EnFramePtrTable1_{AREA} ; 01:9DE0
    const FRAME_COUNT: usize = 148;
    let (fptrs, _) = parse_ptr_table(&b1[at(FRAME_PTRS)..], FRAME_COUNT).unwrap();
    println!("③b 帧表 148 词：[0]=${:04X}（期望 $9FC2=EnFrame00 注址）→ {}",
        fptrs[0], if fptrs[0] == 0x9FC2 { "✓" } else { "✗" });
    assert_eq!(fptrs[0], 0x9FC2);
    // asm EnAnim 标签坐标集（enemy_sprite_data.asm:34-41 段抄录）——逐入口验证
    let asm_anim: &[u16] = &[0x9D7B, 0x9D7D, 0x9D7F, 0x9D81, 0x9D83, 0x9D85];
    let mut all_ok = true;
    for &a in asm_anim {
        match parse_enemy_anim(&b1[at(a)..], a) {
            Some((chain, _n)) => println!("  入口 ${:04X}: {} 帧 → $FF 终 ✓ {:?}", a,
                chain.frames.len(), chain.frames),
            None => { all_ok = false; println!("  入口 ${:04X}: ✗ 无 $FF 终", a); }
        }
    }
    println!("③b EnAnim 逐标签互核（共享尾模型）：{} 入口全合法={}",
        asm_anim.len(), all_ok);
    assert!(all_ok, "EnAnim 入口验证失败");
}
