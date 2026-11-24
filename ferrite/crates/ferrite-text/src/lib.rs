//! Text components, and how the game writes them as NBT.
//!
//! A component is some content (literal text, a translation key with arguments, a selector, a
//! score, a key binding or NBT data), a style, and siblings that inherit the style. On the wire
//! plain unstyled text is a bare string tag; everything else is a compound. Lists of mixed
//! element types cannot exist in NBT, so the game wraps each non-compound element of such a list
//! in a compound under the empty key (`{"": 1}`); this crate does the same.

#![forbid(unsafe_code)]

use ferrite_nbt::{Compound, List, Tag};

/// One of the sixteen named colors, or an RGB value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
    Black,
    DarkBlue,
    DarkGreen,
    DarkAqua,
    DarkRed,
    DarkPurple,
    Gold,
    Gray,
    DarkGray,
    Blue,
    Green,
    Aqua,
    Red,
    LightPurple,
    Yellow,
    White,
    Rgb(u32),
}

impl Color {
    /// The name or `#rrggbb` the style codec writes.
    pub fn name(self) -> String {
        match self {
            Color::Black => "black".into(),
            Color::DarkBlue => "dark_blue".into(),
            Color::DarkGreen => "dark_green".into(),
            Color::DarkAqua => "dark_aqua".into(),
            Color::DarkRed => "dark_red".into(),
            Color::DarkPurple => "dark_purple".into(),
            Color::Gold => "gold".into(),
            Color::Gray => "gray".into(),
            Color::DarkGray => "dark_gray".into(),
            Color::Blue => "blue".into(),
            Color::Green => "green".into(),
            Color::Aqua => "aqua".into(),
            Color::Red => "red".into(),
            Color::LightPurple => "light_purple".into(),
            Color::Yellow => "yellow".into(),
            Color::White => "white".into(),
            Color::Rgb(v) => format!("#{:06x}", v & 0xFF_FFFF),
        }
    }
}

/// What happens when the text is clicked.
#[derive(Clone, Debug, PartialEq)]
pub enum ClickEvent {
    OpenUrl(String),
    RunCommand(String),
    SuggestCommand(String),
    ChangePage(i32),
    CopyToClipboard(String),
}

/// What shows when the text is hovered.
#[derive(Clone, Debug, PartialEq)]
pub enum HoverEvent {
    ShowText(Box<Component>),
    ShowItem { id: String, count: i32 },
    ShowEntity { id: String, uuid: [i32; 4], name: Option<Box<Component>> },
}

/// How text looks and behaves; unset fields inherit.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Style {
    pub color: Option<Color>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underlined: Option<bool>,
    pub strikethrough: Option<bool>,
    pub obfuscated: Option<bool>,
    pub click: Option<ClickEvent>,
    pub hover: Option<HoverEvent>,
    pub insertion: Option<String>,
    pub font: Option<String>,
}

impl Style {
    fn is_empty(&self) -> bool {
        *self == Style::default()
    }
}

/// An argument of a translation.
#[derive(Clone, Debug, PartialEq)]
pub enum Arg {
    Text(Box<Component>),
    String(String),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    Bool(bool),
}

impl From<Component> for Arg {
    fn from(c: Component) -> Arg {
        Arg::Text(Box::new(c))
    }
}
impl From<&str> for Arg {
    fn from(s: &str) -> Arg {
        Arg::String(s.to_string())
    }
}
impl From<String> for Arg {
    fn from(s: String) -> Arg {
        Arg::String(s)
    }
}
impl From<i32> for Arg {
    fn from(v: i32) -> Arg {
        Arg::Int(v)
    }
}
impl From<i64> for Arg {
    fn from(v: i64) -> Arg {
        Arg::Long(v)
    }
}
impl From<f32> for Arg {
    fn from(v: f32) -> Arg {
        Arg::Float(v)
    }
}
impl From<f64> for Arg {
    fn from(v: f64) -> Arg {
        Arg::Double(v)
    }
}
impl From<bool> for Arg {
    fn from(v: bool) -> Arg {
        Arg::Bool(v)
    }
}

/// What a component says.
#[derive(Clone, Debug, PartialEq)]
pub enum Content {
    Literal(String),
    Translatable { key: String, fallback: Option<String>, args: Vec<Arg> },
    Selector { pattern: String, separator: Option<Box<Component>> },
    Score { name: String, objective: String },
    Keybind(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Component {
    pub content: Content,
    pub style: Style,
    pub extra: Vec<Component>,
}

impl Component {
    pub fn new(content: Content) -> Component {
        Component { content, style: Style::default(), extra: Vec::new() }
    }

    /// Text as written.
    pub fn literal(s: impl Into<String>) -> Component {
        Component::new(Content::Literal(s.into()))
    }

    /// No text; the usual start of a message built from parts.
    pub fn empty() -> Component {
        Component::literal("")
    }

    pub fn translatable(key: &str, args: Vec<Arg>) -> Component {
        Component::new(Content::Translatable { key: key.to_string(), fallback: None, args })
    }

    pub fn keybind(key: &str) -> Component {
        Component::new(Content::Keybind(key.to_string()))
    }

    pub fn selector(pattern: &str) -> Component {
        Component::new(Content::Selector { pattern: pattern.to_string(), separator: None })
    }

    pub fn score(name: &str, objective: &str) -> Component {
        Component::new(Content::Score { name: name.to_string(), objective: objective.to_string() })
    }

    pub fn color(mut self, c: Color) -> Component {
        self.style.color = Some(c);
        self
    }

    pub fn bold(mut self, on: bool) -> Component {
        self.style.bold = Some(on);
        self
    }

    pub fn italic(mut self, on: bool) -> Component {
        self.style.italic = Some(on);
        self
    }

    pub fn underlined(mut self, on: bool) -> Component {
        self.style.underlined = Some(on);
        self
    }

    pub fn click(mut self, e: ClickEvent) -> Component {
        self.style.click = Some(e);
        self
    }

    pub fn hover(mut self, e: HoverEvent) -> Component {
        self.style.hover = Some(e);
        self
    }

    pub fn insertion(mut self, s: &str) -> Component {
        self.style.insertion = Some(s.to_string());
        self
    }

    /// Adds a sibling that inherits this component's style.
    pub fn append(mut self, c: Component) -> Component {
        self.extra.push(c);
        self
    }

    /// The component as the tag the game writes: bare text is a string tag.
    pub fn to_tag(&self) -> Tag {
        if let Content::Literal(s) = &self.content
            && self.style.is_empty()
            && self.extra.is_empty()
        {
            return Tag::String(s.clone());
        }
        let mut c = Compound::default();
        match &self.content {
            Content::Literal(s) => {
                c.0.insert("text".into(), Tag::String(s.clone()));
            }
            Content::Translatable { key, fallback, args } => {
                c.0.insert("translate".into(), Tag::String(key.clone()));
                if let Some(f) = fallback {
                    c.0.insert("fallback".into(), Tag::String(f.clone()));
                }
                if !args.is_empty() {
                    c.0.insert("with".into(), list(args.iter().map(Arg::to_tag).collect()));
                }
            }
            Content::Selector { pattern, separator } => {
                c.0.insert("selector".into(), Tag::String(pattern.clone()));
                if let Some(s) = separator {
                    c.0.insert("separator".into(), s.to_tag());
                }
            }
            Content::Score { name, objective } => {
                let mut s = Compound::default();
                s.0.insert("name".into(), Tag::String(name.clone()));
                s.0.insert("objective".into(), Tag::String(objective.clone()));
                c.0.insert("score".into(), Tag::Compound(s));
            }
            Content::Keybind(k) => {
                c.0.insert("keybind".into(), Tag::String(k.clone()));
            }
        }
        write_style(&self.style, &mut c);
        if !self.extra.is_empty() {
            c.0.insert("extra".into(), list(self.extra.iter().map(Component::to_tag).collect()));
        }
        Tag::Compound(c)
    }
}

impl Arg {
    fn to_tag(&self) -> Tag {
        match self {
            Arg::Text(c) => c.to_tag(),
            Arg::String(s) => Tag::String(s.clone()),
            Arg::Int(v) => Tag::Int(*v),
            Arg::Long(v) => Tag::Long(*v),
            Arg::Float(v) => Tag::Float(*v),
            Arg::Double(v) => Tag::Double(*v),
            Arg::Bool(v) => Tag::Byte(i8::from(*v)),
        }
    }
}

fn write_style(s: &Style, c: &mut Compound) {
    if let Some(col) = s.color {
        c.0.insert("color".into(), Tag::String(col.name()));
    }
    for (key, v) in [
        ("bold", s.bold),
        ("italic", s.italic),
        ("underlined", s.underlined),
        ("strikethrough", s.strikethrough),
        ("obfuscated", s.obfuscated),
    ] {
        if let Some(v) = v {
            c.0.insert(key.into(), Tag::Byte(i8::from(v)));
        }
    }
    if let Some(i) = &s.insertion {
        c.0.insert("insertion".into(), Tag::String(i.clone()));
    }
    if let Some(f) = &s.font {
        c.0.insert("font".into(), Tag::String(f.clone()));
    }
    if let Some(e) = &s.click {
        let mut m = Compound::default();
        let (action, field, value) = match e {
            ClickEvent::OpenUrl(v) => ("open_url", "url", Tag::String(v.clone())),
            ClickEvent::RunCommand(v) => ("run_command", "command", Tag::String(v.clone())),
            ClickEvent::SuggestCommand(v) => ("suggest_command", "command", Tag::String(v.clone())),
            ClickEvent::ChangePage(v) => ("change_page", "page", Tag::Int(*v)),
            ClickEvent::CopyToClipboard(v) => {
                ("copy_to_clipboard", "value", Tag::String(v.clone()))
            }
        };
        m.0.insert("action".into(), Tag::String(action.into()));
        m.0.insert(field.into(), value);
        c.0.insert("click_event".into(), Tag::Compound(m));
    }
    if let Some(e) = &s.hover {
        let mut m = Compound::default();
        match e {
            HoverEvent::ShowText(t) => {
                m.0.insert("action".into(), Tag::String("show_text".into()));
                m.0.insert("value".into(), t.to_tag());
            }
            HoverEvent::ShowItem { id, count } => {
                m.0.insert("action".into(), Tag::String("show_item".into()));
                m.0.insert("id".into(), Tag::String(id.clone()));
                if *count != 1 {
                    m.0.insert("count".into(), Tag::Int(*count));
                }
            }
            HoverEvent::ShowEntity { id, uuid, name } => {
                m.0.insert("action".into(), Tag::String("show_entity".into()));
                m.0.insert("id".into(), Tag::String(id.clone()));
                m.0.insert("uuid".into(), Tag::IntArray(uuid.to_vec()));
                if let Some(n) = name {
                    m.0.insert("name".into(), n.to_tag());
                }
            }
        }
        c.0.insert("hover_event".into(), Tag::Compound(m));
    }
}

/// A list of tags the way the game writes one: elements of one type stay as they are; if the
/// types differ, every element that is not a compound is wrapped as `{"": element}`.
pub fn list(items: Vec<Tag>) -> Tag {
    let first = items.first().map(Tag::type_id);
    let same = items.iter().all(|t| Some(t.type_id()) == first);
    if same {
        return Tag::List(typed_list(items));
    }
    let wrapped: Vec<Compound> = items
        .into_iter()
        .map(|t| match t {
            Tag::Compound(c) => c,
            other => {
                let mut c = Compound::default();
                c.0.insert(String::new(), other);
                c
            }
        })
        .collect();
    Tag::List(List::Compound(wrapped))
}

fn typed_list(items: Vec<Tag>) -> List {
    match items.first() {
        None => List::Empty,
        Some(Tag::Byte(_)) => List::Byte(
            items.into_iter().map(|t| if let Tag::Byte(v) = t { v } else { 0 }).collect(),
        ),
        Some(Tag::Short(_)) => List::Short(
            items.into_iter().map(|t| if let Tag::Short(v) = t { v } else { 0 }).collect(),
        ),
        Some(Tag::Int(_)) => {
            List::Int(items.into_iter().map(|t| if let Tag::Int(v) = t { v } else { 0 }).collect())
        }
        Some(Tag::Long(_)) => List::Long(
            items.into_iter().map(|t| if let Tag::Long(v) = t { v } else { 0 }).collect(),
        ),
        Some(Tag::Float(_)) => List::Float(
            items.into_iter().map(|t| if let Tag::Float(v) = t { v } else { 0.0 }).collect(),
        ),
        Some(Tag::Double(_)) => List::Double(
            items.into_iter().map(|t| if let Tag::Double(v) = t { v } else { 0.0 }).collect(),
        ),
        Some(Tag::String(_)) => List::String(
            items
                .into_iter()
                .map(|t| if let Tag::String(v) = t { v } else { String::new() })
                .collect(),
        ),
        Some(Tag::Compound(_)) => List::Compound(
            items
                .into_iter()
                .map(|t| if let Tag::Compound(v) = t { v } else { Compound::default() })
                .collect(),
        ),
        Some(Tag::ByteArray(_)) => List::ByteArray(
            items
                .into_iter()
                .map(|t| if let Tag::ByteArray(v) = t { v } else { Vec::new() })
                .collect(),
        ),
        Some(Tag::IntArray(_)) => List::IntArray(
            items
                .into_iter()
                .map(|t| if let Tag::IntArray(v) = t { v } else { Vec::new() })
                .collect(),
        ),
        Some(Tag::LongArray(_)) => List::LongArray(
            items
                .into_iter()
                .map(|t| if let Tag::LongArray(v) = t { v } else { Vec::new() })
                .collect(),
        ),
        Some(Tag::List(_)) => List::List(
            items.into_iter().map(|t| if let Tag::List(v) = t { v } else { List::Empty }).collect(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferrite_nbt::snbt::to_sorted_snbt;

    fn snbt(c: &Component) -> String {
        to_sorted_snbt(&c.to_tag())
    }

    #[test]
    fn plain_text_is_a_bare_string() {
        assert_eq!(c_tag("hi"), Tag::String("hi".into()));
        fn c_tag(s: &str) -> Tag {
            Component::literal(s).to_tag()
        }
    }

    // The next ones are messages the official server sent, captured with the scripted client.

    #[test]
    fn gamemode_feedback() {
        let c = Component::translatable(
            "commands.gamemode.success.self",
            vec![Component::translatable("gameMode.creative", vec![]).into()],
        );
        assert_eq!(
            snbt(&c),
            r#"{translate:"commands.gamemode.success.self",with:[{translate:"gameMode.creative"}]}"#
        );
    }

    #[test]
    fn errors_are_red_with_the_message_as_an_extra() {
        let c = Component::empty()
            .color(Color::Red)
            .append(Component::translatable("command.unknown.command", vec![]));
        assert_eq!(
            snbt(&c),
            r#"{color:"red",extra:[{translate:"command.unknown.command"}],text:""}"#
        );
    }

    #[test]
    fn mixed_lists_wrap_their_plain_elements() {
        let c = Component::empty().color(Color::Gray).append(Component::literal("...")).append(
            Component::translatable("command.context.here", vec![]).color(Color::Red).italic(true),
        );
        assert_eq!(
            snbt(&c),
            r#"{color:"gray",extra:[{"":"..."},{color:"red",italic:1b,translate:"command.context.here"}],text:""}"#
        );
        // Numbers next to components are wrapped too; numbers alone are a plain list.
        let mixed = Component::translatable(
            "commands.list.players",
            vec![1.into(), 4.into(), Component::literal("x").bold(true).into()],
        );
        assert_eq!(
            snbt(&mixed),
            r#"{translate:"commands.list.players",with:[{"":1},{"":4},{bold:1b,text:"x"}]}"#
        );
        let numbers = Component::translatable("argument.integer.low", vec![1.into(), 0.into()]);
        assert_eq!(snbt(&numbers), r#"{translate:"argument.integer.low",with:[1,0]}"#);
    }

    #[test]
    fn events_use_the_flat_layout() {
        let c = Component::literal("Bot")
            .insertion("Bot")
            .click(ClickEvent::SuggestCommand("/tell Bot ".into()))
            .hover(HoverEvent::ShowEntity {
                id: "minecraft:player".into(),
                uuid: [1, 2, 3, 4],
                name: Some(Box::new(Component::literal("Bot"))),
            });
        assert_eq!(
            snbt(&c),
            r#"{click_event:{action:"suggest_command",command:"/tell Bot "},hover_event:{action:"show_entity",id:"minecraft:player",name:"Bot",uuid:[I;1,2,3,4]},insertion:"Bot",text:"Bot"}"#
        );
    }
}
