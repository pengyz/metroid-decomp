# spec：Samus 移动（首切片，v0）

> 状态：v0.1（2026-10-14）——G4 pilot；v0.1 增补 R-mv-3 积分公式指令级解码（A 级）。规则全部带证据指针；语义标注 A/B/C 级。
> 证据源：`reference/seed/metroid.asm`（冻结投影）、`analysis/traces/samus-move3.frames.jsonl`
> （A 级动态观测）、m1disasm（**仅线索**，D5 命名不背书）。
> 覆盖声明：本 spec 仅覆盖「水平/垂直移动状态变量与例程入口」层；跳跃弧线/滚球/碰撞
> 规则待后续版本（缺口进矩阵 S/D/B 面，不虚报）。

## 0. 状态变量（自证后采用；实现=game-core::ram_map）

| 常量 | 地址 | 语义 | 级 | 证据 |
|---|---|---|---|---|
| SAMUS_SCR_X | $0051 | 屏幕 X | B | trace 右行 00→8E、左行 8F→70（samus-move3 f1000-f1330） |
| SAMUS_SCR_Y | $0052 | 屏幕 Y | B | trace 下落 B1→B9→C1（f1000-f1150） |
| SAMUS_SUBPIXEL_X | $0311 | 水平亚像素累加 | B | trace 右行稳态 0x40 恒相位；seed bank07:E43E STA $0311（A 级址） |
| SAMUS_SUBPIXEL_Y | $0310 | 垂直亚像素累加 | B | seed bank07:E3DA STA $0310（A 级址） |
| SAMUS_SPEED_SUBPIXEL_X | $0313 | 水平速度线性计数 | B | trace 右行稳态 0x80、左行 0xD0；seed bank07:E400（A 级址） |
| SAMUS_SPEED_SUBPIXEL_Y | $0312 | 垂直速度线性计数 | B | seed bank07:E2FD（A 级址） |
| SAMUS_JUMP_DISPLACEMENT | $030F | 跳跃起点位移（补码） | B | trace FF→F7→EF 后稳定（落地特征） |

## 1. 规则（R-mv-*）

### R-mv-1 水平移动方向与位置响应
按住右键 ⇒ SAMUS_SCR_X 上升；按住左键 ⇒ 下降（屏幕坐标域 $00-$FF）。
- 判定性：可判（逐帧读 $51 单调性 + 方向）
- 证据：samus-move3（右 hold 850-1150 → f1000=8E；左 hold 1180-1330 → f1200=78/f1250=70）
- 级：B（trace 相关 + m1disasm 线索 SamusScrX）
- 边界未验：屏幕边缘/门触发的 X 重置（f950 前 X=0——疑房间装载清零，**C 级假设**）

### R-mv-2 垂直下落
过门/离地后 SAMUS_SCR_Y 递增至落点后稳定（下落=+Y）。
- 判定性：可判
- 证据：samus-move3 f1000-f1150（B1→B9→C1）
- 级：B

### R-mv-3 亚像素/速度计数器（移动积分器）
水平移动由 SAMUS_SUBPIXEL_X/SAMUS_SPEED_SUBPIXEL_X 积分驱动（亚像素→整像素进位）。
右行稳态 SPEED_SUBPIXEL_X=0x80、SUBPIXEL_X=0x40 恒相位（等速特征）；反向后速度态切换（0xD0）。
- 判定性：可判（计数器轨迹 vs 整像素进位关系）
- 证据：trace f1000-f1330；seed STA 写址（A 级）
- 级：**A 级公式（v0.1 指令级解码，seed bank07:E3A5-E445）**：
  - 垂直：`speed_y(16bit: $308 int + $312 frac) += accel_y($314)`（CLC/ADC 进位链）；
    **整数钳位 [-6, +5]**（E3C3 `CMP #6`/LDX #$FA=E3C5；E3C9 `CMP #5`；越限经 sub_D147
    钳位后 STX $308）；`subpix_y($310) += speed frac`，进位加 speed int → 位移输出 $00。
  - 水平：`speed_x(16bit: $309 + $313) += accel_x($315)`（BIT $315 符号扩展链）；
    幅值钳位经 sub_E449（上限源自 maxspeed_src($316) × sub_C2C5/C2BF 变换）；
    `subpix_x($311) += speed frac` 进位入整像素。
  - 证据：metroid.asm 105390-105460（逐指令）；**子例程已定谳**：sub_D147=LDY#0/STY $308/
    STY $312/RTS（钳位=速度重置 ±5.0/-6.0）、sub_E449=16 位取负（$00/$01）、sub_C2C5=ASL×4、
    sub_C2BF=LSR×4（限幅=16×$316）。**残留待验**：$314/$315 的写入来源（加速度表？下一段考证）。
  - 钳位阈值语义（2026-10-15 更正——旧 quirk 注记系符号-幅值误读的**假怪癖**，已废）：
    干净单调 |v|≥6.0 含边界（mag_hi=-int-(frac≠0)≥6）+ 上下限不对称（+5.0/-6.0）；
    实测 -6.004($F9:$FF) 钳位、-5.996($FA:$01) 不钳（逃逸值比 -6.0 更小幅=正确行为）。
    **Stage B 禁止按「修 quirk」处理**（会引入行为变更）；见 l2-proof-assumptions.md A6。

### R-mv-4 跳跃位移计
SAMUS_JUMP_DISPLACEMENT 为自跳跃起点的垂直像素位移（补码），落地后冻结。
- 判定性：可判
- 证据：trace f1000=FF→f1050=F7→f1100=EF 稳定
- 级：B（「冻结」语义 vs 持续演化待跳跃场景验证）

### R-mv-5 入局链（前置）
可操作状态需输入链 `title→start→(A 键)→start`，绝对帧 f≥800 ∧ OAM 玩家条目在场
（F_a 判据，design/state-fixture §3）。
- 证据：findings/samus-movement-observations.md A1/A3/A4（A 级）
- 级：A

## 2. 例程入口（S 面格候选，移植队列）

| 入口 | 说明（线索） | 级 | 证据 |
|---|---|---|---|
| bank07:$CC0D UpdateSamus | 显示/移动总控（被 CB33 调用） | A（址）/C（语义） | seed 101625/101522 |
| bank07:$C27C JumpEngine | 跳跃分发表（C13E/CC1F/CC7A 调用） | A（址）/B（语义） | seed 100448 |
| bank07:$CFC3 SetSamusJump | 起跳状态置位（线索） | A（址）/C（语义） | seed 101678 |
| bank07:$D0B5 SetSamusRoll | 滚球态置位（线索） | A（址）/C（语义） | 待 seed 复核 |
| bank07:$E2FD-$E43E | 亚像素/速度积分写址区间 | A（址） | seed 105317-105458 |

### 2.1 batch-1 靶点链（E37A 头部，2026-10-14 定尺寸）

S-FUNC-07-E37A 头部（E37A-E3A4，accel/maxspeed 初始化）移植前置依赖：
- `sub_E7AD`（E7AD-…：$301,X 取负 + sub_E8BE）及其子链；
- `sub_D976`（D976-D9B9：$307/$7D 清零 + 敌槽扫描循环，调 F1xx 族 F186/F152/F1BF/F1FA/F1F4
  + sub_D9BA + sub_DC82）——**深调用图，非 L2 可切片**，需独立 decomp-analyst 考证单元；
- 语义线索（C 级，仅定位）：constants_ram 注释「EnemyReactToSamusProjectile/ExplodeEnemy…」。
结论：头部移植=batch-1 的 F1xx 族考证先行（S 面格随矩阵聚类），完成后 E37A 整格计 covered。

### 2.2 F162 变体状态更正（2026-10-15）
「无调用点悬置」主张**已被证伪**：bank07:$F109 `JSR $F162` 实存（ROM 字节级），
$F0A2-$F148 数据岛实为代码流（findings/seed-decode-gap-f0a2-f148.md）。F162/F1E7/F1EE
族待 decode-sweep 定界后作为 batch-1 单元移植。

### 2.3 E37A 头部考证半交付（2026-10-16，task-19）

考证记录：[e37a-header-forensics.md](../findings/e37a-header-forensics.md)（三址
ROM↔nmikstas 逐字节互核 A 级）。**R-mv-6 下落发起规则**（新增，B 级=指令流+注释锚）：

- **前置**：SamusGravity($0314)==0（非升非降）；
- **相位门**：`(ObjectY($030D)+ObjRadY) and #$07 == 0`（8 像素网格合拍才检碰撞）；
- **阻挡门**：`sub_E7AD`（E7AD）CF=1 畅通（CF=0=撞实心瓦/门——瓦片分类：
  <$80 实心、$80-$9F 可炸、≥$A0 可行、$4E 特例；Samus 面 $A0/$A1 写
  SamusDoorData($58) 门标志）；
- **支撑面门**：`sub_D976`（D976）双旗判定——SamusOnElevator($0307)/OnFrozenEnemy($7D)
  （敌槽扫描 EnStatus==$04 冻结面 + 电梯面 ElevatorStatus($0320)，命中旗经
  sub_D9BA 写 SamusHit($030A)）；
- **效果**：两旗皆 0 ⇒ `SamusGravity=$1A`（下落重力）→ X=#$05 极速 →
  VertCntrLinear 溢出积分→ObjVertSpeed（重力积分链，R-mv-3 衔接）。

移植半注：sub_E7AD/sub_D976 均为深调用图（F1xx 族+几何子链）——**非 L2 可切片**，
batch-1 需 F1xx 族考证先行（遗留清单见 findings §7）。命名候选全 C 级待台账自证。

### R-mv-7 高跳重力规则（新增，A 级=字节自证）

**语义**：跳跃启动时重力常数按 gear 位分流——`gr_HIGHJUMP（$02）置位⇒SamusAccelY=$12
（低重力=高跳）/否则 $18（普通）`。生效点=jump 启动面（SFX_SamusJump 后）。

**证据**（asm 内容锚）：`Lx017: ; 07:CFF0`（prg7_engine：
`jsr SFX_SamusJump/ldy #$18/lda SamusGear/and #gr_HIGHJUMP/beq Lx018/ldy #$12/
Lx018: sty SamusAccelY`）——字节自证可复现。

**移植注**：samus_jump 启动面折（handler.rs——241cbd7）；decomp 跳动学线疑②定案
（物理常数 $18/$12）。

## 3. 缺口显式声明（D7）

- 未覆盖：跳跃弧线数值（初速/重力表）、滚球物理、碰撞/推墙、门/房间切换 X 重置语义、
  动画帧调度——进矩阵 B/D 面。
- C 级假设区：$51 在房间装载时清零（f950 观测）；$030F「落地冻结」语义；
  UpdateSamus/C27C 的语义名（m1disasm 线索未自证）。

## 4. 验证计划

1. L2：积分器函数（E2FD-E43E 区间可切片子程序）——case 自动生成（ROM 腿）+ golden；
2. L3：samus-move3 场景双跑（oracle vs game-core dump，S5 后）；
3. 判定依据：subroutine-diff / trace-diff 门禁 + matrix 格覆盖登记。
