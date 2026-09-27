//! The Claude Code session JSON piped to the status line on stdin (model and context window).
//! Every field is optional: Claude Code omits context numbers before the first API response.

use serde_json::Value;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Session {
    /// API model ID Claude Code sends (matches LiteLLM's model group names).
    pub model_id: Option<String>,
    /// Friendly name, e.g. "Sonnet".
    pub model_name: Option<String>,
    /// Percent of the context window used (input tokens only, 0–100).
    pub context_pct: Option<f64>,
    /// Tokens currently in the context window.
    pub context_tokens: Option<u64>,
    /// Context window size in tokens.
    pub context_size: Option<u64>,
}

impl Session {
    pub fn from_input(v: &Value) -> Session {
        let str_at = |p: &str| v.pointer(p).and_then(Value::as_str).filter(|s| !s.is_empty()).map(str::to_string);
        Session {
            model_id: str_at("/model/id"),
            model_name: str_at("/model/display_name"),
            context_pct: v.pointer("/context_window/used_percentage").and_then(Value::as_f64),
            context_tokens: v.pointer("/context_window/total_input_tokens").and_then(Value::as_u64).filter(|t| *t > 0),
            context_size: v.pointer("/context_window/context_window_size").and_then(Value::as_u64).filter(|t| *t > 0),
        }
    }

    /// Name to show: the friendly name, else the model ID.
    pub fn display_model(&self) -> Option<&str> {
        self.model_name.as_deref().or(self.model_id.as_deref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_model_and_context() {
        let v: Value = serde_json::from_str(
            r#"{"model":{"id":"claude-sonnet-4-6","display_name":"Sonnet 4.6"},
                "context_window":{"total_input_tokens":84000,"context_window_size":200000,"used_percentage":42}}"#,
        )
        .unwrap();
        let s = Session::from_input(&v);
        assert_eq!(s.model_id.as_deref(), Some("claude-sonnet-4-6"));
        assert_eq!(s.display_model(), Some("Sonnet 4.6"));
        assert_eq!(s.context_pct, Some(42.0));
        assert_eq!(s.context_tokens, Some(84_000));
        assert_eq!(s.context_size, Some(200_000));
    }

    #[test]
    fn tolerates_missing_fields() {
        let s = Session::from_input(&serde_json::json!({"model":{"id":"opus","display_name":""}}));
        assert_eq!(s.display_model(), Some("opus"));
        assert_eq!(s.context_pct, None);
        assert_eq!(Session::from_input(&serde_json::json!({})), Session::default());
    }
}
