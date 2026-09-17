use std::convert::Infallible;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process;
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};
use directories::ProjectDirs;
use sysinfo::{Pid, ProcessesToUpdate, System};
use warp::http::header::CONTENT_TYPE;
use warp::http::{Response, StatusCode};
use warp::{Filter, Reply};

const APP_NAME: &str = "ez-vend";
const LOCK_FILE_NAME: &str = "launcher.lock";
const PORT_RANGE: std::ops::RangeInclusive<u16> = 8080..=8089;
const LOCK_RETRY_DELAY: Duration = Duration::from_millis(50);
const MAX_LOCK_RETRIES: usize = 5;
const CONTENT_SECURITY_POLICY: &str = "default-src 'self'; script-src 'self' 'unsafe-inline' 'unsafe-eval' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; font-src 'self' data:; connect-src 'self' ws: wss:; worker-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'";
const FRAME_OPTIONS: &str = "DENY";
const CONTENT_TYPE_OPTIONS: &str = "nosniff";
const KNOWN_BINARY_NAMES: &[&str] = &[
    "ez-vend",
    "ez-vend.exe",
    "ez-vend-linux",
    "ez-vend-macos",
    "ez-vend-launcher",
    "ez-vend-launcher.exe",
];

enum LockState {
    Acquired(LockFile),
    ActiveInstance(PathBuf),
    Warning(anyhow::Error),
}

struct LockFile {
    path: PathBuf,
}

impl LockFile {
    fn try_acquire() -> LockState {
        match Self::lock_path() {
            Ok(path) => match Self::acquire_at(path) {
                Ok(lock) => LockState::Acquired(lock),
                Err(LockAcquireError::ActiveInstance(path)) => LockState::ActiveInstance(path),
                Err(LockAcquireError::Warning(err)) => LockState::Warning(err),
            },
            Err(err) => LockState::Warning(err),
        }
    }

    fn lock_path() -> Result<PathBuf> {
        let project_dirs = ProjectDirs::from("", "", "ez-vend")
            .context("could not determine a user config directory for ez-vend")?;
        Ok(project_dirs.config_dir().join(LOCK_FILE_NAME))
    }

    fn acquire_at(path: PathBuf) -> Result<Self, LockAcquireError> {
        let parent = path
            .parent()
            .context("lock file path is missing a parent directory")
            .map_err(LockAcquireError::Warning)?;
        fs::create_dir_all(parent)
            .with_context(|| format!("could not create config directory at {}", parent.display()))
            .map_err(LockAcquireError::Warning)?;

        Self::create_lock_file(&path)?;

        Ok(Self { path })
    }

    fn create_lock_file(path: &Path) -> Result<(), LockAcquireError> {
        let pid = process::id().to_string();

        for attempt in 0..MAX_LOCK_RETRIES {
            match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
            {
                Ok(mut file) => {
                    use std::io::Write;

                    file.write_all(pid.as_bytes())
                        .with_context(|| format!("could not write lock file at {}", path.display()))
                        .map_err(LockAcquireError::Warning)?;
                    return Ok(());
                }
                Err(err) if err.kind() == ErrorKind::AlreadyExists => {
                    Self::validate_or_cleanup_existing(path)?;

                    if attempt + 1 == MAX_LOCK_RETRIES {
                        return Err(LockAcquireError::Warning(anyhow::anyhow!(
                            "could not acquire lock file at {} after {MAX_LOCK_RETRIES} attempts",
                            path.display()
                        )));
                    }

                    thread::sleep(LOCK_RETRY_DELAY);
                }
                Err(err) => {
                    return Err(LockAcquireError::Warning(anyhow::Error::new(err).context(
                        format!("could not create lock file at {}", path.display()),
                    )));
                }
            }
        }

        Err(LockAcquireError::Warning(anyhow::anyhow!(
            "could not acquire lock file at {} after {MAX_LOCK_RETRIES} attempts",
            path.display()
        )))
    }

    fn validate_or_cleanup_existing(path: &Path) -> Result<(), LockAcquireError> {
        if !path.exists() {
            return Ok(());
        }

        let lock_contents = match fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(err) if err.kind() == ErrorKind::NotFound => return Ok(()),
            Err(err) => {
                return Err(LockAcquireError::Warning(anyhow::Error::new(err).context(
                    format!("could not read lock file at {}", path.display()),
                )));
            }
        };

        let pid = match lock_contents.trim().parse::<u32>() {
            Ok(pid) => pid,
            Err(_) => {
                println!("Cleaned up invalid lock file from previous session.");
                fs::remove_file(path)
                    .with_context(|| {
                        format!("could not remove invalid lock file at {}", path.display())
                    })
                    .map_err(LockAcquireError::Warning)?;
                return Ok(());
            }
        };

        if process_is_active(pid) {
            return Err(LockAcquireError::ActiveInstance(path.to_path_buf()));
        }

        println!("Cleaned up stale lock file from previous session.");
        fs::remove_file(path)
            .with_context(|| format!("could not remove stale lock file at {}", path.display()))
            .map_err(LockAcquireError::Warning)?;

        Ok(())
    }

    fn cleanup(&self) {
        if let Err(err) = fs::remove_file(&self.path) {
            if err.kind() != ErrorKind::NotFound {
                eprintln!(
                    "Warning: could not remove lock file at {}: {err}",
                    self.path.display()
                );
            }
        }
    }
}

impl Drop for LockFile {
    fn drop(&mut self) {
        self.cleanup();
    }
}

enum LockAcquireError {
    ActiveInstance(PathBuf),
    Warning(anyhow::Error),
}

fn process_is_active(pid: u32) -> bool {
    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::All, true);

    system
        .process(Pid::from_u32(pid))
        .map(|process| {
            let process_name = process.name().to_string_lossy().to_ascii_lowercase();
            KNOWN_BINARY_NAMES.contains(&process_name.as_str())
        })
        .unwrap_or(false)
}

#[tokio::main]
async fn main() -> Result<()> {
    let _lock = match LockFile::try_acquire() {
        LockState::Acquired(lock) => Some(lock),
        LockState::ActiveInstance(path) => {
            eprintln!(
                "Error: Another ez-vend instance is already running.\n\nPlease close the existing instance before starting a new one.\nIf you believe this is an error, delete the lock file at:\n  {}",
                path.display()
            );
            process::exit(1);
        }
        LockState::Warning(err) => {
            eprintln!(
                "Warning: {}.\nMultiple instances may run simultaneously.\n\nContinuing anyway...",
                err
            );
            None
        }
    };

    let port = find_available_port()?;
    let url = format!("http://127.0.0.1:{port}");

    println!("{APP_NAME} is starting...");
    println!("Opening browser at: {url}");
    println!("\nPress Ctrl+C to stop the server.\n");

    if let Err(err) = webbrowser::open(&url) {
        eprintln!("Could not open the browser automatically: {err}");
        eprintln!("Open this URL manually: {url}");
    }

    serve_app(port).await
}

fn find_available_port() -> Result<u16> {
    for port in PORT_RANGE {
        if std::net::TcpListener::bind(("127.0.0.1", port)).is_ok() {
            return Ok(port);
        }
    }

    Err(anyhow::anyhow!(
        "Could not find an available port (tried 8080-8089).\n\nThis may indicate another ez-vend instance is running,\nor these ports are in use by other applications."
    ))
}

async fn serve_app(port: u16) -> Result<()> {
    let app_dir = app_dir()?;
    let routes =
        warp::get()
            .and(warp::path::full())
            .and_then(move |full_path: warp::path::FullPath| {
                let app_dir = app_dir.clone();
                async move { Ok::<_, Infallible>(serve_path(&app_dir, full_path.as_str()).await) }
            });

    warp::serve(routes)
        .bind(([127, 0, 0, 1], port))
        .await
        .graceful(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .run()
        .await;
    println!("{APP_NAME} stopped.");
    Ok(())
}

fn app_dir() -> Result<PathBuf> {
    let executable = std::env::current_exe().context("could not determine launcher path")?;
    let app_dir = executable
        .parent()
        .map(Path::to_path_buf)
        .context("launcher path does not have a parent directory")?;

    fs::canonicalize(&app_dir).with_context(|| {
        format!(
            "could not resolve application directory at {}",
            app_dir.display()
        )
    })
}

async fn serve_path(app_dir: &Path, request_path: &str) -> impl Reply {
    match read_response(app_dir, request_path).await {
        Ok(response) => response,
        Err(err) => text_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to serve the application: {err}"),
            "text/plain; charset=utf-8",
        ),
    }
}

async fn read_response(app_dir: &Path, request_path: &str) -> Result<Response<Vec<u8>>> {
    let Some(sanitized) = sanitize_request_path(request_path) else {
        return Ok(text_response(
            StatusCode::NOT_FOUND,
            "File not found".to_string(),
            "text/plain; charset=utf-8",
        ));
    };

    let asset_path = app_dir.join(sanitized.as_str());

    if let Some(response) = file_response_if_exists(app_dir, &asset_path).await? {
        return Ok(response);
    }

    if !has_extension(sanitized.as_ref()) {
        let fallback = app_dir.join("index.html");
        if let Some(response) = file_response_if_exists(app_dir, &fallback).await? {
            return Ok(response);
        }
    }

    Ok(text_response(
        StatusCode::NOT_FOUND,
        "File not found".to_string(),
        "text/plain; charset=utf-8",
    ))
}

fn sanitize_request_path(request_path: &str) -> Option<String> {
    let decoded = urlencoding::decode(request_path).ok()?;
    let normalized = decoded.replace('\\', "/");
    let trimmed = normalized.trim_start_matches('/');

    if trimmed.is_empty() {
        return Some("index.html".to_string());
    }

    if trimmed.split('/').any(|segment| segment == "..") {
        return None;
    }

    let cleaned: Vec<&str> = trimmed
        .split('/')
        .filter(|segment| !segment.is_empty() && *segment != ".")
        .collect();

    if cleaned.is_empty() {
        Some("index.html".to_string())
    } else {
        Some(cleaned.join("/"))
    }
}

fn has_extension(path: &str) -> bool {
    Path::new(path).extension().is_some()
}

async fn file_response_if_exists(app_dir: &Path, path: &Path) -> Result<Option<Response<Vec<u8>>>> {
    let Some(validated_path) = validate_asset_path(app_dir, path)? else {
        return Ok(None);
    };

    match tokio::fs::read(&validated_path).await {
        Ok(bytes) => Ok(Some(bytes_response(bytes, content_type(&validated_path)))),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => {
            Err(err).with_context(|| format!("could not read {}", validated_path.display()))
        }
    }
}

fn validate_asset_path(app_dir: &Path, path: &Path) -> Result<Option<PathBuf>> {
    let canonical_path = match fs::canonicalize(path) {
        Ok(path) => path,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(
                anyhow::Error::new(err).context(format!("could not resolve {}", path.display()))
            );
        }
    };

    if !canonical_path.starts_with(app_dir) || !canonical_path.is_file() {
        return Ok(None);
    }

    Ok(Some(canonical_path))
}

fn bytes_response(body: Vec<u8>, content_type: &'static str) -> Response<Vec<u8>> {
    response_builder(StatusCode::OK, content_type)
        .body(body)
        .expect("response builder should not fail")
}

fn text_response(
    status: StatusCode,
    body: String,
    content_type: &'static str,
) -> Response<Vec<u8>> {
    response_builder(status, content_type)
        .body(body.into_bytes())
        .expect("response builder should not fail")
}

fn response_builder(
    status: StatusCode,
    content_type: &'static str,
) -> warp::http::response::Builder {
    Response::builder()
        .status(status)
        .header(CONTENT_TYPE, content_type)
        .header("Content-Security-Policy", CONTENT_SECURITY_POLICY)
        .header("X-Content-Type-Options", CONTENT_TYPE_OPTIONS)
        .header("X-Frame-Options", FRAME_OPTIONS)
}

fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "application/javascript; charset=utf-8",
        Some("mjs") => "application/javascript; charset=utf-8",
        Some("wasm") => "application/wasm",
        Some("json") => "application/json",
        Some("txt") => "text/plain; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("ico") => "image/x-icon",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn sanitize_rejects_decoded_parent_segments() {
        assert_eq!(sanitize_request_path("/%2e%2e/secrets.txt"), None);
        assert_eq!(sanitize_request_path("/safe/%2E%2E/file.txt"), None);
    }

    #[test]
    fn sanitize_normalizes_empty_and_relative_segments() {
        assert_eq!(sanitize_request_path("/"), Some("index.html".to_string()));
        assert_eq!(
            sanitize_request_path("/assets//./app.js"),
            Some("assets/app.js".to_string())
        );
    }

    #[test]
    fn validate_asset_path_stays_inside_app_dir() {
        let temp_root = unique_temp_dir();
        let app_dir = temp_root.join("app");
        let outside_file = temp_root.join("outside.txt");
        let inside_file = app_dir.join("index.html");

        fs::create_dir_all(&app_dir).expect("create app dir");
        fs::write(&inside_file, b"ok").expect("write inside file");
        fs::write(&outside_file, b"nope").expect("write outside file");

        let canonical_app_dir = fs::canonicalize(&app_dir).expect("canonicalize app dir");
        let validated_inside =
            validate_asset_path(&canonical_app_dir, &inside_file).expect("validate inside path");
        let validated_outside =
            validate_asset_path(&canonical_app_dir, &outside_file).expect("validate outside path");

        assert_eq!(
            validated_inside,
            Some(fs::canonicalize(&inside_file).expect("canonicalize inside file"))
        );
        assert_eq!(validated_outside, None);

        fs::remove_dir_all(&temp_root).expect("cleanup temp dir");
    }

    #[test]
    fn responses_include_security_headers() {
        let response = text_response(
            StatusCode::OK,
            "hello".to_string(),
            "text/plain; charset=utf-8",
        );

        assert_eq!(
            response.headers().get("Content-Security-Policy").unwrap(),
            CONTENT_SECURITY_POLICY
        );
        assert_eq!(
            response.headers().get("X-Content-Type-Options").unwrap(),
            CONTENT_TYPE_OPTIONS
        );
        assert_eq!(
            response.headers().get("X-Frame-Options").unwrap(),
            FRAME_OPTIONS
        );
    }

    fn unique_temp_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("current time")
            .as_nanos();
        std::env::temp_dir().join(format!("ez-vend-launcher-test-{nanos}"))
    }
}
