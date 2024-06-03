mod executor;
mod parser;

use crate::data::Credentials;

#[derive(Debug, Clone)]
pub(crate) struct InputCommand {
    raw_command: String,
    command: Command,
}

impl std::fmt::Display for InputCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.raw_command)
    }
}

impl std::ops::Deref for InputCommand {
    type Target = Command;
    fn deref(&self) -> &Self::Target {
        &self.command
    }
}

impl std::str::FromStr for InputCommand {
    type Err = CommandParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parsed = s.parse::<Command>()?;
        Ok(Self {
            raw_command: s.to_string(),
            command: parsed,
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Command {
    Set(Setting),
    Preset {
        name: String,
    },
    Prob {
        prob: String,
    },
    Build {
        build: Option<String>,
    },
    Run {
        cmd: Option<String>,
        input: Option<String>,
    },
    Test {
        cmd: Option<String>,
    },
    Submit {
        lang: Option<String>,
        file: Option<String>,
    },
    Help,
    Exit,
    Shell(String),
}

#[derive(Debug, Clone)]
pub(crate) enum Setting {
    Credentials(Credentials),
    Lang(String),
    File(String),
    Init(String),
    Build(String),
    Cmd(String),
    Input(String),
}

#[derive(Debug)]
pub(crate) struct CommandParseError {
    msg: String,
}

impl std::fmt::Display for CommandParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.msg.fmt(f)
    }
}

impl std::error::Error for CommandParseError {}

#[derive(Debug)]
pub(crate) struct CommandExecuteError {
    msg: String,
}

impl std::fmt::Display for CommandExecuteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.msg.fmt(f)
    }
}

impl std::error::Error for CommandExecuteError {}
