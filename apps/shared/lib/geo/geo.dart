/// 围栏模块 - 共享包
///
/// 提供电子围栏能力:
///   1. LocationService - 定时上报位置+围栏事件回调
///   2. FenceEventBanner - 围栏事件提示Widget
///
/// 使用方式:
///   ```dart
///   final locService = LocationService(
///     baseUrl: apiBaseUrl,
///     driverId: driverId,
///   );
///   locService.onFenceEvents = (events) {
///     for (final e in events) {
///       print('${e.eventType}: ${e.fenceName}');
///     }
///   };
///   locService.start();
///   ```
library geo;

export 'location_service.dart';
