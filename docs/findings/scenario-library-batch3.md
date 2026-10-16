# findings：场景库批次 3——密码输入/菜单序列（task-36，title 深路径对拍素材）

> 状态：**事件面全达**（2026-10-17）——0x18 深路径 face 实达+字符录入实证。
> 复跑：`scenario_run run sc-title-menu2` / `sc-password-entry3` ×2。

## 1. 路径考古（键位定谳，A 级）

- **菜单切换键=SELECT**（非 up/down！）：`L90EB CMP #$20 / EOR #$01`（ChooseStartCont
  $90D7 指令流）——up/down 长按皆无应（像素同像实录）；
- **确认键=START**：`L90DB CMP #$10`——StartContinue=0→InitializeStats（新游戏）/
  =1→**TitleRoutine=$17 LoadPswdScreen**（$90E7 指令流）；
- 分发表定谳（nmikstas Bank00 $8034-$8066）：0x15=StartContScrn/0x16=ChooseStartCont/
  **0x17=LoadPswdScreen/0x18=EnterPassword**/0x1A=WaitForSTART/0x1B=StartContScrn 二例。

## 2. 事件核实证（双包）

| 包 | 事件核 | 双跑 |
|---|---|---|
| sc-title-menu2 | 菜单屏实达（START/CONTINUE+游标截图）+**确认=START→游戏起始**（f620 房间屏实证） | ✓ 4ef3e07f |
| sc-password-entry3 | **$1F→0x17(f501)→0x18(f505) EnterPassword 实达**+**字符录入「22」显示实证**（f740 绿字）+输入序列 right/a/b/select | ✓ 2151171c |

- 0x18 深路径 face（专件验证仓）**活**——对拍素材就位（bundle 随包）；
- pw 写点注：录入字符缓存面≠PasswordByte00（$6988=提交后字节——录入缓冲另址，
  开放项）。

## 3. 迭代史（调径材料）

sc-title-menu(v1)/sc-title-menu2(v2 定稿)/sc-title-menu3(键位判别)/
sc-password-entry(v1-2)/sc-password-entry3(v3 定稿)——screenshots+逐帧 $1F
时间线全在库（up/down 无应实录=键位考古的关键反证）。

## 4. 开放项

1. 录入字符缓冲址（写点级 probe 扩面——密码提交→$6988 编码链）；
2. 退格/清屏语义（b 键在 0x18 面的判定——EnterPassword 指令流待细读）；
3. 0x1A/0x1B 面（WaitForSTART/game-over 显示）=game-over 路径场景（批 4 面）。

## 5. 【批次 4·2026-10-17】title 仓补完三件

| 件 | 状态 | 证据 |
|---|---|---|
| 录入缓冲 | **定址 $0780 起**（A 级） | 写点与首键同刻（f621，$0780/$0781=11/02 双字节写）——录入缓冲≠$6988 提交字节（开放：编码语义 0x11/0x02↔显示「22」映射） |
| 退格语义 | **未观测效果**（如实） | b@680 后缓冲/显示零变（「22」不变）——b=取消/条件触发/或非退格（EnterPassword 指令流细读开放） |
| 0x1A/0x1B game-over 面 | **fixture 注入径实验关闭**（负结果如实） | fixture 注血量 0 实证生效（f850 $0106/$0107 归零）但**死局不触发**（health=0 零响应）——死亡判定挂**伤害应用路径**（SubtractHealth $CE92 完成态检查）——与碰撞族阻断**同根**（一锤三果：拾取/受击/死亡全挂判定族使能） |

**fixture DSL 二次实战**（task-28 ① 刀刃上阵）：血量注入精确生效——夹具工具面
复证；game-over 链（0x19/0x1A/0x1B）待**碰撞族使能锤**解锁（判定族开火=伤害→死亡→
Game Over→DisplayPassword→WaitForSTART 全链活）。
