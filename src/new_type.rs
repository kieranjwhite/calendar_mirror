/*
Copyright [2019] [Kieran White]

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
*/

///
/// Newtype implementation for non-Copy types.
///
/// The new type implements AsRef and From on the wrapper for the inner type.
///
/// # Examples
///
/// ```
/// non_copyable!(Width, u32);
///
/// let w=Width(5);
///
/// assert_eq!(&5, w.as_ref());
/// assert_eq!(5, w.into);
/// ```
#[macro_export]
macro_rules! non_copyable {
    ($outer_type:ident, $inner_type: ident) => {
        pub struct $outer_type($inner_type);

        impl AsRef<$inner_type> for $outer_type {
            fn as_ref(&self) -> &$inner_type {
                &self.0
            }
        }

        impl From<$outer_type> for $inner_type {
            fn from(self) -> $inner_type {
                self.0
            }
        }
    };
}

///
/// Newtype implementation for Copy types.
///
/// The new type implements AsRef on the wrapper for the inner type.
///
/// # Examples
///
/// ```
/// non_copyable!(Width, u32);
///
/// let w=Width(5);
///
/// assert_eq!(&5, w.as_ref());
/// ```
#[macro_export]
macro_rules! copyable {
    ($outer_type:ident, $inner_type: ident) => {
        #[derive(Copy, Clone, Debug, PartialEq)]
        pub struct $outer_type(pub $inner_type);

        impl AsRef<$inner_type> for $outer_type {
            fn as_ref(&self) -> &$inner_type {
                &self.0
            }
        }
    };
}

///
/// Newtype implementation for Clone types.
///
/// The new type implements AsRef on the wrapper for the inner type.
///
/// # Examples
///
/// ```
/// non_copyable!(Width, u32);
///
/// let w=Width(5);
///
/// assert_eq!(&5, w.as_ref());
/// ```
#[macro_export]
macro_rules! cloneable {
    ($outer_type:ident, $inner_type: ty) => {
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct $outer_type(pub $inner_type);

        impl AsRef<$inner_type> for $outer_type {
            fn as_ref(&self) -> &$inner_type {
                &self.0
            }
        }
    };
}

///
/// Newtype implementation for non-Clone types.
///
/// The new type implements AsRef on the wrapper for the inner type.
/// From is not implemented.
///
/// # Examples
///
/// ```
/// non_copyable!(Width, u32);
///
/// let w=Width(5);
///
/// assert_eq!(&5, w.as_ref());
/// ```
#[macro_export]
macro_rules! reffable {
    ($outer_type:ident, $inner_type: ident) => {
        #[derive(Debug)]
        pub struct $outer_type($inner_type);

        impl AsRef<$inner_type> for $outer_type {
            fn as_ref(&self) -> &$inner_type {
                &self.0
            }
        }
    };
}
