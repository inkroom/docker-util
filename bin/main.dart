import 'dart:io';

import 'package:args/args.dart';
import 'package:bili_novel_packer/light_novel/base/light_novel_model.dart';
import 'package:bili_novel_packer/novel_packer.dart';
import 'package:bili_novel_packer/pack_argument.dart';
import 'package:bili_novel_packer/pack_callback.dart';
import 'package:console/console.dart';

const String gitUrl = "https://gitee.com/Montaro2017/bili_novel_packer";
const String version = "0.1.5";

void main(List<String> args) async {
  printWelcome();
  start(args);
}

void printWelcome() {
  print("欢迎使用轻小说打包器!");
  print("作者: Sparks");
  print("当前版本: $version");
  print("如遇报错请先查看能否正常访问 https://w.linovelib.com");
  print("否则请至开源地址携带报错信息进行反馈: $gitUrl");
}

void start(List<String> args) async {
  var parser = ArgParser();
  parser.addOption("id", abbr: "i", help: "书籍ID");
  parser.addOption("url", abbr: "u", help: "书籍URL");
  parser.addOption("catalog",
      abbr: "c", help: "下载卷，如2,5，默认全下载", defaultsTo: "0");
  parser.addOption("dir", abbr: "d", help: "输出目录");
  parser.addFlag("title",
      abbr: "t", help: "是否为每章添加标题", negatable: true, defaultsTo: true);
  parser.addFlag("help", abbr: "h", defaultsTo: false, hide: true,
      callback: (help) {
    if (help) {
      print("""
${parser.usage}
    """);
    }
  });
  ArgResults result = parser.parse(args);
  if (result['help']) {
    return;
  }
  var url = result['url'] ?? readUrl();
  var packer = NovelPacker.fromUrl(url);
  printNovelDetail(await packer.getNovel());
  Catalog catalog = await packer.getCatalog();

  var arg = PackArgument();

  if (result['url'] == null) {
    arg.packVolumes = readSelectVolume(catalog);
  } else {
    arg.packVolumes = selectVolume(catalog, result['catalog']?.toString());
  }
  arg.addChapterTitle = result['title'];
  arg.out = result['dir'] ?? arg.out;
  dir(arg);
  packer.pack(arg, ConsolePackCallback());
}

void dir(PackArgument arg) {
  if (arg.out != null && !arg.out!.endsWith("/")) {
    arg.out = "${arg.out}/";
  }
}

String readUrl() {
  String? url;
  do {
    print("请输入URL(目前暂不支持直接输入id):");
    url = stdin.readLineSync();
  } while (url == null || url.isEmpty);
  return url;
}

void printNovelDetail(Novel novel) {
  Console.write("\n");
  Console.write(novel.toString());
}

PackArgument readPackArgument(Catalog catalog) {
  var arg = PackArgument();
  var select = readSelectVolume(catalog);
  arg.packVolumes = select;

  Console.write("\n");

  arg.addChapterTitle =
      Chooser(["是", "否"], message: "是否为每章添加标题?").chooseSync() == "是";
  Console.write("\n");
  return arg;
}

List<Volume> selectVolume(Catalog catalog, String? v) {
  var input = v;
  List<Volume> selectVolumeIndex = [];

  if (input == null || input == "0") {
    for (int i = 0; i < catalog.volumes.length; i++) {
      selectVolumeIndex.add(catalog.volumes[i]);
    }
    return selectVolumeIndex;
  }
  List<String> parts = input.split(",");
  for (var part in parts) {
    List<String> range = part.split("-");
    if (range.length == 1) {
      int index = int.parse(range[0]) - 1;
      selectVolumeIndex.add(catalog.volumes[index]);
    } else {
      int from = int.parse(range[0]);
      int to = int.parse(range[1]);
      if (from > to) {
        int tmp = from;
        from = to;
        to = tmp;
      }
      for (int i = from; i <= to; i++) {
        int index = i - 1;
        selectVolumeIndex.add(catalog.volumes[index]);
      }
    }
  }
  return selectVolumeIndex;
}

List<Volume> readSelectVolume(Catalog catalog) {
  Console.write("\n");
  for (int i = 0; i < catalog.volumes.length; i++) {
    Console.write("[${i + 1}] ${catalog.volumes[i].volumeName}\n");
  }
  Console.write("[0] 选择全部\n");
  Console.write(
    "请选择需要下载的分卷(可输入如1-9进行范围选择以及如2,5单独选择):",
  );
  return selectVolume(catalog, Console.readLine());
}
