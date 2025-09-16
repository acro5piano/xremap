use log::{debug, warn};
use std::collections::HashMap;
use x11::keysym;
use x11::xlib::{self, Display, KeyCode, KeySym, XKeyEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyPress {
    pub keycode: KeyCode,
    pub modifiers: u32,
}

#[derive(Debug, Clone)]
pub struct KeyMapper {
    display: *mut Display,
    keysym_map: HashMap<String, KeySym>,
    modifier_map: HashMap<String, u32>,
}

impl KeyMapper {
    pub fn new(display: *mut Display) -> Self {
        let mut keysym_map = HashMap::new();
        let mut modifier_map = HashMap::new();

        // Common key mappings
        keysym_map.insert("Left".to_string(), keysym::XK_Left as KeySym);
        keysym_map.insert("Right".to_string(), keysym::XK_Right as KeySym);
        keysym_map.insert("Up".to_string(), keysym::XK_Up as KeySym);
        keysym_map.insert("Down".to_string(), keysym::XK_Down as KeySym);
        keysym_map.insert("Home".to_string(), keysym::XK_Home as KeySym);
        keysym_map.insert("End".to_string(), keysym::XK_End as KeySym);
        keysym_map.insert("BackSpace".to_string(), keysym::XK_BackSpace as KeySym);
        keysym_map.insert("Delete".to_string(), keysym::XK_Delete as KeySym);
        keysym_map.insert("Return".to_string(), keysym::XK_Return as KeySym);
        keysym_map.insert("Tab".to_string(), keysym::XK_Tab as KeySym);
        keysym_map.insert("Escape".to_string(), keysym::XK_Escape as KeySym);
        keysym_map.insert("space".to_string(), keysym::XK_space as KeySym);

        // Function keys
        for i in 1..=12 {
            keysym_map.insert(format!("F{}", i), keysym::XK_F1 as KeySym + i - 1);
        }

        // Letters
        for c in 'a'..='z' {
            keysym_map.insert(c.to_string(), c as KeySym);
            keysym_map.insert(
                c.to_uppercase().to_string(),
                c.to_uppercase().next().unwrap() as KeySym,
            );
        }

        // Numbers
        for i in '0'..='9' {
            keysym_map.insert(i.to_string(), i as KeySym);
        }

        // Modifiers
        modifier_map.insert("Ctrl".to_string(), xlib::ControlMask);
        modifier_map.insert("C".to_string(), xlib::ControlMask);
        modifier_map.insert("Alt".to_string(), xlib::Mod1Mask);
        modifier_map.insert("M".to_string(), xlib::Mod1Mask);
        modifier_map.insert("Shift".to_string(), xlib::ShiftMask);
        modifier_map.insert("S".to_string(), xlib::ShiftMask);
        modifier_map.insert("Super".to_string(), xlib::Mod4Mask);

        Self {
            display,
            keysym_map,
            modifier_map,
        }
    }

    pub fn parse_key(&self, key_expr: &str) -> Option<(KeySym, u32)> {
        debug!("Parsing key expression: '{}'", key_expr);
        let parts: Vec<&str> = key_expr.split('-').collect();
        let mut modifiers = 0u32;
        let mut key_part = "";

        for (i, part) in parts.iter().enumerate() {
            if i == parts.len() - 1 {
                key_part = part;
            } else if let Some(mod_mask) = self.modifier_map.get(*part) {
                modifiers |= mod_mask;
                debug!("Found modifier '{}' -> {:#x}", part, mod_mask);
            } else {
                warn!("Unknown modifier: '{}'", part);
            }
        }

        let keysym = if key_part.len() == 1 {
            let ch = key_part.chars().next().unwrap();
            ch as KeySym
        } else {
            match self.keysym_map.get(key_part) {
                Some(sym) => *sym,
                None => {
                    warn!("Unknown key: '{}'", key_part);
                    return None;
                }
            }
        };

        debug!(
            "Parsed '{}' -> keysym={:#x}, modifiers={:#x}",
            key_expr, keysym, modifiers
        );
        Some((keysym, modifiers))
    }

    pub fn keycode_from_keysym(&self, keysym: KeySym) -> KeyCode {
        unsafe { xlib::XKeysymToKeycode(self.display, keysym) as KeyCode }
    }

    pub fn send_key(&self, window: xlib::Window, keysym: KeySym, modifiers: u32) {
        debug!(
            "Sending key: keysym={:#x}, modifiers={:#x} to window={}",
            keysym, modifiers, window
        );
        unsafe {
            let keycode = self.keycode_from_keysym(keysym);

            if keycode == 0 {
                warn!("Failed to get keycode for keysym {:#x}", keysym);
                return;
            }

            let mut event = XKeyEvent {
                type_: xlib::KeyPress,
                serial: 0,
                send_event: xlib::True,
                display: self.display,
                window,
                root: xlib::XDefaultRootWindow(self.display),
                subwindow: 0,
                time: xlib::CurrentTime,
                x: 1,
                y: 1,
                x_root: 1,
                y_root: 1,
                state: modifiers,
                keycode: keycode as u32,
                same_screen: xlib::True,
            };

            // Send key press
            let result = xlib::XSendEvent(
                self.display,
                window,
                xlib::True,
                xlib::KeyPressMask,
                &mut event as *mut XKeyEvent as *mut xlib::XEvent,
            );
            debug!("XSendEvent press result: {}", result);

            // Send key release
            event.type_ = xlib::KeyRelease;
            let result = xlib::XSendEvent(
                self.display,
                window,
                xlib::True,
                xlib::KeyReleaseMask,
                &mut event as *mut XKeyEvent as *mut xlib::XEvent,
            );
            debug!("XSendEvent release result: {}", result);

            xlib::XFlush(self.display);
        }
    }

    pub fn send_key_sequence(&self, window: xlib::Window, keys: &[String]) {
        debug!("Sending key sequence: {:?} to window={}", keys, window);
        for key in keys {
            if let Some((keysym, modifiers)) = self.parse_key(key) {
                self.send_key(window, keysym, modifiers);
            } else {
                warn!("Failed to parse key in sequence: '{}'", key);
            }
        }
    }
}
