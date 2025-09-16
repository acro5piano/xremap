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
            let wm_class_atom =
                xlib::XInternAtom(display, b"WM_CLASS\0".as_ptr() as *const c_char, xlib::True);
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
            // Method 1: Try _NET_ACTIVE_WINDOW first
            let mut actual_type: c_ulong = 0;
            let mut actual_format: c_int = 0;
            let mut nitems: c_ulong = 0;
            let mut bytes_after: c_ulong = 0;
            let mut prop_data: *mut u8 = ptr::null_mut();

            let result = xlib::XGetWindowProperty(
                self.display,
                self.root_window,
                self.net_active_window_atom,
                0,
                1,
                xlib::False,
                xlib::XA_WINDOW,
                &mut actual_type,
                &mut actual_format,
                &mut nitems,
                &mut bytes_after,
                &mut prop_data,
            );

            if result == xlib::Success as i32 && !prop_data.is_null() && nitems > 0 {
                let window = *(prop_data as *const Window);
                debug!("_NET_ACTIVE_WINDOW returned window={}", window);
                xlib::XFree(prop_data as *mut _);
                if window != 0 && window != self.root_window {
                    self.current_window = Some(window);
                    return Some(window);
                }
            } else if !prop_data.is_null() {
                xlib::XFree(prop_data as *mut _);
            }

            // Method 2: XGetInputFocus fallback
            let mut window: Window = 0;
            let mut revert_to: c_int = 0;

            xlib::XGetInputFocus(self.display, &mut window, &mut revert_to);
            debug!(
                "XGetInputFocus returned window={}, revert_to={}",
                window, revert_to
            );

            if window != 0 && window != 1 && window != self.root_window {
                self.current_window = Some(window);
                Some(window)
            } else {
                debug!("No valid active window found, trying to find focused window manually");
                // Method 3: Try to find a window with input focus by checking children
                if let Some(focused) = self.find_focused_window(self.root_window) {
                    debug!("Found focused window via tree search: {}", focused);
                    self.current_window = Some(focused);
                    Some(focused)
                } else {
                    debug!("Using root window as fallback");
                    self.current_window = Some(self.root_window);
                    Some(self.root_window)
                }
            }
        }
    }

    pub fn get_window_class(&self, window: Window) -> Option<String> {
        debug!("Getting window class for window={}", window);
        unsafe {
            // First try direct property lookup without climbing the tree
            if let Some(class) = self.try_get_class_direct(window) {
                debug!("Found class directly: '{}'", class);
                return Some(class);
            }

            // If that fails, climb the window tree
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

                // Try WM_CLASS first
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

                // If WM_CLASS failed, try getting window name as fallback
                let mut name_prop = XTextProperty {
                    value: ptr::null_mut(),
                    encoding: 0,
                    format: 0,
                    nitems: 0,
                };

                let name_status = xlib::XGetWMName(self.display, search_window, &mut name_prop);
                if name_status != 0 && name_prop.nitems > 0 && !name_prop.value.is_null() {
                    debug!(
                        "Found WM_NAME property as fallback with {} items",
                        name_prop.nitems
                    );
                    prop = name_prop;
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
            debug!(
                "Window changed: {:?} -> {:?}",
                self.current_window, new_window
            );
            self.current_window = new_window;
            true
        } else {
            false
        }
    }

    fn try_get_class_direct(&self, window: Window) -> Option<String> {
        unsafe {
            // Try multiple property types commonly used for window class
            let properties = [
                self.wm_class_atom,
                xlib::XInternAtom(
                    self.display,
                    b"_NET_WM_NAME\0".as_ptr() as *const c_char,
                    xlib::False,
                ),
                xlib::XInternAtom(
                    self.display,
                    b"WM_NAME\0".as_ptr() as *const c_char,
                    xlib::False,
                ),
            ];

            for &atom in &properties {
                let mut prop = XTextProperty {
                    value: ptr::null_mut(),
                    encoding: 0,
                    format: 0,
                    nitems: 0,
                };

                let status = xlib::XGetTextProperty(self.display, window, &mut prop, atom);

                if status != 0 && prop.nitems > 0 && !prop.value.is_null() {
                    let result = if prop.encoding == xlib::XA_STRING {
                        CStr::from_ptr(prop.value as *const c_char)
                            .to_string_lossy()
                            .into_owned()
                    } else {
                        let mut list: *mut *mut c_char = ptr::null_mut();
                        let mut count: c_int = 0;
                        let convert_status = xlib::XmbTextPropertyToTextList(
                            self.display,
                            &prop,
                            &mut list,
                            &mut count,
                        );

                        if convert_status == xlib::Success as i32 && count > 0 && !list.is_null() {
                            let first_str = *list;
                            let result = CStr::from_ptr(first_str).to_string_lossy().into_owned();
                            xlib::XFreeStringList(list);
                            result
                        } else {
                            String::new()
                        }
                    };

                    xlib::XFree(prop.value as *mut _);

                    if !result.is_empty() {
                        debug!("Found property value: '{}' from atom {}", result, atom);
                        return Some(result);
                    }
                }
            }

            None
        }
    }

    fn find_focused_window(&self, parent: Window) -> Option<Window> {
        unsafe {
            let mut root: Window = 0;
            let mut parent_return: Window = 0;
            let mut children: *mut Window = ptr::null_mut();
            let mut n_children: u32 = 0;

            let status = xlib::XQueryTree(
                self.display,
                parent,
                &mut root,
                &mut parent_return,
                &mut children,
                &mut n_children,
            );

            if status == 0 || children.is_null() {
                return None;
            }

            let children_slice = std::slice::from_raw_parts(children, n_children as usize);

            for &child in children_slice {
                // Check if this window has WM_CLASS (indicates it's a real application window)
                if self.try_get_class_direct(child).is_some() {
                    debug!("Found window with class: {}", child);
                    xlib::XFree(children as *mut _);
                    return Some(child);
                }

                // Recursively search children
                if let Some(focused) = self.find_focused_window(child) {
                    xlib::XFree(children as *mut _);
                    return Some(focused);
                }
            }

            xlib::XFree(children as *mut _);
            None
        }
    }
}
