use indexmap::IndexMap;

use crate::id;

/// A single NBT value.
#[derive(Clone, Debug, PartialEq)]
pub enum Tag {
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(Vec<i8>),
    String(String),
    List(List),
    Compound(Compound),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
}

/// A list of unnamed tags that all share one type, which the enum makes unrepresentable to get
/// wrong.
#[derive(Clone, Debug, PartialEq, Default)]
pub enum List {
    /// The only list with no element type (`TAG_End`); also what zero-length lists decode to.
    #[default]
    Empty,
    Byte(Vec<i8>),
    Short(Vec<i16>),
    Int(Vec<i32>),
    Long(Vec<i64>),
    Float(Vec<f32>),
    Double(Vec<f64>),
    ByteArray(Vec<Vec<i8>>),
    String(Vec<String>),
    List(Vec<List>),
    Compound(Vec<Compound>),
    IntArray(Vec<Vec<i32>>),
    LongArray(Vec<Vec<i64>>),
    /// Elements of different types, which text (SNBT) allows. On the wire and on disk vanilla
    /// stores such a list as a list of compounds, wrapping each non-compound element as
    /// `{"": element}`, and the writer does the same.
    Mixed(Vec<Tag>),
}

/// Named tags, in insertion order. Vanilla treats compounds as unordered; keeping order makes our
/// output deterministic.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Compound(pub IndexMap<String, Tag>);

impl Tag {
    /// The wire type id of this tag.
    pub fn type_id(&self) -> u8 {
        match self {
            Tag::Byte(_) => id::BYTE,
            Tag::Short(_) => id::SHORT,
            Tag::Int(_) => id::INT,
            Tag::Long(_) => id::LONG,
            Tag::Float(_) => id::FLOAT,
            Tag::Double(_) => id::DOUBLE,
            Tag::ByteArray(_) => id::BYTE_ARRAY,
            Tag::String(_) => id::STRING,
            Tag::List(_) => id::LIST,
            Tag::Compound(_) => id::COMPOUND,
            Tag::IntArray(_) => id::INT_ARRAY,
            Tag::LongArray(_) => id::LONG_ARRAY,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Tag::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_compound(&self) -> Option<&Compound> {
        match self {
            Tag::Compound(c) => Some(c),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match *self {
            Tag::Byte(v) => Some(v.into()),
            Tag::Short(v) => Some(v.into()),
            Tag::Int(v) => Some(v.into()),
            Tag::Long(v) => Some(v),
            _ => None,
        }
    }
}

impl List {
    /// The element type id (`TAG_End` for [`List::Empty`]).
    pub fn element_id(&self) -> u8 {
        match self {
            List::Empty => id::END,
            List::Byte(_) => id::BYTE,
            List::Short(_) => id::SHORT,
            List::Int(_) => id::INT,
            List::Long(_) => id::LONG,
            List::Float(_) => id::FLOAT,
            List::Double(_) => id::DOUBLE,
            List::ByteArray(_) => id::BYTE_ARRAY,
            List::String(_) => id::STRING,
            List::List(_) => id::LIST,
            List::Compound(_) => id::COMPOUND,
            List::IntArray(_) => id::INT_ARRAY,
            List::LongArray(_) => id::LONG_ARRAY,
            List::Mixed(_) => id::COMPOUND,
        }
    }

    pub fn len(&self) -> usize {
        match self {
            List::Empty => 0,
            List::Byte(v) => v.len(),
            List::Short(v) => v.len(),
            List::Int(v) => v.len(),
            List::Long(v) => v.len(),
            List::Float(v) => v.len(),
            List::Double(v) => v.len(),
            List::ByteArray(v) => v.len(),
            List::String(v) => v.len(),
            List::List(v) => v.len(),
            List::Compound(v) => v.len(),
            List::IntArray(v) => v.len(),
            List::LongArray(v) => v.len(),
            List::Mixed(v) => v.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Compound {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, key: &str) -> Option<&Tag> {
        self.0.get(key)
    }

    /// Inserts `value` under `key` and returns `self` for chaining.
    pub fn with(mut self, key: impl Into<String>, value: Tag) -> Self {
        self.0.insert(key.into(), value);
        self
    }

    pub fn insert(&mut self, key: impl Into<String>, value: Tag) -> Option<Tag> {
        self.0.insert(key.into(), value)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl From<Compound> for Tag {
    fn from(c: Compound) -> Self {
        Tag::Compound(c)
    }
}
impl From<&str> for Tag {
    fn from(s: &str) -> Self {
        Tag::String(s.to_owned())
    }
}
