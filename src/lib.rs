//! `linkscope` — an in-linker phase profiler.

mod counters;
mod fmt;
mod mermaid;
mod model;
mod phase;
mod profile;
mod report;
mod rss;
mod trace;
mod trace_detail;
mod trace_fields;
mod trace_render;
#[cfg(test)]
mod trace_tests;

pub use counters::count;
#[cfg(feature = "macros")]
pub use linkscope_macros::{instrument, main};
pub use model::{
    CounterSnapshot,
    FieldSnapshot,
    PhaseSnapshot,
    RssSnapshot,
    Snapshot,
    TraceSnapshot,
};
pub use phase::{Span, enable, is_enabled, phase, record_bytes, record_items};
pub use profile::{Field, FieldValue, Profile, Record, SourceLocation, ThreadInfo};
pub use rss::record_rss;
pub use trace::{
    TraceFrame,
    detail_event_fields,
    event,
    event_fields,
    trace,
    trace_detail_enable,
    trace_detail_enabled,
    trace_enable,
    trace_fields,
    trace_stack_detail_enable,
    trace_stack_enable,
};
pub use trace_fields::TraceField;
pub use trace_render::trace_tree;

#[must_use]
pub const fn snapshot_disabled() -> Snapshot {
    Snapshot {
        total_nanos: 0,
        phases: Vec::new(),
        counters: Vec::new(),
        rss: Vec::new(),
        traces: Vec::new(),
    }
}

pub fn snapshot() -> Snapshot {
    if !is_enabled() {
        return Snapshot::default();
    }
    let (total_nanos, phases) = phase::snapshot();
    Snapshot {
        total_nanos,
        phases: phases
            .into_iter()
            .map(|phase| PhaseSnapshot {
                name: phase.name,
                nanos: phase.nanos,
                spans: phase.spans,
                bytes: phase.bytes,
                items: phase.items,
            })
            .collect(),
        counters: counters::snapshot(),
        rss: rss::snapshot(),
        traces: trace::snapshot_nodes()
            .into_iter()
            .map(|node| TraceSnapshot {
                label: node.label.to_owned(),
                file: node.loc.file().to_owned(),
                line: node.loc.line(),
                thread: format!("{:?}", node.thread),
                depth: node.depth,
                nanos: node.nanos,
                enter_seq: node.enter_seq,
                is_event: node.is_event,
                detail: node.detail,
                fields: node.fields.iter().map(TraceField::snapshot).collect(),
                stack: node.stack,
            })
            .collect(),
    }
}

pub fn profile() -> Profile {
    if !is_enabled() {
        return Profile::default();
    }
    profile::from_trace_nodes(trace::snapshot_nodes())
}

#[must_use]
pub fn to_mermaid_markdown() -> String {
    mermaid::current_markdown()
}

#[cfg(feature = "json")]
pub fn to_json_string() -> Result<String, serde_json::Error> {
    serde_json::to_string(&snapshot())
}

pub fn report() {
    if !is_enabled() {
        return;
    }
    report::render(&snapshot());
}

pub struct ReportGuard;

impl ReportGuard {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Drop for ReportGuard {
    fn drop(&mut self) {
        report();
    }
}

pub(crate) fn reset_all() {
    counters::reset();
    rss::reset();
    trace::reset();
}

#[cfg(test)]
pub(crate) static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
