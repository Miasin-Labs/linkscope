# Full synthesis: turning `linkscope` into a Rust app profiler

Date: 2026-06-27
Workspace: `/home/cole/RustProjects/active/linkscope`
Research corpus: cloned under `research/*`, each repo initialized with
CodeGraph.

## Executive summary

`linkscope` is a good start, but it is currently a linker-internal profiler, not
yet an application profiler. It has manual RAII timing spans, structured trace
fields, RSS snapshots, and a text report. That is enough to explain Peony's link
pipeline, but it is missing the layers that make tools like hotpath, puffin,
Tracy, samply, dhat, and bytehound useful on arbitrary Rust applications.

The largest missing layer is ergonomic instrumentation. In Rust, the `#[]`
things you mentioned are attributes. When attributes generate or transform code,
they are procedural macros. The researched crates show two important patterns:
`#[hotpath::measure]`/`#[hotpath::future_fn]` for app functions and async code,
and `#[derive(Allocative)]` for type/enum memory traversal. A future `linkscope`
should add a `linkscope-macros` proc-macro crate with `#[linkscope::instrument]`,
`#[linkscope::main]`, bulk impl/module instrumentation, skip/name/field options,
and no-op expansion when disabled.

The second missing layer is a real profile data model. Today `linkscope` renders
straight to stderr. To become a profiler, it needs `Profile`/`Snapshot` records
that can be exported to JSON, Firefox Profiler, pprof, or eventually Tracy/Puffin
adapters. Samply, jemalloc-pprof, puffin, and perf/Hotspot all reinforce the same
lesson: collection, data model, export, and UI must be separate layers.

The third missing layer is runtime integrations. App profiling needs allocation
tracking, async/future/channel/stream/lock instrumentation, tracing integration,
and optional sampling-profiler correlation. `linkscope` should not try to replace
perf, samply, Tracy, or bytehound. Its best niche is Rust-native semantic
instrumentation that explains what external profilers cannot infer, then exports
or correlates with those tools when their viewers are better.

## What `linkscope` is today

Current public surface:

- Phase timers: `phase`, `Span`, `record_bytes`, `record_items` in
  `src/phase.rs`.
- Trace spans/events: `trace`, `trace_fields`, `event`, `event_fields`,
  `detail_event_fields`, `TraceFrame` in `src/trace.rs`.
- Structured fields: `TraceField::text`, `count`, `bytes`, `hex`, `byte_range`,
  `addr_range` in `src/trace_fields.rs`.
- RSS snapshots: `record_rss` in `src/rss.rs`.
- Text report: `report()` in `src/lib.rs`.
- Counter table: static `COUNTER_NAMES` and `count(name, n)` in
  `src/counters.rs`.

Strengths:

- Near-zero dependency footprint.
- Cheap disabled paths via atomic checks.
- Good manual API for compiler/linker phases.
- Domain-friendly `TraceField` constructors for bytes and address ranges.
- Simple enough to keep in hot paths.

Current bottlenecks:

- Manual instrumentation only; no `#[...]` attribute macros.
- No derive macro for type/object memory inspection.
- No feature-gated no-op macros.
- No structured `Snapshot`/`Profile` API.
- No JSON, pprof, Firefox Profiler, Tracy, or Puffin export.
- No live sink/subscriber model.
- No allocator, async, channel, lock, or `tracing` integration.
- String counters can silently disappear when the string is not in the static
  counter table.
- `report()` still says `peony --stats`, so the renderer is not generic yet.

## Research corpus and what each repo teaches

### `hotpath-rs`: best direct model for Rust app profiling

Local path: `research/hotpath-rs`

Relevant evidence:

- Attribute macros are exported from `crates/hotpath-macros/src/lib.rs`:
  `measure` at line 166 and `future_fn` at line 206.
- Example usage shows `#[hotpath::measure]` on a normal function and
  `#[hotpath::main]` on `main` in
  `crates/test-tokio-async/examples/benchmark_alloc.rs:4` and
  `crates/test-tokio-async/examples/benchmark_alloc.rs:12`.
- Allocation counting uses a `GlobalAlloc` wrapper in
  `crates/hotpath/src/lib_on/functions/alloc/allocator.rs:16` and implements
  `alloc`/`dealloc` at lines 31-47.
- CPU sampling is delegated to samply through a helper binary in
  `crates/hotpath/bin/hotpath-samply/main.rs:110`.
- JSON/TUI/report routes appear throughout `crates/hotpath/src/lib_on/report.rs`
  and the ratatui console under `crates/hotpath/bin/hotpath/cmd/console`.
- It includes a `tracing_subscriber::Layer` SQL integration in
  `crates/hotpath/src/lib_on/sql/tracing_layer.rs`.

Lessons for `linkscope`:

1. Create a proc-macro crate. Users want attributes, not just manual guards.
2. Keep disabled instrumentation in source, but compile it to no-op code when
   features are off.
3. Treat app profiling as multiple surfaces: functions, allocations, CPU samples,
   async data flow, locks, SQL/tracing, TUI, JSON, and MCP.
4. Do not reimplement sampling if samply can do it; correlate with it.

What not to copy first:

- Hotpath's scope is very broad. `linkscope` should not jump straight to MCP,
  SQL, TUI, and async wrappers before it has a clean profile model.

### `profiling`: best small facade pattern

Local path: `research/profiling`

Relevant evidence:

- `profiling/src/lib.rs` selects backends by feature: puffin, optick,
  superluminal, tracing, Tracy, type-check, or empty implementation at lines
  102-167.
- Proc macro reexports for `all_functions`, `function`, and `skip` are gated by
  the `procmacros` feature at lines 47-100.
- `profiling-procmacros/src/lib.rs` implements `#[function]` at lines 6-23 and
  `#[all_functions]` at lines 33-88.
- Disabled/no-backend expansion returns the original body at lines 90-106, while
  enabled backends inject `profiling::function_scope!()` at lines 108-125 or a
  `tracing` span at lines 127-140.

Lessons for `linkscope`:

1. A backend-agnostic macro layer is realistic and small.
2. Support multiple output backends through feature selection.
3. No-op expansion is part of the API contract, not an afterthought.
4. `tracing` can be one backend, not necessarily the whole system.

Recommended adaptation:

- `linkscope-macros` should initially target the native `linkscope` collector.
- Later, it can offer `backend-puffin`, `backend-tracy`, `backend-tracing`, or
  `export-firefox` feature modes.

### `allocative`: best model for derive-based memory/object traversal

Local path: `research/allocative`

Relevant evidence:

- `allocative/src/lib.rs` describes the crate as lightweight memory profiling
  through object traversal at lines 11-25.
- It explicitly distinguishes object-tree memory profiling from call-stack malloc
  profiling at lines 27-41.
- It exports `allocative_derive::Allocative` and `allocative_derive::root` at
  lines 64-65.
- `allocative_derive/src/derive_allocative.rs` generates enum variant traversal
  at lines 160-218.
- It generates field visitor calls and handles field attributes at lines 272-353.

Lessons for `linkscope`:

1. App memory profiling has two complementary views: allocation stack profiles
   and object graph profiles.
2. A derive macro can explain enum/struct memory layout and nested ownership in a
   way malloc profilers cannot.
3. Field-level attributes are mandatory: skip fields, rename fields, custom visit
   hooks, and custom bounds.

Recommended adaptation:

- Add a later `#[derive(LinkscopeMemory)]` or `#[derive(ProfileMemory)]` rather
  than mixing memory traversal into the first function-instrumentation macro.
- Build the visitor API around explicit node kinds: inline, unique allocation,
  shared allocation, external resource, and redacted/skipped.

### `puffin`: best low-overhead frame/scope collector

Local path: `research/puffin`

Relevant evidence:

- RAII scope type `ProfilerScope` is in `puffin/src/lib.rs:131` and ends on
  `Drop` at lines 154-158.
- Compact per-thread event streams are represented by `Stream` in
  `puffin/src/data.rs:73`.
- Stream encoding uses explicit begin/end scope events in `begin_scope` and
  `end_scope`, lines 112-159.
- `GlobalProfiler` gathers thread streams and emits frames to sinks in
  `puffin/src/global_profiler.rs:45` through line 188.
- The egui UI consumes `FrameView` and shows flamegraphs/table views in
  `puffin_egui/src/lib.rs:431` and below.

Lessons for `linkscope`:

1. A good profiler does not store everything as a growing vector of cloned nodes.
2. Per-thread buffers or streams reduce lock contention.
3. A frame/sink concept enables live UI without coupling the collector to one UI.
4. UI can be a separate crate that consumes stable frame data.

Recommended adaptation:

- Keep the current simple vector collector for now, but design `Snapshot` so it
  can later come from per-thread streams.
- Add a sink trait before adding a TUI.

### `tracy`: best full timeline profiler reference

Local path: `research/tracy`

Relevant evidence:

- C++ zones use an RAII `ScopedZone` in `public/client/TracyScoped.hpp:19`.
- `ScopedZone` writes begin events at lines 27-48 or dynamic source locations at
  lines 50-73, then writes end events in the destructor at lines 77-86.
- Zone text/color/name payloads are separate methods on the live zone object at
  lines 88 and below.
- `Profiler` allocates source locations and sends events through queue items in
  `public/client/TracyProfiler.hpp`.
- Worker threads and queues are started in `public/client/TracyProfiler.cpp`.

Lessons for `linkscope`:

1. Timelines need event types beyond spans: frames, plots, messages, allocation
   events, thread names, source locations, and callstacks.
2. Source-location interning matters for overhead.
3. Remote viewers are powerful but require a stable protocol and data model.
4. Tracy already exists; `linkscope` should export/adapt rather than compete.

Recommended adaptation:

- Start with a Tracy-inspired event schema, not a Tracy-compatible protocol.
- Add a `source_location` table to the profile model before building exporters.

### `dhat-rs`: best simple allocator profiler pattern

Local path: `research/dhat-rs`

Relevant evidence:

- `Profiler::new_heap()` starts heap profiling in `src/lib.rs:993`.
- `ProfilerBuilder` carries mode and output settings in `src/lib.rs:1031`.
- Heap/ad-hoc stats are exposed as public structs at `src/lib.rs:1609` and
  `src/lib.rs:1634`.
- Tests show `#[global_allocator] static ALLOC: dhat::Alloc = dhat::Alloc;` in
  `tests/ad-hoc.rs:1`.
- Ad-hoc event recording appears in `tests/ad-hoc.rs:5`, `tests/ad-hoc.rs:11`,
  and `tests/ad-hoc.rs:34`.

Lessons for `linkscope`:

1. Heap profiling can be simple if the user opts into a global allocator.
2. Provide public live stats, not only final reports.
3. Keep ad-hoc events separate from heap events but merge them in reports.
4. Testing mode is important for regression assertions.

Recommended adaptation:

- Add a `linkscope-alloc` optional allocator wrapper later.
- Expose `AllocationStats::current()` and per-span allocation deltas.

### `rust-jemalloc-pprof`: best pprof heap export bridge

Local path: `research/rust-jemalloc-pprof`

Relevant evidence:

- Async activation helpers are in `src/lib.rs:37` and `src/lib.rs:55`.
- Global profiling control lives in `PROF_CTL` at `src/lib.rs:72`.
- The C API dumps pprof data from jemalloc in `capi/src/lib.rs:40` through
  `capi/src/lib.rs:83`.
- Example usage configures jemalloc as the global allocator and sets
  `malloc_conf` at `example/src/main.rs:4` through `example/src/main.rs:10`.
- The example serves `/debug/pprof/allocs` in `example/src/main.rs:19` and dumps
  pprof bytes in `handle_get_heap` at lines 33-39.

Lessons for `linkscope`:

1. pprof is a good export target for heap profiles.
2. Jemalloc profiling is a bridge, not a replacement for linkscope's own data.
3. HTTP debug endpoints are a familiar app-profiler surface.

Recommended adaptation:

- Do not make jemalloc mandatory.
- Add an optional `jemalloc-pprof` bridge only after JSON/snapshot is stable.

### `bytehound`: best full allocation event model

Local path: `research/bytehound`

Relevant evidence:

- Allocation tracking is per-thread via `AllocationTracker` in
  `preload/src/allocation_tracker.rs:53`.
- Server protocol has allocation and backtrace response types in
  `server-core/src/protocol.rs:121`, `server-core/src/protocol.rs:297`, and
  `server-core/src/protocol.rs:329`.
- Allocation queries are represented by `RequestAllocations` at
  `server-core/src/protocol.rs:786`.
- Server allocation query handling is in `server-core/src/lib.rs:552` through
  line 646.
- Internal data tracks allocation IDs, backtrace IDs, flags, timestamps, chains,
  and deallocations in `cli-core/src/data.rs`.

Lessons for `linkscope`:

1. Useful allocation analysis requires allocation identity, deallocation, chain,
   timestamp, backtrace, thread, and flags.
2. Queryability matters as much as collection.
3. A server protocol enables UIs and agents to inspect data without parsing logs.

Recommended adaptation:

- For `linkscope`, start with lightweight per-span allocation deltas. Full
  allocation identity tracking is a separate, heavier feature.

### `samply`: best Firefox Profiler export and sampling reference

Local path: `research/samply`

Relevant evidence:

- Platform-specific profilers are separated into Linux, macOS, and Windows
  modules in `samply/src/linux/mod.rs`, `samply/src/mac/mod.rs`, and
  `samply/src/windows/mod.rs`.
- macOS sampling creates a `Profile`, gathers tasks, samples live tasks, and
  flushes sample data in `samply/src/mac/sampler.rs:61` through line 242.
- Native symbol tables include addresses, function sizes, library indexes, and
  names in `fxprof-processed-profile/src/native_symbols.rs:40` through line 142.
- The Firefox profile model includes samples, threads, libraries, stack tables,
  frame tables, function tables, resource tables, and markers across the
  `fxprof-processed-profile` crate.

Lessons for `linkscope`:

1. Firefox Profiler JSON is a strong first visual export target.
2. A profile needs thread/process/module/source/symbol metadata, not just spans.
3. Sampling should remain delegated; `linkscope` can correlate semantic spans
   with sampled stacks.

Recommended adaptation:

- Add a Firefox Profiler JSON exporter before building a custom GUI.
- Add marker export for linkscope spans/events.

### `linux-perf`: best sampling and ring-buffer reference

Local path: `research/linux-perf`

Relevant evidence:

- `perf_event_mmap_page` and the kernel ring-buffer contract are in
  `include/uapi/linux/perf_event.h:593` through line 769.
- perf's user-space mmap wrapper is in `tools/perf/util/mmap.h:25` through line
  60.
- `record` and `report` modes keep separate option/session/tool state in
  `tools/perf/builtin-record.c:155` and `tools/perf/builtin-report.c:78`.
- `tools/perf/scripts/python/flamegraph.py` builds a JSON tree from perf script
  events at lines 52-123 and writes HTML/JSON at lines 147-235.

Lessons for `linkscope`:

1. Sampling is an OS integration problem; do not reimplement perf.
2. Good sampled profiles need callchains, symbolization, filtering, and event
   metadata.
3. Simple flamegraph JSON export can be useful even before a full UI exists.

Recommended adaptation:

- Treat perf as an external companion: run/open/import/correlate, not replace.

### `hotspot`: best perf analysis UI reference

Local path: `research/hotspot`

Relevant evidence:

- Perf parsing happens in `src/parsers/perf/perfparser.cpp:765` and builds
  sample/thread/symbol/tracepoint data.
- `PerfParser` exposes bottom-up, caller-callee, by-file, events, frequency, and
  tracepoint results in `src/parsers/perf/perfparser.h:38` through line 65.
- Flamegraph UI supports top-down/bottom-up views, search, thresholds, color
  schemes, recursion collapse, and navigation in `src/flamegraph.cpp:654` and
  below.
- `src/flamegraph.h:29` exposes a reusable `FlameGraph` widget with export and
  selection signals.

Lessons for `linkscope`:

1. A real UI needs multiple projections: top-down, bottom-up, by-file,
   per-library, timeline, caller/callee.
2. Search, filtering, thresholds, and export matter.
3. Start by exporting to existing viewers, not by building Hotspot from scratch.

### `tracy` and `puffin` together: timeline vs frame model

Tracy is timeline-first: zones, frames, messages, plots, memory, remote viewer,
and system tracing. Puffin is frame/scope-first: gather per-thread streams into
frame data and show flamegraphs/table views. `linkscope` should not choose too
early. Its core model should support both:

- timeline records for long-running apps and background tasks;
- snapshot/frame records for one-shot reports and UI refreshes.

## Gap matrix

| Capability | Current `linkscope` | Best reference | Priority |
|---|---|---|---|
| Manual RAII spans | Yes | puffin/tracy | Already present |
| Attribute macros | No | hotpath/profiling | P0 |
| No-op disabled macros | No | profiling/hotpath | P0 |
| `#[main]` auto enable/report | No | hotpath | P0 |
| Structured snapshot | No | puffin/samply | P0 |
| JSON export | No | hotpath/dhat | P0 |
| Typed/dynamic counters | Partial/stringly | hotpath/linkscope need | P0 |
| Firefox Profiler export | No | samply | P1 |
| pprof export | No | jemalloc-pprof/perf | P1 |
| Global allocator wrapper | No | dhat/hotpath | P1 |
| Object memory derive | No | allocative | P2 |
| Future lifecycle | No | hotpath | P2 |
| Channel/stream wrappers | No | hotpath | P2 |
| Mutex/RwLock wrappers | No | hotpath | P2 |
| `tracing_subscriber::Layer` | No | hotpath/profiling | P1 |
| Live sink/server | No | puffin/bytehound/hotpath | P2 |
| TUI | No | hotpath | P3 |
| GUI | No | puffin/hotspot/tracy | P3/export first |
| Sampling implementation | No | perf/samply | Do not reimplement |
| Sampling correlation | No | hotpath+samply | P2 |
| MCP/API query surface | No | hotpath/bytehound | P3 |

## Recommended product identity

`linkscope` should become:

> A Rust-native semantic instrumentation profiler that records spans, events,
> counters, allocations, async waits, and application-specific fields with
> low disabled overhead, then exports to standard profiler formats and viewers.

It should not be positioned as:

- a replacement for perf or samply CPU sampling;
- a replacement for Tracy's mature remote GUI;
- a replacement for bytehound's full LD_PRELOAD heap tracker;
- a replacement for pprof ecosystem tooling.

Its value is correlation and semantics: “this sampled CPU stack happened while
the app was in this domain span,” “this future spent 95% of wall time pending,”
“this function allocated 10 MB per call,” “this compiler phase processed 4 GB/s,”
or “this channel is backed up because the consumer is slow.”

## Recommended crate architecture

Start with the current single crate, but design toward this workspace:

```text
linkscope/
  Cargo.toml
  crates/
    linkscope/             # core API, guards, profile model, snapshots
    linkscope-macros/      # #[instrument], #[main], bulk impl/module macros
    linkscope-export/      # JSON, Firefox Profiler, pprof exporters
    linkscope-tracing/     # tracing_subscriber Layer + event field mapping
    linkscope-alloc/       # optional GlobalAlloc and jemalloc bridge
    linkscope-async/       # future/channel/stream/lock wrappers
    linkscope-cli/         # report, serve, open, compare
```

Do not split all crates immediately. Split when the dependency or compiler
boundary forces it:

1. `linkscope-macros` must be separate because proc macros are separate crates.
2. `linkscope-export` should be separate once serde/prost/profile-format deps
   appear.
3. `linkscope-alloc` should be separate because global allocator code and
   jemalloc support are optional and opinionated.
4. `linkscope-cli` should be separate because TUI/HTTP/MCP dependencies are not
   core dependencies.

## Proposed core data model

Minimum viable model:

```text
Profile
  metadata: process, command, pid, start_time, clock
  threads: Vec<ThreadInfo>
  sources: SourceLocationTable
  strings: StringTable
  records: Vec<Record>

Record
  SpanBegin { id, parent, thread, time, label, location, fields }
  SpanEnd { id, thread, time, counters }
  Instant { thread, time, label, location, fields }
  Counter { time, key, value, unit, aggregation }
  AllocationDelta { span, bytes_alloc, bytes_dealloc, allocs, deallocs }
  Rss { time, current, peak, label }
  SampleReference { time, thread, external_profile_id, sample_index }
```

Important rules:

- Renderers and exporters consume `Profile`; collectors do not print directly.
- Fields are typed values, not only formatted strings.
- Source locations are interned.
- Counters are registered/typed, or dynamically created with an explicit policy.
- Thread IDs are stable profiler IDs, not only `Debug`-formatted `ThreadId`.

## Macro design

Initial macro set:

```rust
#[linkscope::instrument]
fn parse(input: &Input) -> Output { ... }

#[linkscope::instrument(name = "parse-object", fields(path = %path))]
fn parse_object(path: &Path) -> Result<Object> { ... }

#[linkscope::instrument(skip(large_buffer), err, ret)]
fn emit(large_buffer: &[u8]) -> Result<()> { ... }

#[linkscope::main]
fn main() -> anyhow::Result<()> { ... }

#[linkscope::instrument_all]
impl Resolver { ... }
```

Async support:

```rust
#[linkscope::instrument]
async fn fetch() -> Result<Data> { ... }
```

The async expansion must measure both total wall time and poll lifecycle:
created, poll count, pending durations, ready time, drop-before-ready.

Disabled behavior:

- With no feature enabled, attribute macros return the original function body.
- Manual macros become no-op guards with no allocations.
- Runtime dependencies are absent from the default build.

## Tracing integration

Add a `tracing_subscriber::Layer` after the core profile model exists.

Mapping:

- tracing span enter -> `SpanBegin`
- tracing span exit/close -> `SpanEnd`
- tracing event -> `Instant`
- event fields -> typed `FieldValue`
- target/module/level -> category metadata

This lets existing apps keep `#[tracing::instrument]` and still export linkscope
profiles. It also lets `linkscope` become a profiler facade instead of forcing a
new macro everywhere.

## Export strategy

Order matters:

1. Native JSON: easiest to test, useful for CLI/TUI/MCP.
2. Firefox Profiler JSON: strongest immediate visual win, based on samply's
   profile ecosystem.
3. pprof: useful for heap/counter samples and standard tooling.
4. Tracy/Puffin adapters: valuable later, but avoid premature protocol coupling.

Native JSON should be considered the stable contract for tests and tools.

## Implementation roadmap

### Phase 0: Clean up extracted crate

- Rename report heading from `peony --stats` to `linkscope` or configurable
  application name.
- Add `Snapshot` and make `report()` render from it.
- Replace static string counter table with dynamic registration or typed keys.
- Add tests for unknown counter behavior.

Acceptance:

- Existing Peony `--stats` still works.
- `linkscope::snapshot()` returns the same data that `report()` prints.
- Unknown counters are either recorded or rejected visibly, never silently lost.

### Phase 1: Native JSON export

- Add `serde` behind a `json` feature.
- Define serializable records for phases, counters, events, RSS, and trace nodes.
- Add `write_json(writer)` and `to_json_string()`.

Acceptance:

- Unit test records a phase/event/RSS/counter and snapshots JSON.
- Peony can write a JSON profile file without parsing stderr.

### Phase 2: Proc macros

- Add `linkscope-macros`.
- Implement `#[linkscope::instrument]` for sync functions.
- Implement `#[linkscope::main]`.
- Implement no-op disabled expansion.
- Implement `name =`, `skip(...)`, and simple fields.

Acceptance:

- `trybuild` tests prove generated code compiles enabled and disabled.
- A tiny driver app can be profiled with only `#[linkscope::main]` and
  `#[linkscope::instrument]`.

### Phase 3: Tracing layer

- Add `linkscope-tracing` or a gated module.
- Map tracing spans/events into `Profile` records.
- Preserve target, level, file, line, and typed fields.

Acceptance:

- A tiny app using `tracing::instrument` emits a valid linkscope profile without
  linkscope-specific attributes.

### Phase 4: Allocation deltas

- Add optional global allocator wrapper.
- Track per-thread allocation counters.
- Record per-span allocation deltas.
- Expose total/current/peak stats.

Acceptance:

- Test function with a known allocation reports positive allocation bytes under
  its span.
- Disabled feature has no allocator dependency.

### Phase 5: Async/data-flow wrappers

- Instrument futures: poll count, pending time, ready/drop state.
- Add channel wrappers for a small first target, probably `std::sync::mpsc` or
  Tokio mpsc.
- Add lock wrappers after the histogram type exists.

Acceptance:

- Async example distinguishes CPU work from pending time.
- Channel example reports sent/received counts and queue/backpressure signals.

### Phase 6: Firefox Profiler export and sampler correlation

- Export spans as markers.
- Export threads, timestamps, process metadata, and source locations.
- Optionally invoke `samply record` externally and correlate by time range.

Acceptance:

- Generated profile opens in Firefox Profiler/Samply UI.
- Linkscope spans are visible as named markers.

### Phase 7: Live workflow

- Add a local JSON endpoint or Unix socket.
- Add a minimal TUI that reads the native JSON routes.
- Add MCP only after endpoint/query types stabilize.

Acceptance:

- Running app can be inspected live without waiting for process exit.

## What to build first

The best immediate next implementation is not async wrappers or a TUI. It is:

1. `Snapshot`/`Profile` model.
2. JSON export.
3. `linkscope-macros` with `#[instrument]` and `#[main]`.

Reason: every later feature needs a structured profile model, and attributes are
the ergonomics jump from “Peony-only internal library” to “usable app profiler.”

## Risks and tradeoffs

- Macro expansion can hide cost. Keep manual APIs available and keep disabled
  expansion trivial.
- Allocation tracking via global allocator is invasive. Make it opt-in and
  feature-gated.
- Async wrappers can distort behavior if they proxy channels/streams. Document
  semantics like hotpath does.
- pprof/Firefox/Tracy compatibility can dominate the design if added too early.
  Keep a native model first.
- Live UIs are dependency-heavy. Keep core no-dep or low-dep.
- Linkscope's linker/compiler vocabulary is valuable. Do not erase it by making
  everything generic too early; use categories and fields instead.

## Final recommendation

Build `linkscope` as a Rust semantic instrumentation layer with exports, not as
a universal profiler. The researched tools are better at their own domains:

- perf/samply for CPU sampling;
- Tracy for mature remote timeline UI;
- bytehound/dhat/jemalloc-pprof for heap profiling;
- puffin for embedded frame UI;
- allocative for object-tree memory analysis;
- hotpath for broad Rust app-profiler ergonomics.

`linkscope` can be novel if it combines the parts Rust app developers actually
want in a small, typed, compiler-style crate: attributes, structured fields,
domain spans, allocation deltas, async wait reasons, tracing ingestion, and
exports to existing viewers. Its thesis should be: external profilers show where
CPU or memory went; `linkscope` explains what the application thought it was
doing when that happened.
