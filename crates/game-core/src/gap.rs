//! gap —— 实现缺口 fail-loud 登记（D7 机制②；rounds/r2-fix）。
//!
//! 未实现例程一律 `unported!(bank:addr)` 桩：**禁止空实现/静默默认值**。
//! 桩 panic 携带 bank:addr（缺口确定性爆在精确地址），同时落本登记表（供矩阵格核对）。

use std::sync::Mutex;

static GAP_LOG: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// 登记一个缺口（bank:addr 形式）。
pub fn record(loc: &str) {
    GAP_LOG.lock().unwrap().push(loc.to_string());
}

/// 已登记缺口（测试/诊断读取）。
pub fn recorded() -> Vec<String> {
    GAP_LOG.lock().unwrap().clone()
}

/// 清空登记（仅测试隔离用）。
pub fn reset() {
    GAP_LOG.lock().unwrap().clear();
}

/// fail-loud 桩：爆在精确地址，形态上与移植错可分（pipeline §4.2 路由表）。
#[macro_export]
macro_rules! unported {
    ($loc:expr) => {{
        $crate::gap::record($loc);
        panic!("unported @ {}", $loc);
    }};
}
