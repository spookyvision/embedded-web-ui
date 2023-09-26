# embedded-web-ui

Remote web UI for embedded (Rust) applications. 
Control an embedded application (or anything, really) using a browser, which talks to a websocket/serial bridge.


https://github.com/spookyvision/embedded-web-ui/assets/45840/a1f33a06-5b54-44d1-8552-ea7130e319e5

[![Live FFT chart sampled from microphone](https://img.youtube.com/vi/OVQCu2fmVps/0.jpg)](https://www.youtube.com/watch?v=OVQCu2fmVps)

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


now, start the web server:

(**TODO add more convenient dev server option** by providing a prebuilt app)

```sh
# once: install dioxus CLI
# you can use `cargo install dioxus-cli` when these fixes have made it to the release version:
# https://github.com/DioxusLabs/cli/pull/122
# https://github.com/DioxusLabs/cli/pull/130
$ cargo install --git https://github.com/spookyvision/dioxus-cli --branch patch-1

$ cd web-app
$ dioxus serve --features postcard # for Rust applications
# or
$ dioxus serve --features json # for e.g. micropython/circuitpython

# open in browser: http://localhost:8080/
```

finally, connect the serial device and enjoy your remote UI!

## UI details

- available primitives: see `/src/lib.rs`.


## example stm32f4 app

(extremely proof of concept only - do not take as reference implementation!)
- toggles board LED
- sends random chart data when requested 

## notes on JSON

The serial/ws bridge is largely content agnostic but assumes packets sent from the serial device are framed/terminated with nullbytes. 
The other direction doesn't need such a separator, since WebSockets are already framed.

In practice this means:
- postcard COBS encoding is fine to transmit as-is
- other COBS encodings might need an explicit `\0` appended, since it's strictly speaking not part of the actual payload
- JSON data needs to be terminated with an explicit `\0`
- any other data is fine, as long as it does not contain `\0` except for framing. The web app currently only supports decoding JSON and postcard-cobs, though. See `web-app/src/ser_de.rs` for details.
