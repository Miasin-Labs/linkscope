use std::sync::Mutex;

use linkscope::TraceField;

static TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn mermaid_markdown_exports_nested_trace_graph() {
    let _guard = TEST_LOCK.lock().expect("test mutex poisoned");

    linkscope::trace_enable();
    {
        let _outer = linkscope::trace_fields("outer_compile", [TraceField::count("items", 2)]);
        {
            let _inner = linkscope::trace("expand_macro");
            linkscope::event_fields("derive_graph", [TraceField::text("derive", "Foo::Bar")]);
        }
    }

    let markdown = linkscope::profile().to_mermaid_markdown();

    assert!(markdown.starts_with("```mermaid\nflowchart TD\n"));
    assert!(markdown.contains("R0[\"span: outer_compile"));
    assert!(markdown.contains("items=2"));
    assert!(markdown.contains("R1[\"span: expand_macro"));
    assert!(markdown.contains("R2([\"event: derive_graph"));
    assert!(markdown.contains("derive=Foo::Bar"));
    assert!(markdown.contains("    R0 --> R1\n"));
    assert!(markdown.contains("    R1 --> R2\n"));
    assert!(markdown.ends_with("```\n"));
}

#[test]
fn mermaid_markdown_escapes_labels_and_handles_empty_profiles() {
    let empty = linkscope::Profile::default().to_mermaid_markdown();

    assert!(empty.contains("empty[\"No profile records\"]"));

    let _guard = TEST_LOCK.lock().expect("test mutex poisoned");
    linkscope::trace_enable();
    linkscope::event_fields(
        "macro<derive>",
        [TraceField::text("quoted", "\"Thing\" & [ok]")],
    );

    let markdown = linkscope::to_mermaid_markdown();

    assert!(markdown.contains("event: macro&lt;derive&gt;"));
    assert!(markdown.contains("quoted='Thing' &amp; [ok]"));
}
