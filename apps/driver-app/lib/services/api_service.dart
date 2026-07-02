import 'dart:convert';
import 'package:http/http.dart' as http;
import 'package:shared_preferences/shared_preferences.dart';

class ApiService {
  static const String baseUrl = 'http://10.0.2.2:3000/api/v1';

  static Future<String?> getToken() async {
    final prefs = await SharedPreferences.getInstance();
    return prefs.getString('driver_token');
  }

  static Future<void> setToken(String token) async {
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString('driver_token', token);
  }

  static Future<void> setDriverId(String id) async {
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString('driver_id', id);
  }

  static Future<String?> getDriverId() async {
    final prefs = await SharedPreferences.getInstance();
    return prefs.getString('driver_id');
  }

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

  // Auth
  static Future<Map<String, dynamic>> login(String username, String password) {
    return post('/auth/login', {
      'username': username,
      'password': password,
    });
  }

  // Drivers
  static Future<List<dynamic>> getDrivers({String? keyword}) {
    final q = keyword != null ? '?keyword=${Uri.encodeComponent(keyword)}' : '';
    return getList('/drivers$q');
  }

  static Future<Map<String, dynamic>> createDriver(Map<String, dynamic> data) {
    return post('/drivers', data);
  }

  // Waybills
  static Future<List<dynamic>> getWaybills({String? status, String? driverId}) {
    final params = <String>[];
    if (status != null) params.add('status=$status');
    if (driverId != null) params.add('driver_id=$driverId');
    final q = params.isNotEmpty ? '?${params.join('&')}' : '';
    return getList('/waybills$q');
  }

  static Future<Map<String, dynamic>> arriveWaybill(
      String waybillId, String source) {
    return post('/waybills/$waybillId/arrive', {
      'arrival_source': source,
    });
  }

  static Future<Map<String, dynamic>> cancelWaybill(
      String waybillId, String cancelledBy, String reason) {
    return post('/waybills/$waybillId/cancel', {
      'cancelled_by': cancelledBy,
      'reason': reason,
    });
  }

  // Pits
  static Future<List<dynamic>> getPits() {
    return getList('/pits');
  }

  // Queue
  static Future<List<dynamic>> getPitQueue(String pitId) {
    return getList('/queue/pits/$pitId');
  }

  // Plate Recognition
  static Future<Map<String, dynamic>> arriveByPlate({
    required String waybillId,
    required String driverId,
    required String plateNumber,
    double? confidence,
  }) {
    return post('/waybills/arrive-by-plate', {
      'driver_id': driverId,
      'plate_number': plateNumber,
      if (confidence != null) 'confidence': confidence,
    });
  }
}
