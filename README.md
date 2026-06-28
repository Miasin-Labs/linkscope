# linkscope

`linkscope` is a tiny in-process profiler for linker/compiler pipelines.

It is not a sampling profiler. It records domain events that external profilers
cannot infer cleanly: phase wall time, bytes/items processed, RSS checkpoints,
structured trace fields, and call-flow trees for linker phases.

Peony currently uses it as an internal dependency with near-zero disabled cost.
