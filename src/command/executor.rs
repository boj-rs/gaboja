use super::{Command, CommandExecuteError, Credentials, Setting};
use crate::data::{ExampleIO, Preset, ProblemId};
use crate::global_state::GlobalState;
use crate::infra::console::{report_stderr, report_stdout, Spinner, SubmitProgress, TestProgress};
use crate::infra::subprocess::{run_interactive, run_silent, run_with_input_timed, Output};
use once_cell::sync::Lazy;
use regex::{Captures, Regex, Replacer};
use std::time::Duration;

macro_rules! error {
    ($($t: tt)*) => { Err(CommandExecuteError { msg: format!($($t)*) } ) };
}

impl Command {
    pub(crate) fn is_exit(&self) -> bool {
        matches!(self, Command::Exit)
    }
}

fn substitute_problem(path: &str, problem_id: &ProblemId) -> String {
    static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\{(.?)\}").unwrap());
    struct ProblemReplacer<'a>(&'a ProblemId);
    impl<'a> Replacer for ProblemReplacer<'a> {
        fn replace_append(&mut self, caps: &Captures<'_>, dst: &mut String) {
            let mut sep = &caps[1];
            if sep.is_empty() {
                sep = "_";
            }
            match &self.0 {
                ProblemId::Problem(prob) => {
                    dst.push_str(prob);
                }
                ProblemId::ContestProblem(prob) => {
                    let mut fragments = prob.split('/');
                    dst.push_str(fragments.next().unwrap());
                    dst.push_str(sep);
                    dst.push_str(fragments.next().unwrap());
                }
            }
        }
    }
    RE.replace_all(path, ProblemReplacer(problem_id))
        .to_string()
}

impl GlobalState {
    pub(crate) fn execute(&mut self, command: &Command) -> anyhow::Result<()> {
        match command {
            Command::Set(setting) => self.set(setting)?,
            Command::Preset { name } => {
                let Some(preset) = self.presets.get(name) else {
                    error!("preset: Unknown preset name")?
                };
                let preset = preset.clone();
                self.preset(preset)?;
            }
            Command::Prob { prob } => self.prob(prob)?,
            Command::Build { build } => {
                let Some(prob) = self.problem.as_ref().map(|p| &p.id) else {
                    error!("build: Problem not specified")?
                };
                let stored_build = self.build.clone();
                let build = substitute_problem(build.as_ref().unwrap_or(&stored_build), prob);
                self.build(&build)?;
            }
            Command::Run { cmd, input } => {
                let Some((prob, time, kind)) =
                    self.problem.as_ref().map(|p| (&p.id, p.time, &p.kind))
                else {
                    error!("run: Problem not specified")?
                };
                let mut no_run_reasons = kind.iter().flat_map(|kind| kind.no_run());
                if let Some(first_reason) = no_run_reasons.next() {
                    let mut reason = format!(
                        "run: Current problem does not support run. Reason: {}",
                        first_reason
                    );
                    for rest_reason in no_run_reasons {
                        reason += ", ";
                        reason += &rest_reason;
                    }
                    error!("{}", reason)?
                }
                let stored_cmd = self.cmd.clone();
                let cmd = substitute_problem(cmd.as_ref().unwrap_or(&stored_cmd), prob);
                if kind.iter().any(|kind| kind.is_interactive()) {
                    run_interactive(&cmd)?;
                    return Ok(());
                }
                let stored_input = self.input.clone();
                let input = input.as_ref().unwrap_or(&stored_input);
                let input_data = std::fs::read_to_string(input)?;
                self.run(
                    &cmd,
                    &input_data,
                    Duration::from_secs_f64((time * 3.0 + 2.0).min(10.0)),
                )?;
            }
            Command::Test { cmd } => {
                let Some((prob, time, kind, io)) = self
                    .problem
                    .as_ref()
                    .map(|p| (&p.id, p.time, &p.kind, &p.io))
                else {
                    error!("test: Problem not specified")?
                };
                let mut no_test_reasons = kind.iter().flat_map(|kind| kind.no_test());
                if let Some(first_reason) = no_test_reasons.next() {
                    let mut reason = format!(
                        "test: Current problem does not support test. Reason: {}",
                        first_reason
                    );
                    for rest_reason in no_test_reasons {
                        reason += ", ";
                        reason += &rest_reason;
                    }
                    error!("{}", reason)?
                }
                let mut no_diff_reasons = kind.iter().flat_map(|kind| kind.no_diff());
                let mut diff = true;
                if let Some(first_reason) = no_diff_reasons.next() {
                    let mut reason = format!(
                        "test: Current problem does not support diff on test output. Reason: {}",
                        first_reason
                    );
                    for rest_reason in no_diff_reasons {
                        reason += ", ";
                        reason += &rest_reason;
                    }
                    println!("{}", reason);
                    diff = false;
                }
                let stored_cmd = self.cmd.clone();
                let cmd = substitute_problem(cmd.as_ref().unwrap_or(&stored_cmd), prob);
                self.test(
                    &cmd,
                    io,
                    Duration::from_secs_f64((time * 3.0 + 2.0).min(10.0)),
                    diff,
                )?;
            }
            Command::Submit { lang, file } => {
                let Some(prob) = self.problem.as_ref().map(|p| &p.id) else {
                    error!("submit: Problem not specified")?
                };
                let lang = if let Some(lang) = lang {
                    lang.clone()
                } else if !self.lang.is_empty() {
                    self.lang.clone()
                } else {
                    error!("submit: Language not specified")?
                };
                let file = if let Some(file) = file {
                    file.clone()
                } else if !self.file.is_empty() {
                    self.file.clone()
                } else {
                    error!("submit: Solution file not specified")?
                };
                let file = substitute_problem(&file, prob);
                self.submit(&lang, &file)?;
            }
            Command::Help => {
                self.help()?;
            }
            Command::Exit => {}
            Command::Shell(shell_cmd) => {
                run_interactive(shell_cmd)?;
            }
        }
        Ok(())
    }

    fn set(&mut self, setting: &Setting) -> anyhow::Result<()> {
        match setting {
            Setting::Credentials(Credentials {
                bojautologin,
                onlinejudge,
            }) => {
                self.credentials.bojautologin.clear();
                self.credentials.bojautologin += bojautologin;
                self.credentials.onlinejudge.clear();
                self.credentials.onlinejudge += onlinejudge;

                let spinner = Spinner::new("Logging in...");
                self.browser.login(bojautologin, onlinejudge)?;
                if let Some(username) = self.browser.get_username()? {
                    spinner.finish(&format!("Logged in as {}", username));
                } else {
                    spinner.abandon("Login failed with the credentials provided");
                }
            }
            Setting::Lang(lang) => {
                self.lang.clear();
                self.lang += lang;
            }
            Setting::File(file) => {
                self.file.clear();
                self.file += file;
            }
            Setting::Init(init) => {
                self.init.clear();
                self.init += init;
                self.init()?;
            }
            Setting::Build(build) => {
                self.build.clear();
                self.build += build;
            }
            Setting::Cmd(cmd) => {
                self.cmd.clear();
                self.cmd += cmd;
            }
            Setting::Input(input) => {
                self.input.clear();
                self.input += input;
            }
        }
        Ok(())
    }

    fn preset(&mut self, preset: Preset) -> anyhow::Result<()> {
        let Preset {
            credentials,
            lang,
            file,
            init,
            build,
            cmd,
            input,
            ..
        } = preset;
        if let Some(credentials) = credentials {
            self.set(&Setting::Credentials(credentials))?;
        }
        if let Some(lang) = lang {
            self.set(&Setting::Lang(lang))?;
        }
        if let Some(file) = file {
            self.set(&Setting::File(file))?;
        }
        if let Some(init) = init {
            self.set(&Setting::Init(init))?;
        }
        if let Some(build) = build {
            self.set(&Setting::Build(build))?;
        }
        if let Some(cmd) = cmd {
            self.set(&Setting::Cmd(cmd))?;
        }
        if let Some(input) = input {
            self.set(&Setting::Input(input))?;
        }
        Ok(())
    }

    fn prob(&mut self, prob: &str) -> anyhow::Result<()> {
        let problem_id = prob.parse::<ProblemId>()?;
        if let Some(problem) = self.problem_cache.get(&problem_id) {
            // try copying from the cache first
            self.problem = Some(problem.clone());
        } else {
            // store the fetched problem to the cache
            let spinner = Spinner::new("Fetching problem...");
            self.problem = Some(self.browser.get_problem(&problem_id)?);
            spinner.finish("Fetching done");
            self.problem_cache
                .insert(problem_id, self.problem.clone().unwrap());
        }
        let problem = self.problem.as_ref().unwrap();
        println!("Problem {} {}", problem.id, problem.title);
        println!(
            "Time limit: {:.3}s{} / Memory limit: {}MB{}",
            problem.time,
            if !problem.time_bonus {
                " (No bonus)"
            } else {
                ""
            },
            problem.memory,
            if !problem.memory_bonus {
                " (No bonus)"
            } else {
                ""
            }
        );
        self.init()?;
        Ok(())
    }

    fn init(&self) -> anyhow::Result<()> {
        // if init is empty, do nothing
        if self.init.is_empty() {
            return Ok(());
        }
        // if prob is not set, do not try to run init
        let Some(prob) = self.problem.as_ref() else {
            return Ok(());
        };
        let init_cmd = substitute_problem(&self.init, &prob.id);
        let spinner = Spinner::new("Running init...");
        let res = run_silent(&init_cmd)?;
        if let Some(err) = res {
            spinner.abandon("Init returned nonzero exit code.");
            report_stderr(&err);
        } else {
            spinner.finish("Init finished");
        }
        Ok(())
    }

    fn build(&self, build: &str) -> anyhow::Result<()> {
        let spinner = Spinner::new("Running build...");
        let res = run_silent(build)?;
        if let Some(err) = res {
            spinner.abandon("Build returned nonzero exit code");
            report_stderr(&err);
        } else {
            spinner.finish("Build finished");
        }
        Ok(())
    }

    fn run(&self, cmd: &str, input: &str, time: Duration) -> anyhow::Result<()> {
        let spinner = Spinner::new("Running code...");
        let Some(Output {
            stdout,
            stderr,
            success,
            duration,
        }) = run_with_input_timed(cmd, input, time)?
        else {
            spinner.abandon(&format!("Run did not finish in {:.3}s", time.as_secs_f64()));
            return Ok(());
        };
        let duration = duration.as_secs_f64();
        if !success {
            spinner.abandon(&format!(
                "Run returned nonzero exit code (Elapsed: {:.3}s)",
                duration
            ));
        } else {
            spinner.finish(&format!("Run finished (Elapsed: {:.3}s)", duration));
        }
        report_stdout(&stdout);
        if !stderr.is_empty() {
            report_stderr(&stderr);
        }
        Ok(())
    }

    fn test(&self, cmd: &str, io: &[ExampleIO], time: Duration, diff: bool) -> anyhow::Result<()> {
        let io_count = io.len();
        let test_progress = TestProgress::new(io_count as u64);
        for ExampleIO { input, output } in io {
            let expected = output;
            let output = run_with_input_timed(cmd, input, time)?;
            if !test_progress.handle_test_result(input, expected, output, diff) {
                break;
            }
        }
        Ok(())
    }

    fn submit(&self, lang: &str, file: &str) -> anyhow::Result<()> {
        let Some(prob) = self.problem.as_ref().map(|p| &p.id) else {
            error!("submit: Problem not specified")?
        };
        let Ok(source) = std::fs::read_to_string(file) else {
            error!("submit: File `{}` does not exist", file)?
        };

        let spinner = Spinner::new("Submitting code...");
        self.browser.submit_solution(prob, &source, lang)?;
        spinner.finish("Code submitted.");

        let submit_progress = SubmitProgress::new();
        loop {
            let (status_text, status_class) = self.browser.get_submission_status()?;
            if submit_progress.update(&status_text, &status_class) {
                break;
            }
        }
        Ok(())
    }

    fn help(&self) -> anyhow::Result<()> {
        println!("{}", HELP.trim());
        Ok(())
    }
}

const HELP: &str = "
set credentials <bojautologin> <onlinejudge>
    Set BOJ login cookies and log in with them.
set lang <lang>
set file <file>
set init <init>
set build <build>
set cmd <cmd>
set input <input>
    Set default value for the given variable.
prob <prob>
    Load the problem <prob> and set it as the current problem.
    If <init> is set, run it.
build [build]
    Build your solution.
run [i=input] [c=cmd]
    Run your solution with a custom input file.
test [c=cmd]
    Test your solution against sample test cases.
submit [l=lang] [f=file]
    Submit your solution to BOJ.
preset <name>
    Apply one of the presets defined in boj.toml.
help
    Display this help.
exit
    Exit the program.
";
