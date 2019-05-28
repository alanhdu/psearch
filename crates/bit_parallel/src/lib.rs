#[cfg(all(
    any(target_arch = "x86", target_arch = "x86_64"),
    target_feature = "sse"
))]
mod simd;

pub mod u9x7;

#[cfg(all(
    any(target_arch = "x86", target_arch = "x86_64"),
    target_feature = "sse"
))]
pub use simd::*;

#[cfg(test)]
mod test;
