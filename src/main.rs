use std::{
    f32::consts::E,
    hash::{Hash, Hasher},
    thread::sleep,
    time::Duration,
};

use iepub::prelude::{EpubAssets, EpubBuilder, EpubHtml, EpubNav};
use selenium::{
    SError,
    driver::{self, Driver},
    option::FirefoxBuilder,
};
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
                    // println!("{:?}", res);
                    // println!("body {:?}", res.body_mut().read_to_string());
                    true
                }
                Err(e) => {
                    panic!("{:?}", e);
                }
            }
        }
    }
}

/// 生成一个短链
fn short_url(url: &str) -> String {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    url.hash(&mut h);
    h.finish().to_string()
}

fn open_url(url: String, driver: &Driver, arg: &Args) -> Result<(), SError> {
    let mut sleep_time = arg.retry;
    for i in 0..arg.retry {
        driver.get(url.as_str())?;
        // 判断是否被cf了
        if driver
            .find_element(driver::By::Id("cf-error-details"))
            .is_ok()
            || driver
                .find_element(driver::By::Css("body"))
                .and_then(|f| f.get_text())
                .map(|f| f.contains("Verifying you are human"))
                .unwrap_or(false)
        {
            println!("cf, waiting for refresh");
            if i == 2 {
                // 最后一次
                return Err(SError::Message("CF".to_string()));
            }
            sleep(Duration::from_secs(sleep_time));
            sleep_time = sleep_time + arg.sleep;
        } else {
            break;
        }
    }

    Ok(())
}

fn run(driver: &Driver, url: &str, id: &str, arg: &Args) -> Result<(String, EpubBuilder), SError> {
    let mut book = EpubBuilder::new().custome_nav(true);

    open_url(url.to_string(), driver, arg)?;

    let mut title = driver
        .find_element(driver::By::Css("#content"))?
        .find_elements(driver::By::Css("table"))?[1]
        .find_element(driver::By::Css("b"))?
        .get_text()?;
    if arg.title != 0 {
        // 有的标题有两部分，如 A(B) ,去除括号里的
        let begin = title.find(|f| f == '(');
        let end = title.find(|f| f == ')');
        if let Some(begin) = begin
            && let Some(end) = end
        {
            if end == title.len() - 1 && begin != 0 {
                if arg.title == 1 {
                    title = title[..begin].to_string();
                } else if arg.title == 2 {
                    title = title[(begin + 1)..end].to_string();
                }
            }
        }
    }
    println!("title = {}", title);
    if title.trim().is_empty() {
        return Err(SError::Driver("get book title fail".to_string()));
    }
    book = book.with_title(&title);

    let table = driver
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
        println!("cover src={src}");
        book = book.cover("cover.jpg", data);
    }

    // 作者和出版社
    let td =
        table[0].find_elements(driver::By::Css("tr"))?[2].find_elements(driver::By::Css("td"))?;
    book = book
        .with_publisher(td[0].get_text()?.replace("文库分类：", "").as_str())
        // .with_identifier(id)
        .with_creator(td[1].get_text()?.replace("小说作者：", "").as_str());

    // 目录页
    let f = driver
        .find_element(driver::By::Id("content"))?
        .find_elements(driver::By::Css("fieldset"))?;
    let url = f[0]
        .find_element(driver::By::Css("a"))?
        .get_property("href")?
        .unwrap();
    println!("menu url = {url}");
    Ok((title, get_menu(url, driver, book, id, arg)?))
}

fn download_img(url: &str) -> Result<Vec<u8>, SError> {
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
                println!("download image fail,retry {i}/3, reason: {e}");
                sleep(Duration::from_millis(200));
                continue;
            }
        }
    }
    Err(SError::Http(0, "img download fail".to_string()))
}

fn get_img_data(src: &str, index: usize, id: &str) -> Result<Vec<(String, Vec<u8>)>, SError> {
    let mut assets = Vec::new();

    let s: Vec<_> = src.split("\n").collect();
    // 下载图片
    for (i, ele) in s.iter().enumerate() {
        if ele.trim().is_empty() {
            continue;
        }
        let f = format!("temp/{id}/Images/{}-{}.jpg", index, i);
        if std::fs::exists(&f).unwrap_or(false) {
            let t = std::fs::read(&f)?;
            assets.push((f.replace(format!("temp/{id}/").as_str(), ""), t));
            continue;
        }
        println!("downloading img from {ele} to {f}");
        let n = download_img(ele)?;

        std::fs::write(&f, &n).unwrap();
        assets.push((f.replace(format!("temp/{id}/").as_str(), ""), n));
    }
    Ok(assets)
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

fn get_content(
    url: String,
    driver: &Driver,
    title: &str,
    file_name: &str,
    index: usize,
    id: &str,
    arg: &Args,
) -> Result<(EpubHtml, Vec<(String, Vec<u8>)>), SError> {
    let html_temp = format!("temp/{id}/{}.h", short_url(url.as_str()));
    let src_temp = format!("temp/{id}/{}.s", short_url(url.as_str()));

    if let Ok(html) = std::fs::read_to_string(html_temp.as_str())
        && let Ok(src) = std::fs::read_to_string(src_temp.as_str())
    {
        return Ok((
            EpubHtml::default()
                .with_title(title)
                .with_file_name(file_name)
                .with_data(replace_br_html(html).as_bytes().to_vec()),
            get_img_data(src.as_str(), index, id)?,
        ));
    }

    // 切换新标签页
    let handle = driver.get_window_handle()?;
    let nw = driver.new_window(driver::NewWindowType::Tab)?;
    driver.switch_to_window((nw).as_str())?;

    println!("get content title={title} url={url}");

    open_url(url.clone(), driver, arg)?;

    let src:String = driver.execute_script(r#"
    for(;;){var s = document.getElementById("contentdp");if(s){s.remove();}else{break;}}
    var s=document.getElementById("content");s.removeAttribute("style");
    var src = Array.from(s.getElementsByTagName('img')).map(v=>v.getAttribute('src')).join('\n');
    var start = arguments[0];
    Array.from(s.getElementsByTagName('img')).forEach((v,index)=>v.setAttribute('src','../Images/'+start+'-'+index+'.jpeg'));
    return src;
    "#, &[index.to_string().as_str()])?;

    std::fs::write(src_temp, src.as_str())?;

    let assets = get_img_data(src.as_str(), index, id)?;

    let html = driver
        .find_element(driver::By::Id("content"))?
        .get_property("innerHTML")?;

    std::fs::write(html_temp.as_str(), html.as_ref().unwrap())?;

    sleep(Duration::from_secs(5));
    driver.close_window()?;
    driver.switch_to_window(&handle)?;

    Ok((
        EpubHtml::default()
            .with_title(title)
            .with_file_name(file_name)
            .with_data(
                replace_br_html(html.unwrap_or_else(|| String::new()))
                    .as_bytes()
                    .to_vec(),
            ),
        assets,
    ))
}

fn get_menu(
    url: String,
    driver: &Driver,
    book: EpubBuilder,
    id: &str,
    arg: &Args,
) -> Result<EpubBuilder, SError> {
    let mut book = book;
    std::fs::create_dir_all(format!("temp/{id}/Images"))?;
    let menu_temp = format!("temp/{id}/{}.m", short_url(url.as_str()));

    let menu_str: String = if let Ok(menu_str) = std::fs::read_to_string(menu_temp.as_str()) {
        menu_str
    } else {
        println!("get menu from {url}");
        driver.get(&url)?;
        let v :String =         driver.execute_script(r#"return Array.from(document.getElementsByTagName('td')).filter(v=>v.innerText.trim().length>0).map(v=>{ if(v.getAttribute("class").indexOf("vcss")!=-1){   return v.innerHTML;    }else{ var a= v.childNodes[0];  return a.href +'|'+a.innerHTML;   }  }).join("\n")"#, &[])?;
        std::fs::write(menu_temp.as_str(), v.as_str())?;
        v
    };

    let mut navs = Vec::new();
    let mut nav: Option<EpubNav> = None;

    let menu: Vec<_> = menu_str.split("\n").collect();
    for (index, ele) in menu.iter().enumerate() {
        if let Some(s) = ele.find(|c: char| c == '|') {
            // 普通标题
            let url = &ele[..s];
            let title = &ele[(s + 1)..];

            println!("title = {}, url = {url}", title);

            let t = EpubNav::default()
                .with_title(title)
                .with_file_name(format!("Text/{}.xhtml", index).as_str());
            let (html, assets) = get_content(
                url.to_string(),
                driver,
                t.title(),
                t.file_name(),
                index,
                id,
                arg,
            )?;
            book = book.add_chapter(html);

            for ele in assets {
                book = book.add_assets(ele.0.as_str(), ele.1);
            }

            if let Some(n) = &mut nav {
                n.push(t);
            } else {
                navs.push(t);
            }
        } else {
            // 卷标题
            println!("title = {}", ele);
            if let Some(n) = nav {
                navs.push(n);
            }
            nav = Some(
                EpubNav::default()
                    .with_title(ele)
                    .with_file_name(format!("{}.xhtml", index + 1).as_str()),
            );
        }
    }
    if let Some(n) = nav {
        navs.push(n);
    }

    for ele in navs {
        book = book.add_nav(ele);
    }

    Ok(book)
}

#[derive(Debug)]
struct Args {
    /// 获取的标题部分，0全部，1括号外的，2括号里的，默认为1
    title: usize,
    help: bool,
    url: String,
    /// 不上传，默认为false，也就是要上传
    no_upload: bool,
    /// 等待cf时间，默认5秒
    sleep: usize,
    /// 重试cf次数，默认3次
    retry: usize,
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
        }
    }

    pub(crate) fn print_help() {
        let args: Vec<String> = std::env::args().collect();
        println!("Usage: {} [--title number] [--no-up] url", args[0]);
        println!("--");
        println!("\t--title\t获取的标题部分，0全部，1括号外的，2括号里的，默认为1");
        println!("\t--no-up\t不上传");
        println!("\t--sleep\t等待cf时间，单位秒，默认5秒");
        println!("\t--retry\t重试cf次数，默认3");
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
            } else {
                res.url = arg.to_string();
            }
        }
        res
    }
}

fn main() {
    let arg = Args::parse();
    if arg.help {
        Args::print_help();
        return;
    }
    let option = FirefoxBuilder::new()
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
        .disable_image()
        .timeout(120)
        .build();

    let d = Driver::new(option).unwrap();

    let id = arg
        .url
        .replace("https://www.wenku8.net/book/", "")
        .replace(".htm", "");

    match run(&d, arg.url.as_str(), id.as_str(), &arg) {
        Ok((title, book)) => {
            let f = format!("out/{}.epub", title);
            println!("writing epub book to file {f}");
            let _ = std::fs::create_dir_all("out");
            book.file(f.as_str()).unwrap();

            // 上传到cos
            let remote = format!(
                "epub/data/{}/{}/{f}",
                time_display(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|v| v.as_secs())
                        .unwrap_or(0)
                ),
                id
            );
            if !arg.no_upload {
                println!("upload file to cos {remote}");
                cos::CosClient::new().put_object(&remote, std::fs::read(f.as_str()).unwrap());
            }
        }
        Err(e) => {
            if let Ok(img) = d.take_screenshot() {
                let _ = std::fs::write(format!("temp/{id}/error.png"), img);
            }
            if let Ok(source) = d.get_page_source() {
                let _ = std::fs::write(format!("temp/{id}/source.html"), source);
            }
            panic!("error {}", e);
        }
    };
}

/// 时间戳转换，从1970年开始
pub(crate) fn time_display(value: u64) -> String {
    do_time_display(value, 1970)
}

/// 时间戳转换，支持从不同年份开始计算
pub(crate) fn do_time_display(value: u64, start_year: u64) -> String {
    // 先粗略定位到哪一年
    // 以 365 来计算，年通常只会相比正确值更晚，剩下的秒数也就更多，并且有可能出现需要往前一年的情况

    let per_year_sec = 365 * 24 * 60 * 60; // 平年的秒数

    let mut year = value / per_year_sec;
    // 剩下的秒数，如果这些秒数 不够填补闰年，比如粗略计算是 2024年，还有 86300秒，不足一天，那么中间有很多闰年，所以 年应该-1，只有-1，因为-2甚至更多 需要 last_sec > 365 * 86400，然而这是不可能的
    let last_sec = value - (year) * per_year_sec;
    year += start_year;

    let mut leap_year_sec = 0;
    // 计算中间有多少闰年，当前年是否是闰年不影响回退，只会影响后续具体月份计算
    for y in start_year..year {
        if is_leap(y) {
            // 出现了闰年
            leap_year_sec += 86400;
        }
    }
    if last_sec < leap_year_sec {
        // 不够填补闰年，年份应该-1
        year -= 1;
        // 上一年是闰年，所以需要补一天
        if is_leap(year) {
            leap_year_sec -= 86400;
        }
    }
    // 剩下的秒数
    let mut time = value - leap_year_sec - (year - start_year) * per_year_sec;

    // 平年的月份天数累加
    let mut day_of_year: [u64; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

    // 找到了 计算日期
    let sec = time % 60;
    time /= 60;
    let min = time % 60;
    time /= 60;
    let hour = time % 24;
    time /= 24;

    // 计算是哪天，因为每个月不一样多，所以需要修改
    if is_leap(year) {
        day_of_year[1] += 1;
    }
    let mut month = 0;
    for (index, ele) in day_of_year.iter().enumerate() {
        if &time < ele {
            month = index + 1;
            time += 1; // 日期必须加一，否则 每年的 第 1 秒就成了第0天了
            break;
        }
        time -= ele;
    }

    return format!("{:04}-{:02}-{:02}", year, month, time);
}
//
// 判断是否是闰年
//
fn is_leap(year: u64) -> bool {
    return year % 4 == 0 && ((year % 100) != 0 || year % 400 == 0);
}
