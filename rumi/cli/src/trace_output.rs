//! Human-readable rendering of an `EvalTrace`.
//!
//! Answers the question people actually ask, which is not "what matched" but
//! "why did *that* one match and not mine". So the output leads with the rule
//! index, shows the value that was extracted, and marks the winner.

use std::fmt::Display;

use rumi::{EvalTrace, PredicateTrace};

/// Render a trace to stdout.
pub fn print<A: Display>(trace: &EvalTrace<A>) {
    if trace.steps.is_empty() {
        println!("no rules were evaluated (empty matcher list)");
    }

    for step in &trace.steps {
        let mark = if step.matched { "MATCH" } else { "  -  " };
        println!("[{mark}] rule {}", step.index);
        print_predicate(&step.predicate_trace, 2);
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
