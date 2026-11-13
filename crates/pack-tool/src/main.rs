//! `pack-tool` —— ROM → 资产包提取工具（**开发期独立工具**，不随游戏发行）。
//!
//! 用户流程（见 `docs/spec/data-code-separation.md`）：
//! ```text
//! cargo run -p pack-tool -- extract --rom "rom/Metroid (USA).nes" --out assets/metroid.pack
//! cargo run -p pack-tool -- verify  --rom "rom/Metroid (USA).nes" --pack assets/metroid.pack
//! cargo run -p pack-tool -- list    --pack assets/metroid.pack
//! ```
//!
//! 纪律：本 crate 是唯一允许读 ROM 文件的地方（`tools-boundary` 门禁）；提取逻辑本身
//! 在 `game-data` 的 `extract` feature，本文件只做 IO 与报告。

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use game_data::extract::{
    build_meta_json, sections, write_pack, Section, EXPECTED_ROM_CRC32, EXPECTED_ROM_SIZE,
};
use game_data::pack::{crc32, PackKey, PackView};

fn usage() -> String {
    "\
用法:
  pack-tool extract --rom <rom 路径> --out <pack 路径>   从 ROM 提取资产包
  pack-tool verify  --rom <rom 路径> --pack <pack 路径>  逐段校验包内容与 ROM 一致
  pack-tool list    --pack <pack 路径>                   列出包内段

选项:
  --force    ROM 身份不符时仍然提取（默认拒绝——用错版本 ROM 会得到错数据）"
        .to_string()
}

struct Args {
    cmd: String,
    rom: Option<PathBuf>,
    out: Option<PathBuf>,
    pack: Option<PathBuf>,
    force: bool,
}

fn parse_args(argv: Vec<String>) -> Result<Args, String> {
    let mut it = argv.into_iter().skip(1);
    let cmd = it.next().ok_or_else(|| "缺少子命令".to_string())?;
    let mut a = Args { cmd, rom: None, out: None, pack: None, force: false };
    while let Some(x) = it.next() {
        match x.as_str() {
            "--rom" => a.rom = Some(PathBuf::from(it.next().ok_or("--rom 缺参数")?)),
            "--out" => a.out = Some(PathBuf::from(it.next().ok_or("--out 缺参数")?)),
            "--pack" => a.pack = Some(PathBuf::from(it.next().ok_or("--pack 缺参数")?)),
            "--force" => a.force = true,
            other => return Err(format!("未知参数：{other}")),
        }
    }
    Ok(a)
}

fn read_rom(p: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(p).map_err(|e| format!("读 ROM 失败 {}：{e}", p.display()))
}

fn header_line(rom: &[u8]) -> String {
    format!(
        "ROM {} B  crc32={:08X}（期望 {} B / crc32={:08X}）",
        rom.len(),
        crc32(rom),
        EXPECTED_ROM_SIZE,
        EXPECTED_ROM_CRC32
    )
}

fn cmd_extract(a: &Args) -> Result<(), String> {
    let rom_path = a.rom.as_ref().ok_or("extract 需要 --rom")?;
    let out = a.out.as_ref().ok_or("extract 需要 --out")?;
    let rom = read_rom(rom_path)?;
    println!("{}", header_line(&rom));
    let got_crc = crc32(&rom);
    if (got_crc != EXPECTED_ROM_CRC32 || rom.len() as u32 != EXPECTED_ROM_SIZE) && !a.force {
        return Err(format!(
            "ROM 身份不符（crc32={got_crc:08X}）——请使用 Metroid (USA).nes；\n\
             确实要用别的 ROM 请加 --force（后果自负：数据可能错位）"
        ));
    }
    let mut secs = sections(&rom)?;
    // meta.json 段（id 0）：描述+校验用，运行时从不读。**派生投影**，由提取侧常量生成。
    let meta = build_meta_json(&secs, got_crc, rom.len() as u32)?;
    secs.insert(0, Section { key: PackKey::Meta, data: meta });
    let bytes = write_pack(&secs, got_crc, rom.len() as u32);
    if let Some(dir) = out.parent() {
        if !dir.as_os_str().is_empty() {
            std::fs::create_dir_all(dir).map_err(|e| format!("建目录失败 {}：{e}", dir.display()))?;
        }
    }
    std::fs::write(out, &bytes).map_err(|e| format!("写包失败 {}：{e}", out.display()))?;
    println!("已写入 {}（{} B，{} 段）", out.display(), bytes.len(), secs.len());
    for s in &secs {
        println!(
            "  {:<10} {:>7} B  crc32={:08X}",
            s.key.name(),
            s.data.len(),
            crc32(&s.data)
        );
    }
    Ok(())
}

fn load_pack(p: &Path) -> Result<PackView, String> {
    let bytes = std::fs::read(p).map_err(|e| format!("读包失败 {}：{e}", p.display()))?;
    let leaked: &'static [u8] = Box::leak(bytes.into_boxed_slice());
    PackView::parse(leaked, None).map_err(|e| format!("解析包失败：{e}"))
}

fn cmd_verify(a: &Args) -> Result<(), String> {
    let rom_path = a.rom.as_ref().ok_or("verify 需要 --rom")?;
    let pack_path = a.pack.as_ref().ok_or("verify 需要 --pack")?;
    let rom = read_rom(rom_path)?;
    let pack = load_pack(pack_path)?;
    let (pc, ps) = pack.rom_identity();
    let rc = crc32(&rom);
    println!("{}", header_line(&rom));
    println!("包 ROM 身份：{} B / crc32={:08X}", ps, pc);
    if pc != rc || ps != rom.len() as u32 {
        return Err("包与给定 ROM 身份不符——请用同一份 ROM 重新提取".to_string());
    }
    let secs = sections(&rom)?;
    let mut bad = 0usize;
    for s in &secs {
        match pack.get(s.key) {
            None => {
                println!("  {:<10} 缺段（包版本落后于代码？）", s.key.name());
                bad += 1;
            }
            Some(got) => {
                if got == &s.data[..] {
                    println!("  {:<10} OK  {:>7} B  crc32={:08X}", s.key.name(), got.len(), crc32(got));
                } else {
                    println!(
                        "  {:<10} 不一致：包 {} B / ROM 重算 {} B",
                        s.key.name(),
                        got.len(),
                        s.data.len()
                    );
                    bad += 1;
                }
            }
        }
    }
    // meta 描述段是提取侧常量（`extract.rs::seg_meta`）的派生投影，不进 `sections()`
    // 的重算面——但它一旦漂移，`list`/审计读到的是旧描述（表长、elem 与真实段不符）。
    // 故在此按当前代码重算并逐字节比对：改提取器而忘重生成包 ⇒ 当场红。
    match pack.get(PackKey::Meta) {
        None => {
            println!("  {:<10} 缺段（meta 描述段）", "meta.json");
            bad += 1;
        }
        Some(got) => {
            let want = build_meta_json(&secs, rc, rom.len() as u32)?;
            if got == &want[..] {
                println!("  {:<10} OK  {:>7} B  crc32={:08X}", "meta.json", got.len(), crc32(got));
            } else {
                println!(
                    "  {:<10} 不一致：包 {} B / 代码重算 {} B（提取侧常量已改而包未重生成）",
                    "meta.json",
                    got.len(),
                    want.len()
                );
                bad += 1;
            }
        }
    }
    // 包里多出来的段只提示（代码可能还不认识）
    for (id, _, len) in pack.entries() {
        if PackKey::from_id(*id).is_none() {
            println!("  未知段 id={id}（{len} B）——代码未使用，忽略");
        }
    }
    if bad > 0 {
        return Err(format!("{bad} 段未通过校验"));
    }
    println!("全部段校验通过");
    Ok(())
}

fn cmd_list(a: &Args) -> Result<(), String> {
    let pack_path = a.pack.as_ref().ok_or("list 需要 --pack")?;
    let pack = load_pack(pack_path)?;
    let (c, s) = pack.rom_identity();
    println!("包 ROM 身份：{} B / crc32={:08X}；{} 段", s, c, pack.section_count());
    if let Some(meta) = pack.get(PackKey::Meta) {
        println!("---- meta.json ----");
        print!("{}", String::from_utf8_lossy(meta));
    }
    for (id, off, len) in pack.entries() {
        let name = PackKey::from_id(*id).map(|k| k.name()).unwrap_or("<未知>");
        println!("  id={:<3} {:<10} off={:<8} len={}", id, name, off, len);
    }
    Ok(())
}

fn main() -> ExitCode {
    let args = match parse_args(std::env::args().collect()) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{e}\n\n{}", usage());
            return ExitCode::from(2);
        }
    };
    let r = match args.cmd.as_str() {
        "extract" => cmd_extract(&args),
        "verify" => cmd_verify(&args),
        "list" => cmd_list(&args),
        "help" | "--help" | "-h" => {
            println!("{}", usage());
            Ok(())
        }
        other => Err(format!("未知子命令：{other}\n\n{}", usage())),
    };
    match r {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("错误：{e}");
            ExitCode::from(1)
        }
    }
}
