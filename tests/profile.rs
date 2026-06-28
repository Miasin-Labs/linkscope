use std::sync::Mutex;

use linkscope::{FieldValue, Record, TraceField};

static TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn profile_records_trace_span_and_event_with_typed_fields() {
    let _guard = TEST_LOCK.lock().expect("test mutex poisoned");

    linkscope::trace_enable();
    {
        let _span = linkscope::trace_fields(
            "profile_span",
            [
                TraceField::count("items", 3),
                TraceField::bytes("bytes", 4096),
                TraceField::signed("delta", -5),
                TraceField::hex("addr", 0x4000),
                TraceField::addr("pc", 0x5000),
                TraceField::byte_range("range", 0x10, 0x20),
            ],
        );
        linkscope::event_fields("profile_event", [TraceField::text("symbol", "_start")]);
    }

    let profile = linkscope::profile();

    assert!(!profile.records.is_empty());
    assert!(!profile.source_locations.is_empty());
    assert!(!profile.threads.is_empty());

    let span = profile
        .records
        .iter()
        .find_map(|record| match record {
            Record::Span { label, fields, .. } if label == "profile_span" => Some(fields),
            Record::Event { .. } => None,
            Record::Span { .. } => None,
        })
        .expect("profile span record exists");
    let event = profile
        .records
        .iter()
        .find_map(|record| match record {
            Record::Event { label, fields, .. } if label == "profile_event" => Some(fields),
            Record::Span { .. } => None,
            Record::Event { .. } => None,
        })
        .expect("profile event record exists");

    assert_eq!(field_value(span, "items"), &FieldValue::Count(3));
    assert_eq!(field_value(span, "bytes"), &FieldValue::Bytes(4096));
    assert_eq!(field_value(span, "delta"), &FieldValue::Signed(-5));
    assert_eq!(field_value(span, "addr"), &FieldValue::Hex(0x4000));
    assert_eq!(field_value(span, "pc"), &FieldValue::Addr(0x5000));
    assert_eq!(
        field_value(span, "range"),
        &FieldValue::ByteRange {
            start: 0x10,
            len: 0x20,
        }
    );
    assert_eq!(
        field_value(event, "symbol"),
        &FieldValue::Text("_start".into())
    );
}

#[test]
fn profile_interns_repeated_sources_and_threads() {
    let _guard = TEST_LOCK.lock().expect("test mutex poisoned");

    linkscope::trace_enable();
    emit_repeated_event();
    emit_repeated_event();
    let worker = std::thread::spawn(|| linkscope::event("worker", "done"));
    worker.join().expect("worker finished");

    let profile = linkscope::profile();
    let repeats = profile
        .records
        .iter()
        .filter_map(|record| match record {
            Record::Event {
                label,
                source,
                thread,
                ..
            } if label == "repeat" => Some((*source, *thread)),
            Record::Span { .. } => None,
            Record::Event { .. } => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(repeats.len(), 2);
    assert_eq!(repeats[0].0, repeats[1].0);
    assert_eq!(repeats[0].1, repeats[1].1);
    assert!(
        profile.source_locations[repeats[0].0]
            .file
            .ends_with("profile.rs")
    );
    assert!(profile.source_locations[repeats[0].0].line > 0);
    assert!(profile.threads.len() >= 2);
}

#[cfg(feature = "json")]
#[test]
fn profile_json_serializes_records_tables_and_typed_fields() {
    let _guard = TEST_LOCK.lock().expect("test mutex poisoned");

    linkscope::trace_enable();
    linkscope::event_fields("json_event", [TraceField::bytes("size", 4096)]);

    let value = serde_json::to_value(linkscope::profile()).expect("profile serializes");

    assert!(value.get("source_locations").is_some());
    assert!(value.get("threads").is_some());
    assert!(value.get("records").is_some());
    assert_eq!(value["records"][0]["kind"], "event");
    assert_eq!(value["records"][0]["fields"][0]["value"]["kind"], "bytes");
    assert_eq!(value["records"][0]["fields"][0]["value"]["value"], 4096);
}

fn emit_repeated_event() {
    linkscope::event("repeat", "same call site");
}

fn field_value<'a>(fields: &'a [linkscope::Field], name: &str) -> &'a FieldValue {
    fields
        .iter()
        .find(|field| field.name == name)
        .map(|field| &field.value)
        .expect("field exists")
}
