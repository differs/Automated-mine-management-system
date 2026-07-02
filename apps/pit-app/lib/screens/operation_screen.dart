import 'dart:async';
import 'package:flutter/material.dart';
import '../services/api_service.dart';
import 'login_screen.dart';

class OperationScreen extends StatefulWidget {
  final String pitId;
  final String pitName;
  final String operatorName;

  const OperationScreen({
    super.key,
    required this.pitId,
    required this.pitName,
    required this.operatorName,
  });

  @override
  State<OperationScreen> createState() => _OperationScreenState();
}

class _OperationScreenState extends State<OperationScreen>
    with SingleTickerProviderStateMixin {
  late TabController _tabController;
  List<dynamic> _queue = [];
  Map<String, dynamic>? _currentLoading;
  Map<String, String> _driverNames = {};
  final _finishController = TextEditingController();
  Timer? _timer;

  @override
  void initState() {
    super.initState();
    _tabController = TabController(length: 2, vsync: this);
    _loadData();
    _timer = Timer.periodic(const Duration(seconds: 15), (_) => _loadData());
  }

  @override
  void dispose() {
    _tabController.dispose();
    _timer?.cancel();
    _finishController.dispose();
    super.dispose();
  }

  Future<void> _loadData() async {
    try {
      final drivers = await ApiService.getDrivers();
      final nameMap = <String, String>{};
      for (final d in drivers) {
        nameMap[d['id']] = '${d['name']} - ${d['license_plate']}';
      }

      final queue = await ApiService.getPitQueue(widget.pitId);
      final waybills =
          await ApiService.getWaybills(pitId: widget.pitId, status: 'loading');
      final loading =
          waybills.isNotEmpty ? waybills.first as Map<String, dynamic> : null;

      if (mounted) {
        setState(() {
          _queue = queue;
          _driverNames = nameMap;
          _currentLoading = loading;
        });
      }
    } catch (_) {}
  }

  Future<void> _callNext(String waybillId) async {
    try {
      await ApiService.callNext(waybillId, widget.operatorName);
      await ApiService.startLoading(waybillId, widget.operatorName);
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(
              content: Text('✅ 已叫号并开始装车'),
              backgroundColor: Colors.green),
        );
        _loadData();
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('操作失败: $e'), backgroundColor: Colors.red),
        );
      }
    }
  }

  Future<void> _finishLoading() async {
    final suffix = _finishController.text.trim();
    if (suffix.isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(
            content: Text('请输入运单号后4位'), backgroundColor: Colors.red),
      );
      return;
    }

    try {
      final waybills = await ApiService.getWaybills(
          pitId: widget.pitId, status: 'loading');
      final target = waybills.firstWhere(
        (w) => (w['serial_no'] as String).endsWith(suffix),
      );
      final id = target['id'];
      final estimatedWeight =
          (target['estimated_weight_ton'] ?? 30).toDouble();

      await ApiService.finishLoading(id, widget.operatorName);
      await ApiService.weigh(id, widget.operatorName, estimatedWeight);

      _finishController.clear();
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(
              content: Text('✅ 装车完成并称重完毕'),
              backgroundColor: Colors.green),
        );
        _loadData();
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('操作失败: $e'), backgroundColor: Colors.red),
        );
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: Text('⛰️ ${widget.pitName}'),
        actions: [
          Padding(
            padding: const EdgeInsets.only(right: 8),
            child: Text(widget.operatorName,
                style: const TextStyle(fontSize: 12, color: Colors.white70)),
          ),
          IconButton(
            icon: const Icon(Icons.exit_to_app, size: 20),
            onPressed: () => Navigator.pushReplacement(
              context,
              MaterialPageRoute(builder: (_) => const LoginScreen()),
            ),
          ),
        ],
        bottom: TabBar(
          controller: _tabController,
          tabs: [
            Tab(text: '🔢 队列 (${_queue.length})'),
            const Tab(text: '🚜 装车中'),
          ],
        ),
      ),
      body: TabBarView(
        controller: _tabController,
        children: [
          _buildQueueTab(),
          _buildLoadingTab(),
        ],
      ),
    );
  }

  Widget _buildQueueTab() {
    return RefreshIndicator(
      onRefresh: _loadData,
      child: ListView(
        padding: const EdgeInsets.all(12),
        children: [
          if (_currentLoading != null)
            Card(
              color: Colors.blue.shade50,
              child: Padding(
                padding: const EdgeInsets.all(12),
                child: Row(
                  children: [
                    const Text('🚜', style: TextStyle(fontSize: 20)),
                    const SizedBox(width: 12),
                    Expanded(
                      child: Text(
                        '正在装车: ${_driverNames[_currentLoading!['driver_id']] ?? '未知'}',
                        style: const TextStyle(fontWeight: FontWeight.w500),
                      ),
                    ),
                  ],
                ),
              ),
            ),
          if (_queue.isEmpty)
            const Padding(
              padding: EdgeInsets.only(top: 60),
              child: Column(
                children: [
                  Icon(Icons.check_circle_outline,
                      size: 64, color: Colors.green),
                  SizedBox(height: 16),
                  Text('队列为空',
                      style:
                          TextStyle(fontSize: 18, fontWeight: FontWeight.bold)),
                  SizedBox(height: 8),
                  Text('等待司机到场排队',
                      style: TextStyle(color: Colors.grey)),
                ],
              ),
            )
          else
            ..._queue.map((entry) => Card(
                  margin: const EdgeInsets.only(bottom: 8),
                  child: ListTile(
                    leading: CircleAvatar(
                      backgroundColor: Colors.green,
                      child: Text(
                        '${entry['queue_position']}',
                        style: const TextStyle(
                            color: Colors.white, fontWeight: FontWeight.bold),
                      ),
                    ),
                    title: Text(
                      _driverNames[entry['driver_id']]?.split(' - ')[0] ??
                          '未知司机',
                      style: const TextStyle(fontWeight: FontWeight.w600),
                    ),
                    subtitle: Text(
                      _driverNames[entry['driver_id']]?.split(' - ').length ==
                              2
                          ? _driverNames[entry['driver_id']]!.split(' - ')[1]
                          : '',
                      style: TextStyle(color: Colors.grey[600], fontSize: 12),
                    ),
                    trailing: FilledButton(
                      onPressed: _currentLoading != null
                          ? null
                          : () => _callNext(entry['waybill_id']),
                      style: FilledButton.styleFrom(
                        backgroundColor: Colors.orange,
                      ),
                      child: Text(
                          _currentLoading != null ? '装车中' : '叫号'),
                    ),
                  ),
                )),
        ],
      ),
    );
  }

  Widget _buildLoadingTab() {
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        Card(
          child: Padding(
            padding: const EdgeInsets.all(20),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const Text('完成装车',
                    style:
                        TextStyle(fontSize: 18, fontWeight: FontWeight.bold)),
                const SizedBox(height: 8),
                const Text('输入当前装车辆运单号后4位',
                    style: TextStyle(color: Colors.grey, fontSize: 13)),
                const SizedBox(height: 16),
                Row(
                  children: [
                    Expanded(
                      child: TextField(
                        controller: _finishController,
                        decoration: InputDecoration(
                          hintText: '运单号后4位',
                          border: OutlineInputBorder(
                              borderRadius: BorderRadius.circular(10)),
                          filled: true,
                          fillColor: Colors.grey[50],
                        ),
                        textInputAction: TextInputAction.go,
                        onSubmitted: (_) => _finishLoading(),
                      ),
                    ),
                    const SizedBox(width: 12),
                    FilledButton(
                      onPressed: _finishLoading,
                      style: FilledButton.styleFrom(
                        backgroundColor: const Color(0xFF16A34A),
                        padding: const EdgeInsets.symmetric(
                            horizontal: 24, vertical: 14),
                      ),
                      child: const Text('完成'),
                    ),
                  ],
                ),
              ],
            ),
          ),
        ),
        if (_currentLoading != null) ...[
          const SizedBox(height: 12),
          Card(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  const Text('当前装车',
                      style: TextStyle(
                          fontSize: 16, fontWeight: FontWeight.bold)),
                  const Divider(),
                  _infoRow('司机', _driverNames[_currentLoading!['driver_id']]
                          ?.split(' - ')[0] ??
                      '未知'),
                  _infoRow('运单号', _currentLoading!['serial_no'] ?? '-'),
                ],
              ),
            ),
          ),
        ] else
          const Padding(
            padding: EdgeInsets.only(top: 40),
            child: Column(
              children: [
                Icon(Icons.inbox_outlined, size: 48, color: Colors.grey),
                SizedBox(height: 12),
                Text('暂无装车任务', style: TextStyle(color: Colors.grey)),
              ],
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
          Text(value, style: const TextStyle(fontWeight: FontWeight.w500)),
        ],
      ),
    );
  }
}
