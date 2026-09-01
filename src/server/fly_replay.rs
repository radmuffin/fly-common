use axum::{
    extract::Request,
    http::{Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};

/// Checks if the current machine is NOT in the primary region,
/// and if so, returns a fly-replay redirect response for mutating requests.
///
/// Reads FLY_REGION env var to detect current region.
/// If FLY_REGION != primary_region AND method is POST/PUT/PATCH/DELETE,
/// returns HTTP 409 Conflict with header `fly-replay: region=<primary_region>`.
/// Fly.io edge proxy will transparently retry the request on the primary.
pub async fn fly_replay_middleware(primary_region: String, req: Request, next: Next) -> Response {
    if let Ok(current_region) = std::env::var("FLY_REGION") {
        if current_region != primary_region {
            let method = req.method();
            if method == Method::POST
                || method == Method::PUT
                || method == Method::PATCH
                || method == Method::DELETE
            {
                let mut res = StatusCode::CONFLICT.into_response();
                res.headers_mut().insert(
                    "fly-replay",
                    format!("region={}", primary_region).parse().unwrap(),
                );
                return res;
            }
        }
    }

    next.run(req).await
}
