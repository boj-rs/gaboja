use crate::data::{BojConfig, Credentials, Preset, Problem, ProblemId};
use crate::infra::browser::Browser;
use std::collections::HashMap;

pub(crate) struct GlobalState {
    pub(crate) credentials: Credentials,
    pub(crate) problem: Option<Problem>,
    pub(crate) init: String,
    pub(crate) build: String,
    pub(crate) cmd: String,
    pub(crate) input: String,
    pub(crate) lang: String,
    pub(crate) file: String,
    pub(crate) browser: Browser,
    pub(crate) problem_cache: HashMap<ProblemId, Problem>,
    pub(crate) presets: HashMap<String, Preset>,
}

impl GlobalState {
    pub(crate) fn new() -> anyhow::Result<Self> {
        let mut state = Self {
            credentials: Credentials {
                bojautologin: String::new(),
                onlinejudge: String::new(),
            },
            problem: None,
            init: String::new(),
            build: "cargo build --release".to_string(),
            cmd: "cargo run --release".to_string(),
            input: "input.txt".to_string(),
            lang: "Rust 2021".to_string(),
            file: "src/main.rs".to_string(),
            browser: Browser::new()?,
            problem_cache: HashMap::new(),
            presets: HashMap::new(),
        };
        // println!("state initialized");
        match BojConfig::from_config() {
            Ok(config) => {
                for preset in &config.preset {
                    state.presets.insert(preset.name.clone(), preset.clone());
                }
                if let Some(start) = config.start.as_ref() {
                    for (lineno, line) in start.lines().enumerate() {
                        if line.is_empty() {
                            continue;
                        }
                        match line.parse::<crate::InputCommand>() {
                            Ok(cmd) => {
                                if let Err(err) = state.execute(&cmd) {
                                    println!(
                                        "boj.toml start script execution error at line {}: {}",
                                        lineno + 1,
                                        err
                                    );
                                    break;
                                }
                            }
                            Err(err) => {
                                println!(
                                    "boj.toml start script parse error at line {}: {}",
                                    lineno + 1,
                                    err
                                );
                                break;
                            }
                        }
                    }
                }
            }
            Err(_error) => {}
        }
        Ok(state)
    }

    pub(crate) fn quit(self) -> anyhow::Result<()> {
        self.browser.quit()
    }
}

impl BojConfig {
    fn from_config() -> anyhow::Result<Self> {
        let mut boj_toml = std::env::current_dir()?;
        boj_toml.push("boj.toml");
        let boj_toml_content = std::fs::read_to_string(boj_toml)?;
        let config = toml::from_str(&boj_toml_content);
        if let Err(error) = &config {
            println!("boj.toml parse error:\n{}", error);
        }
        Ok(config?)
    }
}
