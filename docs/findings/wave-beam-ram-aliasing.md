# Wave beam 伪寄存器与 `$05D0-$05FF` 空洞（取证）

> 结论级别：**A**（ROM 字节 + 符号表 + 反汇编三处互证）。
> 触发：Task 5 账本里 `$05D0-$05FF` 曾按 `constants_ram.asm:991` 的注释登记成
> "wave beam 3 槽"。复核指出该注释与符号表/ROM 矛盾，本条记录订正后的取证。

## 1. 符号表说 wave beam 3 个变量在 `$0500-$0502`，不在 `$05D0-$05FF`

`refs/m1disasm/out/M1_NES_NTSC.sym:11992-11994`：

```
00000500 WeaponWaveInstrID
00000501 WeaponWaveInstrTimer
00000502 WeaponWaveDir
00000600 MusicSQ1PeriodLow      <- 下一个有地址的符号直接跳到 $0600
```

## 2. ROM 代码确实只写 `$0500-$0502`

| 位置 | 偏移（headerless bank7） | 字节 | 解码 |
|---|---|---|---|
| bank7 `$D35B` | 119659 | `99 02 05` | `STA $0502,Y`（wave beam 方向表尾） |
| bank7 `$D51F` | 120111 | `FE 00 05` | `INC $0500,X`（wave beam 指令 ID 计数） |

复现：

```bash
python3 - <<'PY'
rom = open('rom/Metroid (USA).nes','rb').read()
off = lambda a, bank=7: 16 + bank*0x4000 + (a - 0xC000)
for a in (0xD35B, 0xD51F):
    print(hex(a), rom[off(a):off(a)+3].hex(' '))
PY
```

## 3. `$0500-$05CF` 是 TileBlasts，`$05D0-$05FF` 在整份符号表里没有任何条目

`M1_NES_NTSC.sym` 有 `TileBlasts.0` … `TileBlasts.12` 共 13 个槽，每槽 16 B，
起点 `$0500`（`:11708`），槽 12 最后一个有名字段 `TileBlasts.12.spare0B = $05CB`（`:11847`），
随后地址轴直接跳到 `$0600`（`:11995`）。`$05D0-$05FF` 区间内**无符号、无代码引用**
（`grep -nE '^000005[de]|^000005f' refs/m1disasm/out/M1_NES_NTSC.sym` 无输出）。

因此 `constants_ram.asm:991` 的 "3 slots" 注释是投影错误（它把 `$0500-$0502` 的三个
wave beam 变量误标成 `$05D0-$05FF`），**不能作为账本 refs**。

## 4. 对账本的影响

- `$0500-$05CF`（208 B）：refs = `constants_ram.asm:986`（`.enum` 声明）+ `.sym:11708`
  （槽 0 起点）+ `crates/game-core/src/world/statue.rs:30` / `world/room_draw.rs:381`（Rust 真源）。
- `$05D0-$05FF`（48 B）：refs = `.sym:11847` / `.sym:11995`（符号表在 `$05CB` 后直接跳到 `$0600`
  ⇒ 该区间无任何符号认领）。账本不写"wave beam 槽"之类的语义声称。
- 该 48 B 仍记 Undeclared：既没有 typed 真源，也没有可引用的符号语义，登记为未解释面是诚实状态。
