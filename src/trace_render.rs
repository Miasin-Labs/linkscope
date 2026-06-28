use crate::model::TraceSnapshot;

pub fn trace_tree() {
    let snapshot = crate::snapshot();
    trace_tree_from_snapshot(&snapshot.traces);
}

pub(crate) fn trace_tree_from_snapshot(nodes: &[TraceSnapshot]) {
    if nodes.is_empty() {
        return;
    }
    let mut nodes = nodes.to_vec();
    nodes.sort_by_key(|node| node.enter_seq);
    let show_threads = nodes
        .first()
        .is_some_and(|first| nodes.iter().any(|node| node.thread != first.thread));
    eprintln!("\n── linkscope --trace (call flow) ──────────────────────────");
    for node in &nodes {
        let indent = "  ".repeat(node.depth);
        let file = node.file.rsplit('/').next().unwrap_or(&node.file);
        let thread = if show_threads {
            format!(" [{}]", node.thread)
        } else {
            String::new()
        };
        let fields = node
            .fields
            .iter()
            .map(|field| format!("{}={}", field.name, field.value))
            .collect::<Vec<_>>()
            .join(" ");
        let fields = if fields.is_empty() {
            String::new()
        } else {
            format!(" [{fields}]")
        };
        if node.is_event {
            let detail = if node.detail.is_empty() {
                String::new()
            } else {
                format!(": {}", node.detail)
            };
            eprintln!(
                "{indent}• {}{}{}{}  ({}:{})",
                node.label, detail, fields, thread, file, node.line
            );
        } else {
            eprintln!(
                "{indent}{} {:>9}{}{}  ({}:{})",
                node.label,
                crate::fmt::fmt_ns(node.nanos),
                fields,
                thread,
                file,
                node.line
            );
        }
        print_stack(&indent, &node.stack);
    }
    eprintln!("───────────────────────────────────────────────────────────");
}

fn print_stack(indent: &str, stack: &str) {
    if stack.is_empty() {
        return;
    }
    for line in stack.lines().take(32) {
        eprintln!("{indent}    ↳ {line}");
    }
}
