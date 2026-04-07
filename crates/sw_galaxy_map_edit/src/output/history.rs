//! Output helpers for audit history rows.

use crate::audit::history::EntityHistoryRow;

pub fn print_history_rows(rows: &[EntityHistoryRow]) {
    if rows.is_empty() {
        println!("No history entries found.");
        return;
    }

    for row in rows {
        println!("#{}", row.id);
        println!("When   : {}", row.edited_at);
        println!("Field  : {}", row.field_name);
        println!("Old    : {}", display_opt(&row.old_value));
        println!("New    : {}", display_opt(&row.new_value));
        println!("Reason : {}", display_opt(&row.reason));
        println!("Source : {}", display_opt(&row.source));
        println!();
    }
}

fn display_opt(value: &Option<String>) -> &str {
    match value.as_deref() {
        Some(v) if !v.trim().is_empty() => v,
        _ => "-",
    }
}