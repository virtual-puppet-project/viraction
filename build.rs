use std::process::Command;

const BUILD_NAME: &str = "Argent";

fn main() {
    {
        let sha = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .output()
            .map(|x| String::from(String::from_utf8_lossy(&x.stdout)))
            .unwrap();

        println!("cargo:rustc-env=GIT_REV={}", sha);
    }

    {
        println!("cargo:rustc-env=BUILD_NAME={}", BUILD_NAME);
    }
}
