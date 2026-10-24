//! 网络瞬时故障的有限重连。
//!
//! 只重连可能自行恢复的错误：连接失败、超时、读写中断、5xx 与限流。
//! 认证失败、参数错误、响应解析失败之类重试也不会变好，直接返回。

use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// 首次请求之外最多再重连两次。
pub const MAX_ATTEMPTS: usize = 3;

const BASE_DELAY: Duration = Duration::from_secs(1);
const MAX_DELAY: Duration = Duration::from_secs(8);
const JITTER_MILLIS: u64 = 250;

/// 第 `attempt` 次失败后的等待：1s、2s、4s……封顶 8s，并加少量抖动。
pub fn backoff(attempt: usize) -> Duration {
    let shift = attempt.saturating_sub(1).min(3) as u32;
    let delay = BASE_DELAY.saturating_mul(1 << shift).min(MAX_DELAY);

    delay + jitter()
}

/// 限流与 5xx 值得重连；其余 4xx 是请求本身的问题。
pub fn transient_status(status: u16) -> bool {
    matches!(status, 408 | 425 | 429) || (500..=599).contains(&status)
}

fn jitter() -> Duration {
    // 只用来错开并发重连，不追求密码学随机。
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.subsec_nanos())
        .unwrap_or(0);

    Duration::from_millis(u64::from(nanos) % JITTER_MILLIS)
}

#[cfg(test)]
mod tests {
    use super::{backoff, transient_status};

    #[test]
    fn backoff_grows_then_caps() {
        let first = backoff(1).as_millis();
        let second = backoff(2).as_millis();
        let capped = backoff(20).as_millis();

        assert!((1000..1250).contains(&first));
        assert!((2000..2250).contains(&second));
        assert!((8000..8250).contains(&capped));
    }

    #[test]
    fn only_transient_statuses_are_retried() {
        for status in [408, 425, 429, 500, 502, 503, 504] {
            assert!(transient_status(status), "{status} 应可重连");
        }

        for status in [400, 401, 403, 404, 422] {
            assert!(!transient_status(status), "{status} 不应重连");
        }
    }
}
