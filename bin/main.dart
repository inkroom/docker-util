import 'dart:async';
import 'dart:io';

import 'package:args/args.dart';
import 'package:bili_novel_packer/bili_novel/bili_novel_http.dart' as bili_http;
import 'package:bili_novel_packer/bili_novel/bili_novel_model.dart';
import 'package:bili_novel_packer/bili_novel_packer.dart';
import 'package:bili_novel_packer/loading_bar.dart';
import 'package:bili_novel_packer/packer_callback.dart';
import 'package:console/console.dart';
import 'package:path/path.dart';
const String gitUrl = "https://gitee.com/Montaro2017/bili_novel_packer";
const String version = "0.0.1-beta";

void main(List<String> arguments) async {
  var parser = ArgParser();

  parser.addOption("id", abbr: "i", help: "书籍ID");
  parser.addOption("url", abbr: "u", help: "书籍URL");
  parser.addFlag("pause",
      abbr: "y", help: "是否等待", defaultsTo: true, negatable: true);
  parser.addFlag("help", abbr: "h", defaultsTo: false, hide: true,
      callback: (help) {
    if (help) {
      print("""
${parser.usage}
    """);
    }
  });

  ArgResults result = parser.parse(arguments);
  if (result['help']) {
  } else {
    printWelcome();
    start(result);
  }
}

void printWelcome() {
  print("欢迎使用哔哩轻小说打包器!");
  print("作者: Sparks");
  print("当前版本: $version");
  print("如遇报错请先查看能否正常访问 https://www.linovelib.com");
  print("否则请至开源地址携带报错信息进行反馈: $gitUrl");
}

Future<void> start(ArgResults arguments) async {
  print("");
  int? id;

  if (arguments['id'] == null && arguments['url'] == null) {
    print("请输入ID或URL:");
    id = readNovelId(stdin.readLineSync());
  } else {
    id = readNovelId(arguments['id'] ?? arguments['url']);
  }

  Novel novel = await bili_http.getNovel(id);
  print("");
  printNovel(novel);
  Catalog catalog = await bili_http.getCatalog(id);
  if (arguments['pause'] == true) pause();
  PackerCallback callback = ConsoleCallback();
  for (Volume volume in catalog.volumes) {
    String dest = getDest(novel, volume);
    BiliNovelVolumePacker packer = BiliNovelVolumePacker(
      novel: novel,
      catalog: catalog,
      volume: volume,
      dest: dest,
      callback: callback,
    );
    await packer.pack();
  }
  (callback as ConsoleCallback).stop("全部任务已完成！");
  exit(0);
}

int readNovelId(String? line) {
  if (line == null) {
    throw "输入内容不能为空";
  }
  int? id = int.tryParse(line);
  if (id != null) return id;
  RegExp exp = RegExp("novel/(\\d+)");
  RegExpMatch? match = exp.firstMatch(line);
  if (match == null || match.groupCount < 1) {
    throw "请输入正确的ID或URL";
  }
  id = int.tryParse(match.group(1)!);
  if (id == null) {
    throw "请输入正确的ID或URL";
  }
  return id;
}

void pause() {
  print("请按回车键继续...");
  stdin.readLineSync();
}

void printNovel(Novel novel) {
  print("书名: ${novel.title}");
  print("作者: ${novel.author}");
  print("状态: ${novel.status}");
  print("标签: ${novel.tags}");
  print(novel.description);
}

String getDest(Novel novel, Volume volume) {
  String name = ensureFileName(novel.title);
  String epub = ensureFileName("$name ${volume.name}.epub");
  return "out$separator$name$separator$epub";
}

String ensureFileName(String name) {
  return name.replaceAllMapped(
      "<|>|:|\"|/|\\\\|\\?|\\*|\\\\|\\|", (match) => " ");
}

class ConsoleCallback extends PackerCallback {
  String? _message;
  bool stopped = false;
  late MyLoadingBar bar = MyLoadingBar(callback: writeMessage);

  set message(String? message) {
    _message = message;
    bar.update();
  }

  String? get message => _message;

  @override
  void onAfterResolveChapter(Chapter chapter) {}

  @override
  void onAfterPack(Volume volume, String dest) {
    String absoluteDest = File(dest).absolute.path;
    Console.overwriteLine("打包完成: ${volume.name} 文件保存路径: $absoluteDest\n\n");
  }

  @override
  void onAfterResolveImage(String src, String relativeImgPath) {}

  @override
  void onBeforePack(Volume volume, String dest) {
    print("开始打包 ${volume.name}");
  }

  @override
  void onBeforeResolveChapter(Chapter chapter) {
    message = "下载章节 ${chapter.name}";
  }

  @override
  void onBeforeResolveImage(String src) {
    message = "下载图片 $src";
  }

  @override
  void onChapterUrlEmpty(Chapter chapter) {}

  @override
  void onError(error, {StackTrace? stackTrace}) {
    print(error);
    print(stackTrace);
  }

  @override
  void onSetCover(String src, String relativePath) {}

  ConsoleCallback() {
    bar.start();
  }

  void writeMessage() {
    if (stopped) return;
    Console.write("\t${message ?? ""}");
  }

  void stop([String? message]) {
    stopped = true;
    Console.overwriteLine("");
    bar.stop(message);
  }
}
