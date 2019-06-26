use crate::{err, stm};
use Machine::*;
use memmap::{Mmap, MmapOptions};
use std::{
    cmp::Ordering,
    fs::File,
    io::{self, Write},
    ptr::read_volatile,
    time::{Duration, Instant},
};

const BLOCK_SIZE: usize = 4 * 1024;
const MAX_PIN: usize = 27;
const READ_REG_OFFSET: usize = 13;

err!(Error {
    File(io::Error),
    InvalidPin(Pin)
});

pub const SW1: Button = Button {
    pin: Pin(16),
    state: NotPressed(button_stm::NotPressed),
};
pub const SW2: Button = Button {
    pin: Pin(26),
    state: NotPressed(button_stm::NotPressed),
};
pub const SW3: Button = Button {
    pin: Pin(20),
    state: NotPressed(button_stm::NotPressed),
};
pub const SW4: Button = Button {
    pin: Pin(21),
    state: NotPressed(button_stm::NotPressed),
};

pub const SHORT_PRESS_DURATION: Duration = Duration::from_millis(150);
pub const LONG_PRESS_DURATION: Duration = Duration::from_secs(4);

stm!(button_stm, Machine, [Pressed] => NotPressed(), {
    [NotPressed] => Pressed()
});

pub struct Button {
    pin: Pin,
    state: Machine,
}

impl Button {
    fn pressing_transition(&mut self) -> bool {
        let mut result = true;
        use std::mem::replace;
        let mut state = replace(&mut self.state, NotPressed(button_stm::NotPressed));

        state = match state {
            NotPressed(st) => Pressed(st.into()),
            Pressed(st) => {
                result = false;
                Pressed(st)
            }
        };

        replace(&mut self.state, state);
        result
    }

    fn not_pressing_transition(&mut self) -> bool {
        use std::mem::replace;
        let mut state = replace(&mut self.state, NotPressed(button_stm::NotPressed));

        state = match state {
            NotPressed(st) => NotPressed(st),
            Pressed(st) => NotPressed(st.into()),
        };

        replace(&mut self.state, state);
        false
    }

    fn press_duration(&mut self, ports: &mut GPIO, duration: &Duration) -> bool {
        match ports.pinin(&self.pin) {
            Ok((true, duration)) if SHORT_PRESS_DURATION.cmp(&duration) == Ordering::Less => {
                self.pressing_transition()
            }
            _ => self.not_pressing_transition(),
        }
    }
    
    pub fn press(&mut self, ports: &mut GPIO) -> bool {
        self.press_duration(ports, &SHORT_PRESS_DURATION)
    }

    pub fn long_pressed(&mut self, ports: &mut GPIO) -> bool {
        self.press_duration(ports, &LONG_PRESS_DURATION)
    }
}

#[derive(Clone,Debug)]
struct Pin(pub usize);

pub struct GPIO {
    map: Mmap,
    snap: [(bool, Instant); MAX_PIN],
}

impl GPIO {
    pub fn new() -> Result<GPIO, Error> {
        let f = File::open("/dev/gpiomem")?;
        let mmap = unsafe {
            MmapOptions::new().len(BLOCK_SIZE).map(&f)?
        };

        let t = Instant::now();
        let mut instance = GPIO {
            map: mmap,
            snap: [(false, t.clone()); MAX_PIN],
        };

        let val = instance.value();
        let mut gpio_num :usize= 0;
        while gpio_num <= MAX_PIN {
            instance.snap[gpio_num] = (GPIO::bit(val, &Pin(gpio_num )), t.clone());
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
        if val & 1 << gpio.0 == 0 {
            true
        } else {
            false
        }
    }

    pub fn pinin(&mut self, gpio: &Pin) -> Result<(bool, Duration), Error> {
        let pin_num = gpio.0;

        if pin_num > MAX_PIN {
            return Err(Error::InvalidPin(gpio.clone()));
        }

        let val = self.value();
        let new_val = GPIO::bit(val, gpio);

        let (old_bit, inst) = self.snap[pin_num];
        if new_val != old_bit {
            self.snap[pin_num] = (new_val, Instant::now());
        }
        Ok((new_val, inst.elapsed()))
    }
}
