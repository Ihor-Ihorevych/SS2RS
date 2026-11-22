//! Optional frame profiling (T11-011).
//!
//! Built with `--features profile`, setting `SS2_PROFILE` makes the app serve [puffin] frame and
//! scope data over TCP (default `127.0.0.1:8585`); a viewer such as `puffin_viewer` connects to
//! it. Without the feature this module costs nothing: the scope macro is empty and asking for
//! profiling only logs that this build cannot do it. See `docs/PROFILING.md`.
//!
//! [puffin]: https://docs.rs/puffin

use std::net::SocketAddr;

/// The environment variable that turns profiling on.
pub const PROFILE_ENV: &str = "SS2_PROFILE";

/// The address puffin viewers connect to by default.
pub const DEFAULT_ADDR: &str = "127.0.0.1:8585";

/// The address to serve profiling data on, from the text of [`PROFILE_ENV`]: unset, empty, `0`,
/// `off` or `false` mean no profiling; `1`, `on`, `true` or `yes` mean [`DEFAULT_ADDR`]; anything
/// else must be an `ip:port` address.
///
/// # Errors
/// A message naming the variable and the value when the text is not an address.
pub fn profile_address(value: Option<&str>) -> Result<Option<SocketAddr>, String> {
    let Some(text) = value.map(str::trim) else {
        return Ok(None);
    };
    match text.to_ascii_lowercase().as_str() {
        "" | "0" | "off" | "false" => Ok(None),
        "1" | "on" | "true" | "yes" => Ok(DEFAULT_ADDR.parse().ok()),
        _ => text.parse().map(Some).map_err(|e| {
            format!("{PROFILE_ENV}={text:?} is not 1, 0 or an address like 127.0.0.1:8585 ({e})")
        }),
    }
}

/// Whether only programs on this machine can connect to `addr`.
pub fn is_local_only(addr: SocketAddr) -> bool {
    addr.ip().is_loopback()
}

/// Keeps the profiling server alive; dropping it stops the server.
pub struct Profiler {
    #[cfg(feature = "profile")]
    _server: puffin_http::Server,
}

/// Start profiling if [`PROFILE_ENV`] asks for it. Problems are logged and never stop the app.
pub fn start() -> Option<Profiler> {
    match profile_address(std::env::var(PROFILE_ENV).ok().as_deref()) {
        Ok(None) => None,
        Ok(Some(addr)) => serve(addr),
        Err(message) => {
            tracing::warn!("{message}; profiling stays off");
            None
        }
    }
}

#[cfg(feature = "profile")]
fn serve(addr: SocketAddr) -> Option<Profiler> {
    puffin::set_scopes_on(true);
    match puffin_http::Server::new(&addr.to_string()) {
        Ok(server) => {
            tracing::info!("profiling: serving puffin data on {addr} (connect puffin_viewer)");
            if !is_local_only(addr) {
                tracing::warn!("profiling: {addr} is reachable from the network");
            }
            Some(Profiler { _server: server })
        }
        Err(e) => {
            tracing::warn!("profiling: cannot listen on {addr}: {e}");
            None
        }
    }
}

#[cfg(not(feature = "profile"))]
fn serve(addr: SocketAddr) -> Option<Profiler> {
    tracing::warn!(
        "{PROFILE_ENV} asks for profiling on {addr}, but this build has no profiler; rebuild with `--features profile`"
    );
    None
}

/// Mark the start of a new frame (puffin groups scopes into frames). A no-op without the
/// `profile` feature or while no viewer has ever been asked for.
pub fn frame() {
    #[cfg(feature = "profile")]
    puffin::GlobalProfiler::lock().new_frame();
}

/// Time the rest of the enclosing block as a named scope (a string literal). Expands to nothing
/// without the `profile` feature.
macro_rules! profile_scope {
    ($name:expr) => {
        #[cfg(feature = "profile")]
        puffin::profile_scope!($name);
    };
}
pub(crate) use profile_scope;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unset_empty_and_off_values_mean_no_profiling() {
        for v in [
            None,
            Some(""),
            Some("  "),
            Some("0"),
            Some("off"),
            Some("false"),
        ] {
            assert_eq!(profile_address(v), Ok(None), "{v:?}");
        }
    }

    #[test]
    fn a_plain_yes_means_the_default_local_address() {
        let default: SocketAddr = DEFAULT_ADDR.parse().unwrap();
        for v in ["1", "on", "true", "yes", " 1 "] {
            assert_eq!(profile_address(Some(v)), Ok(Some(default)), "{v:?}");
        }
        assert!(
            default.ip().is_loopback(),
            "the default must not be reachable from the network"
        );
    }

    #[test]
    fn an_explicit_address_is_used() {
        assert_eq!(
            profile_address(Some("127.0.0.1:9000")),
            Ok(Some("127.0.0.1:9000".parse().unwrap()))
        );
        assert_eq!(
            profile_address(Some("[::1]:8585")),
            Ok(Some("[::1]:8585".parse().unwrap()))
        );
    }

    #[test]
    fn nonsense_is_an_error_naming_the_variable() {
        let err = profile_address(Some("localhost")).unwrap_err();
        assert!(
            err.contains(PROFILE_ENV) && err.contains("localhost"),
            "{err}"
        );
        assert!(profile_address(Some("127.0.0.1:99999")).is_err());
    }

    #[test]
    fn a_public_address_is_allowed_but_flagged() {
        let addr: SocketAddr = "0.0.0.0:8585".parse().unwrap();
        assert!(!is_local_only(addr));
        assert!(is_local_only("127.0.0.1:1".parse().unwrap()));
        assert!(is_local_only("[::1]:1".parse().unwrap()));
    }
}
