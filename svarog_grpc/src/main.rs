use std::fs;

use clap::{arg, Command};
use glob::glob;
use tonic_build;

fn main() {
    // parse arguments
    let matches = Command::new("protoc-rust")
        .arg(
            arg!(-p --proto <PROTO_DIR>)
                .required(false)
                .default_value("proto"),
        )
        .arg(arg!(-r --rust <RUST_LIB_DIR>).required(true))
        .get_matches();
    let proto_dir = matches.get_one::<String>("proto").unwrap().to_owned();
    let rust_dir = matches.get_one::<String>("rust").unwrap().to_owned();

    let mut lib_rs: Vec<String> = Vec::new();
    '_proto_to_rs: {
        let mut protos: Vec<String> = Vec::new();
        for entry in glob(&format!("{}/*.proto", proto_dir)).unwrap() {
            if let Ok(path) = entry {
                protos.push(path.to_str().unwrap().to_string());
                let filename = path.file_stem().unwrap().to_str().unwrap();
                lib_rs.push(format!(
                    "mod {filename}; pub use {filename}::*;",
                    filename = filename
                ));
            }
        }
        fs::create_dir_all(&rust_dir).unwrap();
        tonic_build::configure()
            .out_dir(&rust_dir)
            .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
            .compile(&protos, &[&proto_dir])
            .unwrap();
    }

    '_protogen_lib_rs: {
        // Concat mod_rs into a single string.
        let lib_rs = lib_rs.join("\n");
        // Write mod_rs to src/protogen/mod.rs.
        fs::write(&format!("{}/lib.rs", rust_dir), lib_rs).unwrap();
    }
}
