//! Brigadier's `CommandDispatcher.parse`: walking a line through the tree.

use std::collections::BTreeMap;

use crate::args::Value;
use crate::reader::{Arg, Message, Reader, SyntaxError};
use crate::tree::{NodeKind, Tree};

/// An argument as parsed, with where it sat in the line.
#[derive(Clone, Debug, PartialEq)]
pub struct ParsedArgument {
    pub range: (usize, usize),
    pub value: Value,
}

/// What has been parsed so far.
#[derive(Clone, Debug, Default)]
pub struct Context {
    /// The nodes that matched, with the part of the line each covered.
    pub nodes: Vec<(usize, (usize, usize))>,
    pub arguments: BTreeMap<String, ParsedArgument>,
    /// The last matched node, when a command ends there.
    pub command: Option<usize>,
    /// After a redirect, parsing goes on in a new context.
    pub child: Option<Box<Context>>,
}

impl Context {
    /// The part of the line the top context covers.
    pub fn range(&self) -> (usize, usize) {
        match (self.nodes.first(), self.nodes.last()) {
            (Some(a), Some(b)) => (a.1.0, b.1.1),
            _ => (0, 0),
        }
    }

    /// The innermost context (where the command that runs is found).
    pub fn last(&self) -> &Context {
        match &self.child {
            Some(c) => c.last(),
            None => self,
        }
    }
}

/// The outcome of parsing a line: how far it got and why it went no further.
#[derive(Clone, Debug)]
pub struct ParseResults {
    pub context: Context,
    pub reader: Reader,
    /// Why each node that was tried (and did not match) failed.
    pub exceptions: Vec<(usize, SyntaxError)>,
    /// Some node on the way has an argument type this crate cannot parse yet, so the result
    /// may differ from the official server's.
    pub unsupported: bool,
}

fn unsupported_is_error(tree: &Tree, node: usize) -> bool {
    tree.nodes[node].parser.as_ref().is_some_and(|p| !p.is_supported())
}

/// The children of `node` that could match at the reader's cursor.
fn relevant_nodes(tree: &Tree, node: usize, reader: &Reader) -> Vec<usize> {
    let n = &tree.nodes[node];
    let literals: Vec<usize> =
        n.children.iter().copied().filter(|&c| tree.nodes[c].kind == NodeKind::Literal).collect();
    let arguments = || {
        n.children.iter().copied().filter(|&c| tree.nodes[c].kind == NodeKind::Argument).collect()
    };
    if literals.is_empty() {
        return arguments();
    }
    let mut r = reader.clone();
    let cursor = r.cursor();
    while r.can_read() && r.peek() != ' ' {
        r.skip();
    }
    let text = r.substring(cursor, r.cursor());
    match literals.into_iter().find(|&c| tree.nodes[c].name == text) {
        Some(l) => vec![l],
        None => arguments(),
    }
}

fn parse_node(
    tree: &Tree,
    node: usize,
    reader: &mut Reader,
    context: &mut Context,
) -> Result<(), SyntaxError> {
    let n = &tree.nodes[node];
    let start = reader.cursor();
    match n.kind {
        NodeKind::Literal => {
            let len = n.name.encode_utf16().count();
            if reader.can_read_n(len) {
                let end = start + len;
                if reader.substring(start, end) == n.name {
                    reader.set_cursor(end);
                    if !reader.can_read() || reader.peek() == ' ' {
                        context.nodes.push((node, (start, end)));
                        return Ok(());
                    }
                    reader.set_cursor(start);
                }
            }
            Err(reader.error(Message::tr_with(
                "argument.literal.incorrect",
                vec![Arg::Str(n.name.clone())],
            )))
        }
        NodeKind::Argument => {
            if unsupported_is_error(tree, node) {
                return Err(reader.error(Message::lit(format!(
                    "argument type of '{}' is not supported yet",
                    n.name
                ))));
            }
            let parser = n.parser.as_ref().expect("an argument has a parser");
            let value = parser.parse(reader)?;
            let range = (start, reader.cursor());
            context.arguments.insert(n.name.clone(), ParsedArgument { range, value });
            context.nodes.push((node, range));
            Ok(())
        }
        NodeKind::Root => Ok(()),
    }
}

fn parse_nodes(tree: &Tree, node: usize, original: &Reader, so_far: &Context) -> ParseResults {
    let mut errors: Vec<(usize, SyntaxError)> = Vec::new();
    let mut potentials: Vec<ParseResults> = Vec::new();
    let mut unsupported = false;
    let cursor = original.cursor();
    for child in relevant_nodes(tree, node, original) {
        let mut context = so_far.clone();
        let mut reader = original.clone();
        unsupported |= unsupported_is_error(tree, child);
        let outcome = parse_node(tree, child, &mut reader, &mut context).and_then(|()| {
            if reader.can_read() && reader.peek() != ' ' {
                Err(reader.error(Message::tr("command.expected.separator")))
            } else {
                Ok(())
            }
        });
        if let Err(e) = outcome {
            errors.push((child, e));
            reader.set_cursor(cursor);
            continue;
        }
        let c = &tree.nodes[child];
        context.command = if c.executable { Some(child) } else { None };
        let needed = if c.redirect.is_none() { 2 } else { 1 };
        if reader.can_read_n(needed) {
            reader.skip();
            if let Some(target) = c.redirect {
                let child_context = Context::default();
                let parse = parse_nodes(tree, target, &reader, &child_context);
                context.child = Some(Box::new(parse.context));
                return ParseResults {
                    context,
                    reader: parse.reader,
                    exceptions: parse.exceptions,
                    unsupported: unsupported | parse.unsupported,
                };
            }
            potentials.push(parse_nodes(tree, child, &reader, &context));
        } else {
            potentials.push(ParseResults {
                context,
                reader,
                exceptions: Vec::new(),
                unsupported: false,
            });
        }
    }
    if !potentials.is_empty() {
        let any_unsupported = potentials.iter().any(|p| p.unsupported);
        // A parse that used up the whole line beats one that did not; one without errors beats
        // one with; otherwise the first.
        potentials.sort_by(|a, b| {
            let (ar, br) = (a.reader.can_read(), b.reader.can_read());
            let (ae, be) = (!a.exceptions.is_empty(), !b.exceptions.is_empty());
            ar.cmp(&br).then(ae.cmp(&be))
        });
        let mut best = potentials.swap_remove(0);
        best.unsupported |= unsupported | any_unsupported;
        return best;
    }
    ParseResults {
        context: so_far.clone(),
        reader: original.clone(),
        exceptions: errors,
        unsupported,
    }
}

/// Parses `input` (without the leading `/`).
pub fn parse(tree: &Tree, input: &str) -> ParseResults {
    let reader = Reader::new(input);
    parse_nodes(tree, tree.root, &reader, &Context::default())
}

/// What the official server makes of a parse that cannot run: the error, if any.
/// (`Commands.getParseException`, then the check that a command ends where parsing stopped.)
pub fn parse_error(results: &ParseResults) -> Option<SyntaxError> {
    if results.reader.can_read() {
        if results.exceptions.len() == 1 {
            return Some(results.exceptions[0].1.clone());
        }
        let message = if results.context.range().0 == results.context.range().1 {
            Message::tr("command.unknown.command")
        } else {
            Message::tr("command.unknown.argument")
        };
        return Some(results.reader.error(message));
    }
    // Everything was read; the command must end at the last node.
    if results.context.last().command.is_none() {
        return Some(results.reader.error(Message::tr("command.unknown.command")));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree() -> Tree {
        Tree::for_level(4)
    }

    fn error(line: &str) -> Option<(String, usize)> {
        parse_error(&parse(&tree(), line)).map(|e| (e.message.to_json(), e.cursor))
    }

    #[test]
    fn complete_commands_parse() {
        assert_eq!(error("difficulty"), None);
        assert_eq!(error("difficulty hard"), None);
        assert_eq!(error("list uuids"), None);
    }

    #[test]
    fn unknown_and_incomplete_commands() {
        assert_eq!(
            error("nonsense here"),
            Some((r#"{"translate":"command.unknown.command"}"#.into(), 0))
        );
        assert_eq!(error("say"), Some((r#"{"translate":"command.unknown.command"}"#.into(), 3)));
        assert_eq!(
            error("difficulty hard extra"),
            Some((r#"{"translate":"command.unknown.argument"}"#.into(), 16))
        );
    }
}
