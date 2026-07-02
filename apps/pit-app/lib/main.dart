import 'package:flutter/material.dart';
import 'screens/login_screen.dart';
import 'screens/operation_screen.dart';

void main() {
  runApp(const PitApp());
}

class PitApp extends StatelessWidget {
  const PitApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: '坑口端 - 矿山调度',
      debugShowCheckedModeBanner: false,
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(
          seedColor: const Color(0xFF16A34A),
          brightness: Brightness.light,
        ),
        useMaterial3: true,
      ),
      home: const LoginScreen(),
    );
  }
}
