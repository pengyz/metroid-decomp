#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/bake_items.py —— 区特殊物品表（SpecItmsTbl）ROM 烘焙。

asm 内容锚（refs/m1disasm）：
- 表源：brinstar/items.asm `SpecItmsTbl_{AREA}: ; 01:A3D6`（各区 $9598 银行窗指针→表体）；
- 表格式（items.asm:38-54 注+A 级实录走表）：
  · Y 组：`<MapY> / .word <下一 Y 组地址>`（Y 序链表——$FFFF 终）
  · X 条目：`<MapX> / <下一条目相对偏移 | $FF 末条> / <条目数据…$00 终>`
  · 条目数据：it_Type（低半字节=SpawningRoutine 分派号）+参数（长度按型——
    it_Mellow(3)=0/Elevator(4)=1/PowerUp(2)=2（pu_XXX+位置字节）/Palette(0A)=0）
- 取证径：ROM 字节直读（iNES 偏移=16B 头+N*0x4000+($addr-$8000)）。

产出：crates/game-data/src/area/items.rs（扁平 SpecialItem 表——Y 序链表语义等价
（ScanForItems 扫描径：按 (map_y,map_x) 匹配当前图格））。
"""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ROM = ROOT / "rom" / "Metroid (USA).nes"
OUT = ROOT / "crates" / "game-data" / "src" / "area" / "items.rs"

# 各区表址（bank:addr——refs 各区 items.asm 标号自证）
AREA_TABLES = [
    ("BRINSTAR", 1, 0xA3D6),
    ("NORFAIR", 2, 0xA2D9),
    ("TOURIAN", 3, 0xA83B),
    ("KRAID", 4, 0xA26D),
    ("RIDLEY", 5, 0xA20D),
]

# it_Type（低半字节=ChooseSpawningRoutine 分派序）→ (名, 参数字节数)
# （norfair/items.asm `it_Door, $A0`=1 参实证；Squeept=2 参（| $40 高位旗+2 数据））
IT_TYPES = {
    0x01: ("Squeept", 2),
    0x02: ("PowerUp", 2),
    0x03: ("Mellow", 0),
    0x04: ("Elevator", 1),
    0x05: ("MBCannon", 1),
    0x06: ("MotherBrain", 0),
    0x07: ("Zebetite", 0),
    0x08: ("RinkaSpawner", 0),
    0x09: ("Door", 1),
    0x0A: ("Palette", 0),
}

PU_TYPES = {
    0x00: "BOMBS", 0x01: "HIGHJUMP", 0x02: "LONGBEAM", 0x03: "SCREWATTACK",
    0x04: "MARUMARI", 0x05: "VARIA", 0x06: "WAVEBEAM", 0x07: "ICEBEAM",
    0x08: "ENERGYTANK", 0x09: "MISSILES",
}


def prg_offset(bank: int, addr: int) -> int:
    return 16 + bank * 0x4000 + (addr - 0x8000)


def parse_area(data: bytes, bank: int, addr: int) -> list:
    """Y 链表+X 条目走表→[(map_y, map_x, item_type, args)]（表序）。"""
    items = []
    y_off = prg_offset(bank, addr)
    seen_y = set()
    while True:
        if y_off in seen_y:
            raise SystemExit(f"Y 链成环 @ {y_off:#x}")
        seen_y.add(y_off)
        map_y = data[y_off]
        next_y = data[y_off + 1] | (data[y_off + 2] << 8)
        x_off = y_off + 3
        seen_x = set()
        while True:
            if x_off in seen_x:
                raise SystemExit(f"X 链成环 @ {x_off:#x}")
            seen_x.add(x_off)
            map_x = data[x_off]
            next_rel = data[x_off + 1]
            p = x_off + 2
            while data[p] != 0x00:
                raw = data[p]
                t = raw & 0x0F
                if t not in IT_TYPES:
                    raise SystemExit(f"未知 it_Type {t:#x} @ {p:#x}")
                n_args = IT_TYPES[t][1]
                args = tuple(data[p + 1:p + 1 + n_args])
                items.append((map_y, map_x, raw, args))
                p += 1 + n_args
            if next_rel == 0xFF:
                break
            x_off += next_rel
        if next_y == 0xFFFF:
            break
        y_off = prg_offset(bank, next_y)
    return items


def main() -> int:
    data = ROM.read_bytes()
    L = []
    L.append("//! `game-data::area::items` —— 区特殊物品表（SpecItmsTbl）烘焙（tools/bake_items.py 生成）。")
    L.append("//!")
    L.append("//! 【A 级 ROM 取证】refs/m1disasm 各区 items.asm `SpecItmsTbl_{AREA}` 表体直读：")
    L.append("//! Y 组链表（MapY+下一组 word）+X 条目链（MapX+相对偏移（$FF 末条））+条目数据")
    L.append("//! （it_Type 低半字节=ChooseSpawningRoutine 分派（prg7:9322）；PowerUp=pu_XXX+位置")
    L.append("//! 字节；$00 终）。扁平 SpecialItem 表=Y 序链表语义等价（ScanForItems（ED98）扫描径）。")
    L.append("")
    L.append("/// 特项条目（map_y/map_x=图格坐标；item_type=it_ 分派号（1=Squeept/2=PowerUp/3=Mellow/")
    L.append("/// 4=Elevator/5=MB 炮/6=MB/7=Zebetite/8=Rinka/9=Door/A=调色）；args=类型参数。")
    L.append("#[derive(Clone, Copy, Debug, PartialEq, Eq)]")
    L.append("pub struct SpecialItem {")
    L.append("    pub map_y: u8,")
    L.append("    pub map_x: u8,")
    L.append("    pub item_type: u8,")
    L.append("    pub args: [u8; 3],")
    L.append("}")
    L.append("")
    for name, bank, addr in AREA_TABLES:
        items = parse_area(data, bank, addr)
        L.append(f"/// {name} 特项表（bank{bank:02d}:{addr:04X}——{len(items)} 条，ROM 走表自证）。")
        L.append(f"pub const ITEMS_{name}: &[SpecialItem] = &[")
        for (y, x, t, args) in items:
            a = list(args) + [0, 0, 0]
            tn = IT_TYPES[t & 0x0F][0]
            cmt = ""
            if (t & 0x0F) == 0x02:
                cmt = f" // {PU_TYPES.get(args[0], hex(args[0]))}"
            L.append(
                f"    SpecialItem {{ map_y: 0x{y:02X}, map_x: 0x{x:02X}, "
                f"item_type: 0x{t:02X}, args: [0x{a[0]:02X}, 0x{a[1]:02X}, 0x{a[2]:02X}] }},"
                f" // {tn}{cmt}")
        L.append("];")
        L.append("")
        print(f"{name}: {len(items)} 条")
        for (y, x, t, args) in items:
            print(f"  y={y:02X} x={x:02X} {IT_TYPES[t & 0x0F][0]:14s} args={[hex(v) for v in args]}")
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text("\n".join(L) + "\n", encoding="utf-8")
    print(f"烘焙 → {OUT}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
