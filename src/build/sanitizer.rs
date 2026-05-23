use crate::build::profile::{ResolvedCmake, ResolvedProfile};

/// Sanitizer kind for built-in --asan / --tsan flags.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SanitizerKind {
    /// AddressSanitizer (--asan)
    Address,
    /// ThreadSanitizer (--tsan)
    Thread,
}

impl SanitizerKind {
    /// Short name used as the path segment, e.g. `ccgo_build/debug-asan/android/`.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Address => "asan",
            Self::Thread => "tsan",
        }
    }

    /// C/C++ compiler flags injected into cmake when no user-defined profile overrides.
    fn compiler_flags(&self) -> &'static [&'static str] {
        match self {
            Self::Address => &["-fsanitize=address", "-fno-omit-frame-pointer"],
            Self::Thread => &["-fsanitize=thread", "-fno-omit-frame-pointer"],
        }
    }

    /// CMake arguments that carry sanitizer linker flags.
    ///
    /// `-DCMAKE_EXE_LINKER_FLAGS` / `-DCMAKE_SHARED_LINKER_FLAGS` are used instead of a
    /// dedicated linker-flags field because `CmakeUserConfig` does not have one, and passing
    /// them as `-D` arguments is the idiomatic CMake way.
    fn cmake_arguments(&self) -> Vec<String> {
        let flag = match self {
            Self::Address => "-fsanitize=address",
            Self::Thread => "-fsanitize=thread",
        };
        vec![
            format!("-DCMAKE_EXE_LINKER_FLAGS={flag}"),
            format!("-DCMAKE_SHARED_LINKER_FLAGS={flag}"),
        ]
    }

    /// Build a synthetic `ResolvedProfile` with built-in sanitizer flags.
    ///
    /// This is used when the user passes `--asan`/`--tsan` but has not defined a matching
    /// `[profile.asan]` / `[profile.tsan]` in CCGO.toml.
    pub fn to_resolved_profile(&self) -> ResolvedProfile {
        let flags: Vec<String> = self.compiler_flags().iter().map(|s| s.to_string()).collect();
        ResolvedProfile {
            cmake: ResolvedCmake {
                arguments: self.cmake_arguments(),
                c_flags: flags.clone(),
                cpp_flags: flags,
            },
            ..ResolvedProfile::default()
        }
    }
}
