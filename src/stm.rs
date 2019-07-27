#[macro_export]
macro_rules! stm {
    (@sub_pattern $_t:tt $sub:pat) => {$sub};
    ($mod_name:ident, $enum_name:ident, [$($start_e:ident), *] => $start: ident($($start_arg:ty),*), { $( [$($e:ident), +] => $node:ident($($arg:ty),*) );+ $(;)? } ) => {

        pub mod $mod_name
        {
            #[derive(Debug)]
            pub struct $start;

            $(
                impl From<$start_e> for $start {
                    fn from(_st: $start_e) -> $start {
                        println!("{:?} -> {:?}", stringify!($start_e), stringify!($start));
                        $start
                    }
                }
            )*

            $(
                #[derive(Debug)]
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
                        Some(dot::LabelText::LabelStr("ellipse".into()))
                    }
                }

                fn node_id(&'a self, n: &Nd) -> dot::Id<'a> {
                    dot::Id::new(*n).unwrap()
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

        pub enum $enum_name {
            $start($mod_name::$start $(, $start_arg)*),
            $(
                $node($mod_name::$node $(, $arg )*),
            )*
        }

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
}
