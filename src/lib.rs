/* json11
 *
 * json11 is a tiny JSON library, providing JSON parsing and serialization.
 *
 * This file is an idiomatic Rust port of the original C++11 json11 library.
 *
 * The core object provided by the library is [`Json`]. A `Json` value represents any JSON
 * value: null, bool, number, string ([`String`]), array ([`Vec`]), or object
 * ([`BTreeMap`]).
 *
 * `Json` values act like values: they can be cloned, compared for equality or order, etc.
 * There are also helper methods [`Json::dump`], to serialize a `Json` to a string, and
 * [`Json::parse`] to parse a string as a `Json` value.
 *
 * A note on numbers - JSON specifies the syntax of number formatting but not its semantics,
 * so some JSON implementations distinguish between integers and floating-point numbers, while
 * some don't. In json11, we choose the latter. Because some JSON implementations (namely
 * Javascript itself) treat all numbers as the same type, distinguishing the two leads
 * to JSON that will be *silently* changed by a round-trip through those implementations.
 * Dangerous! To avoid that risk, json11 stores all numbers as `f64` internally, but also
 * provides an integer helper ([`Json::int_value`]).
 */

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

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::ops::Index;

const MAX_DEPTH: i32 = 200;

/// Parsing strategy: whether to allow C-style comments in the input.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum JsonParse {
    Standard,
    Comments,
}

/// The type tag of a [`Json`] value.
///
/// The ordering of the variants matches the C++ library
/// (`Null < Number < Bool < Str < Array < Object`) and is relied upon by
/// [`Json`]'s ordering.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Type {
    Null,
    Number,
    Bool,
    Str,
    Array,
    Object,
}

/// A JSON value.
///
/// All numbers are stored as `f64`, matching the original C++ library's decision to
/// store every number as a `double`.
#[derive(Clone, Debug, Default)]
pub enum Json {
    #[default]
    Null,
    Number(f64),
    Bool(bool),
    Str(String),
    Array(Vec<Json>),
    Object(BTreeMap<String, Json>),
}

static NULL_JSON: Json = Json::Null;
static EMPTY_MAP: BTreeMap<String, Json> = BTreeMap::new();

/* * * * * * * * * * * * * * * * * * * *
 * Conversions (replacing the C++ implicit constructors)
 */

impl From<f64> for Json {
    fn from(value: f64) -> Self {
        Json::Number(value)
    }
}

impl From<i32> for Json {
    fn from(value: i32) -> Self {
        Json::Number(value as f64)
    }
}

impl From<bool> for Json {
    fn from(value: bool) -> Self {
        Json::Bool(value)
    }
}

impl From<&str> for Json {
    fn from(value: &str) -> Self {
        Json::Str(value.to_string())
    }
}

impl From<String> for Json {
    fn from(value: String) -> Self {
        Json::Str(value)
    }
}

impl From<&String> for Json {
    fn from(value: &String) -> Self {
        Json::Str(value.clone())
    }
}

impl<T: Into<Json>> From<Vec<T>> for Json {
    fn from(values: Vec<T>) -> Self {
        Json::Array(values.into_iter().map(Into::into).collect())
    }
}

impl<T: Into<Json>> From<BTreeSet<T>> for Json {
    fn from(values: BTreeSet<T>) -> Self {
        Json::Array(values.into_iter().map(Into::into).collect())
    }
}

impl<T: Into<Json>> From<HashSet<T>> for Json {
    fn from(values: HashSet<T>) -> Self {
        Json::Array(values.into_iter().map(Into::into).collect())
    }
}

impl<K: Into<String>, V: Into<Json>> From<BTreeMap<K, V>> for Json {
    fn from(values: BTreeMap<K, V>) -> Self {
        Json::Object(
            values
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        )
    }
}

impl<K: Into<String>, V: Into<Json>> From<HashMap<K, V>> for Json {
    fn from(values: HashMap<K, V>) -> Self {
        Json::Object(
            values
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        )
    }
}

/* * * * * * * * * * * * * * * * * * * *
 * Accessors
 */

impl Json {
    /// Build a JSON array from an iterator of values.
    pub fn array<I: IntoIterator<Item = Json>>(items: I) -> Json {
        Json::Array(items.into_iter().collect())
    }

    /// Build a JSON object from an iterator of `(key, value)` pairs.
    pub fn object<I: IntoIterator<Item = (String, Json)>>(items: I) -> Json {
        Json::Object(items.into_iter().collect())
    }

    /// The type tag of this value.
    #[allow(clippy::wrong_self_convention)]
    pub fn r#type(&self) -> Type {
        match self {
            Json::Null => Type::Null,
            Json::Number(_) => Type::Number,
            Json::Bool(_) => Type::Bool,
            Json::Str(_) => Type::Str,
            Json::Array(_) => Type::Array,
            Json::Object(_) => Type::Object,
        }
    }

    pub fn is_null(&self) -> bool {
        self.r#type() == Type::Null
    }
    pub fn is_number(&self) -> bool {
        self.r#type() == Type::Number
    }
    pub fn is_bool(&self) -> bool {
        self.r#type() == Type::Bool
    }
    pub fn is_string(&self) -> bool {
        self.r#type() == Type::Str
    }
    pub fn is_array(&self) -> bool {
        self.r#type() == Type::Array
    }
    pub fn is_object(&self) -> bool {
        self.r#type() == Type::Object
    }

    /// Return the enclosed value if this is a number, `0.0` otherwise.
    pub fn number_value(&self) -> f64 {
        match self {
            Json::Number(n) => *n,
            _ => 0.0,
        }
    }

    /// Return the enclosed value truncated to `i32` if this is a number, `0` otherwise.
    pub fn int_value(&self) -> i32 {
        match self {
            Json::Number(n) => *n as i32,
            _ => 0,
        }
    }

    /// Return the enclosed value if this is a boolean, `false` otherwise.
    pub fn bool_value(&self) -> bool {
        match self {
            Json::Bool(b) => *b,
            _ => false,
        }
    }

    /// Return the enclosed string if this is a string, `""` otherwise.
    pub fn string_value(&self) -> &str {
        match self {
            Json::Str(s) => s,
            _ => "",
        }
    }

    /// Return the enclosed slice if this is an array, an empty slice otherwise.
    pub fn array_items(&self) -> &[Json] {
        match self {
            Json::Array(a) => a,
            _ => &[],
        }
    }

    /// Return the enclosed map if this is an object, an empty map otherwise.
    pub fn object_items(&self) -> &BTreeMap<String, Json> {
        match self {
            Json::Object(o) => o,
            _ => &EMPTY_MAP,
        }
    }

    /* * * * * * * * * * * * * * * * * * * *
     * Serialization
     */

    /// Serialize this value to a JSON string.
    pub fn dump(&self) -> String {
        let mut out: Vec<u8> = Vec::new();
        self.dump_bytes(&mut out);
        // SAFETY: the serializer only emits ASCII structural bytes plus the raw bytes
        // of the contained strings. For well-formed values this is valid UTF-8; the
        // parser can, like the C++ original, produce strings holding WTF-8 (lone
        // surrogates), and we faithfully round-trip those bytes here.
        unsafe { String::from_utf8_unchecked(out) }
    }

    fn dump_bytes(&self, out: &mut Vec<u8>) {
        match self {
            Json::Null => out.extend_from_slice(b"null"),
            Json::Number(value) => dump_number(*value, out),
            Json::Bool(value) => out.extend_from_slice(if *value { b"true" } else { b"false" }),
            Json::Str(value) => dump_string(value, out),
            Json::Array(values) => {
                out.push(b'[');
                let mut first = true;
                for value in values {
                    if !first {
                        out.extend_from_slice(b", ");
                    }
                    value.dump_bytes(out);
                    first = false;
                }
                out.push(b']');
            }
            Json::Object(values) => {
                out.push(b'{');
                let mut first = true;
                for (key, value) in values {
                    if !first {
                        out.extend_from_slice(b", ");
                    }
                    dump_string(key, out);
                    out.extend_from_slice(b": ");
                    value.dump_bytes(out);
                    first = false;
                }
                out.push(b'}');
            }
        }
    }

    /* * * * * * * * * * * * * * * * * * * *
     * Parsing
     */

    /// Parse `input` as a single JSON value.
    pub fn parse(input: &str, strategy: JsonParse) -> Result<Json, String> {
        let mut parser = JsonParser::new(input.as_bytes(), strategy);
        let result = parser.parse_json(0);

        // Check for any trailing garbage.
        parser.consume_garbage();
        if parser.failed {
            return Err(parser.err);
        }
        if parser.i != parser.bytes.len() {
            let msg = format!("unexpected trailing {}", esc(parser.at(parser.i)));
            parser.fail_str(msg);
            return Err(parser.err);
        }

        Ok(result)
    }

    /// Parse multiple JSON values, concatenated or separated by whitespace.
    ///
    /// Returns the parsed values, the position at which parsing stopped, and either
    /// `Ok(())` or an error message.
    pub fn parse_multi(input: &str, strategy: JsonParse) -> (Vec<Json>, usize, Result<(), String>) {
        let mut parser = JsonParser::new(input.as_bytes(), strategy);
        let mut stop_pos = 0;
        let mut json_vec = Vec::new();
        while parser.i != parser.bytes.len() && !parser.failed {
            json_vec.push(parser.parse_json(0));
            if parser.failed {
                break;
            }

            // Check for another object.
            parser.consume_garbage();
            if parser.failed {
                break;
            }
            stop_pos = parser.i;
        }

        let res = if parser.failed {
            Err(parser.err)
        } else {
            Ok(())
        };
        (json_vec, stop_pos, res)
    }

    /* * * * * * * * * * * * * * * * * * * *
     * Shape-checking
     */

    /// Return `Ok(())` if this is a JSON object and, for each item in `types`, has a
    /// field of the given type. Otherwise return a descriptive error message.
    pub fn has_shape(&self, types: &[(String, Type)]) -> Result<(), String> {
        let obj = match self {
            Json::Object(o) => o,
            _ => return Err(format!("expected JSON object, got {}", self.dump())),
        };

        for (key, ty) in types {
            match obj.get(key) {
                Some(v) if v.r#type() == *ty => {}
                _ => return Err(format!("bad type for {} in {}", key, self.dump())),
            }
        }

        Ok(())
    }
}

/* * * * * * * * * * * * * * * * * * * *
 * Indexing
 */

impl Index<usize> for Json {
    type Output = Json;

    /// Return a reference to `arr[i]` if this is an array, a shared `Json::Null` otherwise.
    fn index(&self, i: usize) -> &Json {
        match self {
            Json::Array(a) => a.get(i).unwrap_or(&NULL_JSON),
            _ => &NULL_JSON,
        }
    }
}

impl Index<&str> for Json {
    type Output = Json;

    /// Return a reference to `obj[key]` if this is an object, a shared `Json::Null` otherwise.
    fn index(&self, key: &str) -> &Json {
        match self {
            Json::Object(o) => o.get(key).unwrap_or(&NULL_JSON),
            _ => &NULL_JSON,
        }
    }
}

/* * * * * * * * * * * * * * * * * * * *
 * Comparison
 */

impl PartialEq for Json {
    fn eq(&self, other: &Json) -> bool {
        match (self, other) {
            (Json::Null, Json::Null) => true,
            (Json::Number(a), Json::Number(b)) => a == b,
            (Json::Bool(a), Json::Bool(b)) => a == b,
            (Json::Str(a), Json::Str(b)) => a == b,
            (Json::Array(a), Json::Array(b)) => a == b,
            (Json::Object(a), Json::Object(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for Json {}

impl PartialOrd for Json {
    fn partial_cmp(&self, other: &Json) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Json {
    fn cmp(&self, other: &Json) -> Ordering {
        match (self, other) {
            (Json::Number(a), Json::Number(b)) => a.partial_cmp(b).unwrap_or(Ordering::Equal),
            (Json::Bool(a), Json::Bool(b)) => a.cmp(b),
            (Json::Str(a), Json::Str(b)) => a.cmp(b),
            (Json::Array(a), Json::Array(b)) => a.cmp(b),
            (Json::Object(a), Json::Object(b)) => a.cmp(b),
            _ => self.r#type().cmp(&other.r#type()),
        }
    }
}

/* * * * * * * * * * * * * * * * * * * *
 * Serialization helpers
 */

/// Format a `f64` the way the C++ library does with `snprintf(buf, "%.17g", value)`,
/// serializing non-finite values as `null`.
fn dump_number(value: f64, out: &mut Vec<u8>) {
    if !value.is_finite() {
        out.extend_from_slice(b"null");
        return;
    }
    out.extend_from_slice(format_g17(value).as_bytes());
}

/// Emulate C's `printf("%.17g", value)` for a finite `value`.
fn format_g17(value: f64) -> String {
    const P: i32 = 17;

    // Determine the decimal exponent as C would via a `%e` conversion with P-1 digits.
    let sci = format!("{:.*e}", (P - 1) as usize, value);
    let (mantissa, exp_str) = sci.split_once('e').expect("scientific format contains 'e'");
    let exp: i32 = exp_str.parse().expect("valid exponent");

    if !(-4..P).contains(&exp) {
        // Style 'e': strip trailing zeros from the mantissa, print a signed, >= 2-digit exponent.
        let mantissa = strip_trailing_zeros(mantissa);
        let sign = if exp < 0 { '-' } else { '+' };
        format!("{}e{}{:02}", mantissa, sign, exp.abs())
    } else {
        // Style 'f' with precision P-1-exp, then strip trailing zeros.
        let prec = (P - 1 - exp).max(0) as usize;
        strip_trailing_zeros(&format!("{:.*}", prec, value))
    }
}

fn strip_trailing_zeros(s: &str) -> String {
    if !s.contains('.') {
        return s.to_string();
    }
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn dump_string(value: &str, out: &mut Vec<u8>) {
    let bytes = value.as_bytes();
    out.push(b'"');
    let mut i = 0;
    while i < bytes.len() {
        let ch = bytes[i];
        match ch {
            b'\\' => out.extend_from_slice(b"\\\\"),
            b'"' => out.extend_from_slice(b"\\\""),
            0x08 => out.extend_from_slice(b"\\b"),
            0x0c => out.extend_from_slice(b"\\f"),
            b'\n' => out.extend_from_slice(b"\\n"),
            b'\r' => out.extend_from_slice(b"\\r"),
            b'\t' => out.extend_from_slice(b"\\t"),
            _ if ch <= 0x1f => {
                out.extend_from_slice(format!("\\u{:04x}", ch).as_bytes());
            }
            0xe2 if i + 2 < bytes.len() && bytes[i + 1] == 0x80 && bytes[i + 2] == 0xa8 => {
                out.extend_from_slice(b"\\u2028");
                i += 2;
            }
            0xe2 if i + 2 < bytes.len() && bytes[i + 1] == 0x80 && bytes[i + 2] == 0xa9 => {
                out.extend_from_slice(b"\\u2029");
                i += 2;
            }
            _ => out.push(ch),
        }
        i += 1;
    }
    out.push(b'"');
}

/* * * * * * * * * * * * * * * * * * * *
 * Parsing helpers
 */

/// Format a byte suitable for printing in an error message.
fn esc(c: u8) -> String {
    if (0x20..=0x7f).contains(&c) {
        format!("'{}' ({})", c as char, c as i8 as i32)
    } else {
        format!("({})", c as i8 as i32)
    }
}

fn is_hex(c: u8) -> bool {
    c.is_ascii_digit() || (b'a'..=b'f').contains(&c) || (b'A'..=b'F').contains(&c)
}

fn encode_utf8(pt: i64, out: &mut Vec<u8>) {
    if pt < 0 {
        return;
    }

    if pt < 0x80 {
        out.push(pt as u8);
    } else if pt < 0x800 {
        out.push(((pt >> 6) as u8) | 0xC0);
        out.push(((pt & 0x3F) as u8) | 0x80);
    } else if pt < 0x10000 {
        out.push(((pt >> 12) as u8) | 0xE0);
        out.push((((pt >> 6) & 0x3F) as u8) | 0x80);
        out.push(((pt & 0x3F) as u8) | 0x80);
    } else {
        out.push(((pt >> 18) as u8) | 0xF0);
        out.push((((pt >> 12) & 0x3F) as u8) | 0x80);
        out.push((((pt >> 6) & 0x3F) as u8) | 0x80);
        out.push(((pt & 0x3F) as u8) | 0x80);
    }
}

/// Object that tracks all state of an in-progress parse.
struct JsonParser<'a> {
    bytes: &'a [u8],
    i: usize,
    err: String,
    failed: bool,
    strategy: JsonParse,
}

impl<'a> JsonParser<'a> {
    fn new(bytes: &'a [u8], strategy: JsonParse) -> Self {
        JsonParser {
            bytes,
            i: 0,
            err: String::new(),
            failed: false,
            strategy,
        }
    }

    /// The byte at `idx`, or 0 past the end (mirroring C++'s `std::string` NUL terminator).
    fn at(&self, idx: usize) -> u8 {
        self.bytes.get(idx).copied().unwrap_or(0)
    }

    /// The current byte.
    fn cur(&self) -> u8 {
        self.at(self.i)
    }

    /// Mark this parse as failed with the given message.
    fn fail_str(&mut self, msg: String) {
        if !self.failed {
            self.err = msg;
        }
        self.failed = true;
    }

    fn fail_json(&mut self, msg: String) -> Json {
        self.fail_str(msg);
        Json::Null
    }

    /// Advance until the current character is non-whitespace.
    fn consume_whitespace(&mut self) {
        while matches!(self.cur(), b' ' | b'\r' | b'\n' | b'\t') {
            self.i += 1;
        }
    }

    /// Advance past a single comment (C-style inline or multiline). Returns whether a
    /// comment was found.
    fn consume_comment(&mut self) -> bool {
        let mut comment_found = false;
        if self.cur() == b'/' {
            self.i += 1;
            if self.i == self.bytes.len() {
                self.fail_str("unexpected end of input after start of comment".to_string());
                return false;
            }
            if self.cur() == b'/' {
                // inline comment
                self.i += 1;
                while self.i < self.bytes.len() && self.cur() != b'\n' {
                    self.i += 1;
                }
                comment_found = true;
            } else if self.cur() == b'*' {
                // multiline comment
                self.i += 1;
                if self.i + 2 > self.bytes.len() {
                    self.fail_str("unexpected end of input inside multi-line comment".to_string());
                    return false;
                }
                while !(self.cur() == b'*' && self.at(self.i + 1) == b'/') {
                    self.i += 1;
                    if self.i + 2 > self.bytes.len() {
                        self.fail_str(
                            "unexpected end of input inside multi-line comment".to_string(),
                        );
                        return false;
                    }
                }
                self.i += 2;
                comment_found = true;
            } else {
                self.fail_str("malformed comment".to_string());
                return false;
            }
        }
        comment_found
    }

    /// Advance until the current character is non-whitespace and non-comment.
    fn consume_garbage(&mut self) {
        self.consume_whitespace();
        if self.strategy == JsonParse::Comments {
            loop {
                let comment_found = self.consume_comment();
                if self.failed {
                    return;
                }
                self.consume_whitespace();
                if !comment_found {
                    break;
                }
            }
        }
    }

    /// Return the next non-whitespace character. If the end of the input is reached, flag
    /// an error and return 0.
    fn get_next_token(&mut self) -> u8 {
        self.consume_garbage();
        if self.failed {
            return 0;
        }
        if self.i == self.bytes.len() {
            self.fail_str("unexpected end of input".to_string());
            return 0;
        }
        let c = self.cur();
        self.i += 1;
        c
    }

    /// Parse a string, starting at the current position, returning its raw bytes.
    fn parse_string(&mut self) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::new();
        let mut last_escaped_codepoint: i64 = -1;
        loop {
            if self.i == self.bytes.len() {
                self.fail_str("unexpected end of input in string".to_string());
                return out;
            }

            let ch = self.cur();
            self.i += 1;

            if ch == b'"' {
                encode_utf8(last_escaped_codepoint, &mut out);
                return out;
            }

            if ch <= 0x1f {
                self.fail_str(format!("unescaped {} in string", esc(ch)));
                return out;
            }

            // The usual case: non-escaped characters.
            if ch != b'\\' {
                encode_utf8(last_escaped_codepoint, &mut out);
                last_escaped_codepoint = -1;
                out.push(ch);
                continue;
            }

            // Handle escapes.
            if self.i == self.bytes.len() {
                self.fail_str("unexpected end of input in string".to_string());
                return out;
            }

            let ch = self.cur();
            self.i += 1;

            if ch == b'u' {
                // Extract 4-hex-digit escape sequence.
                let end = (self.i + 4).min(self.bytes.len());
                let esc_seq = &self.bytes[self.i..end];
                if esc_seq.len() < 4 {
                    self.fail_str(format!(
                        "bad \\u escape: {}",
                        String::from_utf8_lossy(esc_seq)
                    ));
                    return out;
                }
                for &b in esc_seq {
                    if !is_hex(b) {
                        self.fail_str(format!(
                            "bad \\u escape: {}",
                            String::from_utf8_lossy(esc_seq)
                        ));
                        return out;
                    }
                }

                let codepoint = i64::from_str_radix(
                    std::str::from_utf8(esc_seq).expect("hex digits are ASCII"),
                    16,
                )
                .expect("validated hex");

                // Reassemble UTF-16 surrogate pairs into a single astral-plane character.
                if (0xD800..=0xDBFF).contains(&last_escaped_codepoint)
                    && (0xDC00..=0xDFFF).contains(&codepoint)
                {
                    encode_utf8(
                        (((last_escaped_codepoint - 0xD800) << 10) | (codepoint - 0xDC00))
                            + 0x10000,
                        &mut out,
                    );
                    last_escaped_codepoint = -1;
                } else {
                    encode_utf8(last_escaped_codepoint, &mut out);
                    last_escaped_codepoint = codepoint;
                }

                self.i += 4;
                continue;
            }

            encode_utf8(last_escaped_codepoint, &mut out);
            last_escaped_codepoint = -1;

            match ch {
                b'b' => out.push(0x08),
                b'f' => out.push(0x0c),
                b'n' => out.push(b'\n'),
                b'r' => out.push(b'\r'),
                b't' => out.push(b'\t'),
                b'"' | b'\\' | b'/' => out.push(ch),
                _ => {
                    self.fail_str(format!("invalid escape character {}", esc(ch)));
                    return out;
                }
            }
        }
    }

    /// Parse a number.
    fn parse_number(&mut self) -> Json {
        let start_pos = self.i;

        if self.cur() == b'-' {
            self.i += 1;
        }

        // Integer part.
        if self.cur() == b'0' {
            self.i += 1;
            if self.cur().is_ascii_digit() {
                return self.fail_json("leading 0s not permitted in numbers".to_string());
            }
        } else if (b'1'..=b'9').contains(&self.cur()) {
            self.i += 1;
            while self.cur().is_ascii_digit() {
                self.i += 1;
            }
        } else {
            return self.fail_json(format!("invalid {} in number", esc(self.cur())));
        }

        // Decimal part.
        if self.cur() == b'.' {
            self.i += 1;
            if !self.cur().is_ascii_digit() {
                return self
                    .fail_json("at least one digit required in fractional part".to_string());
            }
            while self.cur().is_ascii_digit() {
                self.i += 1;
            }
        }

        // Exponent part.
        if self.cur() == b'e' || self.cur() == b'E' {
            self.i += 1;
            if self.cur() == b'+' || self.cur() == b'-' {
                self.i += 1;
            }
            if !self.cur().is_ascii_digit() {
                return self.fail_json("at least one digit required in exponent".to_string());
            }
            while self.cur().is_ascii_digit() {
                self.i += 1;
            }
        }

        let text =
            std::str::from_utf8(&self.bytes[start_pos..self.i]).expect("number text is ASCII");
        Json::Number(text.parse::<f64>().expect("validated number"))
    }

    /// Expect that `expected` starts at the character that was just read. If it does,
    /// advance the input and return `res`. If not, flag an error.
    fn expect(&mut self, expected: &str, res: Json) -> Json {
        debug_assert!(self.i != 0);
        self.i -= 1;
        let end = self.i + expected.len();
        if end <= self.bytes.len() && &self.bytes[self.i..end] == expected.as_bytes() {
            self.i = end;
            res
        } else {
            let got = String::from_utf8_lossy(&self.bytes[self.i..end.min(self.bytes.len())]);
            self.fail_json(format!("parse error: expected {}, got {}", expected, got))
        }
    }

    /// Parse a JSON value.
    fn parse_json(&mut self, depth: i32) -> Json {
        if depth > MAX_DEPTH {
            return self.fail_json("exceeded maximum nesting depth".to_string());
        }

        let ch = self.get_next_token();
        if self.failed {
            return Json::Null;
        }

        if ch == b'-' || ch.is_ascii_digit() {
            self.i -= 1;
            return self.parse_number();
        }

        if ch == b't' {
            return self.expect("true", Json::Bool(true));
        }

        if ch == b'f' {
            return self.expect("false", Json::Bool(false));
        }

        if ch == b'n' {
            return self.expect("null", Json::Null);
        }

        if ch == b'"' {
            let bytes = self.parse_string();
            if self.failed {
                return Json::Null;
            }
            // SAFETY: see the note on `Json::dump`; json11 strings may hold WTF-8.
            return Json::Str(unsafe { String::from_utf8_unchecked(bytes) });
        }

        if ch == b'{' {
            let mut data: BTreeMap<String, Json> = BTreeMap::new();
            let mut ch = self.get_next_token();
            if ch == b'}' {
                return Json::Object(data);
            }

            loop {
                if ch != b'"' {
                    return self.fail_json(format!("expected '\"' in object, got {}", esc(ch)));
                }

                let key_bytes = self.parse_string();
                if self.failed {
                    return Json::Null;
                }
                // SAFETY: see the note on `Json::dump`; json11 strings may hold WTF-8.
                let key = unsafe { String::from_utf8_unchecked(key_bytes) };

                ch = self.get_next_token();
                if ch != b':' {
                    return self.fail_json(format!("expected ':' in object, got {}", esc(ch)));
                }

                let value = self.parse_json(depth + 1);
                if self.failed {
                    return Json::Null;
                }
                data.insert(key, value);

                ch = self.get_next_token();
                if ch == b'}' {
                    break;
                }
                if ch != b',' {
                    return self.fail_json(format!("expected ',' in object, got {}", esc(ch)));
                }

                ch = self.get_next_token();
            }
            return Json::Object(data);
        }

        if ch == b'[' {
            let mut data: Vec<Json> = Vec::new();
            let mut ch = self.get_next_token();
            if ch == b']' {
                return Json::Array(data);
            }

            loop {
                self.i = self.i.saturating_sub(1);
                let value = self.parse_json(depth + 1);
                if self.failed {
                    return Json::Null;
                }
                data.push(value);

                ch = self.get_next_token();
                if ch == b']' {
                    break;
                }
                if ch != b',' {
                    return self.fail_json(format!("expected ',' in list, got {}", esc(ch)));
                }

                ch = self.get_next_token();
                let _ = ch;
            }
            return Json::Array(data);
        }

        self.fail_json(format!("expected value, got {}", esc(ch)))
    }
}
