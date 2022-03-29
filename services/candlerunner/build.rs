#[allow(dead_code)]
enum Target {
    Server,
    Client,
    ServerAndClient,
}

impl Target {
    pub fn compiler_settings(&self) -> (bool, bool) {
        match self {
            Target::Server => (true, false),
            Target::Client => (false, true),
            Target::ServerAndClient => (true, true),
        }
    }
}

fn generate_proto_package<P: AsRef<std::path::Path>>(
    path: P,
    target: Target,
) -> Result<(), Box<dyn std::error::Error>> {
    let package_path = path.as_ref();
    let (build_server, build_client) = target.compiler_settings();

    let mut protos: Vec<std::path::PathBuf> = Default::default();

    let proto_extension = std::ffi::OsStr::new("proto");

    for entry in std::fs::read_dir(package_path)? {
        let entry_path = entry?.path();

        if entry_path.is_file() && entry_path.extension() == Some(proto_extension) {
            println!(
                "cargo:rerun-if-changed={}",
                entry_path.as_os_str().to_string_lossy()
            );
            protos.push(entry_path);
        }
    }

    tonic_build::configure()
        .build_client(build_client)
        .build_server(build_server)
        .compile(&protos, &[package_path])?;

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    generate_proto_package("../../proto/vendor/tinkoff_invest_api", Target::Client)?;
    generate_proto_package("../../proto/candlerunner", Target::Server)?;

    Ok(())
}
