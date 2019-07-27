use crate::{copyable, stm};
use std::collections::VecDeque;
use unicode_segmentation::UnicodeSegmentation;
use Machine::*;

#[derive(Debug, PartialEq)]
pub enum Error {
    InvalidGraphemeLength(ByteWidth),
    TokenTooLong,
    IllegalState,
}

#[derive(Debug)]
pub struct Dims(pub GlyphXCnt, pub GlyphYCnt);

impl Dims {
    pub fn width(&self) -> usize {
        (self.0).0
    }
}

const BREAKABLE: &str = "-_";
const SPACES: &str = " \t";

copyable!(ByteWidth, usize);
const ZERO_BYTES: ByteWidth = ByteWidth(0);
copyable!(GlyphXCnt, usize);
const ZERO_GLYPHS: GlyphXCnt = GlyphXCnt(0);
copyable!(GlyphYCnt, usize);

impl GlyphXCnt {
    pub fn is_none(&self) -> bool {
        self.0 == 0
    }
}

struct GlyphLayout {
    screen_widths: VecDeque<ByteWidth>,
    line_length: GlyphXCnt,
    last_line_offset: GlyphXCnt,
    last_line_bytes: ByteWidth,
}

impl GlyphLayout {
    pub fn new(line_length: GlyphXCnt) -> GlyphLayout {
        GlyphLayout {
            screen_widths: VecDeque::<ByteWidth>::new(),
            line_length,
            last_line_offset: ZERO_GLYPHS,
            last_line_bytes: ZERO_BYTES,
        }
    }

    pub fn partial_width(&self) -> GlyphXCnt {
        GlyphXCnt(self.last_line_offset.0)
    }

    pub fn width(&self) -> GlyphXCnt {
        self.line_length
    }

    pub fn reset(&mut self) {
        self.screen_widths.clear();
        self.last_line_offset = ZERO_GLYPHS;
        self.last_line_bytes = ZERO_BYTES;
    }

    pub fn add(&mut self, added_bytes: ByteWidth) -> Result<(), Error> {
        if added_bytes.0 == 0 {
            return Err(Error::InvalidGraphemeLength(added_bytes));
        }
        let new_bytes = ByteWidth(self.last_line_bytes.0 + added_bytes.0);
        let new_offset = GlyphXCnt(self.last_line_offset.0 + 1);
        if new_offset.0 % self.line_length.0 == 0 {
            self.screen_widths.push_back(new_bytes);
            self.last_line_bytes = ZERO_BYTES;
            self.last_line_offset = ZERO_GLYPHS;
        } else {
            self.last_line_bytes = new_bytes;
            self.last_line_offset = new_offset;
        }
        Ok(())
    }

    pub fn unshift_screen(&mut self) -> Option<ByteWidth> {
        self.screen_widths.pop_front()
    }

    pub fn is_multirow(&self) -> bool {
        self.screen_widths.len() > 0
    }

    pub fn next_length(&self) -> GlyphXCnt {
        if self.is_multirow() {
            self.line_length
        } else {
            GlyphXCnt(self.last_line_offset.0)
        }
    }

    pub fn fits(&self, c: GlyphXCnt) -> bool {
        if c.is_none() {
            true
        } else if self.is_multirow() {
            false
        } else {
            return self.last_line_offset.0 + c.0 as usize <= self.width().0;
        }
    }
}

struct SizedString {
    val: String,
    len: GlyphXCnt,
}

impl SizedString {
    pub fn new(val: String, len: GlyphXCnt) -> SizedString {
        SizedString { val, len }
    }
}

struct Pending {
    value: String,
    starting_spaces: String,
    layout: GlyphLayout,
}

enum ConsumptionState {
    Consumed(SizedString),
    Empty,
    TooLarge,
}

enum Placement {
    Assigned(ConsumptionState, GlyphXCnt),
    Invalid,
}

impl Pending {
    pub fn new(line_length: GlyphXCnt) -> Pending {
        Pending {
            value: String::new(),
            starting_spaces: String::with_capacity(10),
            layout: GlyphLayout::new(line_length),
        }
    }

    pub fn consume(&mut self, c: GlyphXCnt) -> ConsumptionState {
        //returns none when the token won't fit
        if c.is_none() {
            self.unshift_to_row_start()
        } else if self.layout.is_multirow() {
            //We've not at the start of a line and the current token is part of a multi-row token sequence so it won't fit
            ConsumptionState::TooLarge
        } else {
            let num_spaces: usize = self.starting_spaces.len();
            let total_len = GlyphXCnt(self.layout.next_length().0 + num_spaces);
            if ZERO_GLYPHS == total_len {
                return ConsumptionState::Empty;
            } else if self.layout.fits(GlyphXCnt(c.0 + num_spaces)) {
                let result = ConsumptionState::Consumed(SizedString {
                    val: self.starting_spaces.to_string() + &self.value,
                    len: GlyphXCnt(self.layout.partial_width().0 + num_spaces),
                });
                self.reset();
                result
            } else {
                //The current token won't fit. It's not part of multi-row token sequence and we're not at the start of a line.
                ConsumptionState::TooLarge
            }
        }
    }

    pub fn add_glyph(&mut self, new_glyph: &str) -> Result<(), Error> {
        if SPACES.contains(new_glyph) && self.value.len() == 0 {
            self.starting_spaces += " ";
        } else {
            self.value += new_glyph;
            self.layout.add(ByteWidth(new_glyph.len()))?;
        }
        Ok(())
    }

    fn reset(&mut self) {
        self.value.clear();
        self.starting_spaces.clear();
        self.layout.reset();
    }

    fn unshift_to_row_start(&mut self) -> ConsumptionState {
        if let Some(ByteWidth(next_screen_width_in_bytes)) = self.layout.unshift_screen() {
            let (screen_glyphs, new_value) = {
                let (screen_glyphs, new_value) =
                    self.value.split_at_mut(next_screen_width_in_bytes);
                (screen_glyphs.to_string(), new_value)
            };
            self.value = new_value.to_string();
            self.starting_spaces.clear();

            ConsumptionState::Consumed(SizedString::new(
                screen_glyphs.to_string(),
                self.layout.width(),
            ))
        } else {
            let result = self.value.clone();
            if result.len() == 0 {
                ConsumptionState::Empty
            } else {
                let length = self.layout.partial_width();
                self.reset();
                ConsumptionState::Consumed(SizedString::new(result, length))
            }
        }
    }
}

stm!(tokenising_stm, Machine, []=> Empty(), {
    [TokenComplete] => BuildingBreakable() |end|;
    [TokenComplete] => NotStartedBuildingNonBreakable() |end|;
    [Empty, NotStartedBuildingNonBreakable, TokenComplete] => StartedBuildingNonBreakable() |end|;
    [Empty, BuildingBreakable, StartedBuildingNonBreakable] => TokenComplete() |end|
});

pub struct LeftFormatter {
    size: Dims,
    all_splitters: String,
}

impl LeftFormatter {
    pub fn new(size: Dims) -> LeftFormatter {
        LeftFormatter {
            size,
            all_splitters: BREAKABLE.to_string() + SPACES,
        }
    }

    fn build_out(
        pending: &mut Pending,
        output: &mut Option<String>,
        col: &mut GlyphXCnt,
    ) -> Result<(), Error> {
        let mut new_col = *col;

        while let Placement::Assigned(
            ConsumptionState::Consumed(SizedString {
                val: tok,
                len: width,
            }),
            placement_col,
        ) = match pending.consume(new_col) {
            consumed @ ConsumptionState::Consumed(_) => Placement::Assigned(consumed, new_col),
            ConsumptionState::TooLarge => {
                if new_col != ZERO_GLYPHS {
                    let start_line_token = pending.consume(ZERO_GLYPHS);
                    match start_line_token {
                        ConsumptionState::Consumed(SizedString {
                            val: tok,
                            len: width,
                        }) => Placement::Assigned(
                            ConsumptionState::Consumed(SizedString {
                                val: "\n".to_owned() + &tok,
                                len: width,
                            }),
                            ZERO_GLYPHS,
                        ),
                        ConsumptionState::Empty => Placement::Invalid,
                        ConsumptionState::TooLarge => Err(Error::TokenTooLong)?,
                    }
                } else {
                    Err(Error::TokenTooLong)?
                }
            }
            ConsumptionState::Empty => Placement::Invalid,
        } {
            *output = if let Some(ref orig) = *output {
                Some(orig.to_owned() + &tok)
            } else {
                Some(tok)
            };
            new_col = GlyphXCnt(placement_col.0 + width.0);
        }
        *col = new_col;

        Ok(())
    }

    pub fn just_lines(&self, unformatted: &str) -> Result<Vec<String>, Error> {
        Ok(unformatted
            .lines()
            .map(|l| {
                let mut mach = Empty(tokenising_stm::Empty);
                let mut col = GlyphXCnt(0);
                let mut output = None;
                let mut pending = Pending::new(GlyphXCnt(self.size.width()));

                let graphemes = l.graphemes(true).collect::<Vec<&str>>();
                for grapheme in graphemes {
                    loop {
                        mach = match mach {
                            Empty(st) => {
                                col = GlyphXCnt(0);
                                if self.all_splitters.contains(grapheme) {
                                    TokenComplete(st.into())
                                } else {
                                    pending.add_glyph(grapheme)?; //pending start
                                    StartedBuildingNonBreakable(st.into())
                                }
                            }
                            BuildingBreakable(st) => TokenComplete(st.into()),
                            StartedBuildingNonBreakable(st) => {
                                if self.all_splitters.contains(grapheme) {
                                    TokenComplete(st.into())
                                } else {
                                    pending.add_glyph(grapheme)?; //pending start
                                    StartedBuildingNonBreakable(st)
                                }
                            }
                            NotStartedBuildingNonBreakable(st) => {
                                pending.add_glyph(grapheme)?; //pending start if grapheme is not a space
                                if SPACES.contains(grapheme) {
                                    NotStartedBuildingNonBreakable(st)
                                } else {
                                    StartedBuildingNonBreakable(st.into())
                                }
                            }
                            TokenComplete(st) => {
                                LeftFormatter::build_out(&mut pending, &mut output, &mut col)?;
                                pending.add_glyph(grapheme)?; //pending start
                                if BREAKABLE.contains(grapheme) {
                                    BuildingBreakable(st.into())
                                } else if SPACES.contains(grapheme) {
                                    NotStartedBuildingNonBreakable(st.into()) //pending start if grapheme is not a space
                                } else {
                                    StartedBuildingNonBreakable(st.into()) //pending start if grapheme is not a space
                                }
                            }
                        };
                        if let &TokenComplete(_) = &mach {
                        } else {
                            break;
                        }
                    }
                }
                LeftFormatter::build_out(&mut pending, &mut output, &mut col)?;

                if let Some(inner) = output {
                    Ok(inner)
                } else {
                    Ok(String::new())
                }
            })
            .collect::<Result<Vec<String>, Error>>()?
            .iter()
            .map(|string_ref| string_ref.to_string())
            .collect::<Vec<String>>())
    }

    pub fn just(&self, unformatted: &str) -> Result<String, Error> {
        Ok(self.just_lines(unformatted)?.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use crate::formatter::{Dims, LeftFormatter};
    #[test]
    fn just() {
        let f = LeftFormatter::new(Dims(5, 15));
        assert_eq!(f.just("foo blah"), Ok("foo\nblah".to_string()));
        assert_eq!(f.just("foo bla-h"), Ok("foo\nbla-h".to_string()));
        assert_eq!(f.just("foo bla h"), Ok("foo\nbla h".to_string()));
        assert_eq!(f.just("foo bl--h"), Ok("foo\nbl--h".to_string()));
        assert_eq!(f.just("foo bl  h"), Ok("foo\nbl  h".to_string()));
        assert_eq!(f.just("fo bl123456h"), Ok("fo\nbl123\n456h".to_string()));
        assert_eq!(f.just("fo  bl123456h"), Ok("fo\nbl123\n456h".to_string()));
        assert_eq!(f.just("fo-bl123456h"), Ok("fo-\nbl123\n456h".to_string()));
        assert_eq!(f.just("fo--bl123456h"), Ok("fo--\nbl123\n456h".to_string()));
        assert_eq!(f.just(" bl123456h"), Ok("bl123\n456h".to_string()));
        assert_eq!(
            f.just("fo----bl123456h"),
            Ok("fo---\n-\nbl123\n456h".to_string())
        );
        assert_eq!(
            f.just("fo-bl123456hfar"),
            Ok("fo-\nbl123\n456hf\nar".to_string())
        );
        assert_eq!(f.just("     bl123456h"), Ok("bl123\n456h".to_string()));
        assert_eq!(
            f.just("abc -52 123456"),
            Ok("abc\n-52\n12345\n6".to_string())
        );
        assert_eq!(
            f.just(" -52456 123456"),
            Ok("-5245\n6\n12345\n6".to_string())
        );
        assert_eq!(f.just(""), Ok("".to_string()));
        assert_eq!(f.just(" "), Ok("".to_string()));
        assert_eq!(f.just("  "), Ok("".to_string()));
        assert_eq!(f.just("     "), Ok("".to_string()));
        assert_eq!(f.just("      "), Ok("".to_string()));
        assert_eq!(f.just("     a"), Ok("a".to_string()));
        assert_eq!(f.just("      a"), Ok("a".to_string()));
        assert_eq!(f.just("ab     a"), Ok("ab\na".to_string()));
        assert_eq!(f.just("ab      a"), Ok("ab\na".to_string()));
        assert_eq!(f.just("     abcdef"), Ok("abcde\nf".to_string()));
        assert_eq!(f.just("      abcdef"), Ok("abcde\nf".to_string()));
        assert_eq!(f.just("ab     abcdef"), Ok("ab\nabcde\nf".to_string()));
        assert_eq!(f.just("ab      abcdef"), Ok("ab\nabcde\nf".to_string()));
        assert_eq!(f.just("     a"), Ok("a".to_string()));
        assert_eq!(f.just("      a"), Ok("a".to_string()));
        assert_eq!(f.just("abcdef     a"), Ok("abcde\nf\na".to_string()));
        assert_eq!(f.just("abcdef      a"), Ok("abcde\nf\na".to_string()));
        assert_eq!(f.just("-"), Ok("-".to_string()));
        assert_eq!(f.just("--"), Ok("--".to_string()));
        assert_eq!(f.just("-----"), Ok("-----".to_string()));
        assert_eq!(f.just("------"), Ok("-----\n-".to_string()));
        assert_eq!(f.just("-----a"), Ok("-----\na".to_string()));
        assert_eq!(f.just("------a"), Ok("-----\n-a".to_string()));
        assert_eq!(f.just("ab-----a"), Ok("ab---\n--a".to_string()));
        assert_eq!(f.just("ab------a"), Ok("ab---\n---a".to_string()));
        assert_eq!(f.just("-----abcdef"), Ok("-----\nabcde\nf".to_string()));
        assert_eq!(f.just("------abcdef"), Ok("-----\n-\nabcde\nf".to_string()));
        assert_eq!(
            f.just("ab-----abcdef"),
            Ok("ab---\n--\nabcde\nf".to_string())
        );
        assert_eq!(
            f.just("ab------abcdef"),
            Ok("ab---\n---\nabcde\nf".to_string())
        );
        assert_eq!(f.just("-----a"), Ok("-----\na".to_string()));
        assert_eq!(f.just("------a"), Ok("-----\n-a".to_string()));
        assert_eq!(f.just("abcdef-----a"), Ok("abcde\nf----\n-a".to_string()));
        assert_eq!(f.just("abcdef------a"), Ok("abcde\nf----\n--a".to_string()));

        assert_eq!(
            f.just(
                "foo blah
foo bla-h
foo bla h
foo bl--h
foo bl  h
fo bl123456h
fo  bl123456h
fo-bl123456h
fo--bl123456h
 bl123456h

 
     
      "
            ),
            Ok("foo
blah
foo
bla-h
foo
bla h
foo
bl--h
foo
bl  h
fo
bl123
456h
fo
bl123
456h
fo-
bl123
456h
fo--
bl123
456h
bl123
456h



"
            .to_string())
        );
    }
}
