import 'dart:convert';
import 'package:http/http.dart' as http;

/// 离线 API 服务
///
/// 扩展 ApiService，增加离线专用的批量同步和状态拉取接口。
class OfflineApiService {
  final String baseUrl;

  OfflineApiService({required this.baseUrl});

  /// 批量提交离线操作
  Future<Map<String, dynamic>> syncOperations({
    required String deviceId,
    required String operatorId,
    required List<Map<String, dynamic>> operations,
  }) async {
    final res = await http.post(
      Uri.parse('$baseUrl/offline/sync'),
      headers: {'Content-Type': 'application/json'},
      body: jsonEncode({
        'device_id': deviceId,
        'operator_id': operatorId,
        'operations': operations,
      }),
    );

    if (res.statusCode != 200) {
      throw ApiException(res.statusCode, res.body);
    }

    return jsonDecode(res.body) as Map<String, dynamic>;
  }

  /// 获取服务端最新状态（增量同步）
  Future<Map<String, dynamic>> fetchSyncState({
    required String operatorId,
    required String operatorType,
    String? lastSyncedAt,
  }) async {
    final res = await http.post(
      Uri.parse('$baseUrl/offline/sync/state'),
      headers: {'Content-Type': 'application/json'},
      body: jsonEncode({
        'operator_id': operatorId,
        'operator_type': operatorType,
        if (lastSyncedAt != null) 'last_synced_at': lastSyncedAt,
      }),
    );

    if (res.statusCode != 200) {
      throw ApiException(res.statusCode, res.body);
    }

    return jsonDecode(res.body) as Map<String, dynamic>;
  }

  /// 强制到场（冲突时以本地为准）
  Future<void> forceArrive(
    String waybillId,
    Map<String, dynamic> payload,
  ) async {
    // 以带 force 参数的 arrive 请求强制覆盖
    final res = await http.post(
      Uri.parse('$baseUrl/waybills/$waybillId/arrive'),
      headers: {'Content-Type': 'application/json'},
      body: jsonEncode({
        ...payload,
        'force': true,
      }),
    );
    if (res.statusCode != 200) {
      throw ApiException(res.statusCode, res.body);
    }
  }

  /// 强制入队（冲突时以本地为准）
  Future<void> forceQueueJoin(
    String waybillId,
    Map<String, dynamic> payload,
  ) async {
    final res = await http.post(
      Uri.parse('$baseUrl/queue/waybills/$waybillId/join'),
      headers: {'Content-Type': 'application/json'},
      body: jsonEncode(payload),
    );
    if (res.statusCode != 200) {
      throw ApiException(res.statusCode, res.body);
    }
  }
}

class ApiException implements Exception {
  final int statusCode;
  final String body;

  ApiException(this.statusCode, this.body);

  @override
  String toString() => 'ApiException($statusCode): $body';
}
