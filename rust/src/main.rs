mod config;
mod event_handler;
mod key_mapper;
mod window_manager;

use anyhow::{Context, Result};
use config::Config;
use event_handler::EventHandler;
use log::{debug, error, info, warn};
use std::env;
use std::fs;
use std::os::raw::c_int;
use std::ptr;
use x11::xlib::{self, Display, XErrorEvent, XEvent};

static mut ERROR_OCCURED: bool = false;

extern "C" fn error_handler(_display: *mut Display, event: *mut XErrorEvent) -> c_int {
    unsafe {
        ERROR_OCCURED = true;
        error!(
            "X11 Error: code={}, request={}, minor={}",
            (*event).error_code,
            (*event).request_code,
            (*event).minor_code
        );
    }
    0
}

fn main() -> Result<()> {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <config.yaml>", args[0]);
        std::process::exit(1);
    }

    info!("Starting xremap with config: {}", args[1]);

    let config_content = fs::read_to_string(&args[1])
        .with_context(|| format!("Failed to read config file: {}", args[1]))?;

    let config = Config::from_yaml(&config_content).context("Failed to parse config file")?;

    info!("Loaded config with {} window rules", config.windows.len());
    for (i, window) in config.windows.iter().enumerate() {
        info!(
            "Window rule {}: class_only={:?}, class_not={:?}, remaps={}",
            i,
            window.class_only,
            window.class_not,
            window.remaps.len()
        );
    }

    unsafe {
        let display = xlib::XOpenDisplay(ptr::null());
        if display.is_null() {
            anyhow::bail!("Failed to open X display");
        }
        info!("Successfully opened X display");

        xlib::XSetErrorHandler(Some(error_handler));

        let root = xlib::XDefaultRootWindow(display);
        xlib::XSelectInput(
            display,
            root,
            xlib::KeyPressMask | xlib::PropertyChangeMask | xlib::SubstructureNotifyMask,
        );

        let mut event_handler = EventHandler::new(display, config);
        event_handler.initialize();

        info!("xremap initialized successfully");
        println!("xremap started. Listening for key events...");
        println!("Press Ctrl-C to quit");
        println!("Set RUST_LOG=debug for verbose output");

        let mut event: XEvent = std::mem::zeroed();

        loop {
            xlib::XNextEvent(display, &mut event);

            match event.get_type() {
                xlib::KeyPress => {
                    let key_event = event.key;
                    debug!(
                        "KeyPress: keycode={}, state={}",
                        key_event.keycode, key_event.state
                    );
                    event_handler.handle_key_press(key_event.keycode as u8, key_event.state);
                }
                xlib::PropertyNotify => {
                    debug!("PropertyNotify event");
                    event_handler.handle_property_notify();
                }
                xlib::MappingNotify => {
                    debug!("MappingNotify event");
                    event_handler.handle_mapping_notify();
                }
                xlib::ClientMessage => {
                    let client_event = event.client_message;
                    debug!(
                        "ClientMessage: type={}, format={}",
                        client_event.message_type, client_event.format
                    );
                }
                _ => {
                    debug!("Unhandled event type: {}", event.get_type());
                }
            }

            if ERROR_OCCURED {
                ERROR_OCCURED = false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_parsing() {
        let yaml = r#"
windows:
  - class_only:
      - 'chromium'
    remaps:
      - 'C-b': 'Left'
      - 'C-f': 'Right'
"#;

        let config = Config::from_yaml(yaml).unwrap();
        assert_eq!(config.windows.len(), 1);
        assert_eq!(config.windows[0].remaps.len(), 2);
    }
}
