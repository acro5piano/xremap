use log::{debug, warn};
use std::ffi::CStr;
use std::os::raw::{c_char, c_int, c_ulong};
use std::ptr;
use x11::xlib::{self, Display, Window, XTextProperty};

pub struct WindowManager {
    display: *mut Display,
    root_window: Window,
    current_window: Option<Window>,
    wm_class_atom: c_ulong,
    net_active_window_atom: c_ulong,
}

impl WindowManager {
    pub fn new(display: *mut Display) -> Self {
        unsafe {
            let root_window = xlib::XDefaultRootWindow(display);
            let wm_class_atom = xlib::XInternAtom(
                display,
                b"WM_CLASS\0".as_ptr() as *const c_char,
                xlib::True,
            );
            let net_active_window_atom = xlib::XInternAtom(
                display,
                b"_NET_ACTIVE_WINDOW\0".as_ptr() as *const c_char,
                xlib::True,
            );
            
            Self {
                display,
                root_window,
                current_window: None,
                wm_class_atom,
                net_active_window_atom,
            }
        }
    }
    
    pub fn get_active_window(&mut self) -> Option<Window> {
        unsafe {
            let mut window: Window = 0;
            let mut revert_to: c_int = 0;
            
            xlib::XGetInputFocus(self.display, &mut window, &mut revert_to);
            debug!("XGetInputFocus returned window={}, revert_to={}", window, revert_to);
            
            if window != 0 && window != 1 {
                self.current_window = Some(window);
                Some(window)
            } else {
                debug!("No valid active window found, using root window");
                // Return root window as fallback so we can still grab keys
                self.current_window = Some(self.root_window);
                Some(self.root_window)
            }
        }
    }
    
    pub fn get_window_class(&self, window: Window) -> Option<String> {
        debug!("Getting window class for window={}", window);
        unsafe {
            let mut prop = XTextProperty {
                value: ptr::null_mut(),
                encoding: 0,
                format: 0,
                nitems: 0,
            };
            
            let mut search_window = window;
            let mut depth = 0;
            
            loop {
                debug!("Searching window={} (depth={})", search_window, depth);
                let status = xlib::XGetTextProperty(
                    self.display,
                    search_window,
                    &mut prop,
                    self.wm_class_atom,
                );
                
                if status != 0 && prop.nitems > 0 && !prop.value.is_null() {
                    debug!("Found WM_CLASS property with {} items", prop.nitems);
                    break;
                }
                
                let mut root: Window = 0;
                let mut parent: Window = 0;
                let mut children: *mut Window = ptr::null_mut();
                let mut n_children: u32 = 0;
                
                let query_status = xlib::XQueryTree(
                    self.display,
                    search_window,
                    &mut root,
                    &mut parent,
                    &mut children,
                    &mut n_children,
                );
                
                if !children.is_null() {
                    xlib::XFree(children as *mut _);
                }
                
                if query_status == 0 || parent == 0 || parent == root {
                    debug!("Reached root or query failed, stopping search");
                    return None;
                }
                
                search_window = parent;
                depth += 1;
                
                if depth > 20 {
                    warn!("Window class search depth exceeded 20, stopping");
                    return None;
                }
            }
            
            if prop.nitems > 0 && !prop.value.is_null() {
                let class_str = CStr::from_ptr(prop.value as *const c_char)
                    .to_string_lossy()
                    .into_owned();
                
                debug!("Found window class: '{}'", class_str);
                
                if !prop.value.is_null() {
                    xlib::XFree(prop.value as *mut _);
                }
                
                Some(class_str)
            } else {
                debug!("No window class found");
                None
            }
        }
    }
    
    pub fn has_window_changed(&mut self) -> bool {
        let new_window = self.get_active_window();
        
        if self.current_window != new_window {
            debug!("Window changed: {:?} -> {:?}", self.current_window, new_window);
            self.current_window = new_window;
            true
        } else {
            false
        }
    }
}