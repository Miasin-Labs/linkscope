use std::sync::Mutex;

static TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn snapshot_includes_dynamic_counters_and_trace_events() {
    let _guard = TEST_LOCK.lock().expect("test mutex poisoned");

    linkscope::trace_enable();
    linkscope::count("custom_counter", 7);
    linkscope::record_items("phase_with_items", 3);
    {
        let _span = linkscope::phase("explicit_phase");
    }
    {
        let _trace =
            linkscope::trace_fields("trace_scope", [linkscope::TraceField::count("items", 3)]);
        linkscope::event("trace_event", "detail");
    }

    let snapshot = linkscope::snapshot();

    assert!(
        snapshot
            .counters
            .iter()
            .any(|counter| counter.name == "custom_counter" && counter.value == 7)
    );
    assert!(
        snapshot
            .phases
            .iter()
            .any(|phase| phase.name == "explicit_phase" && phase.spans == 1)
    );
    assert!(
        snapshot
            .phases
            .iter()
            .any(|phase| phase.name == "phase_with_items" && phase.items == 3)
    );
    assert!(
        snapshot
            .traces
            .iter()
            .any(|node| node.label == "trace_scope")
    );
    assert!(
        snapshot
            .traces
            .iter()
            .any(|node| node.label == "trace_event" && node.detail == "detail")
    );
}

#[cfg(feature = "json")]
#[test]
fn json_export_contains_profile_sections() {
    let _guard = TEST_LOCK.lock().expect("test mutex poisoned");

    linkscope::enable();
    linkscope::count("json_counter", 11);

    let json = linkscope::to_json_string().expect("snapshot serializes");

    assert!(json.contains("\"counters\""));
    assert!(json.contains("json_counter"));
    assert!(json.contains("\"phases\""));
}
