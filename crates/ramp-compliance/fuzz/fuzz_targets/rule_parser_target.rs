#![no_main]
use libfuzzer_sys::fuzz_target;
use ramp_compliance::rule_parser::RuleParser;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Just try to parse it. We don't care if it fails validation,
        // we just want to ensure it doesn't panic/crash.
        let _ = RuleParser::parse_json(s);
        let _ = RuleParser::parse(s);
    }
});
