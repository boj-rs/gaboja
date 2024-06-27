use dialoguer::{theme::ColorfulTheme, BasicHistory, Input};

use crate::command::InputCommand;
use crate::global_state::GlobalState;

mod command;
mod data;
mod global_state;
mod infra;

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

fn main() -> anyhow::Result<()> {
    let mut history = BasicHistory::new().max_entries(8).no_duplicates(true);

    // Reading boj.toml is done inside GlobalState::new
    let mut state = GlobalState::new()?;

    loop {
        let input = Input::<InputCommand>::with_theme(&ColorfulTheme::default())
            .with_prompt("BOJ")
            .history_with(&mut history)
            .interact_text();
        match input {
            Ok(cmd) => {
                if cmd.is_exit() {
                    state.quit()?;
                    break;
                }
                if let Err(e) = state.execute(&cmd) {
                    println!("Error: {}", e);
                }
                if state.ctrlc_channel.try_recv().is_ok() {
                    // consume the ctrlc queue
                    state.ctrlc_channel.try_iter().count();
                }
            }
            Err(err) => {
                match err {
                    dialoguer::Error::IO(io_err) => {
                        if matches!(io_err.kind(), std::io::ErrorKind::Interrupted) {
                            println!("exit");
                            state.quit()?;
                            break;
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
