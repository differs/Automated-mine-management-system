/// 车牌识别 - 共享包
///
/// 提供端侧车牌识别能力（无需联网）：
///   1. PlateRecognizer - 从 OCR 文本提取中国车牌（正则+置信度评估）
///   2. PlateScanner - 封装 Google MLKit OCR（拍照/相册/文件）
///   3. PlateScannerPage - 完整的扫描 UI 页面
///   4. PlateScanButton - 可嵌入任意页面的扫描按钮
///
/// 使用方式:
///   ```dart
///   // 打开扫描页面
///   final plate = await Navigator.push<String>(
///     context,
///     MaterialPageRoute(builder: (_) => const PlateScannerPage()),
///   );
///
///   // 直接扫描
///   final scanner = PlateScanner();
///   final result = await scanner.scanFromCamera();
///   ```
library plate;

export 'plate_recognizer.dart';
export 'plate_scanner.dart';
export 'plate_scanner_widget.dart';
