use crate::data::{ExampleIO, Problem, ProblemId, ProblemKind};
use crate::infra::console::Spinner;
use crate::infra::subprocess::{spawn_cmd_background, run_silent};
use std::future::Future;
use std::process::Stdio;
use thirtyfour::common::cookie::SameSite;
use thirtyfour::prelude::*;
use tokio::runtime;

/// Takes care of interaction with BOJ pages. Internally uses headless Firefox and geckodriver.
pub(crate) struct Browser {
    webdriver: WebDriver,
}

fn with_async_runtime<F, R>(future: F) -> anyhow::Result<R>
where
    F: Future<Output = anyhow::Result<R>>,
{
    let rt = runtime::Builder::new_current_thread()
        .enable_time()
        .enable_io()
        .build()?;
    rt.block_on(future)
}

impl Browser {
    /// Creates a new browser context. This method handles AWS WAF challenge.
    pub(crate) fn new() -> anyhow::Result<Self> {
        with_async_runtime(async {
            let spinner = Spinner::new("Starting geckodriver...");
            spawn_cmd_background("geckodriver")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?;
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;

            spinner.set_message("Starting Firefox...");
            // Use headless firefox to allow running without a graphic device
            let mut caps = DesiredCapabilities::firefox();
            caps.set_headless()?;
            // println!("webdriver initializing");
            let webdriver = WebDriver::new("http://localhost:4444", caps).await?;
            // println!("webdriver initialized");

            spinner.set_message("Waiting for redirect to acmicpc.net...");
            // Handle AWS WAF challenge
            webdriver.get("https://www.acmicpc.net").await?;

            // If this container exists, AWS challenge is activated; wait until refresh starts
            let challenge_elem = webdriver
                .query(By::Id("challenge-container"))
                .first_opt()
                .await?;
            if let Some(elem) = challenge_elem {
                elem.wait_until().stale().await?;
            }

            spinner.finish("Browser initialization complete");
            Ok(Self {
                webdriver,
            })
        })
    }

    /// Sets BOJ credential cookies.
    pub(crate) fn login(&self, bojautologin: &str, onlinejudge: &str) -> anyhow::Result<()> {
        with_async_runtime(async {
            let driver = &self.webdriver;

            // Browser is already on acmicpc.net; safe to set cookies
            let mut cookie = Cookie::new("bojautologin", bojautologin);
            cookie.set_domain(".acmicpc.net");
            cookie.set_path("/");
            cookie.set_same_site(SameSite::Lax);
            driver.add_cookie(cookie.clone()).await?;
            let mut cookie = Cookie::new("OnlineJudge", onlinejudge);
            cookie.set_domain(".acmicpc.net");
            cookie.set_path("/");
            cookie.set_same_site(SameSite::Lax);
            driver.add_cookie(cookie.clone()).await?;
            driver.get("https://www.acmicpc.net").await?;
            Ok(())
        })
    }

    pub(crate) fn get_username(&self) -> anyhow::Result<Option<String>> {
        with_async_runtime(async {
            let driver = &self.webdriver;
            // Browser is already on acmicpc.net
            let username_elem = driver.query(By::ClassName("username")).first_opt().await?;
            let username = if let Some(elem) = username_elem {
                Some(elem.text().await?)
            } else {
                None
            };
            Ok(username)
        })
    }

    /// Fetches relevant information of the given problem.
    pub(crate) fn get_problem(&self, problem_id: &ProblemId) -> anyhow::Result<Problem> {
        with_async_runtime(async {
            let driver = &self.webdriver;
            let problem_page = problem_id.problem_url();
            driver.get(problem_page).await?;
            let title = driver.find(By::Id("problem_title")).await?.text().await?;
            let label_elems = driver.find_all(By::ClassName("problem-label")).await?;
            let mut kind = vec![];
            for label_elem in label_elems {
                let class = label_elem.class_name().await?.unwrap_or(String::new());
                let text = label_elem.text().await?;
                if let Ok(cur_kind) = ProblemKind::from_class_and_text(&class, &text) {
                    kind.push(cur_kind);
                }
            }
            let problem_info_elems = driver
                .find_all(By::Css("#problem-info tbody tr td"))
                .await?;
            let time_limit = if let Some(elem) = problem_info_elems.first() {
                elem.text().await?
            } else {
                "? seconds".to_string()
            };
            let memory_limit = if let Some(elem) = problem_info_elems.get(1) {
                elem.text().await?
            } else {
                "? MB".to_string()
            };
            let time = time_limit
                .split(' ')
                .next()
                .unwrap()
                .parse::<f64>()
                .unwrap();
            let memory = memory_limit
                .split(' ')
                .next()
                .unwrap()
                .parse::<f64>()
                .unwrap();
            let time_bonus = !time_limit.contains('(');
            let memory_bonus = !memory_limit.contains('(');
            let mut io = vec![];
            let sample_elems = driver.find_all(By::ClassName("sampledata")).await?;
            for sample in sample_elems.chunks_exact(2) {
                let input = sample[0].text().await?;
                let output = sample[1].text().await?;
                io.push(ExampleIO { input, output });
            }
            Ok(Problem {
                id: problem_id.clone(),
                title,
                kind,
                time,
                time_bonus,
                memory,
                memory_bonus,
                io,
            })
        })
    }

    /// Submits source code via submit page.
    pub(crate) fn submit_solution(
        &self,
        problem_id: &ProblemId,
        source: &str,
        language: &str,
    ) -> anyhow::Result<()> {
        with_async_runtime(async {
            let driver = &self.webdriver;
            let submit_page = problem_id.submit_url();
            driver.get(submit_page).await?;

            // Set language: click dropdown, search name, select first item
            let lang_elem = driver.query(By::ClassName("chosen-single")).first().await?;
            lang_elem.click().await?;
            let lang_search_elem = driver
                .query(By::ClassName("chosen-search-input"))
                .first()
                .await?;
            lang_search_elem.send_keys(language).await?;
            let lang_found_elem = driver
                .query(By::Css(".active-result.highlighted"))
                .first()
                .await?;
            lang_found_elem.click().await?;

            // Set source: https://stackoverflow.com/a/57621139/4595904 simplified
            // `send_keys` is incorrect, as bracket/quote matching will be triggered as the source code is typed,
            // resulting in CE (https://www.acmicpc.net/source/78678130)
            // Clipboard API seems to require user permission, so inject the string to CodeMirror instance
            driver
                .execute(
                    "document.querySelector('.CodeMirror').CodeMirror.setValue(arguments[0])",
                    vec![serde_json::to_value(source)?],
                )
                .await?;

            // Submit and wait until refresh starts
            let submit_elem = driver.query(By::Id("submit_button")).first().await?;
            submit_elem.click().await?;
            submit_elem.wait_until().stale().await?;
            Ok(())
        })
    }

    /// On submission status page, returns (status text, status class).
    pub(crate) fn get_submission_status(&self) -> anyhow::Result<(String, String)> {
        with_async_runtime(async {
            let driver = &self.webdriver;
            let elem_status = driver.query(By::ClassName("result-text")).first().await?;
            let status = elem_status.text().await?.to_string();
            let class = elem_status.class_name().await?.unwrap_or(String::new());
            Ok((status, class))
        })
    }

    /// Gracefully terminate the browser. Should be called even on error.
    pub(crate) fn quit(self) -> anyhow::Result<()> {
        with_async_runtime(async {
            self.webdriver.quit().await?;
            run_silent("kill $(pidof geckodriver)").ok();
            Ok(())
        })
    }
}
