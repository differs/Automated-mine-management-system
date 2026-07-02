<script setup lang="ts">
import { ref } from 'vue'

const activeRole = ref<'admin' | 'driver' | 'pit' | null>(null)

const features = [
  { icon: '📋', title: '智能派单', desc: '自动分配运单，司机实时接收任务通知' },
  { icon: '🔢', title: '坑口排队', desc: '到场签到、自动排号、按序叫号，杜绝插队' },
  { icon: '🚜', title: '装车管理', desc: '记录装车开始/结束时间，统计作业效率' },
  { icon: '⚖️', title: '智能称重', desc: '地磅录入、重量校验、异常自动告警' },
  { icon: '📊', title: '运营看板', desc: '实时数据大屏，趟次/吨位/效率一目了然' },
  { icon: '🔔', title: '告警中心', desc: '超时/重量偏差/异常行为自动检测' },
]
</script>

<template>
  <div class="demo-hub">
    <!-- Hero -->
    <header class="hero">
      <div class="hero-badge">🚧 演示环境</div>
      <h1>矿山智能调度管理系统</h1>
      <p class="hero-sub">Automated Mine Management System</p>
      <p class="hero-desc">
        替换微信群、电话和 Excel，实现派单、排队、装车、称重全流程数字化闭环
      </p>
      <div class="hero-actions">
        <a href="/admin-web/dist/index.html" class="btn btn-primary" target="_blank">
          🖥️ 进入调度后台
        </a>
        <a href="/driver-miniapp/dist/index.html" class="btn btn-success" target="_blank">
          🚛 进入司机端
        </a>
        <a href="/pit-h5/dist/index.html" class="btn btn-warning" target="_blank">
          ⛰️ 进入坑口端
        </a>
      </div>
    </header>

    <!-- Flow Diagram -->
    <section class="section">
      <h2>核心业务流程</h2>
      <div class="flow">
        <div class="flow-step">
          <div class="flow-icon">📋</div>
          <div class="flow-label">创建运单</div>
          <div class="flow-role">调度员</div>
        </div>
        <div class="flow-arrow">→</div>
        <div class="flow-step">
          <div class="flow-icon">🚛</div>
          <div class="flow-label">派单到场</div>
          <div class="flow-role">司机</div>
        </div>
        <div class="flow-arrow">→</div>
        <div class="flow-step">
          <div class="flow-icon">🔢</div>
          <div class="flow-label">排队叫号</div>
          <div class="flow-role">坑口管理员</div>
        </div>
        <div class="flow-arrow">→</div>
        <div class="flow-step">
          <div class="flow-icon">🚜</div>
          <div class="flow-label">装车作业</div>
          <div class="flow-role">坑口管理员</div>
        </div>
        <div class="flow-arrow">→</div>
        <div class="flow-step">
          <div class="flow-icon">⚖️</div>
          <div class="flow-label">称重完成</div>
          <div class="flow-role">地磅员</div>
        </div>
      </div>
    </section>

    <!-- Features -->
    <section class="section">
      <h2>功能特性</h2>
      <div class="features-grid">
        <div v-for="f in features" :key="f.title" class="feature-card">
          <div class="feature-icon">{{ f.icon }}</div>
          <h3>{{ f.title }}</h3>
          <p>{{ f.desc }}</p>
        </div>
      </div>
    </section>

    <!-- Tech Stack -->
    <section class="section tech-section">
      <h2>技术栈</h2>
      <div class="tech-tags">
        <span class="tag tag-rust">Rust + Axum</span>
        <span class="tag tag-db">PostgreSQL</span>
        <span class="tag tag-cache">Redis</span>
        <span class="tag tag-vue">Vue 3 + Vite</span>
        <span class="tag tag-flutter">Flutter / H5</span>
        <span class="tag tag-docker">Docker</span>
        <span class="tag tag-ws">WebSocket</span>
        <span class="tag tag-jwt">JWT Auth</span>
      </div>
    </section>

    <!-- Architecture diagram -->
    <section class="section">
      <h2>系统架构</h2>
      <div class="arch">
        <div class="arch-layer">
          <div class="arch-label">客户端层</div>
          <div class="arch-items">
            <span class="arch-item">Admin Web<br><small>调度后台</small></span>
            <span class="arch-item">司机端<br><small>H5 / 小程序</small></span>
            <span class="arch-item">坑口端<br><small>H5 / 小程序</small></span>
          </div>
        </div>
        <div class="arch-arrow">⬇ WebSocket / HTTP ⬇</div>
        <div class="arch-layer">
          <div class="arch-label">服务层</div>
          <div class="arch-items">
            <span class="arch-item primary">Rust API<br><small>Axum 服务</small></span>
          </div>
        </div>
        <div class="arch-arrow">⬇ ⬇</div>
        <div class="arch-layer">
          <div class="arch-label">数据层</div>
          <div class="arch-items">
            <span class="arch-item">PostgreSQL<br><small>业务数据</small></span>
            <span class="arch-item">Redis<br><small>实时队列</small></span>
          </div>
        </div>
      </div>
    </section>

    <!-- Footer -->
    <footer class="footer">
      <div class="footer-links">
        <a href="https://github.com" target="_blank">📖 项目文档</a>
        <span class="footer-sep">·</span>
        <a href="/api/v1/health" target="_blank">🔌 API 健康检查</a>
        <span class="footer-sep">·</span>
        <span>v0.1.0</span>
      </div>
      <p class="footer-note">演示环境 · 仅用于产品展示</p>
    </footer>
  </div>
</template>

<style>
*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }

:root {
  --primary: #2563eb;
  --primary-dark: #1d4ed8;
  --success: #16a34a;
  --warning: #d97706;
  --danger: #dc2626;
  --gray-50: #f9fafb;
  --gray-100: #f3f4f6;
  --gray-200: #e5e7eb;
  --gray-300: #d1d5db;
  --gray-400: #9ca3af;
  --gray-500: #6b7280;
  --gray-600: #4b5563;
  --gray-700: #374151;
  --gray-800: #1f2937;
  --gray-900: #111827;
}

body {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'PingFang SC', 'Noto Sans SC', sans-serif;
  color: var(--gray-800);
  background: var(--gray-50);
  line-height: 1.6;
}

.demo-hub { max-width: 1100px; margin: 0 auto; padding: 0 24px; }

/* Hero */
.hero {
  text-align: center;
  padding: 64px 0 48px;
  background: linear-gradient(135deg, #1e3a5f 0%, #2563eb 100%);
  margin: 0 -24px;
  padding: 64px 24px;
  color: white;
}

.hero-badge {
  display: inline-block;
  background: rgba(255,255,255,0.15);
  padding: 4px 14px;
  border-radius: 999px;
  font-size: 12px;
  margin-bottom: 16px;
}

.hero h1 { font-size: 36px; font-weight: 800; margin-bottom: 8px; }
.hero-sub { font-size: 18px; opacity: 0.8; margin-bottom: 12px; }
.hero-desc { font-size: 15px; opacity: 0.7; max-width: 600px; margin: 0 auto 32px; }

.hero-actions {
  display: flex;
  gap: 12px;
  justify-content: center;
  flex-wrap: wrap;
}

.btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 12px 28px;
  border-radius: 8px;
  font-size: 15px;
  font-weight: 600;
  text-decoration: none;
  transition: all 0.2s;
}

.btn:hover { transform: translateY(-2px); box-shadow: 0 8px 24px rgba(0,0,0,0.2); }

.btn-primary { background: white; color: var(--primary); }
.btn-success { background: var(--success); color: white; }
.btn-warning { background: var(--warning); color: white; }

/* Sections */
.section { padding: 48px 0; }
.section h2 {
  text-align: center;
  font-size: 24px;
  font-weight: 700;
  margin-bottom: 32px;
  color: var(--gray-900);
}

/* Flow */
.flow {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  flex-wrap: wrap;
}

.flow-step {
  background: white;
  border: 1px solid var(--gray-200);
  border-radius: 12px;
  padding: 16px 24px;
  text-align: center;
  min-width: 100px;
}

.flow-icon { font-size: 28px; margin-bottom: 4px; }
.flow-label { font-size: 13px; font-weight: 600; color: var(--gray-700); }
.flow-role { font-size: 11px; color: var(--gray-400); margin-top: 2px; }
.flow-arrow { font-size: 24px; color: var(--gray-300); font-weight: 700; }

/* Features */
.features-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: 20px;
}

.feature-card {
  background: white;
  border: 1px solid var(--gray-200);
  border-radius: 12px;
  padding: 28px;
  transition: box-shadow 0.2s;
}

.feature-card:hover { box-shadow: 0 4px 20px rgba(0,0,0,0.06); }

.feature-icon { font-size: 32px; margin-bottom: 12px; }
.feature-card h3 { font-size: 16px; font-weight: 600; margin-bottom: 8px; }
.feature-card p { font-size: 13px; color: var(--gray-500); line-height: 1.7; }

/* Tech */
.tech-section { background: var(--gray-100); margin: 0 -24px; padding: 48px 24px; }
.tech-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  justify-content: center;
}

.tag {
  padding: 6px 16px;
  border-radius: 999px;
  font-size: 13px;
  font-weight: 500;
}
.tag-rust { background: #fed7d7; color: #9b2c2c; }
.tag-db { background: #c6f6d5; color: #276749; }
.tag-cache { background: #fefcbf; color: #975a16; }
.tag-vue { background: #c6f6d5; color: #276749; }
.tag-flutter { background: #bee3f8; color: #2a4365; }
.tag-docker { background: #e9d8fd; color: #553c9a; }
.tag-ws { background: #feebc8; color: #9c4221; }
.tag-jwt { background: #fed7d7; color: #9b2c2c; }

/* Architecture */
.arch {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
}

.arch-layer {
  background: white;
  border: 1px solid var(--gray-200);
  border-radius: 12px;
  padding: 16px 24px;
  width: 100%;
  max-width: 600px;
  text-align: center;
}

.arch-label {
  font-size: 12px;
  font-weight: 600;
  color: var(--gray-400);
  text-transform: uppercase;
  letter-spacing: 0.08em;
  margin-bottom: 12px;
}

.arch-items {
  display: flex;
  gap: 12px;
  justify-content: center;
  flex-wrap: wrap;
}

.arch-item {
  background: var(--gray-50);
  border: 1px solid var(--gray-200);
  border-radius: 8px;
  padding: 10px 18px;
  font-size: 13px;
  font-weight: 600;
  min-width: 100px;
}

.arch-item.primary {
  background: #dbeafe;
  border-color: #93b4f5;
  color: var(--primary);
}

.arch-item small { font-weight: 400; color: var(--gray-400); font-size: 11px; }

.arch-arrow { font-size: 14px; color: var(--gray-400); }

/* Footer */
.footer {
  text-align: center;
  padding: 32px 0;
  border-top: 1px solid var(--gray-200);
}

.footer-links { margin-bottom: 8px; }
.footer-links a { color: var(--primary); text-decoration: none; font-size: 13px; }
.footer-sep { margin: 0 8px; color: var(--gray-300); }
.footer-note { font-size: 12px; color: var(--gray-400); }

/* Responsive */
@media (max-width: 768px) {
  .hero h1 { font-size: 24px; }
  .flow { flex-direction: column; }
  .flow-arrow { transform: rotate(90deg); }
}
</style>
