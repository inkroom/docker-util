import re
import os
import time
from typing import Dict, List, Optional, Union
from urllib.parse import urljoin

import inquirer
import requests

from ..exceptions import LinovelibException
from ..models import LightNovel, LightNovelChapter, LightNovelVolume, LightNovelImage
from ..utils import (cookiedict_from_str, create_folder_if_not_exists,
                     requests_get_with_retry)
from . import BaseNovelWebsiteSpider
from .linovelib_mobile_rules import generate_mapping_result


class UAASpider(BaseNovelWebsiteSpider):

    def __init__(self, spider_settings: Optional[Dict] = None):
        spider_settings['base_url']='https://api.uaa.com'
        super().__init__(spider_settings)
        self._init_http_client()

        # it might be better to refactor to asyncio mode
        self._mapping_result = generate_mapping_result()
        self._html_content_id = self._mapping_result.content_id
        self._mapping_dict = self._mapping_result.mapping_dict

    def dump_settings(self):
        self.logger.info(self.spider_settings)

    def request_headers(self, referer: str = '', random_ua: bool = True):
        default_ua = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/104.0.0.0 Safari/537.36'
        default_referer = 'https://www.uaa.com'
        headers = {
            'Origin': referer if referer else default_referer,
            'referer': referer if referer else default_referer,
            'user-agent': self.spider_settings['random_useragent'] if random_ua else default_ua,
            'Cookie':os.getenv('uaa_cookie'),
            'token':os.getenv('uaa_token')
        }
        return headers

    def fetch(self) -> LightNovel:
        start = time.perf_counter()
        novel_whole = self._fetch()
        self.logger.info('(Perf metrics) Fetch Book took: {} seconds'.format(time.perf_counter() - start))

        return novel_whole

    def get_image_filename(self, url):
        # example: https://img.linovelib.com/0/682/117077/50675.jpg => 117077/50677.jpg
        # "117077" will be treated as a folder
        # "50677.jpg" is the image filename
        return '/'.join(url.split("/")[-2:])

    def _init_http_client(self):
        """
        Tunes http session as needed.

        Guideline: Don't move many concrete init logics to super class __init__()
        """
        self.session = requests.Session()

        if self.spider_settings["disable_proxy"]:
            self.session.trust_env = False

        # cookie example: PHPSESSID=...; night=0; jieqiUserInfo=...; jieqiVisitInfo=...
        if self.spider_settings["http_cookie"]:
            cookie_dict = cookiedict_from_str(self.spider_settings["http_cookie"])
            cookiejar = requests.utils.cookiejar_from_dict(cookie_dict)
            self.session.cookies = cookiejar

    def _crawl_book_basic_info(self, url):
        result = requests_get_with_retry(self.session,
                                    url,
                                    headers=self.request_headers(),
                                    retry_max=self.spider_settings['http_retries'],
                                    timeout=self.spider_settings["http_timeout"],
                                    logger=self.logger)


        if result and result.status_code == 200:
            self.logger.info(f'Succeed to get the novel of book_id: {self.spider_settings["book_id"]}')

            try:
                json = result.json()
                book_title = json['model']['title']
                author = json['model']['authors']
                state = '连载中' if json['model']['finished'] == 0 else '已完结'
                book_summary = f'{state}\n{json["model"]["pornRateDesc"]}\n{json["model"]["categories"]}\n{json["model"]["brief"]}'
                # see issue #10, strip invalid suffix characters after ? from cover url
                book_cover_url = json['model']['coverUrl']
                update_time = json['model']['updateTime']
                latest_update_chapterId = json['model']['latestUpdateChapterId']

                self.logger.info(f'book title = [{book_title}] book state = [{state}]')

                return book_title, author, book_summary, book_cover_url, update_time, latest_update_chapterId

            except Exception as inst:
                self.logger.error(f'Failed to parse basic info of book_id: {self.spider_settings["book_id"]}')

        return None

    def _crawl_book_content(self, catalog_url, book_title: str):
        book_catalog_rs = None
        try:
            book_catalog_rs = requests_get_with_retry(self.session,
                                                 catalog_url,
                                                 headers=self.request_headers(),
                                                 retry_max=self.spider_settings['http_retries'],
                                                 timeout=self.spider_settings["http_timeout"],
                                                 logger=self.logger)
        except (Exception,):
            self.logger.error(f'Failed to get normal response of {catalog_url}. It may be a network issue.')

        if book_catalog_rs and book_catalog_rs.status_code == 200:
            self.logger.info(f'Succeed to get the catalog of book_id: {self.spider_settings["book_id"]}')

            # parse catalog data
            catalog_lis = book_catalog_rs.json()['model']['menus']

            # catalog_lis is an array: [li, li, li, ...]
            # example format:
            # <li class="chapter-bar chapter-li">第一卷 夏娃在黎明时微笑</li>
            # <li class="chapter-li jsChapter"><a href="/novel/682/117077.html" class="chapter-li-a "><span class="chapter-index ">插图</span></a></li>
            # <li class="chapter-li jsChapter"><a href="/novel/682/32683.html" class="chapter-li-a "><span class="chapter-index ">「彩虹与夜色的交会──远在起始之前──」</span></a></li>

            catalog_list = self._convert_to_catalog_list(catalog_lis)
            if self.spider_settings['select_volume_mode']:
                catalog_list = self._handle_select_volume(catalog_list)

            new_novel = LightNovel()
            url_next = ''

            volume_id = 0
            for volume_dict in catalog_list:
                volume_id += 1

                new_volume = LightNovelVolume(volume_id=volume_id,title=volume_dict['volume_title'])

                self.logger.info(f'volume: {volume_dict["volume_title"]}')


                chapter_id = -1
                chapter_list = []  # store chapter for removing duplicate images in the first chapter
                for chapter in volume_dict['chapters']:
                    chapter_title = chapter[0]
                    chapter_id += 1

                    new_chapter = LightNovelChapter(chapter_id=chapter_id,title=chapter_title)
                    chapter_illustrations: List[LightNovelImage] = []

                    new_chapter.illustrations = chapter_illustrations # 不下载插图，所以给个空数组就行
                    # new_chapter.content = 'UNSOLVED'

                    self.logger.info(f'book title : [{book_title}] chapter : {chapter_title}')

                    # fix broken links in place(catalog_lis) if exits
                    # - if chapter[1] is valid link, assign it to url_next
                    # - if chapter[1] is not a valid link,e.g. "javascript:cid(0)" etc. use url_next
                    # if not self._is_valid_chapter_link(chapter[1]):
                    #     # now the url_next value is the correct link of of chapter[1].
                    #     chapter[1] = url_next
                    # else:
                    #     url_next = chapter[1]
                    url_next = chapter[1]

                    resp = requests_get_with_retry(self.session, url_next, headers=self.request_headers(), retry_max=self.spider_settings['http_retries'], logger=None)
                    if resp:
                        soup = resp.json()['model']['lines']
                        new_chapter.content = '<p>' + '</p><p>'.join(filter(lambda x: x!='', soup)) + '</p>';
                    chapter_list.append(new_chapter)

                for chapter in chapter_list:
                    new_volume.add_chapter(
                        cid=chapter.chapter_id,
                        title=chapter.title,
                        content=chapter.content
                    )


                new_novel.add_volume(
                    vid=new_volume.volume_id,
                    title=new_volume.title,
                    chapters=new_volume.chapters
                )
                new_novel.mark_volumes_content_ready()

            return new_novel

        else:
            self.logger.error(f'Failed to get the catalog of book_id: {self.spider_settings["book_id"]}')

        return None

    def _handle_select_volume(self, catalog_list):
        def _reduce_catalog_by_selection(catalog_list, selection_array):
            return [volume for volume in catalog_list if volume['vid'] in selection_array]

        def _get_volume_choices(catalog_list):
            """
            [(volume_title,vid),(volume_title,vid),...]

            :param catalog_list:
            :return:
            """
            return [(volume['volume_title'], volume['vid']) for volume in catalog_list]

        # step 1: need to show UI for user to select one or more volumes,
        # step 2: then reduce the whole catalog_list to a reduced_catalog_list based on user selection
        # UI show
        question_name = 'Selecting volumes'
        question_description = "Which volumes you want to download?(select one or multiple volumes)"
        # [(volume_title,vid),(volume_title,vid),...]
        volume_choices = _get_volume_choices(catalog_list)
        questions = [
            inquirer.Checkbox(question_name,
                              message=question_description,
                              choices=volume_choices, ),
        ]
        # user input
        # answers: {'Selecting volumes': [3, 6]}
        answers = inquirer.prompt(questions)
        catalog_list = _reduce_catalog_by_selection(catalog_list, answers[question_name])
        return catalog_list

    def _convert_to_catalog_list(self, catalog_html_lis) -> list:
        # return example:
        # [{vid:1,volume_title: "XX", chapters:[[xxx,u1,u2,u3],[xx,u1,u2],[...] ]},{},{}]

        catalog_list = []
        current_volume = []
        current_volume_text = catalog_html_lis[0]['title']
        volume_index = 0

        for index, catalog_li in enumerate(catalog_html_lis):
            catalog_li_text = catalog_li['title']
            volume_index += 1
            # reset current_* variables
            current_volume_text = catalog_li_text
            current_volume = []
            only_one = False
   
            if str(catalog_li['type'])=='2':
                for index2, menu in enumerate(catalog_li['children']):
                    whole_url = urljoin(self.spider_settings['base_url'], f'/novel/app/novel/chapter?force=false&offset=0&viewId=16975346141638004&id='+menu['id'])
                    current_volume.append([menu['title'], whole_url])
            else:
                # 不分卷
                only_one=True
                whole_url = urljoin(self.spider_settings['base_url'], f'/novel/app/novel/chapter?force=false&offset=0&viewId=16975346141638004&id='+catalog_li['id'])
                current_volume.append([catalog_li['title'], whole_url])

            catalog_list.append({
                'only_one':only_one,
                'vid': volume_index,
                'volume_title': current_volume_text,
                'chapters': current_volume
            })
        return catalog_list

    @staticmethod
    def _is_valid_chapter_link(href: str):
        # normal link example: https://w.linovelib.com/novel/682/117077.html
        # broken link example: javascript: cid(0)
        # use https://regex101.com/ to debug regular expression
        reg = r"\S+/novel/\d+/\S+\.html"
        re_match = bool(re.match(reg, href))
        return re_match

    @staticmethod
    def _extract_image_list(image_dict=None):
        image_url_list = []
        for volume_images in image_dict.values():
            for index in range(0, len(volume_images)):
                image_url_list.append(volume_images[index])

        return image_url_list

    def _fetch(self):
        book_url = f'{self.spider_settings["base_url"]}/novel/app/novel/intro?id={self.spider_settings["book_id"]}'
        book_catalog_url = f'{self.spider_settings["base_url"]}/novel/app/novel/catalog/{self.spider_settings["book_id"]}'
        create_folder_if_not_exists(self.spider_settings['pickle_temp_folder'])

        book_basic_info = self._crawl_book_basic_info(book_url)
        if not book_basic_info:
            raise LinovelibException(f'Fetch book_basic_info of {self.spider_settings["book_id"]} failed.')

        book_title, author, book_summary, book_cover, update_time, latest_update_chapterId = book_basic_info

        new_novel_with_content = self._crawl_book_content(book_catalog_url, book_title)
        if not new_novel_with_content:
            raise LinovelibException(f'Fetch book_content of {self.spider_settings["book_id"]} failed.')

        # do better: use named tuple or class like NovelBasicInfoGroup
        novel_whole = new_novel_with_content
        novel_whole.mark_volumes_content_ready()

        # set book basic info
        novel_whole.bid = self.spider_settings['book_id']
        novel_whole.book_title = book_title
        novel_whole.author = author
        novel_whole.description = book_summary
        novel_whole.book_cover = LightNovelImage(site_base_url=self.spider_settings["base_url"],
                                                 related_page_url=book_url,
                                                 remote_src=book_cover,
                                                 book_id=self.spider_settings["book_id"],
                                                 is_book_cover=True)
        novel_whole.update_time = update_time
        novel_whole.latest_update_chapterId = latest_update_chapterId
        novel_whole.mark_basic_info_ready()

        return novel_whole
