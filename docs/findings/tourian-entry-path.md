# findings：Tourian 入口径考证窗（T45-5——R-mv 世界路线图补面）

> 状态：**入口径定谳**（2026-10-17，asm-first：Bank00 ItemData 表+Statue 状态族
> 内容锚）。与 T45-4 能力门径图合璧=完整 R-mv 补面。复现：
> `sed -n '2095,2165p' …/Bank00.asm`（ItemData 表）+`grep -n "KraidStatueStat" …`。

## 1. Tourian 入口路径定谳（A 级）

```
世界坐标 (05,02) → [Item 3] 红色导弹门（.word $2882）→ Tourian 电梯
```
- **红门=导弹门**（item type 001010=Missile door——需导弹轰开）；
- 入口前置=①到达坐标 (05,02)（地图可达面）②导弹（能力面）；
- 电梯下 Tourian=区域切换（InArea 族）。

## 2. Statue 状态模型（A 级——porter Statue 窗对表源）

| 变量 | 位 | 语义 |
|---|---|---|
| KraidStatueStat($687B) | bit7=statue up / bit0=defeated | Kraid 面 |
| RidlyStatueStat($687C) | bit7=statue up / bit1=defeated | Ridley 面 |
| PasswordByte0F($6997) | bit7=Kraid up/bit6=Kraid defeated/bit5=Ridley up/bit4=Ridley defeated | **密码持久面**（4 MSB） |

- 双向映射：$687B/$687C→$6997 位打包（L8CBC-L8CF2）+反向装载（L8D4F+）；
- **奖励链**（L8DC6）：双 boss 击败→MaxMissiles +75×2（上限 255）；
- 新游戏清零（L933C）；NARPASSWORD 旁路（L8D12——无敌码跳过全部密码处理）。

## 3. ItemData 世界路线图（A 级，50+ 项全坐标——R-mv 主表）

| 项 | 内容 | 坐标 |
|---|---|---|
| 0 | Maru Mari | (02,0E) |
| 2/3 | 红门→long beam / **红门→Tourian 电梯** | (07,05)/(05,02) |
| 5/6 | 红门→bombs / Bombs | (1A,05)/(19,05) |
| 10/11 | 红门→varia / Varia suit | (0F,02)/(0F,02) |
| 23/24 | 红门→high jump / High jump | (1C,11)/(1B,11) |
| 25/26 | 红门→screw attack / Screw attack | (0E,10)/(0D,1D) |
| 29 | 红门→wave beam | (1C,15) |
| 41/47 | **红门→Kraid** / **紫门→Ridley** | (08,1D)/(10,1D) |
| 其余 | 导弹×19/能量罐×7/普通红门/橙门 | 全表 L9029-L908D+ |

- 类型编码（IIIIII 6 位）：high jump=01…Bombs=0C/Mother brain=0E/Zeebetites=0F-13
  （L8C91 注释表）；
- 字打包：[IIIIII][坐标对]（精确位段留开放项——注释已给全坐标）。

## 4. 能力回环径终图（T45-4×T45-5 合璧）

| 目标 | 路径链 |
|---|---|
| **Tourian** | 到 (05,02) + 导弹轰红门（能力前置=导弹=击杀链/拾取）→电梯 |
| Kraid 域 | 红门 (08,1D)（导弹）→boss→statue up |
| Ridley 域 | 紫门 (10,1D)→boss→statue up |
| 双 boss 后 | +150 MaxMissiles 上限 255（终局供弹） |

## 5. 开放项

1. ItemData 字打包位段精确解（IIIIII+坐标 nibble 布局）；
2. Statue 抬升触发例（SFX_ST_RAISE 消费点在 area 模块——porter Statue 窗合龙点）；
3. 红门轰开机制（导弹碰撞→门瓦变更——UpdateTiles 面）。
