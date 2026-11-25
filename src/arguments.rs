//! Turns the text after the name of a command into a value of any type that implements [`Reflect`].

use bevy::{
    prelude::*,
    reflect::{
        TypeInfo, Typed,
        array::DynamicArray,
        enums::{DynamicEnum, DynamicVariant, EnumInfo, VariantInfo},
        list::DynamicList,
        structs::DynamicStruct,
        tuple::DynamicTuple,
        tuple_struct::DynamicTupleStruct,
    },
};
use std::{any::TypeId, collections::VecDeque};

/// Deeper types than this are most likely recursive.
const MAX_DEPTH: usize = 8;

/// Splits an entry by whitespace, except inside of double quotes.
/// Inside of those, `\"` and `\\` stand for a quote and a backslash.
pub fn split(entry: &str) -> Vec<String> {
    let (words, _) = split_open(entry);
    words.into_iter().map(|(_, word)| word).collect()
}

/// Like [`split`], but includes where every word starts, and whether the last one could continue.
pub fn split_open(entry: &str) -> (Vec<(usize, String)>, bool) {
    let mut words = Vec::new();
    let mut word: Option<(usize, String)> = None;
    let mut quoted = false;
    let mut chars = entry.char_indices().peekable();
    while let Some((index, c)) = chars.next() {
        if c.is_whitespace() && !quoted {
            words.extend(word.take());
            continue;
        }
        // "" is an empty word.
        let (_, word) = word.get_or_insert_with(|| (index, String::new()));
        match c {
            '"' => quoted = !quoted,
            '\\' if quoted && matches!(chars.peek(), Some((_, '"' | '\\'))) => {
                word.extend(chars.next().map(|(_, c)| c));
            }
            c => word.push(c),
        }
    }
    let open = word.is_some();
    words.extend(word);
    (words, open)
}

/// An argument of a command, as shown by `help`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Argument {
    /// Empty if it has none, which is the case outside of structs.
    pub name: String,
    pub kind: String,
    pub arity: Arity,
    /// The doc comment of the field.
    pub docs: String,
    /// Everything it could be, if there are only a few options.
    pub values: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arity {
    Required,
    Optional,
    Many,
}

impl Argument {
    pub fn usage(&self) -> String {
        let text = match self.name.as_str() {
            "" => self.kind.clone(),
            name => format!("{name}: {}", self.kind),
        };
        match self.arity {
            Arity::Required => format!("<{text}>"),
            Arity::Optional => format!("[{text}]"),
            Arity::Many => format!("[{text}...]"),
        }
    }
}

/// Lists the arguments of a type, or returns the path of the first type which has no textual form.
pub fn describe<A: Typed>() -> Result<Vec<Argument>, &'static str> {
    let mut arguments = Vec::new();
    let root = Argument {
        name: String::new(),
        kind: String::new(),
        arity: Arity::Required,
        docs: String::new(),
        values: Vec::new(),
    };
    describe_type(A::type_info(), &root, 0, &mut arguments)?;
    Ok(arguments)
}

/// The doc comment of a type.
pub fn docs<A: Typed>() -> String {
    clean_docs(A::type_info().docs())
}

/// Parses all of the words as a single value.
pub fn parse<A: FromReflect + Typed>(words: &[String]) -> Result<A, String> {
    let mut words: VecDeque<String> = words.iter().cloned().collect();
    let value = parse_type(A::type_info(), "", &mut words)?;
    if let Some(word) = words.front() {
        return Err(format!("unexpected argument {word}"));
    }
    A::from_reflect(&*value).ok_or_else(|| format!("{} cannot be created", A::type_path()))
}

fn describe_type(
    info: &'static TypeInfo,
    argument: &Argument,
    depth: usize,
    arguments: &mut Vec<Argument>,
) -> Result<(), &'static str> {
    let unsupported = Err(info.type_path());
    if depth > MAX_DEPTH {
        return unsupported;
    }
    let mut describe_field = |field: Option<&'static TypeInfo>, argument: &Argument| match field {
        Some(field) => describe_type(field, argument, depth + 1, arguments),
        None => Err(info.type_path()),
    };
    match info {
        TypeInfo::Struct(info) => info.iter().try_for_each(|field| {
            let field_argument = Argument {
                name: join(&argument.name, field.name()),
                docs: clean_docs(field.docs()),
                ..argument.clone()
            };
            describe_field(field.type_info(), &field_argument)
        }),
        TypeInfo::TupleStruct(info) => info
            .iter()
            .try_for_each(|field| describe_field(field.type_info(), argument)),
        TypeInfo::Tuple(info) => info
            .iter()
            .try_for_each(|field| describe_field(field.type_info(), argument)),
        TypeInfo::Array(info) => {
            (0..info.capacity()).try_for_each(|_| describe_field(info.item_info(), argument))
        }
        TypeInfo::List(info) => {
            let argument = Argument {
                arity: Arity::Many,
                ..argument.clone()
            };
            describe_field(info.item_info(), &argument)
        }
        TypeInfo::Enum(info) => {
            if let Some(some) = option_item(info) {
                let arity = match argument.arity {
                    Arity::Many => Arity::Many,
                    _ => Arity::Optional,
                };
                let argument = Argument {
                    arity,
                    ..argument.clone()
                };
                return describe_field(some, &argument);
            }
            let names: Vec<_> = unit_variants(info).collect();
            if names.is_empty() {
                return unsupported;
            }
            arguments.push(Argument {
                kind: names.join("|"),
                values: names.into_iter().map(String::from).collect(),
                ..argument.clone()
            });
            Ok(())
        }
        _ => {
            let Some(kind) = primitive_name(info.type_id()) else {
                return unsupported;
            };
            arguments.push(Argument {
                kind: kind.into(),
                values: match kind {
                    "bool" => vec!["false".into(), "true".into()],
                    _ => Vec::new(),
                },
                ..argument.clone()
            });
            Ok(())
        }
    }
}

fn parse_type(
    info: &'static TypeInfo,
    name: &str,
    words: &mut VecDeque<String>,
) -> Result<Box<dyn PartialReflect>, String> {
    let mut parse_field = |field: Option<&'static TypeInfo>, name: &str| match field {
        Some(field) => parse_type(field, name, words),
        None => Err(unreadable(info)),
    };
    match info {
        TypeInfo::Struct(info) => {
            let mut value = DynamicStruct::default();
            for field in info.iter() {
                let parsed = parse_field(field.type_info(), &join(name, field.name()))?;
                value.insert_boxed(field.name(), parsed);
            }
            Ok(Box::new(value))
        }
        TypeInfo::TupleStruct(info) => {
            let mut value = DynamicTupleStruct::default();
            for field in info.iter() {
                value.insert_boxed(parse_field(field.type_info(), name)?);
            }
            Ok(Box::new(value))
        }
        TypeInfo::Tuple(info) => {
            let mut value = DynamicTuple::default();
            for field in info.iter() {
                value.insert_boxed(parse_field(field.type_info(), name)?);
            }
            Ok(Box::new(value))
        }
        TypeInfo::Array(info) => {
            let items: Result<Vec<_>, _> = (0..info.capacity())
                .map(|_| parse_field(info.item_info(), name))
                .collect();
            Ok(Box::new(DynamicArray::new(items?.into_boxed_slice())))
        }
        TypeInfo::List(list) => {
            let mut value = DynamicList::default();
            while !words.is_empty() {
                let before = words.len();
                let item = list.item_info().ok_or_else(|| unreadable(info))?;
                value.push_box(parse_type(item, name, words)?);
                // Would never end.
                if words.len() == before {
                    return Err(unreadable(info));
                }
            }
            Ok(Box::new(value))
        }
        TypeInfo::Enum(info) => parse_enum(info, name, words),
        _ => parse_primitive(info, name, words),
    }
}

fn parse_enum(
    info: &'static EnumInfo,
    name: &str,
    words: &mut VecDeque<String>,
) -> Result<Box<dyn PartialReflect>, String> {
    if let Some(some) = option_item(info) {
        if words.is_empty() {
            return Ok(Box::new(DynamicEnum::new("None", DynamicVariant::Unit)));
        }
        let some = some.ok_or_else(|| format!("{} cannot be read from text", info.type_path()))?;
        let mut value = DynamicTuple::default();
        value.insert_boxed(parse_type(some, name, words)?);
        return Ok(Box::new(DynamicEnum::new(
            "Some",
            DynamicVariant::Tuple(value),
        )));
    }
    let names: Vec<_> = unit_variants(info).collect();
    let word = next(words, name, || names.join("|"))?;
    match names.iter().find(|name| name.eq_ignore_ascii_case(&word)) {
        Some(variant) => Ok(Box::new(DynamicEnum::new(*variant, DynamicVariant::Unit))),
        None => Err(expected(name, &names.join("|"), &word)),
    }
}

macro_rules! primitives {
    ($($type:ident),* $(,)?) => {
        fn primitive_name(id: TypeId) -> Option<&'static str> {
            $(if id == TypeId::of::<$type>() {
                return Some(stringify!($type));
            })*
            None
        }

        fn parse_primitive(
            info: &'static TypeInfo,
            name: &str,
            words: &mut VecDeque<String>,
        ) -> Result<Box<dyn PartialReflect>, String> {
            let Some(kind) = primitive_name(info.type_id()) else {
                return Err(unreadable(info));
            };
            let word = next(words, name, || kind.into())?;
            $(if info.type_id() == TypeId::of::<$type>() {
                return match word.parse::<$type>() {
                    Ok(value) => Ok(Box::new(value)),
                    Err(_) => Err(expected(name, kind, &word)),
                };
            })*
            Err(unreadable(info))
        }
    };
}

primitives!(
    bool, char, String, f32, f64, u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize,
);

fn next(
    words: &mut VecDeque<String>,
    name: &str,
    kind: impl Fn() -> String,
) -> Result<String, String> {
    words.pop_front().ok_or_else(|| match name {
        "" => format!("missing {}", kind()),
        name => format!("missing {name}"),
    })
}

fn expected(name: &str, kind: &str, word: &str) -> String {
    match name {
        "" => format!("expected {kind}, got {word}"),
        name => format!("{name}: expected {kind}, got {word}"),
    }
}

fn unreadable(info: &TypeInfo) -> String {
    format!("{} cannot be read from text", info.type_path())
}

/// What is inside of an `Option`. The inner one is `None` if it has no type info.
fn option_item(info: &'static EnumInfo) -> Option<Option<&'static TypeInfo>> {
    if info.type_path_table().module_path() != Some("core::option") {
        return None;
    }
    match info.variant("Some")? {
        VariantInfo::Tuple(some) => Some(some.field_at(0)?.type_info()),
        _ => None,
    }
}

fn unit_variants(info: &'static EnumInfo) -> impl Iterator<Item = &'static str> {
    info.iter().filter_map(|variant| match variant {
        VariantInfo::Unit(variant) => Some(variant.name()),
        _ => None,
    })
}

fn join(parent: &str, field: &str) -> String {
    match parent {
        "" => field.into(),
        parent => format!("{parent}.{field}"),
    }
}

/// Puts the first paragraph of a doc comment on a single line.
fn clean_docs(docs: Option<&str>) -> String {
    docs.unwrap_or_default()
        .lines()
        .map(str::trim)
        .skip_while(|line| line.is_empty())
        .take_while(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Reflect, Debug, PartialEq)]
    enum Team {
        Red,
        Blue,
        #[allow(dead_code)]
        Custom(u32),
    }

    /// Joins
    /// a team.
    ///
    /// Not shown.
    #[derive(Reflect, Debug, PartialEq)]
    struct Join {
        /// The team.
        team: Team,
        name: String,
        slot: Option<u32>,
    }

    #[derive(Reflect, Debug, PartialEq)]
    struct Teleport {
        to: Vec3,
        instant: bool,
    }

    #[derive(Reflect, Debug, PartialEq)]
    struct Meters(f32);

    #[derive(Reflect, Debug, PartialEq)]
    struct Empty {}

    #[derive(Reflect)]
    struct Recursive {
        #[allow(dead_code)]
        children: Vec<Recursive>,
    }

    fn read<A: FromReflect + Typed>(entry: &str) -> Result<A, String> {
        parse(&split(entry))
    }

    fn usage<A: Typed>() -> String {
        let arguments = describe::<A>().unwrap();
        let usages: Vec<_> = arguments.iter().map(Argument::usage).collect();
        usages.join(" ")
    }

    #[test]
    fn words_are_split_by_whitespace() {
        assert_eq!(split("  give  apple\t3 "), ["give", "apple", "3"]);
        assert!(split("   ").is_empty());
    }

    #[test]
    fn quotes_keep_words_together() {
        assert_eq!(
            split(r#"say "hello  world" x"#),
            ["say", "hello  world", "x"]
        );
        assert_eq!(split(r#"a"b c"d """#), ["ab cd", ""]);
        assert_eq!(split(r#""a \"b\" \\ \n""#), [r#"a "b" \ \n"#]);
        assert_eq!(split(r#"say "never closed"#), ["say", "never closed"]);
        assert_eq!(split(r"don't C:\dir"), ["don't", r"C:\dir"]);
    }

    #[test]
    fn primitives() {
        assert_eq!(read("-3"), Ok(-3i32));
        assert_eq!(read("1.5"), Ok(1.5f32));
        assert_eq!(read("true"), Ok(true));
        assert_eq!(read("x"), Ok('x'));
        assert_eq!(read(r#""two words""#), Ok("two words".to_string()));
        assert_eq!(read::<u8>("256"), Err("expected u8, got 256".into()));
        assert_eq!(read::<u8>(""), Err("missing u8".into()));
        assert_eq!(read::<u8>("1 2"), Err("unexpected argument 2".into()));
    }

    #[test]
    fn tuples_and_arrays() {
        assert_eq!(read("1 x 2"), Ok((1u32, "x".to_string(), 2.0f32)));
        assert_eq!(read("1 2 3"), Ok([1u32, 2, 3]));
        assert_eq!(read::<(u32, u32)>("1"), Err("missing u32".into()));
        assert_eq!(read("4"), Ok(Meters(4.0)));
        assert_eq!(read(""), Ok(()));
        assert_eq!(read(""), Ok(Empty {}));
    }

    #[test]
    fn structs() {
        let join = |slot| Join {
            team: Team::Blue,
            name: "Al".into(),
            slot,
        };
        assert_eq!(read("blue Al"), Ok(join(None)));
        assert_eq!(read("BLUE Al 3"), Ok(join(Some(3))));
        assert_eq!(
            read::<Join>("green Al"),
            Err("team: expected Red|Blue, got green".into())
        );
        // Variants with fields can't be written as text.
        assert!(read::<Join>("Custom Al").is_err());
        assert_eq!(read::<Join>("Red"), Err("missing name".into()));
        assert_eq!(
            read::<Join>("Red Al x"),
            Err("slot: expected u32, got x".into())
        );
    }

    #[test]
    fn nested_structs() {
        let to = Vec3::new(1.0, 2.0, 3.0);
        assert_eq!(read("1 2 3 true"), Ok(Teleport { to, instant: true }));
        assert_eq!(
            read::<Teleport>("1 y"),
            Err("to.y: expected f32, got y".into())
        );
        assert_eq!(read("1 2 3"), Ok(Some(to)));
        assert_eq!(read(""), Ok(None::<Vec3>));
        assert_eq!(read::<Option<Vec3>>("1 2"), Err("missing z".into()));
    }

    #[test]
    fn lists_take_the_rest() {
        assert_eq!(read("a b"), Ok(vec!["a".to_string(), "b".to_string()]));
        assert_eq!(read(""), Ok(Vec::<u32>::new()));
        assert_eq!(read("x 1 2"), Ok(("x".to_string(), vec![1u32, 2])));
        assert_eq!(read("1 2 3 4"), Ok(vec![(1u32, 2u32), (3, 4)]));
        assert!(read::<Vec<(u32, u32)>>("1 2 3").is_err());
        // Items that consume nothing must not loop forever.
        assert!(read::<Vec<Empty>>("a").is_err());
    }

    #[test]
    fn usage_is_generated() {
        assert_eq!(usage::<()>(), "");
        assert_eq!(usage::<(f32, u8)>(), "<f32> <u8>");
        assert_eq!(usage::<Option<u32>>(), "[u32]");
        assert_eq!(usage::<Vec<String>>(), "[String...]");
        assert_eq!(
            usage::<Join>(),
            "<team: Red|Blue> <name: String> [slot: u32]"
        );
        assert_eq!(
            usage::<Teleport>(),
            "<to.x: f32> <to.y: f32> <to.z: f32> <instant: bool>"
        );
    }

    #[test]
    fn unsupported_types_are_found() {
        assert!(describe::<Recursive>().is_err());
        assert!(describe::<std::time::Duration>().is_err());
        assert!(describe::<(u32, Option<Entity>)>().is_err());
        assert!(read::<std::time::Duration>("1").is_err());
    }

    #[test]
    fn doc_comments_are_read() {
        assert_eq!(docs::<Join>(), "Joins a team.");
        assert_eq!(describe::<Join>().unwrap()[0].docs, "The team.");
    }
}
