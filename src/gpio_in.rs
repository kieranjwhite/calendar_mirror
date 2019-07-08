use crate::{err, stm};
use memmap::{Mmap, MmapOptions};
use std::{
    fs::File,
    io,
    ptr::read_volatile,
    time::{Duration, Instant},
};
use LongPressMachine::*;

const BLOCK_SIZE: usize = 4 * 1024;
const PIN_COUNT: usize = 28;
const READ_REG_OFFSET: usize = 13;

pub const SW1_GPIO: usize = 16;
pub const SW2_GPIO: usize = 26;
pub const SW3_GPIO: usize = 20;
pub const SW4_GPIO: usize = 21;

err!(Error {
    File(io::Error),
    InvalidPin(Pin)
});

pub trait Button<E> {
    fn event(&mut self, ports: &mut GPIO) -> Result<Option<E>, Error>;
}

stm!(long_press_button_stm, LongPressMachine, [ReleasePending, PressedPending, LongPressed] => NotPressed(), {
    [PressedPending, LongPressed] => ReleasePending();
    [NotPressed, ReleasePending, LongPressed] => PressedPending();
    [NotPressed, ReleasePending, PressedPending]=>LongPressed()
});

#[derive(Eq, PartialEq)]
pub enum LongButtonEvent {
    Pressed,
    LongPress,
    Release,
    PressAndRelease,
}

impl LongButtonEvent {
    pub fn is_short_press(&self) -> bool {
        match self {
            LongButtonEvent::Pressed => true,
            LongButtonEvent::PressAndRelease => true,
            _ => false,
        }
    }

    pub fn is_long_press(&self) -> bool {
        match self {
            LongButtonEvent::LongPress => true,
            _ => false,
        }
    }

    pub fn is_release(&self) -> bool {
        match self {
            LongButtonEvent::Release => true,
            LongButtonEvent::PressAndRelease => true,
            _ => false,
        }
    }
}

pub struct DetectableDuration(pub Duration);
pub struct LongReleaseDuration(pub Duration);

// A RepeatableButton (not implemented) would return a press event for
// each PressDuration that the button is pressed.  A relase event
// would be returned immediately after button release.

// A LongPressButton returns a short press if released within
// LongDuration or a long press immediately after a LongDuration press
// (without waiting for release). It returns a release event after
// ReleaseDuration passed after button release
pub struct LongPressButton {
    pin: Pin,
    state: LongPressMachine,
    detectable_after: DetectableDuration,
    long_release_after: LongReleaseDuration,
}

impl LongPressButton {
    pub fn new(
        pin: Pin,
        detectable_after: DetectableDuration,
        long_release_after: LongReleaseDuration,
    ) -> LongPressButton {
        LongPressButton {
            pin,
            detectable_after,
            long_release_after,
            state: LongPressMachine::NotPressed(long_press_button_stm::NotPressed),
        }
    }
}

impl Button<LongButtonEvent> for LongPressButton {
    fn event(&mut self, ports: &mut GPIO) -> Result<Option<LongButtonEvent>, Error> {
        let (pressing, duration) = ports.pinin(&self.pin)?;
        let mut event: Option<LongButtonEvent> = None;
        use std::mem::replace;
        let mut state = replace(
            &mut self.state,
            NotPressed(long_press_button_stm::NotPressed),
        );

        state = match state {
            NotPressed(st) => {
                if !pressing {
                    NotPressed(st)
                } else {
                    if duration < self.detectable_after.0 {
                        PressedPending(st.into())
                    } else {
                        event = Some(LongButtonEvent::LongPress);
                        LongPressed(st.into())
                    }
                }
            }
            ReleasePending(st) => {
                if !pressing {
                    if duration < self.long_release_after.0 {
                        ReleasePending(st)
                    } else {
                        event = Some(LongButtonEvent::Release);
                        NotPressed(st.into())
                    }
                } else {
                    if duration < self.detectable_after.0 {
                        PressedPending(st.into())
                    } else {
                        event = Some(LongButtonEvent::LongPress);
                        LongPressed(st.into())
                    }
                }
            }
            PressedPending(st) => {
                if !pressing {
                    if duration < self.long_release_after.0 {
                        event = Some(LongButtonEvent::Pressed);
                        ReleasePending(st.into())
                    } else {
                        event = Some(LongButtonEvent::PressAndRelease);
                        NotPressed(st.into())
                    }
                } else {
                    if duration < self.detectable_after.0 {
                        PressedPending(st)
                    } else {
                        event = Some(LongButtonEvent::LongPress);
                        LongPressed(st.into())
                    }
                }
            }
            LongPressed(st) => {
                if !pressing {
                    if duration < self.long_release_after.0 {
                        ReleasePending(st.into())
                    } else {
                        event = Some(LongButtonEvent::Release);
                        NotPressed(st.into())
                    }
                } else {
                    if duration < self.detectable_after.0 {
                        PressedPending(st.into())
                    } else {
                        LongPressed(st)
                    }
                }
            }
        };

        replace(&mut self.state, state);
        Ok(event)
    }
}

#[derive(Clone, Debug)]
pub struct Pin(pub usize);

pub struct GPIO {
    map: Mmap,
    snap: [(bool, Instant); PIN_COUNT],
}

impl GPIO {
    pub fn new() -> Result<GPIO, Error> {
        let f = File::open("/dev/gpiomem")?;

        // Reason that map function is unsafe:
        // its return type Mmap implements the trait deref<Target=[u8]> which exposes
        // references to referents that can mutated without the
        // knowledge of the
        // borrow checker. By wrapping in an unsafe block we are committing to
        // preventing exposure of these references. We do allow access to the underlying
        // data via a copy type (bools) but this should be okay.
        let mmap = unsafe { MmapOptions::new().len(BLOCK_SIZE).map(&f)? };

        let t = Instant::now();
        let mut instance = GPIO {
            map: mmap,
            snap: [(false, t.clone()); PIN_COUNT],
        };

        let val = instance.value();
        let mut gpio_num: usize = 0;
        while gpio_num < PIN_COUNT {
            instance.snap[gpio_num] = (GPIO::bit(val, &Pin(gpio_num)), t.clone());
            gpio_num += 1;
        }

        Ok(instance)
    }

    fn value(&self) -> u32 {
        // The following block is unsafe because (1) read_volatile
        // dereferences a pointer, and undefined behaviour can arise
        // if that pointer is not valid (see
        // https://doc.rust-lang.org/std/ptr/index.html#safety) or
        // improperly aligned. If the function's return type was
        // mutable (wihtout being tagged mut) this would also break
        // the borrow checker.
        //
        // By wrapping in an unsafe block we are guaranteeing that
        // address will always be properly aligned and valid. The
        // return value (u32) is an immutable copy type.
        unsafe {
            let base = (&self.map).as_ptr() as *const u32;
            let address = base.add(READ_REG_OFFSET);
            let val: u32 = read_volatile(address);
            val
        }
    }

    fn bit(val: u32, gpio: &Pin) -> bool {
        if (val & (1 << gpio.0)) == 0 {
            //println!("zero pin: {}", gpio.0);
            true
        } else {
            false
        }
    }

    pub fn pinin(&mut self, gpio: &Pin) -> Result<(bool, Duration), Error> {
        let pin_num = gpio.0;

        if pin_num >= PIN_COUNT {
            return Err(Error::InvalidPin(gpio.clone()));
        }

        let val = self.value();
        let new_val = GPIO::bit(val, gpio);

        let (old_bit, _) = self.snap[pin_num];
        if new_val != old_bit {
            println!("pinin change state {}", new_val);
            self.snap[pin_num] = (new_val, Instant::now());
        }
        Ok((new_val, self.snap[pin_num].1.elapsed()))
    }
}
