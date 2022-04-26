# canbed_gs
gs_usb firmware in Rust for CANBED dual board.

I'm publishing this code for posterity. I haven't tested it because [the board](https://docs.longan-labs.cc/1030019/) died from a design issue before I finished the project. This issue is that USB VCC is connected directly to the 3V3 rail.

This might be useful to somebody, as it contains a (untested) implementation of gs_usb class for usb-device.
