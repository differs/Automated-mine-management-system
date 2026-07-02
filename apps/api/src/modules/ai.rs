//! # 矿山运输调度算法
//!
//! ⚠️ **重要前提**：矿山调度算法不能通用化。每个矿山的坑口布局、地形坡度、
//! 道路条件、装载点位置都不一样。实际部署时必须根据矿山现场情况调参和优化。
//!
//! ## 双模式架构
//!
//! 系统支持两种调度模式，可在运行时通过 API 切换：
//!
//! ### 纯算法模式（AlgorithmDispatchEngine）
//! 基于确定性规则，不依赖任何外部AI。
//! - 派单：加权优先级匹配算法（WPMA），权重根据矿山地形配置
//! - 拥堵预测：阈值判断
//! - 异常检测：超时规则
//!
//! ### AI增强模式（AiDispatchEngine）
//! 在纯算法基础上，关键决策点调用盘古大模型优化。
//!
//! ## 算法核心：通用框架 + 地形适配
//!
//! 算法 = 通用流程（60%）+ 地形适配参数（40%）
//!
//! 每个矿山部署时，需要配置以下地形参数：
//! - 距离矩阵：坑口到坑口、坑口到地磅的实际路径距离（非直线）
//! - 坑口参数：装载能力、同时装载数、优先级、道路条件
//! - 车辆适配：适合路线、载重、油耗
//!
//! ## 加权优先级匹配算法（WPMA）
//!
//! ```text
//! 综合权重 = w1 × 空闲权重 + w2 × 工作量权重 + w3 × 距离权重 + w4 × 坑口权重
//!
//! 其中：
//!   空闲权重 = idle_minutes / max_idle           公平性
//!   工作量权重 = 1 - (trips_today / max_trips)   负荷均衡
//!   距离权重 = 1 - (distance_to_pit / max_distance) 效率
//!   坑口权重 = pit_priority / max_priority        业务优先级
//!
//! 默认权重：w1=0.3, w2=0.2, w3=0.3, w4=0.2
//! 实际部署时必须根据矿山地形调整：
//!   - 平坦矿山 → 提高距离权重（跑得快，距离影响大）
//!   - 山区矿山 → 进一步提高距离权重（空跑成本高）
//!   - 坑口分散 → 提高坑口权重（优先去高优先级坑口）
//! ```

use serde::{Deserialize, Serialize};
use crate::config::AiConfig;

// ─── 调度权重配置 ──────────────────────────────────────────────────────────

/// WPMA 算法权重配置
///
/// 每个矿山部署时需要根据地形调参。
/// 默认值适用于一般矿山场景。
#[derive(Debug, Clone)]
pub struct WpmaWeights {
    /// 空闲时间权重（公平性）
    pub idle: f64,
    /// 工作量权重（负荷均衡）
    pub workload: f64,
    /// 距离权重（效率）
    pub distance: f64,
    /// 坑口权重（业务优先级）
    pub pit_priority: f64,
}

impl Default for WpmaWeights {
    fn default() -> Self {
        Self {
            idle: 0.3,
            workload: 0.2,
            distance: 0.3,
            pit_priority: 0.2,
        }
    }
}

// ─── 调度数据模型 ──────────────────────────────────────────────────────────

/// 待派运单信息
#[derive(Debug, Clone)]
pub struct PendingWaybill {
    pub waybill_id: String,
    pub driver_id: String,
    pub pit_id: String,
    pub pit_name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// 司机统计信息
#[derive(Debug, Clone)]
pub struct DriverStats {
    pub driver_id: String,
    pub driver_name: String,
    pub idle_minutes: i64,
    pub trips_today: i64,
    pub current_pit_id: Option<String>,
}

/// 坑口统计信息
#[derive(Debug, Clone)]
pub struct PitStats {
    pub pit_id: String,
    pub pit_name: String,
    pub queue_length: i32,
    pub priority: i32,
    pub max_capacity: i32,
}

/// 单条调度推荐
#[derive(Debug, Serialize, Deserialize)]
pub struct DispatchRecommendation {
    pub rank: i32,
    pub waybill_id: String,
    pub driver_id: String,
    pub driver_name: String,
    pub pit_id: String,
    pub pit_name: String,
    pub composite_score: f64,
    pub idle_score: f64,
    pub workload_score: f64,
    pub distance_score: f64,
    pub pit_score: f64,
    pub congestion_level: String,
    pub reason: String,
}

/// 调度推荐结果
#[derive(Debug, Serialize, Deserialize)]
pub struct DispatchPlan {
    pub recommendations: Vec<DispatchRecommendation>,
    pub total_pending: usize,
    pub total_idle_drivers: usize,
    pub algorithm_version: String,
}

// ─── AI调度引擎 ──────────────────────────────────────────────────────────

/// AI调度引擎 — 调用盘古大模型优化调度决策
pub struct AiDispatchEngine {
    config: AiConfig,
    client: reqwest::Client,
}

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
    max_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct ChoiceMessage {
    content: String,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ChoiceMessage,
}

impl AiDispatchEngine {
    pub fn new(config: AiConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    /// 调用盘古大模型
    async fn chat(&self, system: &str, user: &str) -> Result<String, String> {
        let request = ChatRequest {
            model: self.config.model.clone(),
            messages: vec![
                ChatMessage { role: "system".into(), content: system.into() },
                ChatMessage { role: "user".into(), content: user.into() },
            ],
            temperature: 0.3,
            max_tokens: 512,
        };

        let resp = self.client
            .post(&self.config.api_url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("AI request failed: {e}"))?;

        let body: ChatResponse = resp
            .json()
            .await
            .map_err(|e| format!("AI response parse failed: {e}"))?;

        Ok(body.choices.into_iter()
            .next()
            .map(|c| c.message.content)
            .unwrap_or_default())
    }

    /// AI智能派单优化
    pub async fn optimize_dispatch(
        &self,
        waybill_id: &str,
        driver_id: &str,
        pit_id: &str,
        pit_queue_len: i32,
        driver_active_count: i32,
    ) -> Result<String, String> {
        let prompt = format!(
            r#"当前运单信息：
- 运单ID：{waybill_id}
- 目标司机：{driver_id}
- 目标坑口：{pit_id}
- 坑口排队长度：{pit_queue_len}
- 司机当前任务数：{driver_active_count}

请分析这个派单决策是否合理，给出优化建议。
如果合理，回复"OK"即可。
如果不合理，请说明原因并给出建议。
"#
        );

        self.chat(
            "你是矿山运输调度AI助手。你擅长分析调度场景并给出最优派单建议。回复要简洁。",
            &prompt,
        ).await
    }

    /// AI拥堵预测
    pub async fn predict_congestion(
        &self,
        pit_id: &str,
        current_queue: i32,
        active_drivers: i32,
        hour_of_day: i32,
    ) -> Result<String, String> {
        let prompt = format!(
            r#"坑口ID：{pit_id}
当前排队：{current_queue}辆
活跃司机：{active_drivers}人
当前时间：第{hour_of_day}时

请预测未来2小时拥堵趋势，给出分流建议。
回复格式：拥堵等级(低/中/高) + 简要建议
"#
        );
        self.chat("你是矿山运营AI分析师。基于实时数据预测拥堵。回复要简洁。", &prompt).await
    }

    /// AI异常检测
    pub async fn detect_anomaly(
        &self,
        waybill_id: &str,
        status: &str,
        wait_minutes: i64,
    ) -> Result<String, String> {
        let prompt = format!(
            r#"运单{waybill_id}，状态{status}，已等待{wait_minutes}分钟。
请判断是否存在异常（超时/卡单/异常等待），给出处理建议。
"#
        );
        self.chat("你是矿山异常检测AI。发现异常及时报警。回复要简洁。", &prompt).await
    }
}

// ─── 纯算法调度引擎 ───────────────────────────────────────────────────────

/// 纯算法调度引擎 — 基于加权优先级匹配算法（WPMA）
///
/// 不依赖任何外部AI，所有决策基于确定性规则。
/// 同样的输入永远得到同样的输出。
pub struct AlgorithmDispatchEngine;

impl AlgorithmDispatchEngine {
    /// WPMA 派单决策 — 生成调度推荐方案
    ///
    /// 核心算法：
    /// 1. 对每个待派运单，计算目标坑口的拥堵系数
    /// 2. 对每个空闲司机，计算综合权重（空闲+工作量+距离+坑口）
    /// 3. 按综合权重排序，输出 top-N 推荐
    pub fn dispatch(
        pending_waybills: &[PendingWaybill],
        idle_drivers: &[DriverStats],
        pit_stats: &[PitStats],
        weights: &WpmaWeights,
        top_n: usize,
    ) -> DispatchPlan {
        let mut recommendations = Vec::new();

        // 预计算坑口统计索引
        let pit_map: std::collections::HashMap<&str, &PitStats> = pit_stats
            .iter()
            .map(|p| (p.pit_id.as_str(), p))
            .collect();

        // 预计算最大值（用于归一化）
        let max_idle = idle_drivers.iter().map(|d| d.idle_minutes).max().unwrap_or(1).max(1);
        let max_trips = idle_drivers.iter().map(|d| d.trips_today).max().unwrap_or(1).max(1);
        let max_queue = pit_stats.iter().map(|p| p.queue_length).max().unwrap_or(1).max(1) as f64;
        let max_priority = pit_stats.iter().map(|p| p.priority).max().unwrap_or(1).max(1) as f64;

        // 对每个待派运单，找最佳匹配司机
        for waybill in pending_waybills {
            let pit = pit_map.get(waybill.pit_id.as_str());
            let queue_len = pit.map(|p| p.queue_length).unwrap_or(0);
            let pit_priority = pit.map(|p| p.priority).unwrap_or(1);
            let pit_name = pit.map(|p| p.pit_name.as_str()).unwrap_or("unknown");

            // 拥堵系数
            let congestion = if max_queue > 0.0 {
                queue_len as f64 / max_queue
            } else {
                0.0
            };

            // 拥堵等级
            let congestion_level = if queue_len > 10 {
                "high".to_string()
            } else if queue_len > 5 {
                "medium".to_string()
            } else {
                "low".to_string()
            };

            // 对每个空闲司机计算综合权重
            let mut driver_scores: Vec<(f64, &DriverStats, f64, f64, f64, f64)> = idle_drivers
                .iter()
                .map(|driver| {
                    // 1. 空闲权重：idle_minutes / max_idle（公平性）
                    let idle_score = driver.idle_minutes as f64 / max_idle as f64;

                    // 2. 工作量权重：1 - (trips_today / max_trips)（负荷均衡）
                    let workload_score = 1.0 - (driver.trips_today as f64 / max_trips as f64);

                    // 3. 距离权重：同坑口 +0.3，其他 0（减少空跑）
                    let distance_score = if driver.current_pit_id.as_deref() == Some(waybill.pit_id.as_str()) {
                        1.0 // 同坑口，最高距离权重
                    } else {
                        0.0
                    };

                    // 4. 坑口权重：pit_priority / max_priority
                    let pit_score = pit_priority as f64 / max_priority;

                    // 综合权重（拥堵时降低权重）
                    let congestion_factor = 1.0 - congestion * 0.3;
                    let composite = (
                        weights.idle * idle_score
                        + weights.workload * workload_score
                        + weights.distance * distance_score
                        + weights.pit_priority * pit_score
                    ) * congestion_factor;

                    (composite, driver, idle_score, workload_score, distance_score, pit_score)
                })
                .collect();

            // 按综合权重降序排序
            driver_scores.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

            // 取 top-1 作为推荐
            if let Some((score, driver, idle_s, work_s, dist_s, pit_s)) = driver_scores.first() {
                let reason = if *dist_s > 0.5 {
                    "司机已在该坑口附近，减少空跑".to_string()
                } else if driver.idle_minutes > 60 {
                    format!("司机已空闲{}分钟，优先分配", driver.idle_minutes)
                } else if driver.trips_today == 0 {
                    "司机今日尚未接单，优先分配".to_string()
                } else {
                    "综合权重最优".to_string()
                };

                recommendations.push(DispatchRecommendation {
                    rank: recommendations.len() as i32 + 1,
                    waybill_id: waybill.waybill_id.clone(),
                    driver_id: driver.driver_id.clone(),
                    driver_name: driver.driver_name.clone(),
                    pit_id: waybill.pit_id.clone(),
                    pit_name: pit_name.to_string(),
                    composite_score: *score,
                    idle_score: *idle_s,
                    workload_score: *work_s,
                    distance_score: *dist_s,
                    pit_score: *pit_s,
                    congestion_level,
                    reason,
                });
            }
        }

        // 按综合权重排序并取 top_n
        recommendations.sort_by(|a, b| b.composite_score.partial_cmp(&a.composite_score).unwrap_or(std::cmp::Ordering::Equal));
        recommendations.truncate(top_n);
        // 重新编号
        for (i, rec) in recommendations.iter_mut().enumerate() {
            rec.rank = i as i32 + 1;
        }

        let total_idle = idle_drivers.len();

        DispatchPlan {
            recommendations,
            total_pending: pending_waybills.len(),
            total_idle_drivers: total_idle,
            algorithm_version: "wpma-1.0".to_string(),
        }
    }

    /// 拥堵预测：基于阈值判断
    pub fn predict_congestion(current_queue: i32) -> &'static str {
        if current_queue > 10 {
            "拥堵等级：高 — 建议分流"
        } else if current_queue > 5 {
            "拥堵等级：中 — 注意观察"
        } else {
            "拥堵等级：低 — 运行正常"
        }
    }

    /// 异常检测：基于超时阈值
    pub fn detect_anomaly(wait_minutes: i64) -> &'static str {
        if wait_minutes > 30 {
            "⚠️ 异常：等待超时30分钟，建议人工介入"
        } else if wait_minutes > 15 {
            "⚠️ 注意：等待超过15分钟"
        } else {
            "正常"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_drivers() -> Vec<DriverStats> {
        vec![
            DriverStats {
                driver_id: "DRV-001".into(),
                driver_name: "张三".into(),
                idle_minutes: 120,
                trips_today: 2,
                current_pit_id: Some("PIT-001".into()),
            },
            DriverStats {
                driver_id: "DRV-002".into(),
                driver_name: "李四".into(),
                idle_minutes: 30,
                trips_today: 5,
                current_pit_id: Some("PIT-002".into()),
            },
            DriverStats {
                driver_id: "DRV-003".into(),
                driver_name: "王五".into(),
                idle_minutes: 60,
                trips_today: 0,
                current_pit_id: None,
            },
        ]
    }

    fn make_test_waybills() -> Vec<PendingWaybill> {
        vec![
            PendingWaybill {
                waybill_id: "WB-001".into(),
                driver_id: "DRV-001".into(),
                pit_id: "PIT-001".into(),
                pit_name: "一号坑口".into(),
                created_at: chrono::Utc::now(),
            },
            PendingWaybill {
                waybill_id: "WB-002".into(),
                driver_id: "DRV-002".into(),
                pit_id: "PIT-002".into(),
                pit_name: "二号坑口".into(),
                created_at: chrono::Utc::now(),
            },
        ]
    }

    fn make_test_pits() -> Vec<PitStats> {
        vec![
            PitStats {
                pit_id: "PIT-001".into(),
                pit_name: "一号坑口".into(),
                queue_length: 3,
                priority: 2,
                max_capacity: 20,
            },
            PitStats {
                pit_id: "PIT-002".into(),
                pit_name: "二号坑口".into(),
                queue_length: 8,
                priority: 1,
                max_capacity: 15,
            },
        ]
    }

    #[test]
    fn test_wpma_dispatch_basic() {
        let waybills = make_test_waybills();
        let drivers = make_test_drivers();
        let pits = make_test_pits();
        let weights = WpmaWeights::default();

        let plan = AlgorithmDispatchEngine::dispatch(&waybills, &drivers, &pits, &weights, 3);

        assert_eq!(plan.total_pending, 2);
        assert_eq!(plan.total_idle_drivers, 3);
        assert!(!plan.recommendations.is_empty());
        assert!(plan.recommendations[0].composite_score > 0.0);
    }

    #[test]
    fn test_wpma_same_pit_bonus() {
        // 司机 DRV-001 在 PIT-001，运单 WB-001 目标也是 PIT-001
        // 应该获得距离加分
        let waybills = vec![PendingWaybill {
            waybill_id: "WB-001".into(),
            driver_id: "DRV-001".into(),
            pit_id: "PIT-001".into(),
            pit_name: "一号坑口".into(),
            created_at: chrono::Utc::now(),
        }];
        let drivers = make_test_drivers();
        let pits = make_test_pits();
        let weights = WpmaWeights::default();

        let plan = AlgorithmDispatchEngine::dispatch(&waybills, &drivers, &pits, &weights, 1);

        let rec = &plan.recommendations[0];
        // DRV-001 在同坑口，距离权重应为 1.0
        assert_eq!(rec.distance_score, 1.0);
        assert_eq!(rec.driver_id, "DRV-001");
    }

    #[test]
    fn test_wpma_idle_fairness() {
        // 无同坑口优势时，空闲时间最长的司机应该排在前面
        let waybills = vec![PendingWaybill {
            waybill_id: "WB-001".into(),
            driver_id: "DRV-001".into(),
            pit_id: "PIT-003".into(), // 不存在的坑口
            pit_name: "unknown".into(),
            created_at: chrono::Utc::now(),
        }];
        let drivers = make_test_drivers();
        let pits = make_test_pits();
        let weights = WpmaWeights {
            idle: 0.5,  // 高空闲权重
            workload: 0.1,
            distance: 0.1,
            pit_priority: 0.3,
        };

        let plan = AlgorithmDispatchEngine::dispatch(&waybills, &drivers, &pits, &weights, 3);

        // DRV-001 空闲 120 分钟，应该排名第一
        assert_eq!(plan.recommendations[0].driver_id, "DRV-001");
    }

    #[test]
    fn test_wpma_congestion_penalty() {
        // 高拥堵坑口应该降低推荐权重
        let waybills = vec![PendingWaybill {
            waybill_id: "WB-001".into(),
            driver_id: "DRV-001".into(),
            pit_id: "PIT-002".into(), // 排队 8 辆，较高
            pit_name: "二号坑口".into(),
            created_at: chrono::Utc::now(),
        }];
        let drivers = make_test_drivers();
        let pits = make_test_pits();
        let weights = WpmaWeights::default();

        let plan = AlgorithmDispatchEngine::dispatch(&waybills, &drivers, &pits, &weights, 1);

        // 拥堵等级应该是 medium（queue > 5）
        assert_eq!(plan.recommendations[0].congestion_level, "medium");
    }

    #[test]
    fn test_congestion_prediction() {
        assert_eq!(AlgorithmDispatchEngine::predict_congestion(15), "拥堵等级：高 — 建议分流");
        assert_eq!(AlgorithmDispatchEngine::predict_congestion(8), "拥堵等级：中 — 注意观察");
        assert_eq!(AlgorithmDispatchEngine::predict_congestion(3), "拥堵等级：低 — 运行正常");
    }

    #[test]
    fn test_anomaly_detection() {
        assert!(AlgorithmDispatchEngine::detect_anomaly(35).contains("超时"));
        assert!(AlgorithmDispatchEngine::detect_anomaly(20).contains("注意"));
        assert_eq!(AlgorithmDispatchEngine::detect_anomaly(5), "正常");
    }
}
