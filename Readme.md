# Turboballs-gd
A quick port of Turboballs to Godot 4.2 using [godot-rust](https://godot-rust.github.io/).

## Playing
You can download the game from [itch.io](https://notoriouseng.itch.io/turboballs)

## Building
Rust Library:
* Note: you may need to adjust paths in `bin/gamelib.gdextension`
```sh
cd rust
cargo build # OR cargo build --release 
```
Godot Binary: Use godot to export to your platform of choice
