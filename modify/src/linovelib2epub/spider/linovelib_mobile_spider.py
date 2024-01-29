import re
import time
from typing import Dict, List, Optional
from urllib.parse import urljoin

import demjson3
import inquirer
import requests
from bs4 import (BeautifulSoup, Tag)

from . import BaseNovelWebsiteSpider
from .linovelib_mobile_rules import generate_mapping_result
from ..exceptions import LinovelibException
from ..models import LightNovel, LightNovelChapter, LightNovelVolume, LightNovelImage, CatalogLinovelibMobileChapter, \
    CatalogLinovelibMobileVolume
from ..utils import (cookiedict_from_str, create_folder_if_not_exists,
                     requests_get_with_retry)


class LinovelibMobileSpider(BaseNovelWebsiteSpider):

    def __init__(self, spider_settings: Optional[Dict] = None):
        super().__init__(spider_settings)
        self._init_http_client()

        # it might be better to refactor to asyncio mode
        self._mapping_result = generate_mapping_result()
        self._html_content_id = self._mapping_result.content_id
        self._mapping_dict = self._mapping_result.mapping_dict

        self.FETCH_CHAPTER_CONCURRENCY_LEVEL = 1

    def request_headers(self, referer: str = '', random_ua: bool = True):
        default_ua = 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:109.0) Gecko/20100101 Firefox/119.0'
        default_referer = 'https://www.bilinovel.com'
        headers = {
            # 'Host': 'www.bilinovel.com'
            'User-Agent': 'Mozilla/5.0 (Linux; Android 14; SAMSUNG-SM-T377A Build/NMF26X) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.6099.216/217 Mobile Safari/537.36'
            ,'Accept': 'text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8'
            ,'Accept-Language': 'zh-CN,zh;q=0.8,zh-TW;q=0.7,zh-HK;q=0.5,en-US;q=0.3,en;q=0.2'
            ,'Accept-Encoding': 'gzip, deflate, br'
            ,'DNT': '1'
            ,'Referer': default_referer
            ,'Upgrade-Insecure-Requests': '1'
            ,'Sec-Fetch-Dest': 'document'
            ,'Sec-Fetch-Mode': 'navigate'
            ,'Sec-Fetch-Site': 'same-origin'
            ,'Sec-Fetch-User': '?1'
            ,'TE': 'trailers'
        }
        return headers

    def fetch(self,novel: LightNovel) -> LightNovel:
        start = time.perf_counter()
        novel_whole = self._fetch(novel)
        self.logger.info('(Perf metrics) Fetch Book took: {} seconds'.format(time.perf_counter() - start))

        return novel_whole

    def _init_http_client(self):
        """
        Tunes http session as needed.

        Guideline: Don't move many concrete init logics to super class __init__()
        """
        self.session = requests.Session()

        if self.spider_settings["disable_proxy"]:
            self.session.trust_env = False

        # cookie example: PHPSESSID=...; night=0; jieqiUserInfo=...; jieqiVisitInfo=...
        # if self.spider_settings["http_cookie"]:
        #     cookie_dict = cookiedict_from_str(self.spider_settings["http_cookie"])
        #     cookiejar = requests.utils.cookiejar_from_dict(cookie_dict)
        #     self.session.cookies = cookiejar

    def _crawl_book_basic_info(self, url):
        result = requests_get_with_retry(self.session,
                                         url,
                                         headers=self.request_headers(),
                                         retry_max=self.spider_settings['http_retries'],
                                         timeout=self.spider_settings["http_timeout"],
                                         logger=self.logger)

        if result and result.status_code == 200:
            self.logger.info(f'Succeed to get the novel of book_id: {self.spider_settings["book_id"]}')
            soup = BeautifulSoup(result.text, 'lxml')

            try:
                book_title = soup.find('h1', {'class': 'book-title'}).text
                author = soup.find('div', {'class': 'book-rand-a'}).text[:-2]
                book_summary = soup.find('section', id="bookSummary").text
                # see issue #10, strip invalid suffix characters after ? from cover url
                book_cover_url = soup.find('img', {'class': 'book-cover'})['src'].split("?")[0]
                return book_title, author, book_summary, book_cover_url
            except (Exception,):
                self.logger.error(f'Failed to parse basic info of book_id: {self.spider_settings["book_id"]}')

        return None

    def _crawl_book_content(self, catalog_url,novel : LightNovel):
        def _get_url_from_javascript(base_url, html:str):
            # 从html中找到js代码获取下一页
            s = html.split('\n')
            for m in s :
                if m.startswith('<script type="text/javascript">var ReadParams='):
                    res=re.findall("url_next:'([^']*)'",m)
                    if len(res) == 1:
                        return f'{base_url}/{res[0]}'
                    return ''
            return ''
        def _anti_js_obfuscation(html):
            """
            recover original text of the novel content.

            :param html:
            :return: html after anti-js obfuscation
            """
            table = str.maketrans(self._mapping_dict)
            res = html.translate(table)
            return res

        def _sanitize_html(html: BeautifulSoup) -> str:
            """
            Strip useless script on body tag by reg or soup library method.
            e.g. <script>zation();</script>

            And remove all the content not needed.

            :param html:
            :return:
            """
            html_copy = BeautifulSoup(str(html), 'lxml')

            # remove <p class="ca1"> 去掉一些公告声明
            anouncements = html_copy.select(".ca1")
            for anouncement in anouncements:
                anouncement.decompose()

            # 去除 ins 标签，html中使用的<ins></ins>会被ebooklib处理成 <ins /> 导致浏览器解析错误
            anouncements = html_copy.select("ins")
            for anouncement in anouncements:
                anouncement.decompose()
            return re.sub(r'<script.+?</script>', '', str(html_copy), flags=re.DOTALL)
        if novel.catalog_list is None:
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
                catalog_html = book_catalog_rs.text
                catalog_list: List[CatalogLinovelibMobileVolume] = self._convert_to_catalog_list(catalog_html)
                if self.spider_settings['select_volume_mode']:
                    catalog_list = self._handle_select_volume(catalog_list)
            else:
                self.logger.error(f'Failed to get the catalog of book_id: {self.spider_settings["book_id"]}')
                return None
        else:
            catalog_list: List[CatalogLinovelibMobileVolume] = novel.catalog_list

        new_novel = novel
        url_next = ''
        index = -1;
        volume_id = -1

        # 获取当前已经下载的章节数
        chapter_size = 0
        for s in novel.volumes:
            chapter_size += len(s.chapters)

        for catalog_volume in catalog_list:
            volume_id += 1
            new_volume = LightNovelVolume(volume_id=volume_id)
            new_volume.title = catalog_volume.volume_title
            self.logger.info(f'volume: {catalog_volume.volume_title}')
            # 直接添加，里面会处理重复
            new_volume = new_novel.add_new_volume(new_volume)

            chapter_id = -1
            chapter_list = new_volume.chapters  # store all chapters of one volume

            # new_volume.set_chapter_list(chapter_list)
            # new_volume.chapters = chapter_list
            for catalog_chapter in catalog_volume.chapters:
                chapter_content = ''
                chapter_title = catalog_chapter.chapter_title
                chapter_id += 1
                index += 1;
                if chapter_size -1 >= index:
                    # 当前章节已下载
                    self.logger.info(f'chapter : {chapter_title} has been downloaded before')
                    continue

                # if index == 5:
                #     raise Exception(f'[ERROR]: request {chapter_title} failed.')

                light_novel_chapter = LightNovelChapter(chapter_id=chapter_id)
                light_novel_chapter.title = chapter_title
                chapter_illustrations: List[LightNovelImage] = []
                self.logger.info(f'chapter : {chapter_title} url_next={url_next}')
                # 从js中提取的url
                javascript_url_next = ''
                # 这个函数是含有状态的，必须及时覆盖 url_next 变量，否则状态机会失败
                url_next = self._expand_paginated_chapter_links(catalog_chapter, url_next)
                if len(url_next) == 0 and len(javascript_url_next) != 0:
                    url_next = javascript_url_next
                    self.logger.info(f'chapter : [{light_novel_chapter.title}] has no url, attemp use url from javascript, new url = {url_next}')


                if len(url_next) != 0:
                # for loop [chapter_index_url]+[all paginated chapters] links of one chapter
                    for page_link in catalog_chapter.chapter_urls:
                        page_resp = requests_get_with_retry(self.session, page_link,
                                                            headers=self.request_headers(),
                                                            retry_max=self.spider_settings['http_retries'],
                                                            timeout=self.spider_settings["http_timeout"],
                                                            logger=self.logger)
                        if page_resp:
                            soup = BeautifulSoup(page_resp.text, 'lxml')
                        else:
                            raise Exception(f'[ERROR]: request {page_link} failed.')
                        javascript_url_next = _get_url_from_javascript(self.spider_settings["base_url"],page_resp.text)
                        self.logger.info(f'get url from javascript {javascript_url_next}')
                        images = soup.find_all('img')
                        new_title = soup.find(id='atitle')
                        if not new_title.text.startswith(light_novel_chapter.title):
                            # 目录页部分文字会被隐藏，所以用文章中的标题代替 new_title 带有分页信息，所以不能==
                            self.logger.info(f'chapter : [{light_novel_chapter.title}] New Title= [{new_title.text}]')
                            light_novel_chapter.title = new_title.text
                        article_soup = soup.find(id=self._html_content_id)
                        article = _sanitize_html(article_soup)
                        for _, image in enumerate(images):
                            # <img class="imagecontent lazyload" data-src="https://img1.readpai.com/0/28/109869/146248.jpg" src="/images/photon.svg"/>
                            # <img border="0" class="imagecontent" src="https://img1.readpai.com/0/28/109869/146254.jpg"/>
                            html_image_src = re.search('(?<= src=").*?(?=")', str(image))
                            image_lazyload_src = image.get("data-src")

                            if image_lazyload_src:
                                remote_src = re.search('(?<= data-src=").*?(?=")', str(image)).group()
                            else:
                                remote_src = image.get("src")

                            light_novel_image = LightNovelImage(related_page_url=page_link, remote_src=remote_src,
                                                                chapter_id=chapter_id, volume_id=volume_id,
                                                                book_id=self.spider_settings["book_id"])

                            image_local_src = self.get_img_src(light_novel_image)
                            local_image = str(image).replace(str(html_image_src.group()), image_local_src)
                            article = article.replace(str(image), local_image)
                            self.logger.info(f" im {image_local_src} ")
                            chapter_illustrations.append(light_novel_image)

                        article = _anti_js_obfuscation(article)
                        chapter_content += article

                        self.logger.info(f'Processing page... {page_link}')
                else:
                    # 虽然拿不到内容，但是还是填个空，方便后续手动修改
                    chapter_content = "<p>No Url</p>"
                    self.logger.info(f'chapter : [{light_novel_chapter.title}] has no url, set empty text')
                light_novel_chapter.content = chapter_content
                light_novel_chapter.illustrations = chapter_illustrations
                chapter_list.append(light_novel_chapter)

                self._remove_duplicate_images_in_html(chapter_list)

                self.save_novel_pickle(new_novel)
                # self.logger.info(f'v : {new_novel} nv={new_volume} ll={light_novel_chapter}')
            # for chapter in chapter_list:
            #     new_volume.add_chapter(cid=chapter.chapter_id, title=chapter.title, content=chapter.content,
            #                             illustrations=chapter.illustrations)
            #     self.save_novel_pickle(new_novel)
            self.save_novel_pickle(new_novel)
        return new_novel

    def _expand_paginated_chapter_links(self, chapter: CatalogLinovelibMobileChapter, url_next):
        # fix broken links in place(catalog_lis) if exits
        # - if chapter[1] is valid link, assign it to url_next
        # - if chapter[1] is not a valid link,e.g. "javascript:cid(0)" etc. use url_next
        if not self._is_valid_chapter_link(chapter.chapter_url):
            # now the url_next value is the correct link of of chapter[1].
            chapter.chapter_url = url_next
        else:
            url_next = chapter.chapter_url
        if len(url_next) == 0:
            # 第五章 三个国家的故事：人都需要建议 没有url
            self.logger.warning(f'chapter {chapter.chapter_title} has no url')
            return url_next

        # goal: solve all page links of a certain chapter
        while True:
            resp = requests_get_with_retry(self.session, url_next, logger=self.logger)
            if resp:
                soup = BeautifulSoup(resp.text, 'lxml')
            else:
                raise Exception(f'[ERROR]: request {url_next} failed.')

            first_script = soup.find("body", {"id": "aread"}).find("script")
            first_script_text = first_script.text
            # alternative: use split(':')[-1] to get read_params_text
            read_params_text = first_script_text[len('var ReadParams='):]
            read_params_json = demjson3.decode(read_params_text)
            url_next = urljoin(f'{self.spider_settings["base_url"]}/novel', read_params_json['url_next'])

            if '_' in url_next:
                chapter.add_expand_paginated_chapter_url(url_next)
            else:
                break

        return url_next

    def _remove_duplicate_images_in_html(self, chapter_list):
        # removing duplicate images in the first chapter
        # chapter_list[0] 表示这一卷的第1个章节，在bilinovel中是插图页，这个页面部分插图会重复，会出现在这一卷的后续章节中。
        # chapter_list[1:] 表示这一卷的第2个章节开始的所有章节，也就是正文章节。

        # 这个函数的作用就是将某一卷的第1个章节（插图章节）HTML的所有重复图片img元素，全部去掉。

        def _filter_duplicate_images(match, img_src_list):
            img = match.group()
            img_src = re.search('(?<= src=").*?(?=")', img).group()
            if img_src in img_src_list:
                self.logger.info(f'Remove duplicate image in the first chapter... {img_src}')
                return ""
            else:
                return img

        img_src_list = []

        for chapter in chapter_list[1:]:
            img_src_list.extend(
                [re.search('(?<= src=").*?(?=")', i).group() for i in re.findall('<img.*?/>', chapter.content)]
            )
        chapter_list[0].content = re.sub('<img.*?/>',
                                         lambda match: _filter_duplicate_images(match, img_src_list),
                                         chapter_list[0].content)

    @staticmethod
    def _handle_select_volume(catalog_list: List[CatalogLinovelibMobileVolume]):
        def _reduce_catalog_by_selection(catalog_list: List[CatalogLinovelibMobileVolume], selection_array):
            return [volume for volume in catalog_list if volume.vid in selection_array]

        def _get_volume_choices(catalog_list: List[CatalogLinovelibMobileVolume]):
            return [(volume.volume_title, volume.vid) for volume in catalog_list]

        # step 1: need to show UI for user to select one or more volumes,
        # step 2: then reduce the whole catalog_list to a reduced_catalog_list based on user selection
        # UI show
        question_name = 'Selecting volumes'
        question_description = "Which volumes you want to download?(select one or multiple volumes)"
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

    def _convert_to_catalog_list(self, catalog_html) -> List[CatalogLinovelibMobileVolume]:
        soup_catalog = BeautifulSoup(catalog_html, 'lxml')
        # chapter_count = soup_catalog.find('h4', {'class': 'chapter-sub-title'}).find('output').text
        catalog_wrapper = soup_catalog.find('ol', {'id': 'volumes'})
        catalog_html_items = catalog_wrapper.children  # Use children to get both <h3> and <li> elements

        # catalog_html_lis is an array: [li, li, li, ...]
        # example format:
        # <h3 class="chapter-bar chapter-li">第一卷 夏娃在黎明时微笑<div class="volume-cover">...</div></h3>
        # <li class="chapter-li jsChapter"><a href="/novel/682/117077.html" class="chapter-li-a "><span class="chapter-index ">插图</span></a></li>
        # <li class="chapter-li jsChapter"><a href="/novel/682/32683.html" class="chapter-li-a "><span class="chapter-index ">「彩虹与夜色的交会──远在起始之前──」</span></a></li>

        catalog_list: List[CatalogLinovelibMobileVolume] = []

        _current_chapters: List[CatalogLinovelibMobileChapter] = []
        _current_volume_title = ""
        _volume_index = 0

        for item in catalog_html_items:
            # Check if the item is a BeautifulSoup Tag object
            if not isinstance(item, Tag):
                continue

            # is volume name
            if item.name == 'li' and 'chapter-bar' in item['class']:
                _volume_index += 1
                _current_volume_title = item.get_text()
                _current_chapters: List[CatalogLinovelibMobileChapter] = []
                new_volume = CatalogLinovelibMobileVolume(
                    vid=_volume_index,
                    volume_title=_current_volume_title,
                    chapters=_current_chapters
                )
                catalog_list.append(new_volume)
            # is normal chapter
            elif item.name == 'li' and 'jsChapter' in item['class']:
                href = item.find("a")["href"]
                chapter_url = urljoin(f'{self.spider_settings["base_url"]}/novel', href)
                new_chapter: CatalogLinovelibMobileChapter = CatalogLinovelibMobileChapter(
                    chapter_title=item.get_text(),
                    chapter_url=chapter_url
                )
                _current_chapters.append(new_chapter)

        # sanitize catalog_list => remove volume that has empty chapters
        # https://w.linovelib.com/novel/3847/catalog
        # {'vid': 3, 'volume_title': '第四卷', 'chapters': []}
        catalog_list = [catalog_volume for catalog_volume in catalog_list if catalog_volume.chapters]
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

    def _fetch(self,novel : LightNovel):
        book_url = f'{self.spider_settings["base_url"]}/novel/{self.spider_settings["book_id"]}.html'
        book_catalog_url = f'{self.spider_settings["base_url"]}/novel/{self.spider_settings["book_id"]}/catalog'
        self.logger.info(f'book_catalog_url={book_catalog_url}')
        create_folder_if_not_exists(self.spider_settings['pickle_temp_folder'])
        if len(novel.book_title) == 0:
            book_basic_info = self._crawl_book_basic_info(book_url)
            if not book_basic_info:
                raise LinovelibException(f'Fetch book_basic_info of {self.spider_settings["book_id"]} failed.')
            book_title, author, book_summary, book_cover = book_basic_info
            # set book basic info
            novel.book_id = self.spider_settings['book_id']
            novel.book_title = book_title
            novel.author = author
            novel.description = book_summary
            novel.book_cover = LightNovelImage(related_page_url=book_url,
                                                    remote_src=book_cover,
                                                    book_id=self.spider_settings["book_id"],
                                                    is_book_cover=True)
            novel.mark_basic_info_ready()

        new_novel_with_content = self._crawl_book_content(book_catalog_url,novel)
        if not new_novel_with_content:
            raise LinovelibException(f'Fetch book_content of {self.spider_settings["book_id"]} failed.')
        novel.mark_volumes_content_ready()

        if len(novel.book_title) == 0:
        # do better: use named tuple or class like NovelBasicInfoGroup
            book_title, author, book_summary, book_cover = book_basic_info
            novel = new_novel_with_content
        

            # set book basic info
            novel.book_id = self.spider_settings['book_id']
            novel.book_title = book_title
            novel.author = author
            novel.description = book_summary
            novel.book_cover = LightNovelImage(related_page_url=book_url,
                                                    remote_src=book_cover,
                                                    book_id=self.spider_settings["book_id"],
                                                    is_book_cover=True)
        novel.mark_basic_info_ready()

        return novel
