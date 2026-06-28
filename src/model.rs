#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "json", derive(serde::Serialize))]
pub struct Snapshot {
    pub total_nanos: u128,
    pub phases: Vec<PhaseSnapshot>,
    pub counters: Vec<CounterSnapshot>,
    pub rss: Vec<RssSnapshot>,
    pub traces: Vec<TraceSnapshot>,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "json", derive(serde::Serialize))]
pub struct PhaseSnapshot {
    pub name: String,
    pub nanos: u128,
    pub spans: u64,
    pub bytes: u64,
    pub items: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "json", derive(serde::Serialize))]
pub struct CounterSnapshot {
    pub name: String,
    pub value: u64,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "json", derive(serde::Serialize))]
pub struct RssSnapshot {
    pub label: String,
    pub current_kb: u64,
    pub peak_kb: u64,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "json", derive(serde::Serialize))]
pub struct TraceSnapshot {
    pub label: String,
    pub file: String,
    pub line: u32,
    pub thread: String,
    pub depth: usize,
    pub nanos: u128,
    pub enter_seq: u64,
    pub is_event: bool,
    pub detail: String,
    pub fields: Vec<FieldSnapshot>,
    pub stack: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "json", derive(serde::Serialize))]
pub struct FieldSnapshot {
    pub name: String,
    pub value: String,
}
