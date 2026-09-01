use crate::models::Candidate;
use comfy_table::{presets::UTF8_FULL, Cell, Table};

pub fn print_candidates(candidates: &[Candidate]) {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec!["ID", "Name", "Version", "Backend", "Origin"]);

    for c in candidates {
        table.add_row(vec![
            Cell::new(&c.canonical_id),
            Cell::new(&c.name),
            Cell::new(c.version.as_deref().unwrap_or("-")),
            Cell::new(&c.backend),
            Cell::new(c.origin.as_deref().unwrap_or("-")),
        ]);
    }
    println!("{table}");
}
