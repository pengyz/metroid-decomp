# 典型场景功能测试方案（Scene Functional Test Plan）——L4 调研工单④

> 【2026-10-21 设计件——L4 调研工单④（用户裁定（第一步不是跑 TAS（定制典型场景
> （功能测试）先行）））】调研判据=方案文档实录（一行收口（长报告禁））。

## 核心原则

**第一步不是跑 TAS**（几小时流程（L4 截图爆炸））——**定制典型场景（功能测试
（跳跃/变球（功能验证）））先行**。

## 场景 DSL 设计（Scene DSL）

### 1. state setup（主角状态+地图状态设定）

```yaml
state_setup:
  protagonist:
    displacement: {x: 0, y: 0}        # 位移（房间内坐标）
    speed: {vx: 0, vy: 0}            # 速度（水平/垂直）
    form: standing | morph_ball      # 形态（变球态）
    health: 99                       # 生命值
  map:
    room: room_id                    # 房间（地图状态）
    terrain: terrain_type            # 地形（平台/地面/水）
    camera: {x: 0, y: 0}             # 镜头位置
```

### 2. input sequence（按键序列（跳跃（A）/变球（下）））

```yaml
input_sequence:
  - frame: 0                         # 帧号
    keys: []                         # 按键序列（A=跳跃/下=变球/方向键）
  - frame: 30
    keys: [A]                        # 跳跃（A）
  - frame: 60
    keys: [Down]                     # 变球（下）
  - frame: 90
    keys: [Up]                       # 变回（上）
```

### 3. frame capture（帧级截图（framebuffer））

```yaml
frame_capture:
  frames: [0, 30, 60, 90, 120]       # 帧级截图（framebuffer）
  format: png                        # 截图格式
  whitelist: [font, palette]         # 白名单（字体/调色板映射）
```

### 4. comparison（帧一致（像素 diff（白名单外全等）））

```yaml
comparison:
  method: pixel_diff                 # 像素 diff（白名单外全等）
  tolerance: 0                       # 容差（白名单外全等=0）
  whitelist: [font, palette]         # 白名单（字体/调色板映射）
  result: frame_consistency          # 帧一致（像素 diff（白名单外全等））
```

## 典型场景（Typical Scenes）

| 场景 | 功能 | state setup | input sequence | 验证点 |
|---|---|---|---|---|
| 跳跃 | 跳跃（A） | standing/地面 | A（帧 30） | 垂直位移/速度/落地 |
| 变球 | 变球（下） | standing/地面 | Down（帧 60） | 形态切换/高度变化 |
| 变回 | 变回（上） | morph_ball/地面 | Up（帧 90） | 形态切换/高度恢复 |
| 跳跃+变球 | 组合 | standing/地面 | A（帧 30）+Down（帧 60） | 组合功能（跳跃中变球） |

## 交付判据

**方案文档（docs commit（场景 DSL 设计（可落码形（场景 DSL（state setup+input+capture+
comparison）））））**——一行收口（长报告禁）。

## 执行纪律

- 「候令」禁用（L4 调研工单④）。
- 第一步不是跑 TAS（几小时流程（L4 截图爆炸））——定制典型场景（功能测试（跳跃/变球
  （功能验证）））先行。
- 像素 diff（白名单外全等）——白名单（字体/调色板映射）=唯一允许例外（AGENTS.md §5）。
