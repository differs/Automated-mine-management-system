import 'dart:io';
import 'package:google_mlkit_text_recognition/google_mlkit_text_recognition.dart';
import 'package:image_picker/image_picker.dart';
import 'plate_recognizer.dart';

/// 车牌扫描器
///
/// 封装 Google MLKit OCR，提供：
///   1. 拍照识别
///   2. 从相册选择识别
///   3. 实时摄像头预览识别（需额外集成 camera 插件）
///
/// 使用方式:
///   ```dart
///   final scanner = PlateScanner();
///
///   // 拍照识别
///   final result = await scanner.scanFromCamera();
///
///   // 或者从相册
///   final result = await scanner.scanFromGallery();
///
///   if (result != null) {
///     print('识别到车牌: ${result.plateNumber}');
///   }
///   ```
class PlateScanner {
  final ImagePicker _picker = ImagePicker();
  final TextRecognizer _recognizer =
      TextRecognizer(script: TextRecognitionScript.chinese);

  /// 从拍照识别车牌
  Future<PlateScanResult?> scanFromCamera() async {
    final xFile = await _picker.pickImage(
      source: ImageSource.camera,
      preferredCameraDevice: CameraDevice.rear,
      maxWidth: 1920,
      maxHeight: 1080,
      imageQuality: 85,
    );

    if (xFile == null) return null;

    return _processImage(File(xFile.path));
  }

  /// 从相册选择图片识别车牌
  Future<PlateScanResult?> scanFromGallery() async {
    final xFile = await _picker.pickImage(
      source: ImageSource.gallery,
      maxWidth: 1920,
      maxHeight: 1080,
      imageQuality: 85,
    );

    if (xFile == null) return null;

    return _processImage(File(xFile.path));
  }

  /// 从文件路径识别车牌
  Future<PlateScanResult?> scanFromFile(String filePath) async {
    return _processImage(File(filePath));
  }

  /// 从 bytes 识别车牌
  Future<PlateScanResult?> scanFromBytes(Uint8List bytes, {int? width, int? height}) async {
    final inputImage = InputImage.fromBytes(
      bytes: bytes,
      metadata: InputImageMetadata(
        size: Size(width?.toDouble() ?? 1920, height?.toDouble() ?? 1080),
        rotation: InputImageRotation.rotation0deg,
        format: InputImageFormat.bgra8888,
        bytesPerRow: (width ?? 1920) * 4,
      ),
    );
    return _processInputImage(inputImage);
  }

  /// 核心：处理图片并识别
  Future<PlateScanResult?> _processImage(File file) async {
    try {
      final inputImage = InputImage.fromFile(file);
      return await _processInputImage(inputImage);
    } catch (e) {
      return PlateScanResult(
        success: false,
        errorMessage: '图片处理失败: $e',
      );
    }
  }

  Future<PlateScanResult?> _processInputImage(InputImage inputImage) async {
    try {
      // 1. MLKit 文字识别
      final recognisedText = await _recognizer.processImage(inputImage);

      if (recognisedText.text.isEmpty) {
        return PlateScanResult(
          success: false,
          errorMessage: '未识别到文字，请确保车牌清晰可见',
        );
      }

      // 2. 从识别结果中提取车牌
      final plates = PlateRecognizer.extractPlates(recognisedText.text);

      if (plates.isEmpty) {
        // 返回所有识别到的文字（调试用）
        return PlateScanResult(
          success: false,
          errorMessage: '未识别到车牌号',
          rawText: recognisedText.text,
          allDetectedText: recognisedText.text,
        );
      }

      // 3. 取置信度最高的车牌
      final bestPlate = plates.first;

      // 4. 获取该车牌的详细识别信息
      final plateBlock = _findPlateBlock(recognisedText, bestPlate.plateNumber);

      return PlateScanResult(
        success: true,
        plateNumber: bestPlate.plateNumber,
        confidence: bestPlate.confidence,
        rawText: recognisedText.text,
        allDetectedText: recognisedText.text,
        plateBlock: plateBlock,
      );
    } catch (e) {
      return PlateScanResult(
        success: false,
        errorMessage: '识别失败: $e',
      );
    }
  }

  /// 找到车牌所在的文字块（用于画框）
  TextBlock? _findPlateBlock(RecognizedText text, String plateNumber) {
    for (final block in text.blocks) {
      if (block.text.contains(plateNumber)) {
        return block;
      }
      for (final line in block.lines) {
        if (line.text.contains(plateNumber)) {
          return block;
        }
      }
    }
    return null;
  }

  /// 释放资源
  void dispose() {
    _recognizer.close();
  }
}

/// 车牌扫描结果
class PlateScanResult {
  /// 是否成功识别到车牌
  final bool success;

  /// 识别的车牌号
  final String? plateNumber;

  /// 置信度（0.0 ~ 1.0）
  final double? confidence;

  /// 错误信息
  final String? errorMessage;

  /// OCR 原始识别文本
  final String? rawText;

  /// 所有检测到的文本
  final String? allDetectedText;

  /// 车牌所在文字块（用于 UI 展示）
  final TextBlock? plateBlock;

  const PlateScanResult({
    required this.success,
    this.plateNumber,
    this.confidence,
    this.errorMessage,
    this.rawText,
    this.allDetectedText,
    this.plateBlock,
  });
}
