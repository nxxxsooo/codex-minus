fn main() {
    // Keep inherited pipes usable on Windows; the parent supplies windowsHide.
    let mut args = std::env::args_os().skip(1);
    match args.next().as_deref() {
        Some(flag) if flag == "--stdio" && args.next().is_none() => {
            if codex_minus_lib::runtime::block_on(codex_minus_lib::rpc::serve()).is_err() {
                std::process::exit(1);
            }
        }
        Some(flag) if flag == "--verify-update" => {
            let (Some(artifact), Some(signature), Some(digest), Some(size), None) = (
                args.next(),
                args.next(),
                args.next(),
                args.next(),
                args.next(),
            ) else {
                std::process::exit(2)
            };
            let (Some(digest), Some(size)) = (digest.to_str(), size.to_str()) else {
                std::process::exit(2)
            };
            let Ok(size) = size.parse::<u64>() else {
                std::process::exit(2)
            };
            if codex_minus_lib::update_verify::verify_staged_artifact(
                std::path::Path::new(&artifact),
                std::path::Path::new(&signature),
                digest,
                size,
            )
            .is_err()
            {
                std::process::exit(1);
            }
        }
        Some(flag) if flag == "--swap-update-bundles" => {
            let (Some(left), Some(right), None) = (args.next(), args.next(), args.next()) else {
                std::process::exit(2)
            };
            if codex_minus_lib::update_swap::exchange(
                std::path::Path::new(&left),
                std::path::Path::new(&right),
            )
            .is_err()
            {
                std::process::exit(1);
            }
        }
        _ => std::process::exit(2),
    }
}
