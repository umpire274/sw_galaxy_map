use anyhow::{Context, Result};
use reqwest::blocking::Client;
use serde::Deserialize;

const LAYER_URL: &str =
    "https://services3.arcgis.com/nM57tYg6wB9iTP3P/arcgis/rest/services/planets/FeatureServer/0";

#[derive(Debug, Deserialize)]
pub struct LayerInfo {
    #[serde(rename = "serviceItemId")]
    pub service_item_id: String,

    #[serde(rename = "maxRecordCount")]
    pub max_record_count: i64,

    // Optional fields (safe if missing)
    #[serde(rename = "currentVersion")]
    pub current_version: Option<f64>,

    #[serde(rename = "editingInfo")]
    pub editing_info: Option<EditingInfo>,
}

#[derive(Debug, Deserialize)]
pub struct EditingInfo {
    #[serde(rename = "lastEditDate")]
    pub last_edit_date: Option<i64>,

    // Optional: keep for future use
    #[serde(rename = "schemaLastEditDate")]
    pub schema_last_edit_date: Option<i64>,

    #[serde(rename = "dataLastEditDate")]
    pub data_last_edit_date: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct QueryResponse {
    #[serde(default)]
    pub features: Vec<Feature>,

    #[serde(rename = "exceededTransferLimit", default)]
    pub exceeded_transfer_limit: bool,
}

#[derive(Debug, Deserialize)]
pub struct Feature {
    pub attributes: serde_json::Value,
}

pub fn fetch_layer_info(client: &Client) -> Result<LayerInfo> {
    let url = format!("{LAYER_URL}?f=json");
    let info: LayerInfo = client
        .get(url)
        .send()
        .context("Failed to fetch layer info")?
        .error_for_status()
        .context("Layer info request returned error status")?
        .json()
        .context("Failed to parse layer info JSON")?;

    Ok(info)
}

pub fn fetch_all_features(client: &Client, page_size: i64) -> Result<Vec<serde_json::Value>> {
    let mut out: Vec<serde_json::Value> = Vec::new();

    let mut offset = 0i64;
    loop {
        let url = format!("{LAYER_URL}/query");
        let resp: QueryResponse = client
            .get(&url)
            .query(&[
                ("f", "json"),
                ("where", "1=1"),
                ("outFields", "*"),
                ("returnGeometry", "false"),
                ("orderByFields", "FID"),
                ("resultOffset", &offset.to_string()),
                ("resultRecordCount", &page_size.to_string()),
            ])
            .send()
            .context("Failed to query features")?
            .error_for_status()
            .context("Query request returned error status")?
            .json()
            .context("Failed to parse query JSON")?;

        let n = resp.features.len();
        for f in resp.features {
            out.push(f.attributes);
        }

        // Se non arrivano più record, stop
        if n == 0 {
            break;
        }

        // Se il server dice che c'è ancora roba, continua; altrimenti stop.
        // (ArcGIS può settare exceededTransferLimit quando c'è paginazione)
        if !resp.exceeded_transfer_limit && n < page_size as usize {
            break;
        }

        offset += page_size;
    }

    Ok(out)
}
