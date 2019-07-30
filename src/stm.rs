#[macro_export]
macro_rules! stm {
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
    (@sub_unending_mask ignorable $($sub:tt)*) => {$($sub)*};
    (@sub_unending_mask unending $($sub:tt)*) => {};
    (@sub_unending_mask attention_seeking $($sub:tt)*) => {$($sub)*};
    (@sub_unending_filter ignorable $($sub:tt)*) => {};
    (@sub_unending_filter unending $($sub:tt)*) => {$($sub)*};
    (@sub_unending_filter attention_seeking $($sub:tt)*) => {};
    (@sub_attention_seeking_filter ignorable $($sub:tt)*) => {};
    (@sub_attention_seeking_filter unending $($sub:tt)*) => {};
    (@sub_attention_seeking_filter attention_seeking $($sub:tt)*) => {$($sub)*};
    (@sub_end_filter end $($sub:tt)*) => {$($sub)*};
    (@sub_pattern $_t:tt $sub:pat) => {$sub};
    (@private $machine_tag:tt $pertinence:tt $mod_name:ident, $enum_name:ident, [$($start_e:ident), *] => $start: ident($($start_arg:ty),*) $(| $start_tag:tt |)?, { $( [$($e:ident), +] => $node:ident($($arg:ty),*) $(| $tag:tt |)? );+ $(;)? } ) => {

        mod $mod_name
        {
            trait Quittable {
                fn terminate(&mut self);
            }
            
            #[derive(Debug)]
            pub struct $start {
                term: bool
            }

            $(
                impl From<$start_e> for $start {
                    fn from(mut old_st: $start_e) -> $start {
                        old_st.term=true;
                        println!("{:?} -> {:?}", stringify!($start_e), stringify!($start));
                        let mut new_st=$start::inst();
                        new_st.term=false;
                        new_st
                    }
                }

            )*

            impl $start {
                pub fn inst() -> $start {
                    let node=$start {
                        term: false
                    };
                    crate::stm!{@sub_unending_mask $pertinence

                                $( crate::stm!{@sub_end_filter $start_tag
                                               node.end_tags_found();
                                } )*

                                $(
                                    $( crate::stm!{@sub_end_filter $tag
                                                   node.end_tags_found();
                                    } )*
                                )*
                                
                    }
                    node
                }

                pub fn terminable(&self) -> bool {
                    self.term
                }

                crate::stm!{@sub_unending_mask $pertinence
                            fn end_tags_found(&self){}
                }

                crate::stm!{@sub_attention_seeking_filter $pertinence
                            pub fn ack_inst<E>(self) -> E where E: From<$start> {
                                self.into()
                            }
                }

            }
            
            $( crate::stm!{@sub_end_filter $start_tag
                           impl Quittable for $start {
                               fn terminate(&mut self) {
                                   self.term=true;
                               }
                           }

            } )*
                

            impl Drop for $start {
                fn drop(&mut self) {
                    if !self.terminable() {debug_assert!(false, "unable to drop state: {:?}", self)}
                }
            }

            crate::stm!{@sub_unending_mask $pertinence
            $( crate::stm!{@sub_end_filter $start_tag
                impl $start {
                    pub fn droppable_inst(mut self) -> $start {
                        self.term=true;
                        
                        $start {
                            term: true
                        }
                    }

                }

            } )*
            }

            crate::stm!{@sub_unending_filter $pertinence
                        $( crate::stm!{@sub_end_filter $start_tag
                                       end_tag_in_unending_stm_error {}
                        }
                        )*
            }

            $(
                #[derive(Debug)]
                pub struct $node {
                    term: bool
                }

                    $( crate::stm!{@sub_end_filter $tag
                                   impl Quittable for $node {
                                       fn terminate(&mut self) {
                                           self.term=true;
                                       }
                                   }
                    } )*
                                
                $(
                    impl From<$e> for $node {
                        fn from(mut old_st: $e) -> $node {
                            old_st.term=true;
                            println!("{:?} -> {:?}", stringify!($e), stringify!($node));
                            $node {
                                term: false
                            }
                        }
                    }
                )*

                impl $node {
                    pub fn terminable(&self) -> bool {
                        self.term
                    }

                }

                impl Drop for $node {
                    fn drop(&mut self) {
                        if !self.terminable() {debug_assert!(false, "unable to drop state: {:?}", self)}
                    }
                }

            crate::stm!{@sub_unending_mask $pertinence
                $( crate::stm!{@sub_end_filter $tag
                    impl $node {
                        pub fn droppable_inst(mut self) -> $node {
                            self.term=true;
                            
                            $node {
                                term: true
                            }
                        }
                    }
                 } )*
            }

                crate::stm!{@sub_unending_filter $pertinence
                            $( crate::stm!{@sub_end_filter $tag
                                           illegal_end_tag {}
                            }
                            )*
            }
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
        }

        stm!(@sub_wall $machine_tag $mod_name, $enum_name, $start($($start_arg),*),$($node($($arg),*)),*);

        impl $enum_name {
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
                    )*

                    $(
                        $(
                            edge_vec.push({
                                let f=stringify!($e);
                                let t=stringify!($node);
                                (f,t)
                            });
                        )*
                    )*

                    let edges = $mod_name::MachineEdges(edge_vec);
                    dot::render(&edges, output).unwrap()
                }
            }
        }
    };
    (states ignorable $mod_name:ident, $enum_name:ident, [$($start_e:ident), *] => $start: ident $(| $start_tag:tt |)?, { $( [$($e:ident), +] => $node:ident $(| $tag:tt |)? );+ $(;)? } ) => {
        stm!(@private nowall ignorable $mod_name, $enum_name, [$($start_e),*] => $start() $(|$start_tag|)*, {
            $([$($e),*] => $node() $(|$tag|)*);* });
    };
    (machine ignorable $mod_name:ident, $enum_name:ident, [$($start_e:ident), *] => $start: ident($($start_arg:ty),*) $(| $start_tag:tt |)?, { $( [$($e:ident), +] => $node:ident($($arg:ty),*) $(| $tag:tt |)? );+ $(;)? } ) => {
        stm!(@private wall ignorable $mod_name, $enum_name, [$($start_e), *] => $start($($start_arg),*) $(| $start_tag|)*, { $( [$($e), *] => $node($($arg),*) $(|$tag|)* );* } );
    };
    (states attention_seeking $mod_name:ident, $enum_name:ident, [$($start_e:ident), *] => $start: ident $(| $start_tag:tt |)?, { $( [$($e:ident), +] => $node:ident $(| $tag:tt |)? );+ $(;)? } ) => {
        stm!(@private nowall attention_seeking $mod_name, $enum_name, [$($start_e),*] => $start() $(|$start_tag|)*, {
            $([$($e),*] => $node() $(|$tag|)*);* });
    };
    (machine attention_seeking $mod_name:ident, $enum_name:ident, [$($start_e:ident), *] => $start: ident($($start_arg:ty),*) $(| $start_tag:tt |)?, { $( [$($e:ident), +] => $node:ident($($arg:ty),*) $(| $tag:tt |)? );+ $(;)? } ) => {
        stm!(@private wall attention_seeking $mod_name, $enum_name, [$($start_e), *] => $start($($start_arg),*) $(| $start_tag|)*, { $( [$($e), *] => $node($($arg),*) $(|$tag|)* );* } );
    };
    (states unending $mod_name:ident, $enum_name:ident, [$($start_e:ident), *] => $start: ident $(| $start_tag:tt |)?, { $( [$($e:ident), +] => $node:ident $(| $tag:tt |)? );+ $(;)? } ) => {
        stm!(@private nowall unending $mod_name, $enum_name, [$($start_e),*] => $start() $(|$start_tag|)*, {
            $([$($e),*] => $node() $(|$tag|)*);* });
    };
    (machine unending $mod_name:ident, $enum_name:ident, [$($start_e:ident), *] => $start: ident($($start_arg:ty),*) $(| $start_tag:tt |)?, { $( [$($e:ident), +] => $node:ident($($arg:ty),*) $(| $tag:tt |)? );+ $(;)? } ) => {
        stm!(@private wall unending $mod_name, $enum_name, [$($start_e), *] => $start($($start_arg),*) $(| $start_tag|)*, { $( [$($e), *] => $node($($arg),*) $(|$tag|)* );* } );
    };
}
