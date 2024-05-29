use super::{Command, Setting, CommandParseError, Credentials};
use std::collections::HashMap;

macro_rules! error {
    ($($t: tt)*) => { Err(CommandParseError { msg: format!($($t)*) } ) };
}

struct RawCommand {
    main_cmd: String,
    shell: bool,
    args: Vec<String>,
    kwargs: HashMap<String, String>,
}

impl RawCommand {
    fn parse(input: &str) -> Result<Self, CommandParseError> {
        fn command(input: &[u8]) -> Result<(String, &[u8]), CommandParseError> {
            let mut input = input;
            let mut cmd = vec![];
            while !input.is_empty() && input[0] >= b'a' && input[0] <= b'z' {
                cmd.push(input[0]);
                input = &input[1..];
            }
            let cmd = String::from_utf8_lossy(&cmd);
            if !input.is_empty() && input[0] != b' ' {
                return error!("Unexpected non-whitespace character `{}` after command name `{}`", input[0] as char, cmd);
            }
            while !input.is_empty() && input[0] == b' ' {
                input = &input[1..];
            }
            Ok((cmd.to_string(), input))
        }

        fn argument(input: &[u8]) -> Result<(String, &[u8]), CommandParseError> {
            let mut input = input;
            let mut arg = vec![];
            if input[0] == b'\'' || input[0] == b'"' {
                // quoted argument
                let quote = input[0];
                input = &input[1..];
                while !input.is_empty() && input[0] != quote {
                    if input[0] != b'\\' {
                        arg.push(input[0]);
                        input = &input[1..];
                    } else {
                        input = &input[1..];
                        if input.is_empty() {
                            return error!("Unterminated quoted argument");
                        }
                        if input[0] != b'\\' && input[0] != quote {
                            return error!("Unexpected escaped character `{}` after backslash", input[0] as char);
                        }
                        arg.push(input[0]);
                        input = &input[1..];
                    }
                }
                if input.is_empty() {
                    return error!("Unterminated quoted argument");
                }
                input = &input[1..];
                if !input.is_empty() && input[0] != b' ' {
                    return error!("Unexpected non-whitespace character `{}` after quoted argument", input[0] as char);
                }
            } else {
                // unquoted argument
                while !input.is_empty() && input[0] != b' ' {
                    if input[0] == b'\'' || input[0] == b'"' {
                        return error!("Unexpected quote `{}` in the middle of an unquoted argument", input[0] as char);
                    }
                    arg.push(input[0]);
                    input = &input[1..];
                }
            }
            while !input.is_empty() && input[0] == b' ' {
                input = &input[1..];
            }
            let arg = String::from_utf8_lossy(&arg);
            Ok((arg.to_string(), input))
        }

        // bytes-level parsing.
        // split at space
        // cmd args* kwargs* | $ anything
        // cmd: [a-z]+
        // arg: [- '"]+ | ' ([-'\] | \' | \\)* ' | " ([-"\] | \" | \\)* "
        // kwarg: [a-z]+ = arg
        // quote starts a string literal; can only appear right after space or =
        // inside quote, \ can escape the quote and \
        // positional arg after kwarg is an error
        let input = input.trim_matches(' ');
        if input.is_empty() {
            return error!("Input is empty");
        }
        if input.starts_with("$ ") {
            return Ok(Self {
                main_cmd: input[2..].to_string(),
                shell: true,
                args: vec![],
                kwargs: HashMap::new()
            });
        }
        if input.starts_with("$") {
            return error!("There must be a space after the shell marker $");
        }
        let shell = false;
        let (main_cmd, mut input) = command(input.as_bytes())?;
        let mut args = vec![];
        let mut kwargs = HashMap::new();
        while !input.is_empty() {
            let keyword = 'keyword: {
                if let Some(equal_pos) = input.iter().position(|&b| b == b'=') {
                    if input[..equal_pos].iter().all(|&b| b.is_ascii_lowercase()) {
                        let kw = String::from_utf8_lossy(&input[..equal_pos]);
                        input = &input[..equal_pos + 1];
                        break 'keyword Some(kw.to_string());
                    }
                }
                None
            };
            let (arg, rest) = argument(input)?;
            input = rest;
            if let Some(kw) = keyword {
                kwargs.insert(kw, arg);
            } else {
                args.push(arg);
            }
        }
        Ok(Self {
            main_cmd,
            shell,
            args,
            kwargs,
        })
    }
}

impl std::str::FromStr for Command {
    type Err = CommandParseError;
    fn from_str(input: &str) -> Result<Self, CommandParseError> {
        let RawCommand { main_cmd, shell, mut args, mut kwargs } = RawCommand::parse(input)?;
        if shell {
            return Ok(Self::Shell(main_cmd));
        }

        // replace $VAR with environment variable
        for arg in &mut args {
            if arg.starts_with('$') {
                let Ok(env_var) = std::env::var(&arg[1..]) else {
                    return error!("Environment variable `{}` not found", &arg[1..]);
                };
                *arg = env_var;
            }
        }
        for (_, arg) in kwargs.iter_mut() {
            if arg.starts_with('$') {
                let Ok(env_var) = std::env::var(&arg[1..]) else {
                    return error!("Environment variable `{}` not found", &arg[1..]);
                };
                *arg = env_var;
            }
        }

        match &main_cmd[..] {
            "set" => {
                if args.is_empty() {
                    return error!("set: Missing argument <variable>");
                }
                let variable = &args[0][..];
                let setting = match variable {
                    "credentials" => {
                        if args.len() == 1 {
                            return error!("set credentials: Missing arguments <bojautologin> <onlinejudge>");
                        } else if args.len() == 2 {
                            return error!("set credentials: Missing argument <onlinejudge>");
                        } else if args.len() > 3 {
                            return error!("set credentials: Too many arguments");
                        }
                        Setting::Credentials(Credentials {
                            bojautologin: args[1].clone(),
                            onlinejudge: args[2].clone(),
                        })
                    }
                    "lang" | "file" | "build" | "cmd" | "input" | "init" => {
                        if args.len() == 1 {
                            return error!("set {}: Missing argument <{}>", variable, variable);
                        } else if args.len() > 2 {
                            return error!("set {}: Too many arguments", variable);
                        }
                        let arg = args[1].clone();
                        match variable {
                            "lang" => Setting::Lang(arg),
                            "file" => Setting::File(arg),
                            "init" => Setting::Init(arg),
                            "build" => Setting::Build(arg),
                            "cmd" => Setting::Cmd(arg),
                            "input" => Setting::Input(arg),
                            _ => unreachable!()
                        }
                    }
                    _ => {
                        return error!("set: Unrecognized variable `{}`", args[0]);
                    }
                };
                if !kwargs.is_empty() {
                    return error!("set: Unexpected keyword argument(s)");
                }
                Ok(Command::Set(setting))
            }
            "preset" => {
                if args.len() == 0 {
                    error!("preset: Missing argument <name>")
                } else if args.len() > 1 {
                    error!("preset: Too many positional arguments")
                } else if kwargs.len() > 0 {
                    error!("preset: Unexpected keyword argument(s)")
                } else {
                    Ok(Self::Preset {
                        name: args[0].clone()
                    })
                }
            }
            "prob" => {
                if args.len() == 0 {
                    error!("prob: Missing argument <problem>")
                } else if args.len() > 1 {
                    error!("prob: Too many positional arguments")
                } else if kwargs.len() > 0 {
                    error!("prob: Unexpected keyword argument(s)")
                } else {
                    Ok(Self::Prob {
                        prob: args[0].clone()
                    })
                }
            }
            "build" => {
                let mut build = None;
                if args.len() == 1 {
                    build = Some(args[0].clone());
                } else if args.len() > 0 {
                    return error!("build: Too many positional arguments");
                }
                if !kwargs.is_empty() {
                    return error!("build: Unexpected keyword argument(s)");
                }
                Ok(Self::Build { build })
            }
            "run" => {
                let mut cmd = None;
                let mut input = None;
                if !args.is_empty() {
                    return error!("run: Unexpected positional argument(s)");
                }
                if let Some(c) = kwargs.remove(&"c".to_string()) {
                    cmd = Some(c);
                }
                if let Some(i) = kwargs.remove(&"i".to_string()) {
                    input = Some(i);
                }
                if !kwargs.is_empty() {
                    return error!("run: Unexpected keyword argument(s)");
                }
                Ok(Self::Run { cmd, input })
            }
            "test" => {
                let mut cmd = None;
                if !args.is_empty() {
                    return error!("test: Unexpected positional argument(s)");
                }
                if let Some(c) = kwargs.remove(&"c".to_string()) {
                    cmd = Some(c);
                }
                if !kwargs.is_empty() {
                    return error!("test: Unexpected keyword argument(s)");
                }
                Ok(Self::Test { cmd })
            }
            "submit" => {
                let mut lang = None;
                let mut file = None;
                if !args.is_empty() {
                    return error!("submit: Unexpected positional argument(s)");
                }
                if let Some(l) = kwargs.remove(&"l".to_string()) {
                    lang = Some(l);
                }
                if let Some(f) = kwargs.remove(&"f".to_string()) {
                    file = Some(f);
                }
                if !kwargs.is_empty() {
                    return error!("submit: Unexpected keyword argument(s)");
                }
                Ok(Self::Submit { lang, file })
            }
            "exit" => {
                if !args.is_empty() || !kwargs.is_empty() {
                    return error!("exit: Unexpected argument(s)");
                }
                Ok(Self::Exit)
            }
            "help" => {
                Ok(Self::Help)
            }
            _ => {
                Err(CommandParseError {
                    msg: format!("Unknown command `{}`", main_cmd)
                })
            }
        }
    }
}