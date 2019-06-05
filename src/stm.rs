#[macro_export]
macro_rules! std {
    (create $mod_name: ident, $start: ident, { $( [$($e:ident), +], $node:ident );+ } ) => {
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
                            $node {
                                _secret: ()
                            }
                        }
                    }

                )*


            )*

            pub enum Machine {
                $start($start),
                $(
                    $node($node),
                )*
            }

            impl Machine {
                pub fn new() -> Machine {
                    Machine::$start($start)
                }
            }
        }
    };
}
