import 'offline_database.dart';

/// 离线操作队列包装类
///
/// 提供简洁的 API 让业务代码无需直接操作 SQLite。
class OfflineOperationQueue {
  final OfflineDatabase _db;

  OfflineOperationQueue(this._db);

  /// 添加到场操作到离线队列
  Future<void> addArriveOperation({
    required String waybillId,
    required int waybillVersion,
    required String arrivalSource,
  }) async {
    await _db.addOperation(
      operationType: 'arrive',
      waybillId: waybillId,
      waybillVersion: waybillVersion,
      payload: {
        'arrival_source': arrivalSource,
        'timestamp': DateTime.now().toUtc().toIso8601String(),
      },
    );
  }

  /// 添加入队操作到离线队列
  Future<void> addQueueJoinOperation({
    required String waybillId,
    required int waybillVersion,
    required String driverId,
    required String pitId,
  }) async {
    await _db.addOperation(
      operationType: 'queue_join',
      waybillId: waybillId,
      waybillVersion: waybillVersion,
      payload: {
        'driver_id': driverId,
        'pit_id': pitId,
        'timestamp': DateTime.now().toUtc().toIso8601String(),
      },
    );
  }

  /// 添加开始装车操作到离线队列
  Future<void> addLoadingStartOperation({
    required String waybillId,
    required int waybillVersion,
    required String operatorId,
  }) async {
    await _db.addOperation(
      operationType: 'loading_start',
      waybillId: waybillId,
      waybillVersion: waybillVersion,
      payload: {
        'operator_id': operatorId,
        'timestamp': DateTime.now().toUtc().toIso8601String(),
      },
    );
  }

  /// 添加结束装车操作到离线队列
  Future<void> addLoadingFinishOperation({
    required String waybillId,
    required int waybillVersion,
    required String operatorId,
  }) async {
    await _db.addOperation(
      operationType: 'loading_finish',
      waybillId: waybillId,
      waybillVersion: waybillVersion,
      payload: {
        'operator_id': operatorId,
        'timestamp': DateTime.now().toUtc().toIso8601String(),
      },
    );
  }

  /// 添加称重操作到离线队列
  Future<void> addWeighOperation({
    required String waybillId,
    required int waybillVersion,
    required double grossWeight,
    required double tareWeight,
    String? operatorId,
  }) async {
    await _db.addOperation(
      operationType: 'weigh',
      waybillId: waybillId,
      waybillVersion: waybillVersion,
      payload: {
        'gross_weight': grossWeight,
        'tare_weight': tareWeight,
        'operator_id': operatorId,
        'timestamp': DateTime.now().toUtc().toIso8601String(),
      },
    );
  }

  /// 获取待同步操作数量
  Future<int> getPendingCount() => _db.getPendingCount();

  /// 获取所有待同步操作
  Future<List<Map<String, dynamic>>> getPendingOperations() =>
      _db.getPendingOperations();
}
