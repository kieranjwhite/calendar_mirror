#[macro_export]
macro_rules! stm {
    (@as_path $p:path) => {$p};
    ($mod_name: ident, $enum_name:ident, $start: ident($($start_arg:ty),*), { $( [$($e:ident), +], $node:ident($($arg:ty),*) );+ } ) => {
                
        pub mod $mod_name
        {
            pub struct $start;

            $(
                pub struct $node {
                    _secret: ()
                }

                $(

                    impl From<$e> for $node {
                        fn from(_st: $e) -> $node {
                            println!("{:?} -> {:?}", stringify!($e), stringify!($node));
                            $node {
                                _secret: ()
                            }
                        }
                    }

                )*
            )*
        }
        
        pub enum $enum_name {
            $start($mod_name::$start $(, $start_arg)*),
            $(
                //$node($node $(, super::stm!(@as_path super :: $arg) )*),
                $node($mod_name::$node $(, $arg )*),
            )*
        }
    };
}
