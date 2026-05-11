# vita-input 

A basic wrapper around `vitasdk-sys` to allow for more ergonomic polling of the Vita's controller buttons and joysticks.

## Getting Started

Install `vita-input` by adding it to your projects `Cargo.toml` file:

```toml
vita-input = "0.1"
```

and then start polling the device's input like so:
```rs
use vita_input::VitaInput;

let input = VitaInput::poll();

if input.is_pressed(Button::Cross) {
    // jump
}

let lx = input.joystick(Joystick::Left).axis(Axis::X);
if lx > 0.5 {
    // move right
}
```

## License

Licensed under the [MIT License](LICENSE)