//! A scratchpad that holds a single client
use crate::core::{
    client::Client,
    data_types::{Config, FireAndForget, Region, WinId},
    helpers::spawn,
    hooks::Hook,
    manager::WindowManager,
};

use std::{cell::RefCell, rc::Rc};

/**
 * Position of an Anchor
 * 'Top' and 'Bottom' should only be used for the vertical position
 * 'Left' and 'Right' should only be used for the horizontal position
 * 'Center' can be used for either
 */
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum AnchorPosition {
    /// Top of the screen
    Top,
    /// Center of the screen
    Center,
    /// Bottom of the screen
    Bottom,
    /// Left of the screen
    Left,
    /// Right of the screen
    Right,
}

pub use AnchorPosition::*;

/// Where a Scratchpad should be placed on the screen
#[derive(Debug, Clone, Copy)]
pub struct Anchor {
    horizontal: AnchorPosition,
    vertical: AnchorPosition,
    offset: (u32, u32),
}

impl Anchor {
    /**
     * Create a new Anchor for a Scratchpad. 'horizontal' and 'vertical' determine the Anchor
     * point. 'offset' determines the offset from the Anchor point (in pixels).
     */
    pub fn new(horizontal: AnchorPosition, vertical: AnchorPosition, offset: (u32, u32)) -> Self {
        if horizontal == Top || horizontal == Bottom {
            panic!("Scratchpad: invalid horizontal anchor {:?}", horizontal);
        }

        if vertical == Left || vertical == Right {
            panic!("Scratchpad: invalid vertical anchor {:?}", vertical);
        }

        Self {
            horizontal,
            vertical,
            offset,
        }
    }

    /**
     * Create a new Achor in the center of the screen with no offset.
     */
    pub fn default() -> Self {
        Self::new(Center, Center, (0, 0))
    }
}

/**
 * A Scratchpad spawns and manages a single Client which can then be shown above the current layout
 * using the 'toggle' method when bound to a key combination in your main.rs. The
 * Scratchpad.register method must be called before creating your WindowManager struct in order to
 * register the necessary hooks to spawn, capture and manage the embedded client. The client is
 * spawned when 'toggle' is called and there is no existing client, after that 'toggle' will
 * show/hide the client on the active screen. If the client is removed, calling 'toggle' again will
 * spawn a new client in the same way.
 */
pub struct Scratchpad {
    client: Rc<RefCell<Option<WinId>>>,
    pending: Rc<RefCell<bool>>,
    visible: Rc<RefCell<bool>>,
    prog: &'static str,
    w: f32,
    h: f32,
    anchor: Anchor,
}

impl Scratchpad {
    /// Create a new Scratchpad for holding 'prog'. 'w' and 'h' are the percentage width and height
    /// of the active screen that you want the client to take up when visible.
    /// NOTE: this function will panic if 'w' or 'h' are not within the range 0.0 - 1.0
    pub fn new(prog: &'static str, w: f32, h: f32, anchor: Anchor) -> Scratchpad {
        if w < 0.0 || w > 1.0 || h < 0.0 || h > 1.0 {
            panic!("Scratchpad: w & h must be between 0.0 and 1.0");
        }

        Scratchpad {
            client: Rc::new(RefCell::new(None)),
            pending: Rc::new(RefCell::new(false)),
            visible: Rc::new(RefCell::new(false)),
            prog,
            w,
            h,
            anchor,
        }
    }

    fn boxed_clone(&self) -> Box<Scratchpad> {
        Box::new(Scratchpad {
            client: Rc::clone(&self.client),
            pending: Rc::clone(&self.pending),
            visible: Rc::clone(&self.visible),
            prog: self.prog,
            w: self.w,
            h: self.h,
            anchor: self.anchor,
        })
    }

    /// Register the required hooks for managing this Scratchpad. Must be called before
    /// WindowManager.init.
    pub fn register(&self, conf: &mut Config) {
        conf.hooks.push(self.boxed_clone())
    }

    /// Show / hide the bound client. If there is no client currently, then spawn one.
    pub fn toggle(&self) -> FireAndForget {
        let mut clone = self.boxed_clone();
        Box::new(move |wm: &mut WindowManager| clone.toggle_client(wm))
    }

    fn toggle_client(&mut self, wm: &mut WindowManager) {
        let id = match *self.client.borrow() {
            Some(id) => id,
            None => {
                self.pending.replace(true);
                self.visible.replace(false);
                spawn(self.prog); // caught by new_client
                return;
            }
        };

        if *self.visible.borrow() {
            self.visible.replace(false);
            wm.hide_client(id);
        } else {
            self.visible.replace(true);
            wm.layout_screen(wm.active_screen_index()); // caught by layout_change
        }
    }

    fn get_position(&self, screen: Region) -> Region {
        let (sx, sy, sw, sh) = screen.values();
        let w = (sw as f32 * self.w) as u32;
        let h = (sh as f32 * self.h) as u32;

        let x = match self.anchor.horizontal {
            Left => sx,
            Right => sx + sw - w,
            Center => sx + (sw - w) / 2,
            _ => unreachable!(),
        } + self.anchor.offset.0;

        let y = match self.anchor.vertical {
            Top => sy,
            Bottom => sy + sh - h,
            Center => sy + (sh - h) / 2,
            _ => unreachable!(),
        } + self.anchor.offset.1;

        Region::new(x, y, w, h)
    }
}

impl Hook for Scratchpad {
    fn new_client(&mut self, wm: &mut WindowManager, c: &mut Client) {
        if *self.pending.borrow() && self.client.borrow().is_none() {
            self.pending.replace(false);
            self.client.replace(Some(c.id()));
            c.externally_managed();
            self.toggle_client(wm);
        }
    }

    fn remove_client(&mut self, _: &mut WindowManager, id: WinId) {
        let client = match *self.client.borrow() {
            Some(id) => id,
            None => return,
        };

        if id == client {
            self.client.replace(None);
            self.visible.replace(false);
        }
    }

    fn layout_applied(&mut self, wm: &mut WindowManager, _: usize, screen_index: usize) {
        match *self.client.borrow() {
            None => return, // no active scratchpad client
            Some(id) => {
                if *self.visible.borrow() {
                    if let Some(region) = wm.screen_size(screen_index) {
                        wm.position_client(id, self.get_position(region));
                    }
                    wm.show_client(id);
                }
            }
        }
    }
}
