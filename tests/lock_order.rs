//! Verify that the optional owner cannot reach a waiting primitive or the writer.
//! Function names are conservatively resolved to every definition of that name.
//! on_screen is the single accepted movement call, compared separately in release ELF.
use std::collections::{BTreeMap, BTreeSet};

fn tokens(src: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut ident = String::new();
    let mut quoted = false;
    let mut escape = false;
    for line in src.lines() {
        for c in line.split("//").next().unwrap_or("").chars().chain(['\n']) {
            if quoted {
                quoted = escape || c != '"';
                escape = !escape && c == '\\';
            } else if c.is_ascii_alphanumeric() || c == '_' {
                ident.push(c);
            } else {
                if !ident.is_empty() {
                    words.push(std::mem::take(&mut ident));
                }
                quoted = c == '"';
                if !quoted && !c.is_whitespace() {
                    words.push(c.to_string());
                }
            }
        }
    }
    words
}

fn definitions(src: &str, bodies: &mut BTreeMap<String, Vec<Vec<String>>>) {
    let words = tokens(src.split("#[cfg(test)]").next().unwrap());
    for i in 0..words.len().saturating_sub(2) {
        if words[i] != "fn" || !matches!(words[i + 2].as_str(), "(" | "<") {
            continue;
        }
        let mut array_depth = 0;
        let Some(start) = (i + 2..words.len()).find(|&j| {
            match words[j].as_str() {
                "[" => array_depth += 1,
                "]" => array_depth -= 1,
                "{" | ";" if array_depth == 0 => return true,
                _ => (),
            }
            false
        })
        else {
            continue;
        };
        if words[start] == ";" {
            continue;
        }
        let mut depth = 1;
        let end = (start + 1..words.len())
            .find(|&j| {
                match words[j].as_str() {
                    "{" => depth += 1,
                    "}" => depth -= 1,
                    _ => (),
                }
                depth == 0
            })
            .expect("balanced function body");
        bodies
            .entry(words[i + 1].clone())
            .or_default()
            .push(words[start + 1..end].to_vec());
    }
}

fn audit(sources: &[&str], roots: &[&str]) -> (BTreeSet<String>, Vec<String>) {
    let mut bodies = BTreeMap::new();
    for src in sources {
        definitions(src, &mut bodies);
    }
    let mut queue: Vec<String> = roots.iter().map(|s| s.to_string()).collect();
    let mut visited = BTreeSet::new();
    let mut violations = Vec::new();
    while let Some(name) = queue.pop() {
        if !visited.insert(name.clone()) || name == "on_screen" {
            continue;
        }
        for body in &bodies[&name] {
            for (i, t) in body.iter().enumerate() {
                if t.starts_with("svc")
                    || t.contains("QueryMemory")
                    || t.ends_with("Mutex")
                    || t.ends_with("Event")
                    || (t == "nn" && name != "tick")
                    || matches!(t.as_str(), "skyline" | "nnsdk" | "std")
                    || matches!(
                        t.as_str(),
                        "lock"
                            | "RwLock"
                            | "Condvar"
                            | "spin_loop"
                            | "yield_now"
                            | "sleep"
                            | "join"
                            | "alloc"
                            | "realloc"
                            | "Box"
                            | "Vec"
                            | "String"
                            | "format"
                            | "println"
                            | "thread"
                            | "fs"
                            | "ro"
                            | "libc"
                            | "LookupSymbol"
                            | "send"
                            | "register"
                            | "poll"
                            | "publish"
                            | "transmute"
                            | "p1_buttons"
                    )
                {
                    violations.push(format!("{name} reaches {t}"));
                }
                // A local such as the vector's `start` is not Worker::start().
                // Also follow qualified function items passed to Option/iterator callbacks.
                let called = body.get(i + 1).is_some_and(|n| n == "(" || n == ":");
                let qualified = i >= 2 && body[i-1] == ":" && body[i-2] == ":";
                if (called || qualified) && bodies.contains_key(t) {
                    queue.push(t.clone());
                }
            }
        }
    }
    (visited, violations)
}

#[test]
fn lock_order_optional_owners_reach_only_nonblocking_capture() {
    let sources = [
        include_str!("../src/runtime.rs"),
        include_str!("../src/observed.rs"),
        include_str!("../src/p1.rs"),
        include_str!("../src/layout.rs"),
        include_str!("../src/provider.rs"),
        include_str!("../src/model.rs"),
        include_str!("../src/stage_reader.rs"),
        include_str!("../src/stage.rs"),
    ];
    let (reached, violations) = audit(
        &sources,
        &[
            "observed_css",
            "observed_sss",
            "stage_actor_return",
            "stage_owner_resumed",
        ],
    );
    assert!(violations.is_empty(), "{violations:?}");
    assert_eq!(
        sources[1]
            .matches("on_screen(screen, object, &view, coins.as_ref());")
            .count(),
        1
    );
    for name in [
        "observe",
        "grid",
        "identify",
            "collect",
            "coins",
            "coin_offer",
        "read_local_seat",
        "token",
        "claimed",
        "tick",
        "on_screen",
        "from_getter",
        "tab",
        "qualified_stage",
    ] {
        assert!(reached.contains(name), "missing owner path to {name}");
    }
    let (_, token_violations) = audit(&sources, &["empty_hand", "token_target", "pack_token"]);
    assert!(token_violations.is_empty(), "{token_violations:?}");
}

#[test]
fn lock_order_detects_transitive_violations() {
    let fixture = "fn hook() { inner(); } fn inner(arg: [u64; 2]) { unsafe { svcQueryMemory(); } cell.lock(); }";
    let (_, bad) = audit(&[fixture], &["hook"]);
    assert_eq!(bad, ["inner reaches svcQueryMemory", "inner reaches lock"]);
}
