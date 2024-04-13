use godot::prelude::*;

pub mod flicker;
pub mod turboballs;

struct MyExtension;

#[gdextension]
unsafe impl ExtensionLibrary for MyExtension {}
