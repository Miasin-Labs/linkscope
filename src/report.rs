use crate::model::Snapshot;

pub(crate) fn render(snapshot: &Snapshot) {
    if snapshot.total_nanos == 0 {
        return;
    }
    eprintln!("\n── linkscope --stats ──────────────────────────────────────");
    eprintln!(
        "{:<18} {:>10} {:>7} {:>7} {:>12} {:>10} {:>12}",
        "phase", "wall", "%", "spans", "bytes", "items", "rate"
    );

    let mut timed_ns: u128 = 0;
    for row in &snapshot.phases {
        timed_ns += row.nanos;
        let pct = (row.nanos as f64 / snapshot.total_nanos as f64) * 100.0;
        eprintln!(
            "{:<18} {:>10} {:>6.1}% {:>7} {:>12} {:>10} {:>12}",
            row.name,
            crate::fmt::fmt_ns(row.nanos),
            pct,
            row.spans,
            crate::fmt::human(row.bytes),
            row.items,
            crate::fmt::rate(row.bytes, row.items, row.nanos),
        );
    }

    let other = snapshot.total_nanos.saturating_sub(timed_ns);
    eprintln!(
        "{:<18} {:>10} {:>6.1}%  (startup/teardown/untimed)",
        "other",
        crate::fmt::fmt_ns(other),
        (other as f64 / snapshot.total_nanos as f64) * 100.0
    );
    eprintln!(
        "{:<18} {:>10}",
        "TOTAL",
        crate::fmt::fmt_ns(snapshot.total_nanos)
    );

    crate::counters::report(&snapshot.counters);
    crate::rss::report(&snapshot.rss);
    eprintln!("───────────────────────────────────────────────────────────");
    crate::trace_render::trace_tree_from_snapshot(&snapshot.traces);
}
