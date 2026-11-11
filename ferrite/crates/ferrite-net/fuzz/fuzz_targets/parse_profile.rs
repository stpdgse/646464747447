//! The session server's reply is third-party JSON. Whatever it contains, the parser must not
//! panic, and anything it accepts must satisfy the protocol limits the profile is sent under.
#![no_main]

use ferrite_net::auth::parse_profile;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Some((&n, rest)) = data.split_first() else { return };
    let Ok(body) = std::str::from_utf8(rest) else { return };
    let requested: String = "abcdefghijklmnop".chars().take(usize::from(n % 17)).collect();
    if let Ok(profile) = parse_profile(body, &requested) {
        assert!(profile.name.0.chars().count() <= 16);
        assert!(profile.name.0.eq_ignore_ascii_case(&requested));
        assert!(profile.properties.0.len() <= 16);
        for p in &profile.properties.0 {
            assert!(p.name.0.chars().count() <= 64);
            assert!(p.value.0.chars().count() <= 32767);
            assert!(p.signature.as_ref().is_none_or(|s| s.0.chars().count() <= 1024));
        }
    }
});
