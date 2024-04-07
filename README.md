# Fan controller for Clevo NS50MU

This is Rust version of fan controller for Clevo NS50MU that was inspired by C++ versions of same:
- [Vega D](https://gitlab.com/vega-d)'s [version](https://gitlab.com/vega-d/clevo-ns50mu-fan-controller).
- [François Kneib](https://gitlab.com/francois.kneib)'s [version](https://gitlab.com/francois.kneib/clevo-N151ZU-fan-controller).
- [My (Santeri Hetekivi)](https://github.com/SanteriHetekivi) [version](https://github.com/SanteriHetekivi/Clevo-NS50MU-fan-controller).

## Prerequisites

[Rust and cargo](https://www.rust-lang.org/tools/install).

## Installing

### Clone the repo:

```shell
git clone https://github.com/SanteriHetekivi/x11_edid_auto.git
```

### Go into the project folder:

```shell
cd clevo-ns50mu-fan-controller-rs
```

### Build & install:

```shell
make all
```

This will:

1. Build the [src/main.rs](src/main.rs) file into target/release/ns50mu_fan_controller_rs and make it executable,
2. Copy target/release/ns50mu_fan_controller_rs bin file into /usr/local/bin,
3. Copy the service file NS50MU-fan-controller.service into /etc/systemd/system/,
4. Enable the service at startup,
5. Start the service.

You can now check that the service is running:

```shell
systemctl status NS50MU-fan-controller.service
```