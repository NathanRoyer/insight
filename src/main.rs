use tiny_http::Server;
use tiny_http::Request;
use tiny_http::Response;
use tiny_http::Method;
use tiny_http::Header;

use pulldown_cmark::CowStr;
use pulldown_cmark::Event;
use pulldown_cmark::Parser;
use pulldown_cmark::Options;
use pulldown_cmark::html;

use html_escape::encode_text;
use base64::encode;
use lazy_static::lazy_static;

use json::parse;
use json::object;
use json::JsonValue;

use rand::distributions::Alphanumeric;
use rand::Rng;

use std::env::args;
use std::sync::Arc;
use std::fs::read_to_string;
use std::fs::write;
use std::fs::metadata;
use std::io::Cursor;
use std::hash::Hasher;
use std::collections::hash_map::DefaultHasher;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use std::thread;

mod email;

use email::spawn_email_thread;
use email::Mailer;

pub const DKIM_PRIVATE_KEY_PATH: &'static str = "mail/dkim-private-key.pem";
pub const DNS_TXT_PATH: &'static str = "mail/dns.txt";
pub const DOMAIN_NAME: &'static str = "i.l0.pm";
pub const DKIM_SELECTOR: &'static str = "insight2022";

const STYLESHEET: &'static str = include_str!("style.css");
const SVG_FAVICON: &'static str = include_str!("favicon.svg");
const COMMON_SCRIPT: &'static str = include_str!("common.js");
const EDITOR_SCRIPT: &'static str = include_str!("editor.js");
const MANAGER_SCRIPT: &'static str = include_str!("manager.js");
const INITIAL_MARKDOWN: &'static str = include_str!("initial.md");
const INITIAL_HOMEPAGE: &'static str = include_str!("initial-homepage.md");

const ONE_MINUTE: u64 = 60;
const FIVE_MINUTES: u64 = ONE_MINUTE * 5;

lazy_static! {
    static ref SVG_FAVICON_B64: String = encode(SVG_FAVICON);
    static ref MANAGE_PAGE: String = format!(r#"<!DOCTYPE html>
<html>
    <head>
        <meta charset="utf-8">
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <link rel="icon" type="image/x-icon" href="data:image/svg+xml;base64,{}">
        <title>Manage your posts</title>
        <style>{}</style>
        <script>{}</script>
        <script>{}</script>
    </head>
    <body onload="init()">
        <input type="checkbox" id="theme-checkbox" name="theme-checkbox">
        <div id="themed">
            <div id="auth">
                <p id="status">
                    Posts can be protected with an email address.
                    If you have protected posts with your email address,
                    enter it below and follow the procedure to get
                    access to the posts you protected.
                </p>
                <div>
                    <div>
                        <input type="email" id="email-field" placeholder="email" />
                        <input type="text" id="code-field" placeholder="123456" />
                    </div>
                    <div>
                        <button id="check-button">Check</button>
                        <button id="submit-button">Submit</button>
                    </div>
                </div>
            </div>
            <div id="centered" class="viewer">
                <h1>Manage your posts</h1>
                <p id="status">Be sure to allow popups from this page.</p>
                <ul id="post-list"></ul>
                <button id="list-posts-button">Refresh list</button>
                <div id="spacer"></div>
                <p>[powered by <a href="https://lib.rs/crates/insight">insight</a>]</p>
            </div>
        </div>
    </body>
</html>"#,
        SVG_FAVICON_B64.as_str(),
        STYLESHEET,
        COMMON_SCRIPT,
        MANAGER_SCRIPT,
    );
}

fn post_path(post: &str) -> String {
    format!("posts/{}.json", post)
}

fn email_path(email: &str) -> String {
    let mut hasher = DefaultHasher::new();
    hasher.write(email.as_bytes());
    format!("mail/{:x}.json", hasher.finish())
}

fn now_u64() -> Option<u64> {
    let now = SystemTime::now();
    let since_epoch = now.duration_since(UNIX_EPOCH).ok()?;
    Some(since_epoch.as_secs())
}

fn elapsed_seconds_since(timestamp: u64) -> Option<u64> {
    now_u64()?.checked_sub(timestamp)
}

fn check_and_update(new_json: &str) -> Option<()> {
    let new_value = parse(&new_json).ok()?;
    let post = new_value["post"].as_str()?;

    if post.chars().all(char::is_alphanumeric) {
        let old_json = read_to_string(post_path(post)).ok()?;
        let old_value = parse(&old_json).ok()?;

        let new_value_key = new_value["key"].as_str()?;
        let old_value_key = old_value["key"].as_str()?;
        if old_value_key != new_value_key {
            return None;
        }

        let mut clean_value = JsonValue::new_object();
        clean_value["key"] = new_value_key.into();
        clean_value["author"] = old_value["author"].clone();
        clean_value["post"] = post.into();

        let content = new_value["content"].as_str()?;
        clean_value["content"] = content.into();
        clean_value["title"] = {
            let parser = Parser::new(&content);
            let mut title = String::from("Untitled");

            for event in parser {
                if let Event::Text(cow_str) = event {
                    title = cow_str.to_string();
                    break;
                }
            }

            title
        }.into();

        write(post_path(post), &clean_value.dump()).ok()
    } else {
        None
    }
}

fn view(post: &str) -> Option<String> {
    if post.chars().all(char::is_alphanumeric) {
        let content = read_to_string(post_path(post)).ok()?;
        let value = parse(&content).ok()?;
        let markdown = value["content"].as_str()?;
        let title = value["title"].as_str()?;

        let mut options = Options::empty();
        options.insert(Options::ENABLE_STRIKETHROUGH);
        let parser = Parser::new_ext(&markdown, options);

        let mut body = String::new();
        html::push_html(&mut body, parser.map(|event| {
            match event {
                Event::Html(_) => Event::Text(CowStr::Borrowed("[removed HTML]")),
                _ => event,
            }
        }));

        let response = format!(r#"<!DOCTYPE html>
<html>
    <head>
        <meta charset="utf-8">
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <link rel="icon" type="image/x-icon" href="data:image/svg+xml;base64,{}">
        <title>{}</title>
        <style>{}</style>
    </head>
    <body>
        <input type="checkbox" id="theme-checkbox" name="theme-checkbox">
        <div id="themed">
            <div id="centered" class="viewer">
                {}
                <div id="spacer"></div>
                <p>[powered by <a href="https://lib.rs/crates/insight">insight</a>]</p>
            </div>
        </div>
    </body>
</html>"#,
            SVG_FAVICON_B64.as_str(),
            encode_text(&title),
            STYLESHEET,
            body,
        );
        return Some(response);
    }
    
    None
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

fn create_post(post: &str, content: &str) -> String {
    let path = post_path(post);

    let key = alphanumeric12();
    let value = object!{
        "key": key.as_str(),
        "content": content,
        "post": post,
        "title": "Untitled",
    };

    let _ = write(path, &value.dump());
    format!("/{}/{}", post, key)
}

fn new_post() -> String {
    let mut post: String;

    loop {
        post = alphanumeric12();
        if let Err(_) = metadata(&post_path(&post)) {
            break;
        }
    }

    create_post(&post, INITIAL_MARKDOWN)
}

fn edit(post: &str, key: &str) -> Option<String> {
    if post.chars().all(char::is_alphanumeric) {
        let post_path = post_path(post);
        let mut content = read_to_string(&post_path).ok()?;
        let mut value = parse(&content).ok()?;

        let valid_key;
        let protected;
        if let Some(author) = value["author"].as_str() {
            protected = true;
            let content = read_to_string(email_path(author)).ok()?;
            let mail = parse(&content).ok()?;
            let post_key = mail["posts"][post][0].as_str()?;
            let creation = mail["posts"][post][1].as_u64()?;

            let elapsed = elapsed_seconds_since(creation)?;
            valid_key = key == post_key && elapsed < ONE_MINUTE;
        } else {
            protected = false;
            valid_key = key == value["key"].as_str()?;
        };

        if valid_key {
            if protected {
                // regen key
                value["key"] = alphanumeric12().into();
                content = value.dump();
                write(&post_path, &content).ok()?;
            }

            let response = format!(r#"<!DOCTYPE html>
<html>
    <head>
        <meta charset="utf-8">
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <link rel="icon" type="image/x-icon" href="data:image/svg+xml;base64,{}">
        <title>Editor - i.l0.pm</title>
        <style>{}</style>
    </head>
    <body onload="init();">
        <script>let post = '{}';</script>
        <script>{}</script>
        <script>{}</script>
        <input type="checkbox" id="theme-checkbox" name="theme-checkbox">
        <div id="themed">
            <div id="auth" class="hidden">
                <p id="status">
                    Posts can be protected with an email address.
                    Enter your email address to protect the post.
                    Protected posts are not automatically deleted
                    and their edit links are short-lived. You can
                    manage your protected posts from the
                    <a href="/manage">Manage</a> page.
                </p>
                <div>
                    <div>
                        <input type="email" id="email-field" placeholder="email" />
                        <input type="text" id="code-field" placeholder="123456" />
                    </div>
                    <div>
                        <button id="check-button">Check</button>
                        <button id="submit-button">Submit</button>
                    </div>
                </div>
            </div>
            <div id="centered">
                <div id="editor">
                    <button id="protect-button">Protect</button>
                    <button id="view-button">View ⬀</button>
                </div>
                <textarea id="markdown"></textarea>
            </div>
        </div>
    </body>
</html>"#,
                SVG_FAVICON_B64.as_str(),
                STYLESHEET,
                &encode(&content),
                COMMON_SCRIPT,
                EDITOR_SCRIPT,
            );
            return Some(response);
        }
    }
    
    None
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

fn handle_post_update(body: String) -> Option<()> {
    let json = body;
    check_and_update(&json)
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
            "posts": {},
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

fn list_posts(body: String) -> Option<String> {
    let token = body.get(  ..12)?;
    let email = body.get(12..  )?;

    let json = read_to_string(&email_path(&email)).ok()?;
    let value = parse(&json).ok()?;
    let actual_token = value["token"].as_str()?;

    if token == actual_token {
        let posts = &value["posts"];
        let mut output = String::new();

        for (post_id, _) in posts.entries() {
            let json = read_to_string(&post_path(post_id)).ok()?;
            let post = parse(&json).ok()?;
            let title = post["title"].as_str()?;

            output += post_id;
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

fn protect_post(body: String, post_id: &str) -> Option<String> {
    let key     = body.get(  ..12)?;
    let token   = body.get(12..24)?;
    let email   = body.get(24..)?;

    let mail_path = email_path(&email);
    let post_path = post_path(&post_id);

    let post = read_to_string(&post_path).ok()?;
    let mail = read_to_string(&mail_path).ok()?;
    let mut post = parse(&post).ok()?;
    let mut mail = parse(&mail).ok()?;

    let actual_key = post["key"].as_str()?;
    let actual_token = mail["token"].as_str()?;

    if token == actual_token && key == actual_key {
        post["author"] = email.into();
        post["key"] = JsonValue::Null;

        let key = alphanumeric12();
        mail["posts"][post_id] = [
            key.as_str().into(),
            JsonValue::from(now_u64()?),
        ].as_slice().into();

        write(post_path, &post.dump()).ok()?;
        write(mail_path, &mail.dump()).ok()?;

        Some(key)
    } else {
        None
    }
}

fn get_edit_link(body: String, post: &str) -> Option<String> {
    let token   = body.get(  ..12)?;
    let email   = body.get(12..  )?;

    let mail_path = email_path(&email);
    let mail = read_to_string(&mail_path).ok()?;
    let mut mail = parse(&mail).ok()?;

    let actual_token = mail["token"].as_str()?;
    let owns_that_post = mail["posts"][post].is_array();

    if token == actual_token && owns_that_post {
        let key = alphanumeric12();
        mail["posts"][post] = [
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
                let post = path[0];
                let key = path[1];
                match edit(post, key) {
                    Some(body) => response(&body, "text/html", 200),
                    None => bad_request,
                }
            },
            1 => {
                let post = path[0];
                if post == "new" {
                    redirect(&new_post())
                } else if post == "manage" {
                    response(&MANAGE_PAGE, "text/html", 200)
                } else {
                    match view(post) {
                        Some(body) => response(&body, "text/html", 200),
                        None => if post == "home" {
                            redirect(&create_post(post, INITIAL_HOMEPAGE))
                        } else {
                            bad_request
                        },
                    }
                }
            },
            0 => redirect("/home"),
            _ => bad_request,
        }
        Method::Post => match path.get(0) {
            Some(&"update") => match handle_post_update(body) {
                Some(_) => response("OK", "text", 200),
                None => bad_request,
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
            Some(&"list-posts") => match list_posts(body) {
                Some(body) => response(&body, "text", 200),
                None => bad_request,
            },
            _ => match path.get(1) {
                Some(&"protect") => match protect_post(body, path[0]) {
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
    let mut args = args().rev();
    let address = args.next().unwrap_or("".into());
    if let Some("-l") = args.next().as_ref().map(|s| s.as_str()) {
        let posts_dir = metadata("posts");
        let mail_dir = metadata("mail");

        if posts_dir.is_err() || mail_dir.is_err() {
            println!("Error: cannot find ./posts, ./mail or both directories");
            println!("Please create them manually");
        }

        let server = Server::http(address).unwrap();
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
                    // sender.send(("lolatesy5644@gmail.com".into(), "876345".into())).unwrap();
                    handle_request(request, &mailer);
                }
            });

            guards.push(guard);
        }

        for guard in guards {
            let _ = guard.join();
        }
    } else {
        println!("wrong usage: missing -l argument");
        println!("usage: insight -l address:port");
        println!("       insight -l 0.0.0.0:9090");
    }
}
