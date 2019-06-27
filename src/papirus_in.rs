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
const PIN_COUNT: usize = 28;
const READ_REG_OFFSET: usize = 13;

pub const SW1_GPIO: usize=16;
pub const SW2_GPIO: usize=26;
pub const SW3_GPIO: usize=20;
pub const SW4_GPIO: usize=21;

err!(Error {
    File(io::Error),
    InvalidPin(Pin)
});

pub const SHORT_PRESS_DURATION: Duration = Duration::from_millis(150);
pub const LONG_PRESS_DURATION: Duration = Duration::from_secs(4);

stm!(button_stm, Machine, [Pressed] => NotPressed(), {
    [NotPressed] => Pressed()
});

pub struct Button {
    pub pin: Pin,
    pub state: Machine,
}

impl Button {
    pub fn new(pin: Pin) -> Button {
        Button {
            pin,
            state: Machine::NotPressed(button_stm::NotPressed),            
        }
    }
    
    fn pressing_transition(&mut self) -> bool {
        println!("presssing transition");
        let mut result = true;
        use std::mem::replace;
        let mut state = replace(&mut self.state, NotPressed(button_stm::NotPressed));

        state = match state {
            NotPressed(st) => {println!("first press detected"); Pressed(st.into())},
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

    fn press_duration(&mut self, ports: &mut GPIO, min_duration: &Duration) -> bool {
        match ports.pinin(&self.pin) {
            Ok((true, press_duration)) if min_duration.cmp(&press_duration) == Ordering::Less => {
                println!("press duration exceeded: min: {:?} pressed: {:?}", min_duration, press_duration);
                self.pressing_transition()
            }
            _ => self.not_pressing_transition(),
        }
    }
    
    pub fn short_press(&mut self, ports: &mut GPIO) -> bool {
        self.press_duration(ports, &SHORT_PRESS_DURATION)
    }

    pub fn long_press(&mut self, ports: &mut GPIO) -> bool {
        self.press_duration(ports, &LONG_PRESS_DURATION)
    }
}

#[derive(Clone,Debug)]
pub struct Pin(pub usize);

pub struct GPIO {
    map: Mmap,
    snap: [(bool, Instant); PIN_COUNT],
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
            snap: [(false, t.clone()); PIN_COUNT],
        };

        let val = instance.value();
        let mut gpio_num :usize= 0;
        while gpio_num < PIN_COUNT {
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
