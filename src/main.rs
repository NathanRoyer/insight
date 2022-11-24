use tiny_http::Server;
use tiny_http::Request;
use tiny_http::Response;
use tiny_http::Method;
use tiny_http::Header;

use pulldown_cmark::Event;
use pulldown_cmark::Parser;
use pulldown_cmark::Options;
use pulldown_cmark::Tag;
use pulldown_cmark::HeadingLevel;
use pulldown_cmark::html;

use base64::encode;
use html_escape::encode_text as escape;

use json::parse;
use json::object;
use json::JsonValue;

use rand::distributions::Alphanumeric;
use rand::Rng;

use std::sync::Arc;
use std::path::PathBuf;
use std::fs::read_to_string;
use std::fs::write;
use std::fs::metadata;
use std::io::Cursor;
use std::hash::Hasher;
use std::collections::hash_map::DefaultHasher;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use std::thread;

pub mod config;
mod email;
mod templates;

use config::CONFIG;

use email::spawn_email_thread;
use email::Mailer;

use templates::view_template;
use templates::edit_template;
use templates::MANAGE_PAGE;
use templates::NEW_ARTICLE_PAGE;

const INITIAL_MARKDOWN: &'static str = include_str!("initial.md");
const INITIAL_HOMEPAGE: &'static str = include_str!("initial-homepage.md");
const DEFAULT_TITLE: &'static str = "Untitled";
const TOC_HEADINGS_THRESHOLD: usize = 4;

const ONE_MINUTE: u64 = 60;
const FIVE_MINUTES: u64 = ONE_MINUTE * 5;

fn equal(left: &Option<String>, right: &str) -> bool {
    match left {
        Some(left) => left == right,
        None => false,
    }
}

fn is_char_ok(c: char) -> bool {
    if char::is_alphanumeric(c) || c == '-' {
        true
    } else {
        false
    }
}

fn article_path(article_id: &str) -> Option<PathBuf> {
    if equal(&CONFIG.manage, article_id)
    || equal(&CONFIG.new_article, article_id)
    || !article_id.chars().all(is_char_ok)
    || article_id.chars().next() == Some('-')
    || article_id.chars().last() == Some('-')
    || article_id == "" {
        None
    } else {
        let mut buf = CONFIG.articles_dir.join(article_id);
        buf.set_extension("json");
        Some(buf)
    }
}

fn email_path(email: &str) -> PathBuf {
    let mut hasher = DefaultHasher::new();
    hasher.write(email.as_bytes());
    let hash = format!("{:x}", hasher.finish());
    let mut buf = CONFIG.mail_dir.join(hash);
    buf.set_extension("json");
    buf
}

fn now_u64() -> Option<u64> {
    let now = SystemTime::now();
    let since_epoch = now.duration_since(UNIX_EPOCH).ok()?;
    Some(since_epoch.as_secs())
}

fn elapsed_seconds_since(timestamp: u64) -> Option<u64> {
    now_u64()?.checked_sub(timestamp)
}

fn check_and_update(new_json: &str, article_id: &str) -> Option<()> {
    let path = article_path(article_id)?;

    let old_json = read_to_string(&path).ok()?;
    let old_value = parse(&old_json).ok()?;
    let new_value = parse(&new_json).ok()?;

    let new_value_key = new_value["key"].as_str()?;
    let old_value_key = old_value["key"].as_str()?;
    if old_value_key != new_value_key {
        return None;
    }

    let mut clean_value = JsonValue::new_object();
    clean_value["key"] = new_value_key.into();
    clean_value["author"] = old_value["author"].clone();

    let now = now_u64()?;
    let created = old_value["created"].as_u64().unwrap_or(now);
    clean_value["created"] = created.into();
    clean_value["edited"] = now.into();

    let content = new_value["content"].as_str()?;
    clean_value["content"] = content.into();
    clean_value["title"] = {
        let parser = Parser::new(&content);
        let mut title = String::from(DEFAULT_TITLE);

        for event in parser {
            if let Event::Text(cow_str) = event {
                title = cow_str.to_string();
                break;
            }
        }

        title
    }.into();

    write(path, &clean_value.dump()).ok()
}

fn view(article_id: &str) -> Option<String> {
    let path = article_path(article_id)?;

    let content = read_to_string(path).ok()?;
    let value = parse(&content).ok()?;
    let markdown = value["content"].as_str()?;
    let title = value["title"].as_str()?;

    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(&markdown, options);

    let header = |level| match level {
        HeadingLevel::H1 => "h1",
        HeadingLevel::H2 => "h2",
        HeadingLevel::H3 => "h3",
        HeadingLevel::H4 => "h4",
        HeadingLevel::H5 => "h5",
        HeadingLevel::H6 => "h6",
    };

    let mut table = String::new();
    let mut body = String::new();
    let mut n = 0;
    let mut reading_heading = false;
    html::push_html(&mut body, parser.map(|event| {
        match event {
            Event::Html(html) => Event::Text(html),
            Event::Start(Tag::Heading(l, ..)) => {
                reading_heading = true;
                table += &format!("<{}>• ", header(l));
                Event::Html(format!("<{} id=\"h-{}\">", header(l), n).into())
            },
            Event::Text(text) => {
                if reading_heading {
                    let title = escape(&text);
                    table += &format!("<a href=\"#h-{}\">{}</a>", n, title);
                    Event::Html(format!("<a href=\"#h-{}\">{} <div>🔗</div></a>", n, title).into())
                } else {
                    Event::Text(text)
                }
            },
            Event::End(Tag::Heading(l, ..)) => {
                n += 1;
                reading_heading = false;
                table += &format!("</{}>\n", header(l));
                Event::Html(format!("</{}>", header(l)).into())
            },
            _ => event,
        }
    }));

    Some(view_template(&title, &body, match n > TOC_HEADINGS_THRESHOLD {
        true => Some(&table),
        false => None,
    }))
}

fn alphanumeric12() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(12).map(char::from).collect()
}

fn six_digit_code() -> String {
    let mut rng = rand::thread_rng();
    let mut code = String::new();
    for _ in 0..6 {
        code.push(rng.gen_range('0'..='9'));
    }
    code
}

fn create_article(article_id: &str, content: &str) -> Result<String, &'static str> {
    let path = article_path(article_id)
        .ok_or("Invalid article slug")?;

    if metadata(&path).is_ok() {
        return Err("Article slug already taken");
    }

    let key = alphanumeric12();
    let value = object!{
        "key": key.as_str(),
        "content": content,
        "title": DEFAULT_TITLE,
        "created": now_u64(),
        "edited": now_u64(),
    };

    let _ = write(path, &value.dump());
    Ok(format!("/{}/{}", article_id, key))
}

fn edit(article_id: &str, key: &str) -> Option<String> {
    let path = article_path(article_id)?;

    let mut content = read_to_string(&path).ok()?;
    let mut value = parse(&content).ok()?;

    let valid_key;
    let protected;
    if let Some(author) = value["author"].as_str() {
        protected = true;
        let content = read_to_string(email_path(author)).ok()?;
        let mail = parse(&content).ok()?;
        let article_key = mail["articles"][article_id][0].as_str()?;
        let creation = mail["articles"][article_id][1].as_u64()?;

        let elapsed = elapsed_seconds_since(creation)?;
        valid_key = key == article_key && elapsed < ONE_MINUTE;
    } else {
        protected = false;
        valid_key = key == value["key"].as_str()?;
    };

    if valid_key {
        if protected {
            // regen key
            value["key"] = alphanumeric12().into();
            content = value.dump();
            write(&path, &content).ok()?;
        }

        Some(edit_template(&content))
    } else {
        None
    }
}

fn response(content: &str, content_type: &str, code: u32) -> Response<Cursor<Vec<u8>>> {
    let header = Header::from_bytes("Content-Type", content_type).unwrap();
    Response::from_string(content)
        .with_status_code(code)
        .with_header(header)
}

fn redirect(location: &str) -> Response<Cursor<Vec<u8>>> {
    let header = Header::from_bytes("Location", location).unwrap();
    Response::from_string("Redirecting...")
        .with_status_code(302)
        .with_header(header)
}

fn handle_article_update(body: String, article_id: &str) -> Option<()> {
    let json = body;
    check_and_update(&json, article_id)
}

fn send_email_code(body: String, mailer: &Mailer, create: bool) -> Option<String> {
    let email = body;
    let path = email_path(&email);

    let json = if let Ok(contents) = read_to_string(&path) {
        contents
    } else if create {
        object!{
            "email": email.as_str(),
            "code": "000000",
            "code-created": 0u64,
            "token": "",
            "articles": {},
        }.dump()
    } else {
        return None;
    };

    let mut value = parse(&json).ok()?;
    let timestamp = value["code-created"].as_u64()?;

    let elapsed = elapsed_seconds_since(timestamp)?;
    if elapsed > FIVE_MINUTES {
        let code = six_digit_code();

        value["code-created"] = now_u64()?.into();
        value["code"] = code.as_str().into();
        write(path, &value.dump()).ok()?;

        mailer.send((email, code)).ok()?;

        Some("Code sent; check your spams".into())
    } else {
        Some("Too early to resend a code; wait 5 minutes".into())
    }
}

fn check_email_code(body: String) -> Option<String> {
    let code = body.get(..6)?;
    let email = body.get(6..)?;

    let path = email_path(&email);
    let json = read_to_string(&path).ok()?;
    let mut value = parse(&json).ok()?;
    let actual_code = value["code"].as_str()?;
    let timestamp = value["code-created"].as_u64()?;

    let elapsed = elapsed_seconds_since(timestamp)?;
    if elapsed < FIVE_MINUTES && code == actual_code {
        value["code-created"] = 0u64.into();

        let token = alphanumeric12();
        value["token"] = token.as_str().into();

        write(path, &value.dump()).ok()?;
        Some(token)
    } else {
        None
    }
}

fn list_articles(body: String) -> Option<String> {
    let token = body.get(  ..12)?;
    let email = body.get(12..  )?;

    let json = read_to_string(&email_path(&email)).ok()?;
    let value = parse(&json).ok()?;
    let actual_token = value["token"].as_str()?;

    if token == actual_token {
        let articles = &value["articles"];
        let mut output = String::new();

        for (article_id, _) in articles.entries() {
            let path = article_path(article_id)?;
            let json = read_to_string(path).ok()?;
            let article = parse(&json).ok()?;
            let title = article["title"].as_str()?;

            output += article_id;
            output += ":";
            output += &encode(title);
            output += "\n";
        }
        let _ = output.pop();

        Some(output)
    } else {
        None
    }
}

fn protect_article(body: String, article_id: &str) -> Option<String> {
    let key     = body.get(  ..12)?;
    let token   = body.get(12..24)?;
    let email   = body.get(24..)?;

    let mail_path = email_path(&email);
    let article_path = article_path(&article_id)?;

    let article = read_to_string(&article_path).ok()?;
    let mail = read_to_string(&mail_path).ok()?;
    let mut article = parse(&article).ok()?;
    let mut mail = parse(&mail).ok()?;

    let actual_key = article["key"].as_str()?;
    let actual_token = mail["token"].as_str()?;

    if token == actual_token && key == actual_key {
        article["author"] = email.into();
        article["key"] = JsonValue::Null;

        let key = alphanumeric12();
        mail["articles"][article_id] = [
            key.as_str().into(),
            JsonValue::from(now_u64()?),
        ].as_slice().into();

        write(article_path, &article.dump()).ok()?;
        write(mail_path, &mail.dump()).ok()?;

        Some(key)
    } else {
        None
    }
}

fn get_edit_link(body: String, article: &str) -> Option<String> {
    let token   = body.get(  ..12)?;
    let email   = body.get(12..  )?;

    let mail_path = email_path(&email);
    let mail = read_to_string(&mail_path).ok()?;
    let mut mail = parse(&mail).ok()?;

    let actual_token = mail["token"].as_str()?;
    let owns_that_article = mail["articles"][article].is_array();

    if token == actual_token && owns_that_article {
        let key = alphanumeric12();
        mail["articles"][article] = [
            key.as_str().into(),
            JsonValue::from(now_u64()?),
        ].as_slice().into();

        write(mail_path, &mail.dump()).ok()?;
        Some(key)
    } else {
        None
    }
}

fn handle_request(mut request: Request, mailer: &Mailer) {
    let mut body = String::new();
    let _ = request.as_reader().read_to_string(&mut body);

    let url = request.url();
    let url = url.split("?").next().unwrap();

    let path: Vec<_> = url.split("/")
        .filter(|e| e.len() > 0)
        .collect();

    let bad_request = response("Bad Request", "text", 400);

    let response = match request.method() {
        Method::Get => match path.len() {
            2 => {
                let article = path[0];
                let key = path[1];
                match edit(article, key) {
                    Some(body) => response(&body, "text/html", 200),
                    None => bad_request,
                }
            },
            1 => {
                let article = path[0];

                if equal(&CONFIG.new_article, article) {
                    response(&NEW_ARTICLE_PAGE, "text/html", 200)
                } else if equal(&CONFIG.manage, article) {
                    response(&MANAGE_PAGE, "text/html", 200)
                } else {
                    match view(article) {
                        Some(body) => response(&body, "text/html", 200),
                        None => if article == CONFIG.home {
                            let homepage = create_article(article, INITIAL_HOMEPAGE);
                            redirect(&homepage.unwrap())
                        } else {
                            bad_request
                        },
                    }
                }
            },
            0 => redirect(&CONFIG.home),
            _ => bad_request,
        }
        Method::Post => match path.get(0) {
            Some(&"create") => match create_article(&body, INITIAL_MARKDOWN) {
                Ok(body) => response(&body, "text", 200),
                Err(body) => response(&body, "text", 400),
            },
            Some(&"send-email-code") => match send_email_code(body, mailer, false) {
                Some(body) => response(&body, "text", 200),
                None => bad_request,
            },
            Some(&"send-email-code-create") => match send_email_code(body, mailer, true) {
                Some(body) => response(&body, "text", 200),
                None => bad_request,
            },
            Some(&"check-email-code") => match check_email_code(body) {
                Some(body) => response(&body, "text", 200),
                None => bad_request,
            },
            Some(&"list-articles") => match list_articles(body) {
                Some(body) => response(&body, "text", 200),
                None => bad_request,
            },
            _ => match path.get(1) {
                Some(&"update") => match handle_article_update(body, path[0]) {
                    Some(_) => response("OK", "text", 200),
                    None => bad_request,
                },
                Some(&"protect") => match protect_article(body, path[0]) {
                    Some(body) => response(&body, "text", 200),
                    None => bad_request,
                },
                Some(&"get-edit-link") => match get_edit_link(body, path[0]) {
                    Some(body) => response(&body, "text", 200),
                    None => bad_request,
                },
                _ => bad_request,
            },
        },
        _ => bad_request,
    };

    let _ = request.respond(response);
}

fn main() {
    let server = Server::http(&CONFIG.listen_address).unwrap();
    let server = Arc::new(server);
    let mut guards = Vec::with_capacity(5);

    let (mail_thread, mail_sender) = spawn_email_thread();
    guards.push(mail_thread);

    for _ in 0..4 {
        let server = server.clone();
        let mailer = mail_sender.clone();

        let guard = thread::spawn(move || {
            loop {
                let request = server.recv().unwrap();
                handle_request(request, &mailer);
            }
        });

        guards.push(guard);
    }

    for guard in guards {
        let _ = guard.join();
    }
}
