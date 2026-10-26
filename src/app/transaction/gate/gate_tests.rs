// Tier selection: flags beat policy, policy beats assumption, and the
// unbuilt tier is refused rather than silently downgraded.

use super::select;
use crate::core::gate::Tier;

#[test]
fn fast_selects_the_model_tier_whatever_the_policy_says() {
    assert_eq!(select("check", true, false).expect("tier"), Tier::Model);
}

#[test]
fn strict_selects_the_strict_tier_whatever_the_policy_says() {
    assert_eq!(select("model", false, true).expect("tier"), Tier::Strict);
}

#[test]
fn without_flags_the_policy_tier_runs() {
    assert_eq!(select("check", false, false).expect("tier"), Tier::Check);
    assert_eq!(select("model", false, false).expect("tier"), Tier::Model);
    assert_eq!(select("strict", false, false).expect("tier"), Tier::Strict);
}

#[test]
fn a_policy_naming_the_unbuilt_semantic_tier_is_refused() {
    let found = select("semantic", false, false);

    assert!(found.is_err());
    assert_eq!(found.expect_err("error").code, "usage.unsupported");
}

#[test]
fn an_unknown_policy_tier_is_refused_not_defaulted() {
    let found = select("bogus", false, false);

    assert!(found.is_err());
    assert_eq!(found.expect_err("error").code, "env.bad_policy");
}
