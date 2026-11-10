# r9 对抗复核——Justin Bailey 战役卡 A（emu 腿）+卡 B（rust 密码径）

- 复核人：verifier（cc52c47c 会话，独立证伪立场）
- 复核对象：ff6d2188（卡 A）/ a211747c + 85879f07 + 40779905（卡 B）/ 3e0b7b21（基线上调）
- **verdict：pass ｜ Confidence: 93**（阻塞问题：无；3 条非阻塞备注已收口）

## 逐项实录（A 级）

### ① asm 对读独立重导 — 全部 ✓
- $8F60 Consolidate 三子程序（prg0_title.asm:2191-2228）逐位等价；TwoLowerAndSixLower 三连 ror+`and #$C0` 手推：Cin 落 bit5 被 $C0 掩掉，净效果=((c&3)<<6)|c1 ≡ password.rs:120 ✓
- $8DDE 分支序（:2001-2010 NAR 命中→`bne @validPassword` 直达 clc，不 Unscramble 不 checksum；否则 Unscramble+checksum）≡ password.rs:204-211 ✓
- $8E4E/$8E2D rol/ror 环：stale-carry 被末段 ror/rol Temp00 挤出=纯 128-bit 环；count=0⇒256 轮（dec/bne 体先执行，:2069-2070）✓
- $92D4 InitializeGame（:2699-2758）逐行 vs title_bridge.rs:357-414 ✓；**07:CA30-CA3F ROM 亲验逐字节** `02 03 05 04 06 48 48 20 96 CA AD 84 68 10 0D 29` ≡ BANK_TABLE 常量（表外索引 quirk 照烤属实）✓
- $8D12 LoadPasswordData：JB=PB[8]&$80/InArea=PB[8]&$3F（asm 第三方注释"first 5 bits"为错，ROM 字节 `and #$3F` 为准）✓

### ② NAR 字节勘误（推翻性主张重点打）— ✓ 成立
ROM 亲验 bank0 文件偏移 0xDEE：`00:8DDE = AD B2 69 D0 14 = LDA $69B2(NARPASSWORD); BNE →$8DF7`（恰落 jsr Unscramble）；NARPASSWORDTbl 00:8E07 亲验=`"NARPASSWORD00000"` charmap。

### ③ 1014 组对拍独立性 — ✓ 真 ROM 字节执行
参考径 analysis/password_verify/emu.py（218 行 flag-accurate mini-6502，rd() 直读 prg，unsupported op raise）——非自我对照。verifier 独立复跑 3 组向量（JB 锚/随机/全零）与提交向量表逐字节一致；分布 valid 504/invalid 510、NAR 5+10、shift 极值 {00,01,7F,80,FF} 全覆盖 ✓

### ④ emu 场景 — ✓ 三路互证
帧号表与停机报一致；trace 独立解包：$699A=JB tile 序列、$6988-$6999=A6 F3…、$6878=7F/$6879=FF/$69B3=01/$74=11(Norfair) ✓；frames.jsonl sha256=527fd56d1ab503bb… ✓；线性 65 格独立 asm 验证（00:9264-926C）✓（未 Mesen 现场重放——ROM 执行+trace+字节比对三路互证替代，留痕）

### ⑤ G4 删除合法性 — ✓
被删臂在 dispatch match 深路径臂之后，构造性不可达，无第三入口（TitleRoutine 单入口 dispatch）。

### ⑥ 门禁与防弱化 — ✓
ff6d2188..3e0b7b21 零 gates/ 变更；基线仅 rust_test_count 517→524 sanctioned；独立复跑 cargo test --workspace 520 passed 0 failed；gates check PASS=13 FAIL=0（TOLERATED 2 项均历史账，非本卡新增）。

### ⑦ 出生态锚对齐 — ✓
tests/password_init_parity.rs 102 组 ROM 全径对拍逐字段断言（jib/inarea/gear/missile/nar/y/x/pb）。

## 非阻塞备注（已收口）
1. password.rs password_checksum 的 carry≠asm 末轮进位语义——【2026-11-10 r9 更正】doc 注释已原地更正（防未来消费者误用；现行调用方全丢弃，行为无影响）。
2. 卡 A「prg0_title.asm L9269」引用格式易误读——场景文件头注已改为 `prg0_title.asm:2620 附近，标签 L9269（地址 00:9264-926C）`。
3. 未 Mesen 现场重放（三路互证替代）——本条即留痕，无需动作。

## 整改 commit
见复核收口 commit（注释更正+场景头注格式+本档入库）。
