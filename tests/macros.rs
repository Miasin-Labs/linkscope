use std::sync::Mutex;

static TEST_LOCK: Mutex<()> = Mutex::new(());

#[linkscope::instrument]
fn instrumented_work() {}

#[linkscope::main]
fn profiled_entry() {
    linkscope::count("main_macro_counter", 5);
}

#[test]
fn instrument_attribute_records_function_phase_when_enabled() {
    let _guard = TEST_LOCK.lock().expect("test mutex poisoned");

    linkscope::enable();
    instrumented_work();

    let snapshot = linkscope::snapshot();

    assert!(
        snapshot
            .phases
            .iter()
            .any(|phase| phase.name == "instrumented_work" && phase.spans == 1)
    );
}

#[test]
fn main_attribute_enables_profiler_for_wrapped_function() {
    let _guard = TEST_LOCK.lock().expect("test mutex poisoned");

    profiled_entry();

    let snapshot = linkscope::snapshot();

    assert!(
        snapshot
            .counters
            .iter()
            .any(|counter| counter.name == "main_macro_counter" && counter.value == 5)
    );
}
