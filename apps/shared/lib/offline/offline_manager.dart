import 'package:flutter/material.dart';
import '../offline/offline.dart';

/// 离线调度管理器
///
/// 在 App 入口初始化，为整个应用提供离线能力。
/// 使用方式:
///   ```dart
///   final offline = OfflineManager(
///     baseUrl: 'http://10.0.2.2:3000/api/v1',
///   );
///   await offline.initialize();
///   // 在需要离线操作的页面使用
///   await offline.addOperation(...)
///   ```
class OfflineManager {
  late final OfflineDatabase localDb;
  late final OfflineOperationQueue queue;
  late final OfflineApiService api;
  late final ConnectivityMonitor connectivity;
  late final SyncEngine syncEngine;

  final String baseUrl;

  OfflineManager({required this.baseUrl});

  /// 初始化所有离线组件
  Future<void> initialize() async {
    localDb = OfflineDatabase();
    queue = OfflineOperationQueue(localDb);
    api = OfflineApiService(baseUrl: '$baseUrl/offline');
    connectivity = ConnectivityMonitor();

    syncEngine = SyncEngine(
      localDb: localDb,
      queue: queue,
      api: OfflineApiService(baseUrl: baseUrl),
      connectivity: connectivity,
    );

    // 启动网络监听
    await connectivity.start();

    // 启动同步引擎
    await syncEngine.start();

    // 监听同步状态变化
    syncEngine.onStatusChanged = (status) {
      // 可以在这里做全局 UI 提示
    };
  }

  /// 获取待同步数量
  Future<int> get pendingCount => queue.getPendingCount();

  /// 添加离线操作（业务代码统一入口）
  ///
  /// 示例:
  ///   ```dart
  ///   await offline.addArriveOperation(
  ///     waybillId: waybill.id,
  ///     waybillVersion: waybill.version,
  ///   );
  ///   ```
  Future<void> addArriveOperation({
    required String waybillId,
    required int waybillVersion,
    String arrivalSource = 'offline_app',
  }) async {
    await queue.addArriveOperation(
      waybillId: waybillId,
      waybillVersion: waybillVersion,
      arrivalSource: arrivalSource,
    );
    // 如果在在线状态，立即触发同步
    if (connectivity.isOnline) {
      syncEngine.syncNow();
    }
  }

  Future<void> addQueueJoinOperation({
    required String waybillId,
    required int waybillVersion,
    required String driverId,
    required String pitId,
  }) async {
    await queue.addQueueJoinOperation(
      waybillId: waybillId,
      waybillVersion: waybillVersion,
      driverId: driverId,
      pitId: pitId,
    );
    if (connectivity.isOnline) {
      syncEngine.syncNow();
    }
  }

  Future<void> addLoadingStartOperation({
    required String waybillId,
    required int waybillVersion,
    required String operatorId,
  }) async {
    await queue.addLoadingStartOperation(
      waybillId: waybillId,
      waybillVersion: waybillVersion,
      operatorId: operatorId,
    );
    if (connectivity.isOnline) {
      syncEngine.syncNow();
    }
  }

  Future<void> addLoadingFinishOperation({
    required String waybillId,
    required int waybillVersion,
    required String operatorId,
  }) async {
    await queue.addLoadingFinishOperation(
      waybillId: waybillId,
      waybillVersion: waybillVersion,
      operatorId: operatorId,
    );
    if (connectivity.isOnline) {
      syncEngine.syncNow();
    }
  }

  Future<void> addWeighOperation({
    required String waybillId,
    required int waybillVersion,
    required double grossWeight,
    required double tareWeight,
    String? operatorId,
  }) async {
    await queue.addWeighOperation(
      waybillId: waybillId,
      waybillVersion: waybillVersion,
      grossWeight: grossWeight,
      tareWeight: tareWeight,
      operatorId: operatorId,
    );
    if (connectivity.isOnline) {
      syncEngine.syncNow();
    }
  }

  /// 释放资源
  Future<void> dispose() async {
    await syncEngine.stop();
    connectivity.dispose();
  }
}

/// 离线状态指示器 Widget
///
/// 在页面顶部显示离线/同步状态。
/// ```dart
/// OfflineStatusBar(offlineManager: offline)
/// ```
class OfflineStatusBar extends StatefulWidget {
  final OfflineManager offlineManager;
  final Color connectedColor;
  final Color disconnectedColor;

  const OfflineStatusBar({
    super.key,
    required this.offlineManager,
    this.connectedColor = const Color(0xFF16A34A),
    this.disconnectedColor = const Color(0xFFDC2626),
  });

  @override
  State<OfflineStatusBar> createState() => _OfflineStatusBarState();
}

class _OfflineStatusBarState extends State<OfflineStatusBar> {
  bool _isOnline = true;
  int _pendingCount = 0;

  @override
  void initState() {
    super.initState();
    _isOnline = widget.offlineManager.connectivity.isOnline;

    widget.offlineManager.connectivity.onStatusChanged.listen((online) {
      if (mounted) {
        setState(() => _isOnline = online);
        if (!online) {
          _refreshPendingCount();
        }
      }
    });

    widget.offlineManager.syncEngine.onStatusChanged = (status) {
      if (mounted) {
        setState(() {
          if (status.type == 'sync_completed') {
            _pendingCount = 0;
          }
        });
        _refreshPendingCount();
      }
    };

    _refreshPendingCount();
  }

  Future<void> _refreshPendingCount() async {
    final count = await widget.offlineManager.pendingCount;
    if (mounted) {
      setState(() => _pendingCount = count);
    }
  }

  @override
  Widget build(BuildContext context) {
    if (_isOnline && _pendingCount == 0) {
      return const SizedBox.shrink();
    }

    return Container(
      width: double.infinity,
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
      color: _isOnline
          ? const Color(0xFFFFF3CD) // 黄色：有未同步数据
          : widget.disconnectedColor.withValues(alpha: 0.9),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(
            _isOnline ? Icons.sync : Icons.wifi_off,
            size: 16,
            color: _isOnline ? Colors.orange[900] : Colors.white,
          ),
          const SizedBox(width: 8),
          Text(
            _isOnline
                ? '$_pendingCount 条待同步...'
                : '当前无网络，操作将在恢复后自动同步',
            style: TextStyle(
              fontSize: 13,
              color: _isOnline ? Colors.orange[900] : Colors.white,
            ),
          ),
          if (!_isOnline) ...[
            const SizedBox(width: 8),
            SizedBox(
              width: 14,
              height: 14,
              child: CircularProgressIndicator(
                strokeWidth: 2,
                color: Colors.white.withValues(alpha: 0.7),
              ),
            ),
          ],
        ],
      ),
    );
  }
}

/// 检查网络后执行操作
///
/// 在线时直接调用，离线时加入队列。
/// ```dart
/// await OfflineAction.run(
///   offlineManager: offline,
///   onlineAction: () => api.arrive(waybillId),
///   offlineAction: () => offline.addArriveOperation(...),
///   waybillId: waybill.id,
///   waybillVersion: waybill.version,
/// );
/// ```
class OfflineAction {
  /// 执行操作（自动判断在线/离线路径）
  static Future<void> run({
    required OfflineManager offlineManager,
    required Future<void> Function() onlineAction,
    required Future<void> Function() offlineAction,
    required String waybillId,
    required int waybillVersion,
  }) async {
    if (offlineManager.connectivity.isOnline) {
      try {
        await onlineAction();
      } catch (_) {
        // 在线请求失败，降级到离线
        await offlineAction();
      }
    } else {
      await offlineAction();
    }
  }
}
