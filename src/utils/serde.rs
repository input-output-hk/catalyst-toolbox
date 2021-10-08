use serde::{Deserialize, Deserializer};

pub fn deserialize_truthy_falsy<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let truthy_value: String = String::deserialize(deserializer)?;
    Ok(match truthy_value.to_lowercase().as_ref() {
        "x" => true,
        "1" => true,
        "true" => true,
        _ => false,
    })
}
