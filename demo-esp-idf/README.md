# racegate-esp-idf

**racegate** implementation for *esp-idf* framework.

## Setup

### Prerequisites

You need nightly Rust and `espflash`. To install both of them, just use the
commands below:

```shell
rustup install nightly-2023-04-04
rustup component add rust-src --toolchain nightly-2023-04-04
cargo install cargo-espflash ldproxy
```

## Development

### Flash

To build, flash and see logs, you can use `cargo espflash`:

```shell
cargo espflash --speed 1500000 --monitor /dev/ttyACM0
```

If the firmware does not fit the flash, you can use a more generous partition
table:

```shell
cargo espflash --speed 1500000 --monitor /dev/ttyACM0 \
  --partition-table partitions/partitions_singleapp_4MB.csv
```

For a release build, just add `--release`

```shell
cargo espflash --speed 1500000 --release --monitor /dev/ttyACM0
```

### Debugging

#### Built in JTAG interface

Not all ESP32-C3 have a built-in JTAG interface. Example:

| Model                                                     | JTAG |
|-----------------------------------------------------------|------|
| [ESP32-C3-DevKit-RUST-1](https://mou.sr/40F3w6d)          | Yes  |
| [M5Stamp C3U](https://docs.m5stack.com/en/core/stamp_c3u) | Yes  |
| [M5Stamp C3](https://docs.m5stack.com/en/core/stamp_c3u)  | No   |

#### External JTAG interface

You need an *ESP-Prog Board*.

Reference: https://espressif-docs.readthedocs-hosted.com/projects/espressif-esp-iot-solution/en/latest/hw-reference/ESP-Prog_guide.html

Connect the JTAG interface to ESP32C3:

| JTAG | ESP32C3 |
|------|---------|
| TMS  | GPIO4   |
| TDI  | GPIO5   |
| TCK  | GPIO6   |
| TDO  | GPIO7   |

**TODO** (burn efuse to enable jtag?)

#### gdb and openocd

While `esp-idf` is automatically installed by cargo, you need `openocd`
and `riscv32-elf-gdb`.

On Arch Linux:

```shell
yay -S openocd-esp32 riscv32-elf-gdb
```

Make sure local gdbinit is enabled in `~/.gdbinit`:

```text
set auto-load local-gdbinit on
add-auto-load-safe-path /
```

#### Launch openocd

Launch `openocd`:

```shell
openocd-esp32openocd -f board/esp32c3-builtin.cfg
```

If you have permission problems accessing the device, just use `sudo` (or create
the correct udev rules).

#### CLion

Configure CLion to use remote gdb

- Edit configurations
- Add "Remote debug"
- Select the debugger: Custom GDB Executable -> /usr/bin/riscv32-elf-gdb
- Target remote args: `:3333`
- Select symbol
  file `/path/to/project/target/riscv32imc-esp-espidf/debug/elf-file` (change
  path to project and elf file name)

To check if the debugger is working, just put a breakpoint at the beginning of
main and start the Debug configuration.
