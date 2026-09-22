---
trigger: model_decision
description: When working on the Rust port of json11 (the rust/ crate on the rust-port branches / PRs #1 and #2)
---

json11 is a C++11 JSON library (`json11.hpp`/`.cpp`, tests in `test.cpp`). An idiomatic Rust port lives under `rust/` (Cargo crate: `rust/Cargo.toml`, `rust/src/lib.rs`, `rust/src/tests.rs`) on the `devin/*-rust-port` branches.

Build/test/lint the crate with `(cd rust && cargo test)`, `(cd rust && cargo fmt --check)`, `(cd rust && cargo clippy --all-targets -- -D warnings)`. cargo/rustc are already installed.

## Design invariants
- `Json` is an enum: `Null, Bool(bool), Number(f64), Str(String), Array(Vec<Json>), Object(BTreeMap<String,Json>)`. Numbers are unified as f64 (no int/double split); `dump()` reimplements C `%.17g` so integral values print without a decimal.
- Objects use `BTreeMap` (not HashMap) to match C++ `std::map` sorted-key serialization.
- `type()` is exposed as `r#type()` since `type` is reserved.
- User types convert via `impl From<MyType> for Json`, which grants `to_json()` through a blanket `impl<T: Into<Json>+Clone> ToJson for T`. A blanket `From<T: ToJson> for Json` is impossible (conflicts with std's reflexive `From<T> for T`).
- Accepted deviation: derived `PartialOrd` sorts `Bool` before `Number` (enum order), whereas C++ sorts Number before Bool. This is intentional — keep the enum order Null, Bool, Number, ...
- Lone UTF-16 surrogate `\u` escapes decode to WTF-8 bytes, so parsed `Json::Str` and `dump()` output may contain non-UTF-8 bytes; this is the one spot using `String::from_utf8_unchecked`.
