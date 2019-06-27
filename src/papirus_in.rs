use crate::{err, stm};
use memmap::{Mmap, MmapOptions};
use std::{
    cmp::Ordering,
    fs::File,
    io::{self, Write},
    ptr::read_volatile,
    time::{Duration, Instant},
};
use Machine::*;

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

pub const SHORT_DURATION: Duration = Duration::from_millis(50);
pub const LONGISH_DURATION: Duration = Duration::from_millis(3000);
pub const LONG_DURATION: Duration = Duration::from_secs(4);

pub trait Button<E> {
    fn event(&mut self, ports: &mut GPIO) -> Result<E, Error>;
}

stm!(long_press_button_stm, LongPressMachine, [ReleasedPending, Bouncing] => NotPressed(), {
    [PressedPending, LongPressed]=>ReleasePending(),
    [] => Bouncing(),
    [] => PressedPending(),
    []=>LongPressed()
});

pub enum LongButtonEvent {
    Pressed,
    LongPress,
    Release,
}

pub struct DetectableDuration(Duration);
pub struct LongReleaseDuration(Duration);

pub struct LongPressButton {
    pin: Pin,
    state: LongPresMachine,
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
    pub fn event(&mut self, ports: &mut GPIO) -> Result<LongButtonEvent, Error> {
        let (pressing, duration) = ports.pinin(&self.pin)?;
        let mut event:Option::<LongButtonEvent>=None;
        use std::mem::replace;
        let mut state = replace(&mut self.state, NotPressed(button_stm::NotPressed));

                if !pressing {
                    if duration<self.long_release_after {
                        //add short pressed event if pending
                        if short press pending or long pressed or ReleasePending{
                            ReleasePending()
                        } else {
                            NotPressed
                        }
                    } else {
                        //add short pressed event if pending and released events if previously pressed (short or long)
                        NotPressed
                    }
                } else {
                    if duration < SHORT_DURATION {
                        if ReleasedPending || NotPressed {
                            Bouncing(st.into())
                        }
                    } else if duration < self.detectable_after {
                        if !LongPressed {
                            //short press pending
                            PressedPending(st)
                        }
                    } else {
                        //cancel short press pending
                        //add long pressed event if not already long pressed
                        event=Some(LongButtonEvent::LongPress);
                        LongPressed(st.into())
                    }
                }

        state = match state {
            NotPressed(st) => {
                if !pressing {
                    if duration<self.long_release_after {
                        NotPressed(st)
                    } else {
                    }
                } else {
                    if duration < SHORT_DURATION {
                        Bouncing(st.into())
                    } else if duration < self.detectable_after {
                        PressedPending(st.into())
                    } else {
                        LongPressed(st.into())
                    }
                }
            },
            ReleasePending(st) => {
                if !pressing {
                    if duration<self.long_release_after {
                        ReleasePending(st)
                    } else {
                        //add event
                        NotPressed(st.into())
                    }
                } else {
                    if duration < SHORT_DURATION {
                        Bouncing(st.into())
                    } else if duration < self.detectable_after {
                        PressedPending(st.into())
                    } else {
                        LongPressed(st.into())
                    }
                }
            },
            Bouncing(st) => {
                if !pressing {
                    NotPressed(st.into())
                } else {
                    if duration < SHORT_DURATION {
                        Bouncing(st)
                    } else if duration < self.detectable_after {
                        PressedPending(st.into())
                    } else {
                        //add event
                        event=Some(LongButtonEvent::LongPress);
                        LongPressed(st.into())
                    }
                }
            },
            PressedPending(st) => {
                if !pressing {
                    if duration<self.long_release_after {
                        //add pressed event
                        WasPressed(st.into())
                    } else {
                        //add pressed event
                        ReleasePending(st.into())
                    }
                } else {
                    if duration < self.detectable_after {
                        PressedPending(st)
                    } else {
                        //add event
                        event=Some(LongButtonEvent::LongPress);
                        LongPressed(st.into())
                    }
                }
            },
            WasPressed(st) => {
                if !pressing {
                    NotPressed(st.into())
                } else {
                    if duration < SHORT_DURATION {
                        Bouncing(st.into())
                    } else if duration < self.detectable_after {
                        PressedPending(st.into())
                    } else {
                        //add event
                        event=Some(LongButtonEvent::LongPress);
                        LongPressed(st.into())
                    }
                }
            },
            LongPressed(st) => {
                if !pressing {
                    NotPressed(st.into())
                } else {
                    if duration < SHORT_DURATION {
                        Bouncing(st.into())
                    } else if duration < self.detectable_after {
                        PressedPending(st.into())
                    } else {
                        LongPressed(st)
                    }
                }
            }
        };

        replace(&mut self.state, state);
        result
    }
}

stm!(repeat_button_stm, RepeatableMachine, [Bouncing,Pressed] => NotPressed(), {
    [NotPressed] => Bouncing(),
    [Bouncing] => Pressed()
});

pub enum RepeatableButtonEvent {
    Pressed,
    Release,
}

pub struct RepeatableButton {
    pin: Pin,
    state: RepeatableMachine,
    repeating_duration: Duration,
}

impl RepeatableButton {
    pub fn new(pin: Pin, repeating_duration: Duration) -> RepeatableButton {
        RepeatableButton {
            pin,
            repeating_duration,
            state: RepeatableMachine::NotPressed(repeat_button_stm::NotPressed),
        }
    }
}

impl Button<RepeatableButtonEvent> for RepeatableButton {
    pub fn event(&mut self, ports: &mut GPIO) -> Result<RepeatableButtonEvent, Error> {}
}

/*
fn pressing_transition(&mut self) -> bool {
    //returns true if the just the state can transition from NotPressed to Pressed (if it was already Pressed, or isn't currently pressed false is returned)
    println!("pressing transition");
    let mut result = true;
    use std::mem::replace;
    let mut state = replace(&mut self.state, NotPressed(button_stm::NotPressed));

    state = match state {
        NotPressed(st) => {
            println!("first press detected");
            Pressed(st.into())
        }
        Pressed(st) => {
            result = false;
            Pressed(st)
        }
    };

    replace(&mut self.state, state);
    result
}

fn releasing_transition(&mut self) -> bool {
    //returns true if the just the state can transition from Pressed to NotPressed (if it was already NotPressed, or is currently pressed false is returned)
    println!("releasing transition");
    let mut result = true;
    use std::mem::replace;
    let mut state = replace(&mut self.state, NotPressed(button_stm::NotPressed));

    state = match state {
        NotPressed(st) => {
            result = false;
            NotPressed(st)
        }
        Pressed(st) => {
            println!("button: {:?} just releasing now", self.pin);
            NotPressed(st.into())
        },
    };

    replace(&mut self.state, state);
    result
}

fn press_duration(&mut self, ports: &mut GPIO, min_duration: &Duration) -> Result<ButtonCondition, Error> {
    let pressing = ports.pinin(&self.pin)?;
    if let (true, press_duration)==pressing {
    } else {

    }
    match pressing {
        (true, press_duration) if min_duration.cmp(&press_duration) == Ordering::Less => {
            Ok(if self.pressing_transition() {
                ButtonCondition::JustPressed
            } else {
                ButtonCondition::AlreadyPressed
            })
        }
        _ => {
            Ok(ButtonCondition::Pending)
        }
    }
}

fn release_duration(&mut self, ports: &mut GPIO, min_duration: &Duration) -> Result<ButtonCondition, Error> {
    let pressing = ports.pinin(&self.pin)?;
    match pressing {
        (false, release_duration) if min_duration.cmp(&release_duration) == Ordering::Less => {
            //println!(
            //    "release duration exceeded: min: {:?} released: {:?}",
            //    min_duration, release_duration
            //);
            Ok(if self.releasing_transition() {
                ButtonCondition::JustReleased
            } else {
                ButtonCondition::AlreadyReleased
            })
        },
        _ => {
            Ok(ButtonCondition::Pending)
        }
    }
}
 */
impl<E> Button {
    pub fn event(&mut self, ports: &mut GPIO) -> Result<E, Error> {
        let pressing = ports.pinin(&self.pin)?;
    }

    pub fn pressed(&mut self, ports: &mut GPIO) -> Result<ButtonCondition, Error> {
        //for a repeat button this returns true each time the button press continues for PressDuration
        //for a long press button this returns a short press if released within LongDuration or a long
        //press immediately after a LongDuration press (without waiting for release)
        self.press_duration(ports, &SHORT_DURATION)
    }

    pub fn released(&mut self, ports: &mut GPIO) -> Result<ButtonCondition, Error> {
        //for a repeat button, returns true immediately upon release
        //for a long press button returns true after a ReleaseDuration passed after button release
        self.release_duration(ports, &LONGISH_DURATION)
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
