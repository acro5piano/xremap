# xremap-rust

A Rust rewrite of xremap - Dynamic key remapper for X Window System.

This is a complete rewrite of the original xremap project in Rust, replacing the mruby/C implementation with a pure Rust solution that uses YAML configuration files.

## Features

- **YAML Configuration**: Simple YAML configuration instead of Ruby DSL
- **Application-specific key bindings**: Different key mappings based on window class
- **Multi-key sequences**: Support for key combinations that execute multiple keys
- **Case-insensitive matching**: Window class matching is case-insensitive
- **Fast and lightweight**: Written in Rust for performance

## Installation

### Prerequisites

- Rust toolchain (1.70+)
- X11 development libraries (`libx11-dev` on Debian/Ubuntu)

### Build from source

```bash
cd rust/
RUSTFLAGS="-lX11" cargo build --release
sudo cp target/release/xremap /usr/local/bin/
```

## Usage

```bash
# Basic usage
xremap config.yaml

# With debug logging to troubleshoot issues
RUST_LOG=debug xremap config.yaml

# May require root privileges for key grabbing (depending on your X11 setup)
sudo xremap config.yaml
```

## Troubleshooting

### Key Grabbing Issues

If xremap starts but key remapping doesn't work, check the debug output:

```bash
RUST_LOG=debug ./target/debug/xremap example_config.yaml
```

**Success indicators:**
- Look for `Found handler for keycode=X, state=0xY, executing remap` - this means remapping is working
- `Grabbing N keys` should show a consistent number, not accumulating

**Common issues:**

1. **"Failed to grab key" warnings**: These warnings can often be ignored if you see "executing remap" messages. The warnings may appear due to X11 implementation details, but key grabbing often still works.

2. **No active window found**: 
   - Add a fallback rule in your config that applies to all windows
   - The window focus detection might need adjustment for your setup

3. **Key parsing failures**:
   - Check that key names in config match supported key names
   - Ensure modifier syntax is correct (C- for Ctrl, M- for Alt, etc.)

4. **Multiple window updates**: This is normal when switching between applications or when testing with tools like `xdotool`.

## Configuration

Configuration is done via YAML files. Here's the basic structure:

```yaml
windows:
  - class_only:
      - 'chromium'
      - 'firefox'
    remaps:
      - 'C-b': 'Left'
      - 'C-f': 'Right'
      - 'C-p': 'Up'
      - 'C-n': 'Down'
      - 'C-a': 'Home'
      - 'C-e': 'End'
      - 'C-M-a': 'Ctrl-Home'
      - 'C-M-e': 'Ctrl-End'
      - 'C-h': 'BackSpace'
      - 'C-d': 'Delete'
      - 'M-d': 'Ctrl-Delete'
      - 'C-M-h': 'Ctrl-BackSpace'
      - 'C-y': 'Ctrl-v'
      - 'M-b': 'Ctrl-Left'
      - 'M-f': 'Ctrl-Right'
      - 'C-k': ['Shift-End', 'Ctrl-x']
      - 'C-s': 'Ctrl-f'
  
  - class_not:
      - 'urxvt'
      - 'terminal'
    remaps:
      - 'C-w': ['Ctrl-Shift-Left', 'Ctrl-x']
      - 'C-u': ['Shift-Home', 'Ctrl-x']
```

### Configuration Options

#### Window Matching

- `class_only`: Array of window class names. Rules apply only to these applications (case-insensitive)
- `class_not`: Array of window class names. Rules apply to all applications except these (case-insensitive)

#### Key Notation

- `C-` or `Ctrl-`: Control key
- `M-` or `Alt-`: Alt key  
- `S-` or `Shift-`: Shift key
- `Super-`: Super/Windows key

#### Remapping

- Single key: `'C-b': 'Left'`
- Multiple keys: `'C-k': ['Shift-End', 'Ctrl-x']`

## Examples

### Emacs-like bindings for browsers

```yaml
windows:
  - class_only:
      - 'chromium'
      - 'firefox'
    remaps:
      - 'C-b': 'Left'
      - 'C-f': 'Right'
      - 'C-p': 'Up'
      - 'C-n': 'Down'
      - 'C-a': 'Home'
      - 'C-e': 'End'
```

### Terminal exclusions

```yaml
windows:
  - class_not:
      - 'urxvt'
      - 'gnome-terminal'
      - 'alacritty'
    remaps:
      - 'C-w': ['Ctrl-Shift-Left', 'Ctrl-x']
```

## Differences from Original xremap

1. **Configuration Format**: Uses YAML instead of Ruby DSL
2. **Implementation**: Pure Rust instead of mruby + C
3. **Dependencies**: No Ruby runtime required
4. **Performance**: Potentially faster due to Rust's performance characteristics

## Finding Window Class Names

To find the window class name for your application, run xremap and it will print window information to stdout when the active window changes.

## Building

The project requires linking with X11. Build with:

```bash
RUSTFLAGS="-lX11" cargo build --release
```

## License

This project maintains the same license as the original xremap project.

## Contributing

Contributions are welcome! Please feel free to submit pull requests or open issues.