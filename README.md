# embedded-web-ui

Remote web UI for embedded (Rust) applications. 
Control an embedded application (or anything, really) using a browser, which talks to a websocket/serial bridge.

## High level usage

- You need a serial device connected to the host.
- Describe your desired UI primitives (buttons, sliders, charts, etc.) in your embedded application and send them out over this connection. Same goes for data updates (chart time series ticks, bar data). The list **MUST** start with a `Reset`.
- These widgets are then displayed on a web page. User interactions (button press, slider value change, ...) are sent back to the device.

## Running
Since Mozilla's position on WebSerial is [a hard no](https://mozilla.github.io/standards-positions/#webserial), 
we include a bridge that translates serial packets to WebSockets:

```sh
$ cd serial-ws-bridge

# tries to guess the serial device
$ cargo run

# alternatively, specify the name
$ cargo run -- /dev/cu.usbserial1234
```

**TODO add more convenient dev server option**

now, start the web server:

```sh
# once
$ cargo install dioxus-cli

$ cd web-app
$ dioxus serve

# open in browser: http://localhost:8080/
```

finally, connect the serial device and enjoy your remote UI!

## UI details

- available primitives: see `/src/lib.rs`.


## example stm32f4 app

(extremely proof of concept only - do not take as reference implementation!)
- toggles board LED
- sends random chart data when requested 