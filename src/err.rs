#[macro_export]
macro_rules! err {
    ($enum_name:ident { $( $var:ident($embedded:path) ),+ } ) => {
        $(
            impl From<$embedded> for $enum_name {
                fn from(orig: $embedded) -> $enum_name {
                    use log::error;

                    error!("err: {:?} into {:?}::{:?}({:?})", stringify!($embedded), stringify!($enum_name), stringify!($var), stringify!($embedded));
                    $enum_name::$var(orig)
                }
            }
        )*

        #[derive(Debug)]
        pub enum $enum_name {
            $(
                $var($embedded),
            )*
        }
    };
}
