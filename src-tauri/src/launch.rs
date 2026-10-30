#[derive(Debug, PartialEq, Eq)]
pub enum Action { Settings, Background, Apply(String), Diagnostics }

pub fn action(args: &[String], initial: bool) -> Result<Action, String> {
    let explicit: Vec<_> = args.iter().skip(1).filter(|a|
        a.as_str() == "--settings" || a.as_str() == "--background" || a.as_str() == "--diagnose-runtime" || a.starts_with("--apply=")
    ).collect();
    if explicit.len() > 1 { return Err("Conflicting launch actions / 실행 요청이 중복되었습니다.".into()); }
    match explicit.first().map(|s| s.as_str()) {
        Some("--settings") => Ok(Action::Settings),
        Some("--background") => Ok(Action::Background),
        Some("--diagnose-runtime") => Ok(Action::Diagnostics),
        Some(arg) => {
            let id = arg.strip_prefix("--apply=").unwrap_or_default();
            if id.is_empty() { Err("Missing preset ID / 프리셋 ID가 없습니다.".into()) }
            else { Ok(Action::Apply(id.into())) }
        }
        None if initial && !cfg!(target_os="macos") => Ok(Action::Settings),
        None => Ok(Action::Background),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn args(flags: &[&str]) -> Vec<String> {
        std::iter::once("switcher").chain(flags.iter().copied()).map(str::to_owned).collect()
    }
    #[test] fn repeated_launch_never_opens_settings_implicitly() {
        assert_eq!(action(&args(&[]), false).unwrap(), Action::Background);
        assert_eq!(action(&args(&["--background"]), false).unwrap(), Action::Background);
        assert_eq!(action(&args(&["--settings"]), false).unwrap(), Action::Settings);
        assert_eq!(action(&args(&["--diagnose-runtime"]), false).unwrap(), Action::Diagnostics);
        assert_eq!(action(&args(&[]), true).unwrap(), if cfg!(target_os="macos") {Action::Background} else {Action::Settings});
    }
    #[test] fn explicit_preset_requests_keep_their_identity_and_do_not_open_settings() {
        for initial in [false,true] {
            assert_eq!(action(&args(&["--apply=daily"]), initial).unwrap(), Action::Apply("daily".into()));
            assert_eq!(action(&args(&["--apply=coding"]), initial).unwrap(), Action::Apply("coding".into()));
        }
        assert!(action(&args(&["--apply="]),false).is_err());
        assert!(action(&args(&["--apply=daily","--settings"]),false).is_err());
    }
}
