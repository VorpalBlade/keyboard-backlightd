# `keyboard-backlightd`

[ [crates.io] ] [ [lib.rs] ] [ [AUR] ]

A daemon to turn off the keyboard backlight when idle.

Features:

* Small, minimal dependencies, fast to compile.
* Supports adjusting the keyboard backlight using the normal key (by
  monitoring if brightness was changed by hardware and adjusting accordingly).

Limitations:

* Only tested on modern ThinkPads.
* Very little auto-detection, you will need to configure your input devices
* Linux only. Uses some *very* low level APIs.

## Installation

While this package can be built with `cargo` as most rust packages, it uses
`make` for installation. This is because `cargo` is not able to install the
additional files that are needed: The config file and the systemd unit file.

Why is this needed? Since this is a system daemon it needs to be installed
system-wide and run as root (to be able to access certain files in `/dev`
and `/sys`).

Thus, you should ideally install this with your distro package manager. A
package for Arch Linux is available on [AUR] (maintained by the author of
this package).

If you do not use Arch Linux, I would appreciate if you can contribute
creating a package! However, if you do not want to create a package, do
something like this to install into `/usr/local`:

```bash
# NOTE! Not recommended! This will overwrite your config file for this
# program, which sucks if you customised it. Use your package manager
# instead!
make
sudo make install
```

## Dependencies

Keyboard-backlightd has a couple of dependencies:

* Rust (build time). This can either be installed from your distro's
  package manager or using [rustup](https://rustup.rs/).
* `libevdev` (build time and runtime, for reading input devices)

On some distros you may need to install a package with a name such as `libevdev-dev`
or `libevdev-devel` to get the development files needed to build software using `libevdev`.

Additionally, you may need to install basic build tools. These are usually provided
by a meta-package such as `base-devel` or `build-essential`, but refer to the
documentation for your Linux distro for details. You may also need to install
`pkgconf` or `pkg-config` to help the build process find the `libevdev` library.

The build process will fall back to building `libevdev` from source if it can't
be found on your system. If so you likely need to install additional dependencies,
please refer to the documentation of `libevdev`.

## Warning!

This program will monitor your key presses, but only to detect when you press
the keys. It does however not care about *what* you press. The only thing the
code checks is if it is a LED changing state, in which case it is ignored
(otherwise, pressing Caps Lock on an external keyboard would light up the
keyboard backlight).

In summary: This code works very similar to how a keylogger would, except that
it is benign. But you should read the code first to make sure you trust me with
this! (This is a good idea in general!)

The code that does this is in [`handlers.rs`](src/handlers.rs) in the `mod ev_dev` section.

## Figuring out input devices & LEDs

You probably want this daemon to turn on the keyboard backlight on any of:

* Keyboard keys
* Special Fn-key combos?
* Touchpad
* Trackpoint

The easiest way is to run `sudo evtest` (you may need to install a package
such as `evtest`). This will give you an output such as this:

```console
$ sudo evtest
No device specified, trying to scan all of /dev/input/event*
Available devices:
/dev/input/event0:  Sleep Button
/dev/input/event1:  Lid Switch
/dev/input/event10: ThinkPad Extra Buttons
/dev/input/event11: HDA Intel PCH Mic
/dev/input/event12: HDA Intel PCH Headphone
/dev/input/event13: HDA Intel PCH HDMI/DP,pcm=3
/dev/input/event14: HDA Intel PCH HDMI/DP,pcm=7
/dev/input/event15: HDA Intel PCH HDMI/DP,pcm=8
/dev/input/event16: Synaptics TM3276-022
/dev/input/event17: TPPS/2 IBM TrackPoint
/dev/input/event18: Integrated Camera: Integrated C
/dev/input/event19: MX Vertical Mouse
/dev/input/event2:  Power Button
/dev/input/event3:  AT Translated Set 2 keyboard
/dev/input/event4:  Video Bus
/dev/input/event5:  Video Bus
/dev/input/event6:  Microsoft Comfort Curve Keyboard 3000
/dev/input/event7:  Microsoft Comfort Curve Keyboard 3000
/dev/input/event8:  Raydium Corporation Raydium Touch System
/dev/input/event9:  PC Speaker
Select the device event number [0-19]:
```

This lets you identify the input devices Linux sees. However, this is not
quite what you want, as the numbering is not guaranteed to be stable across
reboots.

Next you need to find a stable ID for these:

```console
$ ls -l /dev/input/by-id                                
lrwxrwxrwx 1 root root  9  7 feb 16.31 usb-Microsoft_Comfort_Curve_Keyboard_3000-event-kbd -> ../event6
lrwxrwxrwx 1 root root  9  7 feb 16.31 usb-Microsoft_Comfort_Curve_Keyboard_3000-if01-event-kbd -> ../event7
lrwxrwxrwx 1 root root  9  7 feb 16.31 usb-Raydium_Corporation_Raydium_Touch_System-event-if00 -> ../event8
lrwxrwxrwx 1 root root 10  7 feb 16.31 usb-SunplusIT_Inc_Integrated_Camera-event-if00 -> ../event18
$ ls -l /dev/input/by-path
lrwxrwxrwx 1 root root  9  7 feb 16.31 pci-0000:00:14.0-usb-0:10:1.0-event -> ../event8
lrwxrwxrwx 1 root root  9  7 feb 16.31 pci-0000:00:14.0-usb-0:1:1.0-event-kbd -> ../event6
lrwxrwxrwx 1 root root  9  7 feb 16.31 pci-0000:00:14.0-usb-0:1:1.1-event-kbd -> ../event7
lrwxrwxrwx 1 root root 10  7 feb 16.31 pci-0000:00:14.0-usb-0:8:1.0-event -> ../event18
lrwxrwxrwx 1 root root 10  7 feb 16.31 pci-0000:00:1f.4-event-mouse -> ../event16
lrwxrwxrwx 1 root root  9  7 feb 16.31 pci-0000:00:1f.4-mouse -> ../mouse1
lrwxrwxrwx 1 root root 10  7 feb 16.31 pci-0000:00:1f.4-serio-2-event-mouse -> ../event17
lrwxrwxrwx 1 root root  9  7 feb 16.31 pci-0000:00:1f.4-serio-2-mouse -> ../mouse2
lrwxrwxrwx 1 root root  9  7 feb 16.31 platform-i8042-serio-0-event-kbd -> ../event3
lrwxrwxrwx 1 root root  9  7 feb 16.31 platform-pcspkr-event-spkr -> ../event9
lrwxrwxrwx 1 root root 10  7 feb 16.31 platform-thinkpad_acpi-event -> ../event10
```

Here you should prefer a `by-id` mapping if one exists for your device, though
for non-USB devices `by-path` will likely work fine.

From the output above we can conclude that we want:

| Device          | Human readable name                      | Number | Path we want                                                               |
|-----------------|------------------------------------------|--------|----------------------------------------------------------------------------|
| Keyboard        | AT Translated Set 2 keyboard             |      3 | `/dev/input/by-path/platform-i8042-serio-0-event-kbd`                      |
| Special buttons | ThinkPad Extra Buttons                   |     10 | `/dev/input/by-path/platform-thinkpad_acpi-event`                          |
| Touchpad        | Synaptics TM3276-022                     |     16 | `/dev/input/by-path/pci-0000:00:1f.4-event-mouse`                          |
| Trackpoint      | TPPS/2 IBM TrackPoint                    |     17 | `/dev/input/by-path/pci-0000:00:1f.4-serio-2-event-mouse`                  |
| Touchscreen     | Raydium Corporation Raydium Touch System |      8 | `/dev/input/by-id/usb-Raydium_Corporation_Raydium_Touch_System-event-if00` |

These are (for this laptop, a ThinkPad T480) all the inputs we want to monitor.

Here are some hints for figuring this out:

* The two most common vendors I have seen for touchpads are Synaptics and ALPS.
* You can always use evtest to check if key presses go to a specific device.

Thus, our command line for `keyboard-backlightd` would look like:

```text
-i /dev/input/by-path/platform-i8042-serio-0-event-kbd \
-i /dev/input/by-path/platform-thinkpad_acpi-event \
-i /dev/input/by-path/pci-0000:00:1f.4-event-mouse \
-i /dev/input/by-path/pci-0000:00:1f.4-serio-2-event-mouse \
-i /dev/input/by-id/usb-Raydium_Corporation_Raydium_Touch_System-event-if00
```

Then you also need to figure out the path to your LED for keyboard backlight.
This is thankfully much easier:

```console
$ ls -d /sys/class/leds/*kbd*
/sys/class/leds/tpacpi::kbd_backlight
```

In this case there was only one option. This means we want:

```text
-l /sys/class/leds/tpacpi::kbd_backlight
```

## Configuration

Once you have found your required command line parameters, edit
`/etc/conf.d/keyboard-backlightd` to set them for use in the systemd service.

Then enable and start the systemd service and make sure everything still works:

```bash
systemctl enable keyboard-backlightd
systemctl start keyboard-backlightd
```

If you have issues with the service not starting during boot, consider
increasing `WAIT` in the configuration file. This might help with late loaded
kernel modules causing the device node to not be available when the daemon
starts.

## Troubleshooting

### The keyboard backlight stops turning on if I hit the key that manually controls the backlight

This is by design. The daemon monitors (on supported laptops) for external
changes to the keyboard backlight. That could be other software or the user
pressing some laptop specific key combo (often involving the `Fn` key). On
modern Thinkpads (2020-ish) this key combo is `Fn+Space` which cycles
between "off", "half" and "bright".

`keyboard-backlightd` will adapt to what the user sets and toggle between
"off" and whatever brightness level the user just set. If you happen to set
it to off, that means you will get toggling between "off" and "off". Just
hit the key combo again to resume operation at the next level of brightness.

If you really don't want this, you can use the flag `--no-adaptive-brightness`
to the daemon. But consider giving it a try first, you might grow to like it.

## Minimum supported rust version (MSRV)

Currently, at least Rust 1.82.0 is needed, but this may change at any time
if needed. MSRV change is not considered a breaking change and as such may
change even in a patch version.

## License

`keyboard-backlightd` is released under GPL 3.0 (only, not "or later").

[AUR]: https://aur.archlinux.org/packages/keyboard-backlightd
[crates.io]: https://crates.io/crates/keyboard-backlightd
[lib.rs]: https://lib.rs/crates/keyboard-backlightd
