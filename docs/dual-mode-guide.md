# 双模式调度系统 — 管理员操作指南

---

## 一、环境变量配置

```bash
# .env 文件
# ─── 调度模式 ─────────────────────────────────────
# pure_algorithm = 纯算法模式（默认，零AI依赖）
# ai_enhanced    = AI增强模式（需要盘古大模型）
DISPATCH_MODE=pure_algorithm

# ─── AI模式配置（仅 ai_enhanced 时需要） ──────────
AI_ENABLED=false
AI_API_URL=http://localhost:8000/v1/chat/completions
AI_API_KEY=your-api-key-here
AI_MODEL=openPangu-2.0-Flash
```

## 二、运行时切换（无需重启）

```bash
# 查看当前模式
curl http://localhost:3000/api/v1/system/dispatch-mode

# 切换到AI模式
curl -X POST http://localhost:3000/api/v1/system/dispatch-mode \
  -H "Content-Type: application/json" \
  -d '{"mode": "ai_enhanced"}'

# 切回纯算法模式
curl -X POST http://localhost:3000/api/v1/system/dispatch-mode \
  -H "Content-Type: application/json" \
  -d '{"mode": "pure_algorithm"}'
```

## 三、两种模式对比

| 维度 | 纯算法模式 | AI增强模式 |
|:----|:---------|:----------|
| **派单策略** | FIFO + 固定优先级 | 盘古大模型动态优化 |
| **拥堵预测** | 基于阈值（>5=中，>10=高） | AI分析历史+实时趋势 |
| **异常检测** | 基于超时阈值（15/30分钟） | AI识别复杂异常模式 |
| **外部依赖** | ❌ 无 | ✅ 需要盘古API或本地部署 |
| **延迟** | 毫秒级 | ~200ms-2s（视部署方式） |
| **成本** | ¥0 | API调用费或硬件成本 |

## 四、推荐策略

```
部署初期：纯算法模式（¥0，先跑通）
验证期：  API切换AI模式测试效果（¥200/月）
稳定后：  每个矿区配ARM盒子（¥1,500/台）
升级时：  集团部署Flash服务器（¥11万）
```

## 五、API参考

```json
// GET /api/v1/system/dispatch-mode
{
  "mode": "pure_algorithm",
  "label": "纯算法模式 — 基于FIFO+规则的确定性调度"
}

// GET /api/v1/system/system-config
{
  "dispatch_mode": "pure_algorithm",
  "ai_enabled": false,
  "ai_model": "openPangu-2.0-Flash",
  "description": "当前模式：pure_algorithm，AI：未启用，模型：openPangu-2.0-Flash"
}
```
