import 'dart:async';
import 'package:flutter/material.dart';
import '../services/api_service.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'login_screen.dart';
import 'package:shared/plate/plate_scanner_widget.dart';

class HomeScreen extends StatefulWidget {
  final String driverId;
  final String driverName;

  const HomeScreen({
    super.key,
    required this.driverId,
    required this.driverName,
  });

  @override
  State<HomeScreen> createState() => _HomeScreenState();
}

class _HomeScreenState extends State<HomeScreen> {
  Map<String, dynamic>? _currentTask;
  Map<String, String> _pitNames = {};
  bool _loading = true;
  Timer? _timer;

  @override
  void initState() {
    super.initState();
    _loadData();
    _timer = Timer.periodic(const Duration(seconds: 15), (_) => _loadData());
  }

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }

  Future<void> _loadData() async {
    try {
      final pits = await ApiService.getPits();
      final pitMap = <String, String>{};
      for (final p in pits) {
        pitMap[p['id']] = p['name'];
      }

      final waybills = await ApiService.getWaybills(driverId: widget.driverId);
      final active = waybills.where(
          (w) => !['completed', 'cancelled'].contains(w['status']));

      if (mounted) {
        setState(() {
          _pitNames = pitMap;
          _currentTask = active.isNotEmpty ? active.first : null;
          _loading = false;
        });
      }
    } catch (e) {
      if (mounted) setState(() => _loading = false);
    }
  }

  String _statusLabel(String status) {
    const labels = {
      'pending_dispatch': '待派车',
      'dispatched': '已派车',
      'arrived': '已到场',
      'queueing': '排队中',
      'loading': '装载中',
      'loaded': '已装载',
      'weighing': '称重中',
      'completed': '已完成',
      'cancelled': '已取消',
    };
    return labels[status] ?? status;
  }

  Color _statusColor(String status) {
    switch (status) {
      case 'dispatched':
        return Colors.blue;
      case 'queueing':
        return Colors.orange;
      case 'loading':
        return Colors.blue;
      case 'completed':
        return Colors.green;
      default:
        return Colors.grey;
    }
  }

  Future<void> _arrive() async {
    if (_currentTask == null) return;
    try {
      await ApiService.arriveWaybill(_currentTask!['id'], 'driver_app');
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(
            content: Text('✅ 签到成功'),
            backgroundColor: Colors.green,
          ),
        );
        _loadData();
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('签到失败: $e'), backgroundColor: Colors.red),
        );
      }
    }
  }

  /// 扫车牌到场
  Future<void> _scanPlateToArrive() async {
    if (_currentTask == null) return;

    // 打开车牌扫描页面
    final plateNumber = await Navigator.push<String>(
      context,
      MaterialPageRoute(
        builder: (_) => const PlateScannerPage(
          title: '扫车牌到场',
        ),
      ),
    );

    if (plateNumber == null || plateNumber.isEmpty) return;

    // 调用服务端车牌到场接口
    try {
      await ApiService.arriveByPlate(
        waybillId: _currentTask!['id'],
        driverId: widget.driverId,
        plateNumber: plateNumber,
      );
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text('✅ 车牌 $plateNumber 识别成功，已到场'),
            backgroundColor: Colors.green,
          ),
        );
        _loadData();
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('到场失败: $e'), backgroundColor: Colors.red),
        );
      }
    }
  }

  Future<void> _cancelTask() async {
    if (_currentTask == null) return;
    final reason = await showDialog<String>(
      context: context,
      builder: (ctx) {
        final ctrl = TextEditingController();
        return AlertDialog(
          title: const Text('取消任务'),
          content: TextField(
            controller: ctrl,
            decoration: const InputDecoration(
              labelText: '取消原因',
              hintText: '请输入取消原因',
            ),
          ),
          actions: [
            TextButton(
                onPressed: () => Navigator.pop(ctx),
                child: const Text('取消')),
            FilledButton(
                onPressed: () => Navigator.pop(ctx, ctrl.text),
                child: const Text('确认取消')),
          ],
        );
      },
    );
    if (reason == null || reason.isEmpty) return;

    try {
      await ApiService.cancelWaybill(_currentTask!['id'], widget.driverId, reason);
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(
            content: Text('✅ 任务已取消'),
            backgroundColor: Colors.green,
          ),
        );
        _loadData();
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('取消失败: $e'), backgroundColor: Colors.red),
        );
      }
    }
  }

  Future<void> _logout() async {
    final prefs = await SharedPreferences.getInstance();
    await prefs.clear();
    if (!mounted) return;
    Navigator.pushReplacement(
      context,
      MaterialPageRoute(builder: (_) => const LoginScreen()),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('🚛 司机端'),
        actions: [
          Padding(
            padding: const EdgeInsets.only(right: 8),
            child: Text(widget.driverName,
                style: const TextStyle(fontSize: 13, color: Colors.white70)),
          ),
          IconButton(
            icon: const Icon(Icons.logout, size: 20),
            onPressed: _logout,
          ),
        ],
      ),
      body: RefreshIndicator(
        onRefresh: _loadData,
        child: _loading
            ? const Center(child: CircularProgressIndicator())
            : _currentTask == null
                ? _buildNoTask()
                : _buildTaskView(),
      ),
    );
  }

  Widget _buildNoTask() {
    return ListView(
      padding: const EdgeInsets.all(24),
      children: [
        const SizedBox(height: 60),
        const Icon(Icons.inbox_outlined, size: 80, color: Colors.grey),
        const SizedBox(height: 16),
        Text(
          '暂无任务',
          textAlign: TextAlign.center,
          style: Theme.of(context).textTheme.headlineSmall?.copyWith(
                fontWeight: FontWeight.bold,
              ),
        ),
        const SizedBox(height: 8),
        const Text(
          '等待调度员派单...',
          textAlign: TextAlign.center,
          style: TextStyle(color: Colors.grey),
        ),
        const SizedBox(height: 24),
        const Center(
          child: SizedBox(
            width: 24,
            height: 24,
            child: CircularProgressIndicator(strokeWidth: 2),
          ),
        ),
        const SizedBox(height: 60),
        const Card(
          child: Padding(
            padding: EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text('💡 提示', style: TextStyle(fontWeight: FontWeight.bold)),
                SizedBox(height: 8),
                Text('系统每15秒自动刷新任务状态',
                    style: TextStyle(color: Colors.grey, fontSize: 13)),
                SizedBox(height: 4),
                Text('如有问题请联系调度员',
                    style: TextStyle(color: Colors.grey, fontSize: 13)),
              ],
            ),
          ),
        ),
      ],
    );
  }

  Widget _buildTaskView() {
    final task = _currentTask!;
    final status = task['status'] as String;

    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        Card(
          child: Padding(
            padding: const EdgeInsets.all(20),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    const Text('当前任务',
                        style: TextStyle(
                            fontSize: 18, fontWeight: FontWeight.bold)),
                    Container(
                      padding: const EdgeInsets.symmetric(
                          horizontal: 10, vertical: 4),
                      decoration: BoxDecoration(
                        color: _statusColor(status).withOpacity(0.1),
                        borderRadius: BorderRadius.circular(20),
                      ),
                      child: Text(
                        _statusLabel(status),
                        style: TextStyle(
                          color: _statusColor(status),
                          fontWeight: FontWeight.w600,
                          fontSize: 12,
                        ),
                      ),
                    ),
                  ],
                ),
                const Divider(height: 24),
                _infoRow('运单号', task['serial_no'] ?? '-'),
                _infoRow('坑口', _pitNames[task['pit_id']] ?? '-'),
                _infoRow('预估重量',
                    '${(task['estimated_weight_ton'] ?? 0).toString()} 吨'),
                if (task['queue_number'] != null)
                  _infoRow('排队序号', '#${task['queue_number']}'),
              ],
            ),
          ),
        ),
        const SizedBox(height: 12),
        if (status == 'dispatched') ...[
          // 车牌扫描到场（推荐）
          SizedBox(
            width: double.infinity,
            height: 56,
            child: FilledButton.icon(
              onPressed: _scanPlateToArrive,
              icon: const Icon(Icons.qr_code_scanner, size: 24),
              label: const Text('📷 扫车牌到场', style: TextStyle(fontSize: 16)),
              style: FilledButton.styleFrom(
                backgroundColor: const Color(0xFF2563EB),
                shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(12)),
              ),
            ),
          ),
          const SizedBox(height: 8),
          SizedBox(
            width: double.infinity,
            height: 44,
            child: OutlinedButton.icon(
              onPressed: _arrive,
              icon: const Icon(Icons.touch_app, size: 18),
              label: const Text('手动签到到场', style: TextStyle(fontSize: 14)),
              style: OutlinedButton.styleFrom(
                shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(12)),
              ),
            ),
          ),
        ],
        if (status == 'queueing')
          Card(
            color: Colors.orange.shade50,
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Row(
                children: [
                  SizedBox(
                    width: 12,
                    height: 12,
                    child: CircularProgressIndicator(
                      strokeWidth: 2,
                      color: Colors.orange.shade700,
                    ),
                  ),
                  const SizedBox(width: 12),
                  const Text('排队中，请等待叫号...'),
                ],
              ),
            ),
          ),
        if (status == 'loading')
          Card(
            color: Colors.blue.shade50,
            child: const Padding(
              padding: EdgeInsets.all(16),
              child: Row(
                children: [
                  Text('🚜', style: TextStyle(fontSize: 20)),
                  SizedBox(width: 12),
                  Text('正在装车中...'),
                ],
              ),
            ),
          ),
        if (status == 'loaded' || status == 'weighing')
          Card(
            color: Colors.green.shade50,
            child: const Padding(
              padding: EdgeInsets.all(16),
              child: Row(
                children: [
                  Text('⚖️', style: TextStyle(fontSize: 20)),
                  SizedBox(width: 12),
                  Text('请前往地磅称重'),
                ],
              ),
            ),
          ),
        if (status == 'completed')
          Card(
            color: Colors.green.shade50,
            child: const Padding(
              padding: EdgeInsets.all(16),
              child: Row(
                children: [
                  Icon(Icons.check_circle, color: Colors.green),
                  SizedBox(width: 12),
                  Text('✅ 已完成'),
                ],
              ),
            ),
          ),
        if (!['completed', 'cancelled'].contains(status)) ...[
          const SizedBox(height: 12),
          SizedBox(
            width: double.infinity,
            height: 44,
            child: OutlinedButton.icon(
              onPressed: _cancelTask,
              icon: const Icon(Icons.cancel_outlined, size: 18),
              label: const Text('放弃任务'),
              style: OutlinedButton.styleFrom(
                foregroundColor: Colors.red,
                side: const BorderSide(color: Colors.red),
                shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(12)),
              ),
            ),
          ),
        ],
        const SizedBox(height: 16),
        const Card(
          child: Padding(
            padding: EdgeInsets.all(12),
            child: Row(
              children: [
                Text('💡', style: TextStyle(fontSize: 16)),
                SizedBox(width: 8),
                Expanded(
                  child: Text('每15秒自动刷新',
                      style: TextStyle(color: Colors.grey, fontSize: 12)),
                ),
              ],
            ),
          ),
        ),
      ],
    );
  }

  Widget _infoRow(String label, String value) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 6),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          Text(label, style: const TextStyle(color: Colors.grey)),
          Text(value,
              style: const TextStyle(fontWeight: FontWeight.w500)),
        ],
      ),
    );
  }
}
