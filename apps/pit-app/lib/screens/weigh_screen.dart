import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import '../services/api_service.dart';
import 'dart:math' as math;

/// 地磅称重界面
///
/// 支持三种称重方式:
///   1. 自动读取（串口/蓝牙地磅 → 自动填入重量）
///   2. 扫码识别（扫描二维码/条形码获取重量）
///   3. 手动输入
///
/// 使用方式:
///   ```dart
///   final result = await Navigator.push<WeighResult>(
///     context,
///     MaterialPageRoute(
///       builder: (_) => WeighScreen(
///         waybillId: waybill['id'],
///         waybillSerial: waybill['serial_no'],
///         vehicleId: waybill['vehicle_id'],
///         pitId: pitId,
///         operatorId: operatorId,
///       ),
///     ),
///   );
///   ```
class WeighScreen extends StatefulWidget {
  final String waybillId;
  final String waybillSerial;
  final String? vehicleId;
  final String pitId;
  final String operatorId;

  const WeighScreen({
    super.key,
    required this.waybillId,
    required this.waybillSerial,
    this.vehicleId,
    required this.pitId,
    required this.operatorId,
  });

  @override
  State<WeighScreen> createState() => _WeighScreenState();
}

class _WeighScreenState extends State<WeighScreen>
    with SingleTickerProviderStateMixin {
  late TabController _tabController;

  // 重量数据
  final _grossController = TextEditingController();
  final _tareController = TextEditingController();
  final _manualGrossController = TextEditingController();
  final _manualTareController = TextEditingController();
  bool _isWeighing = false;
  DateTime? _weighStartTime;

  // 蓝牙模拟读取
  Timer? _simulateTimer;
  double _simulatedWeight = 0.0;
  bool _isSimulating = false;

  // 皮重历史
  List<double> _tareHistory = [];
  bool _loadingTare = false;

  @override
  void initState() {
    super.initState();
    _tabController = TabController(length: 2, vsync: this);
    _loadTareHistory();

    // 启动时记录开始时间（防作弊用）
    _weighStartTime = DateTime.now();
  }

  @override
  void dispose() {
    _tabController.dispose();
    _grossController.dispose();
    _tareController.dispose();
    _manualGrossController.dispose();
    _manualTareController.dispose();
    _simulateTimer?.cancel();
    super.dispose();
  }

  Future<void> _loadTareHistory() async {
    if (widget.vehicleId == null) return;
    setState(() => _loadingTare = true);
    try {
      final tares = await ApiService.getTareHistory(widget.vehicleId!);
      if (mounted) {
        setState(() {
          _tareHistory = tares.cast<double>();
          if (tares.isNotEmpty) {
            _tareController.text = tares.first.toStringAsFixed(1);
          }
        });
      }
    } catch (_) {}
    if (mounted) setState(() => _loadingTare = false);
  }

  /// 自动读取（模拟串口/蓝牙地磅读数）
  void _startAutoRead() {
    setState(() {
      _isSimulating = true;
      _simulatedWeight = 0.0;
      _weighStartTime = DateTime.now();
    });

    // 模拟地磅逐步稳定过程
    int step = 0;
    const targetWeight = 45.0; // 模拟45吨
    _simulateTimer = Timer.periodic(const Duration(milliseconds: 800), (timer) {
      step++;
      final progress = math.min(step / 15.0, 1.0);
      // 模拟从零逐渐升到目标值，带微小波动
      final noise = (math.Random().nextDouble() - 0.5) * 0.3;
      _simulatedWeight = targetWeight * progress + noise;

      if (mounted) {
        setState(() {
          _grossController.text = _simulatedWeight.toStringAsFixed(1);
        });
      }

      // 连续稳定5次后自动停止
      if (progress >= 1.0 && step > 18) {
        timer.cancel();
        if (mounted) {
          setState(() => _isSimulating = false);
          _showStableReadings(_simulatedWeight);
        }
      }
    });
  }

  void _showStableReadings(double weight) {
    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('⚖️ 地磅读数稳定'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(
              '${weight.toStringAsFixed(1)} 吨',
              style: const TextStyle(
                fontSize: 48,
                fontWeight: FontWeight.bold,
                color: Colors.green,
              ),
            ),
            const SizedBox(height: 8),
            const Text('已自动填入毛重，请确认'),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx),
            child: const Text('重新读取'),
          ),
          FilledButton(
            onPressed: () {
              Navigator.pop(ctx);
              _submitWeigh();
            },
            child: const Text('确认称重'),
          ),
        ],
      ),
    );
  }

  /// 扫码识别重量（解析地磅屏幕上的数字）
  Future<void> _scanWeight() async {
    // 简单实现：显示一个输入框让操作员扫码后粘贴
    final scanned = await showDialog<String>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('扫码称重'),
        content: TextField(
          autofocus: true,
          decoration: const InputDecoration(
            hintText: '扫码或粘贴重量数据',
            border: OutlineInputBorder(),
          ),
          inputFormatters: [
            FilteringTextInputFormatter.allow(RegExp(r'[\d.\sA-Za-z,;:]')),
          ],
          onSubmitted: (v) => Navigator.pop(ctx, v),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx),
            child: const Text('取消'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, null),
            child: const Text('确定'),
          ),
        ],
      ),
    );

    if (scanned != null && scanned.isNotEmpty) {
      // 尝试从扫码结果中提取重量
      final weightMatch = RegExp(r'(\d+\.?\d*)\s*[tT吨]?').firstMatch(scanned);
      if (weightMatch != null) {
        final weight = double.tryParse(weightMatch.group(1)!);
        if (weight != null && mounted) {
          setState(() {
            _grossController.text = weight.toStringAsFixed(1);
          });
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(content: Text('识别到重量: $weight 吨')),
          );
        }
      }
    }
  }

  /// 提交称重
  Future<void> _submitWeigh() async {
    final grossText = _grossController.text.trim();
    final tareText = _tareController.text.trim();

    if (grossText.isEmpty) {
      _showError('请填写毛重');
      return;
    }

    final gross = double.tryParse(grossText);
    if (gross == null || gross <= 0) {
      _showError('毛重格式不正确');
      return;
    }

    double? tare;
    if (tareText.isNotEmpty) {
      tare = double.tryParse(tareText);
      if (tare == null || tare < 0) {
        _showError('皮重格式不正确');
        return;
      }
    }

    setState(() => _isWeighing = true);

    try {
      // 计算净重
      final net = tare != null ? gross - tare : gross;

      final duration = _weighStartTime != null
          ? DateTime.now().difference(_weighStartTime!).inSeconds
          : 0;

      // 调用蓝牙称重API（带原始数据）
      await ApiService.bluetoothWeigh(
        waybillId: widget.waybillId,
        operatorId: widget.operatorId,
        deviceId: 'auto', // 实际环境传真实设备ID
        grossWeightTon: gross,
        tareWeightTon: tare,
        rawData: 'manual_input;gross=$gross;tare=$tare',
        readingDurationSec: duration,
      );

      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text('✅ 称重完成: 净重 ${net.toStringAsFixed(1)} 吨'),
            backgroundColor: Colors.green,
          ),
        );
        Navigator.pop(context, WeighResult(
          grossWeightTon: gross,
          tareWeightTon: tare,
          netWeightTon: net,
          completedAt: DateTime.now(),
        ));
      }
    } catch (e) {
      _showError('称重失败: $e');
    } finally {
      if (mounted) setState(() => _isWeighing = false);
    }
  }

  void _showError(String msg) {
    if (!mounted) return;
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text(msg), backgroundColor: Colors.red[700]),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: Text('⚖️ 称重 - ${widget.waybillSerial}'),
        bottom: TabBar(
          controller: _tabController,
          tabs: const [
            Tab(text: '自动读取', icon: Icon(Icons.bluetooth)),
            Tab(text: '手动输入', icon: Icon(Icons.keyboard)),
          ],
        ),
      ),
      body: TabBarView(
        controller: _tabController,
        children: [
          _buildAutoTab(),
          _buildManualTab(),
        ],
      ),
    );
  }

  Widget _buildAutoTab() {
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        // 地磅读数区域
        Card(
          child: Padding(
            padding: const EdgeInsets.all(24),
            child: Column(
              children: [
                const Text('地磅读数', style: TextStyle(fontSize: 16, color: Colors.grey)),
                const SizedBox(height: 8),
                Text(
                  _grossController.text.isEmpty
                      ? '----.-'
                      : '${_grossController.text}',
                  style: TextStyle(
                    fontSize: 56,
                    fontWeight: FontWeight.bold,
                    color: _isSimulating ? Colors.orange : Colors.black,
                  ),
                ),
                const Text('吨', style: TextStyle(fontSize: 18, color: Colors.grey)),
                if (_isSimulating) ...[
                  const SizedBox(height: 8),
                  const Text('读取中...请等待稳定',
                      style: TextStyle(color: Colors.orange)),
                  const SizedBox(height: 8),
                  const LinearProgressIndicator(),
                ],
              ],
            ),
          ),
        ),
        const SizedBox(height: 16),

        // 皮重信息
        Card(
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    const Text('皮重', style: TextStyle(fontWeight: FontWeight.bold)),
                    const Spacer(),
                    if (_loadingTare)
                      const SizedBox(
                        width: 16, height: 16,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      )
                    else if (_tareHistory.isNotEmpty)
                      Text(
                        '上次皮重: ${_tareHistory.first.toStringAsFixed(1)}t',
                        style: TextStyle(fontSize: 12, color: Colors.grey[600]),
                      ),
                  ],
                ),
                const SizedBox(height: 8),
                TextField(
                  controller: _tareController,
                  decoration: const InputDecoration(
                    hintText: '输入皮重（空车重量）',
                    border: OutlineInputBorder(),
                    suffixText: '吨',
                  ),
                  keyboardType: const TextInputType.numberWithOptions(decimal: true),
                ),
              ],
            ),
          ),
        ),

        const SizedBox(height: 16),

        // 自动读取按钮
        SizedBox(
          width: double.infinity,
          height: 56,
          child: FilledButton.icon(
            onPressed: _isSimulating ? null : _startAutoRead,
            icon: _isSimulating
                ? const SizedBox(
                    width: 20, height: 20,
                    child: CircularProgressIndicator(strokeWidth: 2, color: Colors.white),
                  )
                : const Icon(Icons.bluetooth_searching, size: 24),
            label: Text(
              _isSimulating ? '读取中...' : '📡 连接地磅读取',
              style: const TextStyle(fontSize: 16),
            ),
          ),
        ),
        const SizedBox(height: 8),

        // 扫码识别
        SizedBox(
          width: double.infinity,
          height: 48,
          child: OutlinedButton.icon(
            onPressed: _scanWeight,
            icon: const Icon(Icons.qr_code_scanner),
            label: const Text('📷 扫码识别重量'),
          ),
        ),

        if (_grossController.text.isNotEmpty) ...[
          const SizedBox(height: 24),
          SizedBox(
            width: double.infinity,
            height: 56,
            child: FilledButton.icon(
              onPressed: _isWeighing ? null : _submitWeigh,
              icon: _isWeighing
                  ? const SizedBox(
                      width: 20, height: 20,
                      child: CircularProgressIndicator(strokeWidth: 2, color: Colors.white),
                    )
                  : const Icon(Icons.check_circle),
              label: Text(
                _isWeighing ? '提交中...' : '✅ 确认称重并完单',
                style: const TextStyle(fontSize: 16),
              ),
              style: FilledButton.styleFrom(
                backgroundColor: Colors.green,
              ),
            ),
          ),
        ],
      ],
    );
  }

  Widget _buildManualTab() {
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        const Card(
          color: Colors.blue,
          child: Padding(
            padding: EdgeInsets.all(12),
            child: Row(
              children: [
                Icon(Icons.info, color: Colors.white),
                SizedBox(width: 8),
                Expanded(
                  child: Text(
                    '手动输入仅用于地磅故障等异常情况',
                    style: TextStyle(color: Colors.white, fontSize: 13),
                  ),
                ),
              ],
            ),
          ),
        ),
        const SizedBox(height: 16),

        TextField(
          controller: _manualGrossController,
          decoration: const InputDecoration(
            labelText: '毛重（载重后总重）',
            border: OutlineInputBorder(),
            suffixText: '吨',
            prefixIcon: Icon(Icons.monitor_weight),
          ),
          keyboardType: const TextInputType.numberWithOptions(decimal: true),
          style: const TextStyle(fontSize: 20),
        ),
        const SizedBox(height: 16),

        TextField(
          controller: _manualTareController,
          decoration: const InputDecoration(
            labelText: '皮重（空车重量，可选）',
            border: OutlineInputBorder(),
            suffixText: '吨',
            prefixIcon: Icon(Icons.directions_car),
          ),
          keyboardType: const TextInputType.numberWithOptions(decimal: true),
        ),
        const SizedBox(height: 24),

        // 快捷键：填入上次皮重
        if (_tareHistory.isNotEmpty)
          Padding(
            padding: const EdgeInsets.only(bottom: 16),
            child: InkWell(
              onTap: () {
                _manualTareController.text =
                    _tareHistory.first.toStringAsFixed(1);
                setState(() {});
              },
              child: Container(
                padding: const EdgeInsets.all(8),
                decoration: BoxDecoration(
                  color: Colors.grey[100],
                  borderRadius: BorderRadius.circular(8),
                ),
                child: Row(
                  children: [
                    const Icon(Icons.history, size: 16, color: Colors.grey),
                    const SizedBox(width: 8),
                    Text(
                      '使用上次皮重: ${_tareHistory.first.toStringAsFixed(1)} 吨',
                      style: TextStyle(fontSize: 13, color: Colors.grey[700]),
                    ),
                  ],
                ),
              ),
            ),
          ),

        SizedBox(
          width: double.infinity,
          height: 56,
          child: FilledButton.icon(
            onPressed: () {
              // 将手动输入复制到自动读取的字段中
              _grossController.text = _manualGrossController.text;
              if (_manualTareController.text.isNotEmpty) {
                _tareController.text = _manualTareController.text;
              }
              _submitWeigh();
            },
            icon: const Icon(Icons.check),
            label: const Text('提交称重'),
            style: FilledButton.styleFrom(
              backgroundColor: Colors.green,
            ),
          ),
        ),
      ],
    );
  }
}

/// 称重结果
class WeighResult {
  final double grossWeightTon;
  final double? tareWeightTon;
  final double netWeightTon;
  final DateTime completedAt;

  WeighResult({
    required this.grossWeightTon,
    this.tareWeightTon,
    required this.netWeightTon,
    required this.completedAt,
  });
}
