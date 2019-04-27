#[cfg(all(
    any(target_arch = "x86", target_arch = "x86_64"),
    target_feature = "sse"
))]
mod simd;

pub(crate) mod u8x3;

#[cfg(all(
    any(target_arch = "x86", target_arch = "x86_64"),
    target_feature = "sse"
))]
pub(crate) use simd::*;

#[cfg(test)]
mod test;
