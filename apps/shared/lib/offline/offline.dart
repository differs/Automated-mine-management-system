/// 离线调度 - 共享包
///
/// 司机端和坑口端共用此包的离线能力：
///   1. 本地 SQLite 数据库（缓存运单+离线操作队列）
///   2. 离线操作队列（pending → syncing → synced/conflicted）
///   3. 同步引擎（网络恢复时自动批量提交）
///   4. 网络状态监听
///   5. UI 组件（状态指示器、在线/离线操作路由）
///
/// 使用方式:
///   ```dart
///   final offline = OfflineManager(baseUrl: '...');
///   await offline.initialize();
///
///   // 在离线状态下操作
///   await offline.addArriveOperation(waybillId: id, waybillVersion: 1);
///
///   // 在页面顶部显示离线状态
///   OfflineStatusBar(offlineManager: offline)
///   ```
library offline;

export 'offline_database.dart';
export 'offline_queue.dart';
export 'sync_engine.dart';
export 'connectivity_monitor.dart';
export 'offline_api_service.dart';
export 'offline_manager.dart';
