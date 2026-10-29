# 流送内容映射考证（续战①——A 级全链实录（2026-10-28））

> 纪律：先通读 refs/m1disasm（bank:addr 级）后修改——每条规则附 asm 行号证据。
> 背景：左向列流已修（ac951fe2（$4A←ScrollDir））+bog 归因已正（d667d6cc）——
> 本族=载房/流送内容映射链。

## 1. 全链实录（asm（A 级（行号锚）））

| 件 | 址（行） | 语义 |
|---|---|---|
| ScrollLeft/Right | $E6A7/$E6D2（:7836/:7868） | ScrollDir 2/3；ScrollX 越界（0（左）/FF→00（右））⇒GetRoomNum+ToggleNameTable（**PPUCTRL^=3**（双位翻））；每 px⇒CheckUpdateNameTable |
| CheckUpdateNameTable | $E54A（:7587） | SetupRoom 首件；RoomNumber≠$FF⇒RTS；横⇒Horizontal |
| CheckUpdateNameTableHorizontal | $E701（:7906） | ScrollX&7==阈（右 0/左 7）；**ScrollDir==TempScrollDir($4A)**（防重）；idx=(ScrollX&0xF8)/8→UpdateNameTable |
| UpdateNameTable | $E590（:7652） | GetNameAddrs（PPU $20/$2C+RoomRAM $60/$64（GetNT=(PPUCTRL^ScrollDir)&1））；横滚控制位 $1E\|$80（inc32/长 30）；RoomRAM[idx+row*32]→PPU[idx+row*32] |
| GetNameTableAtScrollDir | $EB85（:8853） | (PPUCTRL_ZP^ScrollDir)&1（0=NT0/1=NT3（四象限对角模型（EB84 注块））） |
| SetupRoom | $EA2B（:8578） | $FF→RTS/$FE→占位/$F0+→AttribTableWrite/其余载房：RoomPtrTable→RoomPtr→首字节=RoomPalette→SelectRoomRAM→InitTables→DrawRoom |
| SelectRoomRAM | $EA05（:8617 附近） | GetNT→**asl asl**→ora #$60⇒RoomRAMPtr+1（$6000（NT0）/$6400（NT3）） |
| DrawRoom | $EAAA（:8666） | 对象环：$FF→**EndOfRoom**/$FE→占位/$FD→EndOfObjs（敌门环）/其余→DrawObject |
| DrawObject | $EA60（:8609） | 位=%yyyyxxxx；y 单元→**+$40/单元（2 行）**；x 单元→**×2 瓦**；StructPtrTable[idx]→DrawStruct；RoomPtr+=3 |
| DrawStruct | $EF8C（:9715） | 首字节 $FF⇒出；⇒DrawStructRow |
| DrawStructRow | $EF13（:9590） | 行头：低 nibble=元块计（**0⇒16**）/高 nibble=x 偏移（×2 瓦）；行满（ptr&$1F==0）⇒提前 AdvanceRow |
| DrawMetatile | $EF2B（:9625） | 元块 idx×4→**MetatileDefs[base+k]→TilePosTable[x]（$21/$20/$01/$00（x=3..0 逆）⇒写序=UL,UR,LL,LR（自然序）**）；UpdateAttrib；ptr+=2 |
| EndOfRoom | $EAF4（:8727；commit 锚 :8725 偏 2 行更正（复核 r9）） | **RoomNumber←$F0+TempScrollDir($4A)←ScrollDir**（双写）+滚向尾径 kick |
| AttribTableWrite/RoomFinished | $EA13/$EA26（:8556/:8572） | $F0-$F3 渐写→$FF |

## 2. rust 对照（映射差异）

- 元块层（TILE_POS=[0x00,0x01,0x20,0x21]（UL,UR,LL,LR）+METATILE_DEFS 序）——**同形 ✓**。
- SelectRoomRAM（$60/$64 择）——**同形 ✓**；DrawObject（y×$40+x×2）——**同形 ✓**；
  DrawStructRow（nibble 语义）——**同形 ✓**；GetNT 公式——**同形 ✓**。
- **差异 ①（实证）**：draw_room 0xFF 直终径**内联 $5A=$F0 漏 $4A←ScrollDir**——asm
  双径（DrawRoom $FF/EnemyLoop $FF）同汇 EndOfRoom（双写）——enemy_loop 尾径经
  end_of_room()（ac951fe2 修）/直终径漏⇒**流送防重判永跳⇒内容映射锁死**（f1200/
  002-f1000 族根因候选（以该径终的房））。**已修**（径汇 end_of_room）。
- 测试留档：静态 ROOM_STREAMS 不可注径（弱测（等价断言））——asm 锚为证（径汇形）。

## 3. 遗留疑点（若 ①修后残差）

- InitTables（$EFF8）attrib 初始化形；UpdateAttrib 位计算；RenderState 环模型
  （ring_x mod 512→半区）与流送半区选择的一致性（f1200 端态族）。

Evidence: prg7_engine.asm 行号锚全列+room.rs/render 对照——A 级原文。
Confidence: 92


## 4. CHR 瓦内容径卡（2026-10-28（续（证伪更正·二）））

【自我推翻前证（D4 证伪文化）】前节「UpdateAttrib hi=$63 固定 quirk」=**截读误判**
——asm 写尾 `lda Temp00_RoomRAMPtr+1 / and #$04 / ora Temp02+1 / sta Temp02+1`
（$EF9E 内（跳段漏读））⇒终 hi=**$63|(ptr_hi&4)**（原 rust 形即正）——2563b02b
「quirk 修」=误改回归（已撤正）。
【链全验（A 级）】提取面三件套逐字节对拍全过（MetatileDefs/STRUCTURES/ROOM_STREAMS
（bank1 基））+GFX 面双表对拍全过（GFX_INFO 203B/GFX_DATA 29/29 块）+draw_struct
决定性实验（struct 8（顶壁垂石）rust 输出 vs 独立 asm 语义解释器期望格=**逐格全同**）
+bg fetch 基正（PPUCTRL bit4→$1000（boot $FF=0x90））+载序正（[3,4,5,6,25,22] 同 asm）
+覆盖图（BrinBG1 $00-15/CREBG1 $20-65/CREBG2 $80-FF/Title 残留 $66-7F/零 $16-1F）
——结构瓦集全落三块验证区（缺口/残留假说死）。
【残差定性】f1000/f1150=顶壁/柱纹差（336/363 瓦=邻域无同形（同形异色/异内容））
——静态链全验仍散⇒候选=**emu 侧 PPU 时序**（上传/滚动中途采样（lead 预告分类））或
渲染色彩链（attr 组选（r16 修后（已核））——停待裁定（不强修）。


## 5. 终结判（Mesen2 PPU 真值逐瓦核（2026-10-28））

【方法】双侧 dump 同构 12KB+35B（scenario_capture.lua::dumpPpu（mt=9=PPU 总线
（ppu_probe3 实证（$2000=FF/$0100=7B）））+rust_screenshot 同构导出）——case-002
f1000 时刻逐字节比对。
【四面结果（A 级）】**CHR 8192/8192 全同+NT0 名表 960/960 全同+NT0 属性 64/64 全同+
调色板 32/32 全同**——内容面（名表/CHR/属性/色）**零分歧**。
【定性=分支(a)】名表/CHR 内容同⇒**残差=渲染/采样时序类**（非内容/非上传/非映射）：
11267px 真差源=截屏时刻渲染状态（精灵叠放帧位/细滚相位/采样瞬间（中途态））——
按 lead 预告分类「不强修」——**f1000/f1150 记账收口（TOLERATED（D4 BUG 台账））**。
【附】scroll/ctrl 3B：rust=81 00 90（PPUSCROLL X=$81/PPUCTRL=$90（bit4=表1））——
emu 侧 getState 字段名容错回退 0（不可信（Mesen2 ppu 字段名待核（不影响定性
（四面内容同已充分）））。


## 6. $EAF4 尾径首列 kick 全径（接缝定向卡（2026-10-29））

【asm 实录（A 级）】EndOfRoom（$EAF4（:8725））：
```
EndOfRoom:                  ; 07:EAF4
    stx RoomNumber          ; RoomNumber←$F0（attr 表尾命令）
    lda ScrollDir
    sta TempScrollDir       ; $4A←ScrollDir（防重判基准（ac951fe2 修面）
    and #$02
    bne Lx224
        jmp EndOfRoomVertical   ; 竖滚尾径（首行 kick）
Lx224:
        jmp EndOfRoomHorizontal ; 横滚尾径（**首列 kick**）
```
EndOfRoomHorizontal（$E70C（:7906-7913））：
```
    ldx ScrollDir
    cpx TempScrollDir
    bne RTS_X196            ; 防重（$4A 同向）
    lda ScrollX
    and #$F8
    jsr Adiv8               ; Temp00=(ScrollX&0xF8)/8（列 idx）
    sta Temp00_RoomRAMPtr
    lda #$00
    jmp UpdateNameTable     ; **首列流送**（$E590（30 行竖写））
```
【kick=载房窗内件】EndOfRoom=DrawRoom 对象环 $FF 尾/EnemyLoop $FF 尾同汇——载房
（setup_room→DrawRoom）执行链内⇒**kick 落载房窗**（emu 首列随房即出（l380 emu 有柱））。
【移出史（证伪更正）】ac951fe2 期 kick 移出载房窗（room.rs:488-491 自注）——理由=
当时 stream_column_kick 内含 iter_bogged(2)（goal-r22 形）落入载房窗与 more_init
frame-gate [1,0,0,1] 校准冲突——**d667d6cc 已证伪 goal-r22 逐列写 bog 归因**（摘除
逐写 bog）⇒**冲突前提失效**（kick 现价=0 bog）——恢复 kick=asm 忠实形（缝根因修）。


## 7. 帧记录验址口径更正（2026-10-29（复核注记（bank7 基换算误称）））

> 【原地注】前期记录中「bank7 基换算」措辞**更正**：帧 0x28 记录（$890A）实锤形=
> **区域 bank1-5 五处 $890A 全同**（common_sprite_data.asm 区 bank 共载（:1311
> .include 形）——五 bank 逐字节一致）+**bank7 窗同址字节反证**（bank7 $890A 处
> 字节≠04 02 02 30 FF（非本体（反证排除 bank7 基）））。验址口径=区域 bank 共载
> 形（bank1-5 同址全同）非 bank7 换算。


## 8. 端到端火力验证归档（2026-10-29（复核返工（可复现载体）））

【场景面】case-001 +f815/f825 采点（f810 原点）+case-shootprobe-probe（照抄
case-001 输入形（press 310/350 start+450-550 right+600-650 left+700 a+**800 b**（历史
已证出弹相位）））。
【双侧弹槽实测（f810 相位）】case-001 f810：emu[st=01 r=0202 scr=01 x=AC dd=03]/rust
[st=01 r=0202 scr=01 x=B0 dd=02]——**双侧出弹 ✓**（x/dd=1 帧相位残（FC 域））；
shootprobe fire-10 同证（emu[01/AC]/rust[01/B0]）。
【逐字节归档】f815/f825 槽区（$3D0-$3EF）**32/32 全同**（死后态逐字节对齐（修前
死态发散（radius FF/onScreen 0）））。
【复现载体（仓库可复现链）】①cases/case-001-…+case-shootprobe-probe.txt（已入库）
②`python3 tools/l4_diff_run.py run-all`（双侧 .ram.hex 生成（scenario_capture.lua::
dumpRam+rust_screenshot（已入库）））③比对=上式逐字节（hex 直比（大小写已统一 %02X））。
【修前/修后对照】修前 f810：rust[st=00 r=FF FF scr=00 x=8C]（弹死不可见）→修后
[st=01 r=02 02 scr=01 x=B0]（活可见）——SPRITE_DATA 烘培截断根因（a1708910 修）。


## 9. 问题②敌面三支归因（2026-10-29（终审定性））

【三支证据】①贴图帧：f800 双爬虫=同位（dx=0 最优）非镜像（镜像配更差）——帧内容
真差（持续 106px）；帧表逐字节全同（CrawlerAnimIndexTable ROM $9AA0=35 35 3E 38
3B 3B 38 3E）+reorient 公式同式②AI 轨迹：x=±1px（BD/5D vs BE/5C（FC 域微差））③
touch 伤害：采点未触（截图无碰撞场景）。
【f800 Ens 槽态直比（A 级）】**data05 一致分歧**：emu=1A/5A vs rust=58/18（XOR=0x42
（位 6（30FPS 门）+位 1）双爬同步）——敌状态演化径分歧（确证（新发现））；rust 独有
442/452=80（radiusX 写）+449/459=01（delay 写）（emu 无）——写集面差。
【轴感应 facing 修（a 件）】enemy_type_times2_plus_facing=固定 bit0（注带「?」）→
asm $F844 **axis 感应**（bit7 置⇒bit3/清⇒bit0）——修+TDD 二测（顶爬（竖轴）错帧根因
通用面）——本 f800 案 bit7 清（修=正确但非此案直接因）。
【缺口注】anim 态（EnsExtra $6AF4+）**不在 .ram.hex 覆域**（0x000-0x07FF）——
帧差终钉需扩 dump 覆域（或 .ppu OAM 反推）——遗留下一探。
【定性】残差=敌状态/anim 演化径分歧（data05 位 6/1+写集面）——非贴图表/非位移
——待 data05 演化径 asm 对读（UpdateEnemy_Active 位 6/1 语义）续修。
