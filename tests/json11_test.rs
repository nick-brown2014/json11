/* Copyright (c) 2013 Dropbox, Inc.
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in
 * all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
 * THE SOFTWARE.
 */

use std::collections::{BTreeMap, BTreeSet, HashMap};

use json11::{Json, JsonParse, Type};

/// Helper to build an object literal from `(&str, Json)` pairs.
fn object(items: impl IntoIterator<Item = (&'static str, Json)>) -> Json {
    Json::object(items.into_iter().map(|(k, v)| (k.to_string(), v)))
}

#[test]
fn json11_test() {
    let simple_test = r#"{"k1":"v1", "k2":42, "k3":["a",123,true,false,null]}"#;

    let json = Json::parse(simple_test, JsonParse::Standard).expect("parse simple_test");

    assert_eq!(json["k1"].string_value(), "v1");
    assert_eq!(json["k3"].dump(), r#"["a", 123, true, false, null]"#);

    // --- Comment-mode parsing (valid) ---
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
    let json_comment = Json::parse(comment_test, JsonParse::Comments);
    assert!(json_comment.is_ok());
    assert!(!json_comment.unwrap().is_null());

    let json_comment = Json::parse("{\"a\": 1}//trailing line comment", JsonParse::Comments);
    assert!(json_comment.is_ok());
    assert!(!json_comment.unwrap().is_null());

    let json_comment = Json::parse(
        "{\"a\": 1}/*trailing multi-line comment*/",
        JsonParse::Comments,
    );
    assert!(json_comment.is_ok());
    assert!(!json_comment.unwrap().is_null());

    // --- Comment-mode parsing (failing) ---
    let failing_cases = [
        "{\n/* unterminated comment\n\"a\": 1,\n}",
        "{\n/* unterminated trailing comment }",
        "{\n/ / bad comment }",
        "{// bad comment }",
        "{\n\"a\": 1\n}/",
        "{/* bad\ncomment *}",
    ];
    for case in failing_cases {
        let res = Json::parse(case, JsonParse::Comments);
        assert!(res.is_err(), "expected failure for {case:?}");
    }

    // --- list/vec/set equality conversions ---
    let l1: Vec<i32> = vec![1, 2, 3];
    let l2: Vec<i32> = vec![1, 2, 3];
    let l3: BTreeSet<i32> = [1, 2, 3].into_iter().collect();
    assert!(Json::from(l1) == Json::from(l2.clone()));
    assert!(Json::from(l2) == Json::from(l3));

    // --- map/unordered_map equality conversions ---
    let m1: BTreeMap<String, String> = [("k1", "v1"), ("k2", "v2")]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    let m2: HashMap<String, String> = [("k1", "v1"), ("k2", "v2")]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    assert!(Json::from(m1) == Json::from(m2));

    // --- Json literals ---
    let obj = object([
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
    ]);

    assert_eq!(
        obj.dump(),
        r#"{"k1": "v1", "k2": 42, "k3": ["a", 123, true, false, null]}"#
    );

    assert_eq!(Json::from("a").number_value(), 0.0);
    assert_eq!(Json::from("a").string_value(), "a");
    assert_eq!(Json::Null.number_value(), 0.0);

    assert!(obj == json);
    assert!(Json::from(42) == Json::from(42.0));
    assert!(Json::from(42) != Json::from(42.1));

    // --- Unicode escapes and UTF-16 surrogate-pair reassembly ---
    let unicode_escape_test = r#"[ "blah\ud83d\udca9blah\ud83dblah\udca9blah\u0000blah\u1234" ]"#;
    let utf8: &[u8] =
        b"blah\xf0\x9f\x92\xa9blah\xed\xa0\xbdblah\xed\xb2\xa9blah\x00blah\xe1\x88\xb4";

    let uni = Json::parse(unicode_escape_test, JsonParse::Standard).expect("parse unicode test");
    assert_eq!(uni[0].string_value().len(), utf8.len());
    assert_eq!(uni[0].string_value().as_bytes(), utf8);

    // --- parse_multi table-driven tests ---
    {
        let good_json = r#" {"k1" : "v1"}"#;
        let bad_json1 = format!("{good_json} {{");
        let bad_json2 = format!("{good_json}{{\"k2\":\"v2\", \"k3\":[");

        let k1v1 = object([("k1", Json::from("v1"))]);

        struct TestMultiParse {
            input: String,
            expect_parser_stop_pos: usize,
            expect_not_empty_elms_count: usize,
            expect_parse_res: Json,
        }
        let tests = [
            TestMultiParse {
                input: " {".to_string(),
                expect_parser_stop_pos: 0,
                expect_not_empty_elms_count: 0,
                expect_parse_res: Json::Null,
            },
            TestMultiParse {
                input: good_json.to_string(),
                expect_parser_stop_pos: good_json.len(),
                expect_not_empty_elms_count: 1,
                expect_parse_res: k1v1.clone(),
            },
            TestMultiParse {
                input: bad_json1.clone(),
                expect_parser_stop_pos: good_json.len() + 1,
                expect_not_empty_elms_count: 1,
                expect_parse_res: k1v1.clone(),
            },
            TestMultiParse {
                input: bad_json2.clone(),
                expect_parser_stop_pos: good_json.len(),
                expect_not_empty_elms_count: 1,
                expect_parse_res: k1v1.clone(),
            },
            TestMultiParse {
                input: "{}".to_string(),
                expect_parser_stop_pos: 2,
                expect_not_empty_elms_count: 1,
                expect_parse_res: Json::Object(BTreeMap::new()),
            },
        ];

        for tst in &tests {
            let (res, parser_stop_pos, _err) = Json::parse_multi(&tst.input, JsonParse::Standard);
            assert_eq!(parser_stop_pos, tst.expect_parser_stop_pos);
            let non_null = res.iter().filter(|j| !j.is_null()).count();
            assert_eq!(non_null, tst.expect_not_empty_elms_count);
            if !res.is_empty() {
                assert!(tst.expect_parse_res == res[0]);
            }
        }
    }

    // --- object dump ---
    let my_json = object([
        ("key1", Json::from("value1")),
        ("key2", Json::from(false)),
        (
            "key3",
            Json::array([Json::from(1), Json::from(2), Json::from(3)]),
        ),
    ]);
    assert_eq!(
        my_json.dump(),
        r#"{"key1": "value1", "key2": false, "key3": [1, 2, 3]}"#
    );

    // --- custom type with a to_json equivalent (From impl) ---
    struct Point {
        x: i32,
        y: i32,
    }
    impl From<Point> for Json {
        fn from(p: Point) -> Json {
            Json::array([Json::from(p.x), Json::from(p.y)])
        }
    }

    let points = vec![
        Point { x: 1, y: 2 },
        Point { x: 10, y: 20 },
        Point { x: 100, y: 200 },
    ];
    assert_eq!(Json::from(points).dump(), "[[1, 2], [10, 20], [100, 200]]");

    // --- has_shape ---
    assert!(object([("foo", Json::Null)])
        .has_shape(&[("foo".to_string(), Type::Null)])
        .is_ok());
    assert!(object([("foo", Json::from(1234567))])
        .has_shape(&[("foo".to_string(), Type::Null)])
        .is_err());
    assert!(object([("bar", Json::from(1234567))])
        .has_shape(&[("foo".to_string(), Type::Null)])
        .is_err());
}
