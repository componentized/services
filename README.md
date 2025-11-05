# Services Components <!-- omit in toc -->

A WASM component services lifecycle.

- [Build](#build)
- [Run](#run)
- [Community](#community)
  - [Code of Conduct](#code-of-conduct)
  - [Communication](#communication)
  - [Contributing](#contributing)
- [Acknowledgements](#acknowledgements)
- [License](#license)


## Build

Prereqs:
- a rust toolchain with `wasm32-unknown-unknown` and `wasm32-wasip2` targets (`rustup target add wasm32-unknown-unknown` and `rustup target add wasm32-wasip2`)
- [`cargo component`](https://github.com/bytecodealliance/cargo-component)
- [`static-config`](https://github.com/componentized/static-config)
- [`wac`](https://github.com/bytecodealliance/wac)
- [`wkg`](https://github.com/bytecodealliance/wasm-pkg-tools)

```sh
./update-deps.sh
./build.sh
```

## Run

Prereqs:
- build the components (see above)
- access to a running [Valkey](https://valkey.io) server
- [`wasmtime`](https://github.com/bytecodealliance/wasmtime) 41+ or dev

```sh
./demo.sh
```

## Community

### Code of Conduct

The Componentized project follow the [Contributor Covenant Code of Conduct](./CODE_OF_CONDUCT.md). In short, be kind and treat others with respect.

### Communication

General discussion and questions about the project can occur in the project's [GitHub discussions](https://github.com/orgs/componentized/discussions).

### Contributing

The Componentized project team welcomes contributions from the community. A contributor license agreement (CLA) is not required. You own full rights to your contribution and agree to license the work to the community under the Apache License v2.0, via a [Developer Certificate of Origin (DCO)](https://developercertificate.org). For more detailed information, refer to [CONTRIBUTING.md](CONTRIBUTING.md).

## Acknowledgements

This project was conceived in discussion between [Mark Fisher](https://github.com/markfisher) and [Scott Andrews](https://github.com/scothis).

## License

Apache License v2.0: see [LICENSE](./LICENSE) for details.
