/*
#[macro_export]
macro_rules! start {
    ($e1:ident) => {
        pub struct $e1;
    };
}

#[macro_export]
macro_rules! tr {
        ([$($e1:ty),+], $e2:ident) => {
            struct $e2;

            $(
            impl From<$e1> for $e2 {
                fn from(_st: $e1) -> $e2 {
                    $e2
                }
            }
            )*
        };
    }

#[macro_export]
macro_rules! next {
    ( $orig:ident, $name:ident :: $state:ident ) => {
        $name::$state($orig.into())
    };
}
*/

#[macro_export]
macro_rules! std {
    (create $start: ident, { $( [$($e:ident), +], $node:ident );+ } ) => {
        #[derive(Clone)]
        pub struct $start;

            $(
                #[derive(Clone)]
                pub struct $node;

                $(

                    impl From<$e> for $node {
                        fn from(_st: $e) -> $node {
                            $node
                        }
                    }

                    impl From<Machine<$e>> for Machine<$node>  {
                        fn from(mach: Machine<$e>) -> Machine<$node> {
                            Machine {
                                st: mach.st.into()
                            }
                        }
                    }
                    /*
                    impl<S> Machine<S> {
                        pub fn inst(self, $node) -> Machine {
                            &self.st
                        }
                    }
                    */
                )*


            )*

            pub struct Machine<S> {
                st: S,
            }

        impl Machine<$start> {
            pub fn new() -> Machine<$start> {
                Machine {
                    st: $start
                }
            }
        }

        impl<S> Machine<S> {
            pub fn state(&self) -> &S {
                &self.st
            }
        }
    };
    (accept $machine: ident, $next: ident) => {
        {
            let next_machine=stm::Machine::<$next>::from($machine);
            next_machine
        }
    };

}


                /*
                pub fn accept(self, new_st: States) -> Machine {

                    match new_st {

                        $(
                            States::$node => {
                                match self.st {
                                    $(
                                        States::$e =>  { Machine {
                                            st: new_st
                                        }
                                        }
                                    )*
                                        _ => {
                                            Machine {
                                                st: self.st
                                            }
                                        }
                                }
                            }
                        )*

                            _ => {
                                Machine {
                                    st: self.st
                                }
                            }
                    }
                }
                 */
