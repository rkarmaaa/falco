// Su Windows il menu del tray (muda) richiede Common Controls v6 tramite manifest
use embed_manifest::manifest::{ActiveCodePage, DpiAwareness, Setting};
use embed_manifest::{embed_manifest, new_manifest};

fn main() {
    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        let manifest = new_manifest("Falo.Core")
            .active_code_page(ActiveCodePage::Utf8)
            .dpi_awareness(DpiAwareness::PerMonitorV2)
            .long_path_aware(Setting::Enabled);
        if let Err(e) = embed_manifest(manifest) {
            panic!("manifest non incorporato: {e}");
        }
    }
    println!("cargo:rerun-if-changed=build.rs");
}
