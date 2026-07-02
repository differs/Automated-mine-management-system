import 'dart:async';
import 'dart:math' as math;
import 'package:flutter/material.dart';
import 'package:http/http.dart' as http;
import 'dart:convert';
import 'package:shared_preferences/shared_preferences.dart';

/// 位置上报服务
///
/// 定时获取设备位置 → 上报服务端 → 服务端判定围栏 → 返回事件
///
/// 使用方式:
///   ```dart
///   final location = LocationService(
///     baseUrl: 'http://10.0.2.2:3000/api/v1',
///     driverId: '...',
///   );
///   await location.start();   // 开始定时上报
///   location.dispose();       // 停止
///   ```
class LocationService {
  final String baseUrl;
  final String driverId;
  Timer? _timer;

  /// 上报间隔（秒），司机App每15秒上报一次
  final int intervalSeconds;

  /// 围栏事件回调
  void Function(List<FenceEvent> events)? onFenceEvents;

  /// 上次位置缓存（离线上报用）
  List<Map<String, dynamic>> _pendingPoints = [];

  LocationService({
    required this.baseUrl,
    required this.driverId,
    this.intervalSeconds = 15,
  });

  /// 启动定时上报
  void start() {
    // 立即上报一次
    _report();
    // 定时上报
    _timer = Timer.periodic(Duration(seconds: intervalSeconds), (_) {
      _report();
    });
  }

  /// 停止
  void dispose() {
    _timer?.cancel();
    // 上报剩余的离线点
    if (_pendingPoints.isNotEmpty) {
      _batchReport();
    }
  }

  /// 单次上报
  Future<void> _report() async {
    // 模拟位置（实际项目使用 platform channel 获取真实 GPS）
    // 这里生成以坑口为中心的模拟位置
    final lat = 39.9042 + (math.Random().nextDouble() - 0.5) * 0.01;
    final lng = 116.4074 + (math.Random().nextDouble() - 0.5) * 0.01;

    final point = {
      'driver_id': driverId,
      'lat': lat,
      'lng': lng,
      'accuracy': 10.0 + math.Random().nextDouble() * 5,
      'speed': math.Random().nextDouble() * 30,
      'reported_at': DateTime.now().toUtc().toIso8601String(),
    };

    try {
      final res = await http.post(
        Uri.parse('$baseUrl/fence/report'),
        headers: {'Content-Type': 'application/json'},
        body: jsonEncode(point),
      );

      if (res.statusCode == 200) {
        final data = jsonDecode(res.body) as Map<String, dynamic>;
        if (data['fence_events'] != null) {
          final events = (data['fence_events'] as List)
              .map((e) => FenceEvent.fromJson(e as Map<String, dynamic>))
              .toList();
          if (events.isNotEmpty) {
            onFenceEvents?.call(events);
          }
        }
      }
    } catch (_) {
      // 网络失败，缓存到本地队列
      _pendingPoints.add(point);
      if (_pendingPoints.length >= 10) {
        _batchReport();
      }
    }
  }

  /// 批量上报（离线缓存）
  Future<void> _batchReport() async {
    if (_pendingPoints.isEmpty) return;
    final batch = List<Map<String, dynamic>>.from(_pendingPoints);
    _pendingPoints.clear();

    try {
      await http.post(
        Uri.parse('$baseUrl/fence/report/batch'),
        headers: {'Content-Type': 'application/json'},
        body: jsonEncode({'driver_id': driverId, 'points': batch}),
      );
    } catch (_) {
      // 批量也失败，放回队列
      _pendingPoints.addAll(batch);
    }
  }
}

/// 围栏事件
class FenceEvent {
  final String fenceId;
  final String fenceName;
  final String eventType; // enter / exit
  final DateTime occurredAt;

  FenceEvent({
    required this.fenceId,
    required this.fenceName,
    required this.eventType,
    required this.occurredAt,
  });

  factory FenceEvent.fromJson(Map<String, dynamic> json) {
    return FenceEvent(
      fenceId: json['fence_id'] as String,
      fenceName: json['fence_name'] as String,
      eventType: json['event_type'] as String,
      occurredAt: DateTime.parse(json['occurred_at'] as String),
    );
  }
}

/// 围栏事件提示 Widget
///
/// 当司机进入/离开围栏时显示浮层提示
class FenceEventBanner extends StatelessWidget {
  final FenceEvent event;

  const FenceEventBanner({super.key, required this.event});

  @override
  Widget build(BuildContext context) {
    final isEnter = event.eventType == 'enter';
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
      decoration: BoxDecoration(
        color: isEnter ? Colors.green[50] : Colors.orange[50],
        borderRadius: BorderRadius.circular(8),
        border: Border.all(
          color: isEnter ? Colors.green[200]! : Colors.orange[200]!,
        ),
      ),
      child: Row(
        children: [
          Icon(
            isEnter ? Icons.login : Icons.logout,
            color: isEnter ? Colors.green : Colors.orange,
            size: 20,
          ),
          const SizedBox(width: 8),
          Expanded(
            child: Text(
              isEnter
                  ? '已进入 ${event.fenceName} 范围'
                  : '已离开 ${event.fenceName} 范围',
              style: TextStyle(
                fontSize: 14,
                color: isEnter ? Colors.green[800] : Colors.orange[800],
              ),
            ),
          ),
        ],
      ),
    );
  }
}
