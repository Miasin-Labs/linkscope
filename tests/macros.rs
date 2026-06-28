use std::sync::Mutex;

static TEST_LOCK: Mutex<()> = Mutex::new(());

#[linkscope::instrument]
fn instrumented_work() {}

#[linkscope::main]
fn profiled_entry() {
    linkscope::count("main_macro_counter", 5);
    instrumented_work();
}

#[test]
fn instrument_attribute_records_function_phase_and_trace_when_enabled() {
    let _guard = TEST_LOCK.lock().expect("test mutex poisoned");

    linkscope::trace_enable();
    instrumented_work();

    let snapshot = linkscope::snapshot();
    let profile = linkscope::profile();

    assert!(
        snapshot
            .phases
            .iter()
            .any(|phase| phase.name == "instrumented_work" && phase.spans == 1)
    );
    assert!(profile.records.iter().any(|record| matches!(
        record,
        linkscope::Record::Span { label, .. } if label == "instrumented_work"
    )));
}

#[test]
fn main_attribute_enables_profiler_for_wrapped_function() {
    let _guard = TEST_LOCK.lock().expect("test mutex poisoned");

    profiled_entry();

    let snapshot = linkscope::snapshot();
    let markdown = linkscope::to_mermaid_markdown();

    assert!(
        snapshot
            .counters
            .iter()
            .any(|counter| counter.name == "main_macro_counter" && counter.value == 5)
    );
    assert!(markdown.contains("span: instrumented_work"));
}
