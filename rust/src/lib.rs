use godot::prelude::*;

pub mod player;
pub mod turboballs;

struct MyExtension;

#[gdextension]
unsafe impl ExtensionLibrary for MyExtension {}
