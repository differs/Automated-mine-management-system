use axum::{
    http::{StatusCode, header},
    middleware::Next,
    response::IntoResponse,
};
use redis::{AsyncCommands, aio::ConnectionManager};

/// Redis-based rate limiter middleware factory
///
/// Uses sliding window counter: `rate_limit:{ip}:{window}`
/// Default: 100 requests per 60-second window.
pub fn create_rate_limiter(
    redis: ConnectionManager,
) -> impl Fn(
    axum::http::Request<axum::body::Body>,
    Next,
) -> std::pin::Pin<
    Box<
        dyn std::future::Future<Output = Result<axum::response::Response, StatusCode>>
            + Send
            + 'static,
    >,
> + Clone {
    move |request, next| {
        let mut redis = redis.clone();
        Box::pin(async move {
            // 提取客户端 IP
            let ip = request
                .headers()
                .get("x-forwarded-for")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.split(',').next())
                .or_else(|| {
                    request
                        .headers()
                        .get("x-real-ip")
                        .and_then(|v| v.to_str().ok())
                })
                .unwrap_or("unknown")
                .to_string();

            // 滑动窗口：每 60 秒最多 100 次请求
            let window = 60u64;
            let max_requests = 100i64;
            let now = chrono::Utc::now().timestamp() as u64;
            let window_start = now / window * window;
            let key = format!("rate_limit:{}:{}", ip, window_start);

            let count: Result<i64, _> = redis.incr(&key, 1i64).await;
            if let Ok(count) = count {
                if count == 1 {
                    let _: Result<(), _> = redis.expire(&key, window as i64).await;
                }

                if count > max_requests {
                    let retry_after = window - (now % window);
                    let mut response = (
                        StatusCode::TOO_MANY_REQUESTS,
                        [(header::RETRY_AFTER, retry_after.to_string())],
                        "rate limit exceeded",
                    )
                        .into_response();
                    response
                        .headers_mut()
                        .insert(header::CONTENT_TYPE, "text/plain".parse().unwrap());
                    return Err(StatusCode::TOO_MANY_REQUESTS);
                }
            }

            Ok(next.run(request).await)
        })
    }
}
