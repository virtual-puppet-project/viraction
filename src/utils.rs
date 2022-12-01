/// Gate multiple `pub mod` and `pub use` statements behind a single feature.
macro_rules! feature_gate {
    (
        feature: $feature:literal,
        mods: {$($m:ident,),*},
        uses: {$($u:path,),*}
    ) => {
        $(
            #[cfg(feature = $feature)]
            pub mod $m;
        )*
        $(
            #[cfg(feature = $feature)]
            pub use $u;
        )*
    };
}

pub(crate) use feature_gate;
