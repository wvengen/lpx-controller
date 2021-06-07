#!/usr/bin/env python3
from mididings import *
from mididings.event import CtrlEvent
from sequencer import OSCInterface

LPX_PORT_NAME_IN = 'Launchpad X in'
LPX_PORT_NAME_OUT = 'Launchpad X out'
CTR_PORT_NAME_IN = 'Controller in'
CTR_PORT_NAME_OUT = 'Controller out'

lpx_colors = {
    "black": 0,
    "softwhite": 1,
    "white": 3,
    "red": 5,
    "orange": 9,
    "brown": 11,
    "yellow": 13,
    "green": 21,
    "softgreen": 19, # TODO find right shade of green
    "blue": 45
}

def CTRFilter():
    return PortFilter(CTR_PORT_NAME_IN)

def CTR():
    return Port(CTR_PORT_NAME_OUT)

def LPXFilter():
    return PortFilter(LPX_PORT_NAME_IN)

def LPX():
    return Port(LPX_PORT_NAME_OUT)

def LPXButtonFilter(ctrl):
    return LPXFilter() >> ChannelFilter(1) >> CtrlFilter(ctrl) >> CtrlValueFilter(127)

def LPXButton(ctrl, color):
    return Ctrl(ctrl, lpx_colors[color]) >> Channel(1) >> LPX()

def LPXDawMode(mode):
    """Set DAW mode, mode=0 to enable, mode=1 to disable."""
    return SysEx([0xf0, 0x00, 0x20, 0x29, 0x02, 0x0c, 0x10, mode, 0xf7]) >> LPX()

def LPXSelectLayout(layout):
    """Select layout: session (0), note (1), custom (4-7), mixers (13), programmer (127)"""
    return SysEx([0xf0, 0x00, 0x20, 0x29, 0x02, 0x0c, 0x00, layout, 0xf7]) >> LPX()

def LPXSessionColor(active, inactive):
    """Set session button colors. Use active=0 to reset."""
    return SysEx([0xf0, 0x00, 0x20, 0x29, 0x02, 0x0c, 0x14, lpx_colors[active] if active else 0, lpx_colors[inactive] if inactive else 0, 0xf7]) >> LPX()

def LPXSetupMixers(orientation, polarity, cc, color):
    """Setup mixer mode.
    orientation=0 for vertical, orientation=1 for horizontal
    polarity=0 for unipolar, polarity=1 for bipolar
    cc is the first of the control change range to send
    color is the color
    """
    mixers = zip(range(0,8), [polarity]*8, range(cc, cc+8), [lpx_colors[color]]*8)
    mixers = [y for x in mixers for y in x] # flatten

    return SysEx([0xf0, 0x00, 0x20, 0x29, 0x02, 0x0c, 0x01, 0x00, orientation] + mixers + [0xf7]) >> LPX()

config(
    backend = "alsa",
    client_name = "Launchpad X Helper",
    in_ports = [
        (LPX_PORT_NAME_IN, 'Launchpad X:Launchpad X MIDI 1'),
        CTR_PORT_NAME_IN
    ],
    out_ports = [
        (LPX_PORT_NAME_OUT, 'Launchpad X:Launchpad X MIDI 1'),
        CTR_PORT_NAME_OUT
    ],
    initial_scene = 2,
)

# TODO only on request from command-line
osc = OSCInterface()
hook(osc)

# store state of right buttons sent by the controller (to restore after mixer mode)
btn_state_ccs = [89, 79, 69, 59, 49, 39, 29, 19]
btn_states = dict(zip(btn_state_ccs, [0] * len(btn_state_ccs)))
def store_btn_state(ev):
    btn_states[ev.ctrl] = ev.value
def stored_btn_state(ev, cc):
    if cc in btn_states:
        return CtrlEvent(LPX_PORT_NAME_OUT, 1, cc, btn_states[cc])

def ButtonState():
    return [Process(stored_btn_state, cc) for cc in btn_state_ccs]

# store mixer values sent by the controller
controllers = {}
def store_mixer(ev):
    controllers[ev.ctrl] = ev.value
def stored_mixer_value(ev, cc):
    if cc in controllers:
        return CtrlEvent(LPX_PORT_NAME_OUT, 5, cc, controllers[cc])

def MixerState(orientation, polarity, cc, color):
    return [
        Init([
            LPXSetupMixers(orientation, polarity, cc, color),
            Process(stored_mixer_value, cc),
            Process(stored_mixer_value, cc + 1),
            Process(stored_mixer_value, cc + 2),
            Process(stored_mixer_value, cc + 3),
            Process(stored_mixer_value, cc + 4),
            Process(stored_mixer_value, cc + 5),
            Process(stored_mixer_value, cc + 6),
            Process(stored_mixer_value, cc + 7),
        ])
    ]

def MixerButtons(active):
    return [
        Init([
            LPXSelectLayout(13),
            LPXSessionColor("orange", "softwhite"),
            LPXButton(89, "softgreen" if active == 89 else "softwhite"),
            LPXButton(79, "softgreen" if active == 79 else "softwhite"),
            LPXButton(69, "softgreen" if active == 69 else "softwhite"),
            LPXButton(59, "softgreen" if active == 59 else "softwhite"),
            LPXButton(49, "black"),
            LPXButton(39, "black"),
            LPXButton(29, "black"),
            LPXButton(19, "black"),
        ]),
        Exit([
            LPXSessionColor(None, None),
            ButtonState(),
        ]),
        LPXButtonFilter(95) >> SceneSwitch(2),
        LPXButtonFilter(89) >> SubSceneSwitch(1),
        LPXButtonFilter(79) >> SubSceneSwitch(2),
        LPXButtonFilter(69) >> SubSceneSwitch(3),
        LPXButtonFilter(59) >> SubSceneSwitch(4),
    ]

def NormalMain():
    return [
       # forward messages from LPX to controller and vice versa
       LPXFilter() >> CTR(),
       CTRFilter() >> LPX(),
       # also store state of right buttons in session mode when controller sends it
       CTRFilter() >> ChannelFilter(1) >> CtrlFilter(btn_state_ccs) >> Process(store_btn_state),
    ]

def MixerMain():
    # forward messages, but as we use the right buttons otherwise in the mixer view, don't pass
    # them through to the controller, and store incoming right button changes for the session view
    return [
        CTRFilter() >> ((ChannelFilter(1) & CtrlFilter(btn_state_ccs)) % Process(store_btn_state)) >> LPX(),
        LPXFilter() >> ((ChannelFilter(1) & CtrlFilter(btn_state_ccs)) % Discard()) >> CTR(),
    ]

run(
    scenes = {
        2: Scene("session", [
            Init([LPXDawMode(1), LPXSelectLayout(0)]), # TODO only send once
            LPXButtonFilter(95) >> SceneSwitch(3),
            NormalMain(),
            Process(osc.on_lpx_event),
        ]),
        3: SceneGroup("mixer", [
            Scene("volume", MixerMain() + MixerState(0, 0, 30, "orange") + MixerButtons(89)),
            Scene("pan",    MixerMain() + MixerState(1, 1, 38, "yellow") + MixerButtons(79)),
            Scene("send_a", MixerMain() + MixerState(0, 0, 46, "green" ) + MixerButtons(69)),
            Scene("send_b", MixerMain() + MixerState(0, 0, 54, "blue"  ) + MixerButtons(59)),
        ]),
        4: Scene("note", [
            LPXButtonFilter(95) >> SceneSwitch(2),
            NormalMain(),
        ]),
        5: Scene("custom", [
            LPXButtonFilter(95) >> SceneSwitch(2),
            NormalMain(),
        ]),
    },
    control = [
        # Store mixer control changes from both LPX and controller
        ChannelFilter(5) >> CtrlFilter(range(30, 62)) >> Process(store_mixer),
        # Layout buttons that have one scene only (others are handled in the scenes)
        LPXButtonFilter(96) >> SceneSwitch(4),
        LPXButtonFilter(97) >> SceneSwitch(5),
    ]
)
