//! json11 — a tiny JSON library, an idiomatic Rust port of the C++11
//! [json11](https://github.com/dropbox/json11) library by Dropbox.
//!
//! The core type is [`Json`], an enum that represents any JSON value: null,
//! bool, number, string, array, or object. Following the original library, all
//! numbers are stored as `f64` (see [`Json::number_value`] / [`Json::int_value`]),
//! and objects preserve ordered-key semantics by using a [`BTreeMap`] (the
//! Rust analogue of the C++ `std::map`).
//!
//! ```
//! use json11::{Json, JsonParse};
//!
//! let doc = Json::parse(r#"{"k1":"v1", "k2":42}"#, JsonParse::Standard).unwrap();
//! assert_eq!(doc["k1"].string_value(), "v1");
//! assert_eq!(doc["k2"].int_value(), 42);
//! assert_eq!(doc.dump(), r#"{"k1": "v1", "k2": 42}"#);
//! ```

use std::collections::{BTreeMap, HashMap};
use std::sync::LazyLock;

/// Maximum nesting depth accepted by the parser (matches the C++ library).
const MAX_DEPTH: i32 = 200;

/// Parsing strategy: strict standard JSON, or JSON with C-style comments.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JsonParse {
    /// Strict RFC-style JSON, no comments allowed.
    Standard,
    /// Allow `//` inline and `/* ... */` multi-line comments.
    Comments,
}

/// The dynamic type tag of a [`Json`] value.
///
/// The declaration order matches the C++ `Json::Type` enum so that the
/// `Ord` derived here reproduces the original cross-type ordering.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Type {
    /// `null`
    Nul,
    /// a number (stored as `f64`)
    Number,
    /// a boolean
    Bool,
    /// a string
    String,
    /// an array
    Array,
    /// an object
    Object,
}

/// A JSON value.
///
/// Numbers are always stored as `f64`; use [`Json::number_value`] and
/// [`Json::int_value`] to read them. Objects use a [`BTreeMap`] so keys are
/// serialized in sorted order, matching the C++ `std::map` behavior.
#[derive(Clone, Debug, Default, PartialEq, PartialOrd)]
pub enum Json {
    /// `null`
    #[default]
    Null,
    /// a boolean
    Bool(bool),
    /// a number
    Number(f64),
    /// a string
    Str(String),
    /// an array
    Array(Vec<Json>),
    /// an object with sorted keys
    Object(BTreeMap<String, Json>),
}

/// A single shared `Json::Null`, returned by reference from the indexing
/// operators when a key or index is missing.
static NULL_JSON: Json = Json::Null;

static EMPTY_OBJECT: LazyLock<BTreeMap<String, Json>> = LazyLock::new(BTreeMap::new);

/* * * * * * * * * * * * * * * * * * * *
 * Constructors / conversions
 */

impl From<f64> for Json {
    fn from(v: f64) -> Json {
        Json::Number(v)
    }
}

impl From<i32> for Json {
    fn from(v: i32) -> Json {
        Json::Number(v as f64)
    }
}

impl From<bool> for Json {
    fn from(v: bool) -> Json {
        Json::Bool(v)
    }
}

impl From<&str> for Json {
    fn from(v: &str) -> Json {
        Json::Str(v.to_string())
    }
}

impl From<String> for Json {
    fn from(v: String) -> Json {
        Json::Str(v)
    }
}

impl From<&String> for Json {
    fn from(v: &String) -> Json {
        Json::Str(v.clone())
    }
}

impl<T: Into<Json>> From<Option<T>> for Json {
    fn from(v: Option<T>) -> Json {
        match v {
            Some(t) => t.into(),
            None => Json::Null,
        }
    }
}

impl<T: Into<Json>> From<Vec<T>> for Json {
    fn from(v: Vec<T>) -> Json {
        Json::Array(v.into_iter().map(Into::into).collect())
    }
}

impl<K: Into<String>, V: Into<Json>> From<BTreeMap<K, V>> for Json {
    fn from(m: BTreeMap<K, V>) -> Json {
        Json::Object(m.into_iter().map(|(k, v)| (k.into(), v.into())).collect())
    }
}

impl<K: Into<String>, V: Into<Json>, S> From<HashMap<K, V, S>> for Json {
    fn from(m: HashMap<K, V, S>) -> Json {
        Json::Object(m.into_iter().map(|(k, v)| (k.into(), v.into())).collect())
    }
}

/// Trait implemented by any type that can be converted into a [`Json`] value.
///
/// This is the Rust equivalent of the C++ `to_json()` convention: user-defined
/// types can participate in `Json` construction by implementing
/// `From<MyType> for Json`, which automatically grants them a `to_json()`
/// method via the blanket implementation below.
///
/// ```
/// use json11::{Json, ToJson};
///
/// #[derive(Clone)]
/// struct Point { x: i32, y: i32 }
///
/// impl From<Point> for Json {
///     fn from(p: Point) -> Json {
///         Json::array([p.x, p.y])
///     }
/// }
///
/// assert_eq!(Point { x: 1, y: 2 }.to_json().dump(), "[1, 2]");
/// ```
pub trait ToJson {
    /// Convert `self` into a [`Json`] value.
    fn to_json(&self) -> Json;
}

impl<T: Into<Json> + Clone> ToJson for T {
    fn to_json(&self) -> Json {
        self.clone().into()
    }
}

impl Json {
    /// Build an array from anything iterable whose items convert into `Json`.
    ///
    /// Mirrors the C++ `Json::array { ... }` initializer syntax.
    pub fn array<I, T>(items: I) -> Json
    where
        I: IntoIterator<Item = T>,
        T: Into<Json>,
    {
        Json::Array(items.into_iter().map(Into::into).collect())
    }

    /// Build an object from anything iterable of `(key, value)` pairs.
    ///
    /// Mirrors the C++ `Json::object { ... }` initializer syntax.
    pub fn object<I, K, V>(items: I) -> Json
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<Json>,
    {
        Json::Object(
            items
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
    /// Return the [`Type`] tag of this value.
    ///
    /// Named `r#type` because `type` is a reserved word; call it as
    /// `value.r#type()`.
    pub fn r#type(&self) -> Type {
        match self {
            Json::Null => Type::Nul,
            Json::Number(_) => Type::Number,
            Json::Bool(_) => Type::Bool,
            Json::Str(_) => Type::String,
            Json::Array(_) => Type::Array,
            Json::Object(_) => Type::Object,
        }
    }

    /// `true` if this value is `null`.
    pub fn is_null(&self) -> bool {
        matches!(self, Json::Null)
    }
    /// `true` if this value is a number.
    pub fn is_number(&self) -> bool {
        matches!(self, Json::Number(_))
    }
    /// `true` if this value is a boolean.
    pub fn is_bool(&self) -> bool {
        matches!(self, Json::Bool(_))
    }
    /// `true` if this value is a string.
    pub fn is_string(&self) -> bool {
        matches!(self, Json::Str(_))
    }
    /// `true` if this value is an array.
    pub fn is_array(&self) -> bool {
        matches!(self, Json::Array(_))
    }
    /// `true` if this value is an object.
    pub fn is_object(&self) -> bool {
        matches!(self, Json::Object(_))
    }

    /// Return the number if this is a number, `0.0` otherwise.
    pub fn number_value(&self) -> f64 {
        match self {
            Json::Number(v) => *v,
            _ => 0.0,
        }
    }

    /// Return the number truncated to an `i32` if this is a number, `0` otherwise.
    pub fn int_value(&self) -> i32 {
        match self {
            Json::Number(v) => *v as i32,
            _ => 0,
        }
    }

    /// Return the boolean if this is a boolean, `false` otherwise.
    pub fn bool_value(&self) -> bool {
        match self {
            Json::Bool(v) => *v,
            _ => false,
        }
    }

    /// Return the string if this is a string, `""` otherwise.
    pub fn string_value(&self) -> &str {
        match self {
            Json::Str(s) => s,
            _ => "",
        }
    }

    /// Return the items if this is an array, an empty slice otherwise.
    pub fn array_items(&self) -> &[Json] {
        match self {
            Json::Array(a) => a,
            _ => &[],
        }
    }

    /// Return the map if this is an object, an empty map otherwise.
    pub fn object_items(&self) -> &BTreeMap<String, Json> {
        match self {
            Json::Object(m) => m,
            _ => &EMPTY_OBJECT,
        }
    }
}

impl std::ops::Index<usize> for Json {
    type Output = Json;
    /// Index into an array. Returns a reference to `Json::Null` on a miss or if
    /// this is not an array.
    fn index(&self, i: usize) -> &Json {
        match self {
            Json::Array(a) => a.get(i).unwrap_or(&NULL_JSON),
            _ => &NULL_JSON,
        }
    }
}

impl std::ops::Index<&str> for Json {
    type Output = Json;
    /// Index into an object by key. Returns a reference to `Json::Null` on a
    /// miss or if this is not an object.
    fn index(&self, key: &str) -> &Json {
        match self {
            Json::Object(m) => m.get(key).unwrap_or(&NULL_JSON),
            _ => &NULL_JSON,
        }
    }
}

/* * * * * * * * * * * * * * * * * * * *
 * Serialization
 */

/// Format a finite `f64` the way C's `%.17g` does: up to 17 significant
/// digits, trailing zeros stripped, switching to scientific notation for very
/// large/small magnitudes.
fn format_g(value: f64, precision: usize) -> String {
    let p = if precision == 0 { 1 } else { precision };

    if value == 0.0 {
        return if value.is_sign_negative() {
            "-0".to_string()
        } else {
            "0".to_string()
        };
    }

    let neg = value < 0.0;
    let abs = value.abs();

    // Determine the decimal exponent using scientific formatting.
    let sci = format!("{:.*e}", p - 1, abs);
    let epos = sci.find('e').expect("scientific notation contains 'e'");
    let exp: i32 = sci[epos + 1..].parse().expect("valid exponent");

    let body = if exp >= -4 && exp < p as i32 {
        // Fixed-point notation.
        let frac = (p as i32 - 1 - exp).max(0) as usize;
        let mut s = format!("{:.*}", frac, abs);
        if s.contains('.') {
            while s.ends_with('0') {
                s.pop();
            }
            if s.ends_with('.') {
                s.pop();
            }
        }
        s
    } else {
        // Scientific notation, with a C-style two-digit signed exponent.
        let mut mant = sci[..epos].to_string();
        if mant.contains('.') {
            while mant.ends_with('0') {
                mant.pop();
            }
            if mant.ends_with('.') {
                mant.pop();
            }
        }
        let sign = if exp < 0 { '-' } else { '+' };
        format!("{}e{}{:02}", mant, sign, exp.abs())
    };

    if neg {
        format!("-{}", body)
    } else {
        body
    }
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
            c if c <= 0x1f => {
                out.extend_from_slice(format!("\\u{:04x}", c).as_bytes());
            }
            0xe2 if i + 2 < bytes.len() && bytes[i + 1] == 0x80 && bytes[i + 2] == 0xa8 => {
                out.extend_from_slice(b"\\u2028");
                i += 2;
            }
            0xe2 if i + 2 < bytes.len() && bytes[i + 1] == 0x80 && bytes[i + 2] == 0xa9 => {
                out.extend_from_slice(b"\\u2029");
                i += 2;
            }
            c => out.push(c),
        }
        i += 1;
    }
    out.push(b'"');
}

impl Json {
    fn dump_bytes(&self, out: &mut Vec<u8>) {
        match self {
            Json::Null => out.extend_from_slice(b"null"),
            Json::Bool(b) => out.extend_from_slice(if *b { b"true" } else { b"false" }),
            Json::Number(n) => {
                if n.is_finite() {
                    out.extend_from_slice(format_g(*n, 17).as_bytes());
                } else {
                    out.extend_from_slice(b"null");
                }
            }
            Json::Str(s) => dump_string(s, out),
            Json::Array(values) => {
                out.push(b'[');
                let mut first = true;
                for v in values {
                    if !first {
                        out.extend_from_slice(b", ");
                    }
                    v.dump_bytes(out);
                    first = false;
                }
                out.push(b']');
            }
            Json::Object(values) => {
                out.push(b'{');
                let mut first = true;
                for (k, v) in values {
                    if !first {
                        out.extend_from_slice(b", ");
                    }
                    dump_string(k, out);
                    out.extend_from_slice(b": ");
                    v.dump_bytes(out);
                    first = false;
                }
                out.push(b'}');
            }
        }
    }

    /// Serialize this value to a JSON string.
    ///
    /// The output may contain raw bytes that are not valid UTF-8 if the value
    /// was parsed from input containing lone UTF-16 surrogate escapes (see
    /// [`Json::parse`]); this mirrors the byte-exact behavior of the C++
    /// library, whose strings are arbitrary byte buffers.
    pub fn dump(&self) -> String {
        let mut out = Vec::new();
        self.dump_bytes(&mut out);
        // The escaping above copies string bytes verbatim, which can reproduce
        // the WTF-8 byte sequences json11 emits for lone surrogates.
        unsafe { String::from_utf8_unchecked(out) }
    }
}

/* * * * * * * * * * * * * * * * * * * *
 * Shape checking
 */

impl Json {
    /// Check that this is an object containing, for each `(key, type)` in
    /// `types`, a field of the given type.
    ///
    /// Returns `Ok(())` on success or `Err(message)` describing the first
    /// mismatch, matching the C++ `has_shape` behavior.
    pub fn has_shape(&self, types: &[(&str, Type)]) -> Result<(), String> {
        if !self.is_object() {
            return Err(format!("expected JSON object, got {}", self.dump()));
        }
        let obj = self.object_items();
        for (key, ty) in types {
            match obj.get(*key) {
                Some(v) if v.r#type() == *ty => {}
                _ => return Err(format!("bad type for {} in {}", key, self.dump())),
            }
        }
        Ok(())
    }
}

/* * * * * * * * * * * * * * * * * * * *
 * Parsing
 */

/// Format a byte suitable for printing in an error message.
fn esc(c: u8) -> String {
    if (0x20..=0x7f).contains(&c) {
        format!("'{}' ({})", c as char, c as i32)
    } else {
        format!("({})", c as i8 as i32)
    }
}

fn in_range(x: i64, lower: i64, upper: i64) -> bool {
    x >= lower && x <= upper
}

/// Tracks all state of an in-progress parse.
struct JsonParser<'a> {
    bytes: &'a [u8],
    i: usize,
    err: Option<String>,
    strategy: JsonParse,
}

impl<'a> JsonParser<'a> {
    fn new(input: &'a str, strategy: JsonParse) -> Self {
        JsonParser {
            bytes: input.as_bytes(),
            i: 0,
            err: None,
            strategy,
        }
    }

    fn failed(&self) -> bool {
        self.err.is_some()
    }

    /// Byte at `idx`, or 0 (NUL) when out of range — this mirrors the C++ code,
    /// which relies on `std::string`'s NUL terminator for lookahead.
    fn at(&self, idx: usize) -> u8 {
        self.bytes.get(idx).copied().unwrap_or(0)
    }

    fn cur(&self) -> u8 {
        self.at(self.i)
    }

    /// Mark this parse as failed with `msg` (only the first message is kept).
    fn fail(&mut self, msg: String) {
        if self.err.is_none() {
            self.err = Some(msg);
        }
    }

    fn consume_whitespace(&mut self) {
        while matches!(self.cur(), b' ' | b'\r' | b'\n' | b'\t') {
            self.i += 1;
        }
    }

    /// Advance past a single comment. Returns whether a comment was consumed.
    fn consume_comment(&mut self) -> bool {
        let mut comment_found = false;
        if self.cur() == b'/' {
            self.i += 1;
            if self.i == self.bytes.len() {
                self.fail("unexpected end of input after start of comment".to_string());
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
                // multi-line comment
                self.i += 1;
                if self.i as isize > self.bytes.len() as isize - 2 {
                    self.fail("unexpected end of input inside multi-line comment".to_string());
                    return false;
                }
                while !(self.cur() == b'*' && self.at(self.i + 1) == b'/') {
                    self.i += 1;
                    if self.i as isize > self.bytes.len() as isize - 2 {
                        self.fail("unexpected end of input inside multi-line comment".to_string());
                        return false;
                    }
                }
                self.i += 2;
                comment_found = true;
            } else {
                self.fail("malformed comment".to_string());
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
                if self.failed() {
                    return;
                }
                self.consume_whitespace();
                if !comment_found {
                    break;
                }
            }
        }
    }

    /// Return the next non-whitespace character, or 0 on end of input / failure.
    fn get_next_token(&mut self) -> u8 {
        self.consume_garbage();
        if self.failed() {
            return 0;
        }
        if self.i == self.bytes.len() {
            self.fail("unexpected end of input".to_string());
            return 0;
        }
        let c = self.cur();
        self.i += 1;
        c
    }

    /// Encode `pt` as UTF-8 and append it to `out`.
    fn encode_utf8(pt: i64, out: &mut Vec<u8>) {
        if pt < 0 {
            return;
        }
        if pt < 0x80 {
            out.push(pt as u8);
        } else if pt < 0x800 {
            out.push(((pt >> 6) | 0xC0) as u8);
            out.push(((pt & 0x3F) | 0x80) as u8);
        } else if pt < 0x10000 {
            out.push(((pt >> 12) | 0xE0) as u8);
            out.push((((pt >> 6) & 0x3F) | 0x80) as u8);
            out.push(((pt & 0x3F) | 0x80) as u8);
        } else {
            out.push(((pt >> 18) | 0xF0) as u8);
            out.push((((pt >> 12) & 0x3F) | 0x80) as u8);
            out.push((((pt >> 6) & 0x3F) | 0x80) as u8);
            out.push(((pt & 0x3F) | 0x80) as u8);
        }
    }

    /// Parse a string, starting at the current position (just past the opening quote).
    fn parse_string(&mut self) -> String {
        let mut out: Vec<u8> = Vec::new();
        let mut last_escaped_codepoint: i64 = -1;
        loop {
            if self.i == self.bytes.len() {
                self.fail("unexpected end of input in string".to_string());
                return String::new();
            }

            let mut ch = self.bytes[self.i];
            self.i += 1;

            if ch == b'"' {
                Self::encode_utf8(last_escaped_codepoint, &mut out);
                // Strings may hold lone-surrogate (WTF-8) byte sequences; keep
                // the bytes exactly as json11 produces them.
                return unsafe { String::from_utf8_unchecked(out) };
            }

            if in_range(ch as i64, 0, 0x1f) {
                self.fail(format!("unescaped {} in string", esc(ch)));
                return String::new();
            }

            // The usual case: non-escaped characters.
            if ch != b'\\' {
                Self::encode_utf8(last_escaped_codepoint, &mut out);
                last_escaped_codepoint = -1;
                out.push(ch);
                continue;
            }

            // Handle escapes.
            if self.i == self.bytes.len() {
                self.fail("unexpected end of input in string".to_string());
                return String::new();
            }

            ch = self.bytes[self.i];
            self.i += 1;

            if ch == b'u' {
                // Extract 4-hex-digit escape sequence.
                let end = self.i + 4;
                if end > self.bytes.len() {
                    let esc_str = String::from_utf8_lossy(&self.bytes[self.i..]).into_owned();
                    self.fail(format!("bad \\u escape: {}", esc_str));
                    return String::new();
                }
                let esc_bytes = &self.bytes[self.i..end];
                for &b in esc_bytes {
                    let is_hex = in_range(b as i64, b'a' as i64, b'f' as i64)
                        || in_range(b as i64, b'A' as i64, b'F' as i64)
                        || in_range(b as i64, b'0' as i64, b'9' as i64);
                    if !is_hex {
                        let esc_str = String::from_utf8_lossy(esc_bytes).into_owned();
                        self.fail(format!("bad \\u escape: {}", esc_str));
                        return String::new();
                    }
                }

                let esc_str = std::str::from_utf8(esc_bytes).expect("hex digits are ASCII");
                let codepoint = i64::from_str_radix(esc_str, 16).expect("validated hex");

                // Reassemble UTF-16 surrogate pairs into one astral-plane char.
                if in_range(last_escaped_codepoint, 0xD800, 0xDBFF)
                    && in_range(codepoint, 0xDC00, 0xDFFF)
                {
                    Self::encode_utf8(
                        (((last_escaped_codepoint - 0xD800) << 10) | (codepoint - 0xDC00))
                            + 0x10000,
                        &mut out,
                    );
                    last_escaped_codepoint = -1;
                } else {
                    Self::encode_utf8(last_escaped_codepoint, &mut out);
                    last_escaped_codepoint = codepoint;
                }

                self.i += 4;
                continue;
            }

            Self::encode_utf8(last_escaped_codepoint, &mut out);
            last_escaped_codepoint = -1;

            match ch {
                b'b' => out.push(0x08),
                b'f' => out.push(0x0c),
                b'n' => out.push(b'\n'),
                b'r' => out.push(b'\r'),
                b't' => out.push(b'\t'),
                b'"' | b'\\' | b'/' => out.push(ch),
                _ => {
                    self.fail(format!("invalid escape character {}", esc(ch)));
                    return String::new();
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
            if in_range(self.cur() as i64, b'0' as i64, b'9' as i64) {
                self.fail("leading 0s not permitted in numbers".to_string());
                return Json::Null;
            }
        } else if in_range(self.cur() as i64, b'1' as i64, b'9' as i64) {
            self.i += 1;
            while in_range(self.cur() as i64, b'0' as i64, b'9' as i64) {
                self.i += 1;
            }
        } else {
            self.fail(format!("invalid {} in number", esc(self.cur())));
            return Json::Null;
        }

        // Early-out for plain integers (json11 stores these losslessly; we keep
        // them as f64, which is exact for these magnitudes).
        if self.cur() != b'.'
            && self.cur() != b'e'
            && self.cur() != b'E'
            && (self.i - start_pos) <= 9
        {
            let s = std::str::from_utf8(&self.bytes[start_pos..self.i]).expect("ascii number");
            return Json::Number(s.parse::<f64>().expect("valid integer"));
        }

        // Decimal part.
        if self.cur() == b'.' {
            self.i += 1;
            if !in_range(self.cur() as i64, b'0' as i64, b'9' as i64) {
                self.fail("at least one digit required in fractional part".to_string());
                return Json::Null;
            }
            while in_range(self.cur() as i64, b'0' as i64, b'9' as i64) {
                self.i += 1;
            }
        }

        // Exponent part.
        if self.cur() == b'e' || self.cur() == b'E' {
            self.i += 1;
            if self.cur() == b'+' || self.cur() == b'-' {
                self.i += 1;
            }
            if !in_range(self.cur() as i64, b'0' as i64, b'9' as i64) {
                self.fail("at least one digit required in exponent".to_string());
                return Json::Null;
            }
            while in_range(self.cur() as i64, b'0' as i64, b'9' as i64) {
                self.i += 1;
            }
        }

        let s = std::str::from_utf8(&self.bytes[start_pos..self.i]).expect("ascii number");
        Json::Number(s.parse::<f64>().expect("valid number"))
    }

    /// Expect `expected` to start at the character that was just read.
    fn expect(&mut self, expected: &str, res: Json) -> Json {
        debug_assert!(self.i != 0);
        self.i -= 1;
        let end = self.i + expected.len();
        if end <= self.bytes.len() && &self.bytes[self.i..end] == expected.as_bytes() {
            self.i += expected.len();
            res
        } else {
            let got_end = end.min(self.bytes.len());
            let got = String::from_utf8_lossy(&self.bytes[self.i..got_end]).into_owned();
            self.fail(format!("parse error: expected {}, got {}", expected, got));
            Json::Null
        }
    }

    /// Parse a JSON value.
    fn parse_json(&mut self, depth: i32) -> Json {
        if depth > MAX_DEPTH {
            self.fail("exceeded maximum nesting depth".to_string());
            return Json::Null;
        }

        let ch = self.get_next_token();
        if self.failed() {
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
            return Json::Str(self.parse_string());
        }

        if ch == b'{' {
            let mut data: BTreeMap<String, Json> = BTreeMap::new();
            let mut ch = self.get_next_token();
            if ch == b'}' {
                return Json::Object(data);
            }
            loop {
                if ch != b'"' {
                    self.fail(format!("expected '\"' in object, got {}", esc(ch)));
                    return Json::Null;
                }

                let key = self.parse_string();
                if self.failed() {
                    return Json::Null;
                }

                ch = self.get_next_token();
                if ch != b':' {
                    self.fail(format!("expected ':' in object, got {}", esc(ch)));
                    return Json::Null;
                }

                let value = self.parse_json(depth + 1);
                if self.failed() {
                    return Json::Null;
                }
                data.insert(key, value);

                ch = self.get_next_token();
                if ch == b'}' {
                    break;
                }
                if ch != b',' {
                    self.fail(format!("expected ',' in object, got {}", esc(ch)));
                    return Json::Null;
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
                self.i -= 1;
                let value = self.parse_json(depth + 1);
                if self.failed() {
                    return Json::Null;
                }
                data.push(value);

                ch = self.get_next_token();
                if ch == b']' {
                    break;
                }
                if ch != b',' {
                    self.fail(format!("expected ',' in list, got {}", esc(ch)));
                    return Json::Null;
                }

                ch = self.get_next_token();
                let _ = ch;
            }
            return Json::Array(data);
        }

        self.fail(format!("expected value, got {}", esc(ch)));
        Json::Null
    }
}

impl Json {
    /// Parse `input` as a single JSON value.
    ///
    /// On success returns the parsed [`Json`]; on failure returns a descriptive
    /// error message. Note that, following the original library, lone UTF-16
    /// surrogate `\u` escapes are decoded to WTF-8 byte sequences, so a
    /// resulting [`Json::Str`] may contain bytes that are not valid UTF-8.
    pub fn parse(input: &str, strategy: JsonParse) -> Result<Json, String> {
        let mut parser = JsonParser::new(input, strategy);
        let result = parser.parse_json(0);

        // Check for trailing garbage.
        parser.consume_garbage();
        if let Some(err) = parser.err {
            return Err(err);
        }
        if parser.i != parser.bytes.len() {
            return Err(format!("unexpected trailing {}", esc(parser.at(parser.i))));
        }
        Ok(result)
    }

    /// Parse a sequence of JSON values, concatenated or separated by whitespace.
    ///
    /// Returns the parsed values together with the stop position (the byte
    /// offset up to which parsing succeeded). Mirroring the C++ `parse_multi`,
    /// the returned vector may include a trailing `Json::Null` produced by a
    /// failed final parse.
    pub fn parse_multi(input: &str, strategy: JsonParse) -> (Vec<Json>, usize) {
        let mut parser = JsonParser::new(input, strategy);
        let mut stop_pos = 0;
        let mut out: Vec<Json> = Vec::new();
        while parser.i != parser.bytes.len() && !parser.failed() {
            out.push(parser.parse_json(0));
            if parser.failed() {
                break;
            }
            // Check for another value.
            parser.consume_garbage();
            if parser.failed() {
                break;
            }
            stop_pos = parser.i;
        }
        (out, stop_pos)
    }
}

#[cfg(test)]
mod tests;
