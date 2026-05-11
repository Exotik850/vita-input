# vita-input 

A basic wrapper around `vitasdk-sys` to allow for more ergonomic polling of the Vita's controller buttons, joysticks, and touch panels.

## Getting Started

Install `vita-input` by adding it to your projects `Cargo.toml` file:

```toml
[dependencies]
vita-input = "0.1"
```

### Controller usage

```rust
use vita_input::controller::{VitaInput, Button, Joystick, Axis};

let input = VitaInput::poll();

if input.is_pressed(Button::Cross) {
    // jump
}

let lx = input.joystick(Joystick::Left).axis(Axis::X);
if lx > 0.5 {
    // move right
}
```

### Touch usage

```rust
use vita_input::touch::{TouchInput, TouchPort};

// Start sampling for the front touch panel
let input = TouchInput::start_sampling(TouchPort::Front);

// Poll for touch events
let touch = input.poll();
for point in &touch {
    // Access point.x, point.y
}
```

## License

Licensed under the [MIT License](LICENSE)