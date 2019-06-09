#[macro_export]
macro_rules! stm {
    (@graph ($start: ident,  { }) -> ($($dot_file: tt)*)) => {
        log_syntax!(digraph { start[ shape="point"]; $($dot_file)* });
    };
    (@graph ($start: ident,  { $( [$($e:ident), +], $node:ident );+ }) -> $(dot_file: tt)*) => {
        stm!(@graph ($start {}) -> ("$start" [shape="ellipse"]; start->"$start"; $("$node" [shape="ellipse"]; $("$e" -> "$node";)*)*));
    };
    ($mod_name: ident, $start: ident, { $( [$($e:ident), +], $node:ident );+ } ) => {
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
                pub const fn new_stm() -> Machine {
                    Machine::$start($start)
                }
            }
        }
    };
}
