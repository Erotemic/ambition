//! One declaration for an enum's native, wire, and script choices.

/// Define unit variants with explicit wire tags and script names.
/// The same declaration supplies the field shape and both codecs.
#[macro_export]
macro_rules! port_choices {
    ($(#[$attr:meta])* $vis:vis enum $name:ident {
        $($(#[$variant_attr:meta])* $variant:ident = $tag:literal => $script:literal),+ $(,)?
    }) => {
        $(#[$attr])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[repr(u8)]
        $vis enum $name { $($(#[$variant_attr])* $variant = $tag),+ }

        impl $name {
            pub const fn as_str(self) -> &'static str {
                match self { $(Self::$variant => $script),+ }
            }
            pub fn parse(value: &str) -> Result<Self, ::std::string::String> {
                match value { $($script => Ok(Self::$variant)),+,
                    _ => Err(format!("unknown {} choice {value:?}", stringify!($name))),
                }
            }
        }
        impl $crate::structure::Field<$name> for $name {
            fn shape() -> $crate::structure::Shape {
                $crate::structure::Shape::Choice(vec![$($crate::structure::CaseShape {
                    name: $script, payload: None, script: $script,
                }),+])
            }
            fn put(value: &$name, out: &mut Vec<u8>) { $crate::wire::put_u8(out, *value as u8); }
            fn read(reader: &mut $crate::wire::WireReader<'_>) -> Result<$name, $crate::wire::WireError> {
                match reader.u8()? { $($tag => Ok(Self::$variant)),+,
                    tag => Err($crate::wire::WireError::BadTag(tag)),
                }
            }
            fn to_script(value: &$name) -> $crate::script::ScriptValue {
                $crate::script::ScriptValue::Text(value.as_str().into())
            }
            fn from_script(value: &$crate::script::ScriptValue) -> Result<$name, ::std::string::String> {
                Self::parse(value.as_text()?)
            }
        }
    };
}
