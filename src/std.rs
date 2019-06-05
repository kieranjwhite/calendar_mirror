#[macro_export]
macro_rules! std {
    (create $start: ident, { $( [$($e:ident), +], $node:ident );+ } ) => {
        pub struct $start;

            $(
                pub struct $node;

                $(

                    impl From<$e> for $node {
                        fn from(_st: $e) -> $node {
                            $node
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
    };
}
