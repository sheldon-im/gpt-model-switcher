//! Changes both next-turn settings together without opening menus, sending keys, or setting focus.
use crate::desktop_ipc::Desktop;
use serde_json::Value;
use switcher_core::{same_model, switching::Outcome, Preset, Selection};
pub trait Context {
    fn guard(&mut self) -> Result<Selection, String>;
}
pub trait Connection {
    fn validate(&self, _preset: &Preset) -> Result<(), String> {
        Ok(())
    }
    fn read(&mut self, id: &str) -> Result<Value, String>;
    fn update(
        &mut self,
        id: &str,
        before: &Value,
        model: &str,
        effort: Value,
    ) -> Result<bool, String>;
}
impl Connection for Desktop {
    fn validate(&self, preset: &Preset) -> Result<(), String> {
        crate::desktop_ipc::validate_catalog(preset)
    }
    fn read(&mut self, id: &str) -> Result<Value, String> {
        Desktop::read(self, id)
    }
    fn update(
        &mut self,
        id: &str,
        before: &Value,
        model: &str,
        effort: Value,
    ) -> Result<bool, String> {
        Desktop::update(self, id, before, model, effort)
    }
}
fn raw_matches(v: &Value, p: &Preset) -> bool {
    v["model"].as_str().is_some_and(|m| same_model(m, &p.model)) && v["reasoningEffort"] == p.effort
}
pub fn apply(
    context: &mut impl Context,
    conn: &mut impl Connection,
    id: &str,
    p: &Preset,
) -> Outcome {
    let mut before_visible = None;
    let mut before_raw = None;
    let mut attempted = false;
    let mut already_applied = false;
    let result = (|| {
        let visible = context.guard()?;
        before_visible = Some(visible.clone());
        let raw = conn.read(id)?;
        if !raw["model"]
            .as_str()
            .is_some_and(|m| same_model(m, &visible.model))
            || raw["reasoningEffort"]
                .as_str()
                .is_some_and(|e| e != visible.effort)
        {
            return Err(
                "화면과 앱 설정이 일치하지 않습니다. / Visible and internal settings disagree."
                    .into(),
            );
        }
        before_raw = Some(raw.clone());
        if visible.matches(p) {
            let verified = context.guard()?;
            if !verified.matches(p) {
                return Err(
                    "확인 중 선택값이 바뀌었습니다. / Selection changed during verification."
                        .into(),
                );
            }
            already_applied = true;
            return Ok(verified);
        }
        conn.validate(p)?;
        if context.guard()? != visible {
            return Err("전환 직전에 화면 선택값이 바뀌었습니다. / Visible selection changed before switching.".into());
        }
        attempted = true;
        if !conn.update(id, &raw, &p.model, Value::String(p.effort.clone()))? {
            return Err(
                "전환 직전에 설정이 바뀌었습니다. / Settings changed before switching.".into(),
            );
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(1200);
        loop {
            let actual = context.guard()?;
            let internal = conn.read(id)?;
            if actual.matches(p) && raw_matches(&internal, p) {
                return Ok(actual);
            }
            if std::time::Instant::now() > deadline {
                return Err(
                    "실제 선택값 검증에 실패했습니다. / Selection verification failed.".into(),
                );
            }
            std::thread::sleep(std::time::Duration::from_millis(40));
        }
    })();
    match result {
        Ok(actual) => Outcome {
            success: true,
            already_applied,
            message: format!("{} · {}", p.model, p.effort),
            actual: Some(actual),
            restored: false,
        },
        Err(message) => {
            // Restore only if context/focus are unchanged and the owner still reports our requested pair.
            // Never override a newer user choice, another window, or an unknown/changed context.
            let mut restored = false;
            if attempted && context.guard().is_ok() {
                if let (Some(original), Some(visible)) = (&before_raw, &before_visible) {
                    if let Ok(current) = conn.read(id) {
                        if raw_matches(&current, p) {
                            if let Some(model) = original["model"].as_str() {
                                if conn
                                    .update(
                                        id,
                                        &current,
                                        model,
                                        original["reasoningEffort"].clone(),
                                    )
                                    .unwrap_or(false)
                                {
                                    restored =
                                        context.guard().is_ok_and(|actual| actual == *visible)
                                            && conn.read(id).is_ok_and(|raw| {
                                                raw["model"] == original["model"]
                                                    && raw["reasoningEffort"]
                                                        == original["reasoningEffort"]
                                            });
                                }
                            }
                        }
                    }
                }
            }
            let actual = context.guard().ok();
            Outcome {
                success: false,
                already_applied: false,
                message,
                actual,
                restored,
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use switcher_core::Settings;
    struct Mock {
        raw: Value,
        writes: usize,
        reject: bool,
        lie: bool,
    }
    struct Native {
        changed: bool,
        selection: Selection,
        after: Option<Selection>,
        reads: usize,
        change_after_write: bool,
        edit_before_write: bool,
    }
    impl Context for Native {
        fn guard(&mut self) -> Result<Selection, String> {
            self.reads += 1;
            if self.changed || self.change_after_write && self.reads >= 3 {
                Err("context/focus/busy changed".into())
            } else {
                Ok(if self.edit_before_write && self.reads == 2 {
                    Selection {
                        model: self.selection.model.clone(),
                        effort: "low".into(),
                    }
                } else if self.reads >= 3 {
                    self.after.as_ref().unwrap_or(&self.selection).clone()
                } else {
                    self.selection.clone()
                })
            }
        }
    }
    impl Connection for Mock {
        fn read(&mut self, _: &str) -> Result<Value, String> {
            Ok(self.raw.clone())
        }
        fn update(&mut self, _: &str, b: &Value, m: &str, e: Value) -> Result<bool, String> {
            self.writes += 1;
            assert_eq!(b["model"], self.raw["model"]);
            assert_eq!(b["reasoningEffort"], self.raw["reasoningEffort"]);
            if self.reject {
                return Ok(false);
            }
            if !self.lie {
                self.raw = json!({"model":m,"reasoningEffort":e})
            }
            Ok(true)
        }
    }
    fn setup() -> (Native, Mock) {
        let s = Selection {
            model: "gpt-6.1-sol".into(),
            effort: "high".into(),
        };
        (
            Native {
                changed: false,
                selection: s,
                after: None,
                reads: 0,
                change_after_write: false,
                edit_before_write: false,
            },
            Mock {
                raw: json!({"model":"gpt-6.1-sol","reasoningEffort":null}),
                writes: 0,
                reject: false,
                lie: false,
            },
        )
    }
    #[test]
    fn busy_or_focus_change_blocks_all_writes() {
        let (mut n, mut c) = setup();
        n.changed = true;
        assert!(!apply(&mut n, &mut c, "id", &Settings::default().presets[0]).success);
        assert_eq!(c.writes, 0)
    }
    #[test]
    fn no_op_does_not_mutate_or_move_focus() {
        let (mut n, mut c) = setup();
        let result = apply(&mut n, &mut c, "id", &Settings::default().presets[1]);
        assert!(result.success && result.already_applied);
        assert_eq!(c.writes, 0)
    }
    #[test]
    fn repeated_preset_presses_read_actual_values_without_any_write() {
        let (mut n, mut c) = setup();
        let p = &Settings::default().presets[1];
        for _ in 0..3 {
            let result = apply(&mut n, &mut c, "id", p);
            assert!(result.success && result.already_applied);
            assert_eq!(result.actual.as_ref().unwrap().effort, "high");
        }
        assert_eq!(c.writes, 0);
        assert!(c.raw["reasoningEffort"].is_null());
    }
    #[test]
    fn same_model_with_a_different_effort_is_a_real_change() {
        let (mut n, mut c) = setup();
        let p = &Settings::default().presets[1];
        n.selection.effort = "low".into();
        c.raw["reasoningEffort"] = json!("low");
        n.after = Some(Selection {
            model: p.model.clone(),
            effort: p.effort.clone(),
        });
        let result = apply(&mut n, &mut c, "id", p);
        assert!(result.success && !result.already_applied);
        assert_eq!(c.writes, 1);
    }
    #[test]
    fn no_op_is_not_claimed_when_the_last_selection_check_changes() {
        let (mut n, mut c) = setup();
        n.edit_before_write = true;
        let result = apply(&mut n, &mut c, "id", &Settings::default().presets[1]);
        assert!(!result.success && !result.already_applied);
        assert_eq!(c.writes, 0);
    }
    #[test]
    fn changed_visible_default_does_not_get_overwritten() {
        let (mut n, mut c) = setup();
        n.edit_before_write = true;
        assert!(!apply(&mut n, &mut c, "id", &Settings::default().presets[0]).success);
        assert_eq!(c.writes, 0);
    }
    #[test]
    fn conditional_rejection_does_not_restore_over_new_choice() {
        let (mut n, mut c) = setup();
        c.reject = true;
        assert!(!apply(&mut n, &mut c, "id", &Settings::default().presets[0]).success);
        assert_eq!(c.writes, 1)
    }
    #[test]
    fn internal_and_visible_disagreement_never_mutates() {
        let (mut n, mut c) = setup();
        c.raw["reasoningEffort"] = json!("low");
        assert!(!apply(&mut n, &mut c, "id", &Settings::default().presets[0]).success);
        assert_eq!(c.writes, 0)
    }
    #[test]
    fn unverified_ui_change_is_failure_and_conditionally_restored() {
        let (mut n, mut c) = setup();
        let o = apply(&mut n, &mut c, "id", &Settings::default().presets[0]);
        assert!(!o.success);
        assert!(o.restored);
        assert_eq!(c.writes, 2);
        assert_eq!(c.raw["reasoningEffort"], Value::Null)
    }
    #[test]
    fn paired_update_verifies_internal_and_visible_state() {
        let (mut n, mut c) = setup();
        let p = &Settings::default().presets[0];
        n.after = Some(Selection {
            model: p.model.clone(),
            effort: p.effort.clone(),
        });
        assert!(apply(&mut n, &mut c, "id", p).success);
        assert_eq!(c.writes, 1)
    }
    #[test]
    fn focus_change_after_write_cannot_trigger_blind_restore() {
        let (mut n, mut c) = setup();
        n.change_after_write = true;
        let o = apply(&mut n, &mut c, "id", &Settings::default().presets[0]);
        assert!(!o.success);
        assert!(!o.restored);
        assert_eq!(c.writes, 1);
        assert!(o.actual.is_none())
    }
}
