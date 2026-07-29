use crate::types::DiscoveredModel;

/// Fallback models shipped with the application for offline / first-run use.
/// These match the hardcoded list that was previously in mainwindow.cpp.
pub fn curated_models(models_dir: &str) -> Vec<DiscoveredModel> {
    vec![
        DiscoveredModel {
            name: "Qwen3-1.7B (Q8_0)".into(),
            url: "https://huggingface.co/Qwen/Qwen3-1.7B-GGUF/resolve/main/Qwen3-1.7B-Q8_0.gguf".into(),
            path: format!("{}/Qwen3-1.7B-Q8_0.gguf", models_dir),
            size_bytes: 1800000000,
            size_label: "~1.8 GB".into(),
            sha256: None,
            hf_id: "Qwen/Qwen3-1.7B-GGUF".into(),
            downloads: 0,
        },
        DiscoveredModel {
            name: "Qwen2.5-0.5B (Q5_0)".into(),
            url: "https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct-GGUF/resolve/main/qwen2.5-0.5b-instruct-q5_0.gguf".into(),
            path: format!("{}/qwen2.5-0.5b-instruct-q5_0.gguf", models_dir),
            size_bytes: 500000000,
            size_label: "~500 MB".into(),
            sha256: None,
            hf_id: "Qwen/Qwen2.5-0.5B-Instruct-GGUF".into(),
            downloads: 0,
        },
        DiscoveredModel {
            name: "TinyLlama-1.1B-Chat (Q4_K_M)".into(),
            url: "https://huggingface.co/TheBloke/TinyLlama-1.1B-Chat-v1.0-GGUF/resolve/main/tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf".into(),
            path: format!("{}/tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf", models_dir),
            size_bytes: 669000000,
            size_label: "~669 MB".into(),
            sha256: None,
            hf_id: "TheBloke/TinyLlama-1.1B-Chat-v1.0-GGUF".into(),
            downloads: 0,
        },
        DiscoveredModel {
            name: "Llama-3.2-1B-Instruct (Q4_K_M)".into(),
            url: "https://huggingface.co/bartowski/Llama-3.2-1B-Instruct-GGUF/resolve/main/Llama-3.2-1B-Instruct-Q4_K_M.gguf".into(),
            path: format!("{}/Llama-3.2-1B-Instruct-Q4_K_M.gguf", models_dir),
            size_bytes: 808000000,
            size_label: "~808 MB".into(),
            sha256: None,
            hf_id: "bartowski/Llama-3.2-1B-Instruct-GGUF".into(),
            downloads: 0,
        },
        DiscoveredModel {
            name: "SmolLM2-1.7B-Instruct (Q4_K_M)".into(),
            url: "https://huggingface.co/HuggingFaceTB/SmolLM2-1.7B-Instruct-GGUF/resolve/main/smollm2-1.7b-instruct-q4_k_m.gguf".into(),
            path: format!("{}/smollm2-1.7b-instruct-q4_k_m.gguf", models_dir),
            size_bytes: 1060000000,
            size_label: "~1.06 GB".into(),
            sha256: None,
            hf_id: "HuggingFaceTB/SmolLM2-1.7B-Instruct-GGUF".into(),
            downloads: 0,
        },
        DiscoveredModel {
            name: "Gemma-2-2B-it (Q4_K_M)".into(),
            url: "https://huggingface.co/bartowski/gemma-2-2b-it-GGUF/resolve/main/gemma-2-2b-it-Q4_K_M.gguf".into(),
            path: format!("{}/gemma-2-2b-it-Q4_K_M.gguf", models_dir),
            size_bytes: 1710000000,
            size_label: "~1.71 GB".into(),
            sha256: None,
            hf_id: "bartowski/gemma-2-2b-it-GGUF".into(),
            downloads: 0,
        },
        DiscoveredModel {
            name: "Phi-3.5-mini-instruct (Q4_K_M)".into(),
            url: "https://huggingface.co/bartowski/Phi-3.5-mini-instruct-GGUF/resolve/main/Phi-3.5-mini-instruct-Q4_K_M.gguf".into(),
            path: format!("{}/Phi-3.5-mini-instruct-Q4_K_M.gguf", models_dir),
            size_bytes: 2390000000,
            size_label: "~2.39 GB".into(),
            sha256: None,
            hf_id: "bartowski/Phi-3.5-mini-instruct-GGUF".into(),
            downloads: 0,
        },
    ]
}

/// Query HuggingFace API for GGUF models, merged with curated fallback list.
/// Returns JSON string of discovered models.
pub fn discover_models(models_dir: &str) -> Result<String, String> {
    let mut all = curated_models(models_dir);

    // Attempt live HF discovery — best-effort; curated list is always the fallback
    if let Ok(live) = discover_from_hf_api(models_dir) {
        // Merge: prefer curated entries, add new ones from live
        let curated_urls: std::collections::HashSet<String> =
            all.iter().map(|m| m.url.clone()).collect();
        for m in live {
            if !curated_urls.contains(&m.url) {
                all.push(m);
            }
        }
    }

    serde_json::to_string(&all).map_err(|e| format!("Serialization error: {e}"))
}

fn discover_from_hf_api(models_dir: &str) -> Result<Vec<DiscoveredModel>, String> {
    let rt = tokio::runtime::Runtime::new().map_err(|e| format!("Runtime error: {e}"))?;
    rt.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| format!("HTTP client error: {e}"))?;

        let resp = client
            .get("https://huggingface.co/api/models?search=GGUF&sort=downloads&direction=-1&limit=20")
            .header("User-Agent", "lazydesktop/0.1.0")
            .send()
            .await
            .map_err(|e| format!("HF API error: {e}"))?;

        let body = resp.text().await.map_err(|e| format!("Read error: {e}"))?;
        let entries: Vec<serde_json::Value> =
            serde_json::from_str(&body).map_err(|e| format!("Parse error: {e}"))?;

        let mut models = Vec::new();
        for entry in entries {
            let model_id = entry["modelId"]
                .as_str()
                .or_else(|| {
                    entry["id"]
                        .as_str()
                })
                .unwrap_or("unknown");
            let hf_id = model_id.to_string();

            // Find a GGUF file in the siblings
            if let Some(siblings) = entry["siblings"].as_array() {
                for sib in siblings {
                    let rfilename = sib["rfilename"].as_str().unwrap_or("");
                    if rfilename.ends_with(".gguf") {
                        let name = rfilename
                            .trim_end_matches(".gguf")
                            .replace(|c: char| !c.is_ascii_alphanumeric() && c != '.' && c != '-', "_");
                        let url = format!(
                            "https://huggingface.co/{}/resolve/main/{}",
                            model_id, rfilename
                        );
                        let size = sib["size"].as_i64().unwrap_or(0);
                        let size_label = if size > 1_000_000_000 {
                            format!("~{:.1} GB", size as f64 / 1_000_000_000.0)
                        } else {
                            format!("~{:.0} MB", size as f64 / 1_000_000.0)
                        };

                        models.push(DiscoveredModel {
                            name,
                            url,
                            path: format!("{}/{}", models_dir, rfilename),
                            size_bytes: size,
                            size_label,
                            sha256: None,
                            hf_id: hf_id.clone(),
                            downloads: 0,
                        });
                    }
                }
            }
        }

        Ok(models)
    })
}
