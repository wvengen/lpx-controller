#![allow(non_snake_case)]
use rmididings::proc::*;
use std::cell::Cell;

// Modifier: set output port to Launchpad X
pub fn LPX() -> Port { Port(1) }
// Filter: pass events from the Launchpad X
pub fn LPXFilter() -> PortFilter { PortFilter(1) }
// Modifier: set output port to Controller
pub fn CTR() -> Port { Port(2) }
// Filter: pass events from the Controller
pub fn CTRFilter() -> PortFilter { PortFilter(2) }

// Generator: set DAW mode, mode=0 to enable, mode=1 to disable.
#[macro_export]
macro_rules! LPXDawMode { ($mode:expr) => {
    Chain!(SysEx(&[0xf0, 0x00, 0x20, 0x29, 0x02, 0x0c, 0x10, $mode, 0xf7]), LPX())
} }

// Generator: select layout, session=0, note=1, custom=4-7, mixers=13, programmer=127.
#[macro_export]
macro_rules! LPXSelectLayout { ($layout:expr) => {
    Chain!(SysEx(&[0xf0, 0x00, 0x20, 0x29, 0x02, 0x0c, 0x00, $layout, 0xf7]), LPX())
} }

// Generator: setup mixers
#[macro_export]
macro_rules! LPXSetupMixers { ($orientation:expr, $polarity:expr, $ctrl:expr, $color:expr) => {
    Chain!(
        SysEx(&[0xf0, 0x00, 0x20, 0x29, 0x02, 0x0c, 0x01, 0x00, $orientation as u8,
            0x00, $polarity as u8, $ctrl+0 as u8, $color as u8,
            0x01, $polarity as u8, $ctrl+1 as u8, $color as u8,
            0x02, $polarity as u8, $ctrl+2 as u8, $color as u8,
            0x03, $polarity as u8, $ctrl+3 as u8, $color as u8,
            0x04, $polarity as u8, $ctrl+4 as u8, $color as u8,
            0x05, $polarity as u8, $ctrl+5 as u8, $color as u8,
            0x06, $polarity as u8, $ctrl+6 as u8, $color as u8,
            0x07, $polarity as u8, $ctrl+7 as u8, $color as u8,
            0xf7]),
        LPX()
    )
} }

#[derive(Copy,Clone)]
pub enum LPXOrientation {
    Vertical = 0,
    Horizontal = 1
}

#[derive(Copy,Clone)]
pub enum LPXPolarity {
    Unipolar = 0,
    Bipolar = 1
}

// Generator: set Launchpad X button to a specific color
pub fn LPXButton<'a>(button: u32, color: LPXColor) -> FilterChain<'a> {
    Chain!(Ctrl(button, color as i32), Channel(1), LPX())
}

// Filter: pass events from a specific button press on the Launchpad X
pub fn LPXButtonFilter<'a>(button: u32) -> FilterChain<'a> {
    Chain!(LPXFilter(), TypeFilter!(Ctrl), ChannelFilter(1), CtrlFilter(button), CtrlValueFilter(127))
}

// Generator: set session button colors. Use active=Black to reset.
#[macro_export]
macro_rules! LPXSessionColor { ($active:expr, $inactive:expr) => {
    Chain!(SysEx(&[0xf0, 0x00, 0x20, 0x29, 0x02, 0x0c, 0x14, $active as u8, $inactive as u8, 0xf7]), LPX())
} }

#[allow(dead_code)]
#[derive(Copy,Clone)]
pub enum LPXColor {
    Black = 0,
    Softwhite = 1,
    White = 3,
    Red = 5,
    Orange = 9,
    Brown = 11,
    Yellow = 13,
    Green = 21,
    Softgreen = 19, // TODO find right shade of green
    Blue = 45,
}

// Controller memory.
// TODO move this to RMididings
pub struct CtrlsMemory<'a> {
    ctrls: &'a [u32],
    values: Vec<Cell<Option<i32>>>,
}

impl<'a> CtrlsMemory<'a> {
    pub fn new(ctrls: &'a [u32], initial_value: Option<i32>) -> Self {
        let values = vec![Cell::new(initial_value); ctrls.len()];
        CtrlsMemory { ctrls, values }
    }

    // Return a filter that stores any of the indicated controller values.
    pub fn Store(&self) -> CtrlsMemoryStore {
        CtrlsMemoryStore(&self.ctrls, &self.values)
    }

    // Return a generator that emits any stored controller values.
    pub fn Restore(&self) -> CtrlsMemoryRestore {
        CtrlsMemoryRestore(&self.ctrls, &self.values)
    }
}

pub struct CtrlsMemoryStore<'a>(&'a [u32], &'a Vec<Cell<Option<i32>>>);
impl<'a> FilterTrait for CtrlsMemoryStore<'a> {
    fn run(&self, evs: &mut EventStream) {
        for ev in evs.iter() {
            match ev {
                Event::Ctrl(ev) => {
                    if let Some(i) = self.0.iter().position(|&c| c == ev.ctrl) {
                        self.1[i].set(Some(ev.value));
                    }
                },
                _ => {},
            }
        }
    }
}

pub struct CtrlsMemoryRestore<'a>(&'a [u32], &'a Vec<Cell<Option<i32>>>);
impl<'a> FilterTrait for CtrlsMemoryRestore<'a> {
    fn run(&self, evs: &mut EventStream) {
        for i in 0..self.0.len() {
            if let Some(value) = self.1[i].get() {
                let ctrl = self.0[i];
                evs.push(CtrlEvent(0, 0, ctrl, value));
            }
        }
    }
}
