use regex::Regex;
use std::sync::LazyLock;

static ANSI: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\x1b\[[0-9;?]*[ -/]*[@-~]").expect("valid ANSI regex"));
static AUTH: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(?:Authorization\s*:\s*)?(?:Bearer|Basic)\s+\S+").unwrap());
static SECRET_ASSIGNMENT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)\b(?:[A-Z][A-Z0-9_]*(?:KEY|TOKEN|SECRET|PASSWORD)|api[-_]?key|token|secret|password)\b\s*[:=]\s*(?:"[^"]*"|'[^']*'|\S+)"#).unwrap()
});
static TOKEN_SHAPE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(?:sk-[A-Za-z0-9_-]{12,}|ghp_[A-Za-z0-9]{12,}|github_pat_[A-Za-z0-9_]{12,}|AKIA[A-Z0-9]{12,})\b").unwrap()
});
static CONTROL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[\x00-\x08\x0b\x0c\x0e-\x1f\x7f]").unwrap());
static SPACE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());

pub fn sanitize_text(input: impl AsRef<str>) -> String {
    let mut text = input.as_ref().to_string();
    text = ANSI.replace_all(&text, "").into_owned();
    text = AUTH.replace_all(&text, "[redacted]").into_owned();
    text = SECRET_ASSIGNMENT
        .replace_all(&text, "[redacted]")
        .into_owned();
    text = TOKEN_SHAPE.replace_all(&text, "[redacted]").into_owned();
    if let Ok(home) = std::env::var("HOME")
        && !home.is_empty()
    {
        text = text.replace(&home, "~");
    }
    text = CONTROL.replace_all(&text, " ").into_owned();
    SPACE.replace_all(text.trim(), " ").into_owned()
}

pub fn bounded_text(input: impl AsRef<str>, max: usize) -> String {
    sanitize_text(input).chars().take(max).collect()
}
