use serde_json::{Map, Value, json};

use crate::RosPeekResult;

pub fn flatten_json(json: &Map<String, Value>) -> RosPeekResult<Map<String, Value>> {
    let mut output = Map::new();
    insert_object(&mut output, None, json);
    Ok(output)
}

fn insert_object(
    base_json: &mut Map<String, Value>,
    base_key: Option<&str>,
    object: &Map<String, Value>,
) {
    object.iter().for_each(|(key, value)| {
        let new_key = base_key.map_or_else(|| key.clone(), |base_key| format!("{base_key}.{key}"));

        if let Some(array) = value.as_array() {
            insert_array(base_json, &new_key, array);
        } else if let Some(object) = value.as_object() {
            insert_object(base_json, Some(&new_key), object);
        } else {
            insert_value(base_json, &new_key, value);
        }
    });
}

fn insert_array(base_json: &mut Map<String, Value>, base_key: &str, array: &[Value]) {
    array.iter().for_each(|value| {
        if let Some(object) = value.as_object() {
            insert_object(base_json, Some(base_key), object);
        } else if let Some(sub_array) = value.as_array() {
            insert_array(base_json, base_key, sub_array);
        } else {
            insert_value(base_json, base_key, value);
        }
    });
}

fn insert_value(base_json: &mut Map<String, Value>, key: &str, to_insert: &Value) {
    if let Some(value) = base_json.get_mut(key) {
        if let Some(array) = value.as_array_mut() {
            array.push(to_insert.clone());
        } else {
            base_json[key] = json!([value, to_insert]);
        }
    } else {
        base_json.insert(key.to_string(), json!(to_insert));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_flattening() {
        let mut base: Value = json!({
          "id": "287947",
          "title": "Shazam!",
          "release_date": 1553299200,
          "genres": [
            "Action",
            "Comedy",
            "Fantasy"
          ]
        });
        let json = std::mem::take(base.as_object_mut().unwrap());
        let flat = flatten_json(&json).unwrap_or_else(|_| panic!("Failed to flatten json"));

        println!(
            "got:\n{}\nexpected:\n{}\n",
            serde_json::to_string_pretty(&flat).unwrap(),
            serde_json::to_string_pretty(&json).unwrap()
        );

        assert_eq!(flat, json);
    }

    #[test]
    fn flatten_object() {
        let mut base: Value = json!({
          "a": {
            "b": "c",
            "d": "e",
            "f": "g"
          }
        });
        let json = std::mem::take(base.as_object_mut().unwrap());
        let flat = flatten_json(&json).unwrap_or_else(|_| panic!("Failed to flatten json"));

        assert_eq!(
            &flat,
            json!({
                "a.b": "c",
                "a.d": "e",
                "a.f": "g"
            })
            .as_object()
            .unwrap()
        );
    }

    #[test]
    fn flatten_array() {
        let mut base: Value = json!({
          "a": [
            { "b": "c" },
            { "b": "d" },
            { "b": "e" },
          ]
        });
        let json = std::mem::take(base.as_object_mut().unwrap());
        let flat = flatten_json(&json).unwrap_or_else(|_| panic!("Failed to flatten json"));

        assert_eq!(
            &flat,
            json!({
                "a.b": ["c", "d", "e"],
            })
            .as_object()
            .unwrap()
        );

        // here we must keep 42 in "a"
        let mut base: Value = json!({
          "a": [
            42,
            { "b": "c" },
            { "b": "d" },
            { "b": "e" },
          ]
        });
        let json = std::mem::take(base.as_object_mut().unwrap());
        let flat = flatten_json(&json).unwrap_or_else(|_| panic!("Failed to flatten json"));

        assert_eq!(
            &flat,
            json!({
                "a": 42,
                "a.b": ["c", "d", "e"],
            })
            .as_object()
            .unwrap()
        );
    }

    #[test]
    fn collision_with_object() {
        let mut base: Value = json!({
          "a": {
            "b": "c",
          },
          "a.b": "d",
        });
        let json = std::mem::take(base.as_object_mut().unwrap());
        let flat = flatten_json(&json).unwrap_or_else(|_| panic!("Failed to flatten json"));

        assert_eq!(
            &flat,
            json!({
                "a.b": ["c", "d"],
            })
            .as_object()
            .unwrap()
        );
    }

    #[test]
    fn collision_with_array() {
        let mut base: Value = json!({
          "a": [
            { "b": "c" },
            { "b": "d", "c": "e" },
            [35],
          ],
          "a.b": "f",
        });
        let json = std::mem::take(base.as_object_mut().unwrap());
        let flat = flatten_json(&json).unwrap_or_else(|_| panic!("Failed to flatten json"));

        assert_eq!(
            &flat,
            json!({
                "a.b": ["c", "d", "f"],
                "a.c": "e",
                "a": 35,
            })
            .as_object()
            .unwrap()
        );
    }

    #[test]
    fn flatten_nested_arrays() {
        let mut base: Value = json!({
          "a": [
            ["b", "c"],
            { "d": "e" },
            ["f", "g"],
            [
                { "h": "i" },
                { "d": "j" },
            ],
            ["k", "l"],
          ]
        });
        let json = std::mem::take(base.as_object_mut().unwrap());
        let flat = flatten_json(&json).unwrap_or_else(|_| panic!("Failed to flatten json"));

        assert_eq!(
            &flat,
            json!({
                "a": ["b", "c", "f", "g", "k", "l"],
                "a.d": ["e", "j"],
                "a.h": "i",
            })
            .as_object()
            .unwrap()
        );
    }

    #[test]
    fn flatten_nested_arrays_and_objects() {
        let mut base: Value = json!({
          "a": [
            "b",
            ["c", "d"],
            { "e": ["f", "g"] },
            [
                { "h": "i" },
                { "e": ["j", { "z": "y" }] },
            ],
            ["l"],
            "m",
          ]
        });
        let json = std::mem::take(base.as_object_mut().unwrap());
        let flat = flatten_json(&json).unwrap_or_else(|_| panic!("Failed to flatten json"));

        println!("{}", serde_json::to_string_pretty(&flat).unwrap());

        assert_eq!(
            &flat,
            json!({
                "a": ["b", "c", "d", "l", "m"],
                "a.e": ["f", "g", "j"],
                "a.h": "i",
                "a.e.z": "y",
            })
            .as_object()
            .unwrap()
        );
    }
}
