# QWENCH

![image info](./img/play.png)

## Installation

### Clone and build without install (assumes Rust Cargo is installed)

```bash
git clone https://github.com/BitPusher16/qwench.git
cd qwench
cargo build --release
cp ./target/release/qwench .
```

### Clone, build, and install (assumes Rust Cargo is installed)

```bash
git clone https://github.com/BitPusher16/qwench.git
cd qwench
cargo install --path .
```

### Build and install without clone (assumes Rust Cargo is installed)
```bash
cargo install --git https://github.com/BitPusher16/qwench.git
```

### Download precompiled binary

Go to the [latest release](https://github.com/BitPusher16/qwench/releases/latest) and download the binary for your OS.

### Command Line Args

```bash
$ ./qwench -h
QWENCH - Terminal typing game

Set letter-multiple and symbol-multiple to adjust the ratio of symbols to letters.

Usage: qwench [OPTIONS]

Options:
  -c, --chars-per-min       Characters per minute              (default: 400, range: 100..=2000)
  -g, --game-length-sec     Game length in seconds             (default: 180, range: 30..=600)
  -l, --letter-multiple     Letter frequency                   (default: 8, range: 1..=20)
  -s, --symbol-multiple     Symbol frequency                   (default: 1, range: 1..=20)
  -h, --help                Show this help

Example:
  qwench --chars-per-minute 400 --game-length-sec 240 --letter-multiple 4 --symbol-multiple 3
```

