//! The command tree as the `commands` packet carries it.

use std::collections::VecDeque;

/// What a node is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeKind {
    Root,
    /// A fixed word.
    Literal,
    /// A value of some type.
    Argument,
}

/// How strings are read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StringKind {
    /// A single word (letters, digits, `_-.+`).
    Word,
    /// A word, or a quoted phrase.
    Quotable,
    /// The rest of the line.
    Greedy,
}

/// The type of an argument, with its properties. Names are the ones in the
/// `command_argument_type` registry.
#[derive(Clone, Debug, PartialEq)]
pub enum Parser {
    Bool,
    Float {
        min: Option<f32>,
        max: Option<f32>,
    },
    Double {
        min: Option<f64>,
        max: Option<f64>,
    },
    Integer {
        min: Option<i32>,
        max: Option<i32>,
    },
    Long {
        min: Option<i64>,
        max: Option<i64>,
    },
    String(StringKind),
    /// An entity selector or name; `single`: only one entity, `players`: only players.
    Entity {
        single: bool,
        players: bool,
    },
    /// A score holder; `multiple`: several allowed.
    ScoreHolder {
        multiple: bool,
    },
    /// A duration (`1d`, `5s`, `20t`), at least `min` ticks.
    Time {
        min: i32,
    },
    /// A registry entry given by name (the registry's identifier).
    Resource {
        kind: ResourceKind,
        registry: String,
    },
    /// Any other type: its registry id (no properties).
    Simple(i32),
}

/// Which resource-style parser.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceKind {
    OrTag,
    OrTagKey,
    Resource,
    ResourceKey,
    Selector,
}

/// Registry ids of the argument types with properties (the rest are [`Parser::Simple`]).
mod id {
    pub const BOOL: i32 = 0;
    pub const FLOAT: i32 = 1;
    pub const DOUBLE: i32 = 2;
    pub const INTEGER: i32 = 3;
    pub const LONG: i32 = 4;
    pub const STRING: i32 = 5;
    pub const ENTITY: i32 = 6;
    pub const SCORE_HOLDER: i32 = 31;
    pub const TIME: i32 = 43;
    pub const RESOURCE_OR_TAG: i32 = 44;
    pub const RESOURCE_OR_TAG_KEY: i32 = 45;
    pub const RESOURCE: i32 = 46;
    pub const RESOURCE_KEY: i32 = 47;
    pub const RESOURCE_SELECTOR: i32 = 48;
}

impl ResourceKind {
    fn id(self) -> i32 {
        match self {
            ResourceKind::OrTag => id::RESOURCE_OR_TAG,
            ResourceKind::OrTagKey => id::RESOURCE_OR_TAG_KEY,
            ResourceKind::Resource => id::RESOURCE,
            ResourceKind::ResourceKey => id::RESOURCE_KEY,
            ResourceKind::Selector => id::RESOURCE_SELECTOR,
        }
    }
}

/// One node of the tree.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub kind: NodeKind,
    /// The literal's word or the argument's name (empty for the root).
    pub name: String,
    /// A command can end here.
    pub executable: bool,
    /// Needs a permission (the client shows such commands differently when it lacks it).
    pub restricted: bool,
    pub children: Vec<usize>,
    /// Parsing continues at this node (for aliases and `execute`).
    pub redirect: Option<usize>,
    pub parser: Option<Parser>,
    /// Name of the custom suggestion provider, when the argument has one.
    pub suggestions: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Tree {
    pub nodes: Vec<Node>,
    pub root: usize,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TreeError {
    #[error("the command tree ends early")]
    Truncated,
    #[error("a name is not valid UTF-8")]
    BadString,
    #[error("a varint is too long")]
    BadVarInt,
    #[error("node flags {0:#04x} are not valid")]
    BadFlags(u8),
    #[error("a node refers to node {0}, which does not exist")]
    BadIndex(usize),
    #[error("{0} bytes remain after the tree")]
    Trailing(usize),
}

struct Reader<'a>(&'a [u8]);

impl Reader<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8], TreeError> {
        if self.0.len() < n {
            return Err(TreeError::Truncated);
        }
        let (a, b) = self.0.split_at(n);
        self.0 = b;
        Ok(a)
    }

    fn u8(&mut self) -> Result<u8, TreeError> {
        Ok(self.take(1)?[0])
    }

    fn varint(&mut self) -> Result<i32, TreeError> {
        let mut v: u32 = 0;
        for i in 0..5 {
            let b = self.u8()?;
            v |= u32::from(b & 0x7F) << (7 * i);
            if b & 0x80 == 0 {
                return Ok(v as i32);
            }
        }
        Err(TreeError::BadVarInt)
    }

    fn index(&mut self) -> Result<usize, TreeError> {
        usize::try_from(self.varint()?).map_err(|_| TreeError::BadVarInt)
    }

    fn string(&mut self) -> Result<String, TreeError> {
        let n = self.index()?;
        std::str::from_utf8(self.take(n)?).map(str::to_string).map_err(|_| TreeError::BadString)
    }
}

fn put_varint(out: &mut Vec<u8>, v: i32) {
    let mut v = v as u32;
    loop {
        let b = (v & 0x7F) as u8;
        v >>= 7;
        if v == 0 {
            out.push(b);
            return;
        }
        out.push(b | 0x80);
    }
}

fn put_string(out: &mut Vec<u8>, s: &str) {
    put_varint(out, s.len() as i32);
    out.extend_from_slice(s.as_bytes());
}

impl Parser {
    fn read(r: &mut Reader<'_>) -> Result<Parser, TreeError> {
        let id = r.varint()?;
        Ok(match id {
            id::BOOL => Parser::Bool,
            id::FLOAT => {
                let flags = r.u8()?;
                let mut f = |bit: u8| -> Result<Option<f32>, TreeError> {
                    Ok(if flags & bit != 0 {
                        Some(f32::from_be_bytes(r.take(4)?.try_into().expect("4")))
                    } else {
                        None
                    })
                };
                let (min, max) = (f(1)?, f(2)?);
                Parser::Float { min, max }
            }
            id::DOUBLE => {
                let flags = r.u8()?;
                let mut f = |bit: u8| -> Result<Option<f64>, TreeError> {
                    Ok(if flags & bit != 0 {
                        Some(f64::from_be_bytes(r.take(8)?.try_into().expect("8")))
                    } else {
                        None
                    })
                };
                let (min, max) = (f(1)?, f(2)?);
                Parser::Double { min, max }
            }
            id::INTEGER => {
                let flags = r.u8()?;
                let mut f = |bit: u8| -> Result<Option<i32>, TreeError> {
                    Ok(if flags & bit != 0 {
                        Some(i32::from_be_bytes(r.take(4)?.try_into().expect("4")))
                    } else {
                        None
                    })
                };
                let (min, max) = (f(1)?, f(2)?);
                Parser::Integer { min, max }
            }
            id::LONG => {
                let flags = r.u8()?;
                let mut f = |bit: u8| -> Result<Option<i64>, TreeError> {
                    Ok(if flags & bit != 0 {
                        Some(i64::from_be_bytes(r.take(8)?.try_into().expect("8")))
                    } else {
                        None
                    })
                };
                let (min, max) = (f(1)?, f(2)?);
                Parser::Long { min, max }
            }
            id::STRING => Parser::String(match r.varint()? {
                0 => StringKind::Word,
                1 => StringKind::Quotable,
                _ => StringKind::Greedy,
            }),
            id::ENTITY => {
                let flags = r.u8()?;
                Parser::Entity { single: flags & 1 != 0, players: flags & 2 != 0 }
            }
            id::SCORE_HOLDER => Parser::ScoreHolder { multiple: r.u8()? & 1 != 0 },
            id::TIME => Parser::Time { min: i32::from_be_bytes(r.take(4)?.try_into().expect("4")) },
            id::RESOURCE_OR_TAG
            | id::RESOURCE_OR_TAG_KEY
            | id::RESOURCE
            | id::RESOURCE_KEY
            | id::RESOURCE_SELECTOR => {
                let kind = match id {
                    id::RESOURCE_OR_TAG => ResourceKind::OrTag,
                    id::RESOURCE_OR_TAG_KEY => ResourceKind::OrTagKey,
                    id::RESOURCE => ResourceKind::Resource,
                    id::RESOURCE_KEY => ResourceKind::ResourceKey,
                    _ => ResourceKind::Selector,
                };
                Parser::Resource { kind, registry: r.string()? }
            }
            other => Parser::Simple(other),
        })
    }

    fn write(&self, out: &mut Vec<u8>) {
        fn flags(out: &mut Vec<u8>, min: bool, max: bool) {
            out.push(u8::from(min) | (u8::from(max) << 1));
        }
        match self {
            Parser::Bool => put_varint(out, id::BOOL),
            Parser::Float { min, max } => {
                put_varint(out, id::FLOAT);
                flags(out, min.is_some(), max.is_some());
                for v in min.iter().chain(max.iter()) {
                    out.extend_from_slice(&v.to_be_bytes());
                }
            }
            Parser::Double { min, max } => {
                put_varint(out, id::DOUBLE);
                flags(out, min.is_some(), max.is_some());
                for v in min.iter().chain(max.iter()) {
                    out.extend_from_slice(&v.to_be_bytes());
                }
            }
            Parser::Integer { min, max } => {
                put_varint(out, id::INTEGER);
                flags(out, min.is_some(), max.is_some());
                for v in min.iter().chain(max.iter()) {
                    out.extend_from_slice(&v.to_be_bytes());
                }
            }
            Parser::Long { min, max } => {
                put_varint(out, id::LONG);
                flags(out, min.is_some(), max.is_some());
                for v in min.iter().chain(max.iter()) {
                    out.extend_from_slice(&v.to_be_bytes());
                }
            }
            Parser::String(kind) => {
                put_varint(out, id::STRING);
                put_varint(
                    out,
                    match kind {
                        StringKind::Word => 0,
                        StringKind::Quotable => 1,
                        StringKind::Greedy => 2,
                    },
                );
            }
            Parser::Entity { single, players } => {
                put_varint(out, id::ENTITY);
                out.push(u8::from(*single) | (u8::from(*players) << 1));
            }
            Parser::ScoreHolder { multiple } => {
                put_varint(out, id::SCORE_HOLDER);
                out.push(u8::from(*multiple));
            }
            Parser::Time { min } => {
                put_varint(out, id::TIME);
                out.extend_from_slice(&min.to_be_bytes());
            }
            Parser::Resource { kind, registry } => {
                put_varint(out, kind.id());
                put_string(out, registry);
            }
            Parser::Simple(id) => put_varint(out, *id),
        }
    }
}

impl Tree {
    /// Decodes the body of a `commands` packet.
    pub fn decode(bytes: &[u8]) -> Result<Tree, TreeError> {
        let mut r = Reader(bytes);
        let count = r.index()?;
        let mut nodes = Vec::with_capacity(count.min(1 << 16));
        for _ in 0..count {
            let flags = r.u8()?;
            let kind = match flags & 3 {
                0 => NodeKind::Root,
                1 => NodeKind::Literal,
                2 => NodeKind::Argument,
                _ => return Err(TreeError::BadFlags(flags)),
            };
            if flags & !0x3F != 0 {
                return Err(TreeError::BadFlags(flags));
            }
            let executable = flags & 0x04 != 0;
            let has_redirect = flags & 0x08 != 0;
            let has_suggestions = flags & 0x10 != 0;
            let restricted = flags & 0x20 != 0;
            let n = r.index()?;
            let mut children = Vec::with_capacity(n.min(4096));
            for _ in 0..n {
                children.push(r.index()?);
            }
            let redirect = if has_redirect { Some(r.index()?) } else { None };
            let name = if kind == NodeKind::Root { String::new() } else { r.string()? };
            let (parser, suggestions) = if kind == NodeKind::Argument {
                let parser = Parser::read(&mut r)?;
                (Some(parser), if has_suggestions { Some(r.string()?) } else { None })
            } else {
                (None, None)
            };
            nodes.push(Node {
                kind,
                name,
                executable,
                restricted,
                children,
                redirect,
                parser,
                suggestions,
            });
        }
        let root = r.index()?;
        if !r.0.is_empty() {
            return Err(TreeError::Trailing(r.0.len()));
        }
        let tree = Tree { nodes, root };
        for n in &tree.nodes {
            for &c in n.children.iter().chain(n.redirect.iter()) {
                if c >= tree.nodes.len() {
                    return Err(TreeError::BadIndex(c));
                }
            }
        }
        if root >= tree.nodes.len() {
            return Err(TreeError::BadIndex(root));
        }
        Ok(tree)
    }

    /// The body of the `commands` packet for this tree.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        put_varint(&mut out, self.nodes.len() as i32);
        for n in &self.nodes {
            let mut flags = match n.kind {
                NodeKind::Root => 0,
                NodeKind::Literal => 1,
                NodeKind::Argument => 2,
            };
            if n.executable {
                flags |= 0x04;
            }
            if n.redirect.is_some() {
                flags |= 0x08;
            }
            if n.suggestions.is_some() {
                flags |= 0x10;
            }
            if n.restricted {
                flags |= 0x20;
            }
            out.push(flags);
            put_varint(&mut out, n.children.len() as i32);
            for &c in &n.children {
                put_varint(&mut out, c as i32);
            }
            if let Some(r) = n.redirect {
                put_varint(&mut out, r as i32);
            }
            if n.kind != NodeKind::Root {
                put_string(&mut out, &n.name);
            }
            if let Some(p) = &n.parser {
                p.write(&mut out);
                if let Some(s) = &n.suggestions {
                    put_string(&mut out, s);
                }
            }
        }
        put_varint(&mut out, self.root as i32);
        out
    }

    /// The tree a player of permission level `level` (0 to 4) is sent.
    pub fn for_level(level: usize) -> Tree {
        Tree::decode(ferrite_data::generated::commands::COMMAND_TREES[level.min(4)])
            .expect("the generated command trees decode")
    }

    /// Breadth-first order is how the packet numbers nodes; this lists the nodes under `root`
    /// in that order.
    pub fn breadth_first(&self) -> Vec<usize> {
        let mut seen = vec![false; self.nodes.len()];
        let mut order = Vec::new();
        let mut queue = VecDeque::from([self.root]);
        seen[self.root] = true;
        while let Some(i) = queue.pop_front() {
            order.push(i);
            for &c in &self.nodes[i].children {
                if !seen[c] {
                    seen[c] = true;
                    queue.push_back(c);
                }
            }
        }
        order
    }

    /// The child of `node` that is the literal `word`.
    pub fn literal(&self, node: usize, word: &str) -> Option<usize> {
        self.nodes[node]
            .children
            .iter()
            .copied()
            .find(|&c| self.nodes[c].kind == NodeKind::Literal && self.nodes[c].name == word)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_vanilla_tree_decodes_and_encodes_to_the_same_bytes() {
        for level in 0..5 {
            let bytes = ferrite_data::generated::commands::COMMAND_TREES[level];
            let tree = Tree::decode(bytes).unwrap_or_else(|e| panic!("level {level}: {e}"));
            assert_eq!(tree.encode(), bytes, "level {level} does not round-trip");
        }
    }

    #[test]
    fn higher_levels_have_more_commands() {
        let root = |level: usize| {
            let t = Tree::for_level(level);
            t.nodes[t.root].children.len()
        };
        assert!(root(0) < root(2), "{} vs {}", root(0), root(2));
        assert!(root(2) <= root(4));
        let owner = Tree::for_level(4);
        for word in ["gamemode", "tp", "time", "give", "setblock", "execute", "stop"] {
            assert!(owner.literal(owner.root, word).is_some(), "/{word}");
        }
        // Everyone can use /help and /list but not /gamemode.
        let player = Tree::for_level(0);
        assert!(player.literal(player.root, "help").is_some());
        assert!(player.literal(player.root, "gamemode").is_none());
    }
}

#[cfg(test)]
mod survey {
    use super::*;

    #[test]
    #[ignore]
    fn print_survey() {
        let t = Tree::for_level(4);
        let mut parsers = std::collections::BTreeMap::new();
        let mut redirects = 0;
        for n in &t.nodes {
            if let Some(p) = &n.parser {
                let key = match p {
                    Parser::Simple(id) => format!("simple {id}"),
                    other => format!("{:?}", std::mem::discriminant(other)),
                };
                let name = ferrite_data::generated::registries::command_argument_type::names
                    .get(match p {
                        Parser::Simple(id) => *id as usize,
                        Parser::Bool => 0,
                        Parser::Float { .. } => 1,
                        Parser::Double { .. } => 2,
                        Parser::Integer { .. } => 3,
                        Parser::Long { .. } => 4,
                        Parser::String(_) => 5,
                        Parser::Entity { .. } => 6,
                        Parser::ScoreHolder { .. } => 31,
                        Parser::Time { .. } => 43,
                        Parser::Resource { kind, .. } => match kind {
                            ResourceKind::OrTag => 44,
                            ResourceKind::OrTagKey => 45,
                            ResourceKind::Resource => 46,
                            ResourceKind::ResourceKey => 47,
                            ResourceKind::Selector => 48,
                        },
                    })
                    .copied()
                    .unwrap_or("?");
                let _ = key;
                *parsers.entry(name).or_insert(0) += 1;
            }
            redirects += usize::from(n.redirect.is_some());
        }
        eprintln!("{} nodes, {} redirects", t.nodes.len(), redirects);
        eprintln!("{parsers:#?}");
        let root = &t.nodes[t.root];
        let names: Vec<&str> = root.children.iter().map(|&c| t.nodes[c].name.as_str()).collect();
        eprintln!("{} commands: {}", names.len(), names.join(" "));
    }
}
