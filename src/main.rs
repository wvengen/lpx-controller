#![allow(non_snake_case)]
use std::error::Error;

#[macro_use]
extern crate rmididings;
use rmididings::*;

mod lpxproc;
use lpxproc::*;
use lpxproc::LPXColor::*;
use lpxproc::LPXOrientation::*;
use lpxproc::LPXPolarity::*;

fn main() {
    match run() {
        Ok(_) => (),
        Err(err) => println!("Error: {}", err)
    }
}

// Button ctrls that we need to store because we use them (e.g. in the mixer views).
const STORED_BTNS: [u32; 8] = [89, 79, 69, 59, 49, 39, 29, 19];

fn run() -> Result<(), Box<dyn Error>> {
    let mut md = RMididings::new()?;

    md.config(ConfigArguments {
        client_name: "Launchpad X Controller",
        in_ports: &[
            ["Lauchpad X in", "Launchpad X:Launchpad X MIDI 1"],
            ["Controller in", ""],
        ],
        out_ports: &[
            ["Launchpad X out", "Launchpad X:Launchpad X MIDI 1"],
            ["Controller out", ""],
        ],
        data_offset: 1,
        scene_offset: 0,
        ..ConfigArguments::default()
    })?;

    // Stored state of right buttons (which we use in mixer but want to have free to use in session mode).
    let btnMem = CtrlsMemory::new(&STORED_BTNS, Some(Black as i32));
    // Stored state of mixer values (so that we can use multiple mixers).
    let btnMixerVol = CtrlsMemory::new(&[30, 31, 32, 33, 34, 35, 36, 37], Some(Black as i32));
    let btnMixerPan = CtrlsMemory::new(&[38, 39, 40, 41, 42, 43, 44, 45], Some(Black as i32));
    let btnMixerSdA = CtrlsMemory::new(&[46, 47, 48, 49, 50, 51, 52, 53], Some(Black as i32));
    let btnMixerSdB = CtrlsMemory::new(&[54, 55, 56, 57, 58, 59, 60, 61], Some(Black as i32));

    md.run(RunArguments {
        scenes: &[
            &Scene { // 0
                name: "init",
                init: &Fork!(
                    LPXDawMode!(1),
                    LPXSelectLayout!(0),
                    LPXSessionColor!(Black, Black),
                    SceneSwitch(1)
                ),
                ..Scene::default()
            },
            &Scene { // 1
                name: "session",
                patch: &Fork!(
                    Chain!(LPXButtonFilter(95), SceneSwitch(2)),
                    NormalForward(&btnMem)
                ),
                ..Scene::default()
            },
            &Scene { // 2
                name: "mixer",
                subscenes: &[
                    &Scene { // 2.0
                        name: "volume",
                        init: &Fork!(
                            LPXButton(89, Softgreen),
                            LPXSetupMixers!(Vertical, Unipolar, 30, Orange),
                            Chain!(btnMixerVol.Restore(), Channel(5), LPX())
                        ),
                        patch: &Discard(),
                        exit: &LPXButton(89, Softwhite),
                        ..Scene::default()
                    },
                    &Scene { // 2.1
                        name: "pan",
                        init: &Fork!(
                            LPXButton(79, Softgreen),
                            LPXSetupMixers!(Horizontal, Bipolar, 38, Yellow),
                            Chain!(btnMixerPan.Restore(), Channel(5), LPX())
                        ),
                        patch: &Discard(),
                        exit: &LPXButton(79, Softwhite),
                        ..Scene::default()
                    },
                    &Scene { // 2.2
                        name: "send a",
                        init: &Fork!(
                            LPXButton(69, Softgreen),
                            LPXSetupMixers!(Vertical, Unipolar, 46, Green),
                            Chain!(btnMixerSdA.Restore(), Channel(5), LPX())
                        ),
                        patch: &Discard(),
                        exit: &LPXButton(69, Softwhite),
                        ..Scene::default()
                    },
                    &Scene { // 2.3
                        name: "send b",
                        init: &Fork!(
                            LPXButton(59, Softgreen),
                            LPXSetupMixers!(Vertical, Unipolar, 54, Blue),
                            Chain!(btnMixerSdB.Restore(), Channel(5), LPX())
                        ),
                        patch: &Discard(),
                        exit: &LPXButton(59, Softwhite),
                        ..Scene::default()
                    }
                ],
                init: &Fork!(
                    LPXSelectLayout!(13),
                    LPXSessionColor!(Orange, Softwhite),
                    // Setup right buttons for switching mixer subscenes.
                    LPXButton(89, Softwhite),
                    LPXButton(79, Softwhite),
                    LPXButton(69, Softwhite),
                    LPXButton(59, Softwhite),
                    LPXButton(49, Black),
                    LPXButton(39, Black),
                    LPXButton(29, Black),
                    LPXButton(19, Black)
                ),
                patch: &Fork!(
                    MixerForward(&btnMem),
                    // Switch to mixer subscene when pressing one of the four right buttons.
                    Chain!(LPXButtonFilter(95), SceneSwitch(1)),
                    Chain!(LPXButtonFilter(89), SubSceneSwitch(0)),
                    Chain!(LPXButtonFilter(79), SubSceneSwitch(1)),
                    Chain!(LPXButtonFilter(69), SubSceneSwitch(2)),
                    Chain!(LPXButtonFilter(59), SubSceneSwitch(3))
                ),
                exit: &Fork!(
                    LPXSessionColor!(Black, Black),
                    Chain!(btnMem.Restore(), LPX())
                ),
                ..Scene::default()
            },
            &Scene { // 3
                name: "note",
                patch: &Fork!(
                    Chain!(LPXButtonFilter(95), SceneSwitch(1)),
                    NormalForward(&btnMem)
                ),
                ..Scene::default()
            },
            &Scene { // 4
                name: "custom",
                patch: &Fork!(
                    Chain!(LPXButtonFilter(95), SceneSwitch(1)),
                    NormalForward(&btnMem)
                ),
                ..Scene::default()
            },
        ],
        control: &Fork!(
            Chain!(LPXButtonFilter(96), SceneSwitch(3)),
            Chain!(LPXButtonFilter(97), SceneSwitch(4)),
            // Store mixer values from both LPX and Controller.
            Chain!(ChannelFilter(5), btnMixerVol.Store(), btnMixerPan.Store(), btnMixerSdA.Store(), btnMixerSdB.Store(), Discard())
        ),
        ..RunArguments::default()
    })?;

    Ok(())
}

fn NormalForward<'a>(btnMem: &'a CtrlsMemory) -> FilterChain<'a> {
    Fork!(
        // forward messages from LPX to controller and vice versa
        Chain!(LPXFilter(), CTR()),
        Chain!(CTRFilter(), LPX()),
        // also store state of right buttons in session mode when controller sends it
        Chain!(CTRFilter(), ChannelFilter(1), btnMem.Store(), Discard())
    )
}

fn MixerForward<'a>(btnMem: &'a CtrlsMemory) -> FilterChain<'a> {
    Fork!(
        // forward messages, but as we use the right buttons otherwise in the mixer view, don't pass
        // them through to the controller, and store incoming right button changes for the session view
        Chain!(CTRFilter(), Not!(Chain!(ChannelFilter(1), CtrlsFilter(&STORED_BTNS))), LPX()),
        Chain!(LPXFilter(), Not!(Chain!(ChannelFilter(1), CtrlsFilter(&STORED_BTNS))), CTR()),
        Chain!(CTRFilter(), ChannelFilter(1), btnMem.Store(), Discard())
    )
}
