# Launchpad X Controller

The [Novation Launchpad X](https://novationmusic.com/en/launch/launchpad-x) is a MIDI controller
with 80 buttons that connects over USB. By default, _Note_ and _Custom_ layouts can be used, but
not the _Session_ layout.

While some software may provide support for this device, it's improbable that all useful packages
know about this device. So why not making something that allows music-software to work with the
full functionality of this device using standard MIDI messages? That's what this project is.

This program sits in between your music application and the Launchpad X device. It interacts with
both, and expands the functionality of a 'stock' Launchpad X with the _Session_ layout and four
_Mixer_ layouts (_Volume_, _Pan_, _Send A_ and _Send B_).

When the 'Controller' input and output is connected to your music application, you can use regular
MIDI learn functionality to control it with the Launchpad X.

## Install

Go to [releases](https://github.com/wvengen/lpx-controller/releases) and find the latest release.
Get the binary appropriate for your system (if you don't know, it's probably `x86_64`), and install
it somewhere in your `PATH`:

```sh
mkdir -p $HOME/.local.bin && \
wget -O $HOME/.local/bin/lpx-controller \
  https://github.com/wvengen/lpx-controller/releases/download/latest/lpx-controller-`uname -i` && \
chmod a+x $HOME/.local/bin/lpx-controller
```

Alternatively, you can clone this repository and run `cargo build --release`, after which you
can find the binary as `target/release/lpx-controller`. See [Develop](#develop) for build requirements.

## Run

1. Connect your Launchpad X to the computer.
2. Run `lpx-controller` from the command-line.
3. You should see the _Session_ button light up.
4. Connect your audio application to _Launchpad X Helper_'s ports named _Controller in_ and _Controller out_.
5. When you're done, press `Ctrl-C` in the console to stop this program.

Note that this program doesn't currently reconnect to the Launchpad X when you plug it in and out. You're
probably using a patchbay application already, so you might consider including these too. A later version of
the program could perhaps reconnect automatically (and re-initialize without having to press _Session_ again).

## Notes

The four mixer layouts are initialized to send the following control change messages:
- _Volume_ - Channel 5, CC 30 - 37
- _Pan_ - Channel 5, CC 38 - 45
- _Send A_ - Channel 5, CC 46 - 53
- _Send B_ - Channel 5, CC 54 - 61

Note that your music application must not echo received control changes back to the device, because the
Launchpad X fades the mixer channels, and on receiving the fade will stop.

## Tested with

Feel free you share your usage of this program by submitting an issue or PR.

* [Luppp](http://openavproductions.com/luppp/) -
    with [this PR](https://github.com/openAVproductions/openAV-Luppp/pull/310)
    and [this controller definition](https://gist.github.com/wvengen/dd43cc82ad4ef425630fa290c1f2b3e9)

## Develop

You'll need [Rust](https://www.rust-lang.org/) 1.52.0+ with [Cargo](https://doc.rust-lang.org/cargo/).
The easiest option to get a recent enough Rust is using [Rustup](https://rustup.rs/).

You also need the ALSA headers. On Debian you would need to run `apt-get install libasound2-dev`,
on Fedora `dnf install alsa-lib-devel`.

With these in place, running a development version of lpx-controller is as easy as `cargo run`.

Relevant links:
- [RMididings](https://github.com/wvengen/rmididings), on which lpx-controller is built.
- [mididings documentation](http://dsacre.github.io/mididings/doc/), which RMididings is inspired by.
- [Launchpad X programmer's reference guide](https://fael-downloads-prod.focusrite.com/customer/prod/s3fs-public/downloads/Launchpad%20X%20-%20Programmers%20Reference%20Manual.pdf)
- A previous version of [lpx-controller in Python](https://github.com/wvengen/lpx-controller/tree/python).

## License

This program is licensed under the [GNU GPL v3 or later](LICENSE.md).
