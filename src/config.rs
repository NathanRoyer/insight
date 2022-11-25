use json::parse;
use json::JsonValue;

use lazy_static::lazy_static;

use std::env::args;
use std::process::abort;
use std::path::PathBuf;
use std::fs::canonicalize;
use std::fs::read_to_string;

pub struct Config {
    pub dkim_private_key_path: PathBuf,
    pub dkim_selector: String,
    pub domain_name: String,
    pub articles_dir: PathBuf,
    pub mail_dir: PathBuf,
    pub mail_username: String,
    pub listen_address: String,
    pub new_article: Option<String>,
    pub manage: Option<String>,
    pub home: String,
    pub admin_email: String,
}

lazy_static! {
    pub static ref CONFIG: Config = {
        let mut args = args().rev();
        let config_path = args.next().unwrap_or("".into());
        if let Some("-c") = args.next().as_ref().map(|s| s.as_str()) {
            match parse_config(&config_path) {
                Ok(config) => config,
                Err(e) => {
                    println!("{}", e);
                    abort();
                }
            }
        } else {
            println!("wrong usage: missing -c argument");
            println!("usage: insight -c [config file]");
            println!("       insight -c ./config.json");
            abort();
        }
    };
}

fn try_string(obj: &mut JsonValue, key: &str) -> Result<String, String> {
    obj[key]
        .take_string()
        .ok_or_else(|| format!("config: invalid {}", key))
}

fn parse_config(path: &str) -> Result<Config, String> {
    let config_json = read_to_string(&path).ok()
        .ok_or("Couldn't read config file")?;

    let mut config_obj = parse(&config_json).ok()
        .ok_or("Invalid config file (JSON parsing)")?;

    let dkim_private_key_path = try_string(&mut config_obj, "dkim-private-key-path")?;
    let dkim_selector = try_string(&mut config_obj, "dkim-selector")?;
    let domain_name = try_string(&mut config_obj, "domain-name")?;
    let articles_dir = try_string(&mut config_obj, "articles-dir")?;
    let mail_dir = try_string(&mut config_obj, "mail-dir")?;
    let mail_username = try_string(&mut config_obj, "mail-username")?;
    let listen_address = try_string(&mut config_obj, "listen-address")?;
    let home = try_string(&mut config_obj, "home")?;
    let admin_email = try_string(&mut config_obj, "admin-email")?;

    let new_article = match config_obj["new-article"].is_null() {
        false => Some(try_string(&mut config_obj, "new-article")?),
        true => None,
    };

    let manage = match config_obj["manage"].is_null() {
        false => Some(try_string(&mut config_obj, "manage")?),
        true => None,
    };

    let canonical = canonicalize(path).ok()
        .ok_or("couldn't locate the config file")?;
    let base_path = canonical.parent()
        .ok_or("couldn't locate the config file")?;

    let dkim_private_key_path = base_path.join(dkim_private_key_path);
    let articles_dir = base_path.join(articles_dir);
    let mail_dir = base_path.join(mail_dir);

    Ok(Config {
        dkim_private_key_path,
        dkim_selector,
        domain_name,
        articles_dir,
        mail_dir,
        mail_username,
        listen_address,
        new_article,
        manage,
        home,
        admin_email,
    })
}
