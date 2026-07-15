json11
------

json11 is a tiny JSON library for Rust, providing JSON parsing and serialization. It is an
idiomatic Rust port of the original [C++11 json11 library](https://github.com/dropbox/json11).

The core type provided by the library is `json11::Json`, an enum that represents any JSON
value: null, bool, number (`f64`), string (`String`), array (`Vec<Json>`), or object
(`BTreeMap<String, Json>`).

`Json` values act like values. They can be cloned, compared for equality or order, and so on.
There are also helper methods `Json::dump`, to serialize a `Json` to a `String`, and
`Json::parse` (associated function) to parse a `&str` into a `Json`.

A note on numbers: json11 stores all numbers as `f64` internally (matching the C++ library's
decision to store everything as `double`), but also provides an integer accessor
(`int_value`).

Usage
-----

Add the crate to your `Cargo.toml`:

```toml
[dependencies]
json11 = { path = "." } # or a version/git dependency
```

Building JSON values
---------------------

Use the `Json::object` / `Json::array` helpers together with the `From` conversions:

```rust
use json11::Json;

let my_json = Json::object([
    ("key1".to_string(), Json::from("value1")),
    ("key2".to_string(), Json::from(false)),
    ("key3".to_string(), Json::array([Json::from(1), Json::from(2), Json::from(3)])),
]);
let json_str = my_json.dump();
// {"key1": "value1", "key2": false, "key3": [1, 2, 3]}
```

There are `From` implementations that convert standard types into `Json`, including `f64`,
`i32`, `bool`, `&str`, `String`, `Vec<T: Into<Json>>`, sets, and map-like types. You can
convert your own types by implementing `From<YourType> for Json`:

```rust
use json11::Json;

struct Point {
    x: i32,
    y: i32,
}

impl From<Point> for Json {
    fn from(p: Point) -> Json {
        Json::array([Json::from(p.x), Json::from(p.y)])
    }
}

let points = vec![Point { x: 1, y: 2 }, Point { x: 10, y: 20 }, Point { x: 100, y: 200 }];
let points_json = Json::from(points).dump();
// [[1, 2], [10, 20], [100, 200]]
```

Inspecting JSON values
----------------------

JSON values can be queried and inspected. Indexing an array by `usize` or an object by `&str`
never panics — out-of-range or wrong-type accesses return a reference to a shared `Json::Null`:

```rust
use json11::Json;

let json = Json::array([Json::object([("k".to_string(), Json::from("v"))])]);
let s = json[0]["k"].string_value(); // "v"
```

Accessors mirror the original C++ API: `r#type()`, `is_null` / `is_number` / `is_bool` /
`is_string` / `is_array` / `is_object`, `number_value`, `int_value`, `bool_value`,
`string_value`, `array_items`, and `object_items`.

Parsing
-------

```rust
use json11::{Json, JsonParse};

let json = Json::parse(r#"{"k1": "v1", "k2": 42}"#, JsonParse::Standard).unwrap();
assert_eq!(json["k1"].string_value(), "v1");
assert_eq!(json["k2"].int_value(), 42);
```

`Json::parse` returns `Result<Json, String>` (the `Err` holds a descriptive error message).
Pass `JsonParse::Comments` to allow C-style `//` and `/* */` comments.

Multiple values concatenated or separated by whitespace can be parsed with `Json::parse_multi`,
which returns the parsed values, the position at which parsing stopped, and a `Result<(), String>`:

```rust
use json11::{Json, JsonParse};

let (values, stop_pos, res) = Json::parse_multi("{} {}", JsonParse::Standard);
assert_eq!(values.len(), 2);
assert!(res.is_ok());
let _ = stop_pos;
```

Building and testing
--------------------

```sh
cargo build
cargo test
```
