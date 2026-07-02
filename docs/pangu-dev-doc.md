# 盘古大模型融合接入 — 开发文档

> 2026年6月30日  
> 模型：openPangu 2.0（已开源）  
> API兼容：OpenAI Chat Completions 格式

---

## 一、系统双模式架构

```
我们的调度系统
├── 纯算法模式（传统规则）
│   ├── FIFO排队算法
│   ├── 固定优先级派单
│   ├── 基础异常检测
│   └── 无需外部AI依赖
│
└── AI模式（盘古增强）← 新增
    ├── AI智能派单优化
    ├── AI拥堵预测
    ├── AI异常检测
    ├── AI报表生成
    └── 依赖：盘古大模型API/本地部署
```

### 模式切换

```rust
// Rust服务端：根据配置切换模式
enum DispatchMode {
    PureAlgorithm,  // 纯算法模式
    AiEnhanced,     // AI增强模式
}

struct DispatchConfig {
    mode: DispatchMode,
    pangu_api_url: Option<String>,
    pangu_api_key: Option<String>,
}

impl DispatchEngine {
    async fn optimize_dispatch(&self, context: DispatchContext) -> DispatchResult {
        match self.config.mode {
            DispatchMode::PureAlgorithm => {
                // 传统FIFO + 优先级规则
                self.pure_algorithm_dispatch(context)
            }
            DispatchMode::AiEnhanced => {
                // 调用盘古大模型优化
                self.ai_enhanced_dispatch(context).await
            }
        }
    }
}
```

---

## 二、盘古API调用方式

openPangu 兼容 **OpenAI Chat Completions 格式**，可以用标准 OpenAI 客户端调用。

### 2.1 原生Transformers推理（本地部署）

```python
# 环境要求: Python 3.10, torch, transformers
# 硬件: 昇腾Atlas NPU 或 GPU

from transformers import AutoModelForCausalLM, AutoTokenizer

model_path = "/path/to/openPangu-2.0-Flash"
tokenizer = AutoTokenizer.from_pretrained(
    model_path, trust_remote_code=True
)
model = AutoModelForCausalLM.from_pretrained(
    model_path, trust_remote_code=True,
    torch_dtype="auto", device_map="npu"
)

def chat(prompt: str, system: str = "你是矿山调度专家") -> str:
    messages = [
        {"role": "system", "content": system},
        {"role": "user", "content": prompt},
    ]
    text = tokenizer.apply_chat_template(
        messages, tokenize=False, add_generation_prompt=True
    )
    inputs = tokenizer(text, return_tensors="pt").to(model.device)
    outputs = model.generate(
        **inputs, max_new_tokens=1024,
        temperature=0.7, top_p=0.9
    )
    return tokenizer.decode(outputs[0], skip_special_tokens=True)
```

### 2.2 OpenAI兼容API调用（推荐）

通过 `vllm-ascend` 部署后，可以用标准 OpenAI SDK 调用：

```python
# 部署: vllm-ascend serve openPangu-2.0-Flash
# API地址: http://localhost:8000/v1/chat/completions

from openai import OpenAI

client = OpenAI(
    base_url="http://localhost:8000/v1",  # 本地部署
    api_key="not-needed"  # 本地无需key
)

# ─── 智能派单优化 ─────────────────────────────────
def ai_optimize_dispatch(pit_status: dict, drivers: list) -> str:
    response = client.chat.completions.create(
        model="openPangu-2.0-Flash",
        messages=[
            {"role": "system", "content": "你是矿山运输调度AI助手。根据坑口排队情况和司机位置，给出最优派单方案。"},
            {"role": "user", "content": f"""
当前坑口状态：{pit_status}
可用司机：{drivers}
请给出最优派单顺序，按优先级排列。
"""}
        ],
        temperature=0.3,  # 低温度，更确定性
        max_tokens=512
    )
    return response.choices[0].message.content

# ─── 拥堵预测 ────────────────────────────────────
def ai_predict_congestion(history_data: dict) -> str:
    response = client.chat.completions.create(
        model="openPangu-2.0-Flash",
        messages=[
            {"role": "system", "content": "你是矿山运营分析AI。根据历史数据预测未来拥堵情况。"},
            {"role": "user", "content": f"""
历史排队数据：{history_data}
请预测未来2小时各坑口拥堵情况，并给出分流建议。
"""}
        ],
        temperature=0.3,
        max_tokens=512
    )
    return response.choices[0].message.content
```

### 2.3 华为云API调用（在线服务）

```python
# 通过华为云盘古API调用
import requests

HUAWEI_PANGU_API = "https://api.huaweicloud.com/pangu/v2/chat/completions"
HUAWEI_API_KEY = "your-api-key"

def call_pangu(prompt: str, system: str = "") -> str:
    headers = {
        "Authorization": f"Bearer {HUAWEI_API_KEY}",
        "Content-Type": "application/json"
    }
    data = {
        "model": "openPangu-2.0-Flash",
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": prompt}
        ],
        "temperature": 0.3,
        "max_tokens": 1024
    }
    resp = requests.post(HUAWEI_PANGU_API, headers=headers, json=data)
    return resp.json()["choices"][0]["message"]["content"]
```

---

## 三、Rust服务端集成

```rust
// 在Rust API中调用盘古大模型
#[derive(Deserialize)]
struct PanguConfig {
    api_url: String,
    api_key: String,
    model: String,         // openPangu-2.0-Flash / Pro
    mode: String,          // pure_algorithm / ai_enhanced
}

#[derive(Serialize)]
struct PanguChatRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f32,
    max_tokens: u32,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

async fn call_pangu(config: &PanguConfig, prompt: &str, system: &str) -> Result<String> {
    let client = reqwest::Client::new();
    let request = PanguChatRequest {
        model: config.model.clone(),
        messages: vec![
            Message { role: "system".into(), content: system.into() },
            Message { role: "user".into(), content: prompt.into() },
        ],
        temperature: 0.3,
        max_tokens: 1024,
    };

    let resp = client.post(&config.api_url)
        .header("Authorization", format!("Bearer {}", config.api_key))
        .json(&request)
        .send()
        .await?;

    let result = resp.json::<serde_json::Value>().await?;
    Ok(result["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string())
}

// ─── 派单API集成盘古 ─────────────────────────────
async fn dispatch_waybill(
    State(state): State<AppState>,
    Json(req): Json<DispatchRequest>,
) -> Result<Json<DispatchResponse>, AppError> {
    if state.config.pangu_mode == "ai_enhanced" {
        // AI模式：让盘古推荐派单方案
        let ai_suggestion = call_pangu(
            &state.config.pangu,
            &format!("坑口状态：{:?}，司机状态：{:?}", req.pits, req.drivers),
            "你是矿山调度AI，请给出最优派单顺序。"
        ).await?;
        // 解析AI建议并执行派单
        execute_ai_dispatch(&state, &ai_suggestion).await?
    } else {
        // 纯算法模式：传统规则派单
        execute_algorithm_dispatch(&state, req).await?
    }
}
```

---

## 四、部署方式对照

| 方式 | 延迟 | 数据安全 | 成本 | 适用阶段 |
|:----|:----|:--------|:----|:--------|
| **华为云API** | 网络延迟~500ms | 数据出矿 | 按量付费 | 开发测试 |
| **本地部署Flash** | ~100ms | ✅ 数据不出矿 | 1张昇腾卡≈2-3万 | 生产环境 |
| **本地部署Embedded** | ~30ms | ✅ 数据不出矿 | ARM边缘盒子 | 端侧实时 |

---

## 五、配置文件

```yaml
# config/pangu.yaml
ai:
  mode: pure_algorithm        # pure_algorithm | ai_enhanced
  
  # AI模式配置
  pangu:
    api_url: "http://localhost:8000/v1/chat/completions"
    api_key: ""
    model: "openPangu-2.0-Flash"
    temperature: 0.3
    max_tokens: 1024
    
  # AI功能开关
  features:
    smart_dispatch: true      # 智能派单
    congestion_predict: true  # 拥堵预测
    anomaly_detect: false     # 异常检测
    report_generate: false    # 报表生成
```

---

## 六、快速测试

```bash
# 1. 拉取代码
git clone https://github.com/Winging/openpangu

# 2. 安装依赖
pip install openai  # 或使用华为云SDK

# 3. 测试API调用
python3 -c "
from openai import OpenAI
client = OpenAI(base_url='http://localhost:8000/v1', api_key='test')
resp = client.chat.completions.create(
    model='openPangu-2.0-Flash',
    messages=[{'role': 'user', 'content': '矿山调度优化建议'}]
)
print(resp.choices[0].message.content)
"
```
