# 模块依赖图（可独立编译（推进模块依赖图（确保可编译性）））

> 【2026-10-22 用户裁定】认可，每完成一步，必须派独立 agent 对抗复核确保有效。

## 核心主张

协调者依据依赖图，明确哪些可独立编译，哪些不能，根据模块依赖图推进，尽可能确保可编译性。

## 一、模块依赖图（底层（被依赖）→上层（依赖底层））

```
底层（无依赖（可独立编译））：
  engine::nmi（NMI）（316 行（33 fn））——无依赖
  engine::boot（启动）（664 行（56 fn））——无依赖

中层（依赖底层（可独立编译））：
  engine::frame（帧调度）（145 行（16 fn））——依赖 engine::nmi（顺序依赖）

上层（依赖中层（不可独立编译））：
  engine::mod（陈旧代码）（13564 行（1305 fn））——依赖 engine::{frame,nmi,boot}

其他模块（依赖底层（可独立编译））：
  world/{door,elevator,room}（266 行（11 fn））——依赖 engine::nmi
  samus/{handler,physics}（1074 行（54 fn））——依赖 engine::nmi
  enemy/*（15 模块）（1666 行（85 fn））——依赖 engine::nmi
  pickup/{items,powerup}（146 行（7 fn））——依赖 engine::nmi
  ui/password（97 行（6 fn））——依赖 engine::nmi
  sound/{mod,music,sfx}（1430 行（78 fn））——依赖 engine::nmi
  title/{mod,sparkle}（1822 行（42 fn））——依赖 engine::nmi
  ending/sequence（30 行（2 fn））——依赖 engine::nmi
  render/{mod,sprite}（201 行（10 fn））——依赖 engine::nmi
  input/joy（30 行（2 fn））——依赖 engine::nmi
  weapon/{missile,mod}（137 行（6 fn））——依赖 engine::nmi
```

## 二、推进策略（先底层（后上层）（确保可编译性））

**推进顺序**（按依赖图（先底层（后上层）））：
1. **第 1 层**（底层（无依赖（可独立编译）））：engine::nmi+engine::boot
2. **第 2 层**（中层（依赖底层（可独立编译）））：engine::frame（依赖 engine::nmi）
3. **第 3 层**（上层（依赖中层（不可独立编译）））：engine::mod（依赖 engine::{frame,nmi,boot}）
4. **第 4 层**（其他模块（依赖底层（可独立编译）））：world/samus/enemy/pickup/ui/sound/title/ending/render/input/weapon

## 三、可编译性保证

**可编译性检查**（每步完成后）：
- cargo check（0 错）→**可独立编译**
- cargo check（有错）→**不可独立编译**（需先做底层）

## 四、派单单位（模块或文件（自己拿捏合适的度））

**派单单位**：
- 模块级（engine::frame（engine::nmi（engine::boot）））
- 文件级（world/door.rs（samus/physics.rs））
- 块级（enemy/*（15 模块））

## 五、对抗复核（每步完成后派独立 agent）

**对抗复核**（每步完成后）：
1. **实现正确性**（对拍 rust 和 asm）——穷举等价证明（全域穷举）
2. **完备性**（是否实现了规划中的所有函数）——对照映射表（检查是否实现了所有规划函数）
3. **测试覆盖完备性**——测试覆盖所有函数（边界条件（异常路径））
4. **所有测试 check**——cargo test+int-y-proof+frame-nmi-proof
