use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // Caminho para o texconv.exe no diretório pai
    let texconv_source = "../texconv.exe";
    
    if !Path::new(texconv_source).exists() {
        panic!("texconv.exe não encontrado em: {}", texconv_source);
    }
    
    // Copiar texconv.exe para OUT_DIR para que possa ser incluído com include_bytes!
    let out_dir = env::var("OUT_DIR").unwrap();
    let texconv_dest = Path::new(&out_dir).join("texconv.exe");
    
    fs::copy(texconv_source, &texconv_dest)
        .expect("Falha ao copiar texconv.exe");
    
    println!("cargo:rerun-if-changed={}", texconv_source);
    println!("cargo:rerun-if-changed=build.rs");
    
    // Instruir o Rust a recompilar se o texconv.exe mudar
    println!("cargo:rerun-if-changed=../texconv.exe");
}
