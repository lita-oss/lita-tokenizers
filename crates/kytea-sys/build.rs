use std::path::Path;

type Error = Box<dyn std::error::Error>;

const ROOT: &str = env!("CARGO_MANIFEST_DIR");
const C_LIB_SRC: &str = "kytea-src";
const C_WRAPPER: &str = "kytea-wrapper.cpp";

fn compile() -> Result<(), Error> {
    let src = Path::new(ROOT).join(C_WRAPPER);
    println!("cargo::rerun-if-changed={C_WRAPPER}");

    {
        let src = Path::new(ROOT).join(C_LIB_SRC).join("src");

        let include = src.join("include");
        println!("cargo::rerun-if-changed={}", include.display());
        let lib = src.join("lib");
        println!("cargo::rerun-if-changed={}", lib.display());
    }

    let data_env = "KYTEA_DATA_DIR";
    println!("cargo::rerun-if-env-changed={data_env}");

    let default_data_dir = std::env::var(data_env)
        .map(|s| format!("\"{s}\""))
        .unwrap_or_else(|_| "\"/usr/local/share/kytea\"".to_owned());

    let lib_src = {
        use std::ffi::OsStr;

        let lib = Path::new(ROOT).join(C_LIB_SRC).join("src/lib");

        let mut cpp_files = Vec::new();

        let cpp = OsStr::new("cpp");

        for entry in std::fs::read_dir(&lib)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension() == Some(cpp) {
                cpp_files.push(path);
            }
        }

        cpp_files
    };

    let include = Path::new(ROOT).join(C_LIB_SRC).join("src/include");

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .std("c++20")
        .define("PKGDATADIR", &*default_data_dir)
        .cpp_link_stdlib_static(cfg!(target_env = "musl"));

    let objects = build
        .clone()
        .files(lib_src)
        .include(&include)
        .include(include.join("kytea"))
        .warnings(false)
        .compile_intermediates();

    build
        .flag("-isystem")
        .flag(&include)
        .flag("-isystem")
        .flag(include.join("kytea"))
        .file(&src)
        .warnings(true)
        .objects(objects)
        .compile("kytea");

    println!("cargo:rustc-link-lib=static=kytea");

    Ok(())
}

fn main() -> Result<(), Error> {
    println!("cargo::rerun-if-changed=build.rs");

    compile()?;

    Ok(())
}
