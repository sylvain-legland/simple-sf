use sf_engine::{db, catalog, eval};
use std::sync::Once;

static INIT: Once = Once::new();

fn ensure_db() {
    INIT.call_once(|| {
        let tmp = std::env::temp_dir().join("sf_eval_test.db");
        let _ = std::fs::remove_file(&tmp);
        db::init_db(tmp.to_str().unwrap());
        let data_dir = format!("{}/../SimpleSF/Resources/SFData", env!("CARGO_MANIFEST_DIR"));
        catalog::seed_from_json(&data_dir);
    });
}

#[test]
fn eval_l10_skills() {
    ensure_db();
    let r = eval::eval_all_skills();
    eprintln!("\n{}", r.summary());
    let fail_count = r.total() - r.passed();
    assert!(fail_count <= 2, "L10-Skills: {} failures (max 2)\n{}",
        fail_count,
        r.cases.iter().filter(|c| !c.passed)
            .map(|c| format!("  ✗ {}: {}", c.id, c.detail))
            .collect::<Vec<_>>().join("\n"));
}

#[test]
fn eval_l11_agents() {
    ensure_db();
    let r = eval::eval_all_agents();
    eprintln!("\n{}", r.summary());
    let fail_count = r.total() - r.passed();
    assert!(fail_count <= 1, "L11-Agents: {} failures (max 1)\n{}",
        fail_count,
        r.cases.iter().filter(|c| !c.passed)
            .map(|c| format!("  ✗ {}: {}", c.id, c.detail))
            .collect::<Vec<_>>().join("\n"));
}

#[test]
fn eval_l12_patterns() {
    ensure_db();
    let r = eval::eval_all_patterns();
    eprintln!("\n{}", r.summary());
    let fail_count = r.total() - r.passed();
    assert!(fail_count <= 1, "L12-Patterns: {} failures (max 1)\n{}",
        fail_count,
        r.cases.iter().filter(|c| !c.passed)
            .map(|c| format!("  ✗ {}: {}", c.id, c.detail))
            .collect::<Vec<_>>().join("\n"));
}

#[test]
fn eval_full_report() {
    ensure_db();
    let report = eval::full_eval_report();
    eprintln!("\n{}", report);
}

#[test]
fn eval_single_skill() {
    ensure_db();
    // Test individual skill eval
    let sbd = eval::eval_skill("securebydesign");
    assert!(sbd.is_some(), "SecureByDesign skill must exist");
    let sc = sbd.unwrap();
    assert!(sc.has_name, "SBD must have name");
    assert!(sc.has_description, "SBD must have description");
    assert!(sc.description_actionable, "SBD must be actionable");
    assert!(sc.has_content, "SBD must have content");
    assert!(sc.content_substantial, "SBD must have substantial content");
    assert!(sc.no_placeholders, "SBD must have no placeholders");
    assert!(sc.overall, "SBD must pass overall");
    eprintln!("SecureByDesign skill: {:?}", sc);
}
