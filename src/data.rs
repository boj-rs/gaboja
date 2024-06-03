#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum ProblemId {
    Problem(String),
    ContestProblem(String),
}

#[derive(Debug)]
pub(crate) struct ParseError {
    input: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.input.fmt(f)
    }
}
impl std::error::Error for ParseError {}

impl std::str::FromStr for ProblemId {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.bytes().all(|b| b.is_ascii_digit() || b == b'/') {
            let slash_count = s.bytes().filter(|&b| b == b'/').count();
            match slash_count {
                0 => Ok(ProblemId::Problem(s.to_string())),
                1 => Ok(ProblemId::ContestProblem(s.to_string())),
                _ => Err(ParseError {
                    input: s.to_string(),
                }),
            }
        } else {
            Err(ParseError {
                input: s.to_string(),
            })
        }
    }
}

impl std::fmt::Display for ProblemId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProblemId::Problem(problem) => write!(f, "{}", problem),
            ProblemId::ContestProblem(problem) => write!(f, "{}", problem),
        }
    }
}

impl ProblemId {
    pub(crate) fn problem_url(&self) -> String {
        match self {
            Self::Problem(id) => format!("https://www.acmicpc.net/problem/{}", id),
            Self::ContestProblem(id) => format!("https://www.acmicpc.net/contest/problem/{}", id),
        }
    }

    pub(crate) fn submit_url(&self) -> String {
        match self {
            Self::Problem(id) => format!("https://www.acmicpc.net/submit/{}", id),
            Self::ContestProblem(id) => format!("https://www.acmicpc.net/contest/submit/{}", id),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ExampleIO {
    pub(crate) input: String,
    pub(crate) output: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ProblemKind {
    SpecialJudge,       // spj
    Subtask,            // subtask
    PartialScore,       // partial
    FunctionImpl,       // func
    Interactive,        // interactive
    TwoSteps,           // two-steps
    FullGrade,          // full
    Unofficial,         // unofficial
    Preparing,          // preparing
    LanguageRestrict,   // language-restrict
    ClassImpl,          // class
    Feedback,           // feedback
    TimeAccum,          // time-acc
    RandomKiller,       // random-killer
    SubmitLimit(usize), // submit-limit
}

impl ProblemKind {
    pub(crate) fn from_class_and_text(class: &str, text: &str) -> Result<Self, ParseError> {
        for (class_name, kind) in [
            ("problem-label-spj", Self::SpecialJudge),
            ("problem-label-subtask", Self::Subtask),
            ("problem-label-partial", Self::PartialScore),
            ("problem-label-func", Self::FunctionImpl),
            ("problem-label-interactive", Self::Interactive),
            ("problem-label-two-steps", Self::TwoSteps),
            ("problem-label-full", Self::FullGrade),
            ("problem-label-unofficial", Self::Unofficial),
            ("problem-label-preparing", Self::Preparing),
            ("problem-label-language-restrict", Self::LanguageRestrict),
            ("problem-label-class", Self::ClassImpl),
            ("problem-label-feedback", Self::Feedback),
            ("problem-label-time-acc", Self::TimeAccum),
            ("problem-label-random-killer", Self::RandomKiller),
        ] {
            if class.contains(class_name) {
                return Ok(kind);
            }
        }
        if class.contains("problem-label-submit-limit") {
            if let Some(count) = text.split(' ').last() {
                if let Ok(count) = count.parse::<usize>() {
                    return Ok(Self::SubmitLimit(count));
                }
            }
        }
        Err(ParseError {
            input: class.to_string(),
        })
    }

    pub(crate) fn no_run(&self) -> Option<&'static str> {
        match self {
            Self::FunctionImpl => Some("function implementation"),
            Self::ClassImpl => Some("class implementation"),
            _ => None,
        }
    }

    pub(crate) fn no_test(&self) -> Option<&'static str> {
        match self {
            Self::FunctionImpl => Some("function implementation"),
            Self::ClassImpl => Some("class implementation"),
            Self::Interactive => Some("interactive"),
            Self::TwoSteps => Some("two steps"),
            _ => None,
        }
    }

    pub(crate) fn no_diff(&self) -> Option<&'static str> {
        match self {
            Self::SpecialJudge => Some("special judge"),
            Self::PartialScore => Some("partial score"),
            _ => None,
        }
    }

    pub(crate) fn is_interactive(&self) -> bool {
        matches!(self, Self::Interactive)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Problem {
    pub(crate) id: ProblemId,
    pub(crate) title: String,
    pub(crate) kind: Vec<ProblemKind>,
    pub(crate) time: f64,
    pub(crate) time_bonus: bool,
    pub(crate) memory: f64,
    pub(crate) memory_bonus: bool,
    pub(crate) io: Vec<ExampleIO>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub(crate) struct Credentials {
    pub(crate) bojautologin: String,
    pub(crate) onlinejudge: String,
}

#[derive(Clone, serde::Deserialize)]
pub(crate) struct Preset {
    pub(crate) name: String,
    pub(crate) credentials: Option<Credentials>,
    pub(crate) lang: Option<String>,
    pub(crate) file: Option<String>,
    pub(crate) init: Option<String>,
    pub(crate) build: Option<String>,
    pub(crate) cmd: Option<String>,
    pub(crate) input: Option<String>,
}

#[derive(serde::Deserialize)]
pub(crate) struct BojConfig {
    pub(crate) start: Option<String>,
    pub(crate) preset: Vec<Preset>,
}
