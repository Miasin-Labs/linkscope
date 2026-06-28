use std::collections::HashMap;

use crate::{Field, FieldValue, Profile, Record};

impl Profile {
    #[must_use]
    pub fn to_mermaid_markdown(&self) -> String {
        format!("```mermaid\n{}```\n", self.to_mermaid_flowchart())
    }

    #[must_use]
    pub fn to_mermaid_flowchart(&self) -> String {
        profile_to_flowchart(self)
    }
}

pub(crate) fn current_markdown() -> String {
    crate::profile().to_mermaid_markdown()
}

fn profile_to_flowchart(profile: &Profile) -> String {
    let mut output = String::from("flowchart TD\n");
    if profile.records.is_empty() {
        output.push_str("    empty[\"No profile records\"]\n");
        return output;
    }

    let records = sorted_records(profile);
    for record in &records {
        push_node(&mut output, profile, record);
    }
    push_edges(&mut output, &records);
    output
}

fn sorted_records(profile: &Profile) -> Vec<&Record> {
    let mut records = profile.records.iter().collect::<Vec<_>>();
    records.sort_by_key(|record| record_id(record));
    records
}

fn push_node(output: &mut String, profile: &Profile, record: &Record) {
    output.push_str("    R");
    output.push_str(&record_id(record).to_string());
    match record {
        Record::Span { .. } => output.push_str("[\""),
        Record::Event { .. } => output.push_str("([\""),
    }
    output.push_str(&record_label(profile, record));
    match record {
        Record::Span { .. } => output.push_str("\"]\n"),
        Record::Event { .. } => output.push_str("\"])\n"),
    }
}

fn push_edges(output: &mut String, records: &[&Record]) {
    let mut stacks = HashMap::<usize, Vec<Option<u64>>>::new();
    for record in records {
        let depth = record_depth(record);
        let thread = record_thread(record);
        if let Some(parent) = parent_id(&stacks, thread, depth) {
            output.push_str("    R");
            output.push_str(&parent.to_string());
            output.push_str(" --> R");
            output.push_str(&record_id(record).to_string());
            output.push('\n');
        }
        if matches!(record, Record::Span { .. }) {
            let stack = stacks.entry(thread).or_default();
            if stack.len() <= depth {
                stack.resize(depth + 1, None);
            }
            stack[depth] = Some(record_id(record));
            stack.truncate(depth + 1);
        }
    }
}

fn parent_id(
    stacks: &HashMap<usize, Vec<Option<u64>>>,
    thread: usize,
    depth: usize,
) -> Option<u64> {
    if depth == 0 {
        return None;
    }
    stacks
        .get(&thread)
        .and_then(|stack| stack.get(depth - 1))
        .copied()
        .flatten()
}

fn record_label(profile: &Profile, record: &Record) -> String {
    let mut lines = Vec::new();
    match record {
        Record::Span {
            label,
            source,
            nanos,
            fields,
            ..
        } => {
            lines.push(format!("span: {label}"));
            lines.push(format!("time={}", format_nanos(*nanos)));
            push_source_line(&mut lines, profile, *source);
            push_field_lines(&mut lines, fields);
        }
        Record::Event {
            label,
            detail,
            source,
            fields,
            ..
        } => {
            lines.push(format!("event: {label}"));
            if !detail.is_empty() {
                lines.push(format!("detail={detail}"));
            }
            push_source_line(&mut lines, profile, *source);
            push_field_lines(&mut lines, fields);
        }
    }
    lines
        .into_iter()
        .map(|line| escape_label(&line))
        .collect::<Vec<_>>()
        .join("<br/>")
}

fn push_source_line(lines: &mut Vec<String>, profile: &Profile, source: usize) {
    if let Some(location) = profile.source_locations.get(source) {
        lines.push(format!("source={}:{}", location.file, location.line));
    }
}

fn push_field_lines(lines: &mut Vec<String>, fields: &[Field]) {
    lines.extend(
        fields
            .iter()
            .map(|field| format!("{}={}", field.name, format_field_value(&field.value))),
    );
}

fn format_field_value(value: &FieldValue) -> String {
    match value {
        FieldValue::Text(value) => value.clone(),
        FieldValue::Count(value) => value.to_string(),
        FieldValue::Signed(value) => value.to_string(),
        FieldValue::Bytes(value) => crate::fmt::human(*value),
        FieldValue::Hex(value) => format!("{value:#x}"),
        FieldValue::Addr(value) => format!("{value:#x}"),
        FieldValue::ByteRange { start, len } | FieldValue::AddrRange { start, len } => {
            format_range(*start, *len)
        }
    }
}

fn format_range(start: u64, len: u64) -> String {
    let end = start.saturating_add(len);
    format!("{start:#x}..{end:#x}/{}", crate::fmt::human(len))
}

fn format_nanos(nanos: u128) -> String {
    if nanos >= 1_000_000 {
        format!("{}ms", nanos / 1_000_000)
    } else if nanos >= 1_000 {
        format!("{}us", nanos / 1_000)
    } else {
        format!("{nanos}ns")
    }
}

fn escape_label(value: &str) -> String {
    let mut escaped = String::new();
    for ch in value.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push('\''),
            '`' => escaped.push('\''),
            '\n' | '\r' => escaped.push(' '),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn record_id(record: &Record) -> u64 {
    match record {
        Record::Span { id, .. } | Record::Event { id, .. } => *id,
    }
}

fn record_thread(record: &Record) -> usize {
    match record {
        Record::Span { thread, .. } | Record::Event { thread, .. } => *thread,
    }
}

fn record_depth(record: &Record) -> usize {
    match record {
        Record::Span { depth, .. } | Record::Event { depth, .. } => *depth,
    }
}
