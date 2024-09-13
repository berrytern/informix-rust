use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rustc-link-search=native=/opt/IBM/informix/lib");
    println!("cargo:rustc-link-search=native=/opt/IBM/informix/lib/esql");
    println!("cargo:rustc-link-search=native=/opt/IBM/informix/lib/cli");

    println!("cargo:rustc-link-lib=dylib=ifcli");
    println!("cargo:rustc-link-lib=dylib=ifdmr");
    println!("cargo:rustc-link-lib=dylib=ifos");
    println!("cargo:rustc-link-lib=dylib=ifglx");  // Note: changed from ift6x64 to ifglx
}