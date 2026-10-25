fn main() {
    slint_build::compile("ui/main.slint").expect("Slint build failed");
    // Embed the app icon and FileVersion/ProductVersion into the Windows
    // executable (shown by Windows in the file properties dialog).
    embed_resource::compile("resources/version.rc", embed_resource::NONE)
        .manifest_required();
}
