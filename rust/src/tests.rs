//! Rust port of the assertions in the C++ `test.cpp` (lines 62-253).

use super::*;
use std::collections::{BTreeMap, BTreeSet, HashMap};

fn parse_std(input: &str) -> Result<Json, String> {
    Json::parse(input, JsonParse::Standard)
}

fn parse_comments(input: &str) -> Result<Json, String> {
    Json::parse(input, JsonParse::Comments)
}

#[test]
fn simple_parse_and_accessors() {
    let simple_test = r#"{"k1":"v1", "k2":42, "k3":["a",123,true,false,null]}"#;
    let json = parse_std(simple_test).expect("valid json");

    assert_eq!(json["k1"].string_value(), "v1");
    assert_eq!(json["k3"].dump(), r#"["a", 123, true, false, null]"#);

    let items = json["k3"].array_items();
    assert_eq!(items.len(), 5);
    assert_eq!(items[0].dump(), r#""a""#);
    assert_eq!(items[4], Json::Null);

    // Missing keys / out-of-range indices fall back to Null.
    assert!(json["missing"].is_null());
    assert!(json["k3"][99].is_null());
}

#[test]
fn comment_parsing_success() {
    let comment_test = r#"{
      // comment /* with nested comment */
      "a": 1,
      // comment
      // continued
      "b": "text",
      /* multi
         line
         comment
        // line-comment-inside-multiline-comment
      */
      // and single-line comment
      // and single-line comment /* multiline inside single line */
      "c": [1, 2, 3]
      // and single-line comment at end of object
    }"#;
    let json = parse_comments(comment_test).expect("comments should parse");
    assert!(!json.is_null());

    let json = parse_comments("{\"a\": 1}//trailing line comment").expect("ok");
    assert!(!json.is_null());

    let json = parse_comments("{\"a\": 1}/*trailing multi-line comment*/").expect("ok");
    assert!(!json.is_null());
}

#[test]
fn comment_parsing_failures() {
    let cases = [
        "{\n/* unterminated comment\n\"a\": 1,\n}",
        "{\n/* unterminated trailing comment }",
        "{\n/ / bad comment }",
        "{// bad comment }",
        "{\n\"a\": 1\n}/",
        "{/* bad\ncomment *}",
    ];
    for case in cases {
        let res = parse_comments(case);
        assert!(res.is_err(), "expected failure for {:?}", case);
    }
}

#[test]
fn container_conversions_are_equal() {
    let l1: Vec<i32> = vec![1, 2, 3];
    let l2: Vec<i32> = vec![1, 2, 3];
    let l3: BTreeSet<i32> = [1, 2, 3].into_iter().collect();

    assert_eq!(Json::from(l1), Json::from(l2.clone()));
    assert_eq!(
        Json::from(l2),
        Json::array(l3.into_iter().collect::<Vec<_>>())
    );

    let m1: BTreeMap<String, String> = [("k1", "v1"), ("k2", "v2")]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    let m2: HashMap<String, String> = [("k1", "v1"), ("k2", "v2")]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    assert_eq!(Json::from(m1), Json::from(m2));
}

fn make_obj() -> Json {
    Json::object([
        ("k1", Json::from("v1")),
        ("k2", Json::from(42.0)),
        (
            "k3",
            Json::array([
                Json::from("a"),
                Json::from(123.0),
                Json::from(true),
                Json::from(false),
                Json::Null,
            ]),
        ),
    ])
}

#[test]
fn object_literal_dump() {
    let obj = make_obj();
    assert_eq!(
        obj.dump(),
        r#"{"k1": "v1", "k2": 42, "k3": ["a", 123, true, false, null]}"#
    );
}

#[test]
fn fallback_accessors() {
    assert_eq!(Json::from("a").number_value(), 0.0);
    assert_eq!(Json::from("a").string_value(), "a");
    assert_eq!(Json::Null.number_value(), 0.0);
}

#[test]
fn equality_and_number_unification() {
    let simple_test = r#"{"k1":"v1", "k2":42, "k3":["a",123,true,false,null]}"#;
    let json = parse_std(simple_test).expect("valid json");

    assert_eq!(make_obj(), json);
    assert_eq!(Json::from(42), Json::from(42.0));
    assert_ne!(Json::from(42), Json::from(42.1));
}

#[test]
fn unicode_escape_roundtrip() {
    let unicode_escape_test = r#"[ "blah\ud83d\udca9blah\ud83dblah\udca9blah\u0000blah\u1234" ]"#;

    let mut expected: Vec<u8> = Vec::new();
    expected.extend_from_slice(b"blah");
    expected.extend_from_slice(&[0xf0, 0x9f, 0x92, 0xa9]);
    expected.extend_from_slice(b"blah");
    expected.extend_from_slice(&[0xed, 0xa0, 0xbd]);
    expected.extend_from_slice(b"blah");
    expected.extend_from_slice(&[0xed, 0xb2, 0xa9]);
    expected.extend_from_slice(b"blah");
    expected.push(0x00);
    expected.extend_from_slice(b"blah");
    expected.extend_from_slice(&[0xe1, 0x88, 0xb4]);

    let uni = parse_std(unicode_escape_test).expect("valid json");
    let s = uni[0].string_value();
    assert_eq!(s.len(), expected.len());
    assert_eq!(s.as_bytes(), expected.as_slice());
}

#[test]
fn parse_multi_cases() {
    let good_json = r#" {"k1" : "v1"}"#;
    let bad_json1 = format!("{} {{", good_json);
    let bad_json2 = format!("{}{}", good_json, r#"{"k2":"v2", "k3":["#);

    let expect_kv = Json::object([("k1", Json::from("v1"))]);

    struct Case {
        input: String,
        stop_pos: usize,
        not_empty: usize,
        res: Json,
    }

    let cases = [
        Case {
            input: " {".to_string(),
            stop_pos: 0,
            not_empty: 0,
            res: Json::Null,
        },
        Case {
            input: good_json.to_string(),
            stop_pos: good_json.len(),
            not_empty: 1,
            res: expect_kv.clone(),
        },
        Case {
            input: bad_json1,
            stop_pos: good_json.len() + 1,
            not_empty: 1,
            res: expect_kv.clone(),
        },
        Case {
            input: bad_json2,
            stop_pos: good_json.len(),
            not_empty: 1,
            res: expect_kv.clone(),
        },
        Case {
            input: "{}".to_string(),
            stop_pos: 2,
            not_empty: 1,
            res: Json::object(Vec::<(String, Json)>::new()),
        },
    ];

    for case in cases {
        let (res, stop_pos) = Json::parse_multi(&case.input, JsonParse::Standard);
        assert_eq!(stop_pos, case.stop_pos, "stop_pos for {:?}", case.input);
        let not_empty = res.iter().filter(|j| !j.is_null()).count();
        assert_eq!(not_empty, case.not_empty, "count for {:?}", case.input);
        if !res.is_empty() {
            assert_eq!(res[0], case.res, "res[0] for {:?}", case.input);
        }
    }
}

#[test]
fn nested_object_dump() {
    let my_json = Json::object([
        ("key1", Json::from("value1")),
        ("key2", Json::from(false)),
        ("key3", Json::array([1, 2, 3])),
    ]);
    assert_eq!(
        my_json.dump(),
        r#"{"key1": "value1", "key2": false, "key3": [1, 2, 3]}"#
    );
}

#[derive(Clone)]
struct Point {
    x: i32,
    y: i32,
}

impl From<Point> for Json {
    fn from(p: Point) -> Json {
        Json::array([p.x, p.y])
    }
}

#[test]
fn user_defined_to_json() {
    // The ToJson convention (C++ `to_json()`) is available via the blanket impl.
    assert_eq!(Point { x: 1, y: 2 }.to_json().dump(), "[1, 2]");

    let points = vec![
        Point { x: 1, y: 2 },
        Point { x: 10, y: 20 },
        Point { x: 100, y: 200 },
    ];
    assert_eq!(Json::from(points).dump(), "[[1, 2], [10, 20], [100, 200]]");
}

#[test]
fn has_shape_cases() {
    let with_null = Json::object([("foo", Json::Null)]);
    assert!(with_null.has_shape(&[("foo", Type::Nul)]).is_ok());

    let with_num = Json::object([("foo", Json::from(1234567))]);
    assert!(with_num.has_shape(&[("foo", Type::Nul)]).is_err());

    let wrong_key = Json::object([("bar", Json::from(1234567))]);
    assert!(wrong_key.has_shape(&[("foo", Type::Nul)]).is_err());

    // Non-objects are rejected.
    assert!(Json::from(1).has_shape(&[("foo", Type::Nul)]).is_err());
}

#[test]
fn max_depth_limit() {
    let deep = "[".repeat(500);
    assert!(parse_std(&deep).is_err());
}
