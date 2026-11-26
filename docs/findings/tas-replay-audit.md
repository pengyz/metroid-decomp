# TAS 回放保真审计：按键流逐帧比对与路线实测（#103）

- **任务**：#103（tsk_01M8BMJBJSXYWFA340MJ9Y）。写面仅本文件；数据在 `analysis/`（可再生、gitignored）。
- **一句话结论**：**不可信**。影片解码与注入两者都逐帧零差（A 级），同一份输入在 Mesen2 里的**回放结果却与影片本意不符**：Samus 在 f3889 被打死（影片此时仍在按 `B+右` 走位），f4050 出现 GAME OVER、f4200 起停在 PASS WORD 屏；此后直到影片末帧 f120540 都没离开 Brinstar（`InArea=$74` 恒 0），末帧还停在密码屏。故现行管线里「emu 与 rust 在 f1–22894 逐字节相等」只能证明**两套实现在同一输入下同步**，不能当「影片通关/路线正确」的证据。
- **证据分级**：A=实跑产物/ROM 字节可复现；B=代码语境推断；C=假设（§8 单列）。

---

## 0. 卡面四问的结论

| # | 问题 | 结论 |
|---|---|---|
| ① | 影片解码 vs 注入 `$14` 是否逐帧一致 | **一致，零差**：独立解 fm2 与 `.inputlog` 比 106 362 帧，差 0（对齐式 `inputlog 帧 f ↔ fm2 第 f-1 行`）；Mesen 腿 RAM `$0014` 与 inputlog 比 106 222 帧，差 0。 |
| ② | emu 腿整段跑，`in_area` 变化序列 | **恒 0（Brinstar）**：f10000/20000/30000/40000/50000/60000/70000/80000 八个检查点全 0；f120540 末帧仍 0。 |
| ③ | 「该往右走却站着」的帧与两侧字节 | 不是「平移/位序」问题，而是**角色在 f3889 被打死**：影片在 f3800–4086 一直按 `0x81`（A+右）/`0x01`（右），期望继续走位；我方实测 f3889 血量归零，f4050 GAME OVER。 |
| ④ | 结论与根因层 | **不可信**；根因层=**回放引擎（Mesen2 与录制核心的帧相位/时序差异）**，影片解码与注入链被排除（§8）。修好前「逐帧一致」不得当通关证据。 |

---

## 1. 观测面与勘误（A 级）

### 1.1 `InArea` 是零页 `$74`，不是 `$079F`

```
$ grep -n "InArea" refs/m1disasm/SRC/constants_ram.asm
627:InArea                 db   ;$74     ;#$10(or #$00)=Brinstar, #$11=Norfair, #$12=Kraid hideout,-->
```

`$5A RoomNumber` 的 `0xFF` 是「当前没有房请求」的**稳态值**，不是异常：

```
$ sed -n '589p' refs/m1disasm/SRC/constants_ram.asm
RoomNumber             db   ;$5A     ;Room number currently being loaded. #$FF=no room requested.
```

⇒ 卡面早期用 `$079F`/`RoomNumber` 判路线会得到「看起来没动」的假象；本审计全部改用 `$0074`。

### 1.2 血量编码（判死亡用）

```
$ sed -n '826,830p' refs/m1disasm/SRC/constants_ram.asm
Health                 dw   ;$0106   ;Lower health digit in upper 4 bits.
; Health+1                     $0107   ;Upper health digit in lower 4 bits-->
                                       ;# of full tanks in upper 4 bits.
```

读法：`hi` 低半字节=十位、`lo` 高半字节=个位、`hi` 高半字节=满罐数。下文的「30/22/14/06」即按此解码（`0x0300→30`、`0x0220→22`、`0x0140→14`、`0x0060→6`）。

---

## 2. 影片解码保真（A 级）

脚本 `analysis/tas-audit/aud_movie.py`（可复跑）：

```
$ python3 analysis/tas-audit/aud_movie.py
fm2 rows 120540  inputlog frames 106362
alignment inputlog f <-> fm2 row f-1: mismatches 0  first []
```

解码口径：fm2 每帧行 `|cmd|P1列|P2列|…`，P 列为 8 字符 `RLDUTSBA`（`.`=未按）。逐字符置位后与 `tools/movie_import.py` 产物 `.inputlog` 比，**106 362 帧零差**。ROM 身份也与影片自述一致：

```
$ python3 -c "import hashlib,base64;b=open('rom/Metroid (USA).nes','rb').read();print(base64.b64encode(hashlib.md5(b[16:]).digest()).decode())"
stLZ7Wiz5eDSkFPqUlvTfA==     # == movies/lordtom-metroid-100p.fm2 的 romChecksum
```

⇒ **不是**「ROM 版本不同 / 位序解码错 / 帧偏移错」这类输入侧问题。

## 3. 注入保真（A 级）

脚本 `analysis/tas-audit/aud_inject.py`（读 `analysis/tas-main-f22895/emu/seg-*.jsonl` 的 `Joy1Status`）：

```
$ python3 analysis/tas-audit/aud_inject.py
frames compared 106222  range f1..f120540
diffs 0  first []
```

结合首帧落点（`$14` 首次非零在 f9 = inputlog 的 `0x10` Start），说明 `scenario_run.py` 的 `log_table → PLAN.schedule → emu.setInput` 链**同帧生效**，没有整帧位移。

## 4. 路线实测：八个检查点 + 末帧全在 Brinstar（A 级）

`analysis/tas-audit/aud_route.py` 读取各探针 trace（`snapshot` 承宇长篇回放的 `cp-010000…080000`，各续跑 20 帧）：

```
checkpoint  frame  InArea  RoomNum  MapPosX  ScrX  ScrollX  Joy1
010000 f10000 0 255 5 220 0 4
020000 f20000 0 255 5 220 0 2
030000 f30000 0 255 5 204 0 1
040000 f40000 0 255 5 204 0 2
050000 f50000 0 255 5 220 0 2
060000 f60000 0 255 5 220 0 1
070000 f70000 0 255 2 112 148 2
080000 f80000 0 255 3 143 45 1
```

末帧探针（`scn-probe-120540.txt`，续 `cp-120540`）：

```
120540 InArea 0 health 0 mapx 5 scrx 194 room None
120559 InArea 0 health 0 mapx 5 scrx 194 room None
```

⇒ 影片放完后仍在 Brinstar、血量 0。`Joy1` 列在 1/2/4 之间变化，说明**输入一直在喂**，不是注入中断。

## 5. 死亡链与画面证据（A 级）

基线长跑 `scn-pw.txt`（f0–4950，带 `watch $0106/$0107`）血量时间线：

```
f1 0  → f28 30 → f1759 22 → f2148 14 → f3244 06 → f3889 0
```

截图（`shots/`，sha256 前 16 位）：

| 帧 | 画面 | sha256 |
|---|---|---|
| f4000 | Brinstar gameplay（仍在走位） | `95df9fb69c6e…` |
| f4050 | **GAME OVER** | `f31fb8cdf382bf59` |
| f4200–f4950 | **PASS WORD**（密码 `002000 000000 / 00W01e 000a?d`） | `6c26673fd4d5f958` |
| f80100 | Brinstar 竖井，`EN··22`（能量 22） | `acd7e790b82f4fa1` |
| f120559 | **PASS WORD**（末帧；`000200 000000 / 000W0L8000-9`） | `bb6dd00dc5820990` |

与输入对照（`movie_import.read_inputlog`）：f3800–f3880 按 `0x81`（B+右）、f3881–f4086 按 `0x01`（右）、f4086 空、f4204 起 `0x41`（A+右）。**影片在死亡帧前后都在走位/连射，没有任何「死亡后回菜单」的输入编排**，所以死亡不是影片本意。

## 6. 相位假设的否证（A 级）

「会不会只是整表差 1 帧？」——把 inputlog 整表后移 1 帧（`analysis/tas-audit/lordtom-shift-minus1.inputlog`，前插一行）跑 20 000 帧：

```
f28 30 → f2192 22 → f2518 17 → f4843 12 → f4932 04 → f5145 0
last f19999: InArea 0, health 0
```

死亡只是被推到 f5145，之后同样停在 Brinstar。再测「前移 1 帧」（删首行，`lordtom-shift-realminus1.inputlog`）：f4599 血量已 0。⇒ **单纯的 ±1 帧相位修正救不回来**，问题在更底层的逐帧时序/引擎差异。

## 7. 跨影片对照（A 级）

同一 Mesen 宿主回放另外两部影片：

| 影片 | 录制核心 | 实测 |
|---|---|---|
| `lordtom-metroid-100p`（120 540 帧，100%） | FCEUX 9816 | f3889 死亡 → f4050 GAME OVER → 此后恒 Brinstar |
| `eien86-6725M`（28 408 帧） | quickerNES | f3863 死亡；f4000 截图 = GAME OVER（sha `f31fb8cdf382bf59`，与上同一屏） |
| `metroid-anyp-3666`（28 690 帧） | NesHawk | f4000 仍 gameplay（sha `e790e343a921a377`），仅掉血未死 |

两部高强度影片同样在 f≈3.87k 死亡、同一屏 GAME OVER；任天堂 any% 影片则活着。⇒ 不是「lordtom 这一部影片有问题」，而是**回放环境对帧级压榨型影片不保真**。

## 8. 根因层判定

| 层 | 判定 | 依据 |
|---|---|---|
| 影片解码 | **排除** | §2 零差 106 362 帧；ROM MD5 与影片 romChecksum 相同 |
| 注入链（scenario_run→Lua→setInput） | **排除** | §3 `$14` 与 inputlog 零差 106 222 帧、同帧生效 |
| Rust 端口 | **排除** | 本审计全程只跑 emu 腿；且 emu↔rust 在 f1–22894 逐字节相等 |
| **回放引擎（Mesen2 vs 录制核心）** | **判定所在层** | §5–§7：输入相同、ROM 相同，唯一变量是「谁在跑」；±1 相位不能救（§6） |
| 「emu↔rust 相等」的意义 | **收窄** | 它证明两套实现消费同一输入并同步，**不证明**影片路线/进度；用作通关证据需先有「回放保真」前提 |

C 级假设（未定谳，§9 列验证法）：① FCEUX/quickerNES 的 `setInput` 在帧内施加点与 Mesen 的 `inputPolled` 不同，造成**亚帧级**相位差（不是整帧）；② 上电 RAM 初值/APU 帧 IRQ 时序差异改变早期 RNG 与敌人行为；③ 影片对「满血 30 起步、8 点/次」的早期容错窗口极窄，任何子帧偏差都会在 f≈3.9k 累积成死亡。

## 9. 最小复现

```bash
# ① 解码保真
python3 analysis/tas-audit/aud_movie.py         # 期望 mismatches 0
# ② 注入保真
python3 analysis/tas-audit/aud_inject.py        # 期望 diffs 0
# ③ 死亡链（f0–4950，含 4000/4050/4200 截图）
python3 tools/scenario_run.py run analysis/tas-audit/scn-pw.txt --timeout 400
# ④ 路线八检查点
python3 analysis/tas-audit/aud_route.py
# ⑤ 末帧（续 cp-120540）
python3 tools/scenario_run.py run analysis/tas-audit/scn-probe-120540.txt --timeout 300
```

判据：③ 应看到 `shots/tas-audit-pw_f04050_f4050.png` 为 GAME OVER；⑤ 应看到 f120540 `InArea 0 / health 0` 且末帧截图为 PASS WORD。**若这两条不成立**，说明回放保真问题已被修掉，本结论需按新证据更正（原地标注日期）。

## 10. 对后续任务的影响（判据收窄）

1. 「TAS 逐帧对拍」在修好回放保真前，只能当**实现同步性**判据，不能当**进度/路线**判据；里程碑不得以「跑完 120 540 帧且双侧相等」宣布通关。
2. 需要一条**独立的路线锚**（不依赖影片）：例如用已验收的房间/门/进度规格生成「应有 `InArea`/房号序列」的期望表，或在 FCEUX 侧录一份带 `$74` 采样的参考轨迹。
3. `InArea` 观测统一用 `$0074`；`RoomNumber $5A=0xFF` 是稳态，不得当异常。
