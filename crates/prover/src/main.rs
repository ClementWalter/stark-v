fn main() {
    run();
}

fn run() {
    tracing::info!("Hello, world!");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run() {
        // Initialize tracing for test
        let _ = tracing_subscriber::fmt().with_test_writer().try_init();
        run();
    }
}
