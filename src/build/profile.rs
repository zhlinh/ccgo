use std::collections::HashMap;

use anyhow::{bail, Result};

use crate::commands::build::LinkType;
use crate::config::{
    Linkage, MergeStrategy, ProfileCmake, ProfileConfig, ProfileDepLinkage,
    ProfilePlatformConfig,
};

/// Resolved (post-inheritance) cmake flags for a single scope.
#[derive(Debug, Clone, Default)]
pub struct ResolvedCmake {
    pub arguments: Vec<String>,
    pub c_flags: Vec<String>,
    pub cpp_flags: Vec<String>,
}

/// Resolved dep-linkage hints from a profile scope.
#[derive(Debug, Clone, Default)]
pub struct ResolvedDepLinkage {
    pub default: Option<Linkage>,
    pub on_shared: Option<Linkage>,
    pub on_static: Option<Linkage>,
}

/// Fully-resolved profile after inheritance chain expansion.
#[derive(Debug, Clone, Default)]
pub struct ResolvedProfile {
    pub name: Option<String>,
    pub release: Option<bool>,
    pub link_type: Option<LinkType>,
    pub jobs: Option<u32>,
    pub cmake: ResolvedCmake,
    pub features: Vec<String>,
    pub dep_linkage: ResolvedDepLinkage,
    /// Per-platform cmake flags, keyed by lowercase platform name.
    pub platform_cmake: HashMap<String, ResolvedCmake>,
    /// Per-platform dep linkage, keyed by lowercase platform name.
    pub platform_dep_linkage: HashMap<String, ResolvedDepLinkage>,
}

/// Build a map that includes built-in profiles alongside user-defined ones.
/// User-defined profiles take precedence (can override built-ins).
fn build_full_profiles(
    user_profiles: &HashMap<String, ProfileConfig>,
) -> HashMap<String, ProfileConfig> {
    let mut map = HashMap::new();
    let builtin_debug = ProfileConfig { release: Some(false), ..Default::default() };
    let builtin_release = ProfileConfig { release: Some(true), ..Default::default() };
    map.insert("debug".to_string(), builtin_debug);
    map.insert("release".to_string(), builtin_release);
    for (k, v) in user_profiles {
        map.insert(k.clone(), v.clone());
    }
    map
}

/// Expand the single-inheritance chain for `name` into a list ordered
/// [furthest ancestor, …, direct parent, name].
fn expand_chain<'a>(
    name: &'a str,
    profiles: &'a HashMap<String, ProfileConfig>,
) -> Result<Vec<&'a str>> {
    let mut chain: Vec<&str> = Vec::new();
    let mut cursor = name;
    loop {
        if chain.contains(&cursor) {
            bail!("profile inheritance cycle detected: '{cursor}' appears twice in chain");
        }
        chain.push(cursor);
        match profiles.get(cursor).and_then(|p| p.inherits.as_deref()) {
            Some(parent) => cursor = parent,
            None => break,
        }
    }
    chain.reverse();
    Ok(chain)
}

fn apply_cmake(acc: &mut ResolvedCmake, layer: &ProfileCmake) {
    match layer.merge {
        MergeStrategy::Replace => {
            acc.arguments = layer.arguments.clone();
            acc.c_flags = layer.c_flags.clone();
            acc.cpp_flags = layer.cpp_flags.clone();
        }
        MergeStrategy::Extend => {
            acc.arguments.extend_from_slice(&layer.arguments);
            acc.c_flags.extend_from_slice(&layer.c_flags);
            acc.cpp_flags.extend_from_slice(&layer.cpp_flags);
        }
    }
}

fn apply_dep_linkage(acc: &mut ResolvedDepLinkage, layer: &ProfileDepLinkage) {
    if layer.default.is_some() {
        acc.default = layer.default;
    }
    if layer.on_shared.is_some() {
        acc.on_shared = layer.on_shared;
    }
    if layer.on_static.is_some() {
        acc.on_static = layer.on_static;
    }
}

fn apply_scalar_fields(resolved: &mut ResolvedProfile, cfg: &ProfileConfig) {
    if let Some(v) = cfg.release {
        resolved.release = Some(v);
    }
    if let Some(ref v) = cfg.link_type {
        resolved.link_type = Some(v.clone());
    }
    if let Some(v) = cfg.jobs {
        resolved.jobs = Some(v);
    }
    if let Some(ref v) = cfg.name {
        resolved.name = Some(v.clone());
    }

    if let Some(ref feat) = cfg.features {
        match feat.merge {
            MergeStrategy::Replace => resolved.features = feat.list.clone(),
            MergeStrategy::Extend => resolved.features.extend_from_slice(&feat.list),
        }
    }

    if let Some(ref cmake) = cfg.cmake {
        apply_cmake(&mut resolved.cmake, cmake);
    }

    if let Some(ref dl) = cfg.dep_linkage {
        apply_dep_linkage(&mut resolved.dep_linkage, dl);
    }
}

fn apply_platform_configs(resolved: &mut ResolvedProfile, cfg: &ProfileConfig) {
    let plat_configs: [(&str, Option<&ProfilePlatformConfig>); 6] = [
        ("android", cfg.platforms.as_ref().and_then(|p| p.android.as_ref())),
        ("ios", cfg.platforms.as_ref().and_then(|p| p.ios.as_ref())),
        ("macos", cfg.platforms.as_ref().and_then(|p| p.macos.as_ref())),
        ("windows", cfg.platforms.as_ref().and_then(|p| p.windows.as_ref())),
        ("linux", cfg.platforms.as_ref().and_then(|p| p.linux.as_ref())),
        ("ohos", cfg.platforms.as_ref().and_then(|p| p.ohos.as_ref())),
    ];
    for (plat_name, plat_cfg) in &plat_configs {
        let Some(pc) = plat_cfg else { continue };
        let Some(ref build) = pc.build else { continue };

        if let Some(ref cmake) = build.cmake {
            let acc = resolved.platform_cmake.entry(plat_name.to_string()).or_default();
            apply_cmake(acc, cmake);
        }
        if let Some(ref dl) = build.dep_linkage {
            let acc = resolved
                .platform_dep_linkage
                .entry(plat_name.to_string())
                .or_default();
            apply_dep_linkage(acc, dl);
        }
    }
}

/// Resolve a named profile from the profile map.
///
/// Returns `Err` if:
/// - `name` is not found in `profiles` (and is not a built-in name)
/// - a cycle is detected in the inheritance chain
/// - a profile referenced in `inherits` doesn't exist
///
/// Built-in profiles (`debug`, `release`) are synthesized; no declaration needed.
pub fn resolve_profile(
    name: &str,
    user_profiles: &HashMap<String, ProfileConfig>,
) -> Result<ResolvedProfile> {
    let full = build_full_profiles(user_profiles);

    if !full.contains_key(name) {
        bail!("profile '{name}' not found in CCGO.toml");
    }

    let chain = expand_chain(name, &full)?;

    let mut resolved = ResolvedProfile::default();
    for node_name in &chain {
        let cfg = match full.get(*node_name) {
            Some(c) => c,
            None => bail!("profile '{node_name}' referenced in inherits chain but not defined"),
        };
        apply_scalar_fields(&mut resolved, cfg);
        apply_platform_configs(&mut resolved, cfg);
    }

    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        Linkage, MergeStrategy, ProfileCmake, ProfileConfig, ProfileDepLinkage, ProfileListField,
        ProfilePlatformBuild, ProfilePlatformConfig, ProfilePlatforms,
    };

    fn profiles_from(pairs: &[(&str, ProfileConfig)]) -> HashMap<String, ProfileConfig> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.clone())).collect()
    }

    #[test]
    fn builtin_debug_resolves() {
        let profs = HashMap::new();
        let r = resolve_profile("debug", &profs).unwrap();
        assert_eq!(r.release, Some(false));
    }

    #[test]
    fn builtin_release_resolves() {
        let profs = HashMap::new();
        let r = resolve_profile("release", &profs).unwrap();
        assert_eq!(r.release, Some(true));
    }

    #[test]
    fn unknown_profile_errors() {
        let profs = HashMap::new();
        assert!(resolve_profile("nope", &profs).is_err());
    }

    #[test]
    fn cycle_detected() {
        let mut a = ProfileConfig::default();
        a.inherits = Some("b".to_string());
        let mut b = ProfileConfig::default();
        b.inherits = Some("a".to_string());
        let profs = profiles_from(&[("a", a), ("b", b)]);
        assert!(resolve_profile("a", &profs).is_err());
    }

    #[test]
    fn scalar_fields_inherited_and_overridable() {
        let mut base = ProfileConfig::default();
        base.release = Some(false);
        base.jobs = Some(2);

        let mut child = ProfileConfig::default();
        child.inherits = Some("base".to_string());
        child.jobs = Some(8);

        let profs = profiles_from(&[("base", base), ("child", child)]);
        let r = resolve_profile("child", &profs).unwrap();
        assert_eq!(r.release, Some(false));
        assert_eq!(r.jobs, Some(8));
    }

    #[test]
    fn cmake_replace_strategy() {
        let mut base = ProfileConfig::default();
        base.cmake = Some(ProfileCmake {
            merge: MergeStrategy::Replace,
            arguments: vec!["-DA=1".to_string()],
            ..Default::default()
        });

        let mut child = ProfileConfig::default();
        child.inherits = Some("base".to_string());
        child.cmake = Some(ProfileCmake {
            merge: MergeStrategy::Replace,
            arguments: vec!["-DB=2".to_string()],
            ..Default::default()
        });

        let profs = profiles_from(&[("base", base), ("child", child)]);
        let r = resolve_profile("child", &profs).unwrap();
        assert_eq!(r.cmake.arguments, vec!["-DB=2"]);
    }

    #[test]
    fn cmake_extend_strategy() {
        let mut base = ProfileConfig::default();
        base.cmake = Some(ProfileCmake {
            merge: MergeStrategy::Replace,
            arguments: vec!["-DA=1".to_string()],
            ..Default::default()
        });

        let mut child = ProfileConfig::default();
        child.inherits = Some("base".to_string());
        child.cmake = Some(ProfileCmake {
            merge: MergeStrategy::Extend,
            arguments: vec!["-DB=2".to_string()],
            ..Default::default()
        });

        let profs = profiles_from(&[("base", base), ("child", child)]);
        let r = resolve_profile("child", &profs).unwrap();
        assert_eq!(r.cmake.arguments, vec!["-DA=1", "-DB=2"]);
    }

    #[test]
    fn features_extend_strategy() {
        let mut base = ProfileConfig::default();
        base.features = Some(ProfileListField {
            merge: MergeStrategy::Replace,
            list: vec!["a".to_string()],
        });

        let mut child = ProfileConfig::default();
        child.inherits = Some("base".to_string());
        child.features = Some(ProfileListField {
            merge: MergeStrategy::Extend,
            list: vec!["b".to_string()],
        });

        let profs = profiles_from(&[("base", base), ("child", child)]);
        let r = resolve_profile("child", &profs).unwrap();
        assert_eq!(r.features, vec!["a", "b"]);
    }

    #[test]
    fn platform_cmake_resolved() {
        let mut prof = ProfileConfig::default();
        prof.platforms = Some(ProfilePlatforms {
            android: Some(ProfilePlatformConfig {
                build: Some(ProfilePlatformBuild {
                    cmake: Some(ProfileCmake {
                        merge: MergeStrategy::Replace,
                        arguments: vec!["-DANDROID_ARM_NEON=TRUE".to_string()],
                        ..Default::default()
                    }),
                    dep_linkage: None,
                }),
            }),
            ..Default::default()
        });
        let profs = profiles_from(&[("android_neon", prof)]);
        let r = resolve_profile("android_neon", &profs).unwrap();
        let android_cmake = r.platform_cmake.get("android").unwrap();
        assert_eq!(android_cmake.arguments, vec!["-DANDROID_ARM_NEON=TRUE"]);
    }

    #[test]
    fn dep_linkage_last_wins() {
        let mut base = ProfileConfig::default();
        base.dep_linkage = Some(ProfileDepLinkage {
            default: Some(Linkage::StaticEmbedded),
            on_shared: None,
            on_static: None,
        });

        let mut child = ProfileConfig::default();
        child.inherits = Some("base".to_string());
        child.dep_linkage = Some(ProfileDepLinkage {
            default: Some(Linkage::SharedExternal),
            on_shared: None,
            on_static: None,
        });

        let profs = profiles_from(&[("base", base), ("child", child)]);
        let r = resolve_profile("child", &profs).unwrap();
        assert_eq!(r.dep_linkage.default, Some(Linkage::SharedExternal));
    }

    #[test]
    fn inherits_builtin_debug() {
        let mut child = ProfileConfig::default();
        child.inherits = Some("debug".to_string());
        child.jobs = Some(4);

        let profs = profiles_from(&[("mysanitize", child)]);
        let r = resolve_profile("mysanitize", &profs).unwrap();
        assert_eq!(r.release, Some(false)); // inherited from builtin debug
        assert_eq!(r.jobs, Some(4));
    }
}
