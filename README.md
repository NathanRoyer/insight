# insight

`insight` is a web server allowing you to edit markdown posts and host the rendered result publicly.

It has less than 40 dependencies in total.

CSS media queries are used to automatically select the UI theme (light/dark), but there is also a theme switch on each page.

### Setup

1. Install [the Rust toolchain](https://rust-lang.org/)
2. Get insight:

```text
$ cargo install insight
```

3. Start the server:

```text
$ insight -l 0.0.0.0:9090
```

> This will accept requests from all IP addresses

4. Access the server from a web browser to generate the home page: http://localhost:9090/
5. Edit your home page
6. Save the home page edition link (which is secret) to be able to edit it again later
7. Go to http://localhost:9090/new to create other posts.

### Security considerations

The fact that secret edition keys are parts of the URL is bad.
If you see any alternative that doesn't require user accounts or entering a password, please file an issue on GitHub.

HTML tags are currently stripped from posts at render-time to prevent cross-site scripting vulnerabilities.
We rely on the `pulldown_cmark` crate to detect these tags.
If you manage to get any JS code to execute in the post viewer via markdown, please file an issue because it shouldn't happen.

### License: MIT
