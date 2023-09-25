import 'package:bili_novel_packer/light_novel/base/light_novel_model.dart';

class PackArgument {
  // 是否添加章节标题
  late bool addChapterTitle;
  bool combineVolume = false;

  /// 默认输出位置，如果为空，以书名为目录
  String? out;

  // 选择要打包的分卷
  late List<Volume> packVolumes;
}
