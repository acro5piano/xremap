use crate::config::{Config, KeyAction, Remap};
use crate::key_mapper::{KeyMapper, KeyPress};
use crate::window_manager::WindowManager;
use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::rc::Rc;
use std::thread;
use std::time::Duration;
use x11::xlib::{self, Display, KeyCode, Window};

pub struct EventHandler {
    display: *mut Display,
    config: Config,
    window_manager: WindowManager,
    key_mapper: KeyMapper,
    key_handlers: HashMap<KeyPress, Rc<dyn Fn()>>,
    grabbed_keys: Vec<KeyPress>,
}

impl EventHandler {
    pub fn new(display: *mut Display, config: Config) -> Self {
        let window_manager = WindowManager::new(display);
        let key_mapper = KeyMapper::new(display);

        Self {
            display,
            config,
            window_manager,
            key_mapper,
            key_handlers: HashMap::new(),
            grabbed_keys: Vec::new(),
        }
    }

    pub fn initialize(&mut self) {
        info!("Initializing event handler");
        self.update_key_mappings();
        info!("Event handler initialization complete");
    }

    pub fn handle_key_press(&mut self, keycode: KeyCode, state: u32) {
        let filtered_state =
            state & (xlib::ControlMask | xlib::ShiftMask | xlib::Mod1Mask | xlib::Mod4Mask);
        let key_press = KeyPress {
            keycode,
            modifiers: filtered_state,
        };

        debug!(
            "Handling key press: keycode={}, state={:#x}, filtered_state={:#x}",
            keycode, state, filtered_state
        );

        if let Some(handler) = self.key_handlers.get(&key_press) {
            info!(
                "Found handler for keycode={}, state={:#x}, executing remap",
                keycode, filtered_state
            );
            handler();
        } else {
            debug!(
                "No handler found for keycode={}, state={:#x}",
                keycode, filtered_state
            );
            debug!(
                "Available handlers: {:?}",
                self.key_handlers.keys().collect::<Vec<_>>()
            );
        }
    }

    pub fn handle_property_notify(&mut self) {
        // Add delay similar to original implementation
        thread::sleep(Duration::from_millis(100));

        if self.window_manager.has_window_changed() {
            info!("Active window changed, updating key mappings");
            self.update_key_mappings();
        }
    }

    pub fn handle_mapping_notify(&mut self) {
        self.update_key_mappings();
    }

    fn update_key_mappings(&mut self) {
        debug!("Updating key mappings");
        self.ungrab_all_keys();
        self.key_handlers.clear();
        self.grabbed_keys.clear(); // Clear the grabbed keys list to prevent duplicates

        let active_window = self.window_manager.get_active_window();
        let window_class = active_window.and_then(|w| self.window_manager.get_window_class(w));

        info!(
            "Active window: {:?}, class: {:?}",
            active_window, window_class
        );

        let remaps = self.config.remaps_for_window(window_class.as_deref());
        info!("Found {} remaps for current window", remaps.len());

        for remap in remaps {
            debug!("Registering remap: {} -> {:?}", remap.from, remap.to);
            self.register_remap(remap, active_window);
        }

        info!("Grabbing {} keys", self.grabbed_keys.len());
        self.grab_keys();
    }

    fn register_remap(&mut self, remap: Remap, target_window: Option<Window>) {
        if let Some((from_keysym, from_mods)) = self.key_mapper.parse_key(&remap.from) {
            let keycode = self.key_mapper.keycode_from_keysym(from_keysym);
            let key_press = KeyPress {
                keycode,
                modifiers: from_mods,
            };

            debug!(
                "Registering remap: '{}' (keysym={:#x}, mods={:#x}) -> keycode={}, to={:?}",
                remap.from, from_keysym, from_mods, keycode, remap.to
            );

            if keycode == 0 {
                warn!(
                    "Failed to get keycode for keysym {:#x} (key '{}')",
                    from_keysym, remap.from
                );
                return;
            }

            let key_mapper = KeyMapper::new(self.display);
            let window = target_window.unwrap_or(unsafe { xlib::XDefaultRootWindow(self.display) });

            let handler: Rc<dyn Fn()> = match remap.to {
                KeyAction::Single(key) => {
                    let key_clone = key.clone();
                    Rc::new(move || {
                        debug!("Executing single key remap: {}", key_clone);
                        if let Some((keysym, mods)) = key_mapper.parse_key(&key_clone) {
                            key_mapper.send_key(window, keysym, mods);
                        } else {
                            warn!("Failed to parse target key: {}", key_clone);
                        }
                    })
                }
                KeyAction::Multiple(keys) => {
                    let keys_clone = keys.clone();
                    Rc::new(move || {
                        debug!("Executing multi-key remap: {:?}", keys_clone);
                        key_mapper.send_key_sequence(window, &keys_clone);
                    })
                }
            };

            // Only add if not already present
            if !self.grabbed_keys.contains(&key_press) {
                self.grabbed_keys.push(key_press);
            }
            self.key_handlers.insert(key_press, handler);
            debug!(
                "Successfully registered handler for keycode={}, mods={:#x}",
                keycode, from_mods
            );
        } else {
            warn!("Failed to parse key expression: '{}'", remap.from);
        }
    }

    fn grab_keys(&self) {
        unsafe {
            let root = xlib::XDefaultRootWindow(self.display);

            for key_press in &self.grabbed_keys {
                debug!(
                    "Grabbing key: keycode={}, modifiers={:#x}",
                    key_press.keycode, key_press.modifiers
                );

                let grab_result = xlib::XGrabKey(
                    self.display,
                    key_press.keycode as i32,
                    key_press.modifiers,
                    root,
                    xlib::True,
                    xlib::GrabModeAsync,
                    xlib::GrabModeAsync,
                );

                if grab_result != 0 {
                    debug!("Failed to grab key: keycode={}, modifiers={:#x} (this is usually due to X11 permissions or another app using the key)", 
                          key_press.keycode, key_press.modifiers);
                } else {
                    debug!(
                        "Successfully grabbed key: keycode={}, modifiers={:#x}",
                        key_press.keycode, key_press.modifiers
                    );
                }

                // Also grab with NumLock
                xlib::XGrabKey(
                    self.display,
                    key_press.keycode as i32,
                    key_press.modifiers | xlib::Mod2Mask,
                    root,
                    xlib::True,
                    xlib::GrabModeAsync,
                    xlib::GrabModeAsync,
                );

                // Also grab with CapsLock
                xlib::XGrabKey(
                    self.display,
                    key_press.keycode as i32,
                    key_press.modifiers | xlib::LockMask,
                    root,
                    xlib::True,
                    xlib::GrabModeAsync,
                    xlib::GrabModeAsync,
                );

                // Also grab with both NumLock and CapsLock
                xlib::XGrabKey(
                    self.display,
                    key_press.keycode as i32,
                    key_press.modifiers | xlib::Mod2Mask | xlib::LockMask,
                    root,
                    xlib::True,
                    xlib::GrabModeAsync,
                    xlib::GrabModeAsync,
                );
            }

            xlib::XFlush(self.display);
        }
    }

    fn ungrab_all_keys(&self) {
        debug!("Ungrabbing all keys");
        unsafe {
            let root = xlib::XDefaultRootWindow(self.display);
            xlib::XUngrabKey(self.display, xlib::AnyKey, xlib::AnyModifier, root);
            xlib::XFlush(self.display);
        }
    }
}
