# 密码 FAQ 自证：144 位结构 C→A（2026-11-09）

来源线索：`docs/findings/community-sources-scan-2026-11-09.md` §3.4（C 级）——
社区密码 FAQ 主张 Metroid 密码 = 144 位（128 数据 + 8 校验和 + 8 shift byte）。
纪律：第三方/社区文本不背书（D5），以下全部以 ROM 字节 + m1disasm（已证与 ROM 逐字节等价，
见 `docs/findings/rom-disasm-equivalence.md`）自证。**零游戏逻辑改动、零 rename。**

## 结论（verdict）：FAQ 主张成立，升 A 级

FAQ 的三项结构主张逐项对表全部与 ROM 实测一致；另完成 Justin Bailey 全程 ROM 字节级
roundtrip 铁证（checksum VALID + 语义吻合）。

---

## 已证实（A 级）

### 1. 数据面 16 字节 = 128 bit —— 成立

- 布局（`refs/m1disasm/SRC/prg0_title.asm` `CalculatePassword` @ `00:8C7A`，以及
  `constants_ram.asm:1237` `PasswordByte ds $12 @$6988`）：
  - `$6988-$698F`：UniqueItemHistory 位域（`ProcessUniqueItems` @ `00:8B79` 写入）
  - `$6990` PB+8 = InArea&0x3F | suitless<<7（`prg0_title.asm` `00:8C9E`-`00:8CA9`）
  - `$6991` PB+9 = SamusGear；`$6992` PB+10 = MissileCount；`$6993-$6996` PB+11..14 = SamusAge；
    `$6997` PB+15 = Kraid/Ridley 雕像状态（bit7/6/5/4）
  - 打乱/校验只作用于这 16 字节：`PasswordScramble` @ `00:8E2D` 循环 `ror PasswordByte,x` x=0..15
    （16 字节 ring）；`UnscramblePassword` @ `00:8E4E` 同构 rol。
- 字节级复现：`xxd -s $((0x10+0x8E2D-0x8000)) -l 48 "rom/Metroid (USA).nes"` 可见
  `AD 98 69 85 02 ... 7E 88 69 ...`（LDA $6998/STA $02/…/ROR $6988,X）。

### 2. 校验和字节 8 bit @ `$6999` —— 成立

- `PasswordChecksumAndScramble` @ `00:8E17`：`JSR $8E21` → `STA $6999` → `JSR $8E2D`。
- `PasswordChecksum` @ `00:8E21`：`LDY #$10; LDA #$00`，循环 `CLC; ADC $6988,Y; DEY; BPL`
  —— 对 `$6988..$6998` 共 17 字节（**含 shift byte**）求 8-bit 和。
- 校验方向（`ValidatePassword` @ `00:8DDE`）：`jsr UnscramblePassword; jsr PasswordChecksum;
  cmp PasswordByte+$11; beq @validPassword`。

### 3. shift byte 8 bit @ `$6998`，语义 = 128-bit rotate 计数 —— 成立

- 生成：`CalculatePassword` `@loop_random @ 00:8D00`：`RandomNumber1 & #$0F`（非 0，≤15）存 `$6998`。
- 消费：`PasswordScramble`/`UnscramblePassword` 以 `Temp02_ScrambleCount` 循环执行
  整 16 字节 ring 的 ROR/ROL `$6998` 次。
- **quirk（登记，勿改 Stage A 行为）**：两例程的 ring 闭合都不完美——scramble 用
  `Temp00`（$00）暂存 `$6988` 原值再 ROR 回写；unscramble 用 `Temp00` 暂存 `$6997`。
  首轮旋转的外部 C_in 来自前序指令残留（对 scramble 是 checksum ADC 的进位链尾态），
  被第一个 ROR/ROL 吞掉——即旋转链的首位 bit 不是纯 ring，而是被外部 C 污染一位。
  另 `$9011 TwoLowerAndSixLower`（decode 侧 chars→bytes）的 ror×3 链同样吞入进入时 C
  （其值 = 前序 `Amul16 @ $C2C5` ASL×4 的最后一次进位），使 `PasswordByte[3I+2]` 的 bit6
  依赖进入时的进位态。→ 建议 D4 登记 `BUG-` 条目（本卡不动 bug-ledger，见遗留假设）。
  证据：ROM 字节 `xxd -s $((0x10+0x9011-0x8000)) -l 20 "rom/Metroid (USA).nes"`
  → `B9 9A 69 6A 6A 6A 29 85 ...`（无 CLC/SEC，直接 ROR×3）。

### 4. 144 bit = 18 字节 = 24 字符 × 6 bit —— 成立

- `LoadPasswordChar @ 00:8E6C`：6 组 × 4 字符，每组把 3 个 PasswordByte（24 bit）切成
  4×6 bit；`ConsolidatePassword @ 00:8F60` 为其逆。6×4×6 = 144 bit = 18 字节 = 128+8+8。
- 字符表：6-bit 值 = tile 码（`PasswordRowTbl @ $9998` + `refs/m1disasm/SRC/charmap.tbl`）：
  0x00-09='0'-'9'，0x0A-0x23='A'-'Z'，0x24-0x3D='a'-'z'，0x3E='?'，0x3F='-'。
  屏幕显示 4 组×6，`PasswordToScreen @ $93C6 → $93F9` 把 PasswordChar 原值作 tile 写 PPU。

### 5. 加分项：Justin Bailey 全程 ROM 字节级 roundtrip —— 成立

用 mini-6502（flag-accurate，**直接执行 ROM 原始字节**，无算法重实现）解码社区名密码
`JUSTIN BAILEY ------ ------`（24 字符 = J U S T I N B A I L E Y + 12×'-'(0x3F)）：

```
PasswordByte[0..15] : A6 F3 8E A4 B9 65 24 A9 D1 7F FF FF FF FF FF FF
shift  $6998        : FF
checksum $6999      : FF
sum($6988..$6998)   : FF   ->  VALID（checksum 校验通过）
PB[8]=D1 -> suitless(bit7)=1（粉色无甲 Samus ✓ 与社区记载一致）
PB[10]=FF -> MissileCount=255（✓ 社区记载 Justin Bailey 给满导弹）
encode∘decode∘encode == encode (500 随机 canonical 密码): PASS
```

复现：`cd analysis/password_verify && python3 justin_bailey_check.py`
（`emu.py`：mini-6502，bank 映射 prg[cpu-0x8000]（<0xC000）/ prg[0x1C000+cpu-0xC000]（≥0xC000）；
`pwlib.py`：decode=$8F60+$8E4E、encode=$8E17+$8E6C。）

---

## 与 FAQ 主张逐项对表

| FAQ 主张 | ROM 实测 | 判定 |
|---|---|---|
| 128 bit 数据 | `$6988-$6997` 16 字节，scramble 只动这 16 字节 | ✅ |
| 8 bit checksum | `$6999` = sum($6988..$6998)&0xFF，`ValidatePassword` cmp 校验 | ✅ |
| 8 bit shift byte | `$6998` = RandomNumber1&0x0F，rotate 次数 | ✅ |
| 合计 144 位 | 18 字节 × 8 = 24 字符 × 6 | ✅ |

## 假设区（未证实，不进结论）

1. **BUG 台账编号未落**：§3 的 C-in quirk（decode 侧 `$9011` bit6 依赖进入进位、
   scramble/unscramble 首轮外部 C 污染）按 D4 属「Stage A 保真保留」怪癖，本卡受
   「只写 docs/findings/」约束未动 `docs/bug-ledger.md`，待补 `BUG-` 条目。
2. Justin Bailey 的 `PB[0..7]` 位域里含有若干「垃圾 item bit」（如 PB[5] bit6 来自
   C-quirk）——游戏不校验 item 合法性、只查 checksum，故有效。是否与真实游戏机
   上该密码的行为逐位一致，待 Mesen 差分（本卡未跑 gameplay trace）。
3. shift byte 值域声称「1..15」来自 `& #$0F; beq 重摇`（`00:8D00`）——成立；
   但 Justin Bailey 解出 shift=0xFF（玩家手输密码不受生成侧约束），游戏照单全收
   （rotate 255 次），FAQ 未提及此差异，如实登记。
4. NAR PASSWORD（`NARPASSWORDTbl @ 00:8E07`）走 `ValidatePassword` 旁路（bypass checksum），
   与 FAQ 主张无冲突，未深入。

## 证据等级

- 全部结论 A 级（ROM 字节偏移 + hexdump/模拟器执行可复现）。
- m1disasm 符号名（PasswordChecksum 等）仅作线索引用；语义由 §1-§4 的 ROM 字节自证。

## 遗留假设清单（回执用）

- C-in quirks 待落 `docs/bug-ledger.md`（BUG-xxx）。
- Justin Bailey 与真机 gameplay trace 的逐帧差分未做（可选加分，不影响 A 级结论）。
- 复现脚本现居 `analysis/password_verify/`（可再生工作区，不入库）；本文已内联足够
  的字节级证据与命令。
