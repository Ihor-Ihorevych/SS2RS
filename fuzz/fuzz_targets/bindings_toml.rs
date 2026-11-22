//! A key bindings file of any content is parsed or refused, and what parses survives a save.
#![no_main]

use libfuzzer_sys::fuzz_target;
use ss2_platform_api::BindingSet;

fuzz_target!(|data: &str| {
    if let Ok(set) = BindingSet::from_toml_str(data) {
        let text = set
            .to_toml_string()
            .expect("a parsed binding set can be written");
        let again = BindingSet::from_toml_str(&text).expect("what was written can be read");
        assert_eq!(set.to_toml_string().ok(), again.to_toml_string().ok());
    }
});
