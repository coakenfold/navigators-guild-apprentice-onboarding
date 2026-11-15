> Pick a Rust crate you haven't used before (try serde, reqwest, or tokio). Find it on docs.rs and crates.io. Read the README. Then ask your agent to explain what the crate does and when you'd use it. Compare the agent's explanation to what you read. Do they match? Is the agent's version accurate and current?

- serde
- [docs.rs](https://docs.rs/serde/latest/serde/)
- [crates.io](https://crates.io/crates/serde)

```
/elihs https://serde.rs/, https://serde.rs/derive.html, https://serde.rs/examples.html
```

```
/elihs visit the example links indexed on this page and summarize https://serde.rs/examples.html
```

Created a skill to find documentation on serde:

```
/serde
```

It has a caching system with a version and stale awareness (3 months old triggers fresh fetches). It labels the sources of the information and flags any content that is from 'memory'.

Opus reviewed and impelemented some improvements that I approved. I then asked it for a generic crate documentation lookup skill that behaved similarily to `/serde` but could look up any crate info:

```
/crates serde
```
