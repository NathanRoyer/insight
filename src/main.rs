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
use std::ops::Deref;
use std::io::Cursor;
use std::thread;

fn post_path(post: &str) -> String {
    format!("posts/{}.json", post)
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
        clean_value["post"] = post.into();

        let content = new_value["content"].as_str()?;
        clean_value["content"] = content.into();

        write(post_path(post), &clean_value.dump()).ok()
    } else {
        None
    }
}

const STYLESHEET: &'static str = include_str!("style.css");
const EDITOR_SCRIPT: &'static str = include_str!("editor.js");
const INITIAL_MARKDOWN: &'static str = include_str!("initial.md");
const INITIAL_HOMEPAGE: &'static str = include_str!("initial-homepage.md");

fn view(post: &str) -> Option<String> {
    if post.chars().all(char::is_alphanumeric) {
        let content = read_to_string(post_path(post)).ok()?;
        let value = parse(&content).ok()?;
        let markdown = value["content"].as_str()?;

        let mut options = Options::empty();
        options.insert(Options::ENABLE_STRIKETHROUGH);
        let parser = Parser::new_ext(&markdown, options);

        let mut title: Option<String> = None;

        let mut body = String::new();
        html::push_html(&mut body, parser.map(|event| {
        	if let Event::Html(_) = event {
        		Event::Text(CowStr::Borrowed("[ stripped HTML tag ]"))
        	} else if let (None, Event::Text(string)) = (&title, &event) {
        		title = Some(String::from(string.deref()));
        		event
        	} else {
        		event
        	}
        }));

		let title = title.unwrap_or("Post".into());
        let response = format!(r#"<!DOCTYPE html>
<html>
    <head>
        <meta charset="utf-8">
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <title>{}</title>
        <style>{}</style>
    </head>
    <body id="viewer">{}</body>
</html>"#,
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

fn create_post(post: &str, content: &str) -> String {
    let path = post_path(post);

    let key = alphanumeric12();
    let value = object!{
        "key": key.as_str(),
        "content": content,
        "post": post,
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
        if let Ok(content) = read_to_string(post_path(post)) {
            let value = parse(&content).unwrap();
            let valid_key = value["key"].as_str().unwrap();

            if key == valid_key {
                let response = format!(r#"<!DOCTYPE html>
<html>
    <head>
        <meta charset="utf-8">
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <title>Editor - i.l0.pm</title>
        <style>{}</style>
    </head>
    <body onload="init();">
        <script>let post = '{}';</script>
        <script>{}</script>
        <div id="editor">
            <button id="title-button">Title</button>
            <button id="image-button">Image</button>
            <button id="link-button">Link</button>
            <button id="list-button">List</button>
            <button id="view-button">View ⬀</button>
        </div>
        <textarea id="markdown"></textarea>
    </body>
</html>"#,
                    STYLESHEET,
                    &encode(&content),
                    EDITOR_SCRIPT,
                );
                return Some(response);
            }
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

fn handle_post_request(request: &mut Request) -> Option<()> {
    let mut json = String::new();
    request.as_reader().read_to_string(&mut json).unwrap();

    check_and_update(&json)
}

fn handle_request(mut request: Request) {
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
        Method::Post => match handle_post_request(&mut request) {
            Some(_) => response("OK", "text", 200),
            None => bad_request,
        },
        _ => bad_request,
    };

    let _ = request.respond(response);
}

fn main() {
    let mut args = args().rev();
    let address = args.next().unwrap_or("".into());
    if let Some("-l") = args.next().as_ref().map(|s| s.as_str()) {
        let server = Server::http(address).unwrap();
        let server = Arc::new(server);
        let mut guards = Vec::with_capacity(4);

        for _ in 0..4 {
            let server = server.clone();

            let guard = thread::spawn(move || {
                loop {
                    let request = server.recv().unwrap();
                    handle_request(request);
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
