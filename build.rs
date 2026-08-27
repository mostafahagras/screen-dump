use std::process::Command;

fn main() {
    println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");

    if let Ok(output) = Command::new("xcode-select").arg("-p").output()
        && output.status.success()
    {
        let xcode_path = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        println!(
            "cargo:rustc-link-arg=-Wl,-rpath,/Library/Developer/CommandLineTools/usr/lib/swift-5.5/macosx"
        );
        println!(
            "cargo:rustc-link-arg=-Wl,-rpath,{xcode_path}/Toolchains/XcodeDefault.xctoolchain/usr/lib/swift-5.5/macosx"
        );
        println!(
            "cargo:rustc-link-arg=-Wl,-rpath,{xcode_path}/Toolchains/XcodeDefault.xctoolchain/usr/lib/swift/macosx"
        );
    }
}
