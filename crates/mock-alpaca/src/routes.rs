//! Route handling: maps a recorded request to a status and JSON body.

use crate::state::Recorded;

/// Answer a request. No route is seeded yet, so everything is 404.
pub fn respond(_req: &Recorded) -> (u16, String) {
    (404, r#"{"message":"not found"}"#.to_string())
}

/// The answer to a request missing either auth header.
pub fn unauthorized() -> (u16, String) {
    (401, r#"{"message":"forbidden."}"#.to_string())
}
