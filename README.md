# insight

`insight` is a web server allowing you to edit markdown articles and host the rendered result publicly.

Articles can be anonymous or "protected" (meaning an email address is associated to the article).

CSS media queries are used to automatically select the UI theme (light/dark), but there is also a theme switch on each page.

### Setup

1. Install [the Rust toolchain](https://rust-lang.org/)
2. Get insight:

```text
$ cargo install insight
```

3. Create required directories:

```text
$ mkdir posts mail
```

> If you cloned the repo, they're already here.

4. Start the server:

```text
$ insight -l 0.0.0.0:9090
```

> This will accept requests from all IP addresses

5. Access the server from a web browser to generate the home page: http://localhost:9090/
6. Edit your home page
7. Save the home page edition link (which is secret) to be able to edit it again later
8. Go to http://localhost:9090/new to create other posts.

### Security considerations

HTML tags are currently stripped from posts at render-time to prevent cross-site scripting vulnerabilities.
We rely on the `pulldown_cmark` crate to detect these tags.
If you manage to get any JS code to execute in the post viewer via markdown, please file an issue because it shouldn't happen.

### License: MIT
