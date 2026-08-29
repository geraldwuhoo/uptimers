use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use actix_web::rt::task::spawn_blocking;
use log::debug;

use super::errors::UptimersError;

extern "C" {
    /// Sends `msg` to the shoutrrr `url`, returning NULL on success or an owned
    /// C string describing the failure. See `go/shoutrrr.go`.
    fn Shoutrrr(url: *const c_char, msg: *const c_char) -> *mut c_char;

    /// Releases an error string returned by [`Shoutrrr`].
    fn ShoutrrrFree(err: *mut c_char);
}

/// Sends a notification through shoutrrr.
///
/// `shoutrrr.Send` performs synchronous network I/O, so it runs on the blocking
/// pool rather than stalling the runtime thread driving the site checks until
/// the notification service responds.
pub async fn notify(url: String, msg: String) -> Result<(), UptimersError> {
    debug!("sending shoutrrr notification to {}, msg: {}", url, msg);
    let url = CString::new(url)?;
    let msg = CString::new(msg)?;

    spawn_blocking(move || {
        // SAFETY: both strings outlive the call, and Go only reads through them.
        let err = unsafe { Shoutrrr(url.as_ptr(), msg.as_ptr()) };
        if err.is_null() {
            return Ok(());
        }

        // SAFETY: `err` is a non-NULL, NUL-terminated string that Go allocated
        // for us to own; copy it out before handing the allocation back.
        let message = unsafe { CStr::from_ptr(err) }
            .to_string_lossy()
            .into_owned();
        unsafe { ShoutrrrFree(err) };

        Err(UptimersError::Shoutrrr(message))
    })
    .await?
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drives `notify` to completion on an actix runtime, which `spawn_blocking`
    /// requires.
    fn block_on(url: &str, msg: &str) -> Result<(), UptimersError> {
        actix_web::rt::System::new().block_on(notify(url.to_string(), msg.to_string()))
    }

    /// Exercises the failure path end to end: shoutrrr rejects the scheme
    /// without touching the network, so we get back an owned Go string to copy
    /// out and release.
    #[test]
    fn unknown_service_is_reported_as_an_error() {
        let err = block_on("nosuchservice://example", "hello")
            .expect_err("an unknown service should not report success");

        match err {
            UptimersError::Shoutrrr(msg) => assert!(!msg.is_empty()),
            other => panic!("expected a shoutrrr error, got {other:?}"),
        }
    }

    /// Repeated failures must not accumulate the error strings Go hands us.
    #[test]
    fn repeated_failures_do_not_accumulate() {
        for _ in 0..10_000 {
            block_on("nosuchservice://example", "hello").unwrap_err();
        }
    }

    /// A NUL in the URL cannot cross the FFI boundary, and must be rejected
    /// before we hand anything to Go.
    #[test]
    fn interior_nul_is_rejected() {
        let err = block_on("gotify://host\0/token", "hello")
            .expect_err("an interior NUL should be rejected");

        assert!(matches!(err, UptimersError::Nul(_)), "got {err:?}");
    }
}
