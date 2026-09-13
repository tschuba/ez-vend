pub const LABELS_PUBLIC_URL: &str = match option_env!("LABELS_PUBLIC_URL") {
    Some(url) => url,
    None => "http://localhost:8080/",
};
