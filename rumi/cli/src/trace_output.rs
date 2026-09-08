//! Human-readable rendering of an `EvalTrace`.
//!
//! Answers the question people actually ask, which is not "what matched" but
//! "why did *that* one match and not mine". So the output leads with the rule
//! index, shows the value that was extracted, and marks the winner.

use std::fmt::Display;

use rumi::{EvalSteps, EvalTrace, OnMatchTrace, PredicateTrace, TreeKind, TreeLookupTrace};

/// Render a trace to stdout.
pub fn print<A: Display>(trace: &EvalTrace<A>) {
    match &trace.steps {
        EvalSteps::List(steps) => {
            if steps.is_empty() {
                println!("no rules were evaluated (empty matcher list)");
            }
            for step in steps {
                let mark = if step.matched { "MATCH" } else { "  -  " };
                println!("[{mark}] rule {}", step.index);
                print_predicate(&step.predicate_trace, 2);
            }
        }
        EvalSteps::Tree(lookup) => print_tree_lookup(lookup),
    }

    println!();
    match &trace.result {
        Some(action) if trace.used_fallback => {
            println!("=> {action}   (no rule matched; on_no_match)");
        }
        Some(action) => println!("=> {action}"),
        None => println!("=> (no match, and no on_no_match)"),
    }
}

/// A tree does one lookup, so the trace reports the lookup rather than a list
/// of rules. The three ways it can miss are kept distinct: no usable key at
/// all, a key that matched nothing, and a key that matched an entry whose
/// nested matcher then failed. Collapsing them would send a config author
/// looking in the wrong place.
fn print_tree_lookup<A: Display>(t: &TreeLookupTrace<A>) {
    let rule = match t.kind {
        TreeKind::Exact => "exact",
        TreeKind::Prefix => "longest-prefix",
    };
    println!("[TREE ] {rule} lookup on {}", t.input);

    match &t.key {
        None => println!("  key: (input produced no string — nothing to look up)"),
        Some(key) if t.attempts.is_empty() => println!("  key: {key:?} — no entry"),
        Some(key) => println!("  key: {key:?}"),
    }
    for attempt in &t.attempts {
        let entry = &attempt.matched_key;
        match &attempt.on_match {
            OnMatchTrace::Action(_) => println!("  entry {entry:?} — matched"),
            OnMatchTrace::Nested(nested) if nested.result.is_some() => {
                println!("  entry {entry:?} — matched, nested matcher decided");
            }
            OnMatchTrace::Nested(_) => {
                println!(
                    "  entry {entry:?} — nested matcher missed, trying the next shorter prefix"
                );
            }
        }
    }
}

fn print_predicate(t: &PredicateTrace, indent: usize) {
    let pad = " ".repeat(indent);
    match t {
        PredicateTrace::Single {
            matched,
            input,
            data,
            matcher,
        } => {
            // `data` is the value the input actually pulled out of the context.
            // Showing it is the whole point: a rule that never fires usually
            // means the input read nothing.
            println!(
                "{pad}{} {input} = {data}  vs  {matcher}",
                if *matched { "✓" } else { "✗" }
            );
        }
        PredicateTrace::And { matched, children } => {
            println!("{pad}{} AND", if *matched { "✓" } else { "✗" });
            for c in children {
                print_predicate(c, indent + 2);
            }
        }
        PredicateTrace::Or { matched, children } => {
            println!("{pad}{} OR", if *matched { "✓" } else { "✗" });
            for c in children {
                print_predicate(c, indent + 2);
            }
        }
        PredicateTrace::Not { matched, inner } => {
            println!("{pad}{} NOT", if *matched { "✓" } else { "✗" });
            print_predicate(inner, indent + 2);
        }
    }
}
