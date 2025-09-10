# About

Pandabox is a Java Password manager I wrote while in Uni. The software has significant short comings and this project is an attempt to learn Rust by overhauling Pandabox. The primary components with revolve around the argo2, chacha20poly1305 and Slint frameworks. 

As per the original project a local sqlite 3 database is used to stored the encrypted data. 

## About

The core principle idea of this project is to try and offer a safe, robust and performant local client for storing a large variety of text based information. The initial goal is to enable users to store passwords for various services.
However, I hope to be able to expand that to memo's and other important information that people may want to store secretely and safely behind encyrption. The ability to leverage cloud storage for synchronization between devices is also
a neat feature that would be fantastic to have in this version. 

## Usage

1. Install Rust by following its [getting-started guide](https://www.rust-lang.org/learn/get-started).
   Once this is done, you should have the `rustc` compiler and the `cargo` build system installed in your `PATH`.
2. Download and extract the [ZIP archive of this repository](https://github.com/slint-ui/slint-rust-template/archive/refs/heads/main.zip).
3. Rename the extracted directory and change into it:
    ```
    mv Pandabox
    cd Pandabox  
    ```
4. Build with `cargo`:
    ```
    cargo build
    ```
5. Run the application binary:
    ```
    cargo run
    ```

We recommend using an IDE for development, along with our [LSP-based IDE integration for `.slint` files](https://github.com/slint-ui/slint/blob/master/tools/lsp/README.md). You can also load this project directly in [Visual Studio Code](https://code.visualstudio.com) and install our [Slint extension](https://marketplace.visualstudio.com/items?itemName=Slint.slint).

## Next Steps

## Attributions

Icons provided by Anastassiya Motokhova over at [flaticon.com](https://www.flaticon.com/authors/anastassiya-motokhova) 
