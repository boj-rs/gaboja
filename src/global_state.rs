use crate::data::{Credentials, Problem};
use crate::infra::browser::Browser;

pub(crate) struct GlobalState {
    pub(crate) credentials: Credentials,
    pub(crate) problem: Option<Problem>,
    pub(crate) build: String,
    pub(crate) cmd: String,
    pub(crate) input: String,
    pub(crate) lang: String,
    pub(crate) file: String,
    pub(crate) browser: Browser,
}

impl GlobalState {
    pub(crate) fn new() -> anyhow::Result<Self> {
        Ok(Self {
            credentials: Credentials {
                bojautologin: String::new(),
                onlinejudge: String::new(),
            },
            problem: None,
            build: "cargo build --release".to_string(),
            cmd: "cargo run --release".to_string(),
            input: "input.txt".to_string(),
            lang: "Rust 2021".to_string(),
            file: "src/main.rs".to_string(),
            browser: Browser::new()?
        })
    }

    pub(crate) fn quit(self) -> anyhow::Result<()> {
        self.browser.quit()
    }
}