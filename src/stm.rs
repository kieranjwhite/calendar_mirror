/// Create a state machine
///
/// # Examples
///
/// ```
/// use reqwest;
/// use std::path::PathBuf;
/// use ExampleMach::*;
///
/// pub struct RefreshToken(String);
///
/// pub enum Event {
///    Refresh,
///    Quit,
/// }
///
/// pub struct Appointment {
///    ...
/// }
///
/// stm!(machine
///    example_stm, ExampleMach, StatesAtEnd, AcceptingStates,
///    [ Resetting ] => LoadingConfig(PathBuf), {
///    [ LoadingConfig, PollEvents ] => Retrieving(RefreshToken);
///    [ Retrieving ] => PollEvents(RefreshToken);
///    [ PollEvents ]  => Quitting() |end|;
///    [ LoadingConfig, Retrieving ] => Error(String) |end|;
///    }
/// );
///
/// fn run() {
///    let mut mach = ExampleMach::new((PathBuf::from("/var/opt/calendar_mirror/refresh.json"),),
///       Box::new(|end| {
///           match end {
///               StatesAtEnd::Quitting(st) => AcceptingStates::Quitting(st),
///               StatesAtEnd::Error(st) => AcceptingStates::Error(st),
///               _ => panic!("Not in an accepting state: {:?}", end),
///           }
///       })
///    );
///
///    loop {
///       mach = match mach {
///          LoadingConfig(st, path) =>
///             match load_config(&path) {
///                Ok(refresh_token) => Retrieving(st.into(), refresh_token),
///                Err(error) => Error(st.into(), "Failed to load config"),
///             },
///          Retrieving(st, refresh_token) => {
///             match download_appointments(&refresh_token) {
///                Ok(apps) => PollEvents(st.into(), apps),
///                Err(error) => Error(st.into(), "Failed to download appointments"),
///             },
///          },
///          PollEvents(st, refresh_token) =>
///             match read_event() {
///                Refresh => Retrieving(st.into(), refresh_token),
///                Quit => Quitting(st.into()),
///             },
///          Quitting(st) => break,
///          Error(st, message) => {
///             eprintln!(message);
///             break;
///          }
///       }
///    }
/// }
///
/// fn load_config(config: &Path) -> Result<RefreshToken, IO::Error> {
///    ...
/// }
///
/// fn download_appointments(refresh_token: &RefreshToken) ->
///    Result<Vec<Appointment>, reqwest::Error> {
///    ...
/// }
///
/// fn read_event() -> Event {
///    ...
/// }
/// ```
#[macro_export]
macro_rules! stm {
    (@widen_enum_variant noargs, $tuple:expr, ($($idx:tt),*), ($($arg:tt),* $(,)?) -> $enum_name:ident :: $start:ident ($($comp:expr),*)) => {
        $enum_name :: $start($($comp),*)
    };
    (@widen_enum_variant args, $tuple:expr, ($($idx:expr),*), () -> $enum_name:ident :: $start:ident ($($comp:expr),*)) => {
        $enum_name :: $start ($($comp),*)
    };
    (@widen_enum_variant args, $tuple:expr, ($head_idx:tt, $($idx:tt),*), ($head:tt $(, $arg:tt)* $(,)?) -> $enum_name:ident :: $start:ident ()) => {
        crate::stm!(@widen_enum_variant args, $tuple,  ($($idx),*), ($($arg),*) -> $enum_name :: $start ( $tuple.$head_idx ))
    };
    (@widen_enum_variant args, $tuple:expr, ($head_idx:tt, $($idx:tt),*), ($head:tt $(, $arg:tt)* $(,)?) -> $enum_name:ident :: $start:ident ($($comp:expr),+)) => {
        crate::stm!(@widen_enum_variant args, $tuple, ($($idx),*), ($($arg),*) -> $enum_name :: $start ( $($comp),*, $tuple.$head_idx ))
    };
    (@sub_build_enum () -> { pub enum $enum_name:ident {$($processed_var:ident(dropper::$processed:ident)),*}}) => {
        #[derive(Debug)]
        pub enum $enum_name {
            $($processed_var(dropper::$processed)),*
        }
    };
    (@sub_build_enum ($head:tt |end| $(, $tail:ident $(| $tag:ident |)?)*)-> { pub enum $enum_name:ident { } }) => {
        crate::stm!(@sub_build_enum ($($tail $(| $tag |)*),*) -> {
            pub enum $enum_name {
                $head(dropper::$head)
            }
        });
    };
    (@sub_build_enum ($head:tt |end| $(, $tail:ident $(| $tag:ident |)?)*)-> { pub enum $enum_name:ident { $($processed_var:ident( dropper::$processed:ident)),+} }) => {
        crate::stm!(@sub_build_enum ($($tail $(| $tag |)*),*) -> {
            pub enum $enum_name {
                $($processed_var(dropper::$processed)),*,
                $head(dropper::$head)
            }
        });
    };
    (@sub_build_enum ($head:tt $(, $tail:ident $(| $tag:ident |)?)*)-> { pub enum $enum_name:ident { $($processed_var:ident( dropper::$processed:ident)),*} }) => {
        crate::stm!(@sub_build_enum ($($tail $(| $tag |)*),*) -> {
            pub enum $enum_name {
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
    (@sub_wall nowall $stripped_name:ident $term_name:ident $mod_name:ident, $enum_name:ident, $($var:ident($($arg:ty),*)),*) => {
        #[allow(dead_code)]
        pub enum $enum_name {
            $(
                $var($mod_name::$var $(, $arg )*),
            )*
        }
    };
    (@sub_wall wall $stripped_name:ident $term_name:ident $mod_name:ident, $enum_name:ident, $($var:ident($($arg:ty),*)),*) => {
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

    (@insert_tuple_params noargs, ($($start_arg:ty),*)) => (
        ($($start_arg),*)
    );
    (@insert_tuple_params args, ($($start_arg:ty),*)) => (
        ($($start_arg),*,)
    );

    (@private $machine_tag:tt $pertinence:tt $mod_name:ident, $enum_name:ident, $stripped_name:ident, $term_name:ident, [$($start_e:ident), *] => $start_trailing:tt $start: ident($($start_arg:ty),* ,) $(| $start_tag:tt |)?, { $( [$($e:ident), +] => $node:ident($($arg:ty),*) $(| $tag:tt |)? );+ $(;)? } ) => {

        use $mod_name::$term_name;
        use $mod_name::$stripped_name;

        mod $mod_name
        {
            pub struct $start {
                pub finaliser: Option<Box<dyn FnOnce($stripped_name) -> $term_name>>
            }

            impl Drop for $start {
                fn drop(&mut self) {
                    if let Some(finaliser)=self.finaliser.take() {
                        let _term=(finaliser)($stripped_name::$start(dropper::$start::new()));
                    }
                }
            }

            $(
                impl From<$start_e> for $start {
                    fn from(mut old_st: $start_e) -> $start {
                        use log::trace;

                        trace!("{:?} -> {:?}", stringify!($start_e), stringify!($start));
                        $start {
                            finaliser: old_st.finaliser.take()
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
                pub fn end_tags_found(&self){}

                #[allow(dead_code, unreachable_code)]
                pub fn is_accepting_state(&self) -> bool {
                    $( crate::stm!{@sub_end_filter $start_tag
                                   return true;
                    } )*
                    return false;
                }

            }

            $(
                impl std::fmt::Debug for $node {
                    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
                        f.debug_struct(stringify!($node))
                            .finish()
                    }
                }

                pub struct $node {
                    pub finaliser: Option<Box<dyn FnOnce($stripped_name) -> $term_name>>,
                    _secret: ()
                }

                impl $node {
                    #[allow(dead_code, unreachable_code)]
                    pub fn is_accepting_state(&self) -> bool {
                        $( crate::stm!{@sub_end_filter $tag
                                       return true;
                        } )*;
                        return false;
                    }
                }

                impl Drop for $node {
                    fn drop(&mut self) {
                        if let Some(finaliser)=self.finaliser.take() {
                            let _term=(finaliser)($stripped_name::$node(dropper::$node::new()));
                        }
                    }
                }

                $(
                    impl From<$e> for $node {
                        fn from(mut old_st: $e) -> $node {
                            println!("{:?} -> {:?}", stringify!($e), stringify!($node));
                            $node {
                                finaliser: old_st.finaliser.take(),
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

                    #[allow(dead_code, unreachable_code)]
                    pub fn is_accepting_state(&self) -> bool {
                        $( crate::stm!{@sub_end_filter $start_tag
                                       return true;
                        } )*
                        return false;
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

                        #[allow(dead_code, unreachable_code)]
                        pub fn is_accepting_state(&self) -> bool {
                            $( crate::stm!{@sub_end_filter $tag
                                           return true;
                            } )*;
                            return false;
                        }
                    }
                )*

                $(
                    impl From<$start_e> for $start {
                        fn from(_old_st: $start_e) -> $start {
                            $start{
                                _secret: ()
                            }
                        }
                    }
                )*

                $(
                    $(
                        impl From<$e> for $node {
                            fn from(_old_st: $e) -> $node {
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
                pub enum $term_name {
                }
            });

            crate::stm!(@sub_bare_dropper_enum $stripped_name, $start,$($node),*);
        }

        stm!(@sub_wall $machine_tag $stripped_name $term_name $mod_name, $enum_name, $start($($start_arg),*),$($node($($arg),*)),*);

        impl $stripped_name {
            #[allow(dead_code)]
            pub fn at_accepting_state(&self) -> bool {
                match self {
                    $stripped_name::$start(st) =>
                        st.is_accepting_state(),
                    $(
                        $stripped_name::$node(st) =>
                            st.is_accepting_state(),
                    )*
                }
            }
        }

        impl std::fmt::Debug for $enum_name {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
                f.debug_struct(stringify!($enum_name))
                    .field("state", &self.state().to_string())
                    .finish()
            }
        }

        impl $enum_name {
            #[allow(unused_variables)]
            pub fn new(arg : crate::stm!(@insert_tuple_params $start_trailing, ($($start_arg),*)), finaliser: Box<dyn FnOnce($mod_name::$stripped_name) -> $mod_name::$term_name>) -> $enum_name {
                let node=$mod_name::$start {
                    finaliser: Some(finaliser)
                };

                $( crate::stm!{@sub_end_filter $start_tag
                               node.end_tags_found();
                } )*;

                $(
                    $( crate::stm!{@sub_end_filter $tag
                                   node.end_tags_found();
                    } )*
                )*;

                crate::stm!(@widen_enum_variant $start_trailing, arg, (0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31), ($($start_arg,)*) -> $enum_name::$start(node))
            }

            #[allow(dead_code)]
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
        stm!(@private nowall ignorable $mod_name, $enum_name, $stripped_name, $term_name, ||{}, $term_name, [$($start_e),*] => noargs $start( , ) $(|$start_tag|)*, {
            $([$($e),*] => $node() $(|$tag|)*);* });
    };
    (machine $mod_name:ident, $enum_name:ident, $stripped_name:ident, $term_name:ident, [$($start_e:ident), *] => $start: ident($($start_arg:ty),+  $(,)?) $(| $start_tag:tt |)?, { $( [$($e:ident), +] => $node:ident($($arg:ty),*) $(| $tag:tt |)? );+ $(;)? } ) => {
        stm!(@private wall ignorable $mod_name, $enum_name, $stripped_name, $term_name, [$($start_e), *] => args $start($($start_arg),*,) $(| $start_tag|)*, { $( [$($e), *] => $node($($arg),*) $(|$tag|)* );* } );
    };
    (machine $mod_name:ident, $enum_name:ident, $stripped_name:ident, $term_name:ident, [$($start_e:ident), *] => $start: ident() $(| $start_tag:tt |)?, { $( [$($e:ident), +] => $node:ident($($arg:ty),*) $(| $tag:tt |)? );+ $(;)? } ) => {
        stm!(@private wall ignorable $mod_name, $enum_name, $stripped_name, $term_name, [$($start_e), *] => noargs $start( , ) $(| $start_tag|)*, { $( [$($e), *] => $node($($arg),*) $(|$tag|)* );* } );
    };
}
