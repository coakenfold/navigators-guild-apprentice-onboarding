> Introduce a version mismatch on purpose. Ask your agent to use an old pattern from a library (it will probably do this naturally if you just ask "use [library]" without specifying a version). When it breaks, find the changelog, identify what changed, and use the current documentation to fix it.

Prompt:

````
/crates reqwest help me access the result from `Client::new`, eg:
```

let client = reqwest::Client::new()?;
let res = client.post("http://httpbin.org/post")?
    .body("the exact body that is sent")
    .send()?;

```
````

Response:

````
Your snippet matches an older reqwest API (about 0.8). In current reqwest (0.13.5), `Client::new()` doesn't return a `Result`, so the `?` on it won't compile.

## What changed

- **`Client::new()`** returns a `Client` directly. *[docs: https://docs.rs/reqwest/0.13.5/reqwest/struct.Client.html]*
  - `Client::builder()...build()` is the fallible route. It returns `Result<Client>`, for example when the TLS backend can't initialise. *[summary: reqwest@0.13.5, checked 2026-11-09]*
- **`client.post(url)`** returns a `RequestBuilder`, not a `Result`. It has no `?` either. *[docs: https://docs.rs/reqwest/0.13.5/reqwest/struct.Client.html]*
- **`.body(...)`** takes `T: Into<Body>` and returns the builder. *[docs: https://docs.rs/reqwest/0.13.5/reqwest/struct.RequestBuilder.html]*
- **`.send()`** is async. It returns a future that resolves to `Result<Response, Error>`, so you need `.await?`. *[docs: https://docs.rs/reqwest/0.13.5/reqwest/struct.RequestBuilder.html]*
- **Reading the result** takes more async calls on the `Response`. `.status()` is synchronous. `.text()` and `.json::<T>()` are async and consume the response. `.json()` needs the `json` feature. *[docs: https://docs.rs/reqwest/0.13.5/reqwest/struct.Response.html]*

## Async version (default)

This is code I composed and haven't compiled or run. It needs `tokio` as well:

```rust
#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    let client = reqwest::Client::new();
    let res = client
        .post("http://httpbin.org/post")
        .body("the exact body that is sent")
        .send()
        .await?;

    println!("status: {}", res.status());
    let body = res.text().await?;
    println!("body: {body}");
    Ok(())
}
```

Cargo.toml:
```toml
reqwest = "0.13"
tokio = { version = "1", features = ["full"] }
```

## Blocking version (closest to your snippet)

The `blocking` module has the same shape as your original, with no async. It needs the `blocking` feature. *[summary: reqwest@0.13.5, checked 2026-11-09]* The blocking `Client::new()` and `send()` match the shape above, but I didn't check their exact signatures. *[from memory, unverified]*

```rust
fn main() -> Result<(), reqwest::Error> {
    let client = reqwest::blocking::Client::new();
    let res = client
        .post("http://httpbin.org/post")
        .body("the exact body that is sent")
        .send()?;

    println!("status: {}", res.status());
    println!("body: {}", res.text()?);
    Ok(())
}
```

Cargo.toml:
```toml
reqwest = { version = "0.13", features = ["blocking"] }
```

I can check the blocking signatures against the docs, or compile and run either version, if you'd like.

**Sources**
- docs: Client, RequestBuilder and Response pages on docs.rs, reqwest 0.13.5.
- summary: reqwest@0.13.5, checked 2026-11-09.
- from memory, unverified: blocking `Client::new` and `send` signatures.
````
