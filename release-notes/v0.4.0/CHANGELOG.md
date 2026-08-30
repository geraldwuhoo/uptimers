# v0.4.0

Features:

* Build libshoutrrr.a in build.rs so a plain `cargo build` works
* Allow prebuilt shoutrrr archives via `SHOUTRRR_LIB_DIR`

Bugfix:

* Free the Go-allocated error string on each failed shoutrrr send
* Run the blocking shoutrrr send off the async runtime

Chores:

* Update Go to 1.27
* Various dependency updates
