use silm::commands::dev::output::{OutputMux, Source};

#[tokio::test]
async fn test_output_mux_prefixes_lines() {
    let mux = OutputMux::new();
    let sender = mux.sender();

    let lines = std::sync::Arc::new(tokio::sync::Mutex::new(Vec::<String>::new()));
    let lines_clone = lines.clone();

    let handle = tokio::spawn(async move {
        mux.run_capturing(lines_clone).await;
    });

    sender.send(Source::Server, "hello from server").await;
    sender.send(Source::Client, "hello from client").await;
    sender.send(Source::Build, "building...").await;
    sender.close().await;

    handle.await.unwrap();
    let result = lines.lock().await;
    assert!(result.iter().any(|l| l.contains("[server]") && l.contains("hello from server")));
    assert!(result.iter().any(|l| l.contains("[client]") && l.contains("hello from client")));
    assert!(result.iter().any(|l| l.contains("[build]") && l.contains("building...")));
}
