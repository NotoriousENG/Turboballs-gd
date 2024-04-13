use godot::engine::{ILabel, Label};
use godot::prelude::*;

#[derive(GodotClass)]
#[class(base=Label)]
pub struct Flicker {
    base: Base<Label>,
    #[export]
    flicker_time: f64,
    time: f64,
}

#[godot_api]
impl ILabel for Flicker {
    fn init(base: Base<Label>) -> Self {
        Self {
            base,
            flicker_time: 0.75,
            time: 0.0,
        }
    }
    fn process(&mut self, delta: f64) {
        // get the time since the game started
        self.time += delta;
        // if the time is greater than the flicker time
        if self.time > self.flicker_time {
            let v = self.base().is_visible();
            // toggle the visibility of the Label
            self.base_mut().set_visible(!v);
            // reset the time
            self.time = 0.0;
        }
    }
}
