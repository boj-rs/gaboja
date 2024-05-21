use crate::data::{ProblemId, ExampleIO, ProblemKind, Problem};
use std::process::{Command, Stdio, Child};
use thirtyfour_sync::prelude::*;
use thirtyfour_sync::common::cookie::SameSite;

/// Takes care of interaction with BOJ pages. Internally uses headless Firefox and geckodriver.
pub(crate) struct Browser {
    geckodriver: Child,
    webdriver: WebDriver,
}

impl Browser {
    /// Creates a new browser context. This method handles AWS WAF challenge.
    pub(crate) fn new() -> anyhow::Result<Self> {
        let geckodriver = Command::new("geckodriver").stdout(Stdio::null()).stderr(Stdio::null()).spawn()?;
        // Use headless firefox to allow running without a graphic device
        let mut caps = DesiredCapabilities::firefox();
        caps.set_headless()?;
        let webdriver = WebDriver::new("http://localhost:4444", caps)?;

        // Handle AWS WAF challenge
        webdriver.get("https://www.acmicpc.net")?;

        // If this container exists, AWS challenge is activated; wait until refresh starts
        let challenge_elem = webdriver.query(By::Id("challenge-container")).first_opt()?;
        if let Some(elem) = challenge_elem {
            elem.wait_until().stale()?;
        }
        Ok(Self {
            geckodriver,
            webdriver
        })
    }

    /// Sets BOJ credential cookies.
    pub(crate) fn login(&self, bojautologin: &str, onlinejudge: &str) -> WebDriverResult<()> {
        let driver = &self.webdriver;

        // Browser is already on acmicpc.net; safe to set cookies
        let mut cookie = Cookie::new("bojautologin", bojautologin.into());
        cookie.set_domain(Some(".acmicpc.net".to_string()));
        cookie.set_path(Some("/".to_string()));
        cookie.set_same_site(Some(SameSite::Lax));
        driver.add_cookie(cookie.clone())?;
        let mut cookie = Cookie::new("OnlineJudge", onlinejudge.into());
        cookie.set_domain(Some(".acmicpc.net".to_string()));
        cookie.set_path(Some("/".to_string()));
        cookie.set_same_site(Some(SameSite::Lax));
        driver.add_cookie(cookie.clone())?;
        driver.get("https://www.acmicpc.net")?;
        Ok(())
    }

    pub(crate) fn get_username(&self) -> WebDriverResult<Option<String>> {
        let driver = &self.webdriver;
        driver.get("https://www.acmicpc.net")?;
        let username_elem = driver.query(By::ClassName("username")).first_opt()?;
        let username = if let Some(elem) = username_elem {
            Some(elem.text()?)
        } else {
            None
        };
        Ok(username)
    }

    /// Fetches relevant information of the given problem.
    pub(crate) fn get_problem(&self, problem_id: &ProblemId) -> WebDriverResult<Problem> {
        let driver = &self.webdriver;
        let problem_page = problem_id.problem_url();
        driver.get(problem_page)?;
        let title = driver.find_element(By::Id("problem_title"))?.text()?;
        let label_elems = driver.find_elements(By::ClassName("problem-label"))?;
        let mut kind = vec![];
        for label_elem in label_elems {
            let class = label_elem.class_name()?.unwrap_or(String::new());
            let text = label_elem.text()?;
            if let Ok(cur_kind) = ProblemKind::from_class_and_text(&class, &text) {
                kind.push(cur_kind);
            }
        }
        let problem_info_elems = driver.find_elements(By::Css("#problem-info tbody tr td"))?;
        let time_limit = if let Some(elem) = problem_info_elems.get(0) {
            elem.text()?
        } else { "? seconds".to_string() };
        let memory_limit = if let Some(elem) = problem_info_elems.get(1) {
            elem.text()?
        } else { "? MB".to_string() };
        let time = time_limit.split(' ').next().unwrap().parse::<f64>().unwrap();
        let memory = memory_limit.split(' ').next().unwrap().parse::<f64>().unwrap();
        let time_bonus = !time_limit.contains('(');
        let memory_bonus = !memory_limit.contains('(');
        let mut io = vec![];
        let sample_elems = driver.find_elements(By::ClassName("sampledata"))?;
        for sample in sample_elems.chunks_exact(2) {
            let input = sample[0].text()?;
            let output = sample[1].text()?;
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
    }

    /// Submits source code via submit page.
    pub(crate) fn submit_solution(&self, problem_id: &ProblemId, source: &str, language: &str) -> WebDriverResult<()> {
        let driver = &self.webdriver;
        let submit_page = problem_id.submit_url();
        driver.get(submit_page)?;

        // Set language: click dropdown, search name, select first item
        let lang_elem = driver.query(By::ClassName("chosen-single")).first()?;
        lang_elem.click()?;
        let lang_search_elem = driver.query(By::ClassName("chosen-search-input")).first()?;
        lang_search_elem.send_keys(language)?;
        let lang_found_elem = driver.query(By::Css(".active-result.highlighted")).first()?;
        lang_found_elem.click()?;

        // Set source: https://stackoverflow.com/a/57621139/4595904 simplified
        let codemirror_elem = driver.query(By::ClassName("CodeMirror-line")).first()?;
        codemirror_elem.click()?;
        let code_elem = driver.query(By::Css(".CodeMirror div textarea")).first()?;
        code_elem.send_keys(source)?;

        // Submit and wait until refresh starts
        let submit_elem = driver.query(By::Id("submit_button")).first()?;
        submit_elem.click()?;
        submit_elem.wait_until().stale()?;
        Ok(())
    }

    /// On submission status page, returns (status text, status class).
    pub(crate) fn get_submission_status(&self) -> WebDriverResult<(String, String)> {
        let driver = &self.webdriver;
        let elem_status = driver.query(By::ClassName("result-text")).first()?;
        let status = elem_status.text()?.to_string();
        let class = elem_status.class_name()?.unwrap_or(String::new());
        Ok((status, class))
    }

    /// Gracefully terminate the browser. Should be called even on error.
    pub(crate) fn quit(self) -> anyhow::Result<()> {
        let Self { mut geckodriver, webdriver } = self;
        webdriver.quit()?;
        geckodriver.kill()?;
        Ok(())
    }
}