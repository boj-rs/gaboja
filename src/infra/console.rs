use crate::infra::subprocess::Output;
use console::{measure_text_width, pad_str, style, Alignment};
use indicatif::{ProgressBar, ProgressStyle};
use once_cell::sync::Lazy;
use regex::Regex;
use similar::ChangeTag;
use std::time::Duration;

pub(crate) struct Spinner {
    progress_bar: ProgressBar,
}

impl Spinner {
    pub(crate) fn new(msg: &str) -> Self {
        let style = ProgressStyle::with_template("{spinner} {msg}").unwrap();
        let spinner = ProgressBar::new_spinner().with_style(style);
        spinner.enable_steady_tick(Duration::from_millis(100));
        spinner.set_message(msg.to_string());
        Self {
            progress_bar: spinner,
        }
    }

    pub(crate) fn set_message(&self, msg: &str) {
        self.progress_bar.set_message(msg.to_string());
    }

    pub(crate) fn finish(self, msg: &str) {
        let style = ProgressStyle::with_template("{prefix} {msg}").unwrap();
        self.progress_bar.set_style(style);
        self.progress_bar
            .set_prefix(console::style("✔".to_string()).green().to_string());
        self.progress_bar.finish_with_message(msg.to_string());
    }

    pub(crate) fn abandon(self, msg: &str) {
        let style = ProgressStyle::with_template("{prefix} {msg}").unwrap();
        self.progress_bar.set_style(style);
        self.progress_bar
            .set_prefix(console::style("✘".to_string()).red().to_string());
        self.progress_bar.finish_with_message(msg.to_string());
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        if !self.progress_bar.is_finished() {
            self.progress_bar.disable_steady_tick();
            self.progress_bar.abandon();
        }
    }
}

pub(crate) struct TestProgress {
    progress_bar: ProgressBar,
}

impl TestProgress {
    pub(crate) fn new(len: u64) -> Self {
        let style =
            ProgressStyle::with_template("[{pos:>2}/{len:>2}] {msg}\n{bar:40.green}").unwrap();
        let progress_bar = ProgressBar::new(len).with_style(style);
        progress_bar.set_message("Running sample tests...".to_string());
        progress_bar.set_position(1);
        Self { progress_bar }
    }

    /// Returns true if test passed
    pub(crate) fn handle_test_result(
        &self,
        stdin: &str,
        expected: &str,
        output: Option<Output>,
        diff: bool,
    ) -> bool {
        let fail_style =
            ProgressStyle::with_template("[{pos:>2}/{len:>2}] {msg}\n{bar:40.red}").unwrap();
        let ac = console::style("AC".to_string()).green();
        let wa = console::style("WA".to_string()).red();
        let tle = console::style("TLE".to_string()).red();
        let re = console::style("RE".to_string()).red();
        let ok = console::style("OK".to_string()).yellow();
        let check = console::style("✔".to_string()).green();
        let cross = console::style("✘".to_string()).red();
        let pos = self.progress_bar.position();
        let len = self.progress_bar.length().unwrap();

        if let Some(Output {
            stdout,
            stderr,
            success,
            duration,
        }) = output
        {
            let duration = duration.as_secs_f64();
            fn trim_lines(s: &str) -> String {
                s.trim_end()
                    .lines()
                    .flat_map(|l| [l.trim_end(), "\n"])
                    .collect()
            }
            let stdin = trim_lines(stdin);
            let expected = trim_lines(expected);
            let stdout = trim_lines(&stdout);
            let stderr = trim_lines(&stderr);
            if !success {
                self.progress_bar.set_style(fail_style);
                self.progress_bar.abandon_with_message(format!(
                    "{} Test {} {} ({:.3}s)",
                    cross, pos, re, duration
                ));
                if !stdout.is_empty() {
                    report_stdout(&stdout);
                }
                if !stderr.is_empty() {
                    report_stderr(&stderr);
                }
                return false;
            }
            if !diff || expected == stdout {
                if pos == len {
                    // All tests passed
                    self.progress_bar
                        .finish_with_message(format!("{} All sample tests passed", check));
                } else {
                    self.progress_bar.inc(1);
                }
                if diff {
                    self.progress_bar
                        .println(format!("{} Test {} {} ({:.3}s)", check, pos, ac, duration));
                } else {
                    self.progress_bar
                        .println(format!("{} Test {} {} ({:.3}s)", check, pos, ok, duration));
                    self.progress_bar.suspend(|| {
                        if !stdin.is_empty() {
                            report_stdin(&stdin);
                        }
                        report_stdout(&stdout);
                        if !stderr.is_empty() {
                            report_stderr(&stderr);
                        }
                    });
                }
                return true;
            }
            // diff on and WA
            self.progress_bar.set_style(fail_style);
            self.progress_bar
                .abandon_with_message(format!("{} Test {} {} ({:.3}s)", cross, pos, wa, duration));
            report_diff(&expected, &stdout);
            if !stderr.is_empty() {
                report_stderr(&stderr);
            }
        } else {
            self.progress_bar.set_style(fail_style);
            self.progress_bar
                .abandon_with_message(format!("{} Test {} {}", cross, pos, tle));
        }
        false
    }
}

impl Drop for TestProgress {
    fn drop(&mut self) {
        if !self.progress_bar.is_finished() {
            self.progress_bar.abandon();
        }
    }
}

pub(crate) struct SubmitProgress {
    progress_bar: ProgressBar,
}

impl SubmitProgress {
    pub(crate) fn new() -> Self {
        let style = ProgressStyle::with_template("{msg}\n{bar:40.green}").unwrap();
        let progress_bar = ProgressBar::new(100).with_style(style);
        progress_bar.set_message("Waiting for response...".to_string());
        Self { progress_bar }
    }

    /// Returns true if finished
    pub(crate) fn update(&self, status_text: &str, status_class: &str) -> bool {
        static CONTINUE_CLASS: Lazy<Regex> = Lazy::new(|| {
            Regex::new(
                r"result-wait|result-rejudge-wait|result-no-judge|result-compile|result-judging",
            )
            .unwrap()
        });
        static CONTINUE_TEXT: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"채점|중|Pending|Judg").unwrap());
        static PERCENT: Lazy<Regex> = Lazy::new(|| Regex::new(r"\((\d+)%\)").unwrap());
        if CONTINUE_CLASS.is_match(status_class) || CONTINUE_TEXT.is_match(status_text) {
            // one of them can update first, so keep updating until both are decisive
            self.progress_bar.set_message(status_text.to_string());
            if let Some(capture) = PERCENT.captures(status_text) {
                self.progress_bar
                    .set_position(capture.get(1).unwrap().as_str().parse::<u64>().unwrap());
            }
            return false;
        }
        let fail_style = ProgressStyle::with_template("{msg}\n{bar:40.red}").unwrap();
        let semifail_style = ProgressStyle::with_template("{msg}\n{bar:40.yellow}").unwrap();
        static RESULT: Lazy<Regex> = Lazy::new(|| Regex::new(r" result-([a-z]+)").unwrap());
        let result = RESULT
            .captures(status_class)
            .unwrap()
            .get(1)
            .unwrap()
            .as_str()
            .to_ascii_uppercase();
        let mut color_result = console::style(if result == "RTE" {
            "RE".to_string()
        } else {
            result.clone()
        });
        if result == "AC" {
            color_result = color_result.green();
            self.progress_bar.set_position(100);
        } else if result == "PAC" {
            color_result = color_result.yellow();
            self.progress_bar.set_style(semifail_style);
        } else {
            color_result = color_result.red();
            self.progress_bar.set_style(fail_style);
        }
        self.progress_bar
            .abandon_with_message(format!("{} [{}]", status_text, color_result));
        true
    }
}

impl Drop for SubmitProgress {
    fn drop(&mut self) {
        if !self.progress_bar.is_finished() {
            self.progress_bar.abandon();
        }
    }
}

pub(crate) fn report_stdin(stdin: &str) {
    let header = console::style("STDIN:".to_string()).yellow();
    println!("{}\n{}", header, stdin);
}

pub(crate) fn report_stdout(stdout: &str) {
    let header = console::style("STDOUT:".to_string()).yellow();
    println!("{}\n{}", header, stdout);
}

pub(crate) fn report_stderr(stderr: &str) {
    let header = console::style("STDERR:".to_string()).yellow();
    println!("{}\n{}", header, stderr);
}

fn report_diff(expected: &str, output: &str) {
    let diff = similar::TextDiff::from_lines(expected, output);
    let ops = diff.ops();
    // lineno 4, width 20+a, lineno 4, width (don't need to count)
    // lineno, if different: on_green / on_red; otherwise dim
    // inline change highlight: green / red
    let left_width = expected
        .lines()
        .map(|s| measure_text_width(s) + 2)
        .max()
        .unwrap_or(0)
        .max(20);
    let expected_title = style("Expected".to_string())
        .green()
        .underlined()
        .to_string();
    let expected_title = pad_str(&expected_title, left_width, Alignment::Left, None);
    let output_title = style("Output".to_string()).red().underlined();
    println!("    {}    {}", expected_title, output_title);
    let mut to_print = vec![];
    let mut left_lineno = 1usize;
    let mut right_lineno = 1usize;
    let mut left_idx;
    let mut right_idx;
    for op in ops {
        let old_range = op.old_range();
        let new_range = op.new_range();
        let start_idx = to_print.len();
        to_print.extend(vec![
            (
                String::new(),
                String::new(),
                String::new(),
                String::new()
            );
            old_range.len().max(new_range.len())
        ]);
        left_idx = start_idx;
        right_idx = start_idx;
        for change in diff.iter_inline_changes(op) {
            let is_left = change.tag() == ChangeTag::Delete;
            if is_left {
                to_print[left_idx].0 +=
                    &style(format!("{:>4}", left_lineno)).on_green().to_string();
            }
            let is_right = change.tag() == ChangeTag::Insert;
            if is_right {
                to_print[right_idx].2 +=
                    &style(format!("{:>4}", right_lineno)).on_red().to_string();
            }
            let is_both = change.tag() == ChangeTag::Equal;
            if is_both {
                to_print[left_idx].0 += &style(format!("{:>4}", left_lineno)).dim().to_string();
                to_print[right_idx].2 += &style(format!("{:>4}", right_lineno)).dim().to_string();
            }
            for (highlight, s) in change.iter_strings_lossy() {
                let mut s = s.trim_end().to_string();
                if highlight {
                    s = if is_left {
                        style(s).green().to_string()
                    } else {
                        style(s).red().to_string()
                    };
                }
                if is_left || is_both {
                    to_print[left_idx].1 += &s;
                }
                if is_right || is_both {
                    to_print[right_idx].3 += &s;
                }
            }
            if is_left || is_both {
                left_idx += 1;
                left_lineno += 1;
            }
            if is_right || is_both {
                right_idx += 1;
                right_lineno += 1;
            }
        }
    }
    for (left_lineno, left, right_lineno, right) in to_print {
        let left = pad_str(&left, left_width, Alignment::Left, None);
        println!("{:4}{}{}{}", left_lineno, left, right_lineno, right);
    }
}
