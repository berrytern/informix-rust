use std::env;
use std::path::PathBuf;

fn main() {
    // Get the Informix environment variables
    let informix_dir = env::var("INFORMIXDIR").expect("INFORMIXDIR is not set");
    
    // Build the paths for linking
    let lib_path = PathBuf::from(format!("{}/lib", informix_dir));
    let esql_path = PathBuf::from(format!("{}/lib/esql", informix_dir));
    let cli_path = PathBuf::from(format!("{}/lib/cli", informix_dir));
    
    // Pass the paths to the linker
    println!("cargo:rustc-link-search=native={}", lib_path.display());
    println!("cargo:rustc-link-search=native={}", esql_path.display());
    println!("cargo:rustc-link-search=native={}", cli_path.display());
    
    // Link the required Informix dynamic libraries
    println!("cargo:rustc-link-lib=dylib=ifcli");
    println!("cargo:rustc-link-lib=dylib=ifdmr");
    println!("cargo:rustc-link-lib=dylib=ifos");
    println!("cargo:rustc-link-lib=dylib=ifglx");  // ifglx is preferred over ift6x64
}
