//! Literal, case-insensitive package filtering. Whitespace-separated terms all match.
pub fn terms(query: &str) -> Vec<String> {
    query.split_whitespace().map(str::to_lowercase).collect()
}
pub fn matches(name: &str, id: &str, terms: &[String]) -> bool {
    if terms.is_empty() {
        return true;
    }
    let name = name.to_lowercase();
    let id = id.to_lowercase();
    terms
        .iter()
        .all(|term| name.contains(term) || id.contains(term))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_filter_and_case_insensitive_partial_names() {
        assert!(matches(
            "Visual Studio Code",
            "Microsoft.VisualStudioCode",
            &terms("   ")
        ));
        assert!(matches(
            "Visual Studio Code",
            "Microsoft.VisualStudioCode",
            &terms("VISUAL code")
        ));
    }
    #[test]
    fn terms_can_match_across_name_and_id_but_all_are_required() {
        assert!(matches(
            "Code Editor",
            "Microsoft.VSCode",
            &terms("editor microsoft")
        ));
        assert!(!matches(
            "Code Editor",
            "Microsoft.VSCode",
            &terms("editor google")
        ));
    }
    #[test]
    fn unicode_and_punctuation_are_literal() {
        assert!(matches(
            "반디집",
            "Bandisoft.Bandizip",
            &terms("반디 BANDISOFT.")
        ));
        assert!(!matches("Code", "Microsoft.VSCode", &terms(".*")));
    }
}
