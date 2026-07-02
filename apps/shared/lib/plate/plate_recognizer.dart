import 'dart:io';

/// 中国车牌号识别器
///
/// 负责：
///   1. 从 OCR 文本中提取中国车牌号
///   2. 验证车牌号格式
///   3. 判断识别置信度
class PlateRecognizer {
  // 中国省份简称
  static const _provinces = [
    '京', '津', '沪', '渝', '冀', '豫', '云', '辽', '黑', '湘',
    '皖', '鲁', '新', '苏', '浙', '赣', '鄂', '桂', '甘', '晋',
    '蒙', '陕', '吉', '闽', '贵', '粤', '川', '青', '藏', '琼',
    '宁',
  ];

  // 军/警/外交等特殊牌照前缀
  static const _specialPrefixes = ['WJ', '使', '领'];

  // 完整中国车牌正则
  // 蓝牌: 京A12345
  // 绿牌(新能源): 京AD12345
  // 警车: 京A1234警
  // 教练车: 京A1234学
  // 挂车: 京A1234挂
  // 港澳: 粤Z1234港
  static final RegExp _plateRegex = RegExp(
    r'([京津沪渝冀豫云辽黑湘皖鲁新苏浙赣鄂桂甘晋蒙陕吉闽贵粤川青藏琼宁]'
    r'[A-Z]'
    r'[A-HJ-NP-Z0-9]{4,5}'
    r'[A-HJ-NP-Z0-9挂学警港澳使领]?)',
  );

  /// 从 OCR 识别文本中提取车牌号
  ///
  /// 返回所有匹配的车牌号及其置信度评估。
  static List<PlateResult> extractPlates(String ocrText) {
    final results = <PlateResult>[];
    final seen = <String>{};

    // 分行处理
    final lines = ocrText.split('\n');

    for (int i = 0; i < lines.length; i++) {
      final line = lines[i].trim();
      if (line.isEmpty) continue;

      // 尝试匹配本行
      for (final match in _plateRegex.allMatches(line)) {
        final plate = match.group(1)!;
        if (seen.contains(plate)) continue;
        seen.add(plate);

        // 计算置信度
        final confidence = _evaluateConfidence(plate, line, i, lines);

        results.add(PlateResult(
          plateNumber: plate,
          confidence: confidence,
          sourceLine: line,
          lineIndex: i,
        ));
      }

      // 尝试合并两行（有时省份简称和字母被分行识别）
      if (i < lines.length - 1) {
        final combined = line + lines[i + 1].trim();
        for (final match in _plateRegex.allMatches(combined)) {
          final plate = match.group(1)!;
          if (seen.contains(plate)) continue;
          seen.add(plate);

          results.add(PlateResult(
            plateNumber: plate,
            confidence: _evaluateConfidence(plate, combined, i, lines) - 0.1,
            sourceLine: combined,
            lineIndex: i,
            isCrossLine: true,
          ));
        }
      }
    }

    // 按置信度降序排列
    results.sort((a, b) => b.confidence.compareTo(a.confidence));
    return results;
  }

  /// 评估车牌识别置信度
  ///
  /// 基于以下因素：
  ///   - 完整匹配而非部分匹配
  ///   - 常见误识别字符修正
  ///   - 车牌号长度
  ///   - 是否包含易混淆字符
  static double _evaluateConfidence(
    String plate,
    String sourceLine,
    int lineIndex,
    List<String> allLines,
  ) {
    double confidence = 0.7; // 基础分

    // 1. 长度加分
    if (plate.length == 7) confidence += 0.15; // 蓝牌标准长度
    if (plate.length == 8) confidence += 0.10; // 新能源/警车

    // 2. 省份简称加分
    if (_provinces.any((p) => plate.startsWith(p))) confidence += 0.05;

    // 3. 仅包含允许的字符（排除 I/O 等容易混淆的）
    final validChars = RegExp(r'^[京津沪渝冀豫云辽黑湘皖鲁新苏浙赣鄂桂甘晋蒙陕吉闽贵粤川青藏琼宁A-Z0-9挂学警港澳使领]+$');
    if (validChars.hasMatch(plate)) confidence += 0.05;

    // 4. 本行文本只包含车牌（没有其他文字干扰）加分
    final cleanedLine = sourceLine
        .replaceAll(plate, '')
        .replaceAll(RegExp(r'\s+'), '');
    if (cleanedLine.isEmpty || cleanedLine.length < 3) confidence += 0.1;

    // 5. 检查是否有易混淆字符（降低置信度）
    final confusableChars = RegExp(r'[IOQ]');
    if (confusableChars.hasMatch(plate)) confidence -= 0.1;

    return confidence.clamp(0.0, 1.0);
  }

  /// 获取置信度最高的车牌
  static PlateResult? getBestPlate(String ocrText) {
    final plates = extractPlates(ocrText);
    if (plates.isEmpty) return null;
    return plates.firstWhere(
      (p) => p.confidence >= 0.6,
      orElse: () => plates.first,
    );
  }

  /// 验证车牌号是否合法
  static bool isValidPlate(String plate) {
    return _plateRegex.hasMatch(plate) && plate.length >= 7;
  }
}

/// 车牌识别结果
class PlateResult {
  /// 识别的车牌号
  final String plateNumber;

  /// 置信度（0.0 ~ 1.0）
  final double confidence;

  /// 来源文本行
  final String sourceLine;

  /// 行号
  final int lineIndex;

  /// 是否跨行识别
  final bool isCrossLine;

  const PlateResult({
    required this.plateNumber,
    required this.confidence,
    required this.sourceLine,
    required this.lineIndex,
    this.isCrossLine = false,
  });

  @override
  String toString() =>
      '$plateNumber (${(confidence * 100).toStringAsFixed(0)}%)';
}
