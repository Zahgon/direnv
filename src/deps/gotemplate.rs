//! The sliver of Go's `text/template` that `direnv hook` needs.
//!
//! Every hook script is a fixed string containing at most one kind of action,
//! `{{.SelfPath}}`, substituted from a context with a single field. Rendering
//! it is a scan for `{{ ... }}` with the contents trimmed; anything other than
//! `.SelfPath` is an error rather than a silent empty substitution, so a
//! malformed hook cannot quietly emit a broken script.

/// The variables available during hook template evaluation.
pub struct HookContext {
    /// The unescaped absolute path to direnv.
    pub self_path: String,
}

/// Render `template` against `ctx`.
pub fn execute(name: &str, template: &str, ctx: &HookContext) -> Result<String, String> {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let end = after
            .find("}}")
            .ok_or_else(|| format!("template: {name}: unclosed action"))?;
        let action = after[..end].trim();
        match action {
            ".SelfPath" => out.push_str(&ctx.self_path),
            other => {
                return Err(format!(
                    "template: {name}: can't evaluate field {other} in type HookContext"
                ))
            }
        }
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    Ok(out)
}
