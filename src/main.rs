use std::{
    collections::HashMap,
    f32::consts::E,
    hash::{Hash, Hasher},
    io::Read,
    thread::sleep,
    time::Duration,
};

use enigo::{InputError, Key, Keyboard};
use iepub::prelude::{EpubAssets, EpubBuilder, EpubHtml, EpubNav};
use selenium::{
    SError,
    driver::{self, Driver, Rect},
    option::{FirefoxBuilder, Proxy},
};
use std::result::Result::Ok;
/// 腾讯云存储api
mod cos {
    use crypto::digest::Digest;
    use crypto::hmac::Hmac;
    use crypto::mac::Mac;
    use std::time::SystemTime;

    use crypto::sha1::Sha1;

    static HEX_TABLE: [char; 16] = [
        '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f',
    ];
    #[derive(Clone)]
    pub struct CosClient {
        region: String,
        app_secert: String,
        app_id: String,
        bucket_id: String,
    }

    impl CosClient {
        ///
        /// 从环境变量中构建
        ///
        /// # Errors
        ///
        /// 没有COS_REGION或者没有COS_APP_SECERT 环境变量
        ///
        pub fn new() -> CosClient {
            let region = std::env::var("COS_REGION").expect("需要 COS_REGION 环境变量");
            let app_secert = std::env::var("COS_APP_SECERT").expect("需要 COS_APP_SECERT 环境变量");
            let app_id = std::env::var("COS_APP_ID").expect("需要 COS_APP_ID 环境变量");
            let bucket_id = std::env::var("COS_BUCKET").expect("需要 COS_BUCKET_ID 环境变量");

            CosClient {
                region,
                app_secert,
                app_id,
                bucket_id,
            }
        }
    }

    fn to_hex(data: &[u8]) -> String {
        let len = data.len();
        let mut res = String::with_capacity(len * 2);

        for i in 0..len {
            res.push(HEX_TABLE[usize::from(data[i] >> 4)]);
            res.push(HEX_TABLE[usize::from(data[i] & 0x0F)]);
        }
        res
    }

    fn sign(key: &str, key_time: &str) -> String {
        let mut mac = Hmac::new(Sha1::new(), key.as_bytes());
        mac.input(key_time.as_bytes());

        let res = mac.result();
        return to_hex(res.code());
    }

    impl CosClient {
        fn sign(&self, method: &str, uri: &str, expiration: u64) -> Result<String, String> {
            if let Ok(time) = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
                let key_time = format!("{};{}", time.as_secs(), time.as_secs() + expiration);
                let sign_key = sign(&self.app_secert, &key_time);
                let from_str = format!("{method}\n/{uri}\n\n\n");
                let mut sha1 = Sha1::new();
                sha1.input_str(&from_str);
                let hash_from_str = sha1.result_str(); // 进行 sha1 Hex hash

                let str_to_sign = format!("sha1\n{key_time}\n{hash_from_str}\n");
                let sign = sign(&sign_key, &str_to_sign);

                let authoriation_str = format!(
                    "q-sign-algorithm=sha1&q-ak={}&q-sign-time={key_time}&q-key-time={key_time}&q-header-list=&q-url-param-list=&q-signature={sign}",
                    self.app_id
                );

                return Ok(authoriation_str);
            }
            Err(String::from("app error"))
        }

        pub fn get_object_url(&self, bucket: &str, key: &str) -> String {
            return format!(
                "https://{}-{}.cos.ap-{}.myqcloud.com/{}",
                bucket, self.bucket_id, &self.region, key
            );
        }

        pub fn put_object(&self, path: &str, data: Vec<u8>) -> bool {
            // let path ;
            let host = format!(
                "https://{}.cos.ap-{}.myqcloud.com",
                self.bucket_id, self.region
            );
            let r = self.sign("put", path, 3600).unwrap();
            match ureq::put(format!("{}/{}", host, path.replace(" ", "%20")))
                .header("Authorization", &r)
                .send(data)
            {
                Ok(mut res) => {
                    // log::info!("{:?}", res);
                    // log::info!("body {:?}", res.body_mut().read_to_string());
                    true
                }
                Err(e) => {
                    panic!("{:?}", e);
                }
            }
        }

        pub fn get_object(&self, path: &str) -> Result<Vec<u8>, u16> {
            let host = format!(
                "https://{}.cos.ap-{}.myqcloud.com/{path}",
                self.bucket_id, self.region
            );
            println!("host {host}");

            match ureq::get(host)
                .header(
                    "Referer",
                    std::env::var("COS_REFERER").unwrap_or("".to_string()),
                )
                .call()
            {
                Ok(mut res) => {
                    println!("{:?}", res.headers());
                    res.body_mut()
                        .with_config()
                        .limit(200 * 1024 * 1024)
                        .read_to_vec()
                        .map_err(|e| {
                            log::error!("cos {:?}", e);
                            0
                        })
                }
                Err(ureq::Error::StatusCode(code)) => {
                    if code == 404 {
                        return Err(code);
                    } else if code == 403 {
                        panic!("referer not be allowed")
                    } else {
                        return Err(code);
                    }
                }
                Err(e) => {
                    panic!("download fail");
                }
            }
        }
    }
}

// #[derive(Debug, thiserror::Error)]
// pub enum NovelError {
//     #[error("driver error: {e}")]
//     Driver { e: SError },

//     #[error("image download fail:{0}")]
//     Img(String),

//     #[error("Cf fail")]
//     Cf,

//     #[error("websit anti: {0}")]
//     Anti(String),

//     #[error("io: {source}")]
//     Io {
//         #[from]
//         source: std::io::Error,
//     },

//     #[error("write epub fail: {0}")]
//     Epub(iepub::prelude::IError),
// }

// impl From<SError> for NovelError {
//     fn from(value: SError) -> Self {
//         Self::Driver { e: value }
//     }
// }

// impl From<iepub::prelude::IError> for NovelError {
//     fn from(value: iepub::prelude::IError) -> Self {
//         Self::Epub(value)
//     }
// }

/// 生成一个短链
fn short_url(url: &str) -> String {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    url.hash(&mut h);
    h.finish().to_string()
}

fn download_img(url: &str) -> anyhow::Result<Vec<u8>> {
    for i in 1..4 {
        match ureq::get(url)
            .call()
            .map_err(|e| SError::Message(format!("download fail {}", e)))
            .and_then(|mut res| {
                res.body_mut()
                    .read_to_vec()
                    .map_err(|e| SError::Message(format!("download fail 2 {}", e)))
            }) {
            Ok(v) => {
                return Ok(v);
            }
            Err(e) => {
                log::info!("download image fail,retry {i}/3, reason: {e}");
                sleep(Duration::from_millis(200));
                continue;
            }
        }
    }
    Err(anyhow::Error::msg("img download fail"))
}

fn replace_br_html(html: String) -> String {
    let v = html
        .replace("&nbsp;&nbsp;&nbsp;&nbsp;", "<p>")
        .replace(
            r#"<br>
<br>"#,
            "</p>",
        )
        .replace("></a>", "/></a>");
    if v.contains("<p>") {
        format!("{}</p>", v.trim())
    } else {
        v.trim().to_string()
    }
}

#[derive(Debug)]
struct Args {
    /// 获取的标题部分，0全部，1括号外的，2括号里的，默认为1
    title: usize,
    help: bool,
    url: String,
    /// 不上传，默认为false，也就是要上传
    no_upload: bool,
    /// 不上传cache，默认为false，也就是要上传
    no_upload_cache: bool,
    /// 等待cf时间，默认5秒
    sleep: u64,
    /// 重试cf次数，默认3次
    retry: usize,
    /// 代理
    proxy: String,
    /// 标题
    new_title: String,
    /// 不处理图片
    no_img: bool,
}

impl Args {
    fn default() -> Args {
        Args {
            title: 1,
            help: false,
            url: String::new(),
            no_upload: false,
            sleep: 5,
            retry: 3,
            proxy: String::new(),
            new_title: String::new(),
            no_img: false,
            no_upload_cache: false,
        }
    }

    pub(crate) fn print_help() {
        let args: Vec<String> = std::env::args().collect();
        log::info!("Usage: {} [--title number] [--no-up] url", args[0]);
        log::info!("--");
        log::info!("\t--title\t获取的标题部分，0全部，1括号外的，2括号里的，默认为1");
        log::info!("\t--no-up\t不上传");
        log::info!("\t--sleep\t等待cf时间，单位秒，默认5秒");
        log::info!("\t--retry\t重试cf次数，默认3");
        log::info!("\t--proxy\t代理，如host:port");
        log::info!(r#"\t--nt\t哔哩特供，修正标题，格式：{{"旧标题":"新标题"}}"#);
    }
    pub(crate) fn parse() -> Self {
        let mut args: Vec<String> = std::env::args().collect();
        args.remove(0); // 第一个是程序自己，需要去除

        let mut res = Self::default();

        // 解析参数
        let mut iter = args.iter_mut().peekable();

        loop {
            let next = iter.next();

            if next.is_none() {
                break;
            }

            let arg = next.unwrap();

            if arg == "--title" {
                // 获取下一个参数
                res.title = iter
                    .next()
                    .expect("--title number")
                    .parse()
                    .expect("--title 0,1,2");
                if res.title > 2 {
                    panic!("--title 0,1,2")
                }
            } else if arg == "--help" {
                res.help = true;
                return res;
            } else if arg == "--no-up" {
                res.no_upload = true;
                return res;
            } else if arg == "--sleep" {
                res.sleep = iter
                    .next()
                    .expect("--sleep number")
                    .parse()
                    .expect("--sleep number");
                if res.title > 2 {
                    panic!("--sleep number")
                }
            } else if arg == "--retry" {
                res.retry = iter
                    .next()
                    .expect("--retry number")
                    .parse()
                    .expect("--retry number");
                if res.title > 2 {
                    panic!("--retry number")
                }
            } else if arg == "--proxy" {
                res.proxy = iter.next().expect("--proxy host:port").to_string();
            } else if arg == "--nt" {
                res.new_title = iter.next().expect("--nt json").to_string();
            } else {
                res.url = arg.to_string();
            }
        }
        res
    }
}

#[derive(Debug, Clone)]
pub struct ImgSrc {
    filename: String,
    url: String,
    book_id: String,
}

impl ImgSrc {
    /// 文件缓存位置
    pub fn cache_path(&self) -> String {
        format!(
            "temp/{}/{}/{}",
            self.book_id,
            Self::dir_name(),
            self.filename
        )
    }

    /// epub 里的路径
    pub fn epub_path(&self) -> String {
        format!("{}/{}", Self::dir_name(), self.filename)
    }

    pub fn dir_name() -> &'static str {
        "Images"
    }
}

pub(crate) trait Spider {
    fn get_driver(&self) -> &Driver;

    fn get_arg(&self) -> &Args;

    fn get_book_id(&self) -> String;

    ///
    /// 获取书籍基本信息
    ///
    /// # Returns
    ///
    /// 书本，标题，目录页url
    ///
    fn get_book_info(&self) -> anyhow::Result<(EpubBuilder, String, String)>;

    ///
    /// 获取menu的字符串形式
    ///
    fn get_menu_str(&self, url: String) -> anyhow::Result<String>;

    ///
    /// 解析menu_str
    ///
    /// # Returns
    /// url和标题
    fn get_menu_from_str(&self, menu_str: String) -> anyhow::Result<Vec<(Option<String>, String)>>;

    ///
    /// 获取content内容
    ///
    /// # Params
    /// url 章节的url
    /// index 当前章节的索引
    ///
    /// # Returns
    ///
    /// 返回html和图片的src集合字符串；html应该已经处理好img src的转变
    fn get_content(&self, url: String, img_src_prefix: String) -> anyhow::Result<(String, String)>;

    ///
    /// url,filename
    ///
    fn get_img_src(&self, src: &str, img_src_prefix: String) -> Vec<ImgSrc> {
        if src.is_empty() {
            return Vec::new();
        }
        let v: Vec<_> = src.split("\n").collect();

        v.iter()
            .enumerate()
            .filter(|(_, url)| !url.trim().is_empty())
            .map(|(index, url)| ImgSrc {
                book_id: self.get_book_id(),
                url: url.to_string(),
                filename: format!(
                    "{}{}.jpg",
                    img_src_prefix,
                    if index == 0 { index } else { index - 1 }
                ),
            })
            .collect()
    }

    ///
    /// 修正原始的html
    ///
    fn convert_html(&self, html: String) -> String;

    fn open_url(&self, url: &str) -> anyhow::Result<()> {
        let mut sleep_time = self.get_arg().sleep;
        for i in 0..self.get_arg().retry {
            self.get_driver().get(url)?;
            // 判断是否被cf了
            if self
                .get_driver()
                .find_element(driver::By::Id("cf-error-details"))
                .is_ok()
                || self
                    .get_driver()
                    .find_element(driver::By::Css("body"))
                    .and_then(|f| f.get_text())
                    .map(|f| f.contains("Verifying you are human"))
                    .unwrap_or(false)
                || self
                    .get_driver()
                    .execute_script("return !!window._cf_chl_opt;", &[])?
                || self
                    .get_driver()
                    .get_title()
                    .unwrap_or_else(|_| String::new())
                    .trim()
                    == "Just a moment..."
            {
                log::info!("cf, waiting for refresh");
                if i == 2 {
                    // 最后一次
                    return Err(anyhow::Error::msg("CF"));
                }
                sleep(Duration::from_secs(sleep_time));
                sleep_time = sleep_time + self.get_arg().sleep;
            } else {
                break;
            }
        }

        Ok(())
    }

    fn get_img_data(
        &self,
        src: &str,
        img_filename_prefix: String,
    ) -> anyhow::Result<Vec<(String, Vec<u8>)>> {
        let mut assets = Vec::new();
        let img = self.get_img_src(src, img_filename_prefix);
        for i in img {
            let url = &i.url;
            if url.is_empty() {
                continue;
            }
            let f = i.cache_path();
            if std::fs::exists(&f).unwrap_or(false) {
                let t = std::fs::read(&f)?;
                assets.push((i.epub_path(), t));
                continue;
            }
            log::info!("downloading img from {url} to {f}");
            let n = download_img(url.as_str())?;

            std::fs::write(&f, &n).unwrap();
            assets.push((i.epub_path(), n));
        }
        Ok(assets)
    }

    fn run(&self) -> anyhow::Result<()> {
        self.download_cache()?;

        let id = self.get_book_id();
        std::fs::create_dir_all(format!("temp/{id}/{}", ImgSrc::dir_name()))?;

        let menu_temp = format!("temp/{id}/{}.m", short_url(self.get_arg().url.as_str()));

        // 获取bookInfo
        let (mut builder, title, menu_url) = self.get_book_info()?;
        // 获取menu_str
        let menu_str: String = if let Ok(menu_str) = std::fs::read_to_string(menu_temp.as_str()) {
            menu_str
        } else {
            log::info!("get menu from {}", menu_url);
            let v = self.get_menu_str(menu_url)?;
            std::fs::write(menu_temp.as_str(), v.as_str())?;
            v
        };

        let mut navs = Vec::new();
        let mut nav: Option<EpubNav> = None;

        // 解析menu_str
        let m = self.get_menu_from_str(menu_str)?;

        for (index, (url, title)) in m.iter().enumerate() {
            if let Some(url) = url {
                log::info!("title = {}, url = {url}", title);
                let t = EpubNav::default()
                    .with_title(title.as_str())
                    .with_file_name(format!("Text/{}.xhtml", index).as_str());

                let html_temp = format!("temp/{id}/{:03}-{}.h", index, short_url(url.as_str()));
                let src_temp = format!("temp/{id}/{:03}-{}.s", index, short_url(url.as_str()));

                let (html, assets) = if let Ok(html) = std::fs::read_to_string(html_temp.as_str())
                    && let Ok(src) = std::fs::read_to_string(src_temp.as_str())
                {
                    (
                        self.convert_html(html),
                        self.get_img_data(src.as_str(), format!("{index}-"))?,
                    )
                } else {
                    let (html, src) = self
                        .get_content(url.clone(), format!("../{}/{index}-", ImgSrc::dir_name()))?;
                    std::fs::write(html_temp.as_str(), html.as_str())?;
                    std::fs::write(src_temp.as_str(), src.as_str())?;

                    (
                        self.convert_html(html),
                        self.get_img_data(src.as_str(), format!("{index}-"))?,
                    )
                };

                builder = builder.add_chapter(
                    EpubHtml::default()
                        .with_file_name(t.file_name())
                        .with_title(t.title())
                        .with_data(html.as_bytes().to_vec()),
                );
                for ele in assets {
                    builder = builder.add_assets(ele.0.as_str(), ele.1);
                }

                if let Some(n) = &mut nav {
                    n.push(t);
                } else {
                    navs.push(t);
                }
            } else {
                log::info!("title = {}", title);
                if let Some(n) = nav {
                    navs.push(n);
                }
                nav = Some(
                    EpubNav::default()
                        .with_title(title.as_str())
                        .with_file_name(format!("{}.xhtml", index + 1).as_str()),
                );
            }
        }

        if let Some(n) = nav {
            navs.push(n);
        }

        for ele in navs {
            builder = builder.add_nav(ele);
        }

        let out = self.write_epub(builder, title.as_str())?;

        self.upload(out, title.as_str())
    }

    fn write_epub(&self, book: EpubBuilder, title: &str) -> anyhow::Result<String> {
        let f = format!("out/{}.epub", title);
        log::info!("writing epub book to file {f}");
        let _ = std::fs::create_dir_all("out");
        book.file(f.as_str())?;
        Ok(f)
    }

    fn upload(&self, epub_file: String, title: &str) -> anyhow::Result<()> {
        let id = self.get_book_id();
        // 上传到cos
        let remote = format!(
            "epub/data/{}/{id}/{title}.epub",
            iepub::DateTimeFormater::default()
                .with_timezone_offset(8)
                .format("%Y-%M-%d"),
        );
        if !self.get_arg().no_upload {
            log::info!("upload file to cos {remote}");
            let client = cos::CosClient::new();
            client.put_object(&remote, std::fs::read(&epub_file).unwrap());
            let remote = format!("epub/data/cache/{id}.zip");
            if !self.get_arg().no_upload_cache {
                log::info!("uploda cache dir to cos {}", remote);
                let temp = format!(
                    "{}/novel-{id}-{}.zip",
                    std::env::temp_dir().display(),
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|f| f.as_millis())
                        .unwrap_or(0)
                );
                // 压缩目录
                let mut writer = std::fs::OpenOptions::new()
                    .create_new(true)
                    .truncate(true)
                    .write(true)
                    .open(&temp)
                    .expect("zip file create fail");
                {
                    let mut zip_w: zip::ZipWriter<&mut std::fs::File> =
                        zip::ZipWriter::new(&mut writer);
                    zip(&mut zip_w, format!("temp/{id}/").as_str()).expect("zip file fail");
                }
                client.put_object(&remote, std::fs::read(&temp)?);
            }
        }
        Ok(())
    }

    fn download_cache(&self) -> anyhow::Result<()> {
        let id = self.get_book_id();
        let base_dir = format!("temp/{id}");
        if std::fs::metadata(&base_dir)
            .map(|f| f.is_dir())
            .unwrap_or(false)
        {
            return Ok(());
        }

        let remote = format!("epub/data/cache/{id}.zip");

        let client = cos::CosClient::new();
        match client.get_object(&remote) {
            Ok(data) => {
                log::info!("extract cache zip {}", data.len());
                std::fs::write("1.zip", data.as_slice()).unwrap();
                // 解压zip
                let mut archive = zip::ZipArchive::new(std::io::Cursor::new(data))?;

                for i in 0..archive.len() {
                    let mut file = archive.by_index(i).unwrap();
                    let outpath = match file.enclosed_name() {
                        Some(path) => path,
                        None => continue,
                    };

                    let outpath =
                        std::path::PathBuf::from(format!("{base_dir}/{}", outpath.display()));
                    log::info!("extract file {}", outpath.display());
                    if file.is_dir() {
                        std::fs::create_dir_all(&outpath).unwrap();
                    } else {
                        if let Some(p) = outpath.parent() {
                            if !p.exists() {
                                std::fs::create_dir_all(p).unwrap();
                            }
                        }
                        let mut outfile = std::fs::File::create(&outpath).unwrap();
                        std::io::copy(&mut file, &mut outfile).unwrap();
                    }

                    // Get and Set permissions
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;

                        if let Some(mode) = file.unix_mode() {
                            std::fs::set_permissions(
                                &outpath,
                                std::fs::Permissions::from_mode(mode),
                            )?;
                        }
                    }
                }

                Ok(())
            }
            Err(code) => {
                if code == 404 {
                    log::warn!("not found cache from remote {remote}");
                    return Ok(());
                }
                panic!("download cache fail {code}")
            }
        }
    }
}

struct Wenku8 {
    driver: Driver,
    arg: Args,
}
impl Wenku8 {
    fn support(url: &str) -> bool {
        url.contains("wenku8")
    }

    fn new(driver: Driver, arg: Args) -> Box<dyn Spider> {
        Box::new(Wenku8 { driver, arg })
    }
}
impl Spider for Wenku8 {
    fn get_book_id(&self) -> String {
        self.arg
            .url
            .replace("https://www.wenku8.net/book/", "")
            .replace(".htm", "")
    }

    fn get_driver(&self) -> &Driver {
        &self.driver
    }

    fn get_arg(&self) -> &Args {
        &self.arg
    }

    fn get_book_info(&self) -> anyhow::Result<(EpubBuilder, String, String)> {
        let mut book = EpubBuilder::new().custome_nav(true);

        self.open_url(self.arg.url.as_str())?;

        let mut title = self
            .driver
            .find_element(driver::By::Css("#content"))?
            .find_elements(driver::By::Css("table"))?[1]
            .find_element(driver::By::Css("b"))?
            .get_text()?;
        if self.arg.title != 0 {
            // 有的标题有两部分，如 A(B) ,去除括号里的
            let begin = title.find(|f| f == '(');
            let end = title.find(|f| f == ')');
            if let Some(begin) = begin
                && let Some(end) = end
            {
                if end == title.len() - 1 && begin != 0 {
                    if self.arg.title == 1 {
                        title = title[..begin].to_string();
                    } else if self.arg.title == 2 {
                        title = title[(begin + 1)..end].to_string();
                    }
                }
            }
        }
        log::info!("book name = {}", title);
        if title.trim().is_empty() {
            return Err(anyhow::Error::msg("get book title fail"));
        }
        book = book.with_title(&title);

        let table = self
            .driver
            .find_element(driver::By::Css("#content"))?
            .find_elements(driver::By::Css("table"))?;

        let mut tags = Vec::new();

        let mut has_desc = false;

        let desc = table[2].find_elements(driver::By::Css("span"))?;

        for ele in desc {
            if let Ok(text) = ele.get_text() {
                if text.contains("作品Tags：") {
                    let temp = text.replace("作品Tags：", "");
                    tags.append(&mut temp.split(" ").map(|f| f.to_string()).collect());

                    book = book.with_subject(tags.join(",").as_str());
                } else if text.contains("内容简介：") {
                    has_desc = true;
                } else if has_desc {
                    has_desc = false;

                    book = book.with_description(&text);
                }
            }
        }

        // 封面
        let src = table[2]
            .find_element(driver::By::Css("img"))?
            .get_attribute("src")?;
        if let Some(src) = src
            && let Ok(data) = download_img(&src)
        {
            log::info!("cover src={src}");
            book = book.cover("cover.jpg", data);
        }

        // 作者和出版社
        let td = table[0].find_elements(driver::By::Css("tr"))?[2]
            .find_elements(driver::By::Css("td"))?;
        book = book
            .with_publisher(td[0].get_text()?.replace("文库分类：", "").as_str())
            // .with_identifier(id)
            .with_creator(td[1].get_text()?.replace("小说作者：", "").as_str());

        // 目录页
        let f = self
            .driver
            .find_element(driver::By::Id("content"))?
            .find_elements(driver::By::Css("fieldset"))?;
        let url = f[0]
            .find_element(driver::By::Css("a"))?
            .get_property("href")?
            .unwrap();

        Ok((book, title, url))
    }

    fn get_menu_str(&self, url: String) -> anyhow::Result<String> {
        self.open_url(url.as_str())?;
        let v :String =self.driver.execute_script(r#"return Array.from(document.getElementsByTagName('td')).filter(v=>v.innerText.trim().length>0).map(v=>{ if(v.getAttribute("class").indexOf("vcss")!=-1){   return v.innerHTML;    }else{ var a= v.childNodes[0];  return a.href +'|'+a.innerHTML;   }  }).join("\n")"#, &[])?;
        Ok(v)
    }

    fn get_menu_from_str(&self, menu_str: String) -> anyhow::Result<Vec<(Option<String>, String)>> {
        let mut res = Vec::new();
        let menu: Vec<_> = menu_str.split("\n").collect();
        for ele in menu {
            if let Some(s) = ele.find(|c: char| c == '|') {
                // 普通标题
                let url = &ele[..s];
                let title = &ele[(s + 1)..];
                res.push((Some(url.to_string()), title.to_string()));
            } else {
                res.push((None, ele.to_string()));
            }
        }
        Ok(res)
    }

    fn get_content(&self, url: String, img_src_prefix: String) -> anyhow::Result<(String, String)> {
        // 切换新标签页
        let handle = self.driver.get_window_handle()?;
        let nw = self.driver.new_window(driver::NewWindowType::Tab)?;
        self.driver.switch_to_window((nw).as_str())?;

        self.open_url(url.as_str())?;

        let src:String = self.driver.execute_script(r#"
    for(;;){var s = document.getElementById("contentdp");if(s){s.remove();}else{break;}}
    var s=document.getElementById("content");s.removeAttribute("style");
    var src = Array.from(s.getElementsByTagName('img')).map(v=>v.getAttribute('src')).join('\n');
    var start = arguments[0];
    Array.from(s.getElementsByTagName('img')).forEach((v,index)=>v.setAttribute('src', start+index+'.jpg'));
    return src;
    "#, &[img_src_prefix.as_str()])?;

        let html = self
            .driver
            .find_element(driver::By::Id("content"))?
            .get_property("innerHTML")?;

        sleep(Duration::from_secs(self.arg.sleep));
        self.driver.close_window()?;
        self.driver.switch_to_window(&handle)?;
        Ok((html.unwrap_or_else(|| String::new()), src))
    }

    fn convert_html(&self, html: String) -> String {
        replace_br_html(html)
    }
}

struct Bili {
    driver: Driver,
    arg: Args,
}

impl Bili {
    fn support(url: &str) -> bool {
        url.contains("bilinovel")
    }

    fn new(driver: Driver, arg: Args) -> Box<dyn Spider> {
        Box::new(Bili { driver, arg })
    }

    fn get_host(&self) -> String {
        "https://www.bilinovel.com".to_string()
    }

    fn get_real_url(&self, url: &str, next: bool) -> anyhow::Result<String> {
        let mut url = url.to_string();
        loop {
            println!("real url = {}", url);
            self.open_url(url.as_str())?;
            let s: String = self.driver.execute_script(
                "return ReadParams[arguments[0]];",
                &[if next { "url_next" } else { "url_previous" }],
            )?;
            log::info!("next = {} {next} temp={s}", url);

            if s.contains("_") {
                url = format!("{}{}", self.get_host(), s);
            } else {
                url = s;
                break;
            }
        }

        Ok(url)
    }
}

impl Spider for Bili {
    fn get_driver(&self) -> &Driver {
        &self.driver
    }

    fn get_arg(&self) -> &Args {
        &self.arg
    }

    fn get_book_id(&self) -> String {
        self.arg
            .url
            .replace("https://www.bilinovel.com/novel/", "")
            .replace(".html", "")
    }

    fn get_book_info(&self) -> anyhow::Result<(EpubBuilder, String, String)> {
        let sep = "|||";
        let cache = format!(
            "temp/{}/{}.b",
            self.get_book_id(),
            short_url(self.arg.url.as_str())
        );

        if let Ok(v) = std::fs::read_to_string(cache.as_str()) {
            let s: Vec<&str> = v.split(sep).collect();
            return Ok((
                EpubBuilder::new()
                    .custome_nav(true)
                    .with_title(s[0])
                    .with_creator(s[1])
                    .with_description(s[2])
                    .with_subject(s[3]),
                s[0].to_string(),
                s[4].to_string(),
            ));
        }

        self.open_url(self.arg.url.as_str())?;
        let d = self.get_driver();

        let title = d
            .find_element(driver::By::Class("book-title"))?
            .get_text()?;

        log::info!("book name={}", title);
        let author = d
            .find_element(driver::By::Class("authorname"))?
            .get_text()?;
        let tag = d
            .find_elements(driver::By::Class("tag-small"))?
            .iter()
            .flat_map(|f| f.get_text())
            .collect::<Vec<String>>()
            .join(",");

        let desc = d
            .find_element(driver::By::Id("bookSummary"))?
            .find_element(driver::By::TagName("content"))?
            .get_text()?;

        let menu_url = d
            .find_element(driver::By::Id("btnReadBook"))?
            .get_property("href")?;

        let ass = vec![
            &title,
            &author,
            &desc,
            &tag,
            menu_url.as_deref().unwrap_or(""),
        ]
        .join(sep);
        std::fs::write(cache, ass).unwrap();

        Ok((
            EpubBuilder::new()
                .custome_nav(true)
                .with_title(&title)
                .with_creator(&author)
                .with_description(&desc)
                .with_subject(&tag),
            title,
            menu_url.unwrap_or_else(|| String::new()),
        ))
    }

    fn get_menu_str(&self, url: String) -> anyhow::Result<String> {
        let d = self.get_driver();

        // 先避开cf
        self.open_url(url.as_str())?;

        let str :Vec<HashMap<String,String>> = d.execute_script(r#"return Array.from(document.getElementsByClassName("chapter-li")).filter(v=> v.className.indexOf("volume-cover") === -1)
.map(li=>{
	if (li.className.indexOf("chapter-bar") != -1 ){
		return { title:li.innerText }
	}else {
		return {title:li.innerText,url:li.children[0].href}
	}
})"#, &[]).unwrap();

        #[inline]
        fn get_content_url(urls: &[HashMap<String, String>], index: usize) -> (String, bool) {
            let mut current = index;
            let mut next = true;
            let mut size = 0;
            let temp = String::new();
            loop {
                if size >= urls.len() {
                    // 找不到
                    return (String::new(), next);
                }
                let url = urls[current].get("url").unwrap_or(&temp);
                if urls[current].len() > 1 && !url.contains("java") {
                    return (url.to_string(), next);
                }
                if current == 0 {
                    next = true;
                    current = index;
                }
                if next {
                    current += 1;
                } else {
                    current -= 1;
                }
                size += 1;
            }
        }

        let res: Vec<String> = str
            .iter()
            .enumerate()
            .map(|(s_index, f)| {
                if f.len() == 2 {
                    let url = f.get("url").unwrap();
                    let title = f.get("title").unwrap();
                    // if let Some(index) = f.find(|s| s == '|') {
                    //     let title = &f[..index];
                    //     let url = &f[(index + 1)..];
                    if url.contains("javascript") {
                        // 链接被隐藏，需要从内容页尝试获取
                        let (next_url, mode) = get_content_url(str.as_slice(), s_index);
                        let real: String = self
                            .get_real_url(
                                if next_url.starts_with("http") {
                                    next_url
                                } else {
                                    format!("{}{next_url}", self.get_host())
                                }
                                .as_str(),
                                !mode,
                            )
                            .unwrap();

                        format!(
                            "{title}|{}",
                            if real.starts_with("http") {
                                real
                            } else {
                                format!("{}{}", self.get_host(), real)
                            }
                        )
                    } else {
                        format!(
                            "{title}|{}",
                            if url.starts_with("http") {
                                url.to_string()
                            } else {
                                format!("{}{}", self.get_host(), url)
                            }
                        )
                    }
                } else {
                    f.get("title").unwrap().to_string()
                    // f.to_string()
                }
            })
            .collect();
        Ok(res.join("\n"))
    }

    fn get_menu_from_str(&self, menu_str: String) -> anyhow::Result<Vec<(Option<String>, String)>> {
        if menu_str.is_empty() {
            return Ok(Vec::new());
        }

        let v: Vec<_> = menu_str.split("\n").collect();

        Ok(v.iter()
            .map(|f| {
                if let Some(index) = f.find(|s| s == '|') {
                    let title = &f[..index];
                    let url = &f[(index + 1)..];
                    (
                        Some(format!(
                            "{}{url}",
                            if url.starts_with("http") {
                                String::new()
                            } else {
                                self.get_host()
                            }
                        )),
                        title.to_string(),
                    )
                } else {
                    (None, f.to_string())
                }
            })
            .collect())
    }

    fn get_content(&self, url: String, img_src_prefix: String) -> anyhow::Result<(String, String)> {
        let mut html = String::new();
        let mut url = url;
        let mut src = String::new();
        src.push_str(url.as_str());
        src.push_str("\n");
        loop {
            self.open_url(url.as_str())?;

            // 校验文本截断
            for j in 0..3 {
                let s :bool = self.driver.execute_script(r#"return document.getElementById('acontent').innerText.indexOf("客户端停用") == -1"#, &[])?;
                if s {
                    break;
                } else {
                    if j == 2 {
                        return Err(anyhow::Error::msg("content sub"));
                    }
                    log::info!("contnet sub refresh {j}");
                    self.driver.refresh()?;
                }
            }

            let out:Vec<String> = self.driver.execute_script(r#"var start = arguments[0]; Array.from(document.getElementsByClassName('cgo')).map(v=>v.remove()); Array.from(document.getElementById("acontent").getElementsByTagName("div")).map(v=>v.remove());  return [ location.protocol+"//"+location.host + ReadParams.url_next, Array.from(document.getElementById("acontent").getElementsByTagName("img")).map((v,index)=>{  var src = v.getAttribute("data-src"); v.removeAttribute("data-src");v.setAttribute("src",start + index+'.jpg'); return src;    }).join("\n"), document.getElementById("acontent").innerHTML ];"#, &[img_src_prefix.as_str()])?;
            // let out:Vec<String> = self.driver.execute_script(r#"var start = arguments[0]; Array.from(document.getElementsByClassName('cgo')).map(v=>v.remove()); Array.from(document.getElementById("acontent").getElementsByTagName("div")).map(v=>v.remove());  return [ location.protocol+"//"+location.host + ReadParams.url_next, "", document.getElementById("acontent").innerHTML ];"#, &[img_src_prefix.as_str()])?;

            let next = &out[0];
            if !&out[1].trim().is_empty() {
                src.push_str(&out[1].trim());
                src.push('\n');
            }
            html.push_str(out[2].as_str());
            if next.contains("_") {
                // 还要翻页
                url = next.to_string();
                sleep(Duration::from_secs(self.arg.sleep));
            } else {
                break;
            }
        }
        return Ok((html, src));
    }

    fn convert_html(&self, html: String) -> String {
        html
    }

    fn get_img_data(
        &self,
        src: &str,
        img_filename_prefix: String,
    ) -> anyhow::Result<Vec<(String, Vec<u8>)>> {
        use enigo::{
            Direction::{Click, Press, Release},
            Enigo, Key, Keyboard, Settings,
        };
        let mut assets = Vec::new();
        let img = self.get_img_src(src, img_filename_prefix);
        // 第一条是对应的章节url
        if img.is_empty() || img.len() == 1 {
            return Ok(assets);
        }
        log::info!("img src = {:?}", img);

        // 分成两部分，已下载的直接读取，未下载的重新加载
        let mut downloaded_img = Vec::new();
        let mut undownload_img = Vec::new();

        for (index, i) in img.iter().skip(1).enumerate() {
            if std::fs::exists(i.cache_path()).unwrap_or(false) {
                downloaded_img.push(i);
            } else {
                undownload_img.push((index, i));
            }
        }
        if undownload_img.len() > 0 {
            // 加载图片，只加载未下载的图片，同时只要有加载完成的就行
            let mut enigo = enigo::Enigo::new(&enigo::Settings::default()).unwrap();

            let url = &img[0].url;
            log::info!("loading img = {url}");
            self.open_url(url.as_str())?;
            let mut wait = 0;
            let count = 10;
            for j in 0..count {
                if undownload_img.is_empty() {
                    break;
                }
                wait = wait + self.get_arg().sleep;

                let img_map = format!(
                    "{{{}}}",
                    undownload_img
                        .iter()
                        .map(|f| format!(
                            r#""{}":{{"index":{},"path":"{}"}}"#,
                            f.1.url,
                            f.0,
                            f.1.cache_path()
                        ))
                        .collect::<Vec<String>>()
                        .join(",")
                );
                log::info!("undownload img = {}", img_map);
                log::info!("downloaded img = {:?}", downloaded_img);

                // 首先判断内容截断，然后处理图片，再判断图片加载
                let r: String = self.get_driver().execute_async_script(
                            r#"var callback = arguments[arguments.length - 1]; var img_map = JSON.parse(arguments[0]); document.getElementById('acontent').removeAttribute('style'); if(document.getElementById('acontent').innerText.indexOf("客戶端停用中")!=-1) {  callback("-2");}  else {  Array.from(document.getElementsByClassName("imagecontent")).filter(v=> (img_map.hasOwnProperty( v.getAttribute("src") )) || (img_map.hasOwnProperty( v.getAttribute("data-src") ) )    ).map((v,index)=>{ v.setAttribute('id','img-'+(img_map[ v.getAttribute("data-src") ]['index'])); v.setAttribute('path',img_map[ v.getAttribute("data-src") ]['path']);  v.setAttribute('src', v.getAttribute('data-src') );return v; }); setTimeout(()=> {callback( Array.from(document.getElementsByClassName("imagecontent")).filter(v=>v.complete).map(v=> v.id+"|" +v.getAttribute("path") ).filter(id=>id.startsWith("img-")).join(","));},1000); } "#,
                            &[img_map.as_str()],
                        ).unwrap();
                if r == "-2" {
                    log::info!("content sub refresh {j}");
                    // self.get_driver().refresh()?;
                    let _: () = self
                        .get_driver()
                        .execute_script("location.reload();", &[])
                        .unwrap();
                    sleep(Duration::from_secs(wait));
                } else if j == count - 1 {
                    return Err(anyhow::Error::msg("img load fail, retry "));
                } else {
                    log::info!("complete img = {}", r);
                    if r.trim().is_empty() {
                        sleep(Duration::from_secs(wait));
                        continue;
                    }

                    log::info!("waiting img load = {j} complete = {r}/{}", img.len() - 1);
                    // 下载具体的图片
                    let id: Vec<_> = r.split(",").collect();

                    for ele in id {
                        let s: Vec<_> = ele.split("|").collect();
                        if std::fs::exists(&s[1]).unwrap_or(false) {
                            // 上一次循环已经下载了的
                            continue;
                        }
                        // 利用id跳转到img
                        let _: () = self.get_driver().execute_script(
                                r#"document.getElementById('acontent').removeAttribute('style');  location.hash=  arguments[0]; "#,                    &[s[0]],
                            )?;
                        log::info!("mouse {}", ele);
                        sleep(Duration::from_secs(1));
                        // 执行下载操作
                        enigo::Mouse::move_mouse(&mut enigo, 500, 200, enigo::Coordinate::Abs)
                            .unwrap();
                        enigo::Mouse::button(
                            &mut enigo,
                            enigo::Button::Right,
                            enigo::Direction::Click,
                        )
                        .unwrap();
                        sleep(Duration::from_secs(1));
                        enigo::Keyboard::key(
                            &mut enigo,
                            enigo::Key::Unicode('v'),
                            enigo::Direction::Click,
                        )
                        .unwrap();
                        sleep(Duration::from_secs(1));

                        // 全选删除，目前后缀都统一了，如果不改后缀可以不全选删除
                        enigo.key(Key::Control, Press).unwrap();
                        enigo.key(Key::Unicode('a'), Click).unwrap();
                        enigo.key(Key::Control, Release).unwrap();

                        sleep(Duration::from_secs(1));
                        enigo.key(Key::Delete, Click).unwrap();
                        let download_path =
                            format!("{}/{}", std::env::current_dir().unwrap().display(), s[1]);
                        // 输入下载位置
                        enigo.text(download_path.as_str()).unwrap();

                        enigo::Keyboard::key(
                            &mut enigo,
                            enigo::Key::Return,
                            enigo::Direction::Click,
                        )
                        .unwrap();
                        sleep(Duration::from_secs(2));
                        if !std::fs::exists(&download_path).unwrap_or(false) {
                            return Err(anyhow::Error::msg("download img fail 2"));
                        }

                        let v: Vec<_> = s[0].split("-").collect();
                        let ind: usize = v[1].parse().unwrap();
                        log::info!("remove down {:?}  {:?}", v, undownload_img);
                        downloaded_img.push(
                            undownload_img
                                .remove(
                                    undownload_img
                                        .iter()
                                        .enumerate()
                                        .find(|f| f.1.0 == ind)
                                        .map(|f| f.0)
                                        .unwrap(),
                                )
                                .1,
                        );

                        if undownload_img.len() == 0 {
                            break;
                        }
                    }

                    // 睡眠等待
                    continue;
                }
            }
        }

        if undownload_img.len() > 0 {
            log::warn!("img not be download all, {:?}", undownload_img);
            return Err(anyhow::Error::msg("download img fail"));
        }

        // 读取图片
        for ele in downloaded_img {
            assets.push((ele.epub_path(), std::fs::read(ele.cache_path())?));
        }
        return Ok(assets);
    }
}

fn zip<W: std::io::Write + std::io::Seek>(
    zip: &mut zip::ZipWriter<&mut W>,
    current: &str,
) -> anyhow::Result<()> {
    use std::io::Write;
    fn zip_inner<W: std::io::Write + std::io::Seek>(
        zip: &mut zip::ZipWriter<&mut W>,
        current: &str,
        dir: &str,
    ) -> anyhow::Result<()> {
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored)
            .unix_permissions(0o755);
        if let Ok(meta) = std::fs::metadata(current) {
            if meta.is_file() {
                zip.start_file(current.replace(dir, ""), options).unwrap();
                let mut b = std::fs::read(current)?;
                zip.write_all(&mut b).unwrap();
            } else if meta.is_dir() {
                let entries = std::fs::read_dir(current)?
                    .map(|res| res.map(|e| e.path()))
                    .collect::<Result<Vec<_>, std::io::Error>>()?;

                for ele in entries {
                    zip_inner(zip, format!("{}", ele.display()).as_str(), dir)?;
                }
            }
        }
        Ok(())
    }

    zip_inner(zip, current, current)
}

fn main() {
    use std::io::Write;
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format(|buf, record| {
            writeln!(
                buf,
                "{} {}[{}:{}]: {}",
                iepub::DateTimeFormater::default()
                    .with_timezone_offset(8)
                    .format("%Y-%M-%d %H:%m:%s"),
                record.module_path().unwrap_or(""),
                record.line().unwrap_or(0),
                record.level(),
                record.args()
            )
        })
        .filter_module("enigo", log::LevelFilter::Off)
        .init();

    let arg = Args::parse();
    if arg.help {
        Args::print_help();
        return;
    }
    let mut option = FirefoxBuilder::new()
        .driver(
            format!(
                "{}/geckodriver",
                std::env::current_dir()
                    .map_err(|f| SError::Message(f.to_string()))
                    .unwrap()
                    .display()
            )
            .as_str(),
        )
        .disable_css()
        .add_pref_string("intl.accepg_languages", "zh-CN,en-US")
        .timeout(120);
    if !arg.proxy.is_empty() {
        option = option.proxy(Proxy::manual().ssl_proxy(&arg.proxy));
    }

    let d = Driver::new(option.build()).unwrap();

    let spider = if Wenku8::support(&arg.url) {
        Wenku8::new(d, arg)
    } else if Bili::support(&arg.url) {
        Bili::new(d, arg)
    } else {
        panic!("unsupport url")
    };

    let id = spider.get_book_id();

    match spider.run() {
        Ok(()) => {}
        Err(e) => {
            if let Ok(img) = spider.get_driver().take_screenshot() {
                let _ = std::fs::write(format!("temp/{id}/error.png"), img);
            }
            if let Ok(source) = spider.get_driver().get_page_source() {
                let _ = std::fs::write(format!("temp/{id}/source.html"), source);
            }
            panic!("error {:?}", e);
        }
    };
}
