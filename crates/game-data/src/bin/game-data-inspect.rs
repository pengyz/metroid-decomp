//! game-data-inspect —— brinstar 批解析对拍（task-17 r2）。
//!
//! 字节级复现命令（判据①）：
//!   cargo run -p game-data --bin game-data-inspect -- "rom/Metroid (USA).nes"
//!
//! 产出：47 房批解析 + 50 结构批解析 + L2 敌/门段（tail）长度/首字节聚类
//!（C→B 自证素材：变长段结构一致性实证）。

use game_data::{parse_ptr_table, parse_room, parse_structure, brinstar};
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
    println!("L4 元块表锚点：${:04X}（MetatileDefs，索引闭合校验待全表长度自证）",
        brinstar::METATILE_DEFS.1);
}
