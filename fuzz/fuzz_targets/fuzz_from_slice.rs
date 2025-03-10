#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() <= 10240 {
        _ = dbt_serde_yaml::from_slice::<dbt_serde_yaml::Value>(data);
    }
});
