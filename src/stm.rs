#[macro_export]
macro_rules! stm {
    (@append_tuple $tuple:expr, ($($idx:expr),*), () -> $enum_name:ident :: $start:ident ($($comp:expr),*)) => {
        $enum_name :: $start ($($comp),*)
    };
    (@append_tuple $tuple:expr, ($head_idx:ty, $($idx:expr),*), ($head:ty, $($arg:ty),*) -> $enum_name:ident :: $start:ident ()) => {
        crate::stm!(@append_tuple $tuple,  ($($idx:expr),*), ($($arg:ty),*) -> $enum_name :: $start ( $tuple.$head_idx ))
    };
    (@append_tuple $tupel:expr, ($head_idx:ty, $($idx:expr),*), ($head:ty, $($arg:ty),*) -> $enum_name:ident :: $start:ident ($($comp:expr),+)) => {
        crate::stm!(@append_tuple $tuple, ($($idx:expr),*), ($($arg:ty),*) -> $enum_name :: $start ( $tuple.$head_idx ))
    };
    (@sub_build_enum () -> { pub enum $enum_name:ident<$t1:tt, $t2:tt> {$($processed_var:ident(dropper::$processed:ident)),*}}) => {
        #[derive(Debug)]
        pub enum $enum_name {
            $($processed_var(dropper::$processed)),*
        }
    };
    (@sub_build_enum ($head:tt |end| $(, $tail:ident $(| $tag:ident |)?)*)-> { pub enum $enum_name:ident<$t1:tt, $t2:tt> { } }) => {
        crate::stm!(@sub_build_enum ($($tail $(| $tag |)*),*) -> {
            pub enum $enum_name<$t1,$t2> {
                $head(dropper::$head)
            }
        });
    };
    (@sub_build_enum ($head:tt |end| $(, $tail:ident $(| $tag:ident |)?)*)-> { pub enum $enum_name:ident<$t1:tt, $t2:tt> { $($processed_var:ident( dropper::$processed:ident)),+} }) => {
        crate::stm!(@sub_build_enum ($($tail $(| $tag |)*),*) -> {
            pub enum $enum_name<$t1, $t2> {
                $($processed_var(dropper::$processed)),*,
                $head(dropper::$head)
            }
        });
    };
    (@sub_build_enum ($head:tt $(, $tail:ident $(| $tag:ident |)?)*)-> { pub enum $enum_name:ident<$t1:tt,$t2:tt> { $($processed_var:ident( dropper::$processed:ident)),*} }) => {
        crate::stm!(@sub_build_enum ($($tail $(| $tag |)*),*) -> {
            pub enum $enum_name<$t1,$t2> {
                $($processed_var(dropper::$processed)),*
            }
        });
    };
    (@sub_bare_dropper_enum $enum_name:ident, $($var:ident),*) => {
        #[allow(dead_code)]
        #[derive(Debug)]
        pub enum $enum_name {
            $(
                $var(dropper::$var),
            )*
        }
    };
    (@sub_wall nowall $mod_name:ident, $enum_name:ident, $($var:ident($($arg:ty),*)),*) => {
        #[allow(dead_code)]
        pub enum $enum_name {
            $(
                $var($mod_name::$var $(, $arg )*),
            )*
        }
    };
    (@sub_wall wall $mod_name:ident, $enum_name:ident, $($var:ident($($arg:ty),*)),*) => {
        #[warn(dead_code)]
        pub enum $enum_name {
            $(
                $var($mod_name::$var $(, $arg )*),
            )*
        }
    };

    (@sub_end_filter end $($sub:tt)*) => {$($sub)*};
    (@sub_end_filter $tag:tt $($sub:tt)*) => {};
    
    (@sub_pattern $_t:tt $sub:pat) => {$sub};
    (@private $machine_tag:tt $pertinence:tt $mod_name:ident, $enum_name:ident, $stripped_name:ident, $term_name:ident, [$($start_e:ident), *] => $start: ident($($start_arg:ty),*) $(| $start_tag:tt |)?, { $( [$($e:ident), +] => $node:ident($($arg:ty),*) $(| $tag:tt |)? );+ $(;)? } ) => {

        use $mod_name::$term_name;
        use $mod_name::$stripped_name;
        
        mod $mod_name
        {
            use super::$enum_name;
            //use super::$stripped_name;
            //use super::$term_name;
            
            pub struct $start {
                pub finaliser: Box<dyn FnOnce($stripped_name) -> $term_name>
            }

            impl Drop for $start {
                fn drop(&mut self) {
                    let finaliser=(*self.finaliser).clone();
                    let _term=finaliser($stripped_name::$start(dropper::$start::new()));                    
                }
            }

            $(
                impl From<$start_e> for $start {
                    fn from(mut old_st: $start_e) -> $start {
                        println!("{:?} -> {:?}", stringify!($start_e), stringify!($start));
                        $start {
                            finaliser: old_st.finaliser
                        }
                    }
                }
            )*

            impl std::fmt::Debug for $start {
                fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
                    f.debug_struct(stringify!($start))
                        .finish()
                }
            }

            impl $start {
                pub fn new(finaliser: Box<dyn FnOnce($stripped_name) -> $term_name>) -> $start {
                    let node=$start {
                        finaliser:finaliser
                    };
                    $( crate::stm!{@sub_end_filter $start_tag
                                   node.end_tags_found();
                    } )*;
                    
                    $(
                        $( crate::stm!{@sub_end_filter $tag
                                       node.end_tags_found();
                        } )*
                    )*;
                    
                    node
                }

                pub fn end_tags_found(&self){}

                pub fn is_accepting_state(&self) -> bool {
                    $( crate::stm!{@sub_end_filter $start_tag
                                   return true;
                    } )*
                        return false;
                }
                
            }

            $( crate::stm!{@sub_end_filter $start_tag
                impl $start {
                        pub fn dropper<S>(old: S) -> $start where $start:From<S> {
                            $start::from(old)
                        }

                }

            } )*

            $(
                impl std::fmt::Debug for $node {
                    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
                        f.debug_struct(stringify!($node))
                            .finish()
                    }
                }

                pub struct $node {
                    pub finaliser: Box<dyn FnOnce($stripped_name) -> $term_name>,
                    _secret: ()
                }

                impl $node {
                    pub fn is_accepting_state(&self) -> bool {
                        $( crate::stm!{@sub_end_filter $tag
                                       return true;
                        } )*;
                        return false;
                    }
                }
                
                impl Drop for $node {
                    fn drop(&mut self) {
                        let _term=(*self.finaliser)($stripped_name::$node(dropper::$node::new()));
                    }
                }
                
                $(
                    impl From<$e> for $node {
                        fn from(mut old_st: $e) -> $node {
                            println!("{:?} -> {:?}", stringify!($e), stringify!($node));
                            $node {
                                finaliser: old_st.finaliser,
                                _secret: ()
                            }
                        }
                    }
                )*
                    
                $( crate::stm!{@sub_end_filter $tag
                               impl $node {
                                   pub fn dropper<S>(old: S) -> $node where $node:From<S> {
                                       $node::from(old)
                                   }
                               }
                } )*
                    
            )*

            #[cfg(feature = "render_stm")]
            pub type Nd = &'static str;
            #[cfg(feature = "render_stm")]
            pub type Ed=(&'static str, &'static str);

            #[cfg(feature = "render_stm")]
            pub struct MachineEdges(pub Vec<Ed>);

            #[cfg(feature = "render_stm")]
            pub const START_NODE_NAME:&str="_start";

            #[cfg(feature = "render_stm")]
            impl<'a> dot::GraphWalk<'a, Nd, Ed> for MachineEdges {
                fn nodes(&self) -> dot::Nodes<'a, Nd> {
                    // (assumes that |N| \approxeq |E|)
                    let &MachineEdges(ref v) = self;
                    let mut nodes = Vec::with_capacity(v.len()*2);
                    nodes.push(START_NODE_NAME);
                    for &(s,t) in v {
                        nodes.push(s); nodes.push(t);
                    }
                    nodes.sort();
                    nodes.dedup();

                    std::borrow::Cow::Owned(nodes)
                }

                fn edges(&'a self) -> dot::Edges<'a, Ed> {
                    let &MachineEdges(ref edges) = self;
                    std::borrow::Cow::Borrowed(&edges[..])
                }

                fn source(&self, e: &Ed) -> Nd { e.0 }
                fn target(&self, e: &Ed) -> Nd { e.1 }
            }

            #[cfg(feature = "render_stm")]
            impl<'a> dot::Labeller<'a, Nd, Ed> for MachineEdges {
                fn graph_id(&'a self) -> dot::Id<'a> { dot::Id::new(stringify!($mod_name)).unwrap() }

                fn node_shape(&'a self, node: &Nd) -> Option<dot::LabelText<'a>> {
                    if &START_NODE_NAME==node {
                        Some(dot::LabelText::LabelStr("point".into()))
                    } else {
                        #[allow(unused_mut)]
                        let mut shape=Some(dot::LabelText::LabelStr("ellipse".into()));
                        if node==&stringify!($start) {
                            $( crate::stm!(@sub_end_filter $start_tag {
                                shape=Some(dot::LabelText::LabelStr("doublecircle".into()));
                            } ) )*
                        }
                        $(
                            if node==&stringify!($node) {
                                $( crate::stm!(@sub_end_filter $tag {
                                    shape=Some(dot::LabelText::LabelStr("doublecircle".into()));

                                } ) )*
                            }
                        )*
                        shape
                    }
                }

                fn node_id(&'a self, n: &Nd) -> dot::Id<'a> {
                    dot::Id::new(*n).unwrap()
                }

                #[allow(unused_mut, unused_variables)]
                fn node_label(&'a self, node: &Nd) -> dot::LabelText<'a> {
                    let mut last: Option<char>=None;
                    let mut rows=1.0;
                    let mut cols=0.0;
                    let mut name=String::new();
                    for ch in node.chars() {
                        if let Some(last)=last {
                            $(
                                if node==&stringify!($start) {
                                    $( crate::stm!(@sub_end_filter $tag {
                                        if last.is_lowercase() && ch.is_uppercase() && cols>3.0+1.25*rows {
                                            name.push('\n');

                                            rows+=1.0;
                                            cols=0.0;
                                        }
                                    } ) )*
                                }
                                if node==&stringify!($node) {
                                    $( crate::stm!(@sub_end_filter $tag {
                                        if last.is_lowercase() && ch.is_uppercase() && cols>3.0+1.25*rows {
                                            name.push('\n');

                                            rows+=1.0;
                                            cols=0.0;
                                        }
                                    } ) )*
                                }
                            )*
                        }

                        cols+=1.0;
                        name.push(ch);
                        last=Some(ch);
                    }

                    dot::LabelText::LabelStr(name.into())
                }

                fn edge_label(&'a self, (f, to): &Ed) -> dot::LabelText<'a> {
                    {
                        let dest_name=stringify!($start);
                        if &dest_name==to {
                            let mut edge_name=if START_NODE_NAME==*f {
                                String::from(format!("<TABLE BORDER=\"0\"><TR><TD><B><I> -&gt; {:?}</I></B></TD></TR>", to.replace("<", "&lt;").replace(">", "&gt;")))
                            } else {
                                String::from(format!("<TABLE BORDER=\"0\"><TR><TD><I>{:?} -&gt; {:?}</I></TD></TR>", f.replace("<", "&lt;").replace(">", "&gt;"), to.replace("<", "&lt;").replace(">", "&gt;")))
                            };
                            edge_name.push_str(""); //to avoid warning about edge_name not needing to be mutable
                            $(
                                let arg=stringify!($start_arg);
                                let arg_line=format!("<TR><TD>{}</TD></TR>", arg.replace("<", "&lt;").replace(">", "&gt;"));
                                (&mut edge_name).push_str(&arg_line);
                            )*;
                            return dot::LabelText::HtmlStr(format!("{}</TABLE>", edge_name).into())
                        }
                    }
                    $(
                        {
                            let dest_name=stringify!($node);
                            if &dest_name==to {
                                let mut edge_name=String::from(format!("<TABLE BORDER=\"0\"><TR><TD><I>{:?} -&gt; {:?}</I></TD></TR>", f.replace("<", "&lt;").replace(">", "&gt;"), to.replace("<", "&lt;").replace(">", "&gt;")));
                                edge_name.push_str(""); //to avoid warning about edge_name not needing to be mutable
                                $(
                                    let arg=stringify!($arg);
                                    let arg_line=format!("<TR><TD>{}</TD></TR>", arg.replace("<", "&lt;").replace(">", "&gt;"));
                                    (&mut edge_name).push_str(&arg_line);
                                )*;
                                return dot::LabelText::HtmlStr(format!("{}</TABLE>", edge_name).into())
                            }
                        }
                    )*;
                    dot::LabelText::EscStr("".into())
                }
            }

            mod dropper {
                #[derive(Debug)]
                pub struct $start{
                    _secret: ()
                }

                impl $start {
                    pub fn new() -> $start {
                        $start {
                            _secret: ()
                        }
                    }
                }
                
                $(
                    #[derive(Debug)]
                    pub struct $node {
                        _secret: ()
                    }

                    impl $node {
                        pub fn new() -> $node {
                            $node {
                                _secret: ()
                            }
                        }
                    }
                )*

                $(
                    impl From<$start_e> for $start {
                        fn from(mut old_st: $start_e) -> $start {
                            $start{
                                _secret: ()
                            }
                        }
                    }
                )*

                $(
                    $(
                        impl From<$e> for $node {
                            fn from(mut old_st: $e) -> $node {
                                $node{ _secret: ()}
                            }
                        }
                    )*
                )*
            }

            crate::stm!(@sub_build_enum (
                $start $(| $start_tag |)*,
                $($node $(| $tag |)*),*
            ) -> {
                pub enum $term_name<S,T> {
                }
            });
            
            crate::stm!(@sub_bare_dropper_enum $stripped_name, $start,$($node),*);
        }

        stm!(@sub_wall $machine_tag $mod_name, $enum_name, $start($($start_arg),*),$($node($($arg),*)),*);

        //stm!(@sub_enum $pertinence $mod_name, $term_name,
        //     $( crate::stm!(@sub_end_filter $start_tag {$start} ) ),*
        //     ($($($node $tag)*),*);

        /*
        impl Drop for $enum_name {
            fn drop(&mut self) {
                let finaliser=match self {
                    $enum_name::$start(st $(, stm!(@sub_pattern ($start_arg) _ ))*) => st.finaliser,
                    $(
                        $enum_name::$node(st $(, stm!(@sub_pattern ($arg) _))*) => st.finaliser,
                    )*
                };
                let _term=finaliser(self);
            }
        }            
         */
        
        impl std::fmt::Debug for $enum_name {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
                f.debug_struct(stringify!($enum_name))
                    .field("state", &self.state().to_string())
                    .finish()
            }
        }

        impl $enum_name {
            #[allow(unused_variables)]
            pub fn new(arg : ($($start_arg:ty),*), finaliser: Box<dyn FnOnce($mod_name::$stripped_name) -> $mod_name::$term_name>) -> $enum_name {
                let node=$mod_name::$start {
                    finaliser
                };
                
                $( crate::stm!{@sub_end_filter $start_tag
                               node.end_tags_found();
                } )*;

                $(
                    $( crate::stm!{@sub_end_filter $tag
                                   node.end_tags_found();
                    } )*
                )*;
                crate::stm!(@append_tuple arg, (0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15), ($($start_arg:ty),*) -> $enum_name::$start(node))
            }

            pub fn at_accepting_state(&self) -> bool {
                match self {
                    $enum_name::$start(st $(, stm!(@sub_pattern ($start_arg) _ ))*) =>
                        st.is_accepting_state(),
                    $(
                        $enum_name::$node(st $(, stm!(@sub_pattern ($arg) _))*) =>
                            st.is_accepting_state(),
                    )*
                }
            }

            #[allow(dead_code)]
            pub fn state(&self) -> &'static str {
                match self {
                    $enum_name::$start(_st $(, stm!(@sub_pattern ($start_arg) _ ))*) => stringify!($start),
                    $(
                        $enum_name::$node(_st $(, stm!(@sub_pattern ($arg) _))*) => stringify!($node),
                    )*
                }
            }

            #[allow(unused_variables)]
            pub fn render_to<W: std::io::Write>(output: &mut W) {
                #[cfg(feature = "render_stm")]
                {
                    let mut edge_vec=Vec::new();
                    edge_vec.push(($mod_name::START_NODE_NAME, stringify!($start)));
                    
                    $(
                        edge_vec.push({
                            let f=stringify!($start_e);
                            let t=stringify!($start);
                            (f,t)
                        });
                    )*;
                    
                    $(
                        $(
                            edge_vec.push({
                                let f=stringify!($e);
                                let t=stringify!($node);
                                (f,t)
                            });
                        )*
                    )*;
                        
                    let edges = $mod_name::MachineEdges(edge_vec);
                    dot::render(&edges, output).unwrap()
                }
            }
        }
    };

    (states $mod_name:ident, $enum_name:ident, $stripped_name:ident, $term_name:ident, [$($start_e:ident), *] => $start: ident $(| $start_tag:tt |)?, { $( [$($e:ident), +] => $node:ident $(| $tag:tt |)? );+ $(;)? } ) => {
        stm!(@private nowall ignorable $mod_name, $enum_name, $stripped_name, $term_name, ||{}, $term_name, [$($start_e),*] => $start() $(|$start_tag|)*, {
            $([$($e),*] => $node() $(|$tag|)*);* });
    };
    (machine $mod_name:ident, $enum_name:ident, $stripped_name:ident, $term_name:ident, [$($start_e:ident), *] => $start: ident($($start_arg:ty),*) $(| $start_tag:tt |)?, { $( [$($e:ident), +] => $node:ident($($arg:ty),*) $(| $tag:tt |)? );+ $(;)? } ) => {
        stm!(@private wall ignorable $mod_name, $enum_name, $stripped_name, $term_name, [$($start_e), *] => $start($($start_arg),*) $(| $start_tag|)*, { $( [$($e), *] => $node($($arg),*) $(|$tag|)* );* } );
    };
}
