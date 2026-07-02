import 'package:flutter/material.dart';
import '../services/api_service.dart';
import 'operation_screen.dart';

class LoginScreen extends StatefulWidget {
  const LoginScreen({super.key});

  @override
  State<LoginScreen> createState() => _LoginScreenState();
}

class _LoginScreenState extends State<LoginScreen> {
  final _nameController = TextEditingController();
  String? _selectedPitId;
  List<dynamic> _pits = [];
  bool _loading = true;

  @override
  void initState() {
    super.initState();
    _loadPits();
  }

  Future<void> _loadPits() async {
    try {
      final pits = await ApiService.getPits();
      if (mounted) setState(() => _pits = pits);
    } catch (_) {}
    if (mounted) setState(() => _loading = false);
  }

  void _enter() {
    if (_selectedPitId == null || _nameController.text.trim().isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(
          content: Text('请选择坑口并输入操作员名称'),
          backgroundColor: Colors.red,
        ),
      );
      return;
    }
    final pit = _pits.firstWhere((p) => p['id'] == _selectedPitId);
    Navigator.push(
      context,
      MaterialPageRoute(
        builder: (_) => OperationScreen(
          pitId: _selectedPitId!,
          pitName: pit['name'],
          operatorName: _nameController.text.trim(),
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: Center(
        child: SingleChildScrollView(
          padding: const EdgeInsets.all(32),
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              Text('⛰️', style: TextStyle(fontSize: 80)),
              const SizedBox(height: 16),
              Text('坑口作业端',
                  style: Theme.of(context).textTheme.headlineMedium?.copyWith(
                        fontWeight: FontWeight.bold,
                      )),
              const SizedBox(height: 32),
              if (_loading)
                const CircularProgressIndicator()
              else ...[
                DropdownButtonFormField<String>(
                  value: _selectedPitId,
                  decoration: InputDecoration(
                    labelText: '选择坑口',
                    border: OutlineInputBorder(
                        borderRadius: BorderRadius.circular(12)),
                    filled: true,
                    fillColor: Colors.grey[50],
                  ),
                  items: _pits.map<DropdownMenuItem<String>>((p) {
                    return DropdownMenuItem(
                        value: p['id'],
                        child: Text(
                            '${p['name']} (排队: ${p['current_queue_count']})'));
                  }).toList(),
                  onChanged: (v) => setState(() => _selectedPitId = v),
                ),
                const SizedBox(height: 16),
                TextField(
                  controller: _nameController,
                  decoration: InputDecoration(
                    labelText: '操作员名称',
                    border: OutlineInputBorder(
                        borderRadius: BorderRadius.circular(12)),
                    filled: true,
                    fillColor: Colors.grey[50],
                  ),
                ),
                const SizedBox(height: 24),
                SizedBox(
                  width: double.infinity,
                  height: 48,
                  child: FilledButton(
                    onPressed: _enter,
                    style: FilledButton.styleFrom(
                      backgroundColor: const Color(0xFF16A34A),
                      shape: RoundedRectangleBorder(
                          borderRadius: BorderRadius.circular(12)),
                    ),
                    child: const Text('进入坑口', style: TextStyle(fontSize: 16)),
                  ),
                ),
              ],
            ],
          ),
        ),
      ),
    );
  }

  @override
  void dispose() {
    _nameController.dispose();
    super.dispose();
  }
}
