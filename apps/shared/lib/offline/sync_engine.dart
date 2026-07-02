import 'dart:async';
import 'dart:convert';
import 'offline_database.dart';
import 'offline_queue.dart';
import 'connectivity_monitor.dart';
import 'offline_api_service.dart';

/// 同步引擎
///
/// 核心职责：
///   1. 监听网络状态变化（离线→在线时触发同步）
///   2. 定时轮询待同步操作
///   3. 批量提交离线操作到服务端
///   4. 处理冲突和错误
///   5. 同步完成后拉取服务端最新状态
///
/// 生命周期：
///   start() → 启动监听循环 → 每次同步完成继续等待 → stop() 停止
class SyncEngine {
  final OfflineDatabase _localDb;
  final OfflineOperationQueue _queue;
  final OfflineApiService _api;
  final ConnectivityMonitor _connectivity;

  bool _isRunning = false;
  bool _isSyncing = false;
  StreamSubscription? _connectivitySub;
  Timer? _periodicTimer;

  /// 同步状态回调
  void Function(SyncStatus status)? onStatusChanged;

  /// 冲突回调：返回 true = 以本地为准，false = 以服务端为准
  Future<bool> Function(String waybillId, Map<String, dynamic> serverState)?
      onConflict;

  SyncEngine({
    required OfflineDatabase localDb,
    required OfflineOperationQueue queue,
    required OfflineApiService api,
    required ConnectivityMonitor connectivity,
  })  : _localDb = localDb,
        _queue = queue,
        _api = api,
        _connectivity = connectivity;

  /// 启动同步引擎
  Future<void> start() async {
    if (_isRunning) return;
    _isRunning = true;

    // 监听网络状态
    _connectivitySub = _connectivity.onStatusChanged.listen((isOnline) {
      if (isOnline) {
        _sync();
      }
    });

    // 定时同步（每30秒检查一次，防止网络状态监听漏掉）
    _periodicTimer = Timer.periodic(const Duration(seconds: 30), (_) {
      if (_connectivity.isOnline) {
        _sync();
      }
    });

    // 启动时立即尝试同步
    if (_connectivity.isOnline) {
      await _sync();
    }
  }

  /// 停止同步引擎
  Future<void> stop() async {
    _isRunning = false;
    await _connectivitySub?.cancel();
    _periodicTimer?.cancel();
  }

  /// 主动触发一次同步
  Future<SyncResult> syncNow() => _sync();

  /// 核心同步方法
  Future<SyncResult> _sync() async {
    if (_isSyncing) return SyncResult(isSyncing: true);
    _isSyncing = true;

    _notify(SyncStatus(type: 'sync_started'));

    try {
      // 1. 获取待同步操作
      final pendingOps = await _localDb.getPendingOperations();

      if (pendingOps.isEmpty) {
        // 没有待同步操作，但尝试拉取服务端最新状态
        await _fetchServerState();
        _notify(SyncStatus(type: 'sync_completed', syncedCount: 0));
        _isSyncing = false;
        return SyncResult(syncedCount: 0);
      }

      // 2. 构建批量同步请求
      final operations = pendingOps.map((op) {
        return {
          'idempotency_key': op['idempotency_key'],
          'operation_type': op['operation_type'],
          'waybill_id': op['waybill_id'],
          'waybill_version': op['waybill_version'],
          'payload': jsonDecode(op['payload'] as String),
          'occurred_at': op['created_at'],
        };
      }).toList();

      // 获取 operator_id
      final sampleOp = pendingOps.first;
      final payload = jsonDecode(sampleOp['payload'] as String);
      final operatorId = payload['driver_id'] ??
          payload['operator_id'] ??
          payload['pit_id'] ??
          'unknown';

      // 3. 提交到服务端
      final response = await _api.syncOperations(
        deviceId: 'flutter_${_connectivity.deviceId}',
        operatorId: operatorId,
        operations: operations,
      );

      // 4. 处理每个操作的结果
      int synced = 0;
      int conflicted = 0;
      int failed = 0;

      for (final result in response['results'] as List<dynamic>) {
        final key = result['idempotency_key'] as String;
        final status = result['status'] as String;
        final serverVersion = result['server_version'] as int? ?? 0;

        // 找到对应的本地操作
        final localOp = pendingOps.firstWhere(
          (o) => o['idempotency_key'] == key,
          orElse: () => <String, dynamic>{},
        );
        if (localOp['id'] == null) continue;

        final localId = localOp['id'] as String;

        switch (status) {
          case 'synced':
            await _localDb.markSynced(localId);
            // 更新本地运单版本号
            await _updateLocalVersion(
              localOp['waybill_id'] as String,
              serverVersion,
            );
            synced++;
            break;

          case 'conflict':
            // 通知业务层处理冲突
            final resolveWithLocal = await onConflict?.call(
                  localOp['waybill_id'] as String,
                  result,
                ) ??
                false;

            if (resolveWithLocal) {
              // 以本地为准：强制覆盖
              await _forceSyncOperation(localOp);
              await _localDb.markSynced(localId);
              synced++;
            } else {
              await _localDb.markConflicted(localId, jsonEncode(result));
              conflicted++;
            }
            break;

          case 'error':
          case 'skipped':
            await _localDb.markOperationStatus(
              id: localId,
              status: 'failed',
              errorMessage: result['message'],
            );
            failed++;
            break;
        }
      }

      // 5. 同步完成后拉取服务端最新状态
      await _fetchServerState();

      _notify(SyncStatus(
        type: 'sync_completed',
        syncedCount: synced,
        conflictedCount: conflicted,
        failedCount: failed,
      ));

      _isSyncing = false;
      return SyncResult(
        syncedCount: synced,
        conflictedCount: conflicted,
        failedCount: failed,
      );
    } catch (e) {
      _notify(SyncStatus(type: 'sync_error', error: e.toString()));
      _isSyncing = false;
      return SyncResult(error: e.toString());
    }
  }

  /// 强制同步（以本地数据为准覆盖服务端）
  Future<void> _forceSyncOperation(Map<String, dynamic> op) async {
    try {
      // 直接调用对应的业务 API
      final payload = jsonDecode(op['payload'] as String);
      switch (op['operation_type'] as String) {
        case 'arrive':
          await _api.forceArrive(op['waybill_id'] as String, payload);
          break;
        case 'queue_join':
          await _api.forceQueueJoin(op['waybill_id'] as String, payload);
          break;
        case 'loading_start':
        case 'loading_finish':
        case 'weigh':
          // 这些操作有幂等性，直接重试
          break;
      }
    } catch (e) {
      // 强制同步失败，记录日志但继续
    }
  }

  /// 更新本地运单版本号
  Future<void> _updateLocalVersion(String waybillId, int serverVersion) async {
    final db = await OfflineDatabase.instance;
    await db.update(
      'local_waybills',
      {'version': serverVersion, 'last_synced_at': DateTime.now().toUtc().toIso8601String()},
      where: 'id = ?',
      whereArgs: [waybillId],
    );
  }

  /// 拉取服务端最新状态更新本地缓存
  Future<void> _fetchServerState() async {
    try {
      final db = await OfflineDatabase.instance;

      // 获取本地活跃运单中的 driver_id 和 pit_id
      final localWaybills = await db.query(
        'local_waybills',
        columns: ['driver_id'],
        distinct: true,
      );
      final localPits = await db.query(
        'local_waybills',
        columns: ['pit_id'],
        distinct: true,
      );

      // 只拉取第一个活跃运单的状态（简化处理）
      if (localWaybills.isNotEmpty) {
        final driverId = localWaybills.first['driver_id'] as String;
        final serverState = await _api.fetchSyncState(
          operatorId: driverId,
          operatorType: 'driver',
        );
        if (serverState['waybills'] != null) {
          await _localDb.upsertWaybills(
            List<Map<String, dynamic>>.from(serverState['waybills']),
          );
        }
      }
    } catch (e) {
      // 拉取失败不阻塞主流程
    }
  }

  void _notify(SyncStatus status) {
    onStatusChanged?.call(status);
  }
}

/// 同步状态
class SyncStatus {
  final String type; // sync_started / syncing / sync_completed / sync_error
  final int syncedCount;
  final int conflictedCount;
  final int failedCount;
  final int pendingCount;
  final String? error;

  SyncStatus({
    required this.type,
    this.syncedCount = 0,
    this.conflictedCount = 0,
    this.failedCount = 0,
    this.pendingCount = 0,
    this.error,
  });
}

/// 同步结果
class SyncResult {
  final bool isSyncing;
  final int syncedCount;
  final int conflictedCount;
  final int failedCount;
  final String? error;

  SyncResult({
    this.isSyncing = false,
    this.syncedCount = 0,
    this.conflictedCount = 0,
    this.failedCount = 0,
    this.error,
  });
}
