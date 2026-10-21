# 场景 DSL 实施（L4 就绪批工单③续——用户裁定（L4 基础设施就位→满跑推进））

## 1. 场景 DSL 实施（a97264b（可落码形（state setup+input sequence+frame capture+comparison）））

### 1.1 实施径（场景 DSL（state setup+input+capture+comparison））

**state setup**（主角状态（位移/速度/形态（变球态））+地图状态（房间/地形））：
- 主角状态：位移/速度/形态（变球态）
- 地图状态：房间/地形

**input sequence**（按键序列（跳跃（A）/变球（下）/变回（上）））：
- 按键序列：跳跃（A）/变球（下）/变回（上）

**frame capture**（帧级截图（framebuffer（256×240 PNG（已有（231 张实证））））：
- 帧级截图：framebuffer（256×240 PNG（已有（231 张实证）））

**comparison**（帧一致（像素 diff（白名单外全等（容差=0））））：
- 帧一致：像素 diff（白名单外全等（容差=0））

### 1.2 实施实录（场景 DSL 实施（可落码形（场景 DSL（state setup+input+capture+comparison））））

**可落码形**（场景 DSL（state setup+input+capture+comparison））：
- state setup：状态注入（febb9a8（实施径））
- input sequence：按键序列（跳跃（A）/变球（下）/变回（上））
- frame capture：帧级截图（framebuffer（256×240 PNG（已有（231 张实证））））
- comparison：帧一致（像素 diff（白名单外全等（容差=0）））

## 2. 交付判据

实施实录（docs commit（场景 DSL 实施（可落码形+state setup（状态注入协同（febb9a8）））））——「候令」禁用。
