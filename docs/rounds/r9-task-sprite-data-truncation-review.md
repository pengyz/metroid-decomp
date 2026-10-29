# R9 对抗复核 verdict——a1708910（SPRITE_DATA 烘焙截断修+DieDelay 偏移正）

- 复核员：verifier-scroll（证伪立场，非实施方）
- 日期：2026-10-29
- **verdict：needs_revision**（修复本体与数据面证据 PASS；**端到端火力验证面两处 FAIL**：
  f810 槽态证据不可复现+新火探针 case 空转（假绿面））

> 【2026-10-29 入库注（后续会话补）】本档两处方已由 03c72bfb 返工落实（双侧出弹实锤+
> case 载体入库），返工复审 pass 见 r9-task-fire-e2e-rework-review.md——打不出子弹
> 任务已收口。本档按原样入库作历史记录。

## 逐条复核

### 1. 截断实证核 —— PASS（A 级独立复现）

- **旧 blob_len 计漏**（代码读证）：旧形（a1708910~1）只计 `\$XX` 字面
  （`re.findall(r"\$[0-9A-Fa-f]{2}")`）——符号形 `.byte _id_ObjFrame_*` 行计 0 ⇒
  截断机制成立。旧式计数复算=**915**（与声明一致）；注解址跨 $8572→max $8B03 ⇒
  真域 **1457B**（新 blob_len=项计+址跨兜底，代码读证）。
- **帧 0x28 记录 ROM 直证（复现命令：python 读 rom/Metroid (USA).nes）**：
  `04 02 02 30 FF` 实锤于**bank1-5 的 $890A**（区域共享 bank，五处全同；
  文件偏移 0x491A 等）。**注记**：commit「bank7 基换算」措辞不准——固定 bank7
  （$C000 窗）$890A 处为 `84 93 84 94 AD`；记录在 $8000 窗区域 bank（1-5）。
  字节主张本身正确，bank 定位表述需更正。
- 重烘焙正确性：SPRITE_DATA=1457B ✓；抽查两条记录==ROM：
  blob[0x398]（=$890A）=`04 02 02 30 FF` ✓；blob[0x310]（=$8882）=`08 32 FF 41 0F`
  ==ROM $8882 ✓。
- DieDelay 偏移：constants_ram.asm:**924** `SamusProjectileDieDelay =
  Objects.0.data0F` ⇒ +0x0F ✓；Objects 槽stride 0x10 ⇒ 旧 +0x15 越槽（$3D0+0x15=
  $3E5，落 $E0 槽）实证 ✓。

### 2. 测有效性 —— PASS（unit 面）

- `frame_record_coverage`：断言 blob 覆盖 $890A 且五字节==ROM 记录——截断回归
  （915B）⇒ len 断言红 ✓（期望值硬编码=ROM hexdump，独立对拍锚）。
- `fire_write_set_full`：radiusY==2（来自帧记录读）——截断 ⇒ FF ⇒ 红 ✓。
- `burst_four_bullets`：DieDelay 槽内 +0x0F==0x0C 锚——旧 +0x15 形 ⇒ 红 ✓
  （注：alive≥1 断言单独偏弱，由前两测补强）。

### 3. 端到端火力验证面 —— **FAIL（needs_revision 项）**

- **①f810 槽态「全对齐」不可复现**：仓库内无 f810 采集对（cases 无 f810 点；
  analysis/traces+shots_rust 均无 f810 ram.hex）。commit 的「f810 槽态直比/
  f810 492→329px/f820=359」出自未归档的 ad-hoc 探针——**声称的 A 级证据不可
  从仓库复现**（违证据纪律「可复现指针」）。
- **②新 case-shootprobe-probe 空转（假绿面）**：实读双侧 ram.hex——fire-2/5/10/20
  四采集点**三弹槽（$03D0/$03E0/$03F0）双侧全零**（emu 侧 f420→f502 项目区
  $0300-03FF 亦无弹写入）⇒ B press 未产生弹（press 语义/帧相位问题）⇒ 5/0
  match 对「子弹可打出」**零证明力**（空对空匹配）。该 case 作为火力验证点
  入库=假绿，须修复场景（使弹真实存在并存活于采集窗）后重落。
- 缓解（非豁免）：leftwalk-probe2 l380 在 kick 修复后 492→626 已含 jump+fire 链
  间接面？——否（该 case 无 B press）；真实端到端火力验证目前**仅 unit 面**。

### 4. L4 无回退 + 其余 —— PASS

- 复跑 run-all：**39/58**（旧 34 精确保持+shootprobe 5 空转点）；门链五点/落地三点
  match；l380=626/l580=498/l780=531/l950=490/f950=732 保持（kick 修复值未动）。
- `cargo test --workspace`：**444 passed / 0 failed**（game-core **410**=407+3 ✓）。
- `gates/cli.py check`：**PASS=13 FAIL=0，CHECK: PASS**。

## 裁定

| 点 | 结论 |
|---|---|
| 1 截断实证（旧计漏 915/ROM $890A 记录/1457B 覆盖/DieDelay +0x0F） | pass（注记：bank 定位措辞「bank7」不准，实为区域 bank1-5） |
| 2 测有效性（unit 三锚 mutation） | pass |
| 3 端到端火力验证（f810 不可复现+shootprobe 空转假绿） | **fail** |
| 4 L4 无回退+验证（444/0；gates 13/0） | pass |

**verdict = needs_revision**（范围限定）：修复本体（blob 截断+DieDelay 偏移）证据
充分**不返工**；返工项=①修 case-shootprobe-probe 使弹真实存活于采集窗（查 press
语义/开火帧相位；或改 hold b/贴紧 capture 帧）重落非空转探针；②f810 槽态证据
归档（双侧 ram.hex+case 落库）或改由①的新探针承担端到端面。完成后复审即 pass。

Evidence: ROM hexdump $890A（bank1-5 五处 `04 02 02 30 FF`；bank7 窗异值反证）；
sprite_data.rs（1457B+blob[0x398]/[0x310] 抽查）；constants_ram.asm:924；
bake_sprite_data.py 新旧 diff；analysis/traces+shots_rust shootprobe ram.hex 逐槽
实读（三槽双侧全零）；shots_rust/matrix.md（39/58）；cargo test（444/0）；
gates check（PASS=13 FAIL=0）。
