# insight

`insight` is a web server allowing you to edit markdown articles and host the rendered result publicly.

### Features

- customizable homepage (it's an article as well)
- anonymous and email-protected articles (sending emails requires easy DKIM/SPF configuration)
- CSS queries-based light/dark theme selection
- on-disk JSON database → easy backups
- easily dockerized

### To-do

- email templates
- automatic deletion of anonymous posts after configurable duration

### Setup

1. Install [the Rust toolchain](https://rust-lang.org/)
2. Get insight:

```text
$ cargo install insight
```

3. Create `config.json`:

```json
{
    "domain-name": "i.l0.pm",
    "dkim-private-key-path": "dkim.pem",
    "dkim-selector": "insight2022",
    "articles-dir": "articles",
    "mail-dir": "mail",
    "mail-username": "insight",
    "listen-address": "127.0.0.1:9090",
    "new-article": "new",
    "manage": "manage",
    "home": "home",
    "allow-creation": true
}
```

4. Create required directories:

```text
$ mkdir articles mail
```

5. Start the server:

```text
$ insight -c config.json
```

> This will accept requests from all IP addresses

6. Access the server from a web browser to generate the home page: http://localhost:9090/
7. Edit your home page
8. Save the home page edition link (which is secret) to be able to edit it again later
9. Go to http://localhost:9090/new to create other posts.

### Security considerations

HTML tags are currently stripped from posts at render-time to prevent cross-site scripting vulnerabilities.
We rely on the `pulldown_cmark` crate to detect these tags.
If you manage to get any JS code to execute in the post viewer via markdown, please file an issue because it shouldn't happen.

### License: MIT
