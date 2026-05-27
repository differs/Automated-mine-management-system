import 'package:flutter/material.dart';

void main() {
  runApp(const PitApp());
}

class PitApp extends StatelessWidget {
  const PitApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Pit App',
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(seedColor: const Color(0xFF8A4B08)),
      ),
      home: const PitHomePage(),
    );
  }
}

class PitHomePage extends StatelessWidget {
  const PitHomePage({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Pit Operations')),
      body: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: const [
            Text(
              'Live queue',
              style: TextStyle(fontSize: 22, fontWeight: FontWeight.bold),
            ),
            SizedBox(height: 12),
            Card(
              child: ListTile(
                title: Text('Queue data not connected yet'),
                subtitle: Text('This placeholder app will handle check-in, loading, and weigh flow.'),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

