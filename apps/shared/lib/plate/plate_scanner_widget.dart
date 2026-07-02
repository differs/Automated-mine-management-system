import 'dart:io';
import 'dart:typed_data';
import 'dart:ui' as ui;
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:google_mlkit_text_recognition/google_mlkit_text_recognition.dart';
import 'package:image_picker/image_picker.dart';
import 'plate_recognizer.dart';
import 'plate_scanner.dart';

/// 车牌扫描界面
///
/// 提供拍照识别和手动输入两种模式。
///
/// 使用方式:
///   ```dart
///   final result = await Navigator.push<String>(
///     context,
///     MaterialPageRoute(builder: (_) => const PlateScannerPage()),
///   );
///   if (result != null) {
///     print('识别到车牌: $result');
///   }
///   ```
class PlateScannerPage extends StatefulWidget {
  /// 预填车牌号（手动输入模式）
  final String? initialPlate;

  /// 是否允许手动输入
  final bool allowManualInput;

  /// 标题
  final String title;

  /// 识别成功回调（返回车牌号）
  final void Function(String plateNumber)? onPlateScanned;

  const PlateScannerPage({
    super.key,
    this.initialPlate,
    this.allowManualInput = true,
    this.title = '扫描车牌',
    this.onPlateScanned,
  });

  @override
  State<PlateScannerPage> createState() => _PlateScannerPageState();
}

class _PlateScannerPageState extends State<PlateScannerPage> {
  final PlateScanner _scanner = PlateScanner();
  final TextEditingController _manualController = TextEditingController();
  bool _isScanning = false;
  String? _lastError;

  @override
  void initState() {
    super.initState();
    _manualController.text = widget.initialPlate ?? '';
  }

  @override
  void dispose() {
    _scanner.dispose();
    _manualController.dispose();
    super.dispose();
  }

  Future<void> _scanFromCamera() async {
    setState(() {
      _isScanning = true;
      _lastError = null;
    });

    try {
      final result = await _scanner.scanFromCamera();
      await _handleResult(result);
    } catch (e) {
      _showError('扫描失败: $e');
    } finally {
      if (mounted) setState(() => _isScanning = false);
    }
  }

  Future<void> _scanFromGallery() async {
    setState(() {
      _isScanning = true;
      _lastError = null;
    });

    try {
      final result = await _scanner.scanFromGallery();
      await _handleResult(result);
    } catch (e) {
      _showError('识别失败: $e');
    } finally {
      if (mounted) setState(() => _isScanning = false);
    }
  }

  Future<void> _handleResult(PlateScanResult? result) async {
    if (result == null) return;

    if (!mounted) return;

    if (result.success && result.plateNumber != null) {
      // 识别成功 → 弹出确认对话框
      final confirmed = await _showPlateConfirmDialog(result.plateNumber!,
          result.confidence ?? 0.0, result.plateBlock);

      if (confirmed == true && mounted) {
        widget.onPlateScanned?.call(result.plateNumber!);
        Navigator.pop(context, result.plateNumber!);
      }
    } else {
      // 识别失败 → 显示错误并提供重试/手动输入
      if (mounted) {
        _showScanResultDialog(
          success: false,
          plateNumber: null,
          errorMessage: result?.errorMessage,
          rawText: result?.allDetectedText,
        );
      }
    }
  }

  Future<bool?> _showPlateConfirmDialog(
    String plateNumber,
    double confidence,
    TextBlock? block,
  ) {
    return showDialog<bool>(
      context: context,
      barrierDismissible: false,
      builder: (ctx) => AlertDialog(
        title: const Text('确认车牌'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            // 车牌号大字显示
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 16),
              decoration: BoxDecoration(
                color: Colors.blue[50],
                borderRadius: BorderRadius.circular(12),
                border: Border.all(color: Colors.blue[200]!),
              ),
              child: Text(
                plateNumber,
                style: const TextStyle(
                  fontSize: 36,
                  fontWeight: FontWeight.bold,
                  letterSpacing: 4,
                  fontFamily: 'monospace',
                ),
              ),
            ),
            const SizedBox(height: 12),
            // 置信度
            Row(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                Icon(Icons.check_circle,
                    size: 16,
                    color: confidence > 0.8 ? Colors.green : Colors.orange),
                const SizedBox(width: 4),
                Text(
                  '置信度 ${(confidence * 100).toStringAsFixed(0)}%',
                  style: TextStyle(
                    fontSize: 14,
                    color: confidence > 0.8 ? Colors.green[700] : Colors.orange[700],
                  ),
                ),
              ],
            ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: const Text('重新扫描'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, true),
            child: const Text('确认'),
          ),
        ],
      ),
    );
  }

  Future<void> _showScanResultDialog({
    required bool success,
    String? plateNumber,
    String? errorMessage,
    String? rawText,
  }) async {
    if (!mounted) return;

    await showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(success ? '识别成功' : '识别失败'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            if (plateNumber != null)
              Text('车牌号: $plateNumber', style: const TextStyle(fontSize: 18)),
            if (errorMessage != null) Text(errorMessage),
            if (rawText != null && rawText.length > 10) ...[
              const SizedBox(height: 8),
              const Text('识别到的文字:', style: TextStyle(fontWeight: FontWeight.bold)),
              const SizedBox(height: 4),
              Container(
                padding: const EdgeInsets.all(8),
                decoration: BoxDecoration(
                  color: Colors.grey[100],
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text(rawText, style: const TextStyle(fontSize: 12)),
              ),
            ],
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx),
            child: const Text('关闭'),
          ),
          if (!success)
            FilledButton(
              onPressed: () {
                Navigator.pop(ctx);
                _scanFromCamera();
              },
              child: const Text('重试'),
            ),
        ],
      ),
    );
  }

  void _showError(String message) {
    if (!mounted) return;
    setState(() => _lastError = message);
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text(message), backgroundColor: Colors.red[700]),
    );
  }

  void _submitManualPlate() {
    final plate = _manualController.text.trim().toUpperCase();
    if (plate.isEmpty) {
      _showError('请输入车牌号');
      return;
    }
    if (!PlateRecognizer.isValidPlate(plate)) {
      _showError('车牌号格式不正确');
      return;
    }
    widget.onPlateScanned?.call(plate);
    Navigator.pop(context, plate);
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: Text(widget.title),
        actions: [
          if (widget.allowManualInput)
            TextButton.icon(
              onPressed: () => _showManualInputDialog(),
              icon: const Icon(Icons.keyboard),
              label: const Text('手动输入'),
            ),
        ],
      ),
      body: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          children: [
            // 提示区域
            Card(
              child: Padding(
                padding: const EdgeInsets.all(16),
                child: Row(
                  children: [
                    Icon(Icons.info_outline, color: Colors.blue[700]),
                    const SizedBox(width: 12),
                    const Expanded(
                      child: Text(
                        '将车牌对准取景框，确保车牌清晰、光线充足',
                        style: TextStyle(fontSize: 14),
                      ),
                    ),
                  ],
                ),
              ),
            ),
            const SizedBox(height: 24),

            // 车辆图标占位
            Expanded(
              child: Center(
                child: Column(
                  mainAxisAlignment: MainAxisAlignment.center,
                  children: [
                    Icon(
                      Icons.directions_car_filled,
                      size: 120,
                      color: Colors.grey[300],
                    ),
                    const SizedBox(height: 16),
                    Text(
                      _lastError ?? '点击下方按钮开始扫描',
                      style: TextStyle(
                        fontSize: 16,
                        color: _lastError != null ? Colors.red[700] : Colors.grey[600],
                      ),
                      textAlign: TextAlign.center,
                    ),
                  ],
                ),
              ),
            ),

            // 拍照按钮
            SafeArea(
              child: Column(
                children: [
                  SizedBox(
                    width: double.infinity,
                    height: 56,
                    child: FilledButton.icon(
                      onPressed: _isScanning ? null : _scanFromCamera,
                      icon: _isScanning
                          ? const SizedBox(
                              width: 20,
                              height: 20,
                              child: CircularProgressIndicator(
                                strokeWidth: 2,
                                color: Colors.white,
                              ),
                            )
                          : const Icon(Icons.camera_alt, size: 28),
                      label: Text(
                        _isScanning ? '识别中...' : '拍照识别',
                        style: const TextStyle(fontSize: 18),
                      ),
                    ),
                  ),
                  const SizedBox(height: 12),
                  SizedBox(
                    width: double.infinity,
                    height: 48,
                    child: OutlinedButton.icon(
                      onPressed: _isScanning ? null : _scanFromGallery,
                      icon: const Icon(Icons.photo_library),
                      label: const Text('从相册选择'),
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }

  void _showManualInputDialog() {
    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('手动输入车牌号'),
        content: TextField(
          controller: _manualController,
          textCapitalization: TextCapitalization.characters,
          maxLength: 8,
          decoration: const InputDecoration(
            hintText: '例如: 京A12345',
            border: OutlineInputBorder(),
            counterText: '',
          ),
          style: const TextStyle(
            fontSize: 24,
            letterSpacing: 4,
            fontFamily: 'monospace',
          ),
          inputFormatters: [
            FilteringTextInputFormatter.allow(RegExp(r'[A-Za-z0-9\u4e00-\u9fff]')),
            UpperCaseTextFormatter(),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx),
            child: const Text('取消'),
          ),
          FilledButton(
            onPressed: () {
              Navigator.pop(ctx);
              _submitManualPlate();
            },
            child: const Text('确认'),
          ),
        ],
      ),
    );
  }
}

/// 自动转大写 TextInputFormatter
class UpperCaseTextFormatter extends TextInputFormatter {
  @override
  TextEditingValue formatEditUpdate(
    TextEditingValue oldValue,
    TextEditingValue newValue,
  ) {
    return TextEditingValue(
      text: newValue.text.toUpperCase(),
      selection: newValue.selection,
    );
  }
}

/// 车牌扫描确认按钮（可嵌入任意页面）
///
/// ```dart
/// PlateScanButton(
///   onPlateScanned: (plate) {
///     print('扫描到车牌: $plate');
///     // 自动完成到场
///   },
/// )
/// ```
class PlateScanButton extends StatelessWidget {
  final void Function(String plateNumber) onPlateScanned;
  final String label;
  final IconData icon;

  const PlateScanButton({
    super.key,
    required this.onPlateScanned,
    this.label = '扫车牌到场',
    this.icon = Icons.qr_code_scanner,
  });

  @override
  Widget build(BuildContext context) {
    return FilledButton.tonalIcon(
      onPressed: () async {
        final result = await Navigator.push<String>(
          context,
          MaterialPageRoute(
            builder: (_) => PlateScannerPage(onPlateScanned: onPlateScanned),
          ),
        );
        if (result != null) {
          onPlateScanned(result);
        }
      },
      icon: Icon(icon),
      label: Text(label),
    );
  }
}
