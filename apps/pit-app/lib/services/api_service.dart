import 'dart:convert';
import 'package:http/http.dart' as http;

class ApiService {
  static const String baseUrl = 'http://10.0.2.2:3000/api/v1';

  static Future<Map<String, dynamic>> get(String path) async {
    final res = await http.get(
      Uri.parse('$baseUrl$path'),
      headers: {'Content-Type': 'application/json'},
    );
    if (res.statusCode != 200) throw Exception(res.body);
    return jsonDecode(res.body);
  }

  static Future<List<dynamic>> getList(String path) async {
    final res = await http.get(
      Uri.parse('$baseUrl$path'),
      headers: {'Content-Type': 'application/json'},
    );
    if (res.statusCode != 200) throw Exception(res.body);
    return jsonDecode(res.body);
  }

  static Future<Map<String, dynamic>> post(
      String path, Map<String, dynamic> body) async {
    final res = await http.post(
      Uri.parse('$baseUrl$path'),
      headers: {'Content-Type': 'application/json'},
      body: jsonEncode(body),
    );
    if (res.statusCode != 200) throw Exception(res.body);
    return jsonDecode(res.body);
  }

  static Future<List<dynamic>> getPits() => getList('/pits');
  static Future<List<dynamic>> getDrivers() => getList('/drivers');
  static Future<List<dynamic>> getPitQueue(String pitId) =>
      getList('/queue/pits/$pitId');
  static Future<List<dynamic>> getWaybills({String? pitId, String? status}) {
    final params = <String>[];
    if (pitId != null) params.add('pit_id=$pitId');
    if (status != null) params.add('status=$status');
    final q = params.isNotEmpty ? '?${params.join('&')}' : '';
    return getList('/waybills$q');
  }

  static Future<Map<String, dynamic>> callNext(
          String waybillId, String operatorId) =>
      post('/queue/waybills/$waybillId/call-next', {
        'operator_id': operatorId,
      });

  static Future<Map<String, dynamic>> startLoading(
          String waybillId, String operatorId) =>
      post('/loading/waybills/$waybillId/start', {
        'operator_id': operatorId,
      });

  static Future<Map<String, dynamic>> finishLoading(
          String waybillId, String operatorId) =>
      post('/loading/waybills/$waybillId/finish', {
        'operator_id': operatorId,
      });

  static Future<Map<String, dynamic>> weigh(
      String waybillId, String operatorId, double netWeight) {
    return post('/weighing/waybills/$waybillId', {
      'operator_id': operatorId,
      'gross_weight_ton': netWeight,
      'tare_weight_ton': 0,
      'net_weight_ton': netWeight,
      'source': 'manual',
    });
  }

  // ─── 地磅自动采集 ────────────────────────────────────────────────────

  /// 蓝牙称重
  static Future<Map<String, dynamic>> bluetoothWeigh({
    required String waybillId,
    required String operatorId,
    required String deviceId,
    required double grossWeightTon,
    double? tareWeightTon,
    String? rawData,
    int readingDurationSec = 0,
  }) {
    return post('/scale/bluetooth/weigh/$waybillId', {
      'device_id': deviceId,
      'operator_id': operatorId,
      'waybill_id': waybillId,
      'gross_weight_ton': grossWeightTon,
      if (tareWeightTon != null) 'tare_weight_ton': tareWeightTon,
      'raw_data': rawData ?? 'manual_input',
      'reading_duration_sec': readingDurationSec,
    });
  }

  /// 获取皮重历史
  static Future<List<dynamic>> getTareHistory(String vehicleId) {
    return getList('/scale/tare-history/$vehicleId');
  }
}
