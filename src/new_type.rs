#[macro_export]
macro_rules! non_copyable {
    ($outer_type:ident, $inner_type: ident) => {
        pub struct $outer_type($inner_type);

        impl From<$outer_type> for $inner_type {
            fn from(self) -> $inner_type {
                self.0
            }
        }
    };
}

#[macro_export]
macro_rules! copyable {
    ($outer_type:ident, $inner_type: ident) => {
        #[derive(Copy,Clone,Debug,PartialEq)]
        pub struct $outer_type($inner_type);
    }
}

#[macro_export]
macro_rules! cloneable {
    ($outer_type:ident, $inner_type: ident) => {
        #[derive(Clone,Debug)]
        pub struct $outer_type($inner_type);
    }
}

#[macro_export]
macro_rules! reffable {
    ($outer_type:ident, $inner_type: ident) => {
        #[derive(Debug)]
        pub struct $outer_type($inner_type);

        impl $outer_type {
            pub fn inner_ref() -> &$inner_type {
                &self.0
            }
        }
    };
}
