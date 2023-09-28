from linovelib2epub.linovel import Linovelib2Epub
import argparse
import sys
from pathlib import Path
import os

# warning!: must run within __main__ module guard due to process spawn issue.
if __name__ == '__main__':
    parser = argparse.ArgumentParser(description='Demo of argparse')
    parser.add_argument('-i','--id',type=int,help="id",required=True)
    parser.add_argument('-s','--select',help="选择卷",default=False,action="store_true")
    parser.add_argument('-d','--divide',help="分卷",default=False,action="store_true")

    args = parser.parse_args()


    if not os.path.exists('temp/images'):
        os.makedirs('temp/images');
    if not os.path.exists('temp/pickle'):
        os.makedirs('temp/pickle');
    linovelib_epub = Linovelib2Epub(book_id=args.id,divide_volume=args.divide,select_volume_mode=args.select,clean_artifacts=False,custom_style_chapter='h1{text-align: center;}h2{text-align: center;}',image_download_folder='temp/images',pickle_temp_folder='temp/pickle')
    linovelib_epub.run()