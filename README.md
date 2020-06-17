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

At this moment, the program uses [mididings](http://das.nasophon.de/mididings/), which unfortunately
is not packaged as a Python package, and it was removed from
[Debian](https://packages.debian.org/search?keywords=mididings&searchon=names&suite=all&section=all)
and [Ubuntu](https://packages.ubuntu.com/search?keywords=mididings&searchon=names&suite=all&section=all).

At this moment, you'll have to build it from source. For Debian/Ubuntu:

```
sudo apt install git build-essential libboost-python-dev libboost-thread-dev python3-decorator \
                 libglib2.0-dev libasound2-dev
git clone https://github.com/dsacre/mididings.git
cd mididings
python3 setup.py install --user
```

If you get a failure in the last step with `cannot find -lboost_python` (and maybe a `SyntaxError`),
try running:
```
sed -i "s/boost_python_suffixes\.append('3')/boost_python_suffixes.append('38')/"  setup.py
find . -name \*.py | xargs sed -ri 's/\basync\b/asyn/g'
python3 setup.py install --user
```

To install the program, download [`lpx-controller.py`](lpx-controller.py) e.g. to `~/.local/.bin/`:
```
wget -O ~/.local/bin/lpx-controller https://github.com/wvengen/lpx-controller/raw/master/lpx-controller.py
chmod a+x ~/.local/bin/lpx-controller
```

## Run

1. Connect your Launchpad X to the computer.
2. Run `lpx-controller` from the command-line. (*)
3. You should see the _Session_ button light up.
4. Connect your audio application to _Launchpad X Helper_'s ports named _Controller in_ and _Controller out_.
5. When you're done, press `Ctrl-C` in the console to stop this program.

Note that this program doesn't currently reconnect to the Launchpad X when you plug it in and out. You're
probably using a patchbay application already, so you might consider including these too. A later version of
the program could perhaps reconnect automatically (and re-initialize without having to press _Session_ again).

(*) If you didn't install the program in your `PATH`, you may need to run it with `python3 lpx-controller.py`
instead.

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

## License

This program is licensed under the [GNU GPL v3 or later](LICENSE.md).
