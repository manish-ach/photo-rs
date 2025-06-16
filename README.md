# 📸 photo-rs

`photo-rs` is a Rust + WebAssembly (WASM) utility for converting PNG images to JPEG format in the browser with high performance.

## 🚀 Features

- ⚡ Powered by Rust for performance
- 🕸️ Compiles to WebAssembly to run in the browser
- 🖼️ Converts PNG to JPEG (`Uint8Array -> Uint8Array`)
- 🧪 Easy to integrate with JavaScript/TypeScript frontends

## 🛠️ Usage

### 1. Install dependencies and build:

```sh
wasm-pack build --target web
```

This generates a `pkg/` directory with WebAssembly bindings and JavaScript glue code.

### 2. JavaScript integration

Import and use the WASM module:
note: i am lazy so havent done myuch with the js front

```js
import init, { convert_png_to_jpg } from './pkg/photo_rs.js';

async function runConversion(pngBuffer) {
    await init(); // load the WASM module

    const jpegBuffer = convert_png_to_jpg(pngBuffer);
    console.log(jpegBuffer); // Uint8Array containing JPEG bytes
}
```

> Make sure to serve the `.wasm` file with the correct MIME type (`application/wasm`).

---

## 🧩 WebAssembly Interface (Rust)

```rust
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn convert_png_to_jpg(png_bytes: &[u8]) -> Result<Vec<u8>, JsValue> {
    // decoding and re-encoding logic here
}
```

---

## 📦 Project Structure

```
photo-rs/
├── src/
│   └── lib.rs             # Core Rust logic
├── pkg/                   # Output from wasm-pack
│   ├── photo_rs_bg.wasm   # WASM binary (generated)
│   ├── photo_rs.js        # JS bindings (generated)
└── README.md
```

---

## 🧪 Development Setup

1. Install [Rust](https://www.rust-lang.org/)
2. Install `wasm-pack`:

```sh
cargo install wasm-pack
```

3. Run build:

```sh
wasm-pack build --target web
```

---

## 🙏 Credits

Thanks to [wasm-bindgen](https://github.com/rustwasm/wasm-bindgen) and [wasm-pack](https://github.com/rustwasm/wasm-pack) teams.

```
