# Update device tree to use devices

Welcome to our instructional guide for updating device tree for our NVIDIA Jetson Xavier NX / Orin NX compatible carrier board. This program helps you to enable microSD card slot, CSI camera dual lane support and fan tachometer.

This program is written in Rust 2021 and precompiled with rustc 1.70.0 (90c541806 2023-05-31)

### Description

* Read extlinux.conf and create backup dtb for default boot menu.
* Decompile device tree blob.
* (Xavier NX Only) Enable microSD slot. GPIO01(PQ.05) will be used as SD_CD for sd detection.
* Apply patch for CSI Camera dual lane support.
* Apply patch for cooling fan tachometer.
* Compile modified device tree specification.
* Add new boot menu with patched device tree blob.

### Quick install

```
$ git clone https://github.com/sg-jaehyeon/dtb_patcher.git
$ cd dtb_patcher
$ sudo ./patcher_prebuilt
```

### Modify source code

Jetson Xavier NX / Orin NX is needed to install curl

```
$ sudo apt update
$ sudo apt install -y curl
```

Install Rust

```
$ curl --proto '=https' --tlsv1.2 https://sh.rustup.rs -sSf | sh
```

Modify source code and build

```
$ cargo build
$ sudo ./target/debug/dtb_patcher
```

