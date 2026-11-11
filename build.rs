//! Embute no binário todos os mapas de assets/maps (inclusive os criados no editor nativo)
//! e o manifesto world.json. No navegador não há sistema de arquivos: assim o jogo web
//! leva junto tudo o que estava na pasta quando foi compilado.

use std::fmt::Write as _;
use std::path::Path;

fn main() {
    let dir = Path::new("assets/maps");
    println!("cargo:rerun-if-changed=assets/maps");
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .expect("assets/maps")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "txt"))
        .filter_map(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .collect();
    names.sort();

    let root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let mut out = String::from("/// Gerado pelo build.rs: (id, texto) de cada mapa em assets/maps.\n");
    out.push_str("pub const EMBEDDED_MAPS: &[(&str, &str)] = &[\n");
    for name in &names {
        println!("cargo:rerun-if-changed=assets/maps/{name}.txt");
        writeln!(out, "    ({name:?}, include_str!({:?})),", format!("{root}/assets/maps/{name}.txt")).unwrap();
    }
    out.push_str("];\n");
    println!("cargo:rerun-if-changed=assets/maps/world.json");
    writeln!(out, "pub const EMBEDDED_WORLD: &str = include_str!({:?});", format!("{root}/assets/maps/world.json")).unwrap();

    let dest = Path::new(&std::env::var("OUT_DIR").unwrap()).join("embedded_maps.rs");
    std::fs::write(dest, out).unwrap();

    // Cutscenes: mesma ideia (assets/cutscenes/*.json). No nativo o jogo relê a pasta ao
    // abrir, então editar um arquivo não exige recompilar; o embutido serve ao navegador.
    println!("cargo:rerun-if-changed=assets/cutscenes");
    let mut files: Vec<String> = std::fs::read_dir("assets/cutscenes")
        .map(|dir| {
            dir.filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|x| x == "json"))
                .filter_map(|p| p.file_name().map(|s| s.to_string_lossy().into_owned()))
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    let mut out = String::from("/// Gerado pelo build.rs: (arquivo, texto) de cada cutscene em assets/cutscenes.\n");
    out.push_str("pub const EMBEDDED_CUTSCENES: &[(&str, &str)] = &[\n");
    for file in &files {
        println!("cargo:rerun-if-changed=assets/cutscenes/{file}");
        writeln!(out, "    ({file:?}, include_str!({:?})),", format!("{root}/assets/cutscenes/{file}")).unwrap();
    }
    out.push_str("];\n");
    let dest = Path::new(&std::env::var("OUT_DIR").unwrap()).join("embedded_cutscenes.rs");
    std::fs::write(dest, out).unwrap();
}
