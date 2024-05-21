use dialoguer::{theme::ColorfulTheme, BasicHistory, Input};
use once_cell::sync::Lazy;
use std::env;

use crate::data::ProblemId;
use crate::global_state::GlobalState;
use crate::command::InputCommand;

mod command;
mod data;
mod infra;
mod global_state;

static BOJAUTOLOGIN: Lazy<String> = Lazy::new(|| env::var("BUB_BOJAUTOLOGIN").unwrap());
static ONLINEJUDGE: Lazy<String> = Lazy::new(|| env::var("BUB_ONLINEJUDGE").unwrap());

// const LANGUAGE: &str = "Rust 2021";
// const SOURCE: &str = r#"fn main() { println!("Hello World!"); }"#;

// language:
// help
// set credentials <bojautologin> <onlinejudge>
// set lang <lang>
// set file <file> (may contain `{<separator>}` for problem number; separator is used for contests)
// set build <build>
// set cmd <cmd>
// set input <input>
// prob <prob>
// build [build]
// run [i=input] [c=cmd]
// test [c=cmd]
// submit [l=lang] [f=file]
// exit
// run/test not supported for funcimpl/classimpl.
// if prob is interactive, run ignores input and lets the user input the response instead. test not supported
// features:
// strings in the form of $VAR are assumed to be env vars; error if not exist
// add .bojrc at the current folder or home directory which works like .bashrc
// inputs that don't start with a known keyword are assumed to be shell command
// without setting, lang = 'Rust 2021', runcmd = 'cargo run --release', input = 'input.txt'

const TEST: bool = false;

fn main() -> anyhow::Result<()> {
    if TEST {
        browser_test()?;
        return Ok(());
    }

    let mut history = BasicHistory::new().max_entries(8).no_duplicates(true);

    let mut state = GlobalState::new()?;

    // Read and execute .bojrc before entering the loop
    // TODO: Attach indicatif progress bar
    let mut bojrc = std::env::current_dir()?;
    bojrc.push(".bojrc");
    if let Ok(bojrc_content) = std::fs::read_to_string(bojrc) {
        for (lineno, line) in bojrc_content.lines().enumerate() {
            if line.is_empty() { continue; }
            match line.parse::<InputCommand>() {
                Ok(cmd) => {
                    if let Err(err) = state.execute(&cmd) {
                        println!(".bojrc execution error at line {}: {}", lineno + 1, err);
                        break;
                    }
                }
                Err(err) => {
                    println!(".bojrc parse error at line {}: {}", lineno + 1, err);
                    break;
                }
            }
        }
    }

    loop {
        if let Ok(cmd) = Input::<InputCommand>::with_theme(&ColorfulTheme::default())
            .with_prompt("BOJ")
            .history_with(&mut history)
            .interact_text()
        {
            if cmd.is_exit() {
                state.quit()?;
                break;
            }
            if let Err(e) = state.execute(&cmd) {
                println!("Error: {}", e);
            }
        }
    }
    Ok(())
}

fn browser_test() -> anyhow::Result<()> {
    use crate::infra::browser::Browser;
    let browser = Browser::new()?;
    browser.login(&BOJAUTOLOGIN, &ONLINEJUDGE)?;
    fn inner(browser: &Browser) -> anyhow::Result<()> {
        let username = browser.get_username()?;
        if let Some(username) = username {
            println!("Username: {}", username);
        } else {
            println!("Not logged in");
        }
        let problem_id = "2557".parse::<ProblemId>()?;
        let problem = browser.get_problem(&problem_id)?;
        println!("{:?}", problem);
        // browser.submit_solution(&problem_id, SOURCE, LANGUAGE)?;
        // println!("solution submitted");
        // let (text, class) = browser.get_submission_status()?;
        // println!("{} {}", text, class);
        Ok(())
    }
    if let Err(e) = inner(&browser) {
        println!("{:?}", e);
    }
    browser.quit()?;
    Ok(())
}