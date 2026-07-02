import 'dart:convert';
import 'package:sqflite/sqflite.dart';
import 'package:path/path.dart' as p;
import 'package:uuid/uuid.dart';

/// 本地 SQLite 数据库
///
/// 缓存运单数据，存储离线操作队列。
/// 每个设备一个数据库文件。
class OfflineDatabase {
  static Database? _db;
  static const _dbName = 'offline_cache.db';
  static const _dbVersion = 1;

  // 表名
  static const String tableWaybills = 'local_waybills';
  static const String tableOperations = 'offline_operations';
  static const String tableFences = 'fence_states';

  /// 获取数据库实例（懒加载单例）
  static Future<Database> get instance async {
    _db ??= await _initDatabase();
    return _db!;
  }

  static Future<Database> _initDatabase() async {
    final dbPath = await getDatabasesPath();
    final path = p.join(dbPath, _dbName);

    return openDatabase(
      path,
      version: _dbVersion,
      onCreate: (db, version) async {
        await db.execute('''
          CREATE TABLE $tableWaybills (
            id TEXT PRIMARY KEY,
            serial_no TEXT NOT NULL,
            driver_id TEXT NOT NULL,
            pit_id TEXT NOT NULL,
            pit_name TEXT,
            status TEXT NOT NULL,
            queue_number INTEGER,
            estimated_weight_ton REAL,
            actual_weight_ton REAL,
            dispatch_time TEXT,
            arrive_time TEXT,
            version INTEGER NOT NULL DEFAULT 1,
            data_json TEXT,
            updated_at TEXT NOT NULL,
            last_synced_at TEXT
          )
        ''');

        await db.execute('''
          CREATE TABLE $tableOperations (
            id TEXT PRIMARY KEY,
            idempotency_key TEXT NOT NULL UNIQUE,
            operation_type TEXT NOT NULL,
            waybill_id TEXT NOT NULL,
            waybill_version INTEGER NOT NULL,
            payload TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            -- pending / syncing / synced / conflicted / failed
            retry_count INTEGER NOT NULL DEFAULT 0,
            error_message TEXT,
            created_at TEXT NOT NULL,
            last_tried_at TEXT
          )
        ''');

        await db.execute('''
          CREATE TABLE $tableFences (
            fence_id TEXT NOT NULL,
            driver_id TEXT NOT NULL,
            inside INTEGER NOT NULL DEFAULT 0,
            entered_at TEXT,
            PRIMARY KEY (fence_id, driver_id)
          )
        ''');

        // 索引
        await db.execute('''
          CREATE INDEX idx_ops_status ON $tableOperations(status)
        ''');
        await db.execute('''
          CREATE INDEX idx_ops_created ON $tableOperations(created_at)
        ''');
        await db.execute('''
          CREATE INDEX idx_wb_status ON $tableWaybills(status)
        ''');
      },
    );
  }

  // ─── 运单缓存 ──────────────────────────────────────────────────────────

  /// 批量更新本地运单缓存（从服务端同步后调用）
  Future<void> upsertWaybills(List<Map<String, dynamic>> waybills) async {
    final db = await instance;
    final batch = db.batch();

    for (final wb in waybills) {
      batch.insert(
        tableWaybills,
        {
          'id': wb['id'],
          'serial_no': wb['serial_no'] ?? '',
          'driver_id': wb['driver_id'] ?? '',
          'pit_id': wb['pit_id'] ?? '',
          'pit_name': wb['pit_name'],
          'status': wb['status_raw'] ?? wb['status'] ?? '',
          'queue_number': wb['queue_number'],
          'estimated_weight_ton': wb['estimated_weight_ton'],
          'actual_weight_ton': wb['actual_weight_ton'],
          'dispatch_time': wb['dispatch_time'],
          'arrive_time': wb['arrive_time'],
          'version': wb['version'] ?? 1,
          'data_json': jsonEncode(wb),
          'updated_at': DateTime.now().toUtc().toIso8601String(),
          'last_synced_at': DateTime.now().toUtc().toIso8601String(),
        },
        conflictAlgorithm: ConflictAlgorithm.replace,
      );
    }

    await batch.commit(noResult: true);
  }

  /// 获取活跃运单（本地缓存中未完成的）
  Future<List<Map<String, dynamic>>> getActiveWaybills() async {
    final db = await instance;
    return db.query(
      tableWaybills,
      where: "status NOT IN ('completed', 'cancelled')",
      orderBy: 'updated_at DESC',
    );
  }

  /// 按ID获取运单
  Future<Map<String, dynamic>?> getWaybill(String id) async {
    final db = await instance;
    final rows = await db.query(tableWaybills, where: 'id = ?', whereArgs: [id]);
    return rows.isNotEmpty ? rows.first : null;
  }

  /// 获取运单的版本号（用于离线操作的乐观锁）
  Future<int> getWaybillVersion(String waybillId) async {
    final db = await instance;
    final rows = await db.query(
      tableWaybills,
      columns: ['version'],
      where: 'id = ?',
      whereArgs: [waybillId],
    );
    if (rows.isEmpty) return 0;
    return (rows.first['version'] as int?) ?? 1;
  }

  /// 更新运单状态（客户端乐观更新）
  Future<void> updateWaybillStatus(
    String waybillId,
    String newStatus,
  ) async {
    final db = await instance;
    await db.update(
      tableWaybills,
      {
        'status': newStatus,
        'updated_at': DateTime.now().toUtc().toIso8601String(),
      },
      where: 'id = ?',
      whereArgs: [waybillId],
    );
  }

  // ─── 离线操作队列 ──────────────────────────────────────────────────────

  /// 添加离线操作到队列
  ///
  /// 每次用户在离线状态下执行操作（到场/入队/装车/称重）时调用。
  Future<Map<String, dynamic>> addOperation({
    required String operationType,
    required String waybillId,
    required int waybillVersion,
    required Map<String, dynamic> payload,
  }) async {
    final db = await instance;
    final id = const Uuid().v4();
    final idempotencyKey = const Uuid().v4();
    final now = DateTime.now().toUtc().toIso8601String();

    final op = {
      'id': id,
      'idempotency_key': idempotencyKey,
      'operation_type': operationType,
      'waybill_id': waybillId,
      'waybill_version': waybillVersion,
      'payload': jsonEncode(payload),
      'status': 'pending',
      'retry_count': 0,
      'error_message': null,
      'created_at': now,
      'last_tried_at': null,
    };

    await db.insert(tableOperations, op);
    return op;
  }

  /// 获取待同步的离线操作（按创建时间排序）
  Future<List<Map<String, dynamic>>> getPendingOperations() async {
    final db = await instance;
    return db.query(
      tableOperations,
      where: "status IN ('pending', 'failed') AND retry_count < 5",
      orderBy: 'created_at ASC',
    );
  }

  /// 获取所有待同步操作（含重试超限的）
  Future<List<Map<String, dynamic>>> getAllPendingOperations() async {
    final db = await instance;
    return db.query(
      tableOperations,
      where: "status IN ('pending', 'failed')",
      orderBy: 'created_at ASC',
    );
  }

  /// 标记操作状态
  Future<void> markOperationStatus({
    required String id,
    required String status,
    String? errorMessage,
  }) async {
    final db = await instance;
    final now = DateTime.now().toUtc().toIso8601String();
    await db.update(
      tableOperations,
      {
        'status': status,
        'error_message': errorMessage,
        'last_tried_at': now,
        if (status == 'synced') ...{
          'retry_count': 0,
        } else ...{
          'retry_count': db.rawUpdate(
            'UPDATE $tableOperations SET retry_count = retry_count + 1 WHERE id = ?',
            [id],
          ),
        },
      },
      where: 'id = ?',
      whereArgs: [id],
    );
  }

  /// 标记操作同步成功
  Future<void> markSynced(String id) async {
    await markOperationStatus(id: id, status: 'synced');
  }

  /// 标记操作冲突
  Future<void> markConflicted(String id, String serverState) async {
    await markOperationStatus(
      id: id,
      status: 'conflicted',
      errorMessage: serverState,
    );
  }

  /// 获取待同步数量
  Future<int> getPendingCount() async {
    final db = await instance;
    final result = await db.rawQuery(
      "SELECT COUNT(*) as cnt FROM $tableOperations WHERE status IN ('pending', 'failed')",
    );
    return (result.first['cnt'] as int?) ?? 0;
  }

  // ─── 围栏状态 ──────────────────────────────────────────────────────────

  /// 更新围栏状态
  Future<void> updateFenceState(
    String fenceId,
    String driverId,
    bool inside,
  ) async {
    final db = await instance;
    await db.insert(
      tableFences,
      {
        'fence_id': fenceId,
        'driver_id': driverId,
        'inside': inside ? 1 : 0,
        'entered_at': inside ? DateTime.now().toUtc().toIso8601String() : null,
      },
      conflictAlgorithm: ConflictAlgorithm.replace,
    );
  }

  /// 获取围栏状态
  Future<bool> getFenceState(String fenceId, String driverId) async {
    final db = await instance;
    final rows = await db.query(
      tableFences,
      columns: ['inside'],
      where: 'fence_id = ? AND driver_id = ?',
      whereArgs: [fenceId, driverId],
    );
    if (rows.isEmpty) return false;
    return (rows.first['inside'] as int?) == 1;
  }

  // ─── 清理 ──────────────────────────────────────────────────────────────

  /// 清理已同步的操作（7天前的）
  Future<void> cleanOldOperations() async {
    final db = await instance;
    final cutoff = DateTime.now()
        .subtract(const Duration(days: 7))
        .toUtc()
        .toIso8601String();
    await db.delete(
      tableOperations,
      where: "status = 'synced' AND created_at < ?",
      whereArgs: [cutoff],
    );
  }

  /// 清理已完成运单（3天前的）
  Future<void> cleanOldWaybills() async {
    final db = await instance;
    final cutoff = DateTime.now()
        .subtract(const Duration(days: 3))
        .toUtc()
        .toIso8601String();
    await db.delete(
      tableWaybills,
      where: "status IN ('completed', 'cancelled') AND updated_at < ?",
      whereArgs: [cutoff],
    );
  }

  /// 关闭数据库
  Future<void> close() async {
    final db = await instance;
    await db.close();
    _db = null;
  }
}
