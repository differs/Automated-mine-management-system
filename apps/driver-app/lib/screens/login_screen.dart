import 'package:flutter/material.dart';
import '../services/api_service.dart';
import 'home_screen.dart';
import 'package:shared_preferences/shared_preferences.dart';

class LoginScreen extends StatefulWidget {
  const LoginScreen({super.key});

  @override
  State<LoginScreen> createState() => _LoginScreenState();
}

class _LoginScreenState extends State<LoginScreen> {
  final _phoneController = TextEditingController();
  bool _loading = false;

  Future<void> _login() async {
    final phone = _phoneController.text.trim();
    if (phone.isEmpty) {
      _showError('请输入手机号');
      return;
    }

    setState(() => _loading = true);

    try {
      final drivers = await ApiService.getDrivers(keyword: phone);
      Map<String, dynamic> driver;

      final found =
          drivers.where((d) => d['phone'] == phone).toList();
      if (found.isNotEmpty) {
        driver = found.first;
      } else {
        driver = await ApiService.createDriver({
          'name': '司机${phone.substring(phone.length - 4)}',
          'phone': phone,
          'license_plate': '临时${phone.substring(phone.length - 4)}',
          'vehicle_type': 'dump_truck',
          'capacity_ton': 30,
        });
      }

      final prefs = await SharedPreferences.getInstance();
      await prefs.setString('driver_id', driver['id']);
      await prefs.setString('driver_name', driver['name']);
      await prefs.setString('driver_phone', phone);

      if (!mounted) return;
      Navigator.pushReplacement(
        context,
        MaterialPageRoute(
          builder: (_) => HomeScreen(
            driverId: driver['id'],
            driverName: driver['name'],
          ),
        ),
      );
    } catch (e) {
      _showError('登录失败: $e');
    } finally {
      if (mounted) setState(() => _loading = false);
    }
  }

  void _showError(String msg) {
    if (!mounted) return;
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text(msg), backgroundColor: Colors.red),
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
              Text('🚛', style: TextStyle(fontSize: 80)),
              const SizedBox(height: 16),
              Text(
                '司机登录',
                style: Theme.of(context).textTheme.headlineMedium?.copyWith(
                      fontWeight: FontWeight.bold,
                    ),
              ),
              const SizedBox(height: 8),
              Text(
                '输入手机号登录 / 自动注册',
                style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                      color: Colors.grey,
                    ),
              ),
              const SizedBox(height: 32),
              TextField(
                controller: _phoneController,
                keyboardType: TextInputType.phone,
                decoration: InputDecoration(
                  labelText: '手机号',
                  prefixIcon: const Icon(Icons.phone_android),
                  border: OutlineInputBorder(
                    borderRadius: BorderRadius.circular(12),
                  ),
                  filled: true,
                  fillColor: Colors.grey[50],
                ),
                textInputAction: TextInputAction.go,
                onSubmitted: (_) => _login(),
              ),
              const SizedBox(height: 20),
              SizedBox(
                width: double.infinity,
                height: 48,
                child: FilledButton(
                  onPressed: _loading ? null : _login,
                  style: FilledButton.styleFrom(
                    shape: RoundedRectangleBorder(
                      borderRadius: BorderRadius.circular(12),
                    ),
                  ),
                  child: _loading
                      ? const SizedBox(
                          width: 20,
                          height: 20,
                          child: CircularProgressIndicator(
                            strokeWidth: 2,
                            color: Colors.white,
                          ),
                        )
                      : const Text('登录', style: TextStyle(fontSize: 16)),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  @override
  void dispose() {
    _phoneController.dispose();
    super.dispose();
  }
}
