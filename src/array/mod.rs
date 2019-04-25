#[cfg(all(
    any(target_arch = "x86", target_arch = "x86_64"),
    target_feature = "sse"
))]
mod simd;

#[cfg(all(
    any(target_arch = "x86", target_arch = "x86_64"),
    target_feature = "sse"
))]
pub(crate) use simd::*;

#[cfg(test)]
mod test;
