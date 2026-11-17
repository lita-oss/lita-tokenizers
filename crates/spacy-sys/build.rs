fn main() -> Result<(), &'static str> {
    println!("cargo::rerun-if-changed=build.rs");
    let python = pyo3_build_config::get();
    if python.implementation() != pyo3_build_config::PythonImplementation::CPython {
        return Err("spacy-sys requires CPython; select it with PYO3_PYTHON");
    }
    if !python.shared() {
        return Err(
            "spacy-sys requires a shared CPython library; select a shared-library CPython installation with PYO3_PYTHON",
        );
    }
    Ok(())
}
