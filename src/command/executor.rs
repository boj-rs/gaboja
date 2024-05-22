use super::{Command, Setting, CommandExecuteError};
use crate::global_state::GlobalState;
use crate::data::{ProblemId, ExampleIO};
use crate::infra::subprocess::{run_silent, run_with_input_timed, run_interactive, Output};
use regex::{Regex, Captures, Replacer};
use once_cell::sync::Lazy;
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
            if sep.is_empty() { sep = "_"; }
            match &self.0 {
                ProblemId::Problem(prob) => {
                    dst.push_str(&prob);
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
    RE.replace_all(path, ProblemReplacer(problem_id)).to_string()
}

impl GlobalState {
    pub(crate) fn execute(&mut self, command: &Command) -> anyhow::Result<()> {
        match command {
            Command::Set(setting) => self.set(setting)?,
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
                let Some((prob, time, kind)) = self.problem.as_ref().map(|p| (&p.id, p.time, &p.kind)) else {
                    error!("run: Problem not specified")?
                };
                let mut no_run_reasons = kind.iter().flat_map(|kind| kind.no_run());
                if let Some(first_reason) = no_run_reasons.next() {
                    let mut reason = format!("run: Current problem does not support run. Reason: {}", first_reason);
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
                self.run(&cmd, &input_data, Duration::from_secs_f64((time * 3.0 + 2.0).min(10.0)))?;
            }
            Command::Test { cmd } => {
                let Some((prob, time, kind, io)) = self.problem.as_ref().map(|p| (&p.id, p.time, &p.kind, &p.io)) else {
                    error!("test: Problem not specified")?
                };
                let mut no_test_reasons = kind.iter().flat_map(|kind| kind.no_test());
                if let Some(first_reason) = no_test_reasons.next() {
                    let mut reason = format!("test: Current problem does not support test. Reason: {}", first_reason);
                    for rest_reason in no_test_reasons {
                        reason += ", ";
                        reason += &rest_reason;
                    }
                    error!("{}", reason)?
                }
                let mut no_diff_reasons = kind.iter().flat_map(|kind| kind.no_diff());
                let mut diff = true;
                if let Some(first_reason) = no_diff_reasons.next() {
                    let mut reason = format!("test: Current problem does not support diff on test output. Reason: {}", first_reason);
                    for rest_reason in no_diff_reasons {
                        reason += ", ";
                        reason += &rest_reason;
                    }
                    println!("{}", reason);
                    diff = false;
                }
                let stored_cmd = self.cmd.clone();
                let cmd = substitute_problem(cmd.as_ref().unwrap_or(&stored_cmd), prob);
                self.test(&cmd, &io, Duration::from_secs_f64((time * 3.0 + 2.0).min(10.0)), diff)?;
            }
            Command::Submit { lang, file } => {
                let Some(prob) = self.problem.as_ref().map(|p| &p.id) else {
                    error!("submit: Problem not specified")?
                };
                let lang = if let Some(lang) = lang { lang.clone() }
                else if !self.lang.is_empty() { self.lang.clone() }
                else { error!("submit: Language not specified")? };
                let file = if let Some(file) = file { file.clone() }
                else if !self.file.is_empty() { self.file.clone() }
                else { error!("submit: Solution file not specified")? };
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
            Setting::Credentials { bojautologin, onlinejudge } => {
                self.credentials.bojautologin.clear();
                self.credentials.bojautologin += bojautologin;
                self.credentials.onlinejudge.clear();
                self.credentials.onlinejudge += onlinejudge;
                self.browser.login(bojautologin, onlinejudge)?;
                if let Some(username) = self.browser.get_username()? {
                    println!("Logged in as {}", username);
                } else {
                    error!("Login failed with the credentials provided")?
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

    fn prob(&mut self, prob: &str) -> anyhow::Result<()> {
        // TODO: try fetching from local datastore first
        let problem_id = prob.parse::<ProblemId>()?;
        self.problem = Some(self.browser.get_problem(&problem_id)?);
        let problem = self.problem.as_ref().unwrap();
        println!("Problem {} {}", problem.id, problem.title);
        println!("Time limit: {:.3}s{} / Memory limit: {}MB{}", problem.time, if !problem.time_bonus { " (No bonus)" } else { "" }, problem.memory, if !problem.memory_bonus { " (No bonus)" } else { "" });
        // TODO: store the fetched problem to the local datastore
        Ok(())
    }

    fn build(&self, build: &str) -> anyhow::Result<()> {
        let res = run_silent(build)?;
        if let Some(err) = res {
            error!("Build returned nonzero exit code. STDERR:\n{}", err)?
        }
        Ok(())
    }

    fn run(&self, cmd: &str, input: &str, time: Duration) -> anyhow::Result<()> {
        // TODO: Add color to STDOUT, STDERR, etc.
        let Some(Output { stdout, stderr, success, duration }) = run_with_input_timed(cmd, input, time)? else {
            error!("Run did not finish in {:.3}s", time.as_secs_f64())?
        };
        println!("STDOUT:");
        println!("{}", stdout);
        println!("STDERR:");
        println!("{}", stderr);
        println!("Time: {:.3}s", duration.as_secs_f64());
        if !success {
            error!("Run returned nonzero exit code")?
        }
        Ok(())
    }

    fn test(&self, cmd: &str, io: &[ExampleIO], time: Duration, diff: bool) -> anyhow::Result<()> {
        // TODO: Add color to STDOUT, STDERR, etc.
        let io_count = io.len();
        for (io_no, ExampleIO { input, output }) in io.iter().enumerate() {
            println!("Running Test {}/{}...", io_no + 1, io_count);
            let Some(Output { stdout, stderr, success, duration }) = run_with_input_timed(cmd, input, time)? else {
                println!("Test {}/{} Time Limit Exceeded", io_no + 1, io_count);
                error!("Run did not finish in {:.3}s", time.as_secs_f64())?
            };
            let output = trim_lines(output);
            let stdout = trim_lines(&stdout);
            let stderr = trim_lines(&stderr);
            if !success {
                println!("STDOUT:");
                println!("{}", stdout);
                if !stderr.is_empty() {
                    println!("STDERR:");
                    println!("{}", stderr);
                }
                println!("Time: {:.3}s", duration.as_secs_f64());
                error!("Run returned nonzero exit code")?
            }
            if diff {
                if output == stdout {
                    println!("Test {}/{} Success", io_no + 1, io_count);
                    println!("Time: {:.3}s", duration.as_secs_f64());
                } else {
                    println!("Test {}/{} Wrong Answer", io_no + 1, io_count);
                    println!("{}", similar::TextDiff::from_lines(&output, &stdout).unified_diff().header("Expected", "Output"));
                    if !stderr.is_empty() {
                        println!("STDERR:");
                        println!("{}", stderr);
                    }
                    println!("Time: {:.3}s", duration.as_secs_f64());
                    error!("Test failed")?
                }
            } else {
                println!("STDIN:");
                println!("{}", trim_lines(input));
                println!("STDOUT:");
                println!("{}", stdout);
                if !stderr.is_empty() {
                    println!("STDERR:");
                    println!("{}", stderr);
                }
                println!("Time: {:.3}s", duration.as_secs_f64());
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
        self.browser.submit_solution(prob, &source, lang)?;
        let (mut status_text, mut status_class) = self.browser.get_submission_status()?;
        println!("Status: {} ({})", status_text, status_class);
        static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"result-wait|result-rejudge-wait|result-no-judge|result-compile|result-judging").unwrap());
        while RE.is_match(&status_class) {
            let (next_status_text, next_status_class) = self.browser.get_submission_status()?;
            if next_status_text != status_text {
                (status_text, status_class) = (next_status_text, next_status_class);
                println!("Status: {} ({})", status_text, status_class);
            }
        }
        // TODO: replace with indicatif progress bar
        Ok(())
    }

    fn help(&self) -> anyhow::Result<()> {
        println!("{}", HELP.trim());
        Ok(())
    }
}

fn trim_lines(s: &str) -> String {
    s.trim_end().lines().flat_map(|l| [l.trim_end(), "\n"]).collect()
}

const HELP: &str = "
set credentials <bojautologin> <onlinejudge>
    Set BOJ login cookies and log in with them.
set lang <lang>
set file <file>
set build <build>
set cmd <cmd>
set input <input>
    Set default value for the given variable.
prob <prob>
    Load the problem <prob> and set it as the current problem.
build [build]
    Build your solution.
run [i=input] [c=cmd]
    Run your solution with a custom input file.
test [c=cmd]
    Test your solution against sample test cases.
submit [l=lang] [f=file]
    Submit your solution to BOJ.
help
    Display this help.
exit
    Exit the program.
";