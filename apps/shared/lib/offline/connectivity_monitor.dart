import 'dart:async';
import 'dart:io';
import 'package:connectivity_plus/connectivity_plus.dart';
import 'package:uuid/uuid.dart';

/// 网络状态监听器
///
/// 同时监听网络连通性和类型变化。
/// 提供 isOnline 状态和 onStatusChanged 流。
class ConnectivityMonitor {
  final Connectivity _connectivity = Connectivity();
  final String _deviceId;
  StreamSubscription<List<ConnectivityResult>>? _subscription;

  bool _isOnline = true;
  Timer? _pingTimer;

  /// 网络状态变化流
  final StreamController<bool> _statusController =
      StreamController<bool>.broadcast();

  Stream<bool> get onStatusChanged => _statusController.stream;

  /// 当前是否在线
  bool get isOnline => _isOnline;

  /// 设备唯一标识
  String get deviceId => _deviceId;

  ConnectivityMonitor() : _deviceId = const Uuid().v4();

  /// 启动监听
  Future<void> start() async {
    // 初始检测
    _isOnline = await _checkConnectivity();

    // 监听系统网络变化
    _subscription = _connectivity.onConnectivityChanged.listen((results) {
      final online = results.any((r) => r != ConnectivityResult.none);
      _updateStatus(online);
    });

    // 定时 Ping 检测真实连通性（防止系统状态不准）
    _pingTimer = Timer.periodic(const Duration(seconds: 15), (_) async {
      final online = await _checkRealConnectivity();
      _updateStatus(online);
    });
  }

  /// 停止监听
  Future<void> stop() async {
    await _subscription?.cancel();
    await _pingTimer?.cancel();
    await _statusController.close();
  }

  void _updateStatus(bool online) {
    if (_isOnline != online) {
      _isOnline = online;
      _statusController.add(online);
    }
  }

  /// 检查系统网络状态
  Future<bool> _checkConnectivity() async {
    try {
      final results = await _connectivity.checkConnectivity();
      return results.any((r) => r != ConnectivityResult.none);
    } catch (_) {
      return true; // 检查失败时默认在线，避免误判
    }
  }

  /// 通过 TCP 连接检测真实连通性
  Future<bool> _checkRealConnectivity() async {
    try {
      final result = await InternetAddress.lookup('api.mnr.gov.cn')
          .timeout(const Duration(seconds: 3));
      return result.isNotEmpty && result[0].rawAddress.isNotEmpty;
    } catch (_) {
      // Ping 失败不立刻判定离线，避免短时波动
      return _isOnline;
    }
  }

  /// 释放资源
  void dispose() {
    stop();
  }
}
