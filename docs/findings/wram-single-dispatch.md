# WRAM 面单一分派：typed 化的隐藏前提

> 事故：`7aa04e16`（敌槽 typed 存储）把 `case-elev-norfair-transition`
> 4 个采集点从 match 打成 **196px diff**；修复 `b661a71d` 后该案 10/10 全 0px。

## 1. 现象

| 采集点 | 修前 | 修后 |
|---|---|---|
| f1300 / f1420 / f1560 / f1600 | diff 196px | match 0px |
| 全矩阵 | match 122 · diff 10 | 待复跑 |

## 2. 根因：同一格存储有两条分派径

```rust
// 修前：read_ram/write_ram 在 $6000-$7FFF 上直通字节数组
if (0x6000..0x8000).contains(&addr) {
    self.wram_mut()[addr as usize - 0x6000] = v;
    return;
}

// 而 read_wram/write_wram 先判敌槽 typed 域
if let Some((slot, off)) = split_ens_extra(addr) {
    if self.enemies[slot].set_byte(off, v) { return; }
}
```

于是 `write_ram(0x6AF4+x, v)` 写进字节数组、`read_wram(0x6AF4+x)` 读 typed 旧值。
混用两种写法的路径（房间装载 / 清槽）立刻分叉——电梯换区正是那条路。

**为什么以前没事**：两条径都写字节，答案碰巧一致。typed 化把"碰巧"取消了。

## 3. 修法（分派非点修）

WRAM 两径委托同一入口：

```rust
if (0x6000..0x8000).contains(&addr) { return self.read_wram(addr); }
if (0x6000..0x8000).contains(&addr) { self.write_wram(addr, v); return; }
```

typed 判定只存在一处（`read_wram`/`write_wram`）。

## 4. 同族事故（同一类，第三次）

| 日期 | 位置 | 症状 | 教训 |
|---|---|---|---|
| 2026-11-17 | RAM 侧镜像别名 | `read_ram(0x0B00)` 绕过 typed（`0x0B00 & 0x7FF == 0x0300`） | 归一先于 typed |
| 2026-11-17 | 整面 dump | ram.hex 走裸存储，typed 字段不出现 | 导出面必须走 `snapshot_nes()` |
| 2026-11-17 | **WRAM 分派** | `write_ram` 与 `write_wram` 对同一格给出两个答案 | 分派必须单点 |

## 5. 纪律（写入 typed 路线）

1. **迁一个域前，先查这个域的所有写径**：`write_ram`/`write_wram`/`ram_mut()`/`wram_mut()`/整面拷贝，任何一条漏掉都等于留下一个静默分叉。
2. **同一块存储只允许一个入口**：两径并存时，二者必须互相委托，不许各自实现。
3. **矩阵是唯一裁判**：这批改动自认为是纯结构迁移（读写径都走 `write_wram`），实测证明不是。结构性改动同样要跑全矩阵。
