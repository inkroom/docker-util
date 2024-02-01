from linovelib2epub import Linovelib2Epub, TargetSite
import argparse
import sys
from pathlib import Path
import os
import signal

def sigint_handler(signum, frame):
  global is_sigint_up
  is_sigint_up = True
  print('exit')
  sys.exit(0)



# warning!: must run within __main__ module guard due to process spawn issue.
if __name__ == '__main__':

    signal.signal(signal.SIGINT, sigint_handler)
    signal.signal(signal.SIGHUP, sigint_handler)
    signal.signal(signal.SIGTERM, sigint_handler)

    from_choices =[];
    for s in TargetSite:
        
        from_choices.append(s.value)

    parser = argparse.ArgumentParser(description='Demo of argparse')
    parser.add_argument('-i','--id',type=int,help="id",required=True)
    parser.add_argument('-s','--select',help="选择卷",default=False,action="store_true")
    parser.add_argument('-d','--divide',help="分卷",default=False,action="store_true")
    parser.add_argument('-l','--load',help="加载pickle",default=False,action="store_true")
    parser.add_argument('-f','--site',help="来源",default='linovelib_mobile',choices=from_choices)
    parser.add_argument('-w','--date',help="输出文件夹附带日志",default=False,action="store_true")
    parser.add_argument('-n','--new_title',help='json object格式的新标题',default='')
    parser.add_argument('-wh','--webdav_host',help='webdav_host',default='')
    parser.add_argument('-wu','--webdav_username',help='webdav_username',default='')
    parser.add_argument('-wp','--webdav_password',help='webdav_password',default='')






    args = parser.parse_args()

    def get_site(site):
        for color in TargetSite:
            if color.value == site:
                return color

    image_folder = 'temp/' + str(args.id) + '/images'
    pickle_folder = 'temp/' + str(args.id) +'/pickle'
    if not os.path.exists(image_folder):
        os.makedirs(image_folder);
    if not os.path.exists(pickle_folder):
        os.makedirs(pickle_folder);
    linovelib_epub = Linovelib2Epub(book_id=args.id,target_site=get_site(args.site), divide_volume=args.divide,has_illustration=True,select_volume_mode=args.select,clean_artifacts=False,custom_style_chapter='h1{text-align: center;}h2{text-align: center;}',image_download_folder=image_folder,pickle_temp_folder=pickle_folder,load_pickle=args.load,with_date=args.date,new_title=args.new_title,webdav_host=args.webdav_host,webdav_username=args.webdav_username,webdav_password=args.webdav_password)
    linovelib_epub.run()
