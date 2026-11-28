fn main() {
    glib_build_tools::compile_resources(
        &["../../data/icons/hicolor/scalable/apps"],
        "../../data/resources.gresource.xml",
        "argento.gresource",
    );
}
