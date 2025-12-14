use std::collections::HashSet;

pub fn conditions_met(conditions: &[String], flags: &HashSet<String>) -> bool {
    for cond in conditions {
        if let Some(name) = cond.strip_prefix('!') {
            // Negated condition: flag must NOT be present
            if flags.contains(name) {
                return false;
            }
        } else {
            // Positive condition: flag must be present
            if !flags.contains(cond) {
                return false;
            }
        }
    }
    true
}
