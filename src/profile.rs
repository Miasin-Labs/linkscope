use std::collections::HashMap;
use std::panic::Location;
use std::thread::ThreadId;

use crate::trace::TraceNode;

#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "json", derive(serde::Serialize))]
pub struct Profile {
    pub source_locations: Vec<SourceLocation>,
    pub threads: Vec<ThreadInfo>,
    pub records: Vec<Record>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "json", derive(serde::Serialize))]
pub struct SourceLocation {
    pub file: String,
    pub line: u32,
    pub column: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "json", derive(serde::Serialize))]
pub struct ThreadInfo {
    pub debug_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "json", derive(serde::Serialize))]
#[cfg_attr(feature = "json", serde(tag = "kind", rename_all = "snake_case"))]
pub enum Record {
    Span {
        id: u64,
        label: String,
        source: usize,
        thread: usize,
        depth: usize,
        nanos: u128,
        fields: Vec<Field>,
        stack: String,
    },
    Event {
        id: u64,
        label: String,
        detail: String,
        source: usize,
        thread: usize,
        depth: usize,
        fields: Vec<Field>,
        stack: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "json", derive(serde::Serialize))]
pub struct Field {
    pub name: String,
    pub value: FieldValue,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FieldValue {
    Text(String),
    Count(u64),
    Signed(i64),
    Bytes(u64),
    Hex(u64),
    Addr(u64),
    ByteRange { start: u64, len: u64 },
    AddrRange { start: u64, len: u64 },
}

pub(crate) fn from_trace_nodes(nodes: Vec<TraceNode>) -> Profile {
    let mut builder = ProfileBuilder::default();
    for node in nodes {
        builder.push(node);
    }
    builder.profile
}

#[derive(Default)]
struct ProfileBuilder {
    profile: Profile,
    sources: HashMap<SourceKey, usize>,
    threads: HashMap<ThreadId, usize>,
}

impl ProfileBuilder {
    fn push(&mut self, node: TraceNode) {
        let source = self.intern_source(node.loc);
        let thread = self.intern_thread(node.thread);
        let fields = node.fields.iter().map(crate::TraceField::profile).collect();
        let record = if node.is_event {
            Record::Event {
                id: node.enter_seq,
                label: node.label.to_owned(),
                detail: node.detail,
                source,
                thread,
                depth: node.depth,
                fields,
                stack: node.stack,
            }
        } else {
            Record::Span {
                id: node.enter_seq,
                label: node.label.to_owned(),
                source,
                thread,
                depth: node.depth,
                nanos: node.nanos,
                fields,
                stack: node.stack,
            }
        };
        self.profile.records.push(record);
    }

    fn intern_source(&mut self, loc: &'static Location<'static>) -> usize {
        let key = SourceKey {
            file: loc.file(),
            line: loc.line(),
            column: loc.column(),
        };
        if let Some(index) = self.sources.get(&key) {
            return *index;
        }
        let index = self.profile.source_locations.len();
        self.profile.source_locations.push(SourceLocation {
            file: key.file.to_owned(),
            line: key.line,
            column: key.column,
        });
        self.sources.insert(key, index);
        index
    }

    fn intern_thread(&mut self, thread: ThreadId) -> usize {
        if let Some(index) = self.threads.get(&thread) {
            return *index;
        }
        let index = self.profile.threads.len();
        self.profile.threads.push(ThreadInfo {
            debug_id: format!("{thread:?}"),
        });
        self.threads.insert(thread, index);
        index
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct SourceKey {
    file: &'static str,
    line: u32,
    column: u32,
}

#[cfg(feature = "json")]
impl serde::Serialize for FieldValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            FieldValue::Text(value) => serialize_value(serializer, "text", value),
            FieldValue::Count(value) => serialize_value(serializer, "count", value),
            FieldValue::Signed(value) => serialize_value(serializer, "signed", value),
            FieldValue::Bytes(value) => serialize_value(serializer, "bytes", value),
            FieldValue::Hex(value) => serialize_value(serializer, "hex", value),
            FieldValue::Addr(value) => serialize_value(serializer, "addr", value),
            FieldValue::ByteRange { start, len } => {
                serialize_range(serializer, RangeValue::new("byte_range", *start, *len))
            }
            FieldValue::AddrRange { start, len } => {
                serialize_range(serializer, RangeValue::new("addr_range", *start, *len))
            }
        }
    }
}

#[cfg(feature = "json")]
struct RangeValue {
    kind: &'static str,
    start: u64,
    len: u64,
}

#[cfg(feature = "json")]
impl RangeValue {
    const fn new(kind: &'static str, start: u64, len: u64) -> Self {
        Self { kind, start, len }
    }
}

#[cfg(feature = "json")]
fn serialize_value<S, T>(serializer: S, kind: &'static str, value: &T) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
    T: serde::Serialize,
{
    use serde::ser::SerializeStruct;

    let mut state = serializer.serialize_struct("FieldValue", 2)?;
    state.serialize_field("kind", kind)?;
    state.serialize_field("value", value)?;
    state.end()
}

#[cfg(feature = "json")]
fn serialize_range<S>(serializer: S, value: RangeValue) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::ser::SerializeStruct;

    let mut state = serializer.serialize_struct("FieldValue", 3)?;
    state.serialize_field("kind", value.kind)?;
    state.serialize_field("start", &value.start)?;
    state.serialize_field("len", &value.len)?;
    state.end()
}
