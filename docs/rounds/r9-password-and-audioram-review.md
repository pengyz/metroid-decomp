# r9 对抗复核：密码 FAQ 自证 + 音频 RAM 契约（commit 44fc01c9）

- 复核人：verifier（证伪立场，独立重导，未修改任何实施产物）
- 对象：`44fc01c9`（docs/findings/password-faq-verify.md、docs/findings/audio-ram-contract.md、tools/oracle/accuracycoin/metroid_wram_dump.lua）
- 复核日期：2026-11-09
- **verdict：needs_revision（件1 §3 C-in quirk 三项主张全部被实测推翻；其余各点成立）**
- Confidence: 95（推翻性结论有双向控制实验支撑）

## 总裁决

| 件 | 复核点 | 判定 |
|---|---|---|
| 1 | ① 16字节/checksum/shift 三主张独立重导 | ✓ 成立 |
| 1 | ② Justin Bailey mini-6502 真实性 + VALID/PB 语义 | ✓ 成立（附 2 条纪律备注） |
| 1 | ③ C-in quirk（$9011 ror×3 / scramble / unscramble） | ✗ **全部推翻 → needs_revision** |
| 2 | ④ 静态谱抽查 + 动态 3 地址核对 + 零冲突 | ✓ 成立（3 处表述性瑕疵） |
| 2 | ⑤ Mesen 纪律（无残留/锁释放/memType=8） | ✓ 成立（附当前环境备注） |
| ⑥ | 44fc01c9 范围审计（3 文件/零逻辑/零 rename/SFX B 级） | ✓ 成立 |

**未通过原因**：`password-faq-verify.md` §3"quirk（登记，勿改 Stage A 行为）"整段与 commit message
的对应主张（$9011 ror×3 → PB[3I+2] bit6 依赖进入进位态；scramble/unscramble 首轮外部 C 污染）
经双向控制实验证伪——**decode/encode 输出对进入进位完全不敏感**。该段被写为可登记 BUG 的
行为怪癖（A 级表述+ROM 字节证据），属"行为主张错报"，按复核纪律 = needs_revision。
修正方向：将该段改写为"瞬态吞 C 但被 AND 掩码 / temp 回写丢弃，**行为无影响**（已实验证实）"，
且**禁止**据此落 bug-ledger。

---

## 件 1：密码（password-faq-verify.md）

### ① 独立重导：三项结构主张 —— ✓ 全部成立

换算式独立提取（bank0: `file=0x10+(cpu-0x8000)`；bank7: `file=0x10+7*0x4000+(cpu-0xC000)`），
与档案逐条对表（A 级实测摘录）：

```
$8C7A: A9 00 A0 0F 99 88 69          (CalculatePassword, 清 $6988..)
$8DDE: AD B2 69 ... 20 4E 8E 20 21 8E CD 99 69 F0 03   (Unscramble→Checksum→CMP $6999→beq valid)
$8D00: 20 00 C0 A5 2E 29 0F F0      (shift = RandomNumber1($2E)&#$0F, =0 重摇)
$8E17: 20 21 8E 8D 99 69 20 2D 8E   (Checksum→STA $6999→Scramble)
$8E21: A0 10 A9 00 18 79 88 69 88 10 F9 60   (LDY #$10; CLC; ADC $6988,Y; DEY; BPL → 17 字节含 shift)
$8E2D: ... A2 00 A0 0F 7E 88 69 E8 88 10 F9 66 00 A5 00 8D 88 69 C6 02 D0 E5 60   (ROR ring ×16 + temp)
$8E4E: ... A2 0F 3E 88 69 CA 10 FA 26 00 A5 00 8D 97 69 ...   (ROL ring ×16 + temp)
$9011: B9 9A 69 6A 6A 6A 29 C0 85 00 B9 9B 69 05 00 60   (ror×3 + AND #$C0)
bank7 $C2C5: 0A 0A 0A 0A 60   ($C2BF: 4A×4 60 — Amul16/Adiv16)
```

- 16 字节数据面（$6988-$6997，scramble ring X=0..15/15..0 各 16 次）✓
- checksum = sum($6988..$6998)&0xFF（17 字节含 shift）@ $6999，ValidatePassword `cmp $6999` ✓
- shift byte @ $6998 = RandomNumber1&#$0F ✓
- m1disasm 侧 `constants_ram.asm` PasswordByte @$6988 布局、`prg0_title.asm` 地址均对上 ✓

### ② Justin Bailey 铁证审查 —— ✓ 成立（真实 ROM 字节执行，非算法重实现）

审计结论：
- `analysis/password_verify/emu.py` 为逐 opcode 分派的 mini-6502（`op = self.rd(pc)` 直接取 ROM
  字节），JSR/RTS 用硬件栈语义（push pc+3/等效），C/N/Z 全程维护。**无任何密码算法重实现**。
- 插桩 trace 实测（decode∘encode 全程）：231 条指令执行，PC 覆盖 bank0 `$8E17-$9011` +
  bank7 `$C2BF/$C2C5`，执行过的 opcode 全集 = {05,09,0A,10,18,20,26,29,2A,3E,4A,60,66,6A,79,
  7E,85,88,8D,A0,A2,A5,A9,AD,B9,C6,CA,D0,E8}，全部实现正确；`STY zp`(0x84) 虽被错误实现为
  LDY 语义，但 **从未执行到**（无害，仍建议修正注释或删除该分支）。
- 本复核独立复跑 `justin_bailey_check.py` 输出与档案一致：PB=`A6 F3 8E A4 B9 65 24 A9 D1 7F
  FF…`，shift=$6998=FF，checksum=$6999=FF，sum=$6988..$6998=FF → VALID；PB[8]=D1 bit7=1
  （suitless ✓）、PB[10]=FF（MissileCount=255 ✓）；500/500 恒等 PASS。另补测档案未做的
  反方向恒等：`decode∘encode(JB) == JB 字符序列` ✓。
- 备注 1（不降级）：档案称"VALID（checksum 校验通过）"，实际 VALID 判定在 Python 侧做
  `sum==checksum` 比较（emu 未实现 ValidatePassword 的 `CMP abs`(0xCD)，$8DDE 未被执行）。
  checksum 字节本身是 ROM 产物，比较对象均出自 ROM，结论不失实，但"全程 ROM 字节执行"
  的表述对此一处有轻微拔高，建议在档案补一句说明。
- 备注 2：`pwlib.py:2` `sys.path.insert(0,'/tmp/pwverify')` — 实际执行的 emu 来自 /tmp 暂存
  目录（当前与仓库副本逐字节一致，diff 已验）。复现脆弱（/tmp 清空即失败），建议改为相对
  导入仓库内 `analysis/password_verify/emu.py`。

### ③ C-in quirk 主张 —— ✗ 推翻（needs_revision 触发点）

档案三项主张： (a) `$9011 ror×3` 吞外部 C → `PB[3I+2] bit6` 依赖进入进位态（C 值=前序
$C2C5 ASL×4 末次进位）； (b) scramble 首轮外部 C 污染一位； (c) unscramble 首轮外部 C 污染一位。
且建议落 BUG 台账。

**双向控制实验（A 级实测，mini-6502 执行 ROM 原字节）**：

```
$9011 standalone, 输入 $699A=AB $699B=CD:
  C_in=0 → STA $00=C0 ; C_in=1 → STA $00=C0   （输出逐位相同）
decode(JB) 在 $8F60/$8E4E 入口分别强置 C=0/1，四组 (0,0)(0,1)(1,0)(1,1)：
  18 字节输出完全相同 → PB[5] bit6 与任何进入进位无关
encode(JB 数据) 在 $8E17 入口强置 C=0/1：24 字符输出完全相同
```

机理（逐指令推演 + 实验吻合）：
1. `$9011`: ROR×3 后 C_in 落在 **bit5**（bit7=原v1、bit6=原v0），随即 `AND #$C0` 把 bit5
   清零——外部进位被**掩码丢弃**，PB[3I+2] bit6 = $699A,Y 的 bit0，恒定。档案声称的
   "bit6 依赖进入进位态" 不成立；且 Consolidate 调用序里 $9011 入口的 C 实为前序 $9001
   内 `LSR×2` 的进位（LSR 覆写了 $C2C5 的 ASL 进位），"C 值 = ASL×4 末次进位"的因果链也错。
2. scramble `$8E2D`: 首轮 ROR $6988,X(X=0) 虽吞 C_in，但该次写回的 $6988 随后被
   `ROR $00 → LDA $00 → STA $6988`（temp 值）**覆盖**——C_in 只进入被丢弃的字节，保留数据
   每轮 = 纯 128-bit 环转 + 环内进位。unscramble `$8E4E` 同构（被覆盖的是环内轮转的 $6997）。
3. 同理 `$8F42`（encode 侧 ROL×3）的 C_in 落 bit2 后被 `AND #3` 掩码——ROM 各处均无行为级
   C 泄漏。**结论：这是"设计上吞掉并丢弃外部进位"的正常代码，不构成怪癖，不应登记 BUG。**

判罚依据：该段以 A 级表述+ROM 字节证据写入"已证实"区并建议落 bug-ledger，属对新发现的
ROM 行为主张的错报（verifier 纪律：错报 = needs_revision）。修正 = 原地标注更正，改写为
"瞬态吞 C、行为无影响（2026-11-09 复核实验证实）"，并撤销 bug-ledger 建议。其余 §1-§5
与 FAQ 对表结论不受影响。

---

## 件 2：音频 RAM（audio-ram-contract.md + metroid_wram_dump.lua）

### ④ 静态谱 × 动态 dump 抽查 —— ✓ 成立

- `refs/m1disasm/SRC/constants_ram.asm:977` `.enum $0600` ✓；符号地址逐条对上
  （$0624-27 RepeatCounter、$062B NoteLengthTblOffset、$0638-3B InstrID、$0640-43
  InstrDelay、$066C-6F VolumeData/SpareMem066E 等）；`sound_engine.asm` 共 28 处 $06 字面量，
  抽查 5 处均吻合（:2040 $062B-$0637 初始化注释、:1664/:1965 `lda/sta MusicSQ1VolumeData,x`
  与 $066E 无意写溢出主张一致、:2543/:2700 $0640-43/$0638-3B 复位注释、:2712-2715 init 代码
  实写 $062B..$0643）✓
- 动态 3 地址（4 帧实测）：`$0638-3B`: 03 0D 03 09 → 08 1D 07 03 → 0A 1F 13 03 → 0E 27 24 04
  （曲序指针持续推进 ✓）；`$0624-27`: 01 01 03 27 → 02 02 02 23 → 01 01 01 20 → 00 00 03 1D
  ✓；`$066E`: 00 → 3C → 3C → 1C（"无意写从不读"注释被动态证实 ✓）
- 零冲突复核：程序化 diff 四帧，活跃地址共 **27** 个，全部落在档案标记"动态证据 ✓"的行内；
  档案标"仅静态/恒 0"的字段（$0610-17、$0652-55、$0660-69、$0670-7B、$0680-8D 等）无一活跃
  → **"静态谱与动态行为零冲突"主张成立** ✓
- 表述性瑕疵 3 处（不触发判罚，建议顺手更正）：(i) 档案称"22 个地址活跃"，实测 27；
  (ii) $0624-27 证据栏写"单调递减"，实测 t600→t900 是 01→02（递增后递减），非单调；
  (iii) $063C-3F 证据串"01→06/1B/05/1F"与帧序对位混乱（t900=06 1B 05 01）。

### ⑤ Mesen 纪律 —— ✓ 成立

- `metroid_wram_dump.lua` 显式 `emu.read(addr, 8)`（memType=8 逐点传参）✓；帧计数/Start 注入
  与档案描述一致 ✓；文件头注明 Adapted from accuracycoin_dump.lua ✓
- 锁 `analysis/.scenario.lock` 当前**空闲**（flock -n 可获）✓；本复核时点无该 commit 遗留
  Mesen 进程 ✓（当前另有一例 **其他场景** 的 Mesen/Xvfb（case-001 lua，23:39 启动）在跑，
  属并行工作非本卡残留，不构成对"跑后无残留"主张的反证，但复核时无法直接重现其空态，
  以锁空闲+无 metroid_wram_dump 进程为准）。

### ⑥ 防弱化 + 范围审计 —— ✓ 成立

- `git show --stat 44fc01c9`：恰好 3 个文件（2 findings + 1 lua，+239 行），全为 docs/tools，
  零 crates/、零 data/naming/、零 reference/、零门禁改动；无 rename、无业务逻辑 ✓
- SFX 恒 0 字段诚实标注"仅静态"/B 级，未伪装 A ✓；$BBA8 分歧维持原档案未越权仲裁 ✓
- 唯一降分点仍归件1：§3 quirk 段属"带 ROM 字节证据的行为主张错报"（见 ③），非 C 级伪装。

## 遗留（不阻塞 verdict）

1. `password-faq-verify.md` §3 需原地更正（verdict=needs_revision 的唯一修复项）。
2. `pwlib.py` 的 `/tmp/pwverify` 导入路径应指向仓库内副本（可再生性卫生，建议下卡处理）。
3. `gates/cli.py status` 在本复核环境无输出（挂起至超时）——疑似环境问题，建议 lead 关注，
   不影响本卡（未动门禁）。
