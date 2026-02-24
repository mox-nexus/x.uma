//! rumi CLI — driving adapter for the rumi matcher engine.
//!
//! Subcommands:
//! - `eval <config>  [--context key=value...]` — evaluate config against context
//! - `check <config>` — validate config loads without errors
//! - `info` — print registered type URLs

use std::collections::HashMap;
use std::process;

use rumi::MatcherConfig;
use rumi_test::TestContext;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_usage();
        process::exit(1);
    }

    let result = match args[1].as_str() {
        "eval" => cmd_eval(&args[2..]),
        "check" => cmd_check(&args[2..]),
        "info" => cmd_info(),
        "--help" | "-h" | "help" => {
            print_usage();
            Ok(())
        }
        other => {
            eprintln!("error: unknown command \"{other}\"");
            print_usage();
            process::exit(1);
        }
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        process::exit(1);
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Commands
// ═══════════════════════════════════════════════════════════════════════════════

fn cmd_eval(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("eval requires a config file path".into());
    }

    let config_path = &args[0];
    let context = parse_context(&args[1..])?;

    let config = load_config(config_path)?;
    let registry = build_registry();
    let matcher = registry
        .load_matcher(config)
        .map_err(|e| format!("config load failed: {e}"))?;

    let ctx = build_test_context(&context);
    match matcher.evaluate(&ctx) {
        Some(action) => println!("{action}"),
        None => println!("(no match)"),
    }

    Ok(())
}

fn cmd_check(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("check requires a config file path".into());
    }

    let config_path = &args[0];
    let config = load_config(config_path)?;
    let registry = build_registry();

    registry
        .load_matcher(config)
        .map_err(|e| format!("config invalid: {e}"))?;

    println!("Config valid");
    Ok(())
}

#[allow(clippy::unnecessary_wraps)] // Uniform return type for all commands
fn cmd_info() -> Result<(), String> {
    let registry = build_registry();

    println!("Registered inputs:");
    for url in registry.input_type_urls() {
        println!("  {url}");
    }

    println!("\nRegistered matchers:");
    for url in registry.matcher_type_urls() {
        println!("  {url}");
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// Registry assembly (composition root)
// ═══════════════════════════════════════════════════════════════════════════════

fn build_registry() -> rumi::Registry<TestContext> {
    let builder = rumi::RegistryBuilder::new();
    rumi_test::register(builder).build()
}

fn build_test_context(pairs: &HashMap<String, String>) -> TestContext {
    let mut ctx = TestContext::new();
    for (k, v) in pairs {
        ctx = ctx.with(k, v);
    }
    ctx
}

// ═══════════════════════════════════════════════════════════════════════════════
// Config loading
// ═══════════════════════════════════════════════════════════════════════════════

fn load_config(path: &str) -> Result<MatcherConfig<String>, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("failed to read \"{path}\": {e}"))?;

    let is_json = std::path::Path::new(path)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("json"));

    if is_json {
        serde_json::from_str(&content).map_err(|e| format!("JSON parse error: {e}"))
    } else {
        // Default to YAML (handles .yaml and .yml)
        serde_yaml::from_str(&content).map_err(|e| format!("YAML parse error: {e}"))
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Argument parsing
// ═══════════════════════════════════════════════════════════════════════════════

fn parse_context(args: &[String]) -> Result<HashMap<String, String>, String> {
    let mut map = HashMap::new();
    let mut i = 0;

    while i < args.len() {
        if args[i] == "--context" {
            i += 1;
            while i < args.len() && !args[i].starts_with("--") {
                let pair = &args[i];
                let (key, value) = pair.split_once('=').ok_or_else(|| {
                    format!("invalid context pair \"{pair}\", expected key=value")
                })?;
                map.insert(key.to_owned(), value.to_owned());
                i += 1;
            }
        } else {
            return Err(format!("unexpected argument \"{}\"", args[i]));
        }
    }

    Ok(map)
}

fn print_usage() {
    eprintln!(
        "Usage: rumi <command> [options]

Commands:
  eval <config> [--context key=value...]   Evaluate config against context
  check <config>                           Validate config
  info                                     Print registered type URLs
  help                                     Show this help"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_context_empty() {
        let result = parse_context(&[]);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn parse_context_pairs() {
        let args: Vec<String> = vec!["--context".into(), "method=GET".into(), "path=/api".into()];
        let result = parse_context(&args).unwrap();
        assert_eq!(result.get("method").unwrap(), "GET");
        assert_eq!(result.get("path").unwrap(), "/api");
    }

    #[test]
    fn parse_context_missing_equals() {
        let args: Vec<String> = vec!["--context".into(), "badformat".into()];
        let result = parse_context(&args);
        assert!(result.is_err());
    }

    #[test]
    fn build_registry_has_expected_types() {
        let registry = build_registry();
        assert!(registry.contains_input("xuma.test.v1.StringInput"));
        assert!(registry.contains_matcher("xuma.core.v1.StringMatcher"));
        assert!(registry.contains_matcher("xuma.core.v1.BoolMatcher"));
    }

    #[test]
    fn load_yaml_config() {
        let config = load_config("/tmp/xuma-yaml-test/config.yaml");
        assert!(
            config.is_ok(),
            "failed to load YAML config: {:?}",
            config.err()
        );
    }

    #[test]
    fn eval_yaml_config() {
        let config = load_config("/tmp/xuma-yaml-test/config.yaml").unwrap();
        let registry = build_registry();
        let matcher = registry.load_matcher(config).unwrap();

        let ctx = TestContext::new().with("method", "GET");
        assert_eq!(matcher.evaluate(&ctx), Some("route-get".to_string()));

        let ctx = TestContext::new().with("method", "POST");
        assert_eq!(matcher.evaluate(&ctx), Some("route-post".to_string()));

        let ctx = TestContext::new().with("method", "DELETE");
        assert_eq!(matcher.evaluate(&ctx), Some("fallback".to_string()));
    }
}
