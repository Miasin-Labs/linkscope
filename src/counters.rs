use std::sync::Mutex;

use crate::model::CounterSnapshot;

static COUNTERS: Mutex<Vec<CounterSnapshot>> = Mutex::new(Vec::new());

#[inline]
pub fn count(name: &str, n: u64) {
    if !crate::is_enabled() {
        return;
    }
    with_counters_mut(
        |counters| match counters.iter_mut().find(|counter| counter.name == name) {
            Some(counter) => counter.value = counter.value.saturating_add(n),
            None => counters.push(CounterSnapshot {
                name: name.to_owned(),
                value: n,
            }),
        },
    );
}

pub(crate) fn reset() {
    with_counters_mut(Vec::clear);
}

pub(crate) fn snapshot() -> Vec<CounterSnapshot> {
    match COUNTERS.lock() {
        Ok(counters) => counters.clone(),
        Err(poisoned) => poisoned.into_inner().clone(),
    }
}

pub(crate) fn report(counters: &[CounterSnapshot]) {
    let mut printed_header = false;
    for counter in counters {
        if counter.value == 0 {
            continue;
        }
        if !printed_header {
            eprintln!("── counters ──");
            printed_header = true;
        }
        eprintln!("  {:<22} {}", counter.name, counter.value);
    }
}

fn with_counters_mut(f: impl FnOnce(&mut Vec<CounterSnapshot>)) {
    match COUNTERS.lock() {
        Ok(mut counters) => f(&mut counters),
        Err(poisoned) => {
            let mut counters = poisoned.into_inner();
            f(&mut counters);
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn count_is_disabled_when_profiler_is_off() {
        let _guard = crate::TEST_LOCK.lock().expect("test mutex poisoned");
        crate::phase::disable_for_tests();
        super::reset();
        super::count("file_opens", 7);
        assert!(super::snapshot().is_empty());
    }

    #[test]
    fn count_accumulates_known_counter() {
        let _guard = crate::TEST_LOCK.lock().expect("test mutex poisoned");
        crate::enable();
        super::count("file_opens", 7);
        super::count("file_opens", 5);
        assert_eq!(super::snapshot()[0].value, 12);
        crate::phase::disable_for_tests();
    }

    #[test]
    fn count_records_new_counter_names() {
        let _guard = crate::TEST_LOCK.lock().expect("test mutex poisoned");
        crate::enable();
        super::count("previously_unknown", 9);
        assert!(
            super::snapshot()
                .iter()
                .any(|counter| counter.name == "previously_unknown" && counter.value == 9)
        );
        crate::phase::disable_for_tests();
    }
}
