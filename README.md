json11
------

json11 is a tiny JSON library for C++11, providing JSON parsing and serialization.

The core object provided by the library is json11::Json. A Json object represents any JSON
value: null, bool, number (int or double), string (std::string), array (std::vector), or
object (std::map).

Json objects act like values. They can be assigned, copied, moved, compared for equality or
order, and so on. There are also helper methods Json::dump, to serialize a Json to a string, and
Json::parse (static) to parse a std::string as a Json object.

It's easy to make a JSON object with C++11's new initializer syntax:

    Json my_json = Json::object {
        { "key1", "value1" },
        { "key2", false },
        { "key3", Json::array { 1, 2, 3 } },
    };
    std::string json_str = my_json.dump();

There are also implicit constructors that allow standard and user-defined types to be
automatically converted to JSON. For example:

    class Point {
    public:
        int x;
        int y;
        Point (int x, int y) : x(x), y(y) {}
        Json to_json() const { return Json::array { x, y }; }
    };

    std::vector<Point> points = { { 1, 2 }, { 10, 20 }, { 100, 200 } };
    std::string points_json = Json(points).dump();

JSON values can have their values queried and inspected:

    Json json = Json::array { Json::object { { "k", "v" } } };
    std::string str = json[0]["k"].string_value();

For more documentation see json11.hpp.

Rust port
---------

An idiomatic Rust port of this library lives in the [`rust/`](rust/) directory as a
Cargo crate. It mirrors the C++ semantics: all numbers are stored as `f64`, objects use
a `BTreeMap` (the analogue of `std::map`) so keys serialize in sorted order, and the
parser reproduces the same whitespace/comment handling, `\uXXXX` surrogate decoding, and
`max_depth = 200` nesting limit.

The core type is the `Json` enum:

```rust
pub enum Json {
    Null,
    Bool(bool),
    Number(f64),
    Str(String),
    Array(Vec<Json>),
    Object(BTreeMap<String, Json>),
}
```

Build values with the `Json::array` / `Json::object` constructors or the `From` impls
(`f64`, `i32`, `bool`, `&str`, `String`, `Vec<T: Into<Json>>`, `BTreeMap`/`HashMap`):

```rust
use json11::Json;

let my_json = Json::object([
    ("key1", Json::from("value1")),
    ("key2", Json::from(false)),
    ("key3", Json::array([1, 2, 3])),
]);
let json_str = my_json.dump();
```

User-defined types convert by implementing `From<MyType> for Json`, which automatically
grants a `to_json()` method through the `ToJson` trait (the Rust equivalent of the C++
`to_json()` convention):

```rust
use json11::{Json, ToJson};

#[derive(Clone)]
struct Point { x: i32, y: i32 }

impl From<Point> for Json {
    fn from(p: Point) -> Json {
        Json::array([p.x, p.y])
    }
}

let points = vec![Point { x: 1, y: 2 }, Point { x: 10, y: 20 }];
let points_json = Json::from(points).dump();      // "[[1, 2], [10, 20]]"
let single = Point { x: 1, y: 2 }.to_json();      // via the ToJson blanket impl
```

Query values with the accessors (`number_value`, `int_value`, `bool_value`,
`string_value`, `array_items`, `object_items`, the `is_*` predicates, and `r#type()`) and
index into arrays (by `usize`) and objects (by `&str`). Missing keys or out-of-range
indices return a reference to `Json::Null` rather than panicking:

```rust
use json11::{Json, JsonParse};

let json = Json::parse(r#"[{"k":"v"}]"#, JsonParse::Standard).unwrap();
let s = json[0]["k"].string_value();              // "v"
let missing = json[0]["nope"].string_value();     // ""
```

Parse with `Json::parse(input, strategy)` (returns `Result<Json, String>`) or
`Json::parse_multi(input, strategy)` (returns the parsed values plus the byte offset where
parsing stopped). `JsonParse::Standard` is strict JSON; `JsonParse::Comments` additionally
allows `//` and `/* ... */` comments. Lightweight schema validation is available via
`has_shape`, which returns `Result<(), String>`.

Note: as in the C++ library, lone UTF-16 surrogate `\u` escapes are decoded to WTF-8 byte
sequences, so a parsed `Json::Str` (and the output of `dump()`) may contain bytes that are
not valid UTF-8.

Build and test the crate with:

```sh
cd rust
cargo test
```
