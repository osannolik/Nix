use core::fmt::Debug;
use embedded_hal::digital::v2::InputPin;

#[derive(Copy, Clone)]
pub enum PinLevel {
    Low,
    High,
    Rising,
    Falling,
}

impl PinLevel {
    pub fn next(&self, pin_is_pressed: bool) -> PinLevel {
        match self {
            PinLevel::Low if pin_is_pressed => PinLevel::Rising,
            PinLevel::High if !pin_is_pressed => PinLevel::Falling,
            PinLevel::Rising if pin_is_pressed => PinLevel::High,
            PinLevel::Rising => PinLevel::Falling,
            PinLevel::Falling if pin_is_pressed => PinLevel::Rising,
            PinLevel::Falling => PinLevel::Low,
            _ => *self,
        }
    }
}

#[derive(Copy, Clone)]
pub struct ButtonState {
    pub level: PinLevel,
    pub count: u32,
}

impl ButtonState {
    pub fn new(pressed: bool) -> ButtonState {
        ButtonState {
            level: match pressed {
                true => PinLevel::High,
                false => PinLevel::Low,
            },
            count: 0,
        }
    }

    pub fn is_pressed(&self, counts: u32) -> bool {
        matches!(self.level, PinLevel::High if self.count >= counts)
    }

    pub fn update(&mut self, pressed: bool) {
        let level = self.level.next(pressed);
        let count = match (self.level, level) {
            (PinLevel::Low, PinLevel::Low | PinLevel::Rising) => self.count + 1,
            (PinLevel::High, PinLevel::High | PinLevel::Falling) => self.count + 1,
            (_, _) => 0,
        };
        *self = ButtonState { level, count }
    }
}

#[derive(Copy, Clone)]
pub struct ButtonStates {
    pub set: ButtonState,
    pub up: ButtonState,
    pub down: ButtonState,
}

pub struct Buttons<PinSet, PinUp, PinDown> {
    set_pin: PinSet,
    up_pin: PinUp,
    down_pin: PinDown,
    state: ButtonStates,
}

fn is_pressed_level<PinE: Debug, Pin: InputPin<Error = PinE>>(pin: &Pin) -> bool {
    pin.is_low().unwrap()
}

impl<PinSet, PinUp, PinDown, PinE: Debug> Buttons<PinSet, PinUp, PinDown>
where
    PinSet: InputPin<Error = PinE>,
    PinUp: InputPin<Error = PinE>,
    PinDown: InputPin<Error = PinE>,
{
    pub fn new(set: PinSet, up: PinUp, down: PinDown) -> Self {
        let state = ButtonStates {
            set: ButtonState::new(is_pressed_level(&set)),
            up: ButtonState::new(is_pressed_level(&up)),
            down: ButtonState::new(is_pressed_level(&down)),
        };
        Buttons {
            set_pin: set,
            up_pin: up,
            down_pin: down,
            state,
        }
    }

    pub fn poll_state(&mut self) -> ButtonStates {
        self.state.set.update(is_pressed_level(&self.set_pin));
        self.state.up.update(is_pressed_level(&self.up_pin));
        self.state.down.update(is_pressed_level(&self.down_pin));
        self.state
    }
}
