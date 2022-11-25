use base64::encode;
use html_escape::encode_text as escape;
use lazy_static::lazy_static;

const STYLESHEET: &'static str = include_str!("style.css");
const SVG_FAVICON: &'static str = include_str!("favicon.svg");
const COMMON_SCRIPT: &'static str = include_str!("common.js");
const EDITOR_SCRIPT: &'static str = include_str!("editor.js");
const MANAGER_SCRIPT: &'static str = include_str!("manager.js");
const NEW_ARTICLE_SCRIPT: &'static str = include_str!("new-article.js");

lazy_static! {
    static ref SVG_FAVICON_B64: String = encode(SVG_FAVICON);
    pub static ref MANAGE_PAGE: String = format!(r#"<!DOCTYPE html>
<html>
    <head>
        <meta charset="utf-8">
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <link rel="icon" type="image/x-icon" href="data:image/svg+xml;base64,{}">
        <title>Manage your articles</title>
        <style>{}</style>
        <script>{}</script>
        <script>{}</script>
    </head>
    <body onload="init()">
        <input type="checkbox" id="theme-checkbox" name="theme-checkbox">
        <div id="themed">
            <div id="popup" class="manage">
                <p id="status">
                    Articles can be protected with an email address.
                    If you have protected articles with your email address,
                    enter it below and follow the procedure to get
                    access to the articles you protected.
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
                <h1>Manage your articles</h1>
                <p id="status">Be sure to allow popups from this page.</p>
                <ul id="article-list"></ul>
                <h3 id="anon-article-title" class="hidden">Anonymous articles on your server:</h3>
                <ul id="anon-article-list"></ul>
                <button id="list-articles-button">Refresh list</button>
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

    pub static ref NEW_ARTICLE_PAGE: String = format!(r#"<!DOCTYPE html>
<html>
    <head>
        <meta charset="utf-8">
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <link rel="icon" type="image/x-icon" href="data:image/svg+xml;base64,{}">
        <title>New Article</title>
        <style>{}</style>
        <script>{}</script>
        <script>{}</script>
    </head>
    <body onload="init()">
        <input type="checkbox" id="theme-checkbox" name="theme-checkbox">
        <div id="themed">
            <div id="popup">
                <p id="status">
                    Choose the <a href="https://en.wikipedia.org/wiki/Clean_URL#Slug">slug</a> for your article.
                    You can use characters a-z, A-Z, 1-9 and simple dashes. The slug cannot begin or end with a dash.
                </p>
                <div>
                    <div>
                        <input type="text" pattern="^[a-zA-Z1-9]([a-zA-Z1-9\-]*[a-zA-Z1-9])?$" id="article-id-field" placeholder="hogwarts-corruption-report" />
                    </div>
                    <div>
                        <button id="create-button">Create</button>
                    </div>
                </div>
            </div>
            <div id="centered" class="viewer">
            </div>
        </div>
    </body>
</html>"#,
        SVG_FAVICON_B64.as_str(),
        STYLESHEET,
        COMMON_SCRIPT,
        NEW_ARTICLE_SCRIPT,
    );
}

pub fn view_template(title: &str, body: &str, table_of_contents: Option<&str>) -> String {
    let class = match table_of_contents.is_some() {
        true => "",
        false => "hidden",
    };

    let title = escape(title);
    format!(r#"<!DOCTYPE html>
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
                <h1><a href="{}centered">{}</a></h1>
                <span id="table-of-contents" class="{}"><a href="{}table-of-contents">[Table of Contents]</a></span>
                <hr>
                <div>{}</div>
                <hr>
                {}
                <div id="spacer"></div>
                <p>[powered by <a href="https://lib.rs/crates/insight">insight</a>]</p>
            </div>
        </div>
    </body>
</html>"#,
        SVG_FAVICON_B64.as_str(),
        &title,
        STYLESHEET,
        "#",
        &title,
        class,
        "#",
        table_of_contents.unwrap_or(""),
        body,
    )
}

pub fn edit_template(content: &str) -> String {
    format!(r#"<!DOCTYPE html>
<html>
    <head>
        <meta charset="utf-8">
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <link rel="icon" type="image/x-icon" href="data:image/svg+xml;base64,{}">
        <title>Editor - i.l0.pm</title>
        <style>{}</style>
    </head>
    <body onload="init();">
        <script>let article = '{}';</script>
        <script>{}</script>
        <script>{}</script>
        <input type="checkbox" id="theme-checkbox" name="theme-checkbox">
        <div id="themed">
            <div id="popup" class="hidden">
                <p id="status">
                    Articles can be protected with an email address.
                    Enter your email address to protect the article.
                    Protected articles are not automatically deleted
                    and their edit links are short-lived. You can
                    manage your protected articles from the
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
                    <button id="delete-button">Delete</button>
                </div>
                <textarea id="markdown"></textarea>
            </div>
        </div>
    </body>
</html>"#,
        SVG_FAVICON_B64.as_str(),
        STYLESHEET,
        encode(content),
        COMMON_SCRIPT,
        EDITOR_SCRIPT,
    )
}
