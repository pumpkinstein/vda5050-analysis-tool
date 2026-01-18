/// Defines a fixed string-backed enum with stable compact codes.
///
/// The declaration is the single source of truth for the enum variants, their
/// serialized strings, and the category order used by dictionary columns.
#[macro_export]
macro_rules! fixed_string_enum {
    (
        $(#[$enum_meta:meta])*
        $vis:vis enum $name:ident {
            $(
                $(#[$variant_meta:meta])*
                $variant:ident => $value:literal
            ),+ $(,)?
        }
    ) => {
        $(#[$enum_meta])*
        #[repr(u8)]
        #[derive(
            serde::Serialize,
            serde::Deserialize,
            Debug,
            Clone,
            Copy,
            PartialEq,
            Eq
        )]
        $vis enum $name {
            $(
                $(#[$variant_meta])*
                #[serde(rename = $value)]
                $variant,
            )+
        }

        impl $name {
            /// Serialized values in the same order as the enum discriminants.
            pub const VALUES: &'static [&'static str] = &[$($value),+];

            /// Returns the compact code used by fixed categorical columns.
            #[inline]
            pub const fn code(self) -> u8 {
                self as u8
            }

            /// Returns the VDA 5050 serialized value.
            #[inline]
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $value,)+
                }
            }
        }
    };
}

pub mod common;
pub mod connection;
pub mod factsheet;
pub mod instant_actions;
pub mod order;
pub mod state;
pub mod visualization;
