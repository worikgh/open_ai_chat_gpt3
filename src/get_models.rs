use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Permission {
    id: String,
    object: String,
    created: u64,
    allow_create_engine: bool,
    allow_sampling: bool,
    allow_logprobs: bool,
    allow_search_indices: bool,
    allow_view: bool,
    allow_fine_tuning: bool,
    organization: String,
    group: Option<String>,
    is_blocking: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct Model {
    id: String,
    object: String,
    created: u64,
    owned_by: String,
    permission: Vec<Permission>,
    root: String,
    parent: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ModelReturned {
    object: String,
    data: Vec<Model>,
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ModelExampleData;
    #[test]
    fn models_from_json() {
        let model_data = ModelExampleData::new();
        let json_data: String = model_data.json;
        let v: ModelReturned = match serde_json::from_str(json_data.as_str()) {
            Ok(v) => v,
            Err(err) => panic!("{err}"),
        };
        eprintln!("{:?}", &v);
        assert!(!v.data.is_empty());
    }
}
